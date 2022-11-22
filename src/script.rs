use std::{
    collections::HashMap,
    fmt,
    path::PathBuf,
    process::Command,
    sync::mpsc::{channel, RecvError, Sender},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use tracing::{error, info, warn};
use wait_timeout::ChildExt;

#[derive(Debug, Clone)]
pub enum EnvVar {
    Message(String),
    Json(String),

    #[allow(dead_code)]
    Custom {
        key: String,
        value: String,
    },
}

impl fmt::Display for EnvVar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EnvVar::Message(_) => write!(f, "JNB_MESSAGE"),
            EnvVar::Json(_) => write!(f, "JNB_JSON"),
            EnvVar::Custom { key, value: _ } => write!(f, "JNB_{key}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Script {
    path: PathBuf,
    envs: HashMap<String, String>,
    timeout: Option<u64>,
}

impl Script {
    pub fn new(path: PathBuf, timeout: Option<u64>) -> Self {
        Self {
            path,
            envs: HashMap::new(),
            timeout,
        }
    }

    pub fn add_env(&mut self, env_var: EnvVar) -> Result<()> {
        let value = match &env_var {
            EnvVar::Message(value) | EnvVar::Json(value) | EnvVar::Custom { key: _, value } => {
                value
            }
        };

        self.envs.insert(env_var.to_string(), value.to_string());
        Ok(())
    }

    pub fn run(self) -> Result<()> {
        info!("Execute `{}`", &self.path.display());
        let mut process = match Command::new(&self.path)
            .envs(self.envs)
            .spawn()
            .with_context(|| format!("Failed to execute `{}`", &self.path.display()))
        {
            Ok(process) => process,
            Err(err) => bail!("{err:#}"),
        };

        if let Some(timeout) = self.timeout {
            match process
                .wait_timeout(Duration::from_secs(timeout))
                .context("Failed to wait until child process to finish or timeout")?
            {
                Some(exit_code) => {
                    info!("Finished `{}`, {exit_code}", &self.path.display());
                    return Ok(());
                }
                None => {
                    process.kill()?;
                    let exit_code = process.wait()?;
                    bail!(
                        "Execute timeout `{}`, >= {timeout}, {exit_code}",
                        &self.path.display()
                    );
                }
            }
        } else {
            // Not wait for child process to finish, use thread to wait for child process' return code.
            thread::spawn(move || {
                match process
                    .wait()
                    .context("Failed to wait until child process to finish")
                {
                    Ok(exit_code) => info!("Finished {}, {exit_code}", &self.path.display()),
                    Err(err) => warn!("{err:#}"),
                }
            });
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Launcher {
    tx: Sender<Box<Script>>,
}

impl Launcher {
    pub fn new() -> Launcher {
        let (tx, rx) = channel::<Box<Script>>();

        thread::spawn(move || loop {
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
        });

        Launcher { tx }
    }

    /// Add a script to execute queue
    pub fn add(&self, script: Script) -> Result<()> {
        self.tx
            .send(Box::new(script))
            .context("Failed to send a script to launcher channel")?;
        Ok(())
    }
}
