mod common;

use std::{
    io::{BufReader, Seek},
    path::Path,
};

use journald_broker::script::{EnvVar, Script};

use crate::common::log_check::{next_log, setup_log};

// Script is not exist.
#[test]
fn script_is_not_exist() {
    let mut log_file = setup_log();
    log_file.seek(std::io::SeekFrom::End(0)).unwrap();
    let mut reader = BufReader::new(log_file);

    let script_path = Path::new("/tmp/not-exist-script-nowait.sh");

    let mut script: Script = Script::new(script_path, Some(20), false).unwrap();

    script
        .add_env(EnvVar::Message("SOME ERROR".to_string()))
        .unwrap();

    script
        .add_env(EnvVar::Json("SOME JSON".to_string()))
        .unwrap();

    let ret = script.run();
    assert!(ret.is_err(), "Script is not exist");

    assert_eq!(
        next_log(&mut reader),
        format!(
            " INFO journald_broker::script: Execute `{}`\n",
            script_path.display()
        )
    );

    assert_eq!(
        format!("{}", ret.unwrap_err().root_cause()),
        format!(
            "Failed to execute `{}`: No such file or directory (os error 2)",
            script_path.display()
        )
    );

    assert_eq!(next_log(&mut reader), "");
}
