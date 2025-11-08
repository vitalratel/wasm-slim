//! Configuration and template management for wasm-slim
//!
//! This module provides:
//! - Template definitions (minimal, balanced, aggressive, framework-specific)
//! - .wasm-slim.toml config file support
//! - Template application logic

pub mod file;
pub mod loader;
pub mod profile_config;
pub mod resolver;
pub mod template;
pub mod validator;
pub mod wasm_config;

pub use file::{ConfigFile, CONFIG_FILE_NAME};
pub use loader::ConfigLoader;
pub use profile_config::ProfileConfig;
pub use resolver::TemplateResolver;
pub use template::{Template, TemplateBuilder, TemplateType};
pub use validator::{
    ConfigValidator, ValidationIssue, ValidationResult, ValidationSeverity, ValidatorRegistry,
};
pub use wasm_config::{WasmBindgenConfig, WasmOptConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_module_exports_are_accessible() {
        // Ensure all exports compile and are accessible
        let _: Option<Template> = None;
        let _: Option<ConfigFile> = None;
    }

    #[test]
    fn test_template_has_all_standard_templates() {
        assert!(Template::get("minimal").is_some());
        assert!(Template::get("balanced").is_some());
        assert!(Template::get("aggressive").is_some());
    }

    #[test]
    fn test_config_file_name_constant_is_correct() {
        assert_eq!(CONFIG_FILE_NAME, ".wasm-slim.toml");
    }
}
