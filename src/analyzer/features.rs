//! Feature flag analyzer for detecting unused features
//!
//! Analyzes Cargo.toml dependencies to find enabled features that might not be used,
//! potentially reducing binary size by 10-30%.

use thiserror::Error;

/// Errors that can occur during feature analysis
#[derive(Error, Debug)]
pub enum FeatureAnalysisError {
    /// cargo metadata command failed
    #[error("Failed to run cargo metadata: {0}")]
    MetadataCommand(#[from] cargo_metadata::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// UTF-8 conversion error
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// cargo tree command failed
    #[error("cargo tree failed")]
    CargoTreeFailed,
}
use crate::infra::{CommandExecutor, RealCommandExecutor};
use cargo_metadata::MetadataCommand;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Feature flag analyzer
///
/// Identifies potentially unused feature flags in dependencies
/// to reduce WASM bundle size.
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::analyzer::FeatureAnalyzer;
/// use std::path::Path;
///
/// let analyzer = FeatureAnalyzer::new(Path::new("."));
/// let results = analyzer.analyze()?;
///
/// println!("Found {} potentially unused features", results.unused_features.len());
/// println!("Estimated savings: {} KB", results.estimated_savings_kb);
///
/// for feature in &results.unused_features {
///     println!("  {}::{} ({})", feature.package, feature.feature, feature.enabled_by);
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct FeatureAnalyzer<CE: CommandExecutor = RealCommandExecutor> {
    project_root: std::path::PathBuf,
    cmd_executor: CE,
}

/// Unused feature detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedFeature {
    /// Package name
    pub package: String,
    /// Feature name
    pub feature: String,
    /// How the feature is enabled (default, explicit, transitive)
    pub enabled_by: String,
    /// Estimated size impact in KB
    pub estimated_impact_kb: u64,
    /// Confidence level (High, Medium, Low)
    pub confidence: String,
}

/// Complete feature analysis results
#[derive(Debug, Serialize, Deserialize)]
pub struct FeatureAnalysisResults {
    /// Total features analyzed
    pub total_features: usize,
    /// Potentially unused features
    pub unused_features: Vec<UnusedFeature>,
    /// Estimated total savings in KB
    pub estimated_savings_kb: u64,
    /// Recommendations
    pub recommendations: Vec<String>,
}

impl FeatureAnalyzer<RealCommandExecutor> {
    /// Create a new feature analyzer with real command execution
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self::with_executor(project_root, RealCommandExecutor)
    }
}

