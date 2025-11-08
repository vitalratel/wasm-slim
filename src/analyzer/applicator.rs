//! Automatic application of dependency optimization suggestions
//!
//! Takes DependencyReport and applies fixes to Cargo.toml automatically.

use crate::infra::{FileSystem, RealFileSystem};
use crate::optimizer::BackupManager;
use anyhow::{Context, Result};
use console::style;
use std::collections::HashMap;
use std::path::Path;
use toml_edit::{value, DocumentMut, Item, Table};

use super::deps::{DependencyIssue, DependencyReport};
use super::heavy_deps::{get_heavy_dependency_info, AlternativeType};

/// Applies dependency optimization suggestions to Cargo.toml
pub struct SuggestionApplicator<FS: FileSystem = RealFileSystem> {
    project_root: std::path::PathBuf,
    fs: FS,
    backup_manager: BackupManager<FS>,
}

impl SuggestionApplicator<RealFileSystem> {
    /// Create a new suggestion applicator for the given project root
    ///
    /// # Arguments
    /// * `project_root` - Path to the project root containing Cargo.toml
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::analyzer::SuggestionApplicator;
    /// use std::path::Path;
    ///
    /// let applicator = SuggestionApplicator::new(Path::new("."));
    /// // Use with a DependencyReport to apply fixes
    /// ```
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self::with_fs(project_root, RealFileSystem)
    }
}

impl<FS: FileSystem + Clone> SuggestionApplicator<FS> {
    /// Create a new suggestion applicator with a custom filesystem implementation
    pub fn with_fs(project_root: impl AsRef<Path>, fs: FS) -> Self {
        let project_root = project_root.as_ref().to_path_buf();
        let backup_manager = BackupManager::with_fs(&project_root, fs.clone());
        Self {
            project_root,
            fs,
            backup_manager,
        }
    }

