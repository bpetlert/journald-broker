use std::{io, process::ExitCode};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use mimalloc::MiMalloc;
use tracing::{debug, error};
use tracing_subscriber::EnvFilter;

use journald_broker::{args::Arguments, monitor::Monitor, settings::Settings};
use walkdir::WalkDir;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn run() -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or(EnvFilter::try_new("journald_broker=info")?);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .with_writer(io::stderr)
        .try_init()
        .map_err(|err| anyhow!("{err:#}"))
        .context("Failed to initialize tracing subscriber")?;

    // Panic to log
    std::panic::set_hook(Box::new(|panic_info| {
        let (filename, line) = panic_info
            .location()
            .map(|loc| (loc.file(), loc.line()))
            .unwrap_or(("<unknown>", 0));

        let cause = match (
            panic_info.payload().downcast_ref::<&str>(),
            panic_info.payload().downcast_ref::<String>(),
        ) {
            (Some(s), _) => *s,
            (_, Some(s)) => s,
            (None, None) => "<unknown cause>",
        };

        error!("A panic occurred at {filename}:{line}: {cause}");
    }));

    let arguments = Arguments::parse();
    debug!("Run with {:?}", arguments);

    let mut settings = Settings::new()?;

    if let Some(config_file) = arguments.config_file {
        // Load single configuration file
        settings
            .read(config_file.to_str().unwrap())
            .context("Failed to load settings")?;
    } else {
        // Load multiple configuration files
        for entry in WalkDir::new(arguments.config_dir)
            .min_depth(1)
            .max_depth(1)
            .follow_links(false)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter()
            .filter_entry(|e| {
                e.file_type().is_file()
                    && e.file_name()
                        .to_str()
                        .map(|s| s.ends_with(".conf"))
                        .unwrap_or(false)
            })
            .filter_map(|e| e.ok())
        {
            debug!("Load configuration file '{}'", entry.path().display());
            settings.read(entry.path().to_str().unwrap())?;
        }
    }

    debug!("{settings:#?}");

    Monitor::new(settings)
        .context("Could not create journal watcher")?
        .watch()
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        error!("{err:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
