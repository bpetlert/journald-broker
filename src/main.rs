mod monitor;
mod script;
mod settings;

use std::{io, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use mimalloc::MiMalloc;
use tracing::debug;
use tracing_subscriber::EnvFilter;

use crate::{monitor::Monitor, settings::Settings};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Configuration file
    #[arg(short, long, default_value = "/etc/systemd/journald-broker.toml")]
    pub config_file: PathBuf,
}

fn main() -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or(EnvFilter::try_new("journald_broker=info")?);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .with_writer(io::stderr)
        .try_init()
        .map_err(|err| anyhow!("{err:#}"))
        .context("Failed to initialize tracing subscriber")?;

    let arguments = Arguments::parse();
    debug!("Run with {:?}", arguments);

    let settings = Settings::new(arguments.config_file.to_str().unwrap())
        .context("Failed to load settings")?;
    debug!("{settings:#?}");

    Monitor::new(settings)
        .context("Could not create journal watcher")?
        .watch()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, FromArgMatches};

    #[test]
    fn test_args() {
        // Default arguments
        let args = Arguments::from_arg_matches(
            &Arguments::command().get_matches_from(vec![env!("CARGO_CRATE_NAME")]),
        )
        .expect("Paring argument");
        assert_eq!(
            args.config_file,
            PathBuf::from("/etc/systemd/journald-broker.toml")
        );

        // Full long arguments
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "--config-file",
            "/etc/systemd/journald-broker2.toml",
        ]))
        .expect("Paring argument");
        assert_eq!(
            args.config_file,
            PathBuf::from("/etc/systemd/journald-broker2.toml")
        );

        // Full short arguments
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "-c",
            "/etc/systemd/journald-broker3.toml",
        ]))
        .expect("Paring argument");
        assert_eq!(
            args.config_file,
            PathBuf::from("/etc/systemd/journald-broker3.toml")
        );
    }
}