    /// Apply suggestions from a dependency report
    ///
    /// Automatically fixes detected dependency issues by:
    /// - Disabling problematic default features
    /// - Replacing heavy dependencies with lighter alternatives
    /// - Optimizing feature flags for WASM targets
    ///
    /// # Arguments
    /// * `report` - Dependency analysis report with issues
    /// * `dry_run` - If true, show what would be changed without modifying files
    ///
    /// # Returns
    /// Number of fixes applied
    ///
    /// # Errors
    /// Returns error if Cargo.toml is not found, cannot be parsed, or write fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::analyzer::{DependencyAnalyzer, SuggestionApplicator};
    /// use std::path::Path;
    ///
    /// let analyzer = DependencyAnalyzer::new(Path::new("."));
    /// let report = analyzer.analyze()?;
    ///
    /// let applicator = SuggestionApplicator::new(Path::new("."));
    /// let fixes = applicator.apply_suggestions(&report, false)?;
    /// println!("Applied {} fixes", fixes);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn apply_suggestions(&self, report: &DependencyReport, dry_run: bool) -> Result<usize> {
        let cargo_toml_path = self.project_root.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            anyhow::bail!("Cargo.toml not found in {}", self.project_root.display());
        }

        let content = self
            .fs
            .read_to_string(&cargo_toml_path)
            .context("Failed to read Cargo.toml")?;

        let mut doc = content
            .parse::<DocumentMut>()
            .context("Failed to parse Cargo.toml")?;

        let mut fixes_applied = 0;

        // Group issues by package
        let mut issues_by_package: HashMap<String, Vec<&DependencyIssue>> = HashMap::new();
        for issue in &report.issues {
            issues_by_package
                .entry(issue.package.clone())
                .or_default()
                .push(issue);
        }

        // Apply fixes for each package
        for package_name in issues_by_package.keys() {
            // Get the heavy dependency info
            if let Some(heavy_info) = get_heavy_dependency_info(package_name) {
                // Find the best alternative (highest savings)
                let best_alternative = heavy_info
                    .alternatives
                    .iter()
                    .max_by_key(|alt| alt.savings_percent);

                if let Some(alternative) = best_alternative {
                    if self.apply_fix(&mut doc, package_name, alternative.alt_type, dry_run)? {
                        fixes_applied += 1;

                        if !dry_run {
                            println!(
                                "   {} Fixed {} ({}% savings expected)",
                                style("✓").green(),
                                style(package_name).bold(),
                                alternative.savings_percent
                            );
                        } else {
                            println!(
                                "   {} Would fix {} ({}% savings expected)",
                                style("•").yellow(),
                                style(package_name).bold(),
                                alternative.savings_percent
                            );
                        }
                    }
                }
            }
        }

        if !dry_run && fixes_applied > 0 {
            // Create backup before writing
            let backup_path = self
                .backup_manager
                .create_backup(&cargo_toml_path)
                .context("Failed to create backup")?;
            println!(
                "   {} Backup created: {}",
                style("💾").dim(),
                backup_path.display()
            );

            // Write updated Cargo.toml
            self.fs
                .write(&cargo_toml_path, doc.to_string().as_bytes())
                .context("Failed to write Cargo.toml")?;
        }

        Ok(fixes_applied)
    }

    /// Apply a specific fix based on alternative type
    fn apply_fix(
        &self,
        doc: &mut DocumentMut,
        package_name: &str,
        alt_type: AlternativeType,
        dry_run: bool,
    ) -> Result<bool> {
        match alt_type {
            AlternativeType::FeatureMinimization => {
                self.apply_feature_minimization(doc, package_name, dry_run)
            }
            AlternativeType::WasmFix => self.apply_wasm_fix(doc, package_name, dry_run),
            AlternativeType::Replacement | AlternativeType::Split | AlternativeType::Optional => {
                // These require manual intervention or more complex logic
                // For now, we skip them (Phase 4.5.1 enhancement)
                Ok(false)
            }
        }
    }

    // ===== TOML Manipulation Helpers =====

    /// Get mutable reference to dependencies table
    fn get_dependencies_table(doc: &mut DocumentMut) -> Option<&mut Table> {
        match doc.get_mut("dependencies") {
            Some(Item::Table(table)) => Some(table),
            _ => None,
        }
    }

    /// Convert a string version to a table with the given key-value pairs
    fn version_string_to_table(version_str: &str) -> Table {
        let mut dep_table = Table::new();
        dep_table.insert("version", value(version_str));
        dep_table
    }

    /// Convert inline table to regular table
    fn inline_table_to_regular(inline_table: &toml_edit::InlineTable) -> Table {
        let mut dep_table = Table::new();
        for (key, val) in inline_table.iter() {
            dep_table.insert(key, Item::Value(val.clone()));
        }
        dep_table
    }

    /// Add features to a table, avoiding duplicates
    fn add_features_to_table(table: &mut Table, features: &[&str]) -> bool {
        if features.is_empty() {
            return false;
        }

        let features_item = table
            .entry("features")
            .or_insert(value(toml_edit::Array::new()));

        if let Some(features_array) = features_item.as_array_mut() {
            let mut added = false;
            for feature in features {
                // Only add if not already present
                if !features_array.iter().any(|v| v.as_str() == Some(feature)) {
                    features_array.push(*feature);
                    added = true;
                }
            }
            return added;
        }

        false
    }

    /// Create a features array from a slice of feature strings
    fn create_features_array(features: &[&str]) -> toml_edit::Array {
        let mut features_array = toml_edit::Array::new();
        for feature in features {
            features_array.push(*feature);
        }
        features_array
    }

    // ===== End TOML Helpers =====

    /// Apply feature minimization (default-features = false)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::analyzer::SuggestionApplicator;
    /// use std::path::Path;
    /// use toml_edit::DocumentMut;
    ///
    /// let applicator = SuggestionApplicator::new(Path::new("."));
    /// let mut doc = "[dependencies]\nlopdf = \"0.32\"".parse::<DocumentMut>().unwrap();
    /// // Applies feature minimization to the document
    /// ```
    fn apply_feature_minimization(
        &self,
        doc: &mut DocumentMut,
        package_name: &str,
        _dry_run: bool,
    ) -> Result<bool> {
        // Get dependencies table
        let deps = match Self::get_dependencies_table(doc) {
            Some(table) => table,
            None => return Ok(false),
        };

        // Find the dependency
        if let Some(dep_item) = deps.get_mut(package_name) {
            match dep_item {
                Item::Value(val) if val.is_str() => {
                    // Simple version string, convert to table
                    let version = val.as_str().context("Failed to extract version string")?;
                    let version = version.to_string();

                    let mut dep_table = Self::version_string_to_table(&version);
                    dep_table.insert("default-features", value(false));

                    // Add common minimal features based on package
                    if let Some(features) = self.get_minimal_features(package_name) {
                        let features_array = Self::create_features_array(&features);
                        dep_table.insert("features", value(features_array));
                    }

                    *dep_item = Item::Table(dep_table);
                    return Ok(true);
                }
                Item::Value(val) => {
                    // Inline table like { version = "1.10" }
                    if let Some(inline_table) = val.as_inline_table() {
                        // Convert inline table to regular table
                        let mut dep_table = Self::inline_table_to_regular(inline_table);

                        // Add default-features = false if not present
                        if !dep_table.contains_key("default-features") {
                            dep_table.insert("default-features", value(false));

                            // Add minimal features if not present
                            if !dep_table.contains_key("features") {
                                if let Some(features) = self.get_minimal_features(package_name) {
                                    let features_array = Self::create_features_array(&features);
                                    dep_table.insert("features", value(features_array));
                                }
                            }

                            *dep_item = Item::Table(dep_table);
                            return Ok(true);
                        }
                    }
                }
                Item::Table(table) => {
                    // Already a table, just set default-features = false
                    if !table.contains_key("default-features") {
                        table.insert("default-features", value(false));

                        // Add minimal features if not present
                        if !table.contains_key("features") {
                            if let Some(features) = self.get_minimal_features(package_name) {
                                let features_array = Self::create_features_array(&features);
                                table.insert("features", value(features_array));
                            }
                        }
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }

        Ok(false)
    }

    /// Apply WASM-specific fixes (e.g., getrandom features)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::analyzer::SuggestionApplicator;
    /// use std::path::Path;
    /// use toml_edit::DocumentMut;
    ///
    /// let applicator = SuggestionApplicator::new(Path::new("."));
    /// let mut doc = "[dependencies]\ngetrandom = \"0.2\"".parse::<DocumentMut>().unwrap();
    /// // Applies WASM-specific features to getrandom
    /// ```
    fn apply_wasm_fix(
        &self,
        doc: &mut DocumentMut,
        package_name: &str,
        _dry_run: bool,
    ) -> Result<bool> {
        let deps = match Self::get_dependencies_table(doc) {
            Some(table) => table,
            None => return Ok(false),
        };

        if let Some(dep_item) = deps.get_mut(package_name) {
            match dep_item {
                Item::Value(val) if val.is_str() => {
                    // Simple version string, convert to table with WASM features
                    let version = val.as_str().context("Failed to extract version string")?;
                    let version = version.to_string();

                    let mut dep_table = Self::version_string_to_table(&version);

                    // Add WASM-specific features
                    if let Some(wasm_features) =
                        self.get_wasm_features(package_name, Some(&version))
                    {
                        let features_array = Self::create_features_array(&wasm_features);
                        dep_table.insert("features", value(features_array));
                    }

                    *dep_item = Item::Table(dep_table);
                    return Ok(true);
                }
                Item::Value(val) => {
                    // Inline table like { version = "1.10" }
                    if let Some(inline_table) = val.as_inline_table() {
                        // Extract version from inline table
                        let version = inline_table
                            .get("version")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        // Convert inline table to regular table
                        let mut dep_table = Self::inline_table_to_regular(inline_table);

                        // Add WASM-specific features
                        if let Some(wasm_features) =
                            self.get_wasm_features(package_name, version.as_deref())
                        {
                            let added = Self::add_features_to_table(&mut dep_table, &wasm_features);
                            if added {
                                *dep_item = Item::Table(dep_table);
                                return Ok(true);
                            }
                        }
                    }
                }
                Item::Table(table) => {
                    // Extract version from table
                    let version = table
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Add WASM features to existing table
                    if let Some(wasm_features) =
                        self.get_wasm_features(package_name, version.as_deref())
                    {
                        return Ok(Self::add_features_to_table(table, &wasm_features));
                    }
                }
                _ => {}
            }
        }

        Ok(false)
    }

    /// Get minimal features for known packages
    fn get_minimal_features(&self, package_name: &str) -> Option<Vec<&'static str>> {
        match package_name {
            "lopdf" => Some(vec!["pom_parser"]),
            "image" => Some(vec!["png"]),
            _ => None,
        }
    }

    /// Get WASM-specific features for known packages
    ///
    /// Returns the appropriate feature flags based on package name and version.
    /// For getrandom, handles both v0.2 (uses "js") and v0.3+ (uses "wasm_js").
    fn get_wasm_features(
        &self,
        package_name: &str,
        version: Option<&str>,
    ) -> Option<Vec<&'static str>> {
        match package_name {
            "getrandom" => {
                // getrandom 0.2.x uses "js" feature
                // getrandom 0.3+ uses "wasm_js" feature
                if let Some(ver) = version {
                    if ver.starts_with("0.2") || ver.starts_with("^0.2") {
                        return Some(vec!["js"]);
                    }
                }
                // Default to wasm_js for 0.3+ or unknown versions
                Some(vec!["wasm_js"])
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ============ Happy Path Tests ============

    #[test]
    fn test_apply_feature_minimization_adds_default_features_false() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        // Create a test Cargo.toml
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
regex = "1.10"
"#;
        fs::write(&cargo_toml, content).unwrap();

        let applicator = SuggestionApplicator::new(temp_dir.path());

        let mut doc: DocumentMut = content.parse().unwrap();
        let result = applicator.apply_feature_minimization(&mut doc, "regex", false);

        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify the change
        let updated = doc.to_string();
        assert!(updated.contains("default-features = false"));
    }

    #[test]
    fn test_apply_wasm_fix_for_getrandom_adds_wasm_js_feature() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
getrandom = "0.3"
"#;
        fs::write(&cargo_toml, content).unwrap();

        let applicator = SuggestionApplicator::new(temp_dir.path());

        let mut doc: DocumentMut = content.parse().unwrap();
        let result = applicator.apply_wasm_fix(&mut doc, "getrandom", false);

        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify the change
        let updated = doc.to_string();
        assert!(updated.contains("wasm_js"));
    }

    // ============ P2-TEST-COV-011: Negative Test Cases ============

    #[test]
    fn test_apply_suggestions_with_missing_cargo_toml_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let applicator = SuggestionApplicator::new(temp_dir.path());

        // Create an empty report
        let report = DependencyReport {
            total_deps: 0,
            direct_deps: 0,
            issues: vec![],
            duplicates: std::collections::HashMap::new(),
        };

        let result = applicator.apply_suggestions(&report, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cargo.toml not found"));
    }

    #[test]
    fn test_apply_suggestions_with_malformed_toml_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        // Create malformed TOML
        fs::write(&cargo_toml, "[invalid toml {{").unwrap();

        let applicator = SuggestionApplicator::new(temp_dir.path());
        let report = DependencyReport {
            total_deps: 0,
            direct_deps: 0,
            issues: vec![],
            duplicates: std::collections::HashMap::new(),
        };

        let result = applicator.apply_suggestions(&report, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
    }

    #[test]
    fn test_apply_feature_minimization_with_no_dependencies_table_returns_false() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        let result = applicator.apply_feature_minimization(&mut doc, "regex", false);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (no fix applied)
    }

    #[test]
    fn test_apply_feature_minimization_with_nonexistent_package_returns_false() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
other-package = "1.0"
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        let result = applicator.apply_feature_minimization(&mut doc, "non-existent", false);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (package not found)
    }

    #[test]
    fn test_apply_feature_minimization_with_existing_default_features_false_returns_false() {
        // Test with dependency that already has default-features = false
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
regex = { version = "1.10", default-features = false }
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        let result = applicator.apply_feature_minimization(&mut doc, "regex", false);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (already has default-features)

        // Verify no changes were made
        let updated = doc.to_string();
        assert!(updated.contains("default-features = false"));
        // Should not have duplicate or modified the existing setting
        assert_eq!(updated.matches("default-features").count(), 1);
    }

    #[test]
    fn test_apply_wasm_fix_with_no_dependencies_table_returns_false() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        let result = applicator.apply_wasm_fix(&mut doc, "getrandom", false);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (no fix applied)
    }

    #[test]
    fn test_apply_wasm_fix_with_nonexistent_package_returns_false() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
