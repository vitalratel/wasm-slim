//! Template system for wasm-slim optimization presets
//!
//! Provides production-tested optimization templates for different use cases:
//! - `minimal`: Maximum size reduction, may affect performance
//! - `balanced`: Warp-style settings (opt-level="s", lto=true) - recommended
//! - `aggressive`: All optimizations enabled, maximum reduction
//! - Framework-specific: Yew, Leptos, Dioxus presets
//!
//! # Examples
//!
//! ```no_run
//! use wasm_slim::config::{Template, TemplateType};
//!
//! // Use a preset
//! let template = Template::new(TemplateType::Balanced);
//! println!("Using: {}", template.name);
//!
//! // Customize with builder
//! use wasm_slim::config::TemplateBuilder;
//! let custom = TemplateBuilder::from_template(&template)
//!     .with_opt_level("3")
//!     .build();
//! ```

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::profile_config::ProfileConfig;
use super::wasm_config::{WasmBindgenConfig, WasmOptConfig};

/// Template type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateType {
    /// Minimal size, may impact performance
    Minimal,
    /// Balanced size/performance (Warp-validated, recommended)
    Balanced,
    /// Maximum size reduction with all optimizations
    Aggressive,
    /// Yew framework optimizations
    Yew,
    /// Leptos framework optimizations
    Leptos,
    /// Dioxus framework optimizations
    Dioxus,
    /// User-defined custom template
    Custom,
}

impl FromStr for TemplateType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(Self::Minimal),
            "balanced" => Ok(Self::Balanced),
            "aggressive" => Ok(Self::Aggressive),
            "yew" => Ok(Self::Yew),
            "leptos" => Ok(Self::Leptos),
            "dioxus" => Ok(Self::Dioxus),
            "custom" => Ok(Self::Custom),
            _ => Err(format!("Unknown template type: {}", s)),
        }
    }
}

impl TemplateType {
    /// Get template name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
            Self::Yew => "yew",
            Self::Leptos => "leptos",
            Self::Dioxus => "dioxus",
            Self::Custom => "custom",
        }
    }

    /// Get template description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Minimal => "Maximum size reduction, may affect performance",
            Self::Balanced => "Balanced size/performance (Warp-validated, recommended)",
            Self::Aggressive => "All optimizations enabled, maximum reduction",
            Self::Yew => "Optimized for Yew framework projects",
            Self::Leptos => "Optimized for Leptos framework projects",
            Self::Dioxus => "Optimized for Dioxus framework projects",
            Self::Custom => "User-defined custom configuration",
        }
    }
}

/// Complete optimization template
///
/// Provides pre-configured optimization settings for different use cases:
/// - **minimal**: Maximum size reduction (may affect performance)
/// - **balanced**: Size/performance balance (recommended, Warp-validated)
/// - **aggressive**: Aggressive optimizations
/// - Framework-specific: **yew**, **leptos**, **dioxus**
///
/// # Examples
///
/// ```
/// use wasm_slim::config::{Template, TemplateType};
///
/// // Get the balanced template (recommended)
/// let template = Template::new(TemplateType::Balanced);
/// println!("Using template: {}", template.name);
/// println!("Description: {}", template.description);
///
/// // Get a template by name
/// if let Some(template) = Template::get("minimal") {
///     println!("Found template: {}", template.name);
/// }
///
/// // List all available templates
/// for name in Template::names() {
///     println!("Available: {}", name);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Template type
    pub template_type: TemplateType,
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Cargo profile settings
    pub profile: ProfileConfig,
    /// wasm-opt settings
    pub wasm_opt: WasmOptConfig,
    /// wasm-bindgen settings
    pub wasm_bindgen: WasmBindgenConfig,
    /// Recommended dependencies to optimize
    pub dependency_hints: Vec<String>,
    /// Additional notes
    pub notes: Vec<String>,
}

