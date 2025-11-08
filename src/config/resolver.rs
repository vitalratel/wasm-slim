//! Template resolution logic

use super::file::ConfigFile;
use super::template::Template;
use anyhow::Result;

/// Handles template resolution and merging
///
/// Merges base template settings with user overrides from `.wasm-slim.toml`.
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::config::{ConfigFile, Template, TemplateResolver};
///
/// // Resolve config with overrides
/// let config = ConfigFile::default();
/// let template = TemplateResolver::resolve(&config)?;
///
/// // Create config from template
/// let balanced = Template::get("balanced").unwrap();
/// let config = TemplateResolver::from_template(&balanced);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct TemplateResolver;

impl TemplateResolver {
    /// Merge template settings with config overrides
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::config::{ConfigFile, TemplateResolver};
    ///
    /// let config = ConfigFile::default();
    /// let template = TemplateResolver::resolve(&config)?;
    /// println!("Resolved template: {}", template.name);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn resolve(config: &ConfigFile) -> Result<Template> {
        // Get base template
        let mut template = super::template::Template::get(&config.template)
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", config.template))?;

        // Apply profile overrides
        if let Some(ref profile) = config.profile {
            if let Some(ref opt_level) = profile.opt_level {
                template.profile.opt_level = opt_level.clone();
            }
            if let Some(ref lto) = profile.lto {
                template.profile.lto = lto.clone();
            }
            if let Some(strip) = profile.strip {
                template.profile.strip = strip;
            }
            if let Some(codegen_units) = profile.codegen_units {
                template.profile.codegen_units = codegen_units;
            }
            if let Some(ref panic) = profile.panic {
                template.profile.panic = panic.clone();
            }
        }

        // Apply wasm-opt overrides
        if let Some(ref wasm_opt) = config.wasm_opt {
            if let Some(ref flags) = wasm_opt.flags {
                template.wasm_opt.flags = flags.clone();
            }
        }

