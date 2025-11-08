//! Common test utilities and helpers
//!
//! This module provides shared functionality for integration tests:
//! - Assertion helpers for robust comparisons
//! - Test fixture creation utilities
//! - Common setup/teardown patterns
//!
//! # Usage
//!
//! ```rust,no_run
//! mod common;
//! use common::assertions::*;
//!
//! fn test_size_calculation() {
//!     let result = calculate_size();
//!     // Use approximate comparison for robustness
//!     assert_size_within(result, 1048576, 1024);
//! }
//! ```

pub mod assertions;
pub mod fixtures;

/// Check if running in CI environment
#[allow(dead_code)]
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

/// Check if wasm-bindgen-cli is available
#[allow(dead_code)]
pub fn has_required_wasm_tools() -> bool {
    which::which("wasm-bindgen").is_ok()
}

/// Setup a test project with a given name
#[allow(dead_code)]
pub fn setup_test_project(name: &str) -> anyhow::Result<std::path::PathBuf> {
    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path().to_path_buf();

    // Create basic project structure
    std::fs::create_dir_all(project_path.join("src"))?;
    std::fs::write(
        project_path.join("Cargo.toml"),
        format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
"#,
            name
        ),
    )?;

    std::fs::write(
        project_path.join("src/lib.rs"),
        r#"
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[no_mangle]
pub extern "C" fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )?;

    // Leak temp_dir to keep it alive for the test duration
    std::mem::forget(temp_dir);
    Ok(project_path)
}

/// Macro to skip tests when WASM tools are not available
/// In CI, this will fail the test instead of skipping
#[macro_export]
macro_rules! require_wasm_tools {
    () => {
        if !$crate::common::has_required_wasm_tools() {
            if $crate::common::is_ci() {
                panic!("Required WASM tools missing in CI! Install wasm-bindgen-cli");
            } else {
                eprintln!("⚠️  Skipping test: wasm-bindgen-cli not found in PATH");
                eprintln!("   Install with: cargo install wasm-bindgen-cli");
                return Ok(());
            }
        }
    };
}