impl Template {
    /// Create a new template with defaults
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::config::{Template, TemplateType};
    ///
    /// let minimal = Template::new(TemplateType::Minimal);
    /// assert_eq!(minimal.name, "minimal");
    ///
    /// let balanced = Template::new(TemplateType::Balanced);
    /// assert_eq!(balanced.name, "balanced");
    /// ```
    pub fn new(template_type: TemplateType) -> Self {
        match template_type {
            TemplateType::Minimal => Self::minimal(),
            TemplateType::Balanced => Self::balanced(),
            TemplateType::Aggressive => Self::aggressive(),
            TemplateType::Yew => Self::yew(),
            TemplateType::Leptos => Self::leptos(),
            TemplateType::Dioxus => Self::dioxus(),
            TemplateType::Custom => Self::custom(),
        }
    }

    /// Minimal template - maximum size reduction
    fn minimal() -> Self {
        Self {
            template_type: TemplateType::Minimal,
            name: "minimal".to_string(),
            description: "Maximum size reduction, may affect performance".to_string(),
            profile: ProfileConfig {
                opt_level: "z".to_string(),
                lto: "fat".to_string(),
                strip: true,
                codegen_units: 1,
                panic: "abort".to_string(),
            },
            wasm_opt: WasmOptConfig {
                flags: vec![
                    "-Oz".to_string(),
                    "--enable-mutable-globals".to_string(),
                    "--enable-bulk-memory".to_string(),
                    "--enable-sign-ext".to_string(),
                    "--enable-nontrapping-float-to-int".to_string(),
                    "--strip-debug".to_string(),
                    "--strip-dwarf".to_string(),
                    "--strip-producers".to_string(),
                ],
            },
            wasm_bindgen: WasmBindgenConfig {
                debug: false,
                remove_producers_section: true,
                flags: vec![],
            },
            dependency_hints: vec![
                "Set default-features = false on all dependencies".to_string(),
                "Use getrandom with wasm_js feature".to_string(),
            ],
            notes: vec![
                "Prioritizes size over performance".to_string(),
                "May increase compile time significantly".to_string(),
            ],
        }
    }

    /// Balanced template - Warp-validated settings (RECOMMENDED)
    fn balanced() -> Self {
        Self {
            template_type: TemplateType::Balanced,
            name: "balanced".to_string(),
            description: "Balanced size/performance (Warp-validated, recommended)".to_string(),
            profile: ProfileConfig {
                opt_level: "s".to_string(),
                lto: "fat".to_string(),
                strip: true,
                codegen_units: 1,
                panic: "abort".to_string(),
            },
            wasm_opt: WasmOptConfig {
                flags: vec![
                    "-Oz".to_string(),
                    "--enable-mutable-globals".to_string(),
                    "--enable-bulk-memory".to_string(),
                    "--enable-sign-ext".to_string(),
                    "--enable-nontrapping-float-to-int".to_string(),
                    "--strip-debug".to_string(),
                    "--strip-dwarf".to_string(),
                    "--strip-producers".to_string(),
                ],
            },
            wasm_bindgen: WasmBindgenConfig {
                debug: false,
                remove_producers_section: true,
                flags: vec![],
            },
            dependency_hints: vec![
                "Minimize feature flags where possible".to_string(),
                "Use getrandom with wasm_js feature".to_string(),
            ],
            notes: vec![
                "Production-tested by Warp.dev (62% size reduction)".to_string(),
                "Good balance between size and performance".to_string(),
                "Recommended for most projects".to_string(),
            ],
        }
    }

    /// Aggressive template - all optimizations enabled
    fn aggressive() -> Self {
        Self {
            template_type: TemplateType::Aggressive,
            name: "aggressive".to_string(),
            description: "All optimizations enabled, maximum reduction".to_string(),
            profile: ProfileConfig {
                opt_level: "z".to_string(),
                lto: "fat".to_string(),
                strip: true,
                codegen_units: 1,
                panic: "abort".to_string(),
            },
            wasm_opt: WasmOptConfig {
                flags: vec![
                    "-Oz".to_string(),
                    "--enable-mutable-globals".to_string(),
                    "--enable-bulk-memory".to_string(),
                    "--enable-sign-ext".to_string(),
                    "--enable-nontrapping-float-to-int".to_string(),
                    "--strip-debug".to_string(),
                    "--strip-dwarf".to_string(),
                    "--strip-producers".to_string(),
                    "--vacuum".to_string(),
                    "--closed-world".to_string(),
                    "--gufa-optimizing".to_string(),
                ],
            },
            wasm_bindgen: WasmBindgenConfig {
                debug: false,
                remove_producers_section: true,
                flags: vec!["--omit-default-module-path".to_string()],
            },
            dependency_hints: vec![
                "Set default-features = false on ALL dependencies".to_string(),
                "Use getrandom with wasm_js feature".to_string(),
                "Consider lighter alternatives for heavy deps".to_string(),
                "Externalize assets (fonts, images)".to_string(),
            ],
            notes: vec![
                "Maximum size reduction at all costs".to_string(),
                "Significantly longer compile times".to_string(),
                "May affect runtime performance".to_string(),
                "Use for production builds with strict size budgets".to_string(),
                "Use nightly Rust for build-std (additional 10-20% reduction)".to_string(),
            ],
        }
    }

