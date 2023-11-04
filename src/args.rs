use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Location under which to look for configuration files
    #[arg(short = 'C', long, default_value = "/etc/journald-broker.d")]
    pub config_dir: PathBuf,

    /// Run program using a specificed configuration file
    #[arg(short = 'c', long)]
    pub config_file: Option<PathBuf>,
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
        assert_eq!(args.config_dir, PathBuf::from("/etc/journald-broker.d"));
        assert_eq!(args.config_file, None);

        // Config dir
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "--config-dir",
            "/etc/journald-broker2.d",
        ]))
        .expect("Paring argument");
        assert_eq!(args.config_dir, PathBuf::from("/etc/journald-broker2.d"));

        // Config dir
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "-C",
            "/etc/journald-broker2.d",
        ]))
        .expect("Paring argument");
        assert_eq!(args.config_dir, PathBuf::from("/etc/journald-broker2.d"));

        // Config file
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "--config-file",
            "/etc/journald-broker.d/00-some-event.conf",
        ]))
        .expect("Paring argument");
        assert_eq!(
            args.config_file,
            Some(PathBuf::from("/etc/journald-broker.d/00-some-event.conf"))
        );

        // Config file
        let args = Arguments::from_arg_matches(&Arguments::command().get_matches_from(vec![
            env!("CARGO_CRATE_NAME"),
            "-c",
            "/etc/journald-broker.d/00-some-event.conf",
        ]))
        .expect("Paring argument");
        assert_eq!(
            args.config_file,
            Some(PathBuf::from("/etc/journald-broker.d/00-some-event.conf"))
        );
    }
}
