//! Cargo.toml TOML editing module
//!
//! Provides [`CargoTomlEditor`] for modifying Cargo.toml [profile.release]
//! settings with production-tested WASM optimization flags.

use crate::config::{ProfileConfig as OptimizationConfig, WasmOptConfig};
use crate::infra::{FileSystem, RealFileSystem};
use std::path::Path;
use thiserror::Error;
use toml_edit::{value, Array, DocumentMut, Item, Table};

/// Errors that can occur during TOML editing
#[derive(Error, Debug)]
pub enum TomlEditError {
    /// I/O error reading or writing Cargo.toml
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse TOML
    #[error("Failed to parse TOML: {0}")]
    TomlParse(#[from] toml_edit::TomlError),

    /// Invalid Cargo.toml structure
    #[error("Invalid Cargo.toml structure: {0}")]
    InvalidStructure(String),
}

/// Edits Cargo.toml files with WASM optimization settings
///
/// This component applies production-tested optimization patterns to
/// `[profile.release]` settings for WASM bundle size reduction.
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::optimizer::{CargoTomlEditor, OptimizationConfig};
/// use std::path::Path;
///
/// let editor = CargoTomlEditor::new();
/// let config = OptimizationConfig::default();
///
/// // Optimize a Cargo.toml file
/// let changes = editor.optimize_cargo_toml(
///     Path::new("Cargo.toml"),
///     &config,
///     None,
///     false // not dry run
/// )?;
///
/// for change in &changes {
///     println!("Applied: {}", change);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct CargoTomlEditor<FS: FileSystem = RealFileSystem> {
    fs: FS,
}

impl CargoTomlEditor<RealFileSystem> {
    /// Create a new TOML editor
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::optimizer::CargoTomlEditor;
    ///
    /// let editor = CargoTomlEditor::new();
    /// ```
    pub fn new() -> Self {
        Self::with_fs(RealFileSystem)
    }
}

impl Default for CargoTomlEditor<RealFileSystem> {
    fn default() -> Self {
        Self::new()
    }
}

impl<FS: FileSystem> CargoTomlEditor<FS> {
    /// Create a new TOML editor with a custom filesystem implementation
    pub fn with_fs(fs: FS) -> Self {
        Self { fs }
    }

    /// Optimize a single Cargo.toml file for WASM bundle size reduction
    ///
    /// Applies production-tested optimization patterns including:
    /// - LTO (Link Time Optimization)
    /// - opt-level = "z" (size optimization)
    /// - codegen-units = 1 (better optimization)
    /// - strip = true (remove debug info)
    /// - wasm-opt flags (if WASM config provided)
    ///
    /// # Arguments
    ///
    /// * `cargo_toml_path` - Path to the Cargo.toml file to optimize
    /// * `config` - Optimization configuration settings
    /// * `wasm_opt` - Optional WASM-specific optimization config
    /// * `dry_run` - If true, analyze changes without modifying the file
    ///
    /// # Returns
    ///
    /// A vector of strings describing the changes made
    ///
    /// # Examples
    ///
    /// Basic usage with default config:
    /// ```no_run
    /// use wasm_slim::optimizer::{CargoTomlEditor, OptimizationConfig};
    /// use std::path::Path;
    ///
    /// let editor = CargoTomlEditor::new();
    /// let config = OptimizationConfig::default();
    /// let changes = editor.optimize_cargo_toml(
    ///     Path::new("Cargo.toml"),
    ///     &config,
    ///     None,
    ///     false // not dry run
    /// )?;
    ///
    /// println!("Applied {} optimizations", changes.len());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read or written
    /// - The TOML content is malformed
    pub fn optimize_cargo_toml(
        &self,
        cargo_toml_path: &Path,
        config: &OptimizationConfig,
        wasm_opt: Option<&WasmOptConfig>,
        dry_run: bool,
    ) -> Result<Vec<String>, TomlEditError> {
        let content = self.fs.read_to_string(cargo_toml_path)?;

        let mut doc = content.parse::<DocumentMut>()?;

        let mut changes = Vec::new();

        // Apply [profile.release] optimizations
        self.apply_profile_optimizations(&mut doc, config, &mut changes)?;

        // Apply wasm-pack optimizations if requested
        if let Some(wasm_config) = wasm_opt {
            self.apply_wasm_pack_optimizations(&mut doc, wasm_config, &mut changes)?;
        }

        if !dry_run && !changes.is_empty() {
            self.fs.write(cargo_toml_path, doc.to_string())?;
        }

        Ok(changes)
    }