    /// Yew framework template
    fn yew() -> Self {
        let mut template = Self::balanced();
        template.template_type = TemplateType::Yew;
        template.name = "yew".to_string();
        template.description = "Optimized for Yew framework projects".to_string();
        template.dependency_hints = vec![
            "yew = { version = \"*\", default-features = false }".to_string(),
            "Use yew-router with minimal features".to_string(),
            "Avoid heavy dependencies in components".to_string(),
            "Consider code-splitting for large apps".to_string(),
        ];
        template.notes = vec![
            "Based on balanced template".to_string(),
            "Optimized for Yew's component model".to_string(),
            "Minimizes framework overhead".to_string(),
        ];
        template
    }

    /// Leptos framework template
    fn leptos() -> Self {
        let mut template = Self::balanced();
        template.template_type = TemplateType::Leptos;
        template.name = "leptos".to_string();
        template.description = "Optimized for Leptos framework projects".to_string();
        template.dependency_hints = vec![
            "leptos = { version = \"*\", default-features = false }".to_string(),
            "Enable only needed features (csr, hydrate, ssr)".to_string(),
            "Use leptos_router with minimal features".to_string(),
            "Leverage Leptos's fine-grained reactivity".to_string(),
        ];
        template.notes = vec![
            "Based on balanced template".to_string(),
            "Optimized for Leptos's fine-grained reactivity".to_string(),
            "Supports both CSR and SSR modes".to_string(),
            "Use nightly + build-std for 10-20% additional reduction".to_string(),
            "Consider lightweight serialization (miniserde, serde-lite)".to_string(),
        ];
        template
    }

    /// Dioxus framework template
    fn dioxus() -> Self {
        let mut template = Self::balanced();
        template.template_type = TemplateType::Dioxus;
        template.name = "dioxus".to_string();
        template.description = "Optimized for Dioxus framework projects".to_string();
        template.dependency_hints = vec![
            "dioxus = { version = \"*\", default-features = false }".to_string(),
            "Enable only target features (web, desktop, mobile)".to_string(),
            "Use dioxus-router with minimal features".to_string(),
            "Leverage Dioxus's virtual DOM efficiently".to_string(),
        ];
        template.notes = vec![
            "Based on balanced template".to_string(),
            "Optimized for Dioxus's component model".to_string(),
            "Supports multiple platforms".to_string(),
        ];
        template
    }

    /// Custom template - user-defined
    fn custom() -> Self {
        Self {
            template_type: TemplateType::Custom,
            name: "custom".to_string(),
            description: "User-defined custom configuration".to_string(),
            profile: ProfileConfig {
                opt_level: "s".to_string(),
                lto: "fat".to_string(),
                strip: true,
                codegen_units: 1,
                panic: "abort".to_string(),
            },
            wasm_opt: WasmOptConfig {
                flags: vec!["-Oz".to_string()],
            },
            wasm_bindgen: WasmBindgenConfig {
                debug: false,
                remove_producers_section: false,
                flags: vec![],
            },
            dependency_hints: vec![],
            notes: vec!["Customize this template in .wasm-slim.toml".to_string()],
        }
    }