other-package = "1.0"
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        let result = applicator.apply_wasm_fix(&mut doc, "getrandom", false);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (package not found)
    }

    #[test]
    fn test_apply_wasm_fix_with_existing_wasm_js_feature_does_not_duplicate() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
getrandom = { version = "0.3", features = ["wasm_js"] }
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        let result = applicator.apply_wasm_fix(&mut doc, "getrandom", false);
        // Should succeed but not duplicate the feature
        assert!(result.is_ok());

        let updated = doc.to_string();
        // Count occurrences of "wasm_js" - should still be 1
        let count = updated.matches("wasm_js").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_apply_suggestions_with_dry_run_does_not_modify_file() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
regex = "1.10"
"#;
        fs::write(&cargo_toml, content).unwrap();
        let original_content = fs::read_to_string(&cargo_toml).unwrap();

        let applicator = SuggestionApplicator::new(temp_dir.path());
        let report = DependencyReport {
            total_deps: 1,
            direct_deps: 1,
            issues: vec![],
            duplicates: std::collections::HashMap::new(),
        };

        let result = applicator.apply_suggestions(&report, true); // dry_run = true
        assert!(result.is_ok());

        // File should not be modified
        let current_content = fs::read_to_string(&cargo_toml).unwrap();
        assert_eq!(original_content, current_content);

        // No backup should be created
        let backup_path = cargo_toml.with_extension("toml.backup");
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_apply_feature_minimization_with_inline_table_converts_to_regular_table() {
        // Test with inline table format dependency (now supported!)
        // Inline tables like { version = "1.10" } are parsed as Value(InlineTable)
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
regex = { version = "1.10" }
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        let result = applicator.apply_feature_minimization(&mut doc, "regex", false);
        assert!(result.is_ok());

        // Should return true - inline tables are now converted to regular tables
        assert!(
            result.unwrap(),
            "Inline table format should now be supported"
        );

        // Document should be converted to regular table format with default-features = false
        let updated = doc.to_string();
        assert!(
            !updated.contains("{ version = \"1.10\" }"),
            "Should convert from inline table"
        );
        assert!(
            updated.contains("default-features = false"),
            "Should add default-features"
        );
        assert!(
            updated.contains("version = \"1.10\""),
            "Should preserve version"
        );
    }

    #[test]
    fn test_apply_feature_minimization_with_multiline_table_adds_default_features() {
        // Test with multi-line table format (this IS supported)
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies.regex]
version = "1.10"
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        let result = applicator.apply_feature_minimization(&mut doc, "regex", false);
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should work with multi-line table format

        let updated = doc.to_string();
        assert!(updated.contains("default-features = false"));
    }

    #[test]
    fn test_get_minimal_features_with_known_packages_returns_features() {
        let applicator = SuggestionApplicator::new(".");

        // Test known packages
        assert_eq!(
            applicator.get_minimal_features("lopdf"),
            Some(vec!["pom_parser"])
        );
        assert_eq!(applicator.get_minimal_features("image"), Some(vec!["png"]));

        // Test unknown package
        assert_eq!(applicator.get_minimal_features("unknown-package"), None);
    }

    #[test]
    fn test_get_wasm_features_with_known_packages_returns_features() {
        let applicator = SuggestionApplicator::new(".");

        // Test getrandom 0.3+ (default: wasm_js)
        assert_eq!(
            applicator.get_wasm_features("getrandom", None),
            Some(vec!["wasm_js"])
        );
        assert_eq!(
            applicator.get_wasm_features("getrandom", Some("0.3")),
            Some(vec!["wasm_js"])
        );

        // Test getrandom 0.2 (js feature)
        assert_eq!(
            applicator.get_wasm_features("getrandom", Some("0.2")),
            Some(vec!["js"])
        );
        assert_eq!(
            applicator.get_wasm_features("getrandom", Some("^0.2")),
            Some(vec!["js"])
        );

        // Test unknown package
        assert_eq!(applicator.get_wasm_features("unknown-package", None), None);
    }

    #[test]
    fn test_apply_fix_with_unsupported_alternative_type_handles_gracefully() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
some-package = "1.0"
"#;
        let mut doc: DocumentMut = content.parse().unwrap();
        let applicator = SuggestionApplicator::new(".");

        // Test unsupported alternative types
        let result = applicator.apply_fix(
            &mut doc,
            "some-package",
            AlternativeType::Replacement,
            false,
        );
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (not supported yet)

        let result = applicator.apply_fix(&mut doc, "some-package", AlternativeType::Split, false);
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let result =
            applicator.apply_fix(&mut doc, "some-package", AlternativeType::Optional, false);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
