//! Build-std optimizer for nightly Rust toolchains
//!
//! Provides [`BuildStdOptimizer`] for configuring `.cargo/config.toml` to use
//! the `build-std` unstable feature with `panic_immediate_abort`, which can
//! provide an additional 10-20% WASM size reduction.
//!
//! **Requires nightly Rust toolchain.**
//!
//! # Examples
//!
//! ```no_run
//! # use wasm_slim::optimizer::BuildStdOptimizer;
//! # use std::path::Path;
//! let optimizer = BuildStdOptimizer::new(Path::new("."));
//! let config = Default::default();
//!
//! // Check nightly before applying
//! if wasm_slim::toolchain::ToolchainDetector::new().is_nightly_toolchain().unwrap_or(false) {
//!     optimizer.apply_build_std(&config, false)?;
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Background
//!
//! From Leptos binary size guide:
//! - `build-std` rebuilds standard library with your settings
//! - `panic_immediate_abort` eliminates panic infrastructure (~10-20% reduction)
//! - Only works on nightly Rust
//!
//! Reference: <https://book.leptos.dev/deployment/binary_size.html>

use crate::infra::{FileSystem, RealFileSystem};
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml_edit::{value, Array, DocumentMut, Item, Table};

/// Errors that can occur during build-std configuration
#[derive(Error, Debug)]
pub enum BuildStdError {
    /// I/O error reading or writing .cargo/config.toml
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse TOML
    #[error("Failed to parse TOML: {0}")]
    TomlParse(#[from] toml_edit::TomlError),

    /// Invalid config.toml structure
    #[error("Invalid config.toml structure: {0}")]
    InvalidStructure(String),

    /// Nightly toolchain required
    #[error("Build-std requires nightly Rust toolchain (detected stable/beta)")]
    NightlyRequired,
}

/// Configuration for build-std optimization
#[derive(Debug, Clone)]
pub struct BuildStdConfig {
    /// Enable build-std feature
    pub enabled: bool,

    /// Standard library components to rebuild
    /// Default: ["std", "panic_abort", "core", "alloc"]
    pub std_components: Vec<String>,

    /// Build-std features to enable
    /// Default: ["panic_immediate_abort"]
    pub features: Vec<String>,

    /// Target triple for SSR projects (optional)
    /// Example: "x86_64-unknown-linux-gnu"
    pub target: Option<String>,

    /// Additional rustflags
    /// Default: ["--cfg=has_std"] for SSR projects
    pub rustflags: Vec<String>,
}

impl Default for BuildStdConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in only (requires nightly)
            std_components: vec![
                "std".to_string(),
                "panic_abort".to_string(),
                "core".to_string(),
                "alloc".to_string(),
            ],
            features: vec!["panic_immediate_abort".to_string()],
            target: None,          // Auto-detect if needed
            rustflags: Vec::new(), // Empty by default
        }
    }
}

impl BuildStdConfig {
    /// Create config with SSR support
    ///
    /// Adds target specification and rustflags for server-side rendering projects.
    ///
    /// # Arguments
    /// * `target` - Target triple (e.g., "x86_64-unknown-linux-gnu")
    pub fn with_ssr(target: String) -> Self {
        Self {
            enabled: true,
            target: Some(target),
            rustflags: vec!["--cfg=has_std".to_string()],
            ..Default::default()
        }
    }

    /// Create minimal config for pure WASM projects
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }
}

/// Optimizer for .cargo/config.toml build-std settings
///
/// Configures unstable build-std feature for nightly Rust, providing
/// additional 10-20% WASM size reduction through panic_immediate_abort.
///
/// # Examples
///
/// Basic usage:
/// ```no_run
/// use wasm_slim::optimizer::{BuildStdOptimizer, BuildStdConfig};
/// use std::path::Path;
///
/// let optimizer = BuildStdOptimizer::new(Path::new("."));
/// let config = BuildStdConfig::default();
///
/// // Creates .cargo/config.toml with build-std settings
/// optimizer.apply_build_std(&config, false)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// SSR project:
/// ```no_run
/// use wasm_slim::optimizer::{BuildStdOptimizer, BuildStdConfig};
/// use std::path::Path;
///
/// let optimizer = BuildStdOptimizer::new(Path::new("."));
/// let config = BuildStdConfig::with_ssr("x86_64-unknown-linux-gnu".to_string());
///
/// optimizer.apply_build_std(&config, false)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct BuildStdOptimizer<FS: FileSystem = RealFileSystem> {
    project_root: PathBuf,
    fs: FS,
}

