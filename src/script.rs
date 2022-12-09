use std::{
    collections::HashMap,
    fmt,
    os::unix::prelude::MetadataExt,
    path::{Path, PathBuf},
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
    pub fn new(path: &Path, timeout: Option<u64>, validate: bool) -> Result<Self> {
        if validate {
            Script::validate_script(path)?;
        }

        Ok(Self {
            path: path.to_path_buf(),
            envs: HashMap::new(),
            timeout,
        })
    }

    /// Verify if a script is owned by root and exectable.
    fn validate_script(path: &Path) -> Result<()> {
        let metadata = path
            .metadata()
            .with_context(|| format!("Could not get metadata of `{}`", path.display()))?;

        if metadata.is_dir() {
            bail!("`{}` is a directory.", path.display());
        }

        if metadata.uid() != 0 {
            bail!("`{}` is not owned by uid 0", path.display());
        }

        if metadata.gid() != 0 {
            bail!("`{}` is not owned by gid 0", path.display());
        }

        // has at least 500 for file mode
        if metadata.mode() & 0o500 != 0o500 {
            bail!("`{}` is not executable.", path.display());
        }

        Ok(())
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
                        "Execute timeout `{}`, >= {timeout} seconds, {exit_code}",
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
                    Ok(exit_code) => info!("Finished `{}`, {exit_code}", &self.path.display()),
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

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::prelude::OpenOptionsExt};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_validate_script() {
        // Test executable-root-script
        assert!(Script::validate_script(Path::new("/usr/bin/bash")).is_ok());

        let temp_dir = TempDir::new().unwrap();
        assert!(temp_dir.path().to_owned().exists());

        // Test executable-non-root-script
        let exec_non_root = temp_dir.path().join("executable-non-root-script");
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o500)
            .open(&exec_non_root)
            .unwrap();
        assert!(Script::validate_script(Path::new(&exec_non_root)).is_err());

        // Test non-executable-non-root-script
        let non_exec_non_root = temp_dir.path().join("non-executable-non-root-script");
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o400)
            .open(&non_exec_non_root)
            .unwrap();
        assert!(Script::validate_script(Path::new(&non_exec_non_root)).is_err());
    }
}
