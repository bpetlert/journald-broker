mod common;

use std::{
    io::{BufReader, Seek},
    path::Path,
};

use journald_broker::script::{EnvVar, Script};

use crate::common::log_check::{next_log, setup_log};

// Script execution timeout.
#[test]
fn script_execution_timeout() {
    let mut log_file = setup_log();
    log_file.seek(std::io::SeekFrom::End(0)).unwrap();
    let mut reader = BufReader::new(log_file);

    let script_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests",
        "/scripts",
        "/script-execute-test.sh"
    ));

    let mut script: Script = Script::new(script_path, Some(2), false).unwrap();

    script
        .add_env(EnvVar::Message("SOME ERROR".to_string()))
        .unwrap();

    script
        .add_env(EnvVar::Json("SOME JSON".to_string()))
        .unwrap();

    script
        .add_env(EnvVar::Custom {
            key: "SCRIPT_TEST_CASE".to_string(),
            value: "2".to_string(),
        })
        .unwrap();

    let ret = script.run();
    assert_eq!(
        next_log(&mut reader),
        format!(
            " INFO journald_broker::script: Execute `{}`\n",
            script_path.display()
        )
    );
    assert!(ret.is_err(), "Script execution timeout");
    assert_eq!(
        format!("{}", ret.unwrap_err().root_cause()),
        format!(
            "Execute timeout `{}`, >= 2 seconds, signal: 9 (SIGKILL)",
            script_path.display()
        )
    );
    assert_eq!(next_log(&mut reader), "");
}
