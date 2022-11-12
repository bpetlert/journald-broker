use crate::{
    script::{EnvVar, Launcher, Script},
    settings::Settings,
};
use anyhow::{bail, Result};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use systemd::{daemon, journal, Journal};
use tracing::{debug, error, info, warn};

pub struct Monitor {
    filters: Option<Vec<String>>,
    events: Vec<Event>,
    launcher: Launcher,
}

struct Event {
    pub name: String,
    pub msg_filter: String,
    next_watch_delay: Option<Duration>,
    last_found: Option<Instant>,
    pub script: PathBuf,
    pub script_timeout: Option<u64>,
}

impl Monitor {
    pub fn new(settings: Settings) -> Result<Self> {
        let events = settings
            .events
            .into_iter()
            .map(|(name, event)| Event {
                name,
                msg_filter: event.message,
                next_watch_delay: event.next_watch_delay,
                last_found: None,
                script: PathBuf::from(event.script),
                script_timeout: settings.global.as_ref().unwrap().script_timeout,
            })
            .collect();
        Ok(Self {
            filters: settings.global.and_then(|v| v.filters),
            events,
            launcher: Launcher::new(),
        })
    }

    pub fn watch(&mut self) -> Result<()> {
        // Open all kind (system + user) of log journal for reading.
        let mut journal: Journal = journal::OpenOptions::default()
            .local_only(true)
            .runtime_only(false)
            .all_namespaces(true)
            .open()?;

        // Add filters
        if let Some(filters) = &self.filters {
            for filter in filters {
                debug!("Add filter: {filter}");
                let (key, val) = {
                    let field = filter.split('=').collect::<Vec<&str>>();
                    if field.len() != 2 {
                        warn!("Incorrect filter format, {filter}");
                        continue;
                    }
                    (field[0], field[1])
                };
                journal.match_add(key, val)?;
            }
        }

        debug!("Notify systemd that we are ready :)");
        if !daemon::notify(false, vec![("READY", "1")].iter())? {
            error!("Cannot notify systemd, READY=1");
        }

        let notify_msg = "Start monitor journal message...";
        if !daemon::notify(false, vec![("STATUS", &notify_msg)].iter())? {
            error!("Cannot notify systemd, STATUS={notify_msg}");
        }

        info!("{notify_msg}");

        // Go to end of journal before start waiting for new entry
        journal.seek_tail()?; // move to the position after the most recent available entry.
        if journal.previous()? != 1 {
            bail!("Cannot move to the most recent journal entry");
        }

        loop {
            // Wait for new journal entry
            let entry = match journal.next_entry()? {
                Some(new_entry) => new_entry,
                None => loop {
                    if let Some(new_entry) = journal.await_next_entry(None)? {
                        break new_entry;
                    }
                },
            };

            let Some(log_msg) = entry.get("MESSAGE") else {
                continue;
            };
            debug!("MESSAGE: {log_msg}");

            for event_index in self.matches(log_msg)? {
                self.respond(event_index, log_msg, &entry)?;
            }
        }
    }

    fn respond(
        &mut self,
        event_index: usize,
        log_msg: &str,
        entry: &BTreeMap<String, String>,
    ) -> Result<()> {
        if self.events[event_index].in_watch_delay() {
            debug!(
                "Skip {}, it is still in next watch delay.",
                self.events[event_index].name
            );
            return Ok(());
        }

        self.events[event_index].record_last_found();

        info!(
            "Found EVENT: {name}, LOG_MESSAGE: {log_msg} => Try to execute {script}",
            name = self.events[event_index].name,
            script = self.events[event_index].script.display()
        );

        // Put script in queue.
        let mut script: Script = Script::new(
            self.events[event_index].script.clone(),
            self.events[event_index].script_timeout,
        );
        script.add_env(EnvVar::Message(log_msg.to_owned()))?;
        script.add_env(EnvVar::Json(serde_json::to_string(&entry)?))?;

        if let Err(err) = self.launcher.add(script) {
            warn!("{err}");
        }

        Ok(())
    }

    fn matches(&self, log_msg: &str) -> Result<Vec<usize>> {
        let event_regex_set = {
            static RE: once_cell::sync::OnceCell<regex::RegexSet> =
                once_cell::sync::OnceCell::new();
            RE.get_or_init(|| {
                regex::RegexSet::new(
                    &self
                        .events
                        .iter()
                        .map(|event| event.msg_filter.clone())
                        .collect::<Vec<String>>(),
                )
                .expect("Creating event message regex")
            })
        };

        Ok(event_regex_set
            .matches(log_msg)
            .into_iter()
            .collect::<Vec<usize>>())
    }
}

impl Event {
    /// Still in next watch delay?
    pub fn in_watch_delay(&self) -> bool {
        if self.next_watch_delay.is_some()
            && self.last_found.is_some()
            && self.last_found.unwrap().elapsed() <= self.next_watch_delay.unwrap()
        {
            return true;
        }

        false
    }

    /// Record last found
    pub fn record_last_found(&mut self) {
        if self.next_watch_delay.is_some() {
            self.last_found = Some(Instant::now());
        }
    }
}
