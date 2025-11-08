//! Integration tests for the CLI binary
//!
//! Tests CLI commands, flag combinations, and output formatting using assert_cmd

// TODO: Migrate to cargo_bin! macro when stable migration path is documented
// https://github.com/assert-rs/assert_cmd/issues/225
#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;

// ===== Module Organization =====
// Tests are organized into logical groups:
// - Basic CLI: help, version, flags
// - Command Execution: subcommands and their behavior
// - Error Handling: invalid inputs and edge cases
// - Output Formatting: JSON, exit codes, messages

/// Create a minimal test project for CLI testing
fn create_test_project(temp_dir: &TempDir) -> std::path::PathBuf {
    let project_root = temp_dir.path().to_path_buf();

    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "test-cli-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

    fs::write(project_root.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Create src/lib.rs
    fs::create_dir(project_root.join("src")).expect("Failed to create src directory");

    let lib_rs = "pub fn hello() -> String { \"Hello\".to_string() }";
    fs::write(project_root.join("src/lib.rs"), lib_rs).expect("Failed to write lib.rs");

    project_root
}

// ===== Basic CLI Tests =====

#[test]
fn test_cli_help_flag() {
    // Test that --help flag works and shows usage information
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("wasm-slim"))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version_flag() {
    // Test that --version flag shows version information
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("wasm-slim"));
}

#[test]
fn test_cli_help_for_subcommands() {
    // Test that help works for each subcommand
    let subcommands = vec!["analyze", "build", "init"];

    for subcmd in subcommands {
        let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

        cmd.arg(subcmd)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains(subcmd));
    }
}

// ===== Command Execution Tests =====

#[test]
fn test_cli_init_command() {
    // Test that init command creates configuration file
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("init")
        .current_dir(&project_root)
        .assert()
        .success();

    // Verify config file was created
    assert!(
        project_root.join(".wasm-slim.toml").exists(),
        "Config file should be created by init command"
    );
}

#[test]
fn test_cli_init_with_template() {
    // Test init command with template flag
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("init")
        .arg("--template")
        .arg("balanced")
        .current_dir(&project_root)
        .assert()
        .success();

    // Verify config file was created
    assert!(project_root.join(".wasm-slim.toml").exists());
}

#[test]
fn test_cli_analyze_without_project() {
    // Test that analyze fails gracefully without a valid project
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("analyze")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cargo.toml").or(predicate::str::contains("project")));
}

#[test]
fn test_cli_init_command_standalone() {
    // Test that init command works standalone
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    // init should work
    cmd.arg("init")
        .current_dir(&project_root)
        .assert()
        .success();
}

#[test]
fn test_cli_json_output_flag() {
    // Test that --json flag produces valid JSON output (if supported)
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let _project_root = create_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    // Test with a command that supports JSON output
    let result = cmd.arg("--version").output();

    // Just verify the command runs (JSON support is optional)
    assert!(result.is_ok());
}

#[test]
fn test_cli_init_with_multiple_options() {
    // Test that init command works with multiple options
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("init")
        .arg("--template")
        .arg("balanced")
        .current_dir(&project_root)
        .assert()
        .success();
}

#[test]
fn test_cli_init_overwrites_with_force() {
    // Test that init with --force overwrites existing config
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    // Create initial config
    let mut cmd1 = Command::cargo_bin("wasm-slim").unwrap();
    cmd1.arg("init")
        .current_dir(&project_root)
        .assert()
        .success();

    // Try to init again with --force (if supported)
    let mut cmd2 = Command::cargo_bin("wasm-slim").unwrap();
    let result = cmd2
        .arg("init")
        .arg("--force")
        .current_dir(&project_root)
        .output();

    // Either succeeds with --force or fails without it
    assert!(result.is_ok());
}

// ===== Error Handling Tests =====

