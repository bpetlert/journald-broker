mod common;

use std::{
    io::{BufReader, Seek},
    path::Path,
};

use journald_broker::script::{EnvVar, Script};

use crate::common::log_check::{next_log, setup_log};

// Missing JNB_JSON environment variable
#[test]
fn missing_jnb_json() {
    let mut log_file = setup_log();
    log_file.seek(std::io::SeekFrom::End(0)).unwrap();
    let mut reader = BufReader::new(log_file);

    let script_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests",
        "/scripts",
        "/script-execute-test.sh"
    ));

    let mut script: Script = Script::new(script_path, Some(20), false).unwrap();

    // Add JNB_MESSAGE env var
    let msg_env = EnvVar::Message("SOME ERROR".to_string());
    script.add_env(msg_env).unwrap();
    let ret = script.run();
    assert!(ret.is_ok(), "Missing JNB_JSON environment variable");
    assert_eq!(
        next_log(&mut reader),
        format!(
            " INFO journald_broker::script: Execute `{}`\n",
            script_path.display()
        )
    );
    assert_eq!(
        next_log(&mut reader),
        format!(
            " INFO journald_broker::script: Finished `{}`, exit status: 52\n",
            script_path.display()
        )
    );
}
