use crate::{
    script::{EnvVar, Launcher, Script},
    settings::Settings,
};
use anyhow::{bail, Result};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use systemd::{daemon, journal, Journal};
use tracing::{debug, info, warn};

pub struct Monitor {
    filters: Option<Vec<String>>,
    events: Vec<Event>,
}

struct Event {
    pub name: String,
    pub msg_filter: String,
    pub next_watch_delay: Option<Duration>,
    pub last_found: Option<Instant>,
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
        })
    }

    pub fn watch(&mut self) -> Result<()> {
        let mut journal: Journal = journal::OpenOptions::default()
            .system(true)
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

        // Go to end of journal
        journal.seek_tail()?;
        while journal.next_skip(1)? > 0 {}

        debug!("Notify systemd that we are ready :)");
        if !daemon::notify(false, vec![("READY", "1")].iter())? {
            bail!("Cannot notify systemd, READY=1");
        }

        debug!("Start script launcher");
        let launcher = Launcher::new();

        let notify_msg = "Start monitor journal message...";
        if !daemon::notify(false, vec![("STATUS", &notify_msg)].iter())? {
            bail!("Cannot notify systemd, STATUS={notify_msg}");
        }
        info!("{notify_msg}");
        loop {
            if let Some(entry) = journal.await_next_entry(None)? {
                if let Some(log_msg) = entry.get("MESSAGE") {
                    for idx in self.matches(log_msg)? {
                        // Still in next watch delay?
                        if self.events[idx].next_watch_delay.is_some()
                            && self.events[idx].last_found.is_some()
                            && self.events[idx].last_found.unwrap().elapsed()
                                <= self.events[idx].next_watch_delay.unwrap()
                        {
                            continue;
                        }

                        // Record last found
                        if self.events[idx].next_watch_delay.is_some() {
                            self.events[idx].last_found = Some(Instant::now());
                        }

                        info!(
                            "Found event: {name}, log message: {log_msg}. Try to execute {script}",
                            name = self.events[idx].name,
                            script = self.events[idx].script.display()
                        );

                        // Put script in queue.
                        let mut script: Script = Script::new(
                            self.events[idx].script.clone(),
                            self.events[idx].script_timeout,
                        );
                        script.add_env(EnvVar::Message, log_msg)?;
                        script.add_env(EnvVar::Json, &serde_json::to_string(&entry)?)?;

                        if let Err(err) = launcher.add(script) {
                            warn!("{err}");
                        }
                    }
                }
            }
        }
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
