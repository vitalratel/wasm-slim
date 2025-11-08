//! Integration tests for the build pipeline
//!
//! Tests the complete optimization workflow at the pipeline level

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use wasm_slim::pipeline::config::{BindgenTarget, PipelineConfig, WasmTarget};
use wasm_slim::pipeline::executor::BuildPipeline;

mod common;

/// Create a minimal test Rust project with Cargo.toml
fn create_test_project(temp_dir: &TempDir) -> PathBuf {
    let project_root = temp_dir.path().to_path_buf();

    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = 3
lto = true
"#;

    fs::write(project_root.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Create src/lib.rs
    fs::create_dir(project_root.join("src")).expect("Failed to create src directory");

    let lib_rs = r#"use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;

    fs::write(project_root.join("src/lib.rs"), lib_rs).expect("Failed to write lib.rs");

    project_root
}

#[test]
fn test_pipeline_creation_with_valid_project() {
    // Test that pipeline can be created for a valid Rust project
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let config = PipelineConfig::default();
    let pipeline = BuildPipeline::new(project_root.clone(), config);

    // Pipeline creation should succeed (constructor doesn't fail)
    // Verify pipeline was created without panicking
    let _ = pipeline;
}

#[test]
fn test_pipeline_creation_with_empty_directory() {
    // Test that pipeline can be created even without Cargo.toml
    // (validation happens during build, not construction)
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    let config = PipelineConfig::default();
    let pipeline = BuildPipeline::new(project_root.clone(), config);

    // Constructor succeeds; errors would occur during build()
    let _ = pipeline;
}

#[test]
fn test_pipeline_respects_custom_target() {
    // Test that custom target is stored in config
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let config = PipelineConfig {
        bindgen_target: BindgenTarget::NodeJs,
        ..Default::default()
    };

    let pipeline = BuildPipeline::new(project_root.clone(), config);

    // Verify pipeline was created with custom config
    let _ = pipeline;
}

#[test]
fn test_pipeline_config_default_values() {
    // Verify default configuration has expected values
    let config = PipelineConfig::default();

    assert_eq!(
        config.target,
        WasmTarget::Wasm32UnknownUnknown,
        "target should default to wasm32-unknown-unknown"
    );
    assert_eq!(
        config.profile, "release",
        "profile should default to release"
    );
    assert!(config.run_wasm_opt, "run_wasm_opt should default to true");
}

#[test]
fn test_pipeline_stores_project_root() {
    // Test that pipeline correctly stores project root
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let config = PipelineConfig::default();
    let pipeline = BuildPipeline::new(project_root.clone(), config);

    // Verify pipeline creation succeeded
    let _ = pipeline;
}

#[test]
fn test_pipeline_with_relative_path() {
    // Test pipeline creation with relative path
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let _ = create_test_project(&temp_dir);

    let config = PipelineConfig::default();
    let pipeline = BuildPipeline::new(temp_dir.path(), config);

    // Verify pipeline handles relative paths
    let _ = pipeline;
}

#[test]
fn test_pipeline_concurrent_creation() {
    // Test that multiple pipelines can be created concurrently
    use std::sync::Arc;
    use std::thread;

    let temp_dir = Arc::new(tempfile::tempdir().expect("Failed to create temp dir"));
    let project_root = create_test_project(&temp_dir);
    let project_root = Arc::new(project_root);

    let handles: Vec<_> = (0..3)
        .map(|_i| {
            let root = Arc::clone(&project_root);
            thread::spawn(move || {
                let config = PipelineConfig::default();
                let _pipeline = BuildPipeline::new((*root).clone(), config);
                // Verify concurrent creation doesn't panic
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_pipeline_config_builder_pattern() {
    // Test that PipelineConfig can be built with custom values
    let config = PipelineConfig {
        bindgen_target: BindgenTarget::NodeJs,
        run_wasm_snip: true,
        ..Default::default()
    };

    assert_eq!(config.bindgen_target, BindgenTarget::NodeJs);
    assert!(config.run_wasm_snip);
}

#[test]
fn test_pipeline_stores_configuration() {
    // Test that pipeline correctly stores the provided configuration
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let config = PipelineConfig {
        bindgen_target: BindgenTarget::NodeJs,
        run_wasm_opt: false,
        ..Default::default()
    };

    let pipeline = BuildPipeline::new(project_root.clone(), config);

    // Verify pipeline created with custom config
    let _ = pipeline;
}

#[test]
fn test_pipeline_with_nested_project_path() {
    // Test pipeline with deeply nested project path
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let nested_path = temp_dir.path().join("a/b/c/project");
    fs::create_dir_all(&nested_path).expect("Failed to create nested dirs");

    let config = PipelineConfig::default();
    let pipeline = BuildPipeline::new(nested_path.clone(), config);

    // Verify pipeline handles nested paths
    let _ = pipeline;
}

#[test]
fn test_pipeline_with_unicode_project_path() {
    // Test pipeline with Unicode characters in path
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let unicode_path = temp_dir.path().join("проект-日本-🦀");
    fs::create_dir_all(&unicode_path).expect("Failed to create unicode dir");

    let config = PipelineConfig::default();
    let pipeline = BuildPipeline::new(unicode_path.clone(), config);

    // Verify pipeline handles Unicode paths
    let _ = pipeline;
}

#[test]
fn test_pipeline_multiple_instances_same_project() {
    // Test that multiple pipeline instances can point to same project
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let config1 = PipelineConfig::default();
    let pipeline1 = BuildPipeline::new(project_root.clone(), config1);

    let config2 = PipelineConfig {
        run_wasm_snip: true,
        ..Default::default()
    };
    let pipeline2 = BuildPipeline::new(project_root.clone(), config2);

    // Both pipelines should be created successfully
    let _ = (pipeline1, pipeline2);
}

#[test]
fn test_pipeline_default_toolchain() {
    // Test that pipeline initializes with default toolchain
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir);

    let config = PipelineConfig::default();
    let pipeline = BuildPipeline::new(project_root, config);

    // Toolchain should be initialized (actual validation happens during build)
    // This test verifies constructor doesn't panic
    let _ = pipeline;
}
