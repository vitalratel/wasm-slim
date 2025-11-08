//! Cargo.toml optimization and backup management
//!
//! This module provides:
//! - Cargo profile optimization with WASM-specific settings
//! - Backup and restore functionality for Cargo.toml files
//! - wasm-opt configuration management
//!
//! ## Key Types
//!
//! - `CargoTomlEditor` - Applies optimization settings to Cargo.toml
//! - `CargoAnalyzer` - Analyzes Cargo.toml for WASM indicators
//! - `CargoFileFinder` - Discovers Cargo.toml files in projects
//! - `BackupManager` - Creates timestamped backups before modifications
//! - `OptimizationConfig` - Cargo profile settings (LTO, opt-level, etc.)
//! - `WasmOptConfig` - wasm-opt flags and configuration
//!
//! ## Usage
//!
//! ```no_run
//! use wasm_slim::optimizer::{CargoTomlEditor, OptimizationConfig};
//! use std::path::Path;
//!
//! let editor = CargoTomlEditor::new();
//! let config = OptimizationConfig::default();
//! editor.optimize_cargo_toml(Path::new("Cargo.toml"), &config, None, false)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod backup;
pub mod build_std;
pub mod cargo;

pub use backup::BackupManager;
pub use build_std::{BuildStdConfig, BuildStdOptimizer};
pub use cargo::{
    CargoAnalyzer, CargoFileFinder, CargoTomlEditor, OptimizationConfig, WasmOptConfig,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_module_exports_are_accessible() {
        // Ensure all exports compile and are accessible
        let _: Option<CargoTomlEditor> = None;
        let _: Option<CargoAnalyzer> = None;
        let _: Option<CargoFileFinder> = None;
        let _: Option<OptimizationConfig> = None;
        let _: Option<WasmOptConfig> = None;
        let _: Option<BackupManager> = None;
    }

    #[test]
    fn test_optimization_configs_default_have_expected_values() {
        let opt_config = OptimizationConfig::default();
        assert_eq!(opt_config.lto, "fat");
        assert_eq!(opt_config.codegen_units, 1);

        let wasm_config = WasmOptConfig::default();
        assert!(!wasm_config.flags.is_empty());
    }
}