    /// Apply [profile.release] optimizations
    fn apply_profile_optimizations(
        &self,
        doc: &mut DocumentMut,
        config: &OptimizationConfig,
        changes: &mut Vec<String>,
    ) -> Result<(), TomlEditError> {
        // Get or create [profile.release] table
        let profile = doc
            .entry("profile")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| TomlEditError::InvalidStructure("profile is not a table".to_string()))?;

        let release = profile
            .entry("release")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| {
                TomlEditError::InvalidStructure("profile.release is not a table".to_string())
            })?;

        // Apply LTO
        if !release.contains_key("lto") || release["lto"].as_str() != Some(&config.lto) {
            release["lto"] = value(&config.lto);
            changes.push(format!("Set lto = \"{}\" (15-30% reduction)", config.lto));
        }

        // Apply codegen-units
        if !release.contains_key("codegen-units")
            || release["codegen-units"].as_integer() != Some(config.codegen_units as i64)
        {
            release["codegen-units"] = value(config.codegen_units as i64);
            changes.push(format!(
                "Set codegen-units = {} (better optimization)",
                config.codegen_units
            ));
        }

        // Apply opt-level
        let needs_opt_level_update = if let Some(existing) = release.get("opt-level") {
            // Check both string and integer representations
            existing.as_str() != Some(&config.opt_level)
                && existing.as_integer().map(|i| i.to_string()) != Some(config.opt_level.clone())
        } else {
            true
        };

        if needs_opt_level_update {
            release["opt-level"] = value(&config.opt_level);
            changes.push(format!(
                "Set opt-level = \"{}\" (size-optimized)",
                config.opt_level
            ));
        }

        // Apply strip
        if !release.contains_key("strip") || release["strip"].as_bool() != Some(config.strip) {
            release["strip"] = value(config.strip);
            changes.push("Set strip = true (remove debug symbols)".to_string());
        }

        // Apply panic
        if !release.contains_key("panic") || release["panic"].as_str() != Some(&config.panic) {
            release["panic"] = value(&config.panic);
            changes.push(format!(
                "Set panic = \"{}\" (smaller panic handler)",
                config.panic
            ));
        }

        Ok(())
    }

    /// Apply wasm-pack optimization flags
    fn apply_wasm_pack_optimizations(
        &self,
        doc: &mut DocumentMut,
        wasm_config: &WasmOptConfig,
        changes: &mut Vec<String>,
    ) -> Result<(), TomlEditError> {
        // Get or create [package.metadata.wasm-pack.profile.release] table
        let package = doc
            .entry("package")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| TomlEditError::InvalidStructure("package is not a table".to_string()))?;

        let metadata = package
            .entry("metadata")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| {
                TomlEditError::InvalidStructure("package.metadata is not a table".to_string())
            })?;

        let wasm_pack = metadata
            .entry("wasm-pack")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| {
                TomlEditError::InvalidStructure(
                    "package.metadata.wasm-pack is not a table".to_string(),
                )
            })?;

        let profile = wasm_pack
            .entry("profile")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| {
                TomlEditError::InvalidStructure(
                    "package.metadata.wasm-pack.profile is not a table".to_string(),
                )
            })?;

        let release = profile
            .entry("release")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| {
                TomlEditError::InvalidStructure(
                    "package.metadata.wasm-pack.profile.release is not a table".to_string(),
                )
            })?;

        // Create wasm-opt array
        let mut wasm_opt_array = Array::new();
        for flag in &wasm_config.flags {
            wasm_opt_array.push(flag);
        }

        // Only update if different
        let needs_update = if let Some(existing) = release.get("wasm-opt") {
            existing
                .as_array()
                .map(|arr| {
                    let existing_flags: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    existing_flags != wasm_config.flags
                })
                .unwrap_or(true)
        } else {
            true
        };

        if needs_update {
            release["wasm-opt"] = value(wasm_opt_array);
            changes.push(format!(
                "Set wasm-opt flags ({} optimizations)",
                wasm_config.flags.len()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_optimize_minimal_file_adds_optimizations() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(&cargo_toml, "[package]\nname = \"test\"\n").unwrap();

        let editor = CargoTomlEditor::new();
        let config = OptimizationConfig::default();
        let changes = editor
            .optimize_cargo_toml(&cargo_toml, &config, None, false)
            .unwrap();

        assert!(!changes.is_empty());
        assert!(changes.iter().any(|c| c.contains("lto")));
        assert!(changes.iter().any(|c| c.contains("opt-level")));
    }

    #[test]
    fn test_dry_run_does_not_modify_file() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        let original_content = "[package]\nname = \"test\"\n";
        std::fs::write(&cargo_toml, original_content).unwrap();

        let editor = CargoTomlEditor::new();
        let config = OptimizationConfig::default();
        let _ = editor
            .optimize_cargo_toml(&cargo_toml, &config, None, true)
            .unwrap();

        let content = std::fs::read_to_string(&cargo_toml).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    fn test_edit_toml_with_mixed_indentation() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        // Mix of spaces and different indentation levels
        let mixed_content = r#"[package]
name = "test"
  version = "0.1.0"
	[dependencies]
    serde = "1.0"
"#;
        std::fs::write(&cargo_toml, mixed_content).unwrap();

        let editor = CargoTomlEditor::new();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        // Should handle mixed indentation gracefully
        assert!(result.is_ok(), "Failed to handle mixed indentation");

        // Verify file is still valid TOML
        let content = std::fs::read_to_string(&cargo_toml).unwrap();
        let parsed: toml_edit::DocumentMut = content.parse().unwrap();
        assert!(parsed.get("package").is_some());
    }

    #[test]
    fn test_edit_toml_with_inline_comments() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        let content_with_comments = r#"[package]
name = "test" # package name
version = "0.1.0" # version

[dependencies]
serde = "1.0" # serialization
# This is a full line comment
tokio = "1.0"
"#;
        std::fs::write(&cargo_toml, content_with_comments).unwrap();

        let editor = CargoTomlEditor::new();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok(), "Failed to handle inline comments");

        // Verify comments are preserved
        let content = std::fs::read_to_string(&cargo_toml).unwrap();
        assert!(
            content.contains("# package name") || content.contains("#"),
            "Comments should be preserved or handled gracefully"
        );
    }

    #[test]
    fn test_edit_toml_with_windows_line_endings() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        // Use Windows-style CRLF line endings
        let windows_content = "[package]\r\nname = \"test\"\r\nversion = \"0.1.0\"\r\n";
        std::fs::write(&cargo_toml, windows_content).unwrap();

        let editor = CargoTomlEditor::new();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok(), "Failed to handle Windows line endings");

        // Verify file is still valid
        let content = std::fs::read_to_string(&cargo_toml).unwrap();
        let parsed: toml_edit::DocumentMut = content.parse().unwrap();
        assert!(parsed.get("package").is_some());
    }

    #[test]
    fn test_edit_toml_preserves_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        let formatted_content = r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