impl BuildStdOptimizer<RealFileSystem> {
    /// Create a new build-std optimizer for the given project
    pub fn new(project_root: &Path) -> Self {
        Self::with_fs(project_root, RealFileSystem)
    }
}

impl<FS: FileSystem> BuildStdOptimizer<FS> {
    /// Create a new build-std optimizer with a custom filesystem implementation
    pub fn with_fs(project_root: &Path, fs: FS) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            fs,
        }
    }

    /// Get the path to .cargo/config.toml
    fn config_path(&self) -> PathBuf {
        self.project_root.join(".cargo").join("config.toml")
    }

    /// Apply build-std configuration to .cargo/config.toml
    ///
    /// Creates `.cargo/` directory if needed, and adds or updates build-std settings.
    ///
    /// # Arguments
    /// * `config` - Build-std configuration
    /// * `dry_run` - If true, return changes without modifying files
    ///
    /// # Returns
    /// List of changes applied (for user feedback)
    ///
    /// # Errors
    /// Returns error if:
    /// - File cannot be read/written
    /// - TOML is malformed
    /// - Config structure is invalid
    ///
    /// # Concurrency
    /// This function is NOT safe for concurrent execution on the same project directory.
    /// In typical single-user development scenarios, this is not a concern. If multiple
    /// processes might call this simultaneously (e.g., parallel CI builds in shared workspace),
    /// external synchronization should be used.
    pub fn apply_build_std(
        &self,
        config: &BuildStdConfig,
        dry_run: bool,
    ) -> Result<Vec<String>, BuildStdError> {
        if !config.enabled {
            return Ok(Vec::new());
        }

        let cargo_dir = self.project_root.join(".cargo");
        let config_path = self.config_path();

        // Create .cargo directory if needed
        if !dry_run && !cargo_dir.exists() {
            self.fs.create_dir_all(&cargo_dir)?;
        }

        // Load existing config or create new
        // Note: Using atomic read instead of exists() check to avoid TOCTOU race condition.
        // If file doesn't exist, read_to_string fails and we use empty string.
        let content = self
            .fs
            .read_to_string(&config_path)
            .unwrap_or_else(|_| String::new());

        let mut doc = if content.is_empty() {
            DocumentMut::new()
        } else {
            content.parse::<DocumentMut>()?
        };

        let mut changes = Vec::new();

        // Apply [unstable] section
        self.apply_unstable_section(&mut doc, config, &mut changes)?;

        // Apply [build] section if target specified
        if config.target.is_some() || !config.rustflags.is_empty() {
            self.apply_build_section(&mut doc, config, &mut changes)?;
        }

        if !dry_run && !changes.is_empty() {
            self.fs.write(&config_path, doc.to_string())?;
        }

        Ok(changes)
    }

    /// Apply [unstable] section with build-std settings
    fn apply_unstable_section(
        &self,
        doc: &mut DocumentMut,
        config: &BuildStdConfig,
        changes: &mut Vec<String>,
    ) -> Result<(), BuildStdError> {
        let unstable = doc
            .entry("unstable")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| {
                BuildStdError::InvalidStructure("unstable is not a table".to_string())
            })?;

        // Apply build-std
        if !unstable.contains_key("build-std") {
            let mut arr = Array::new();
            for component in &config.std_components {
                arr.push(component.as_str());
            }
            unstable["build-std"] = value(arr);
            changes.push(format!(
                "Set build-std = {:?} (10-20% reduction)",
                config.std_components
            ));
        }

        // Apply build-std-features
        if !config.features.is_empty() && !unstable.contains_key("build-std-features") {
            let mut arr = Array::new();
            for feature in &config.features {
                arr.push(feature.as_str());
            }
            unstable["build-std-features"] = value(arr);
            changes.push(format!(
                "Set build-std-features = {:?} (smaller panic handler)",
                config.features
            ));
        }

        Ok(())
    }

    /// Apply [build] section for SSR projects
    fn apply_build_section(
        &self,
        doc: &mut DocumentMut,
        config: &BuildStdConfig,
        changes: &mut Vec<String>,
    ) -> Result<(), BuildStdError> {
        let build = doc
            .entry("build")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| BuildStdError::InvalidStructure("build is not a table".to_string()))?;

        // Apply target
        if let Some(ref target) = config.target {
            if !build.contains_key("target") {
                build["target"] = value(target.as_str());
                changes.push(format!("Set target = \"{}\" (SSR support)", target));
            }
        }

        // Apply rustflags
        if !config.rustflags.is_empty() && !build.contains_key("rustflags") {
            let mut arr = Array::new();
            for flag in &config.rustflags {
                arr.push(flag.as_str());
            }
            build["rustflags"] = value(arr);
            changes.push(format!(
                "Set rustflags = {:?} (SSR compatibility)",
                config.rustflags
            ));
        }

        Ok(())
    }

    /// Check if build-std is already configured
    pub fn is_configured(&self) -> Result<bool, BuildStdError> {
        let config_path = self.config_path();

        if !config_path.exists() {
            return Ok(false);
        }

        let content = self.fs.read_to_string(&config_path)?;
        let doc = content.parse::<DocumentMut>()?;

        // Check if [unstable] section has build-std
        if let Some(unstable) = doc.get("unstable").and_then(|i| i.as_table()) {
            if unstable.contains_key("build-std") {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_build_std_config_default_has_expected_values() {
        let config = BuildStdConfig::default();
        assert!(!config.enabled); // Opt-in only
        assert_eq!(config.std_components.len(), 4);
        assert!(config.std_components.contains(&"std".to_string()));
        assert!(config.std_components.contains(&"panic_abort".to_string()));
        assert_eq!(config.features, vec!["panic_immediate_abort"]);
    }

    #[test]
    fn test_build_std_config_with_ssr_includes_target_and_rustflags() {
        let config = BuildStdConfig::with_ssr("x86_64-unknown-linux-gnu".to_string());
        assert!(config.enabled);
        assert_eq!(config.target, Some("x86_64-unknown-linux-gnu".to_string()));
        assert_eq!(config.rustflags, vec!["--cfg=has_std"]);
    }

    #[test]
    fn test_build_std_config_minimal_enables_feature() {
        let config = BuildStdConfig::minimal();
        assert!(config.enabled);
        assert!(config.target.is_none());
        assert!(config.rustflags.is_empty());
    }

    #[test]
    fn test_optimizer_creates_config_toml_with_build_std() {
        let temp_dir = TempDir::new().unwrap();
        let optimizer = BuildStdOptimizer::new(temp_dir.path());

        let config = BuildStdConfig {
            enabled: true,
            ..Default::default()
        };

        let changes = optimizer.apply_build_std(&config, false).unwrap();

        assert!(!changes.is_empty());
        assert!(changes.iter().any(|c| c.contains("build-std")));

        // Verify file created
        let config_path = temp_dir.path().join(".cargo").join("config.toml");
        assert!(config_path.exists());

        // Verify content
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[unstable]"));
        assert!(content.contains("build-std"));
        assert!(content.contains("panic_immediate_abort"));
    }

    #[test]
    fn test_optimizer_with_ssr_adds_build_section() {
        let temp_dir = TempDir::new().unwrap();
        let optimizer = BuildStdOptimizer::new(temp_dir.path());

        let config = BuildStdConfig::with_ssr("x86_64-unknown-linux-gnu".to_string());

        let changes = optimizer.apply_build_std(&config, false).unwrap();

        assert!(changes.iter().any(|c| c.contains("target")));
        assert!(changes.iter().any(|c| c.contains("rustflags")));

        // Verify content
        let config_path = temp_dir.path().join(".cargo").join("config.toml");
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[build]"));
        assert!(content.contains("x86_64-unknown-linux-gnu"));
        assert!(content.contains("--cfg=has_std"));
    }

    #[test]
    fn test_optimizer_preserves_existing_config() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_dir = temp_dir.path().join(".cargo");
        fs::create_dir_all(&cargo_dir).unwrap();

        let config_path = cargo_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"
[some-existing]
setting = "value"
"#,
        )
        .unwrap();

        let optimizer = BuildStdOptimizer::new(temp_dir.path());
        let config = BuildStdConfig {
            enabled: true,
            ..Default::default()
        };

        optimizer.apply_build_std(&config, false).unwrap();

        // Verify existing content preserved
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[some-existing]"));
        assert!(content.contains("setting = \"value\""));
        assert!(content.contains("[unstable]"));
    }

    #[test]
    fn test_optimizer_dry_run_does_not_modify_files() {
        let temp_dir = TempDir::new().unwrap();
        let optimizer = BuildStdOptimizer::new(temp_dir.path());

        let config = BuildStdConfig {
            enabled: true,
            ..Default::default()
        };

        let changes = optimizer.apply_build_std(&config, true).unwrap();

        assert!(!changes.is_empty());

        // Verify no file created
        let config_path = temp_dir.path().join(".cargo").join("config.toml");
        assert!(!config_path.exists());
    }

    #[test]
    fn test_optimizer_is_configured_detects_existing_setup() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_dir = temp_dir.path().join(".cargo");
        fs::create_dir_all(&cargo_dir).unwrap();

        let config_path = cargo_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"
[unstable]
build-std = ["std", "panic_abort"]
"#,
        )
        .unwrap();

        let optimizer = BuildStdOptimizer::new(temp_dir.path());
        assert!(optimizer.is_configured().unwrap());
    }

    #[test]
    fn test_optimizer_is_configured_returns_false_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let optimizer = BuildStdOptimizer::new(temp_dir.path());
        assert!(!optimizer.is_configured().unwrap());
    }

    #[test]
    fn test_optimizer_disabled_config_returns_no_changes() {
        let temp_dir = TempDir::new().unwrap();
        let optimizer = BuildStdOptimizer::new(temp_dir.path());

        let config = BuildStdConfig::default(); // enabled = false
        let changes = optimizer.apply_build_std(&config, false).unwrap();

        assert!(changes.is_empty());
    }

    #[test]
    fn test_invalid_toml_structure_returns_error() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let cargo_dir = temp_dir.path().join(".cargo");
        fs::create_dir_all(&cargo_dir).unwrap();

        // Create config with invalid structure (unstable is not a table)
        let config_path = cargo_dir.join("config.toml");
        fs::write(&config_path, "unstable = \"not a table\"").unwrap();

        let optimizer = BuildStdOptimizer::new(temp_dir.path());
        let config = BuildStdConfig {
            enabled: true,
            ..Default::default()
        };

        let result = optimizer.apply_build_std(&config, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, BuildStdError::InvalidStructure(_)));
    }

    #[test]
    fn test_toml_parse_error_returns_error() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let cargo_dir = temp_dir.path().join(".cargo");
        fs::create_dir_all(&cargo_dir).unwrap();

        // Create malformed TOML
        let config_path = cargo_dir.join("config.toml");
        fs::write(&config_path, "[unstable\ninvalid toml {{").unwrap();

        let optimizer = BuildStdOptimizer::new(temp_dir.path());
        let config = BuildStdConfig {
            enabled: true,
            ..Default::default()
        };

        let result = optimizer.apply_build_std(&config, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, BuildStdError::TomlParse(_)));
    }

    #[test]
    fn test_build_section_invalid_structure_returns_error() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let cargo_dir = temp_dir.path().join(".cargo");
        fs::create_dir_all(&cargo_dir).unwrap();

        // Create config with invalid build section structure
        let config_path = cargo_dir.join("config.toml");
        fs::write(&config_path, "[build]\nbuild = \"not a table\"").unwrap();

        let optimizer = BuildStdOptimizer::new(temp_dir.path());
        let config = BuildStdConfig {
            enabled: true,
            target: Some("x86_64-unknown-linux-gnu".to_string()),
            ..Default::default()
        };

        // This should succeed because build section is created fresh
        // But if we have invalid structure, it fails
        let config_path = cargo_dir.join("config.toml");
        fs::write(&config_path, "build = \"not a table\"").unwrap();

        let result = optimizer.apply_build_std(&config, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, BuildStdError::InvalidStructure(_)));
    }

    #[test]
    #[ignore = "Fails when running as root in Docker/CI containers"]
    fn test_readonly_directory_returns_io_error() {
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let cargo_dir = temp_dir.path().join(".cargo");

        // Create directory and make it read-only (Unix-specific test)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::create_dir_all(&cargo_dir).unwrap();
            let mut perms = fs::metadata(&cargo_dir).unwrap().permissions();
            perms.set_mode(0o444); // Read-only
            fs::set_permissions(&cargo_dir, perms).unwrap();

            let optimizer = BuildStdOptimizer::new(temp_dir.path());
            let config = BuildStdConfig {
                enabled: true,
                ..Default::default()
            };

            let result = optimizer.apply_build_std(&config, false);

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&cargo_dir).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&cargo_dir, perms).unwrap();

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), BuildStdError::Io(_)));
        }

        // On Windows, skip this test
        #[cfg(not(unix))]
        {
            // Test passes automatically on Windows
        }
    }
}