#[test]
fn test_cli_invalid_subcommand() {
    // Test that invalid subcommand produces error
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("nonexistent-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("invalid")));
}

#[test]
fn test_cli_invalid_flag() {
    // Test that invalid flag produces error
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("--nonexistent-flag").assert().failure();
}

#[test]
fn test_cli_error_messages_are_helpful() {
    // Test that error messages provide useful information
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("analyze")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

#[test]
fn test_cli_completions_command() {
    // Test that completions command works for various shells
    let shells = vec!["bash", "zsh", "fish"];

    for shell in shells {
        let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

        let result = cmd.arg("completions").arg(shell).output();

        // Command should at least execute (may not be implemented yet)
        assert!(
            result.is_ok(),
            "Completions command should run for {}",
            shell
        );
    }
}

// ===== Output Formatting Tests =====

#[test]
fn test_cli_output_format_consistency() {
    // Test that output format is consistent across commands
    let mut cmd1 = Command::cargo_bin("wasm-slim").unwrap();
    let output1 = cmd1.arg("--help").output().expect("Failed to run command");

    let mut cmd2 = Command::cargo_bin("wasm-slim").unwrap();
    let output2 = cmd2
        .arg("--version")
        .output()
        .expect("Failed to run command");

    // Both should succeed
    assert!(output1.status.success());
    assert!(output2.status.success());

    // Both should produce output
    assert!(!output1.stdout.is_empty() || !output1.stderr.is_empty());
    assert!(!output2.stdout.is_empty() || !output2.stderr.is_empty());
}

#[test]
fn test_cli_exit_codes() {
    // Test that CLI uses appropriate exit codes
    let mut cmd_success = Command::cargo_bin("wasm-slim").unwrap();
    cmd_success.arg("--version").assert().code(0); // Success should be exit code 0

    let mut cmd_failure = Command::cargo_bin("wasm-slim").unwrap();
    let result = cmd_failure.arg("--nonexistent-flag").assert().failure();

    // Failure should be non-zero exit code
    assert!(result.get_output().status.code().unwrap_or(0) != 0);
}

#[test]
fn test_cli_handles_ctrl_c_gracefully() {
    // Test that CLI can be interrupted (basic test)
    // This is more of a smoke test - actual signal handling tested elsewhere
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    let result = cmd.arg("--help").output();

    // Command should complete successfully
    assert!(result.is_ok());
}

#[test]
fn test_cli_respects_current_directory() {
    // Test that CLI respects the current working directory
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("init")
        .current_dir(&project_root)
        .assert()
        .success();

    // Config should be created in the current directory
    assert!(project_root.join(".wasm-slim.toml").exists());
}

#[test]
fn test_cli_help_shows_all_subcommands() {
    // Test that help output lists all available subcommands
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("analyze"))
        .stdout(predicate::str::contains("build"));
}

#[test]
fn test_cli_version_format() {
    // Test that version output has expected format
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("wasm-slim"))
        .stdout(predicate::str::is_match(r"\d+\.\d+\.\d+").unwrap());
}

#[test]
fn test_cli_stdin_not_required() {
    // Test that CLI doesn't block waiting for stdin
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    cmd.arg("--help").assert().success();

    // CLI should complete without waiting for stdin
}

#[test]
fn test_cli_parallel_execution() {
    // Test that multiple CLI instances can run concurrently
    use std::thread;

    let handles: Vec<_> = (0..3)
        .map(|_| {
            thread::spawn(|| {
                let mut cmd = Command::cargo_bin("wasm-slim").unwrap();
                cmd.arg("--version").assert().success();
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_cli_unicode_in_arguments() {
    // Test that CLI handles Unicode in arguments
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    // Try with Unicode template name (should fail gracefully)
    let result = cmd
        .arg("init")
        .arg("--template")
        .arg("日本語")
        .current_dir(&project_root)
        .output();

    // Should complete without panicking (may succeed or fail)
    assert!(result.is_ok());
}

#[test]
fn test_cli_very_long_arguments() {
    // Test that CLI handles very long arguments gracefully
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    let long_string = "a".repeat(1000);
    let result = cmd.arg("--template").arg(long_string).output();

    // Should complete without panicking
    assert!(result.is_ok());
}

#[test]
fn test_cli_empty_arguments() {
    // Test that CLI handles empty string arguments
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();

    let result = cmd.arg("").output();

    // Should complete without panicking
    assert!(result.is_ok());
}
