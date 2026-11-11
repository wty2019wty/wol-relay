//! Checks the structured log records the binary emits on startup failures.

use std::process::Command;

use serde_json::Value;

/// Runs the binary with JSON logs and returns its exit code and log records.
fn run(args: &[&str]) -> (i32, Vec<Value>) {
    let output = Command::new(env!("CARGO_BIN_EXE_wol-relay"))
        .args(["-F", "json"])
        .args(args)
        .env_remove("RUST_LOG")
        .env_remove("WOL_RELAY_TARGETS")
        .env_remove("WOL_RELAY_LOG_LEVEL")
        .output()
        .unwrap();

    let records = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("every log line is JSON"))
        .collect();

    (output.status.code().unwrap(), records)
}

#[test]
fn version_flag_reports_the_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_wol-relay"))
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        format!("wol-relay {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unknown_interface_logs_a_structured_resolve_failure() {
    let (code, records) = run(&["no-such-interface-xyz"]);

    assert_eq!(code, 1);
    let record = &records[0];
    assert_eq!(record["level"], "error");
    assert_eq!(record["message"], "failed to resolve argument");
    assert_eq!(record["argument"], "no-such-interface-xyz");
    assert!(record["error"].as_str().unwrap().contains("not found"));
}

#[test]
fn ipv6_cidr_logs_a_structured_resolve_failure() {
    let (code, records) = run(&["::1/128"]);

    assert_eq!(code, 1);
    let record = &records[0];
    assert_eq!(record["level"], "error");
    assert_eq!(record["message"], "failed to resolve argument");
    assert_eq!(record["argument"], "::1/128");
    assert!(
        record["error"]
            .as_str()
            .unwrap()
            .contains("IPv6 not supported")
    );
}
