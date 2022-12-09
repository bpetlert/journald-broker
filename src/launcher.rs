use std::{
    sync::mpsc::{channel, RecvError, Sender},
    thread,
};

use anyhow::{Context, Result};
use tracing::{error, warn};

use crate::script::Script;

#[derive(Debug)]
pub struct Launcher {
    tx: Sender<Box<Script>>,
}

impl Launcher {
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel::<Box<Script>>();

        thread::Builder::new()
            .name("script launcher".to_string())
            .spawn(move || loop {
                match rx.recv() {
                    Ok(script) => {
                        if let Err(err) = script.run() {
                            warn!("{err:#}");
                        }
                    }
                    Err(RecvError {}) => {
                        error!("Failed to receive script");
                    }
                };
            })
            .context("Could not create script launcher thread")?;

        Ok(Launcher { tx })
    }

    /// Add a script to execute queue
    pub fn add(&self, script: Script) -> Result<()> {
        self.tx
            .send(Box::new(script))
            .context("Failed to send a script to launcher channel")?;
        Ok(())
    }
}
