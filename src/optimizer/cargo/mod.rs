//! Cargo.toml optimization module
//!
//! This module provides focused components for Cargo.toml optimization:
//! - [`CargoFileFinder`] - File discovery
//! - [`CargoTomlEditor`] - TOML editing
//! - [`CargoAnalyzer`] - Analysis logic
//!
//! # Examples
//!
//! ```no_run
//! use wasm_slim::optimizer::{CargoTomlEditor, OptimizationConfig};
//! use std::path::Path;
//!
//! let editor = CargoTomlEditor::new();
//! let config = OptimizationConfig::default();
//! let changes = editor.optimize_cargo_toml(
//!     Path::new("Cargo.toml"),
//!     &config,
//!     None,
//!     false,
//! )?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use thiserror::Error;

// Import the focused components
mod cargo_analyzer;
mod cargo_file_finder;
mod cargo_toml_editor;

// Re-export for public use
pub use cargo_analyzer::{AnalysisError, CargoAnalyzer};
pub use cargo_file_finder::CargoFileFinder;
pub use cargo_toml_editor::{CargoTomlEditor, TomlEditError};

/// Errors that can occur during Cargo.toml optimization
#[derive(Error, Debug)]
pub enum CargoOptimizationError {
    /// I/O error reading or writing Cargo.toml
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse TOML
    #[error("Failed to parse TOML: {0}")]
    TomlParse(#[from] toml_edit::TomlError),

    /// Invalid Cargo.toml structure
    #[error("Invalid Cargo.toml structure: {0}")]
    InvalidStructure(String),

    /// TOML editing error
    #[error("TOML editing error: {0}")]
    TomlEdit(#[from] TomlEditError),

    /// Analysis error
    #[error("Analysis error: {0}")]
    Analysis(#[from] AnalysisError),
}

// Re-export config types for convenience
pub use crate::config::ProfileConfig as OptimizationConfig;
pub use crate::config::WasmOptConfig;