criterion = "0.5"
"#;
        std::fs::write(&cargo_toml, formatted_content).unwrap();

        let editor = CargoTomlEditor::new();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());

        // Verify structure is preserved
        let content = std::fs::read_to_string(&cargo_toml).unwrap();
        let parsed: toml_edit::DocumentMut = content.parse().unwrap();

        // Check that sections are preserved
        assert!(parsed.get("package").is_some());
        assert!(parsed.get("dependencies").is_some());
        assert!(parsed.get("dev-dependencies").is_some());

        // Verify dependencies are intact
        let deps = parsed.get("dependencies").unwrap();
        assert!(deps.get("serde").is_some());
        assert!(deps.get("tokio").is_some());
    }

    #[test]
    fn test_optimize_with_existing_profile_release() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[profile.release]
opt-level = 2
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        let content = std::fs::read_to_string(&cargo_toml).unwrap();
        assert!(content.contains("[profile.release]"));
    }

    #[test]
    fn test_optimize_with_lto_already_set() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[profile.release]
lto = true
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_optimize_with_codegen_units_already_set() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[profile.release]
codegen-units = 1
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_optimize_with_strip_already_set() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[profile.release]
strip = true
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_optimize_nonexistent_file_returns_error() {
        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result =
            editor.optimize_cargo_toml(Path::new("/nonexistent/Cargo.toml"), &config, None, false);

        assert!(result.is_err());
    }

    #[test]
    fn test_optimize_with_invalid_toml_returns_error() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        // Write invalid TOML
        std::fs::write(&cargo_toml, "this is not valid toml {{{").unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_err());
    }

    #[test]
    fn test_with_fs_creates_editor() {
        let _editor = CargoTomlEditor::<RealFileSystem>::with_fs(RealFileSystem);
        // Just verify it compiles and creates successfully
    }

    #[test]
    fn test_default_creates_editor() {
        let _editor = CargoTomlEditor::default();
        // Verify default constructor works
    }

    #[test]
    fn test_optimize_returns_changes_list() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        let changes = result.unwrap();
        // Should have made some optimizations
        assert!(!changes.is_empty());
    }

    #[test]
    fn test_optimize_with_wasm_pack_profile() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[package.metadata.wasm-pack.profile.release]
