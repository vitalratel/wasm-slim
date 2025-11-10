//! Init command implementation
//!
//! Handles the `wasm-slim init` command which creates a configuration file
//! from a template (aggressive, balanced, minimal, etc.)

use anyhow::Result;
use console::{style, Emoji};
use std::env;

use crate::config;

static ROCKET: Emoji = Emoji("🚀", ">");
static SPARKLES: Emoji = Emoji("✨", "*");
static CHECKMARK: Emoji = Emoji("✅", "[OK]");
static INFO: Emoji = Emoji("ℹ️", "i");

/// Initialize wasm-slim configuration from a template
///
/// Creates a `.wasm-slim.toml` configuration file using one of the
/// predefined templates (aggressive, balanced, minimal, custom)
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::cmd::init::cmd_init;
///
/// // Initialize with balanced template
/// cmd_init("balanced")?;
///
/// // Initialize with aggressive optimizations
/// cmd_init("aggressive")?;
///
/// // Initialize with minimal changes
/// cmd_init("minimal")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn cmd_init(template: &str) -> Result<()> {
    println!(
        "{} {} Initializing wasm-slim",
        ROCKET,
        style("wasm-slim init").bold()
    );
    println!();

    let project_root = env::current_dir()?;

    // Check if config file already exists
    if config::ConfigLoader::exists(&project_root) {
        println!(
            "{} Config file already exists: {}",
            style("⚠️").yellow(),
            style(config::CONFIG_FILE_NAME).cyan()
        );
        println!("   Delete it first or edit manually to update.");
        return Ok(());
    }

    // Validate template name
    let template_obj = config::Template::get(template)
        .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template))?;

    println!(
        "{} Selected template: {}",
        SPARKLES,
        style(&template_obj.name).bold().cyan()
    );
    println!("   {}", style(&template_obj.description).dim());
    println!();

    // Show template details
    println!("{}  Template Configuration:", INFO);
    println!("   {} Cargo Profile:", style("•").dim());
    println!(
        "      opt-level = {}",
        style(&template_obj.profile.opt_level).green()
    );
    println!("      lto = {}", style(&template_obj.profile.lto).green());
    println!(
        "      strip = {}",
        style(template_obj.profile.strip).green()
    );
    println!(
        "      codegen-units = {}",
        style(template_obj.profile.codegen_units).green()
    );
    println!(
        "      panic = {}",
        style(&template_obj.profile.panic).green()
    );
    println!();
    println!(
        "   {} wasm-opt flags: {} flags",
        style("•").dim(),
        style(template_obj.wasm_opt.flags.len()).green()
    );
    println!();

    // Show dependency hints if any
    if !template_obj.dependency_hints.is_empty() {
        println!("{}  Dependency Recommendations:", INFO);
        for hint in &template_obj.dependency_hints {
            println!("   {} {}", style("•").dim(), hint);
        }
        println!();
    }

    // Show notes
    if !template_obj.notes.is_empty() {
        println!("{}  Notes:", INFO);
        for note in &template_obj.notes {
            println!("   {} {}", style("•").dim(), note);
        }
        println!();
    }

    // Create config file
    let config = config::TemplateResolver::from_template(&template_obj);
    config::ConfigLoader::save(&config, &project_root)?;

    println!(
        "{} Created {}",
        CHECKMARK,
        style(config::CONFIG_FILE_NAME).cyan().bold()
    );
    println!();
    println!("{}  Next Steps:", style("💡").bold());
    println!(
        "   1. Review and customize {} if needed",
        config::CONFIG_FILE_NAME
    );
    println!(
        "   2. Run {} to build with optimizations",
        style("wasm-slim build").cyan()
    );
    println!(
        "   3. Run {} to analyze dependencies",
        style("wasm-slim analyze").cyan()
    );
    println!();

    // Show all available templates
    println!("{}  Available Templates:", INFO);
    for tmpl in config::Template::all() {
        let indicator = if tmpl.name == template { "→" } else { " " };
        println!(
            "   {} {} - {}",
            style(indicator).cyan().bold(),
            style(&tmpl.name).bold(),
            style(&tmpl.description).dim()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Guard that restores the working directory on drop
    struct WorkingDirGuard {
        old_dir: PathBuf,
    }

    impl WorkingDirGuard {
        fn new(new_dir: &std::path::Path) -> std::io::Result<Self> {
            let old_dir = std::env::current_dir()?;
            std::env::set_current_dir(new_dir)?;
            Ok(Self { old_dir })
        }
    }

    impl Drop for WorkingDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.old_dir);
        }
    }

    #[test]
    fn test_cmd_init_with_invalid_template_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = WorkingDirGuard::new(temp_dir.path()).unwrap();

        let result = cmd_init("nonexistent_template");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        drop(_guard);
        drop(temp_dir);
    }

    #[test]
    fn test_cmd_init_creates_config_in_current_dir() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = WorkingDirGuard::new(temp_dir.path()).unwrap();

        let result = cmd_init("balanced");
        assert!(result.is_ok());

        // Verify config file was created
        let config_path = temp_dir.path().join(config::CONFIG_FILE_NAME);
        assert!(config_path.exists());

        drop(_guard);
        drop(temp_dir);
    }

    #[test]
    fn test_cmd_init_with_existing_config_does_not_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = WorkingDirGuard::new(temp_dir.path()).unwrap();

        // Create config first time
        let result = cmd_init("balanced");
        assert!(result.is_ok());

        // Read original content
        let config_path = env::current_dir().unwrap().join(config::CONFIG_FILE_NAME);
        let original_content = fs::read_to_string(&config_path).unwrap();

        // Try to create again
        let result = cmd_init("aggressive");
        assert!(result.is_ok());

        // Verify file was NOT overwritten
        let new_content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(original_content, new_content);

        drop(_guard);
        drop(temp_dir);
    }

    #[test]
    fn test_cmd_init_with_all_templates() {
        // Test that all templates can be loaded without error
        let templates = ["balanced", "aggressive", "minimal"];

        for template_name in &templates {
            let template = config::Template::get(template_name);
            assert!(
                template.is_some(),
                "Template {} should exist",
                template_name
            );

            let t = template.unwrap();
            assert!(!t.name.is_empty());
            assert!(!t.description.is_empty());
        }
    }

    #[test]
    fn test_template_resolver_from_template() {
        let template = config::Template::get("balanced").unwrap();
        let config = config::TemplateResolver::from_template(&template);

        // Verify config structure is created
        assert!(config.profile.is_some());
    }
}
