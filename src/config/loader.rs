//! Configuration file loading and saving

use super::file::{ConfigFile, CONFIG_FILE_NAME};
use crate::infra::{FileSystem, RealFileSystem};
use anyhow::{Context, Result};
use std::path::Path;

/// Handles loading and saving configuration files
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load config from .wasm-slim.toml in the given directory
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::config::ConfigLoader;
    /// use std::path::Path;
    ///
    /// let config = ConfigLoader::load(Path::new("."))?;
    /// println!("Loaded config with template: {:?}", config.template);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn load(project_root: &Path) -> Result<ConfigFile> {
        Self::load_with_fs(project_root, &RealFileSystem)
    }

    /// Load config with a custom filesystem implementation
    pub fn load_with_fs<FS: FileSystem>(project_root: &Path, fs: &FS) -> Result<ConfigFile> {
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // Read file atomically - no TOCTOU race window
        let contents = match fs.read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Return default config if file doesn't exist
                return Ok(ConfigFile::default());
            }
            Err(e) => {
                return Err(e).context("Failed to read .wasm-slim.toml");
            }
        };

        let config: ConfigFile =
            toml_edit::de::from_str(&contents).context("Failed to parse .wasm-slim.toml")?;

        // Validate size budget constraints
        if let Some(ref budget) = config.size_budget {
            budget
                .validate()
                .context("Invalid size budget configuration")?;
        }

        Ok(config)
    }

    /// Save config to .wasm-slim.toml in the given directory
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::config::{ConfigFile, ConfigLoader};
    /// use std::path::Path;
    ///
    /// let mut config = ConfigFile::default();
    /// config.template = "minimal".to_string();
    /// ConfigLoader::save(&config, Path::new("."))?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn save(config: &ConfigFile, project_root: &Path) -> Result<()> {
        Self::save_with_fs(config, project_root, &RealFileSystem)
    }

    /// Save config with a custom filesystem implementation
    pub fn save_with_fs<FS: FileSystem>(
        config: &ConfigFile,
        project_root: &Path,
        fs: &FS,
    ) -> Result<()> {
        let config_path = project_root.join(CONFIG_FILE_NAME);

        let contents =
            toml_edit::ser::to_string_pretty(config).context("Failed to serialize config")?;

        fs.write(&config_path, contents)
            .context("Failed to write .wasm-slim.toml")?;

        Ok(())
    }

    /// Check if config file exists in project
    pub fn exists(project_root: &Path) -> bool {
        project_root.join(CONFIG_FILE_NAME).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    // Mock FileSystem for testing
    struct MockFileSystem {
        file_content: Option<String>,
        should_fail_read: bool,
        should_fail_write: bool,
        written_content: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    }

    impl MockFileSystem {
        fn new() -> Self {
            Self {
                file_content: None,
                should_fail_read: false,
                should_fail_write: false,
                written_content: std::sync::Arc::new(std::sync::Mutex::new(None)),
            }
        }

        fn with_content(content: &str) -> Self {
            Self {
                file_content: Some(content.to_string()),
                should_fail_read: false,
                should_fail_write: false,
                written_content: std::sync::Arc::new(std::sync::Mutex::new(None)),
            }
        }

        fn with_read_error() -> Self {
            Self {
                file_content: None,
                should_fail_read: true,
                should_fail_write: false,
                written_content: std::sync::Arc::new(std::sync::Mutex::new(None)),
            }
        }

        fn with_write_error() -> Self {
            Self {
                file_content: None,
                should_fail_read: false,
                should_fail_write: true,
                written_content: std::sync::Arc::new(std::sync::Mutex::new(None)),
            }
        }

        fn get_written_content(&self) -> Option<String> {
            self.written_content.lock().unwrap().clone()
        }
    }

    impl FileSystem for MockFileSystem {
        fn read_to_string(&self, _path: &Path) -> io::Result<String> {
            if self.should_fail_read {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "permission denied",
                ));
            }
            self.file_content
                .clone()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "file not found"))
        }

        fn write(&self, _path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
            if self.should_fail_write {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "permission denied",
                ));
            }
            let contents_str = String::from_utf8_lossy(contents.as_ref()).to_string();
            *self.written_content.lock().unwrap() = Some(contents_str);
            Ok(())
        }

        fn metadata(&self, _path: &Path) -> io::Result<std::fs::Metadata> {
            unimplemented!()
        }

        fn read_dir(&self, _path: &Path) -> io::Result<std::fs::ReadDir> {
            unimplemented!()
        }

        fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
            unimplemented!()
        }

        fn copy(&self, _from: &Path, _to: &Path) -> io::Result<u64> {
            unimplemented!()
        }
    }

    #[test]
    fn test_loader_loads_from_valid_toml() {
        // Use real filesystem with tempdir for this test
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join(CONFIG_FILE_NAME);

        let toml_content = r#"
template = "balanced"

[profile]
opt-level = "z"
"#;
        std::fs::write(&config_path, toml_content).unwrap();

        let result = ConfigLoader::load(temp.path());
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.template, "balanced");
        assert_eq!(
            config.profile.as_ref().unwrap().opt_level.as_ref().unwrap(),
            "z"
        );
    }

    #[test]
    fn test_loader_with_missing_file_uses_defaults() {
        let fs = MockFileSystem::new();
        let result = ConfigLoader::load_with_fs(Path::new("/test"), &fs);

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.template, ConfigFile::default().template);
    }

    #[test]
    fn test_loader_with_invalid_toml_returns_error() {
        // Use real filesystem with tempdir
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join(CONFIG_FILE_NAME);

        let invalid_toml = "invalid { toml syntax";
        std::fs::write(&config_path, invalid_toml).unwrap();

        let result = ConfigLoader::load(temp.path());
        assert!(result.is_err(), "Expected error for invalid TOML");
    }

    #[test]
    fn test_loader_with_permission_error_returns_error() {
        let fs = MockFileSystem::with_read_error();
        let result = ConfigLoader::load_with_fs(Path::new("/test"), &fs);

        // After TOCTOU fix: read_to_string is called directly
        // PermissionDenied error should be propagated, not swallowed
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read") || err_msg.contains("permission"));
    }

    #[test]
    fn test_save_writes_valid_toml() {
        use super::super::file::ProfileSettings;

        let config = ConfigFile {
            template: "balanced".to_string(),
            profile: Some(ProfileSettings {
                opt_level: Some("3".to_string()),
                lto: None,
                strip: None,
                codegen_units: None,
                panic: None,
            }),
            ..Default::default()
        };

        let fs = MockFileSystem::new();
        let result = ConfigLoader::save_with_fs(&config, Path::new("/test"), &fs);

        assert!(result.is_ok());
        let written = fs.get_written_content();
        assert!(written.is_some());

        let content = written.unwrap();
        assert!(content.contains("balanced"));
        assert!(content.contains("profile"));
    }

    #[test]
    fn test_save_with_write_error_returns_error() {
        let config = ConfigFile::default();
        let fs = MockFileSystem::with_write_error();
        let result = ConfigLoader::save_with_fs(&config, Path::new("/test"), &fs);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to write"));
    }

    #[test]
    fn test_save_preserves_all_config_fields() {
        use super::super::file::ProfileSettings;

        // Use real filesystem with tempdir
        let temp = tempfile::tempdir().unwrap();

        let config = ConfigFile {
            template: "aggressive".to_string(),
            profile: Some(ProfileSettings {
                opt_level: Some("s".to_string()),
                lto: None,
                strip: None,
                codegen_units: None,
                panic: None,
            }),
            ..Default::default()
        };

        ConfigLoader::save(&config, temp.path()).unwrap();

        // Verify round-trip
        let loaded = ConfigLoader::load(temp.path()).unwrap();

        assert_eq!(loaded.template, config.template);
        assert_eq!(
            loaded.profile.as_ref().unwrap().opt_level,
            config.profile.unwrap().opt_level
        );
    }

    #[test]
    fn test_exists_returns_false_for_missing_file() {
        let temp = tempfile::tempdir().unwrap();
        assert!(!ConfigLoader::exists(temp.path()));
    }

    #[test]
    fn test_exists_returns_true_when_file_present() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join(CONFIG_FILE_NAME);
        std::fs::write(&config_path, "template = \"test\"").unwrap();

        assert!(ConfigLoader::exists(temp.path()));
    }

    #[test]
    fn test_loader_handles_empty_file() {
        let fs = MockFileSystem::with_content("");
        let result = ConfigLoader::load_with_fs(Path::new("/test"), &fs);

        // Empty file should parse to default config
        assert!(result.is_ok());
    }

    #[test]
    fn test_loader_handles_partial_config() {
        let toml_content = r#"template = "balanced""#;
        let fs = MockFileSystem::with_content(toml_content);
        let result = ConfigLoader::load_with_fs(Path::new("/test"), &fs);

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.template, "balanced");
        // Other fields should have default values (None)
        assert!(config.profile.is_none());
    }
}
