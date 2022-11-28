mod common;

use std::{
    io::{BufReader, Seek},
    path::Path,
    thread,
};

use journald_broker::script::{EnvVar, Script};

use crate::common::log_check::{next_log, setup_log};

// Script normal exit.
#[test]
fn script_normal_exit() {
    let mut log_file = setup_log();
    log_file.seek(std::io::SeekFrom::End(0)).unwrap();
    let mut reader = BufReader::new(log_file);

    let script_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests",
        "/scripts",
        "/script-execute-test.sh"
    ));

    let mut script: Script = Script::new(script_path, Some(5), false).unwrap();

    script
        .add_env(EnvVar::Message("SOME ERROR".to_string()))
        .unwrap();

    script
        .add_env(EnvVar::Json("SOME JSON".to_string()))
        .unwrap();

    script
        .add_env(EnvVar::Custom {
            key: "SCRIPT_TEST_CASE".to_string(),
            value: "3".to_string(),
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
    thread::sleep(std::time::Duration::from_secs(3));
    assert!(ret.is_ok(), "Script normal exit");
    assert_eq!(
        next_log(&mut reader),
        format!(
            " INFO journald_broker::script: Finished `{}`, exit status: 0\n",
            script_path.display()
        )
    );
}
