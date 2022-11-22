use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use config::{Config, FileFormat, Map};
use serde::Deserialize;

const fn default_true() -> Option<bool> {
    Some(true)
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub global: Option<Global>,
    pub events: Map<String, Event>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Global {
    #[serde(default)]
    pub filters: Option<Vec<String>>,

    #[serde(default)]
    pub script_timeout: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(default)]
    pub message: String,

    #[serde(
        default,
        rename(deserialize = "next-watch-delay"),
        with = "humantime_serde"
    )]
    pub next_watch_delay: Option<Duration>,

    #[serde(default)]
    pub script: String,

    #[serde(default = "default_true", rename(deserialize = "script-wait"))]
    pub script_wait: Option<bool>,
}

impl Settings {
    pub fn new(config_file: &str) -> Result<Self> {
        let settings = Config::builder()
            .set_default("global.script_timeout", Some(20))?
            .add_source(config::File::new(config_file, FileFormat::Toml))
            .build()
            .map_err(|err| anyhow!("{err:#}"))
            .context("Failed to construct configurations")?;

        settings
            .try_deserialize()
            .map_err(|err| anyhow!("{err:#}"))
            .context("Failed to deserialize the entire configuration")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_settings_with_default_values() {
        // settings-1
        let settings = Settings::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/settings-1.toml"
        ))
        .unwrap();
        assert_eq!(settings.global.as_ref().unwrap().script_timeout, Some(20));
        assert_eq!(settings.events["event-1"].message, "regex-1");
        assert_eq!(settings.events["event-1"].next_watch_delay, None);
        assert_eq!(settings.events["event-1"].script, "script-1");
        assert!(settings.events["event-1"].script_wait.unwrap());

        // settings-2
        let settings = Settings::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/settings-2.toml"
        ))
        .unwrap();
        assert_eq!(settings.global.as_ref().unwrap().script_timeout, Some(20));
        assert_eq!(settings.events["event-2"].message, "regex-2");
        assert_eq!(
            settings.events["event-2"].next_watch_delay.unwrap(),
            Duration::from_secs(60)
        );
        assert_eq!(settings.events["event-2"].script, "script-2");
        assert!(settings.events["event-2"].script_wait.unwrap());

        // settings-3
        let settings = Settings::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/settings-3.toml"
        ))
        .unwrap();
        assert_eq!(settings.global.as_ref().unwrap().script_timeout, Some(20));
        assert_eq!(settings.events["event-3"].message, "regex-3");
        assert_eq!(settings.events["event-3"].next_watch_delay, None);
        assert_eq!(settings.events["event-3"].script, "script-3");
        assert!(settings.events["event-3"].script_wait.unwrap());

        // settings-4
        let settings = Settings::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/settings-4.toml"
        ))
        .unwrap();
        assert_eq!(settings.global.as_ref().unwrap().script_timeout, Some(10));
        assert_eq!(settings.events["event-4"].message, "regex-4");
        assert_eq!(
            settings.events["event-4"].next_watch_delay.unwrap(),
            Duration::from_secs(60)
        );
        assert_eq!(settings.events["event-4"].script, "script-4");
        assert!(!settings.events["event-4"].script_wait.unwrap());

        // settings-5
        let settings = Settings::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/settings-5.toml"
        ))
        .unwrap();
        assert_eq!(
            settings.global.as_ref().unwrap().filters.as_ref().unwrap(),
            &vec!["_TRANSPORT=kernel", "PRIORITY=4"]
        );
        assert_eq!(settings.global.as_ref().unwrap().script_timeout, Some(20));
        assert_eq!(
            settings.events["xhci_hcd-error"].message,
            "xhci_hcd 0000:04:00\\.0: WARN waiting for error on ep to be cleared"
        );
        assert_eq!(
            settings.events["xhci_hcd-error"].next_watch_delay.unwrap(),
            Duration::from_secs(60)
        );
        assert_eq!(
            settings.events["xhci_hcd-error"].script,
            "/usr/local/bin/xhci_hcd-rebind.sh"
        );
        assert!(settings.events["xhci_hcd-error"].script_wait.unwrap());
    }

    #[test]
    fn load_settings_example() {
        let _ = dbg!(
            Settings::new(concat!(env!("CARGO_MANIFEST_DIR"), "/journald-broker.toml"))
                .unwrap_err()
        );
    }
}