impl<CE: CommandExecutor> FeatureAnalyzer<CE> {
    /// Create a new feature analyzer with a custom command executor
    pub fn with_executor(project_root: impl AsRef<Path>, cmd_executor: CE) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            cmd_executor,
        }
    }

    /// Analyze feature flags in the project
    #[must_use = "Analysis results should be used or printed"]
    pub fn analyze(&self) -> Result<FeatureAnalysisResults, FeatureAnalysisError> {
        // Get cargo metadata
        let metadata = MetadataCommand::new()
            .current_dir(&self.project_root)
            .exec()?;

        // Get feature tree
        let feature_tree = self.get_feature_tree()?;

        let mut unused_features = Vec::new();
        let mut total_features = 0;

        // Analyze each package's features
        for package in &metadata.packages {
            // Skip workspace members (analyze only dependencies)
            if metadata.workspace_members.contains(&package.id) {
                continue;
            }

            // Get enabled features for this package
            let enabled_features = feature_tree
                .get(&package.name.to_string())
                .cloned()
                .unwrap_or_default();

            total_features += enabled_features.len();

            // Check for default features
            if enabled_features.contains("default") && !package.features.is_empty() {
                // Default features are often bloated
                let default_features = package.features.get("default").cloned().unwrap_or_default();

                if !default_features.is_empty() {
                    unused_features.push(UnusedFeature {
                        package: package.name.to_string(),
                        feature: "default".to_string(),
                        enabled_by: "implicit".to_string(),
                        estimated_impact_kb: 50, // Conservative estimate
                        confidence: "Medium".to_string(),
                    });
                }
            }

            // Check for commonly unused features
            for feature in &enabled_features {
                if self.is_commonly_unused_feature(&package.name, feature) {
                    unused_features.push(UnusedFeature {
                        package: package.name.to_string(),
                        feature: feature.clone(),
                        enabled_by: "explicit".to_string(),
                        estimated_impact_kb: self.estimate_feature_impact(&package.name, feature),
                        confidence: self.get_confidence_level(&package.name, feature),
                    });
                }
            }
        }

        // Calculate total estimated savings
        let estimated_savings_kb = unused_features.iter().map(|f| f.estimated_impact_kb).sum();

        // Generate recommendations
        let recommendations = self.generate_recommendations(&unused_features);

        Ok(FeatureAnalysisResults {
            total_features,
            unused_features,
            estimated_savings_kb,
            recommendations,
        })
    }

    /// Get feature tree using cargo tree
    fn get_feature_tree(&self) -> Result<HashMap<String, HashSet<String>>, FeatureAnalysisError> {
        let output = self.cmd_executor.execute(
            |cmd| {
                cmd.arg("tree")
                    .arg("--format")
                    .arg("{p} {f}")
                    .arg("--edges")
                    .arg("normal")
                    .current_dir(&self.project_root)
            },
            "cargo",
        )?;

        if !output.status.success() {
            return Err(FeatureAnalysisError::CargoTreeFailed);
        }

        let stdout = String::from_utf8(output.stdout)?;
        let mut feature_map = HashMap::new();

        for line in stdout.lines() {
            // Parse format: "package_name v0.1.0 feature1,feature2"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let package_name = parts[0];

                // Extract features from the rest
                let features_parts: Vec<&str> = parts.iter().skip(1).copied().collect();
                let features_str = features_parts.join(" ");
                let features: HashSet<String> = features_str
                    .split(',')
                    .filter(|s| !s.is_empty() && !s.starts_with('v'))
                    .map(|s| String::from(s.trim()))
                    .collect();

                if !features.is_empty() {
                    feature_map
                        .entry(String::from(package_name))
                        .or_insert_with(HashSet::new)
                        .extend(features);
                }
            }
        }

        Ok(feature_map)
    }

    /// Check if a feature is commonly unused
    fn is_commonly_unused_feature(&self, package: &str, feature: &str) -> bool {
        // Database of commonly unused features by package
        match (package, feature) {
            // Serde features
            ("serde", "rc") => true,
            ("serde", "unstable") => true,

            // Tokio features
            ("tokio", "fs") => true, // Not needed in WASM
            ("tokio", "io-std") => true,
            ("tokio", "process") => true,
            ("tokio", "signal") => true,

            // Regex features
            ("regex", "unicode") => true, // Large, often unnecessary
            ("regex", "unicode-perl") => true,

            // Chrono features
            ("chrono", "clock") => true, // WASM alternatives exist
            ("chrono", "std") => false,  // Usually needed

            // Generic patterns
            (_, "std") if feature == "std" => false, // Usually needed
            (_, f) if f.contains("test") || f.contains("bench") => true,
            (_, f) if f.contains("unstable") || f.contains("nightly") => true,

            _ => false,
        }
    }

    /// Estimate size impact of a feature
    fn estimate_feature_impact(&self, package: &str, feature: &str) -> u64 {
        // Estimates based on common feature sizes
        match (package, feature) {
            ("regex", "unicode") => 300, // Unicode tables are large
            ("tokio", _) => 100,
            ("serde", _) => 20,
            ("chrono", _) => 50,
            (_, "default") => 50,
            _ => 30, // Conservative default
        }
    }

    /// Get confidence level for unused feature detection
    fn get_confidence_level(&self, package: &str, feature: &str) -> String {
        match (package, feature) {
            // High confidence: Well-known bloat features
            ("regex", "unicode") => "High".to_string(),
            ("tokio", "fs") | ("tokio", "process") => "High".to_string(),

            // Medium confidence: Commonly unused
            (_, "default") => "Medium".to_string(),
            (_, f) if f.contains("test") => "High".to_string(),

            // Low confidence: Need manual verification
            _ => "Low".to_string(),
        }
    }

    /// Generate recommendations based on unused features
    fn generate_recommendations(&self, unused_features: &[UnusedFeature]) -> Vec<String> {
        let mut recommendations = Vec::new();

        if unused_features.is_empty() {
            recommendations.push("✅ No obvious unused features detected".to_string());
            return recommendations;
        }

        // Group by package
        let mut by_package: HashMap<String, Vec<&UnusedFeature>> = HashMap::new();
        for feature in unused_features {
            by_package
                .entry(feature.package.clone())
                .or_default()
                .push(feature);
        }

        for (package, features) in by_package {
            if features.iter().any(|f| f.feature == "default") {
                recommendations.push(format!(
                    "Add 'default-features = false' to '{}' in Cargo.toml",
                    package
                ));
            }

            let explicit_features: Vec<_> =
                features.iter().filter(|f| f.feature != "default").collect();

            if !explicit_features.is_empty() {
                let feature_names: Vec<_> = explicit_features
                    .iter()
                    .map(|f| f.feature.as_str())
                    .collect();
                recommendations.push(format!(
                    "Consider removing features {:?} from '{}' if not used",
                    feature_names, package
                ));
            }
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_commonly_unused_feature_identifies_known_features() {
        let analyzer = FeatureAnalyzer::new(".");

        assert!(analyzer.is_commonly_unused_feature("regex", "unicode"));
        assert!(analyzer.is_commonly_unused_feature("tokio", "fs"));
        assert!(!analyzer.is_commonly_unused_feature("serde", "derive"));
    }

    #[test]
    fn test_estimate_feature_impact_returns_size_estimate() {
        let analyzer = FeatureAnalyzer::new(".");

        let impact = analyzer.estimate_feature_impact("regex", "unicode");
        assert!(impact > 100); // Should be significant

        let default_impact = analyzer.estimate_feature_impact("unknown", "default");
        assert!(default_impact > 0);
    }

    #[test]
    fn test_get_confidence_level_returns_correct_level() {
        let analyzer = FeatureAnalyzer::new(".");

        assert_eq!(analyzer.get_confidence_level("regex", "unicode"), "High");
        assert_eq!(
            analyzer.get_confidence_level("unknown", "default"),
            "Medium"
        );
    }

    #[test]
    fn test_generate_recommendations_creates_actionable_suggestions() {
        let analyzer = FeatureAnalyzer::new(".");

        let unused = vec![UnusedFeature {
            package: "serde".to_string(),
            feature: "default".to_string(),
            enabled_by: "implicit".to_string(),
            estimated_impact_kb: 50,
            confidence: "Medium".to_string(),
        }];

        let recs = analyzer.generate_recommendations(&unused);
        assert!(!recs.is_empty());
        assert!(recs[0].contains("default-features = false"));
    }
}
