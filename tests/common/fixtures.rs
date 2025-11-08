//! Test fixture helpers for creating test projects
//!
//! Provides utilities for setting up realistic test projects with proper
//! Cargo.toml, source files, and directory structure.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Re-export anyhow for convenience
pub use anyhow;

/// Creates a minimal WASM library project with proper structure
///
/// # Returns
///
/// A tuple of (TempDir, PathBuf to Cargo.toml) - the TempDir must be kept alive
pub fn create_minimal_wasm_lib(name: &str) -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Create Cargo.toml
    fs::write(
        &cargo_toml,
        format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
"#,
            name
        ),
    )?;

    // Create src directory and lib.rs
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir)?;
    fs::write(
        src_dir.join("lib.rs"),
        r#"
//! Minimal WASM library for testing

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    )?;

    Ok((temp_dir, cargo_toml))
}

/// Creates a minimal binary project with proper structure
///
/// # Returns
///
/// A tuple of (TempDir, PathBuf to Cargo.toml)
pub fn create_minimal_bin(name: &str) -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Create Cargo.toml
    fs::write(
        &cargo_toml,
        format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"
"#,
            name
        ),
    )?;

    // Create src directory and main.rs
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir)?;
    fs::write(
        src_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello, world!");
}
"#,
    )?;

    Ok((temp_dir, cargo_toml))
}

/// Creates a WASM library project with wasm-bindgen
///
/// # Returns
///
/// A tuple of (TempDir, PathBuf to Cargo.toml)
pub fn create_wasm_bindgen_project(name: &str) -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Create Cargo.toml with wasm-bindgen
    fs::write(
        &cargo_toml,
        format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
"#,
            name
        ),
    )?;

    // Create src directory and lib.rs with wasm-bindgen usage
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir)?;
    fs::write(
        src_dir.join("lib.rs"),
        r#"
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#,
    )?;

    Ok((temp_dir, cargo_toml))
}

/// Helper to get the project root from temp_dir and cargo_toml
pub fn project_root(temp_dir: &TempDir) -> &Path {
    temp_dir.path()
}

/// Creates a WASM project with getrandom dependency (common WASM issue case)
///
/// # Returns
///
/// A tuple of (TempDir, PathBuf to Cargo.toml)
pub fn create_project_with_getrandom(name: &str) -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Create Cargo.toml with getrandom missing WASM features
    fs::write(
        &cargo_toml,
        format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
getrandom = "0.2"
"#,
            name
        ),
    )?;

    // Create src directory and lib.rs
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir)?;
    fs::write(
        src_dir.join("lib.rs"),
        r#"
pub fn test() -> u32 {
    42
}
"#,
    )?;

    Ok((temp_dir, cargo_toml))
}

/// Creates a project with embedded assets
///
/// # Returns
///
/// A tuple of (TempDir, PathBuf to Cargo.toml)
pub fn create_project_with_assets(name: &str) -> anyhow::Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    // Create Cargo.toml
    fs::write(
        &cargo_toml,
        format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"
"#,
            name
        ),
    )?;

    // Create src directory and lib.rs with include_bytes!
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir)?;
    fs::write(
        src_dir.join("lib.rs"),
        r#"
const LOGO: &[u8] = include_bytes!("../assets/logo.png");

pub fn get_logo() -> &'static [u8] {
    LOGO
}
"#,
    )?;

    // Create assets directory with a test file
    let assets_dir = temp_dir.path().join("assets");
    fs::create_dir(&assets_dir)?;
    fs::write(assets_dir.join("logo.png"), vec![0u8; 1024])?; // 1KB dummy file

    Ok((temp_dir, cargo_toml))
}

/// Creates a minimal test WASM file with magic header
///
/// # Returns
///
/// PathBuf to the created WASM file
pub fn create_minimal_wasm_file(dir: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let wasm_file = dir.join(name);
    // WASM magic number + version
    fs::write(&wasm_file, [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])?;
    Ok(wasm_file)
}

/// Creates two WASM files for comparison testing
///
/// # Returns
///
/// Tuple of (before_wasm PathBuf, after_wasm PathBuf)
pub fn create_wasm_pair_for_comparison(dir: &Path) -> anyhow::Result<(PathBuf, PathBuf)> {
    let before = create_minimal_wasm_file(dir, "before.wasm")?;
    let after = create_minimal_wasm_file(dir, "after.wasm")?;
    Ok((before, after))
}

/// Constants for common test patterns
pub const WASM_MAGIC_HEADER: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
pub const DEFAULT_TEST_LIB_RS: &str = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