        Ok(template)
    }

    /// Create a config from a template
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::config::{Template, TemplateResolver};
    ///
    /// let template = Template::get("minimal").expect("minimal template exists");
    /// let config = TemplateResolver::from_template(&template);
    ///
    /// assert_eq!(config.template, "minimal");
    /// assert!(config.profile.is_some());
    /// ```
    pub fn from_template(template: &Template) -> ConfigFile {
        use super::file::{ProfileSettings, WasmOptSettings};

        ConfigFile {
            template: template.name.clone(),
            profile: Some(ProfileSettings {
                opt_level: Some(template.profile.opt_level.clone()),
                lto: Some(template.profile.lto.clone()),
                strip: Some(template.profile.strip),
                codegen_units: Some(template.profile.codegen_units),
                panic: Some(template.profile.panic.clone()),
            }),
            wasm_opt: Some(WasmOptSettings {
                flags: Some(template.wasm_opt.flags.clone()),
            }),
            size_budget: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::file::ProfileSettings;
    use super::super::template::Template;
    use super::*;

    #[test]
    fn test_resolver_resolves_template_by_name() {
        let config = ConfigFile {
            template: "minimal".to_string(),
            ..Default::default()
        };

        let result = TemplateResolver::resolve(&config);
        assert!(result.is_ok());
        let template = result.unwrap();
        assert_eq!(template.name, "minimal");
    }

    #[test]
    fn test_resolver_with_invalid_template_name_returns_error() {
        let config = ConfigFile {
            template: "nonexistent_template".to_string(),
            ..Default::default()
        };

        let result = TemplateResolver::resolve(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_resolver_merges_opt_level_override() {
        let mut config = ConfigFile {
            template: "minimal".to_string(),
            ..Default::default()
        };
        config.profile = Some(ProfileSettings {
            opt_level: Some("3".to_string()),
            lto: None,
            strip: None,
            codegen_units: None,
            panic: None,
        });

        let template = TemplateResolver::resolve(&config).unwrap();
        assert_eq!(template.profile.opt_level, "3");
    }

    #[test]
    fn test_resolver_merges_lto_override() {
        let mut config = ConfigFile {
            template: "minimal".to_string(),
            ..Default::default()
        };
        config.profile = Some(ProfileSettings {
            opt_level: None,
            lto: Some("fat".to_string()),
            strip: None,
            codegen_units: None,
            panic: None,
        });

        let template = TemplateResolver::resolve(&config).unwrap();
        assert_eq!(template.profile.lto, "fat");
    }

    #[test]
    fn test_resolver_merges_strip_override() {
        let mut config = ConfigFile {
            template: "minimal".to_string(),
            ..Default::default()
        };
        config.profile = Some(ProfileSettings {
            opt_level: None,
            lto: None,
            strip: Some(false),
            codegen_units: None,
            panic: None,
        });

        let template = TemplateResolver::resolve(&config).unwrap();
        assert!(!template.profile.strip);
    }

    #[test]
    fn test_resolver_merges_codegen_units_override() {
        let mut config = ConfigFile {
            template: "minimal".to_string(),
            ..Default::default()
        };
        config.profile = Some(ProfileSettings {
            opt_level: None,
            lto: None,
            strip: None,
            codegen_units: Some(4),
            panic: None,
        });

        let template = TemplateResolver::resolve(&config).unwrap();
        assert_eq!(template.profile.codegen_units, 4);
    }

    #[test]
    fn test_resolver_merges_panic_override() {
        let mut config = ConfigFile {
            template: "minimal".to_string(),
            ..Default::default()
        };
        config.profile = Some(ProfileSettings {
            opt_level: None,
            lto: None,
            strip: None,
            codegen_units: None,
            panic: Some("unwind".to_string()),
        });

        let template = TemplateResolver::resolve(&config).unwrap();
        assert_eq!(template.profile.panic, "unwind");
    }

    #[test]
    fn test_resolver_merges_wasm_opt_flags() {
        use super::super::file::WasmOptSettings;

        let mut config = ConfigFile {
            template: "minimal".to_string(),
            ..Default::default()
        };
        config.wasm_opt = Some(WasmOptSettings {
            flags: Some(vec!["--enable-simd".to_string()]),
        });

        let template = TemplateResolver::resolve(&config).unwrap();
        assert_eq!(template.wasm_opt.flags, vec!["--enable-simd"]);
    }

    #[test]
    fn test_resolver_preserves_template_values_without_overrides() {
        let config = ConfigFile {
            template: "balanced".to_string(),
            profile: None,
            wasm_opt: None,
            ..Default::default()
        };

        let template = TemplateResolver::resolve(&config).unwrap();
        let base_template = Template::get("balanced").unwrap();

        assert_eq!(template.profile.opt_level, base_template.profile.opt_level);
        assert_eq!(template.profile.lto, base_template.profile.lto);
        assert_eq!(template.profile.strip, base_template.profile.strip);
    }

    #[test]
    fn test_from_template_creates_complete_config() {
        let template = Template::get("minimal").unwrap();
        let config = TemplateResolver::from_template(&template);

        assert_eq!(config.template, "minimal");
        assert!(config.profile.is_some());
        assert!(config.wasm_opt.is_some());
    }

    #[test]
    fn test_from_template_preserves_all_profile_settings() {
        let template = Template::get("aggressive").unwrap();
        let config = TemplateResolver::from_template(&template);

        let profile = config.profile.unwrap();
        assert_eq!(profile.opt_level, Some(template.profile.opt_level.clone()));
        assert_eq!(profile.lto, Some(template.profile.lto.clone()));
        assert_eq!(profile.strip, Some(template.profile.strip));
        assert_eq!(profile.codegen_units, Some(template.profile.codegen_units));
        assert_eq!(profile.panic, Some(template.profile.panic.clone()));
    }

    #[test]
    fn test_from_template_preserves_wasm_opt_flags() {
        let template = Template::get("minimal").unwrap();
        let config = TemplateResolver::from_template(&template);

        let wasm_opt = config.wasm_opt.unwrap();
        assert_eq!(wasm_opt.flags, Some(template.wasm_opt.flags.clone()));
    }

    #[test]
    fn test_resolver_with_multiple_overrides() {
        use super::super::file::WasmOptSettings;

        let mut config = ConfigFile {
            template: "balanced".to_string(),
            ..Default::default()
        };
        config.profile = Some(ProfileSettings {
            opt_level: Some("z".to_string()),
            lto: Some("thin".to_string()),
            strip: Some(true),
            codegen_units: Some(1),
            panic: Some("abort".to_string()),
        });
        config.wasm_opt = Some(WasmOptSettings {
            flags: Some(vec!["--custom-flag".to_string()]),
        });

        let template = TemplateResolver::resolve(&config).unwrap();

        assert_eq!(template.profile.opt_level, "z");
        assert_eq!(template.profile.lto, "thin");
        assert!(template.profile.strip);
        assert_eq!(template.profile.codegen_units, 1);
        assert_eq!(template.profile.panic, "abort");
        assert_eq!(template.wasm_opt.flags, vec!["--custom-flag"]);
    }
}
