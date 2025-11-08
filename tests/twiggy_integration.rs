//! Integration tests for twiggy analysis pipeline

use anyhow::Result;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_twiggy_end_to_end_analysis() -> Result<()> {
    require_wasm_tools!();

    let test_dir = setup_test_project("twiggy_e2e")?;

    // Build a simple WASM binary
    let status = Command::new("cargo")
        .current_dir(&test_dir)
        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .status()?;

    assert!(status.success(), "WASM build should succeed");

    // Find the built WASM file (uses project name from Cargo.toml, not directory name)
    let wasm_file = test_dir
        .join("target/wasm32-unknown-unknown/release")
        .join("twiggy_e2e.wasm");

    // Run twiggy analysis via wasm-slim (using 'top' mode)
    let output = Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
        .current_dir(&test_dir)
        .args(["analyze", "--mode", "top", wasm_file.to_str().unwrap()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Twiggy analysis failed: {}", stderr);
    }

    assert!(output.status.success(), "Twiggy analysis should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify output contains expected twiggy data
    assert!(
        stdout.contains("Size") || stdout.contains("Shallow") || stdout.contains("bytes"),
        "Should contain twiggy size analysis. Got: {}",
        stdout
    );

    Ok(())
}

#[test]
fn test_twiggy_version_compatibility() -> Result<()> {
    require_wasm_tools!();

    // Check twiggy is installed and version is compatible
    let output = Command::new("twiggy").arg("--version").output();

    match output {
        Ok(out) => {
            assert!(out.status.success(), "Twiggy should report version");
            let version = String::from_utf8_lossy(&out.stdout);
            println!("Twiggy version: {}", version);
            assert!(version.contains("twiggy"), "Should identify as twiggy");
        }
        Err(_) => {
            println!("⚠️  Twiggy not installed - skipping version compatibility test");
            return Ok(());
        }
    }

    Ok(())
}

#[test]
fn test_twiggy_error_handling_with_invalid_wasm() -> Result<()> {
    require_wasm_tools!();

    let test_dir = setup_test_project("twiggy_invalid")?;

    // Create an invalid WASM file
    let invalid_wasm = test_dir.join("target/wasm32-unknown-unknown/release/invalid.wasm");
    std::fs::create_dir_all(
        invalid_wasm
            .parent()
            .expect("Invalid wasm path should have parent"),
    )?;
    std::fs::write(&invalid_wasm, b"not a valid wasm file")?;

    // Attempt to analyze invalid WASM
    let output = Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
        .current_dir(&test_dir)
        .args(["analyze", "--twiggy"])
        .output()?;

    // Should handle error gracefully
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.is_empty(), "Should provide error message");
    }

    Ok(())
}

#[test]
fn test_twiggy_with_missing_binary() -> Result<()> {
    let test_dir = setup_test_project("twiggy_missing")?;

    // Try to analyze non-existent WASM file
    let missing_wasm = test_dir.join("target/wasm32-unknown-unknown/release/nonexistent.wasm");
    let output = Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
        .current_dir(&test_dir)
        .args(["analyze", "--mode", "top", missing_wasm.to_str().unwrap()])
        .output()?;

    // Should handle missing binary gracefully
    assert!(
        !output.status.success(),
        "Should fail with missing WASM file"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stderr, stdout);

    assert!(
        combined.contains("not found")
            || combined.contains("No such file")
            || combined.contains("does not exist")
            || combined.contains("Failed to"),
        "Error message should indicate missing WASM file. Got stderr: '{}', stdout: '{}'",
        stderr,
        stdout
    );

    Ok(())
}
