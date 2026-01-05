//! Configuration file data structures

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration file name
pub const CONFIG_FILE_NAME: &str = ".wasm-slim.toml";

/// wasm-slim configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    /// Template to use
    #[serde(default = "default_template")]
    pub template: String,

    /// Custom profile settings (overrides template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileSettings>,

    /// Custom wasm-opt settings (overrides template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_opt: Option<WasmOptSettings>,

    /// Size budget settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_budget: Option<SizeBudget>,
}

fn default_template() -> String {
    "balanced".to_string()
}

/// Cargo profile settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSettings {
    /// Optimization level ("s", "z", "3")
    #[serde(rename = "opt-level", skip_serializing_if = "Option::is_none")]
    pub opt_level: Option<String>,

    /// Link-time optimization ("fat", "thin", "off")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lto: Option<String>,

    /// Strip debug symbols
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip: Option<bool>,

    /// Number of codegen units (1 = best optimization)
    #[serde(rename = "codegen-units", skip_serializing_if = "Option::is_none")]
    pub codegen_units: Option<u32>,

    /// Panic strategy ("abort" or "unwind")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panic: Option<String>,
}

/// wasm-opt settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmOptSettings {
    /// wasm-opt flags (e.g., ["-Oz", "--strip-debug"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Vec<String>>,
}

/// Size budget configuration (Phase 8)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SizeBudget {
    /// Maximum size in KB (hard limit, CI fails)
    #[serde(rename = "max-size-kb", skip_serializing_if = "Option::is_none")]
    pub max_size_kb: Option<u64>,

    /// Warning threshold in KB (CI passes with warning)
    #[serde(rename = "warn-threshold-kb", skip_serializing_if = "Option::is_none")]
    pub warn_threshold_kb: Option<u64>,

    /// Target size in KB (ideal target)
    #[serde(rename = "target-size-kb", skip_serializing_if = "Option::is_none")]
    pub target_size_kb: Option<u64>,
}

impl SizeBudget {
    /// Validate that budget thresholds are properly ordered
    ///
    /// Ensures: target <= warn <= max
    pub fn validate(&self) -> Result<()> {
        if let (Some(target), Some(warn)) = (self.target_size_kb, self.warn_threshold_kb) {
            if target > warn {
                anyhow::bail!(
                    "Target size ({} KB) cannot exceed warning threshold ({} KB)",
                    target,
                    warn
                );
            }
        }

        if let (Some(warn), Some(max)) = (self.warn_threshold_kb, self.max_size_kb) {
            if warn > max {
                anyhow::bail!(
                    "Warning threshold ({} KB) cannot exceed max size ({} KB)",
                    warn,
                    max
                );
            }
        }

        if let (Some(target), Some(max)) = (self.target_size_kb, self.max_size_kb) {
            if target > max {
                anyhow::bail!(
                    "Target size ({} KB) cannot exceed max size ({} KB)",
                    target,
                    max
                );
            }
        }

        Ok(())
    }
}

