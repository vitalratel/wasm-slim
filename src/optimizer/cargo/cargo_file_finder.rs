//! Cargo.toml file discovery module
//!
//! Provides [`CargoFileFinder`] for locating Cargo.toml files in projects
//! and workspaces.
//!
//! # Architecture Note
//!
//! `CargoFileFinder` is kept as a separate module (not merged into `CargoAnalyzer`) because:
//! - It's used independently by the CLI module (`cmd/build.rs`)
//! - It's a focused, reusable component with single responsibility (file discovery)
//! - Separation allows different components to find Cargo.toml files without depending on analysis logic

use crate::infra::{FileSystem, RealFileSystem};
use std::path::{Path, PathBuf};

/// Finds Cargo.toml files in projects and workspaces
///
/// This component provides file discovery functionality for locating
/// Cargo.toml files in single-crate projects and workspaces.
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::optimizer::cargo::CargoFileFinder;
/// use std::path::Path;
///
/// let finder = CargoFileFinder::new(Path::new("."));
/// let tomls = finder.find_cargo_tomls()?;
///
/// for toml in &tomls {
///     println!("Found Cargo.toml at: {}", toml.display());
/// }
/// # Ok::<(), std::io::Error>(())
/// ```
pub struct CargoFileFinder<FS: FileSystem = RealFileSystem> {
    project_root: PathBuf,
    fs: FS,
}

impl CargoFileFinder<RealFileSystem> {
    /// Create a new file finder for the given project
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::optimizer::cargo::CargoFileFinder;
    /// use std::path::Path;
    ///
    /// // Create a finder for the current directory
    /// let finder = CargoFileFinder::new(Path::new("."));
    ///
    /// // Create a finder for a specific project
    /// let finder = CargoFileFinder::new(Path::new("/path/to/project"));
    /// ```
    pub fn new(project_root: &Path) -> Self {
        Self::with_fs(project_root, RealFileSystem)
    }
}

impl<FS: FileSystem> CargoFileFinder<FS> {
    /// Create a new file finder with a custom filesystem implementation
    pub fn with_fs(project_root: &Path, fs: FS) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            fs,
        }
    }

    /// Find all Cargo.toml files in the project (workspace + members)
    ///
    /// Returns a list of paths to all Cargo.toml files found in the project.
    /// Currently finds the root Cargo.toml file. Workspace member support
    /// is planned for a future release.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::optimizer::cargo::CargoFileFinder;
    /// use std::path::Path;
    ///
    /// let finder = CargoFileFinder::new(Path::new("."));
    /// let tomls = finder.find_cargo_tomls()?;
    ///
    /// if !tomls.is_empty() {
    ///     println!("Found {} Cargo.toml file(s)", tomls.len());
    ///     for toml in &tomls {
    ///         println!("  - {}", toml.display());
    ///     }
    /// } else {
    ///     println!("No Cargo.toml found in this directory");
    /// }
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn find_cargo_tomls(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut tomls = Vec::new();

        // Always include root Cargo.toml
        let root_toml = self.project_root.join("Cargo.toml");
        if self.fs.metadata(&root_toml).is_ok() {
            tomls.push(root_toml);
        }

        // Note: Workspace member support planned for future release
        // Currently finds root Cargo.toml only (sufficient for most single-crate projects)
        // See: https://github.com/anthropics/wasm-slim/issues/TBD for workspace support

        Ok(tomls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_cargo_tomls_in_empty_directory_returns_empty_list() {
        let temp_dir = TempDir::new().unwrap();
        let finder = CargoFileFinder::new(temp_dir.path());

        let tomls = finder.find_cargo_tomls().unwrap();
        assert!(tomls.is_empty());
    }

    #[test]
    fn test_find_cargo_tomls_with_root_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(&cargo_toml, "[package]\nname = \"test\"\n").unwrap();

        let finder = CargoFileFinder::new(temp_dir.path());
        let tomls = finder.find_cargo_tomls().unwrap();

        assert_eq!(tomls.len(), 1);
        assert_eq!(tomls[0], cargo_toml);
    }
}
