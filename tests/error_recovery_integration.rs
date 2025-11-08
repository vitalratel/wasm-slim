//! Integration tests for error recovery and rollback scenarios

use anyhow::Result;
use std::fs;
use std::process::Command;

mod common;

fn get_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
}

#[test]
fn test_build_failure_preserves_original_files() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();

    // Create a project with invalid configuration
    fs::create_dir_all(project_path.join("src"))?;
    fs::write(
        project_path.join("Cargo.toml"),
        r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
nonexistent-crate = "999.999.999"
"#,
    )?;

    fs::write(project_path.join("src/lib.rs"), "pub fn test() {}")?;

    // Save original Cargo.toml content
    let original_content = fs::read_to_string(project_path.join("Cargo.toml"))?;

    // Attempt to build (will fail)
    let output = get_bin().current_dir(project_path).arg("build").output()?;

    // Build should fail
    assert!(!output.status.success());

    // Original Cargo.toml should be preserved
    let current_content = fs::read_to_string(project_path.join("Cargo.toml"))?;
    assert_eq!(
        original_content, current_content,
        "Cargo.toml should not be modified on build failure"
    );

    Ok(())
}

#[test]
fn test_analyze_with_corrupted_wasm_shows_error() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();

    // Create project structure
    fs::create_dir_all(project_path.join("src"))?;
    fs::write(
        project_path.join("Cargo.toml"),
        r#"
[package]
name = "test-corrupted"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
"#,
    )?;

    fs::write(project_path.join("src/lib.rs"), "pub fn test() {}")?;

    // Create target directory with corrupted WASM file
    let wasm_dir = project_path.join("target/wasm32-unknown-unknown/release");
    fs::create_dir_all(&wasm_dir)?;
    fs::write(
        wasm_dir.join("test_corrupted.wasm"),
        b"not a valid wasm file",
    )?;

    // Try to analyze
    let output = get_bin()
        .current_dir(project_path)
        .arg("analyze")
        .output()?;

    // Should provide error message (may or may not fail depending on analysis type)
    let stderr = String::from_utf8_lossy(&output.stderr);

    // At minimum, should not crash
    assert!(
        !stderr.contains("thread") || !stderr.contains("panicked"),
        "Should not panic on corrupted WASM"
    );

    Ok(())
}

#[test]
fn test_compare_with_missing_baseline_handles_gracefully() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();

    // Create minimal project
    fs::create_dir_all(project_path.join("src"))?;
    fs::write(
        project_path.join("Cargo.toml"),
        r#"
[package]
name = "test-compare"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
"#,
    )?;

    fs::write(project_path.join("src/lib.rs"), "pub fn test() {}")?;

    // Try to compare with a missing baseline file
    let baseline = project_path.join("baseline.wasm");
    let current = project_path.join("current.wasm");

    // Create only the "current" file
    fs::write(&current, b"fake wasm content")?;

    let output = get_bin()
        .current_dir(project_path)
        .arg("compare")
        .arg(baseline.to_str().unwrap())
        .arg(current.to_str().unwrap())
        .output()?;

    // Should handle missing baseline gracefully
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Baseline")
                || stderr.contains("baseline")
                || stderr.contains("not found"),
            "Error message should mention missing baseline. Got: {}",
            stderr
        );
    }

    Ok(())
}

#[test]
fn test_init_with_existing_config_shows_appropriate_message() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();

    // Create project with existing config
    fs::create_dir_all(project_path.join("src"))?;
    fs::write(
        project_path.join("Cargo.toml"),
        r#"
[package]
name = "test-init"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    fs::write(project_path.join("src/lib.rs"), "pub fn test() {}")?;

    // Create initial config
    let output = get_bin()
        .current_dir(project_path)
        .arg("init")
        .arg("--template")
        .arg("balanced")
        .output()?;

    assert!(output.status.success(), "First init should succeed");

    // Try to init again
    let output = get_bin()
        .current_dir(project_path)
        .arg("init")
        .arg("--template")
        .arg("aggressive")
        .output()?;

    // Should handle existing config appropriately
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Either succeeds (overwrites) or provides informative message
    if !output.status.success() {
        assert!(
            stderr.contains("exists") || stderr.contains("already"),
            "Should mention existing config"
        );
    }

    Ok(())
}

#[test]
fn test_cleanup_with_no_backups_handles_gracefully() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();

    // Create minimal project without any backups
    fs::create_dir_all(project_path.join("src"))?;
    fs::write(
        project_path.join("Cargo.toml"),
        r#"
[package]
name = "test-cleanup"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    fs::write(project_path.join("src/lib.rs"), "pub fn test() {}")?;

    // Try to cleanup
    let output = get_bin()
        .current_dir(project_path)
        .arg("cleanup")
        .output()?;

    // Should handle no backups gracefully (success or informative message)
    let stdout = String::from_utf8_lossy(&output.stdout);

    if output.status.success() {
        // If succeeds, should indicate no backups found
        assert!(
            stdout.contains("No backups") || stdout.contains("0 backup") || stdout.is_empty(),
            "Should indicate no backups to clean"
        );
    }

    Ok(())
}
