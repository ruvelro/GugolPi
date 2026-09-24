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
    let dir = tempdir("radical");
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

fn tempdir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("gugolpi-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn pi_16k_verifies_and_reports_loops() {
    let output = gugolpi()
        .args(["pi", "--size", "16K", "--json"])
        .output()
        .expect("run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: gugolpi_core::RunResult = serde_json::from_slice(&output.stdout).expect("json");
    assert!(result.verification.passed());
    assert_eq!(result.timing.loops.len(), 13);
    assert!(result.official);
}

#[test]
fn zeta_free_n_is_unverified_and_unofficial() {
    let output = gugolpi()
        .args(["zeta", "--size", "2000000", "--json"])
        .output()
        .expect("run");
    assert!(output.status.success());
    let result: gugolpi_core::RunResult = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(
        result.verification.status,
        gugolpi_core::result::VerificationStatus::Unverified
    );
    assert!(!result.official);
}

#[test]
fn scaling_prints_a_table() {
    gugolpi()
        .args(["radical", "--size", "200000", "--scaling"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Escalado").and(predicate::str::contains("Speedup")));
}

#[test]
fn suite_from_file_writes_csv_and_compare_reads_it() {
    let dir = tempdir("suite");
    std::fs::create_dir_all(&dir).expect("dir");
    let suite = dir.join("mini.toml");
    std::fs::write(
        &suite,
        "[suite]\nname = \"mini\"\nrepeat = 2\n\n[[test]]\nmodule = \"radical\"\nsize = \"100000\"\n\n[[test]]\nmodule = \"zeta\"\nsize = \"100000\"\nmode = \"multi\"\nthreads = \"2\"\n",
    )
    .expect("write");
    let out = dir.join("out");
    gugolpi()
        .arg("suite")
        .arg(&suite)
        .arg("--out")
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::contains("Suite «mini»"));
    let csv = std::fs::read_to_string(out.join("summary.csv")).expect("csv");
    assert_eq!(
        csv.lines().count(),
        1 + 4,
        "cabecera + 2 tests × 2 repeticiones"
    );
    let jsons = std::fs::read_dir(&out)
        .expect("dir")
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .count();
    assert_eq!(jsons, 4);

    // compare: los runs de tamaño libre no son oficiales → ocultos salvo --include-unofficial.
    gugolpi()
        .arg("compare")
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::contains("no oficiales ocultos"));
    gugolpi()
        .arg("compare")
        .arg(&out)
        .arg("--include-unofficial")
        .arg("--csv")
        .assert()
        .success()
        .stdout(predicate::str::contains("radical 100000 single"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn suite_dry_run_lists_jobs() {
    gugolpi()
        .args(["suite", "trio", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pi 1M single").and(predicate::str::contains("zeta 10G")));
}

#[test]
fn unknown_suite_exits_with_usage_code() {
    gugolpi().args(["suite", "nope"]).assert().code(1);
}