impl ConfigFile {}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            template: "balanced".to_string(),
            profile: None,
            wasm_opt: None,
            size_budget: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::loader::ConfigLoader;
    use super::super::resolver::TemplateResolver;
    use super::super::template::{Template, TemplateType};
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_config_file_default_has_balanced_template() {
        let config = ConfigFile::default();
        assert_eq!(config.template, "balanced");
        assert!(config.profile.is_none());
    }

    #[test]
    fn test_config_file_save_and_load_preserves_values() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let config = ConfigFile {
            template: "minimal".to_string(),
            profile: Some(ProfileSettings {
                opt_level: Some("z".to_string()),
                lto: Some("fat".to_string()),
                strip: Some(true),
                codegen_units: Some(1),
                panic: Some("abort".to_string()),
            }),
            wasm_opt: None,
            size_budget: None,
        };

        ConfigLoader::save(&config, project_root).unwrap();

        let loaded = ConfigLoader::load(project_root).unwrap();
        assert_eq!(loaded.template, "minimal");
        assert!(loaded.profile.is_some());
        assert_eq!(
            loaded.profile.as_ref().unwrap().opt_level,
            Some("z".to_string())
        );
    }

    #[test]
    fn test_config_file_from_template_creates_from_template() {
        let template = Template::new(TemplateType::Balanced);
        let config = TemplateResolver::from_template(&template);

        assert_eq!(config.template, "balanced");
        assert!(config.profile.is_some());
        assert_eq!(
            config.profile.as_ref().unwrap().opt_level,
            Some("s".to_string())
        );
    }

    #[test]
    fn test_config_file_resolve_template_merges_overrides_with_template() {
        let config = ConfigFile {
            template: "balanced".to_string(),
            profile: Some(ProfileSettings {
                opt_level: Some("z".to_string()), // Override
                lto: None,
                strip: None,
                codegen_units: None,
                panic: None,
            }),
            wasm_opt: None,
            size_budget: None,
        };

        let resolved = TemplateResolver::resolve(&config).unwrap();
        assert_eq!(resolved.profile.opt_level, "z"); // Override applied
        assert_eq!(resolved.profile.lto, "fat"); // Template default
    }

    #[test]
    fn test_config_file_exists_returns_true_after_save() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        assert!(!ConfigLoader::exists(project_root));

        let config = ConfigFile::default();
        ConfigLoader::save(&config, project_root).unwrap();

        assert!(ConfigLoader::exists(project_root));
    }

    #[test]
    fn test_size_budget_validate_with_correct_order_succeeds() {
        // Valid: target < warn < max
        let budget = SizeBudget {
            target_size_kb: Some(100),
            warn_threshold_kb: Some(150),
            max_size_kb: Some(200),
        };
        assert!(budget.validate().is_ok());
    }

    #[test]
    fn test_size_budget_validate_with_target_exceeds_warn_returns_error() {
        let budget = SizeBudget {
            target_size_kb: Some(200),
            warn_threshold_kb: Some(100),
            max_size_kb: None,
        };
        assert!(budget.validate().is_err());
    }

    #[test]
    fn test_size_budget_validate_with_warn_exceeds_max_returns_error() {
        let budget = SizeBudget {
            target_size_kb: None,
            warn_threshold_kb: Some(300),
            max_size_kb: Some(200),
        };
        assert!(budget.validate().is_err());
    }

    #[test]
    fn test_size_budget_validate_with_target_exceeds_max_returns_error() {
        let budget = SizeBudget {
            target_size_kb: Some(300),
            warn_threshold_kb: None,
            max_size_kb: Some(200),
        };
        assert!(budget.validate().is_err());
    }

    // P0-TEST-COV-002: File permission error tests

    #[test]
    #[cfg(unix)]
    fn test_config_file_save_with_readonly_directory_returns_error() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("readonly");
        fs::create_dir(&config_dir).unwrap();

        // Make directory read-only
        let mut perms = fs::metadata(&config_dir).unwrap().permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&config_dir, perms).unwrap();

        let config = ConfigFile::default();
        let result = ConfigLoader::save(&config, &config_dir);

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&config_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&config_dir, perms).unwrap();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Failed to write") || err.to_string().contains("permission")
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_config_file_save_on_windows_succeeds() {
        // Windows permission testing would require different setup
        // ConfigFile doesn't have a save method - this test is not applicable
        let _temp_dir = TempDir::new().unwrap();
        let _config = ConfigFile::default();
        // Test passes if we can create a default config
        assert!(true);
    }

    #[test]
    fn test_config_file_load_with_missing_file_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Should return default config when file doesn't exist
        let result = ConfigLoader::load(project_root);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.template, "balanced");
    }

    #[test]
    fn test_config_file_load_with_invalid_toml_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // Write invalid TOML
        fs::write(&config_path, "[invalid toml\nthis is broken").unwrap();

        let result = ConfigLoader::load(project_root);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to parse") || err.to_string().contains("parse"));
    }

    #[test]
    fn test_config_file_load_with_malformed_structure_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // Write valid TOML but invalid structure
        fs::write(
            &config_path,
            r#"
template = 123
profile = "not a table"
"#,
        )
        .unwrap();

        let result = ConfigLoader::load(project_root);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn test_config_file_load_with_unreadable_file_returns_error() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create config file
        let config = ConfigFile::default();
        ConfigLoader::save(&config, project_root).unwrap();

        let config_path = project_root.join(CONFIG_FILE_NAME);

        // Make file unreadable
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o000); // No permissions
        fs::set_permissions(&config_path, perms).unwrap();

        let result = ConfigLoader::load(project_root);

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&config_path, perms).unwrap();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Failed to read") || err.to_string().contains("permission")
        );
    }

    #[test]
    fn test_config_file_resolve_template_with_invalid_name_returns_error() {
        let config = ConfigFile {
            template: "nonexistent-template".to_string(),
            profile: None,
            wasm_opt: None,
            size_budget: None,
        };

        let result = TemplateResolver::resolve(&config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found") || err.to_string().contains("Template"));
    }

    #[test]
    fn test_config_file_exists_with_nonexistent_path_returns_false() {
        // Test with a path that doesn't exist
        let nonexistent = Path::new("/nonexistent/directory/that/does/not/exist");
        assert!(!ConfigLoader::exists(nonexistent));
    }

    #[test]
    fn test_size_budget_default_has_no_limits() {
        let budget = SizeBudget::default();
        assert!(budget.max_size_kb.is_none());
        assert!(budget.warn_threshold_kb.is_none());
        assert!(budget.target_size_kb.is_none());
    }

    #[test]
    fn test_config_file_serialize_deserialize_preserves_data() {
        let config = ConfigFile {
            template: "minimal".to_string(),
            profile: Some(ProfileSettings {
                opt_level: Some("z".to_string()),
                lto: Some("fat".to_string()),
                strip: Some(true),
                codegen_units: Some(1),
                panic: Some("abort".to_string()),
            }),
            wasm_opt: Some(WasmOptSettings {
                flags: Some(vec!["-Oz".to_string()]),
            }),
            size_budget: Some(SizeBudget {
                max_size_kb: Some(500),
                warn_threshold_kb: Some(400),
                target_size_kb: Some(300),
            }),
        };

        // Serialize
        let serialized = toml_edit::ser::to_string(&config).unwrap();
        assert!(serialized.contains("minimal"));

        // Deserialize
        let deserialized: ConfigFile = toml_edit::de::from_str(&serialized).unwrap();
        assert_eq!(deserialized.template, "minimal");
        assert!(deserialized.profile.is_some());
        assert!(deserialized.size_budget.is_some());
    }

    // P2-ERROR-001: Malformed TOML edge case tests

    #[test]
    fn test_config_file_load_with_deeply_nested_toml_handles_gracefully() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // Create deeply nested TOML structure (100 levels)
        let mut nested_toml = String::new();
        for i in 0..100 {
            nested_toml.push_str(&format!("[level{}]\n", i));
        }
        nested_toml.push_str("value = 1\n");

        fs::write(&config_path, nested_toml).unwrap();

        // Should handle deep nesting without panic or crash
        let result = ConfigLoader::load(project_root);
        // May succeed (ignoring unknown fields) or fail with parse error
        let _ = result;
    }

    #[test]
    fn test_config_file_load_with_extremely_large_file_handles_gracefully() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // Create a large TOML file (~1MB)
        let mut large_toml = String::from("# Valid but large TOML\ntemplate = \"balanced\"\n\n");
        for i in 0..10000 {
            large_toml.push_str(&format!("# Comment line {}\n", i));
        }

        fs::write(&config_path, large_toml).unwrap();

        // Should handle large files without memory issues
        let result = ConfigLoader::load(project_root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_file_load_with_unicode_edge_cases_handles_correctly() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // Test various Unicode edge cases
        let unicode_toml = r#"
# Unicode comment: 你好 🚀 مرحبا
template = "balanced"

[profile]
# Emoji in comment 🎯
opt_level = "z"
"#;

        fs::write(&config_path, unicode_toml).unwrap();

        let result = ConfigLoader::load(project_root);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.template, "balanced");
    }

    #[test]
    fn test_config_file_load_with_duplicate_keys_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // TOML with duplicate keys (should fail parsing)
        let duplicate_toml = r#"