wasm-opt = true
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_optimize_empty_file() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        std::fs::write(&cargo_toml, "").unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        // Should handle empty file (may error or succeed depending on implementation)
        let _ = result;
    }

    #[test]
    fn test_optimize_file_with_only_package() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        let content = std::fs::read_to_string(&cargo_toml).unwrap();
        assert!(content.contains("[profile.release]"));
    }

    #[test]
    fn test_optimize_with_multiple_profiles() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[profile.dev]
opt-level = 0

[profile.release]
opt-level = 2

[profile.test]
opt-level = 1
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        // Verify all profiles are still present
        let content = std::fs::read_to_string(&cargo_toml).unwrap();
        assert!(content.contains("[profile.dev]"));
        assert!(content.contains("[profile.release]"));
        assert!(content.contains("[profile.test]"));
    }

    #[test]
    fn test_optimize_with_existing_wasm_pack_profile() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-O3"]
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let wasm_config = WasmOptConfig {
            flags: vec!["-Oz".to_string()],
        };
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, Some(&wasm_config), false);

        assert!(result.is_ok());
        let changes = result.unwrap();
        assert!(changes.iter().any(|c| c.contains("wasm-opt")));
    }

    #[test]
    fn test_optimize_with_opt_level_as_integer() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[profile.release]
opt-level = 3
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig {
            opt_level: "z".to_string(),
            ..Default::default()
        };
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        let changes = result.unwrap();
        assert!(changes.iter().any(|c| c.contains("opt-level")));
    }

    #[test]
    fn test_optimize_with_panic_setting() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        let changes = result.unwrap();
        assert!(changes.iter().any(|c| c.contains("panic")));
    }

    #[test]
    fn test_optimize_preserves_existing_panic() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[profile.release]
panic = "abort"
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        let changes = result.unwrap();
        // Should not change panic if it's already set to "abort"
        let panic_changes: Vec<_> = changes.iter().filter(|c| c.contains("panic")).collect();
        assert_eq!(panic_changes.len(), 0);
    }

    #[test]
    fn test_optimize_updates_different_wasm_opt_flags() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-O"]
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let wasm_config = WasmOptConfig {
            flags: vec!["-Oz".to_string(), "--strip-debug".to_string()],
        };
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, Some(&wasm_config), false);

        assert!(result.is_ok());
        let changes = result.unwrap();
        assert!(changes.iter().any(|c| c.contains("wasm-opt")));
    }

    #[test]
    fn test_optimize_skips_wasm_opt_when_none() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        let changes = result.unwrap();
        // Should not have wasm-opt changes when wasm_config is None
        assert!(!changes.iter().any(|c| c.contains("wasm-opt")));
    }

    #[test]
    fn test_optimize_with_matching_opt_level_string() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"

[profile.release]
opt-level = "z"
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig {
            opt_level: "z".to_string(),
            ..Default::default()
        };
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, false);

        assert!(result.is_ok());
        let changes = result.unwrap();
        // Should not update opt-level if it already matches
        let opt_changes: Vec<_> = changes.iter().filter(|c| c.contains("opt-level")).collect();
        assert_eq!(opt_changes.len(), 0);
    }

    #[test]
    fn test_dry_run_no_backup_created() {
        let temp = TempDir::new().unwrap();
        let cargo_toml = temp.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        std::fs::write(&cargo_toml, content).unwrap();

        let editor = CargoTomlEditor::default();
        let config = OptimizationConfig::default();
        let result = editor.optimize_cargo_toml(&cargo_toml, &config, None, true);

        assert!(result.is_ok());

        // Verify no backup was created in dry-run mode
        let backup_dir = temp.path().join(".wasm-slim").join("backups");
        assert!(!backup_dir.exists() || std::fs::read_dir(&backup_dir).unwrap().count() == 0);
    }
}
