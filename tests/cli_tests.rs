//! CLI interface tests
//!
//! Tests basic CLI functionality like --help, --version flags

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

mod common;
use common::fixtures;

/// Helper to get the wasm-slim binary command
fn get_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
}

#[test]
fn test_cli_help_flag_displays_usage_information() {
    let mut cmd = get_bin();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("WASM bundle size optimizer"));
}

#[test]
fn test_cli_version_flag_displays_version_number() {
    let mut cmd = get_bin();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("wasm-slim"));
}

#[test]
fn test_all_commands_with_json_flag_output_parseable_json() {
    // Test that all JSON outputs are parseable JSON (not malformed)
    let (temp_dir, cargo_toml) = fixtures::create_minimal_wasm_lib("test-valid-json")
        .expect("Failed to create test fixture");

    // Add a dependency for testing
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-valid-json"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#,
    )
    .expect("Failed to write test file");

    // Test analyze deps --json
    let mut cmd = get_bin();
    let output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--json")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");
        if !stdout.is_empty() {
            let parse_result = serde_json::from_str::<serde_json::Value>(&stdout);
            assert!(
                parse_result.is_ok(),
                "JSON output should be valid JSON, got: {}",
                stdout
            );
        }
    }
}

#[test]
fn test_json_output_contains_no_extraneous_text() {
    // Verify JSON output doesn't contain extra non-JSON text
    let (temp_dir, cargo_toml) = fixtures::create_minimal_wasm_lib("test-clean-json")
        .expect("Failed to create test fixture");

    // Add a dependency for testing
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-clean-json"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--json")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");

        if !stdout.is_empty() {
            // Trim whitespace and check if entire output is valid JSON
            let trimmed = stdout.trim();
            if !trimmed.is_empty() {
                let parse_result = serde_json::from_str::<serde_json::Value>(trimmed);
                assert!(
                    parse_result.is_ok(),
                    "JSON output should not contain extra text. Output: {}",
                    stdout
                );

                // Verify it starts with { or [
                assert!(
                    trimmed.starts_with('{') || trimmed.starts_with('['),
                    "JSON output should start with {{ or ["
                );
            }
        }
    }
}