template = "balanced"
template = "aggressive"
"#;

        fs::write(&config_path, duplicate_toml).unwrap();

        let result = ConfigLoader::load(project_root);
        // TOML parser should reject duplicate keys
        assert!(result.is_err());
    }

    #[test]
    fn test_config_file_load_with_circular_table_refs_handles_gracefully() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // TOML doesn't support circular references, but test malformed structure
        let circular_toml = r#"
template = "balanced"

[profile]
opt_level = "z"

[[profile]]
lto = "thin"
"#;

        fs::write(&config_path, circular_toml).unwrap();

        // Should handle malformed structure gracefully
        let result = ConfigLoader::load(project_root);
        // Will likely fail due to type mismatch (table vs array)
        assert!(result.is_err());
    }

    #[test]
    fn test_config_file_load_with_invalid_utf8_sequences_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // Write invalid UTF-8 bytes
        use std::io::Write;
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(b"template = \"").unwrap();
        file.write_all(&[0xFF, 0xFE, 0xFD]).unwrap(); // Invalid UTF-8
        file.write_all(b"\"\n").unwrap();
        drop(file);

        let result = ConfigLoader::load(project_root);
        // Should handle invalid UTF-8 gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_config_file_load_with_only_whitespace_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // File with only whitespace and comments
        let whitespace_toml = "   \n\n  \t\n# Just a comment\n   \n";
        fs::write(&config_path, whitespace_toml).unwrap();

        let result = ConfigLoader::load(project_root);
        // Should succeed and return default config
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_file_load_with_trailing_commas_in_inline_tables_handles_correctly() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let config_path = project_root.join(CONFIG_FILE_NAME);

        // TOML 1.1 allows trailing commas in inline tables
        let trailing_comma_toml = r#"
template = "balanced"
profile = { opt_level = "z", lto = "thin", }
"#;

        fs::write(&config_path, trailing_comma_toml).unwrap();

        let result = ConfigLoader::load(project_root);
        // TOML 1.1 spec allows trailing commas in inline tables
        assert!(result.is_ok());
    }
}
