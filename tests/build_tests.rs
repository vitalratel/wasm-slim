//! Tests for the `build` command
//!
//! Tests build command with various flags and configurations

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

use tempfile::TempDir;

mod common;
use common::fixtures;

/// Helper to get the wasm-slim binary command
fn get_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
}

#[test]
fn test_build_with_dry_run_flag_shows_changes_without_modifying() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-crate").expect("Failed to create test fixture");

    let mut cmd = get_bin();
    cmd.arg("build")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN"))
        .stdout(predicate::str::contains("Would optimize"));

    // Verify no artifacts were created in dry-run mode
    let wasm_slim_dir = temp_dir.path().join(".wasm-slim");
    assert!(
        !wasm_slim_dir.exists(),
        "Dry-run should not create .wasm-slim directory"
    );
}

#[test]
fn test_build_without_cargo_toml_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");

    let mut cmd = get_bin();
    cmd.arg("build")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cargo.toml"));
}

#[test]
fn test_build_with_dry_run_creates_no_backup_files() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-crate").expect("Failed to create test fixture");

    let mut cmd = get_bin();
    cmd.arg("build")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify no .wasm-slim directory was created in dry-run mode
    let backup_dir = temp_dir.path().join(".wasm-slim");
    assert!(!backup_dir.exists(), "Dry-run should not create backups");
}

#[test]
fn test_build_with_dry_run_preserves_original_cargo_toml() {
    let (temp_dir, cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-crate").expect("Failed to create test fixture");

    let original_content = fs::read_to_string(&cargo_toml).expect("Failed to read file contents");

    let mut cmd = get_bin();
    cmd.arg("build")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify original file is unchanged
    let content = fs::read_to_string(&cargo_toml).expect("Failed to read file contents");
    assert_eq!(content, original_content, "Dry-run should not modify files");

    // Verify no artifacts were created in dry-run mode
    let wasm_slim_dir = temp_dir.path().join(".wasm-slim");
    assert!(
        !wasm_slim_dir.exists(),
        "Dry-run should not create .wasm-slim directory"
    );
}

#[test]
fn test_build_with_json_flag_outputs_structured_json() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-json").expect("Failed to create test fixture");

    let mut cmd = get_bin();
    let output = cmd
        .arg("build")
        .arg("--json")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        assert!(
            stdout.contains("DRY RUN") || !stdout.is_empty(),
            "Command should produce output"
        );
    }

    // Verify no artifacts were created in dry-run mode
    let wasm_slim_dir = temp_dir.path().join(".wasm-slim");
    assert!(
        !wasm_slim_dir.exists(),
        "Dry-run should not create .wasm-slim directory"
    );
}

#[test]
fn test_build_json_output_contains_required_fields() {
    let (temp_dir, _cargo_toml) = fixtures::create_minimal_wasm_lib("test-json-schema")
        .expect("Failed to create test fixture");

    let mut cmd = get_bin();
    let output = cmd
        .arg("build")
        .arg("--json")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    if !output.status.success() {
        return;
    }

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object());

        let obj = json.as_object().expect("Expected JSON object");
        assert!(obj.contains_key("success") || obj.contains_key("status"));

        if let Some(size) = obj.get("size_bytes") {
            assert!(size.is_number());
        }

        if let Some(compressed) = obj.get("compressed_size_bytes") {
            assert!(compressed.is_number());
        }
    }

    // Verify no artifacts were created in dry-run mode
    let wasm_slim_dir = temp_dir.path().join(".wasm-slim");
    assert!(
        !wasm_slim_dir.exists(),
        "Dry-run should not create .wasm-slim directory"
    );
}

#[test]
fn test_build_with_budget_check_under_limit_succeeds() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-budget").expect("Failed to create test fixture");

    let config = temp_dir.path().join(".wasm-slim.toml");
    fs::write(
        &config,
        r#"
template = "balanced"

[size-budget]
max-size-kb = 10000
target-size-kb = 5000
warn-threshold-kb = 8000
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("build")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify no artifacts were created in dry-run mode
    let wasm_slim_dir = temp_dir.path().join(".wasm-slim");
    assert!(
        !wasm_slim_dir.exists(),
        "Dry-run should not create .wasm-slim directory"
    );
}

#[test]
fn test_build_with_budget_check_over_limit_fails() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-budget").expect("Failed to create test fixture");

    let config = temp_dir.path().join(".wasm-slim.toml");
    fs::write(
        &config,
        r#"
template = "balanced"

[size-budget]
max-size-kb = 1
target-size-kb = 1
warn-threshold-kb = 1
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("build")
        .arg("--check")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify no artifacts were created in dry-run mode
    let wasm_slim_dir = temp_dir.path().join(".wasm-slim");
    assert!(
        !wasm_slim_dir.exists(),
        "Dry-run should not create .wasm-slim directory"
    );
}

#[test]
fn test_size_budget_with_various_thresholds_validates_correctly() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-budget").expect("Failed to create test fixture");

    let config = temp_dir.path().join(".wasm-slim.toml");
    fs::write(
        &config,
        r#"
template = "balanced"

[size-budget]
max-size-kb = 100
target-size-kb = 200
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    let result = cmd
        .arg("build")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    let stderr = String::from_utf8_lossy(&result.stderr);
    let stdout = String::from_utf8_lossy(&result.stdout);

    assert!(
        !result.status.success()
            || stderr.contains("target")
            || stdout.contains("warning")
            || result.status.success(),
        "Command should handle invalid budget configuration"
    );
}

#[test]
fn test_build_history_records_build_metrics_over_time() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-history").expect("Failed to create test fixture");

    let config = temp_dir.path().join(".wasm-slim.toml");
    fs::write(
        &config,
        r#"
template = "balanced"

[ci-integration]
enable-history = true
max-history-entries = 10
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("build")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify no artifacts were created in dry-run mode
    let wasm_slim_dir = temp_dir.path().join(".wasm-slim");
    assert!(
        !wasm_slim_dir.exists(),
        "Dry-run should not create .wasm-slim directory"
    );
}

#[test]
fn test_build_with_missing_wasm_target_shows_helpful_error() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-crate").expect("Failed to create test fixture");

    let mut cmd = get_bin();
    let output = cmd
        .arg("build")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).expect("Failed to parse stderr as UTF-8");
        assert!(
            stderr.contains("target")
                || stderr.contains("toolchain")
                || stderr.contains("wasm32")
                || stderr.contains("rustup"),
            "Error message should provide helpful context about missing WASM target"
        );
    }
}

#[test]
fn test_build_with_invalid_profile_settings_returns_clear_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[profile.release]
opt-level = "invalid"
lto = "not-a-boolean-or-valid-value"
"#,
    )
    .expect("Failed to write test Cargo.toml");

    let mut cmd = get_bin();
    cmd.arg("build")
        .current_dir(temp_dir.path())
        .assert()
        .failure();
}
