#![warn(missing_docs)]
#![warn(clippy::unwrap_used)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

//! wasm-slim library
//!
//! This library provides the core functionality for WASM bundle size optimization.
//! It can be used programmatically in addition to the CLI interface.
//!
//! # Basic Example
//!
//! Creating and validating configuration:
//!
//! ```
//! use wasm_slim::config::file::{ConfigFile, ProfileSettings};
//!
//! // Create configuration with custom profile settings
//! let config = ConfigFile {
//!     template: "balanced".to_string(),
//!     profile: Some(ProfileSettings {
//!         opt_level: Some("z".to_string()),
//!         lto: Some("thin".to_string()),
//!         strip: Some(true),
//!         codegen_units: Some(1),
//!         panic: Some("abort".to_string()),
//!     }),
//!     wasm_opt: None,
//!     size_budget: None,
//! };
//!
//! assert_eq!(config.template, "balanced");
//! ```
//!
//! # Advanced Example: Size Budget Validation
//!
//! Using size budgets to enforce WASM bundle size limits:
//!
//! ```
//! use wasm_slim::config::file::SizeBudget;
//!
//! // Create a size budget with target and thresholds
//! let budget = SizeBudget {
//!     target_size_kb: Some(300),
//!     warn_threshold_kb: Some(400),
//!     max_size_kb: Some(500),
//! };
//!
//! // Validate budget constraints
//! let validation = budget.validate();
//! assert!(validation.is_ok());
//!
//! // Invalid budget (target > max) should fail
//! let bad_budget = SizeBudget {
//!     target_size_kb: Some(600),
//!     warn_threshold_kb: Some(400),
//!     max_size_kb: Some(500),
//! };
//! assert!(bad_budget.validate().is_err());
//! ```
//!
//! # Advanced Example: Multi-stage Workflow
//!
//! Creating backups before modification:
//!
//! ```
//! use wasm_slim::optimizer::BackupManager;
//! use wasm_slim::config::file::ConfigFile;
//! use std::path::Path;
//! use tempfile::TempDir;
//! use std::fs;
//!
//! // Setup temporary workspace
//! let workspace = TempDir::new().unwrap();
//! let cargo_toml = workspace.path().join("Cargo.toml");
//! fs::write(&cargo_toml, "[package]\nname = \"test\"\n").unwrap();
//!
//! // Create backup before modification
//! let backup_mgr = BackupManager::new(workspace.path());
//! let backup_path = backup_mgr.create_backup(&cargo_toml).unwrap();
//! assert!(backup_path.exists());
//!
//! // Backup contains original content
//! let backup_content = fs::read_to_string(&backup_path).unwrap();
//! assert!(backup_content.contains("test"));
//!
//! // Now safe to modify the original file
//! let config = ConfigFile::default();
//! fs::write(&cargo_toml, "[package]\nname = \"optimized\"\n").unwrap();
//! ```

/// Analysis tools for WASM bundles
pub mod analyzer;
/// Performance tracking utilities
pub mod bench_tracker;
/// CI/CD integration tooling
pub mod cicd;
/// Command handlers for CLI operations
pub mod cmd;
/// Configuration file and template management
pub mod config;
/// Enhanced error types with contextual suggestions
pub mod error;
/// Shared formatting utilities
pub mod fmt;
/// Git metadata utilities
pub mod git;
/// Infrastructure traits for filesystem and command execution
pub mod infra;
/// Cargo.toml optimization and backup management
pub mod optimizer;
/// Build pipeline orchestration
pub mod pipeline;
/// Rust toolchain detection and management
pub mod toolchain;
/// Tool detection and version checking
pub mod tools;
