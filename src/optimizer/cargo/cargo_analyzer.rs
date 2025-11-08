//! Cargo.toml analysis module
//!
//! Provides [`CargoAnalyzer`] for analyzing Cargo.toml files to detect
//! WASM projects and workspace structures.

use crate::infra::{FileSystem, RealFileSystem};
use std::path::Path;
use thiserror::Error;
use toml_edit::DocumentMut;

/// Errors that can occur during analysis
#[derive(Error, Debug)]
pub enum AnalysisError {
    /// I/O error reading Cargo.toml
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse TOML
    #[error("Failed to parse TOML: {0}")]
    TomlParse(#[from] toml_edit::TomlError),
}

/// Analyzes Cargo.toml files for WASM indicators and workspace structure
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::optimizer::cargo::CargoAnalyzer;
/// use std::path::Path;
///
/// let analyzer = CargoAnalyzer::new(Path::new("."));
/// if analyzer.is_wasm_crate(Path::new("Cargo.toml"))? {
///     println!("This is a WASM project!");
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct CargoAnalyzer<FS: FileSystem = RealFileSystem> {
    fs: FS,
}

impl CargoAnalyzer<RealFileSystem> {
    /// Create a new analyzer for the given project
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::optimizer::cargo::CargoAnalyzer;
    /// use std::path::Path;
    ///
    /// let analyzer = CargoAnalyzer::new(Path::new("."));
    /// ```
    pub fn new(project_root: &Path) -> Self {
        Self::with_fs(project_root, RealFileSystem)
    }
}

impl<FS: FileSystem> CargoAnalyzer<FS> {
    /// Create a new analyzer with a custom filesystem implementation
    pub fn with_fs(_project_root: &Path, fs: FS) -> Self {
        Self { fs }
    }

    /// Check if a Cargo.toml appears to be for a WASM crate
    ///
    /// Detects WASM projects by looking for:
    /// - `wasm-bindgen` dependency
    /// - `package.metadata.wasm-pack` section
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::optimizer::cargo::CargoAnalyzer;
    /// use std::path::Path;
    ///
    /// let analyzer = CargoAnalyzer::new(Path::new("."));
    /// let is_wasm = analyzer.is_wasm_crate(Path::new("Cargo.toml"))?;
    ///
    /// if is_wasm {
    ///     println!("Detected WASM project");
    /// } else {
    ///     println!("Not a WASM project");
    /// }
    /// # Ok::<(), wasm_slim::optimizer::cargo::AnalysisError>(())
    /// ```
    pub fn is_wasm_crate(&self, cargo_toml_path: &Path) -> Result<bool, AnalysisError> {
        let content = self.fs.read_to_string(cargo_toml_path)?;
        let doc: DocumentMut = content.parse()?;

        // Check for common WASM indicators
        let has_wasm_bindgen = doc
            .get("dependencies")
            .and_then(|d| d.as_table())
            .map(|table| table.contains_key("wasm-bindgen"))
            .unwrap_or(false);

        let has_wasm_pack_metadata = doc
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.get("wasm-pack"))
            .is_some();

        Ok(has_wasm_bindgen || has_wasm_pack_metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_wasm_crate_with_wasm_bindgen_returns_true() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"[package]
name = "test"

[dependencies]
wasm-bindgen = "0.2"
"#,
        )
        .unwrap();

        let analyzer = CargoAnalyzer::new(temp_dir.path());
        assert!(analyzer.is_wasm_crate(&cargo_toml).unwrap());
    }

    #[test]
    fn test_is_wasm_crate_with_wasm_pack_metadata_returns_true() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"[package]
name = "test"

[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz"]
"#,
        )
        .unwrap();

        let analyzer = CargoAnalyzer::new(temp_dir.path());
        assert!(analyzer.is_wasm_crate(&cargo_toml).unwrap());
    }

    #[test]
    fn test_is_wasm_crate_without_indicators_returns_false() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(&cargo_toml, "[package]\nname = \"test\"\n").unwrap();

        let analyzer = CargoAnalyzer::new(temp_dir.path());
        assert!(!analyzer.is_wasm_crate(&cargo_toml).unwrap());
    }
}