    /// Get all available template types
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::config::Template;
    ///
    /// let templates = Template::available_templates();
    /// assert!(!templates.is_empty());
    /// assert!(templates.len() >= 6); // minimal, balanced, aggressive, yew, leptos, dioxus
    /// ```
    pub fn available_templates() -> Vec<TemplateType> {
        vec![
            TemplateType::Minimal,
            TemplateType::Balanced,
            TemplateType::Aggressive,
            TemplateType::Yew,
            TemplateType::Leptos,
            TemplateType::Dioxus,
        ]
    }

    /// Get a template by name (data-driven lookup)
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::config::Template;
    ///
    /// // Get a template by name
    /// let template = Template::get("balanced").expect("balanced template exists");
    /// assert_eq!(template.name, "balanced");
    ///
    /// // Invalid name returns None
    /// assert!(Template::get("nonexistent").is_none());
    /// ```
    pub fn get(name: &str) -> Option<Self> {
        TemplateType::from_str(name).ok().map(Self::new)
    }

    /// Get all template names (sorted alphabetically)
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::config::Template;
    ///
    /// let names = Template::names();
    /// assert!(names.contains(&"minimal".to_string()));
    /// assert!(names.contains(&"balanced".to_string()));
    /// assert!(names.contains(&"aggressive".to_string()));
    ///
    /// // Names are sorted alphabetically
    /// let mut sorted_names = names.clone();
    /// sorted_names.sort();
    /// assert_eq!(names, sorted_names);
    /// ```
    pub fn names() -> Vec<String> {
        let mut names: Vec<String> = Self::available_templates()
            .iter()
            .map(|t| t.name().to_string())
            .collect();
        names.sort();
        names
    }

    /// Get all templates (sorted by name)
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::config::Template;
    ///
    /// let templates = Template::all();
    /// assert!(!templates.is_empty());
    ///
    /// // Verify templates are sorted by name
    /// for i in 1..templates.len() {
    ///     assert!(templates[i-1].name <= templates[i].name);
    /// }
    /// ```
    pub fn all() -> Vec<Self> {
        let mut templates: Vec<Self> = Self::available_templates()
            .into_iter()
            .map(Self::new)
            .collect();
        templates.sort_by(|a, b| a.name.cmp(&b.name));
        templates
    }
}

/// Builder for customizing templates
pub struct TemplateBuilder {
    template: Template,
}

impl TemplateBuilder {
    /// Create a builder from a template
    pub fn from_template(template: &Template) -> Self {
        Self {
            template: template.clone(),
        }
    }

    /// Set optimization level
    pub fn with_opt_level(mut self, opt_level: &str) -> Self {
        self.template.profile.opt_level = opt_level.to_string();
        self
    }

    /// Set LTO mode
    pub fn with_lto(mut self, lto: &str) -> Self {
        self.template.profile.lto = lto.to_string();
        self
    }

    /// Set strip option
    pub fn with_strip(mut self, strip: bool) -> Self {
        self.template.profile.strip = strip;
        self
    }

    /// Set codegen units
    pub fn with_codegen_units(mut self, units: u32) -> Self {
        self.template.profile.codegen_units = units;
        self
    }

    /// Set panic mode
    pub fn with_panic(mut self, panic: &str) -> Self {
        self.template.profile.panic = panic.to_string();
        self
    }

    /// Set wasm-opt flags
    pub fn with_wasm_opt_flags(mut self, flags: Vec<String>) -> Self {
        self.template.wasm_opt.flags = flags;
        self
    }

