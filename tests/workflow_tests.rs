//! End-to-end workflow tests that combine multiple commands

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

mod common;
use common::fixtures;

/// Check if required WASM tools are available
fn has_required_wasm_tools() -> bool {
    which::which("wasm-bindgen").is_ok()
}

/// Skip test if WASM tools not available
macro_rules! require_wasm_tools {
    () => {
        if !has_required_wasm_tools() {
            eprintln!("⚠️  Skipping test: wasm-bindgen-cli not found in PATH");
            eprintln!("   Install with: cargo install wasm-bindgen-cli");
            return;
        }
    };
}

/// Helper to get the wasm-slim binary command
fn get_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
}

#[test]
fn test_complete_optimization_workflow() {
    require_wasm_tools!();
    // Note: This test requires wasm-bindgen-cli to be installed

    let (temp_dir, cargo_toml) =
        fixtures::create_minimal_wasm_lib("workflow-test").expect("Failed to create test fixture");
    let project_root = temp_dir.path();

    // Add getrandom dependency for autofix testing
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "workflow-test"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
getrandom = "0.2"
"#,
    )
    .expect("Failed to write Cargo.toml");

    let src_dir = project_root.join("src");
    fs::write(
        src_dir.join("lib.rs"),
        r#"
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn calculate(x: i32, y: i32) -> i32 {
    x + y
}
"#,
    )
    .expect("Failed to write lib.rs");

    // Step 2: Initialize config file
    let mut cmd = get_bin();
    cmd.arg("init")
        .arg("--template")
        .arg("minimal")
        .current_dir(project_root)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .wasm-slim.toml"));

    // Verify config was created
    assert!(
        project_root.join(".wasm-slim.toml").exists(),
        "Config file should be created by init command"
    );

    // Step 3: Analyze dependencies to identify issues
    let mut cmd = get_bin();
    let analyze_output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--json")
        .current_dir(project_root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let analyze_json: serde_json::Value =
        serde_json::from_slice(&analyze_output).expect("Analyze output should be valid JSON");

    // Verify dependency analysis succeeded
    assert!(
        analyze_json["total_deps"].is_number(),
        "Analyze output should contain total_deps"
    );

    // Step 4: Apply autofix for known issues
    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--fix")
        .current_dir(project_root)
        .assert()
        .success()
        .stdout(predicate::str::contains("Applied").and(predicate::str::contains("fixes")));

    // Step 5: Build with optimizations
    let mut cmd = get_bin();
    cmd.arg("build")
        .current_dir(project_root)
        .assert()
        .success()
        .stdout(predicate::str::contains("Build complete"));

    // Step 6: Verify backup was created in .wasm-slim/backups/
    let backup_dir = temp_dir.path().join(".wasm-slim/backups");
    assert!(
        backup_dir.exists(),
        "Backup directory should be created after applying fixes"
    );

    let backup_files: Vec<_> = std::fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("backup"))
        .collect();
    assert!(
        !backup_files.is_empty(),
        "At least one backup file should be created"
    );
}

#[test]
fn test_iterative_optimization_workflow() {
    require_wasm_tools!();

    let (temp_dir, cargo_toml) =
        fixtures::create_minimal_wasm_lib("iterative-test").expect("Failed to create test fixture");
    let project_root = temp_dir.path();

    // Step 2: First build (baseline)
    let mut cmd = get_bin();
    cmd.arg("build")
        .current_dir(project_root)
        .assert()
        .success();

    // Read Cargo.toml to verify optimization was applied
    let cargo_content =
        fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml after first build");

    assert!(
        cargo_content.contains("[profile.release]")
            || cargo_content.contains("lto")
            || cargo_content.contains("opt-level"),
        "First build should add optimization profiles"
    );

    // Step 3: Analyze for further opportunities
    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("deps")
        .current_dir(project_root)
        .assert()
        .success();

    // Step 4: Second build (should be idempotent)
    let mut cmd = get_bin();
    cmd.arg("build")
        .current_dir(project_root)
        .assert()
        .success();

    // Verify that multiple builds don't corrupt Cargo.toml
    let final_cargo =
        fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml after second build");

    // Ensure Cargo.toml is still valid TOML by parsing it
    final_cargo
        .parse::<toml_edit::DocumentMut>()
        .expect("Cargo.toml should remain valid after multiple builds");
}

#[test]
fn test_compare_before_after_workflow() {
    require_wasm_tools!();

    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("compare-test").expect("Failed to create test fixture");
    let project_root = temp_dir.path();

    // Step 2: Create "before" WASM file (simulated)
    let before_wasm = project_root.join("before.wasm");
    fs::write(&before_wasm, fixtures::WASM_MAGIC_HEADER).expect("Failed to write before WASM file");

    // Step 3: Build optimized version
    let mut cmd = get_bin();
    cmd.arg("build")
        .current_dir(project_root)
        .assert()
        .success();

    // Step 4: Create "after" WASM file (simulated smaller)
    let after_wasm = project_root.join("after.wasm");
    fs::write(&after_wasm, fixtures::WASM_MAGIC_HEADER).expect("Failed to write after WASM file");

    // Step 5: Compare before and after
    let mut cmd = get_bin();
    cmd.arg("compare")
        .arg(&before_wasm)
        .arg(&after_wasm)
        .current_dir(project_root)
        .assert()
        .success()
        .stdout(predicate::str::contains("Before").or(predicate::str::contains("After")));
}

#[test]
fn test_analyze_assets_then_build_workflow() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_project_with_assets("assets-test").expect("Failed to create test fixture");
    let project_root = temp_dir.path();

    // Step 2: Analyze assets to identify externalization opportunities
    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("assets")
        .current_dir(project_root)
        .assert()
        .success()
        .stdout(predicate::str::contains("Asset").or(predicate::str::contains("include_bytes")));

    // Step 3: Get externalization guide
    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("assets")
        .arg("--guide")
        .current_dir(project_root)
        .assert()
        .success()
        .stdout(predicate::str::contains("Externalization").or(predicate::str::contains("guide")));
}

#[test]
fn test_dry_run_then_real_build_workflow() {
    require_wasm_tools!();

    let (temp_dir, cargo_toml) =
        fixtures::create_minimal_wasm_lib("dryrun-test").expect("Failed to create test fixture");
    let project_root = temp_dir.path();

    let original_cargo_content =
        fs::read_to_string(&cargo_toml).expect("Failed to read original Cargo.toml");

    // Step 2: Dry run to preview changes
    let mut cmd = get_bin();
    cmd.arg("build")
        .arg("--dry-run")
        .current_dir(project_root)
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN"));

    // Step 3: Verify Cargo.toml unchanged after dry run
    let after_dry_run =
        fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml after dry run");

    assert_eq!(
        original_cargo_content.trim(),
        after_dry_run.trim(),
        "Cargo.toml should not be modified by dry run"
    );

    // Step 5: Real build
    let mut cmd = get_bin();
    cmd.arg("build")
        .current_dir(project_root)
        .assert()
        .success();

    // Step 6: Verify Cargo.toml was modified during real build
    let after_real_build =
        fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml after real build");

    assert_ne!(
        original_cargo_content.trim(),
        after_real_build.trim(),
        "Cargo.toml should be modified by real build"
    );
}
