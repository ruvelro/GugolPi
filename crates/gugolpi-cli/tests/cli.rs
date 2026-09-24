//! Tests de integración de la CLI: códigos de salida y JSON válido.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use assert_cmd::Command;
use predicates::prelude::*;

fn gugolpi() -> Command {
    Command::cargo_bin("gugolpi").expect("binario gugolpi")
}

#[test]
fn sysinfo_json_is_valid() {
    let output = gugolpi().args(["sysinfo", "--json"]).output().expect("run");
    assert!(output.status.success());
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert!(doc["system"]["logical_cpus"].as_u64().unwrap_or(0) >= 1);
    assert_eq!(doc["tool"]["name"], "gugolpi");
}

#[test]
fn radical_small_run_produces_sealed_json() {
    let output = gugolpi()
        .args(["radical", "--size", "200000", "--json"])
        .output()
        .expect("run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: gugolpi_core::RunResult = serde_json::from_slice(&output.stdout).expect("json");
    assert!(result.integrity_ok());
    assert!(result.verification.passed());
    assert_eq!(result.test.config.size.value, 200_000);
    assert!(!result.official, "un N libre no puntúa");
}

#[test]
fn radical_multi_with_repeat_writes_files() {
    let dir = tempdir();
    gugolpi()
        .args([
            "radical",
            "--size",
            "300000",
            "--mode",
            "multi",
            "--threads",
            "2",
            "--repeat",
            "2",
            "--quiet",
            "--out",
        ])
        .arg(&dir)
        .assert()
        .success();
    let files: Vec<_> = std::fs::read_dir(&dir).expect("dir").flatten().collect();
    assert_eq!(files.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unknown_size_exits_with_usage_code() {
    gugolpi()
        .args(["radical", "--size", "abc"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("abc"));
}

#[test]
fn bad_flag_exits_with_usage_code() {
    gugolpi().args(["radical", "--nope"]).assert().code(1);
}

#[test]
fn help_exits_zero() {
    gugolpi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("radical"));
}

fn tempdir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("gugolpi-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}