    /// Build the final template
    pub fn build(self) -> Template {
        self.template
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_methods() {
        let minimal = Template::new(TemplateType::Minimal);
        assert_eq!(minimal.name, "minimal");
        assert_eq!(minimal.profile.opt_level, "z");

        let balanced = Template::new(TemplateType::Balanced);
        assert_eq!(balanced.name, "balanced");
        assert_eq!(balanced.profile.opt_level, "s");
    }

    #[test]
    fn test_template_get_by_name() {
        assert!(Template::get("minimal").is_some());
        assert!(Template::get("balanced").is_some());
        assert!(Template::get("nonexistent").is_none());
    }

    #[test]
    fn test_template_builder() {
        let custom = TemplateBuilder::from_template(&Template::new(TemplateType::Balanced))
            .with_opt_level("3")
            .with_codegen_units(8)
            .build();

        assert_eq!(custom.profile.opt_level, "3");
        assert_eq!(custom.profile.codegen_units, 8);
    }

    #[test]
    fn test_template_type_from_str_parses_case_insensitively() {
        assert_eq!(TemplateType::from_str("minimal"), Ok(TemplateType::Minimal));
        assert_eq!(
            TemplateType::from_str("Balanced"),
            Ok(TemplateType::Balanced)
        );
        assert_eq!(
            TemplateType::from_str("AGGRESSIVE"),
            Ok(TemplateType::Aggressive)
        );
        assert_eq!(TemplateType::from_str("yew"), Ok(TemplateType::Yew));
        assert!(TemplateType::from_str("invalid").is_err());
    }

    #[test]
    fn test_template_new_creates_with_correct_defaults() {
        let minimal = Template::new(TemplateType::Minimal);
        assert_eq!(minimal.name, "minimal");
        assert_eq!(minimal.profile.opt_level, "z");

        let balanced = Template::new(TemplateType::Balanced);
        assert_eq!(balanced.name, "balanced");
        assert_eq!(balanced.profile.opt_level, "s");

        let aggressive = Template::new(TemplateType::Aggressive);
        assert_eq!(aggressive.name, "aggressive");
        assert!(aggressive.wasm_opt.flags.len() > 8);
    }

    #[test]
    fn test_template_new_for_frameworks_includes_dependency_hints() {
        let yew = Template::new(TemplateType::Yew);
        assert_eq!(yew.name, "yew");
        assert!(!yew.dependency_hints.is_empty());

        let leptos = Template::new(TemplateType::Leptos);
        assert_eq!(leptos.name, "leptos");

        let dioxus = Template::new(TemplateType::Dioxus);
        assert_eq!(dioxus.name, "dioxus");
    }

    #[test]
    fn test_template_get_and_all_work_correctly() {
        assert!(Template::get("balanced").is_some());
        assert!(Template::get("minimal").is_some());
        assert!(Template::get("yew").is_some());
        assert!(Template::get("invalid").is_none());

        let templates = Template::all();
        assert_eq!(templates.len(), 6);

        let names = Template::names();
        assert!(names.contains(&"balanced".to_string()));
    }

    #[test]
    fn test_template_balanced_matches_warp_configuration() {
        let balanced = Template::new(TemplateType::Balanced);
        assert_eq!(balanced.profile.opt_level, "s");
        assert_eq!(balanced.profile.lto, "fat");
        assert!(balanced.profile.strip);
        assert!(balanced.notes.iter().any(|n| n.contains("Warp")));
    }

    #[test]
    fn test_template_type_from_str_handles_any_case() {
        assert_eq!(TemplateType::from_str("minimal"), Ok(TemplateType::Minimal));
        assert_eq!(TemplateType::from_str("MINIMAL"), Ok(TemplateType::Minimal));
        assert_eq!(TemplateType::from_str("MiNiMaL"), Ok(TemplateType::Minimal));
        assert_eq!(
            TemplateType::from_str("Balanced"),
            Ok(TemplateType::Balanced)
        );
        assert_eq!(
            TemplateType::from_str("BALANCED"),
            Ok(TemplateType::Balanced)
        );
    }

    #[test]
    fn test_template_type_from_str_with_invalid_names_returns_error() {
        assert!(TemplateType::from_str("invalid").is_err());
        assert!(TemplateType::from_str("").is_err());
        assert!(TemplateType::from_str("min").is_err());
        assert!(TemplateType::from_str("minimall").is_err());
        assert!(TemplateType::from_str("balance").is_err());
        assert!(TemplateType::from_str("react").is_err());
    }

    #[test]
    fn test_template_type_from_str_error_messages_are_descriptive() {
        let err = TemplateType::from_str("invalid").unwrap_err();
        assert!(err.contains("Unknown template type"));
        assert!(err.contains("invalid"));
    }

    #[test]
    fn test_template_get_with_nonexistent_name_returns_none() {
        assert!(Template::get("nonexistent").is_none());
        assert!(Template::get("").is_none());
        assert!(Template::get("react").is_none());
        assert!(Template::get("vue").is_none());
    }

    #[test]
    fn test_template_contains_all_expected_templates() {
        assert!(Template::get("minimal").is_some());
        assert!(Template::get("balanced").is_some());
        assert!(Template::get("aggressive").is_some());
        assert!(Template::get("yew").is_some());
        assert!(Template::get("leptos").is_some());
        assert!(Template::get("dioxus").is_some());

        let templates = Template::all();
        assert_eq!(templates.len(), 6);
    }

    #[test]
    fn test_template_names_returns_sorted_list() {
        let names = Template::names();

        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);
    }

