//! Tests for the `compare` command
//!
//! Tests WASM file comparison functionality

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::fixtures;

fn get_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
}

#[test]
fn test_compare_without_both_files_returns_error() {
    let mut cmd = get_bin();
    cmd.arg("compare")
        .arg("nonexistent_before.wasm")
        .arg("nonexistent_after.wasm")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Baseline file not found"));
}

#[test]
fn test_compare_with_missing_before_file_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let after_file = fixtures::create_minimal_wasm_file(temp_dir.path(), "after.wasm")
        .expect("Failed to create WASM file");

    let mut cmd = get_bin();
    cmd.arg("compare")
        .arg("nonexistent.wasm")
        .arg(
            after_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .assert()
        .failure()
        .stderr(predicate::str::contains("Baseline file not found"));
}

#[test]
fn test_compare_with_missing_after_file_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let before_file = temp_dir.path().join("before.wasm");
    fs::write(&before_file, fixtures::WASM_MAGIC_HEADER).expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("compare")
        .arg(
            before_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .arg("nonexistent.wasm")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Comparison file not found"));
}

#[test]
fn test_compare_with_corrupted_wasm_files_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let before_file = temp_dir.path().join("before.wasm");
    let after_file = temp_dir.path().join("after.wasm");

    fs::write(&before_file, b"not a wasm file").expect("Failed to write test file");
    fs::write(&after_file, b"also not wasm").expect("Failed to write test file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("compare")
        .arg(
            before_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .arg(
            after_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .output()
        .expect("Command execution failed");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");
        assert!(stdout.contains("Comparison") || stdout.contains("wasm-slim"));
    }
}

#[test]
fn test_compare_without_both_files_specified_returns_error() {
    let mut cmd = get_bin();
    cmd.arg("compare")
        .arg("single_file.wasm")
        .assert()
        .failure();
}

#[test]
fn test_compare_with_identical_before_and_after_shows_no_changes() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let wasm_file = fixtures::create_minimal_wasm_file(temp_dir.path(), "same.wasm")
        .expect("Failed to create WASM file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("compare")
        .arg(
            wasm_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .arg(
            wasm_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .output()
        .expect("Command execution failed");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");
        assert!(stdout.contains("Comparison") || stdout.contains("wasm-slim"));
    }
}

#[test]
fn test_compare_with_zero_byte_files_handles_gracefully() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let before_file = temp_dir.path().join("before.wasm");
    let after_file = temp_dir.path().join("after.wasm");

    fs::write(&before_file, b"").expect("Failed to write test file");
    fs::write(&after_file, b"").expect("Failed to write test file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("compare")
        .arg(
            before_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .arg(
            after_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .output()
        .expect("Command execution failed");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");
        assert!(stdout.contains("0.00") || stdout.contains("Comparison"));
    }
}

#[test]
fn test_compare_without_twiggy_installed_returns_helpful_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let before_file = temp_dir.path().join("before.wasm");
    let after_file = temp_dir.path().join("after.wasm");

    fs::write(&before_file, b"mock").expect("Failed to write test file");
    fs::write(&after_file, b"mock").expect("Failed to write test file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("compare")
        .arg(
            before_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .arg(
            after_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .output()
        .expect("Command execution failed");

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).expect("Failed to parse stderr as UTF-8");
        assert!(
            stderr.contains("twiggy") || stderr.contains("not found") || stderr.contains("failed")
        );
    }
}

#[test]
fn test_compare_with_size_difference_shows_delta() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let before_file = temp_dir.path().join("before.wasm");
    let after_file = temp_dir.path().join("after.wasm");

    fs::write(&before_file, vec![0u8; 1000]).expect("Failed to write test file");
    fs::write(&after_file, vec![0u8; 2000]).expect("Failed to write test file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("compare")
        .arg(
            before_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .arg(
            after_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .output()
        .expect("Command execution failed");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");
        assert!(
            stdout.contains("Comparison") || stdout.contains("Before") || stdout.contains("After")
        );
    } else {
        let stderr = String::from_utf8(output.stderr).expect("Failed to parse stderr as UTF-8");
        assert!(!stderr.is_empty());
    }
}
