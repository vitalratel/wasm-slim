//! Tests for the `init` command
//!
//! Tests config file creation with different templates

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to get the wasm-slim binary command
fn get_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
}

#[test]
fn test_init_with_default_template_creates_config_file() {
    // Phase 7: Test init command with default (balanced) template
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");

    let mut cmd = get_bin();
    cmd.arg("init")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("balanced"))
        .stdout(predicate::str::contains("Created .wasm-slim.toml"));

    // Verify config file was created
    let config_path = temp_dir.path().join(".wasm-slim.toml");
    assert!(config_path.exists());

    // Verify config content
    let config_content = fs::read_to_string(&config_path).expect("Failed to read file contents");
    assert!(config_content.contains("template = \"balanced\""));
    assert!(config_content.contains("opt-level"));
}

#[test]
fn test_init_with_minimal_template_creates_minimal_config() {
    // Phase 7: Test init command with minimal template
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");

    let mut cmd = get_bin();
    cmd.arg("init")
        .arg("--template")
        .arg("minimal")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("minimal"))
        .stdout(predicate::str::contains("Maximum size reduction"));

    // Verify config file was created with correct template
    let config_path = temp_dir.path().join(".wasm-slim.toml");
    assert!(config_path.exists());

    let config_content = fs::read_to_string(&config_path).expect("Failed to read file contents");
    assert!(config_content.contains("template = \"minimal\""));
    assert!(config_content.contains("opt-level = \"z\""));
}

#[test]
fn test_init_with_framework_template_creates_framework_config() {
    // Phase 7: Test init command with Yew framework template
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");

    let mut cmd = get_bin();
    cmd.arg("init")
        .arg("--template")
        .arg("yew")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("yew"))
        .stdout(predicate::str::contains("Yew framework"));

    // Verify config file
    let config_path = temp_dir.path().join(".wasm-slim.toml");
    assert!(config_path.exists());

    let config_content = fs::read_to_string(&config_path).expect("Failed to read file contents");
    assert!(config_content.contains("template = \"yew\""));
}

#[test]
fn test_init_when_config_exists_returns_error() {
    // Phase 7: Test init command when config already exists
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");

    // Create config file first
    let config_path = temp_dir.path().join(".wasm-slim.toml");
    fs::write(&config_path, "template = \"custom\"").expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("init")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"));
}

#[test]
fn test_init_with_invalid_template_returns_error() {
    // Phase 7: Test init command with invalid template
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");

    let mut cmd = get_bin();
    cmd.arg("init")
        .arg("--template")
        .arg("invalid-template")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