    #[test]
    fn test_template_all_returns_sorted_by_name() {
        let templates = Template::all();

        for i in 1..templates.len() {
            assert!(templates[i - 1].name <= templates[i].name);
        }
    }

    #[test]
    fn test_template_type_all_variants_have_valid_names() {
        let types = vec![
            TemplateType::Minimal,
            TemplateType::Balanced,
            TemplateType::Aggressive,
            TemplateType::Yew,
            TemplateType::Leptos,
            TemplateType::Dioxus,
            TemplateType::Custom,
        ];

        for template_type in types {
            let name = template_type.name();
            assert!(!name.is_empty());
            assert!(name.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }

    #[test]
    fn test_template_type_all_variants_have_descriptions() {
        let types = vec![
            TemplateType::Minimal,
            TemplateType::Balanced,
            TemplateType::Aggressive,
            TemplateType::Yew,
            TemplateType::Leptos,
            TemplateType::Dioxus,
            TemplateType::Custom,
        ];

        for template_type in types {
            let desc = template_type.description();
            assert!(!desc.is_empty());
            assert!(desc.chars().next().unwrap().is_uppercase());
        }
    }

    #[test]
    fn test_template_profile_settings_are_valid() {
        let templates = vec![
            Template::new(TemplateType::Minimal),
            Template::new(TemplateType::Balanced),
            Template::new(TemplateType::Aggressive),
        ];

        for template in templates {
            assert!(["s", "z", "3"].contains(&template.profile.opt_level.as_str()));
            assert!(["fat", "thin", "true", "false"].contains(&template.profile.lto.as_str()));
            assert!(template.profile.codegen_units >= 1 && template.profile.codegen_units <= 256);
            assert!(["abort", "unwind"].contains(&template.profile.panic.as_str()));
        }
    }

    #[test]
    fn test_template_wasm_opt_flags_are_valid() {
        let templates = vec![
            Template::new(TemplateType::Minimal),
            Template::new(TemplateType::Balanced),
            Template::new(TemplateType::Aggressive),
        ];

        for template in templates {
            assert!(!template.wasm_opt.flags.is_empty());
            for flag in &template.wasm_opt.flags {
                assert!(flag.starts_with("-"), "Flag '{}' should start with -", flag);
            }
        }
    }

    #[test]
    fn test_template_framework_variants_have_dependency_hints() {
        let yew = Template::new(TemplateType::Yew);
        assert!(!yew.dependency_hints.is_empty());

        let leptos = Template::new(TemplateType::Leptos);
        assert!(!leptos.dependency_hints.is_empty());

        let dioxus = Template::new(TemplateType::Dioxus);
        assert!(!dioxus.dependency_hints.is_empty());
    }

    #[test]
    fn test_template_clone_preserves_all_fields() {
        let original = Template::new(TemplateType::Balanced);
        let cloned = original.clone();

        assert_eq!(original.name, cloned.name);
        assert_eq!(original.profile.opt_level, cloned.profile.opt_level);
        assert_eq!(original.wasm_opt.flags, cloned.wasm_opt.flags);
    }

    #[test]
    fn test_template_type_all_variants_can_be_parsed() {
        let types = vec![
            (TemplateType::Minimal, "minimal"),
            (TemplateType::Balanced, "balanced"),
            (TemplateType::Aggressive, "aggressive"),
            (TemplateType::Yew, "yew"),
            (TemplateType::Leptos, "leptos"),
            (TemplateType::Dioxus, "dioxus"),
            (TemplateType::Custom, "custom"),
        ];

        for (expected_type, name) in types {
            let parsed = TemplateType::from_str(name).unwrap();
            assert_eq!(parsed, expected_type);
            assert_eq!(expected_type.name(), name);
        }
    }
}
