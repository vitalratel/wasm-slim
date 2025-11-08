//! Dependency tree analysis and optimization suggestions
//!
//! Uses `cargo metadata` to analyze dependencies and provide size optimization recommendations.

use cargo_metadata::{DependencyKind, MetadataCommand};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use thiserror::Error;

use super::allocator::AllocatorDetector;
use super::heavy_deps::{get_heavy_dependency_info, AlternativeType};

// Re-export types for backward compatibility
pub use super::deps_types::{DependencyIssue, DependencyReport, IssueSeverity};

/// Errors that can occur during dependency analysis
#[derive(Error, Debug)]
pub enum DependencyAnalysisError {
    /// Failed to run cargo metadata command
    #[error("Failed to run cargo metadata: {0}")]
    MetadataCommand(#[from] cargo_metadata::Error),

    /// No dependency resolution found in metadata
    #[error("No dependency resolution found in cargo metadata")]
    NoResolution,

    /// No root package found in metadata
    #[error("No root package found in cargo metadata")]
    NoRootPackage,

    /// I/O error during analysis
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Analyzes dependencies for optimization opportunities
///
/// This analyzer scans your project's dependencies to identify:
/// - Heavy dependencies that significantly impact WASM bundle size
/// - Duplicate dependencies with different versions
/// - Opportunities to switch to lighter alternatives
/// - Allocator optimization opportunities
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::analyzer::DependencyAnalyzer;
///
/// // Analyze dependencies for the current project
/// let analyzer = DependencyAnalyzer::new(".");
/// let report = analyzer.analyze()?;
///
/// // Check for issues
/// if !report.issues.is_empty() {
///     println!("Found {} optimization opportunities", report.issues.len());
///     for issue in &report.issues {
///         println!("- {}: {}", issue.severity, issue.issue);
///     }
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct DependencyAnalyzer {
    project_root: std::path::PathBuf,
}

impl DependencyAnalyzer {
    /// Create a new dependency analyzer for the given project root
    ///
    /// # Arguments
    ///
    /// * `project_root` - Path to the directory containing `Cargo.toml`
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::analyzer::DependencyAnalyzer;
    ///
    /// // Analyze current directory
    /// let analyzer = DependencyAnalyzer::new(".");
    ///
    /// // Analyze specific project
    /// let analyzer = DependencyAnalyzer::new("/path/to/project");
    /// ```
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
        }
    }

    /// Run full dependency analysis
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::analyzer::DependencyAnalyzer;
    ///
    /// let analyzer = DependencyAnalyzer::new(".");
    /// let report = analyzer.analyze()?;
    /// println!("Found {} dependency issues", report.issues.len());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use = "Analysis results should be used or printed"]
    pub fn analyze(&self) -> Result<DependencyReport, DependencyAnalysisError> {
        let metadata = MetadataCommand::new()
            .current_dir(&self.project_root)
            .exec()?;

        let mut issues = Vec::new();
        let mut duplicates: HashMap<String, Vec<String>> = HashMap::new();

        // Get workspace members
        let workspace_members: HashSet<_> = metadata.workspace_members.iter().collect();

        // Analyze each package
        for package in &metadata.packages {
            // Skip workspace members (analyze their dependencies, not them)
            if workspace_members.contains(&package.id) {
                continue;
            }

            // Check for heavy dependencies
            if let Some(heavy_info) = get_heavy_dependency_info(&package.name) {
                let severity = self.determine_severity(heavy_info.bundle_percent);

                for alternative in &heavy_info.alternatives {
                    issues.push(DependencyIssue {
                        package: package.name.clone(),
                        version: package.version.to_string(),
                        severity,
                        issue: heavy_info.reason.to_string(),
                        suggestion: self.format_suggestion(alternative),
                        size_impact_kb: alternative.size_kb.map(|(current_min, current_max)| {
                            let saved_min = heavy_info.size_kb.0.saturating_sub(current_max);
                            let saved_max = heavy_info.size_kb.1.saturating_sub(current_min);
                            (saved_min, saved_max)
                        }),
                        savings_percent: Some(alternative.savings_percent),
                    });
                }
            }

            // Track duplicate versions
            duplicates
                .entry(package.name.clone())
                .or_default()
                .push(package.version.to_string());
        }

        // Filter duplicates to only those with multiple versions
        duplicates.retain(|_, versions| {
            versions.sort();
            versions.dedup();
            versions.len() > 1
        });

        // Check for allocator optimization opportunity
        let allocator_detector = AllocatorDetector::new(&self.project_root);
        if let Ok(Some(allocator_issue)) =
            allocator_detector.check_allocator_optimization_with_metadata(&metadata)
        {
            issues.push(allocator_issue);
        }

        // Count dependencies
        let resolve = metadata
            .resolve
            .as_ref()
            .ok_or(DependencyAnalysisError::NoResolution)?;
        let total_deps = resolve.nodes.len();

        // Count direct dependencies
        let root_package = metadata
            .root_package()
            .ok_or(DependencyAnalysisError::NoRootPackage)?;
        let direct_deps = root_package
            .dependencies
            .iter()
            .filter(|d| matches!(d.kind, DependencyKind::Normal))
            .count();

        Ok(DependencyReport {
            total_deps,
            direct_deps,
            issues,
            duplicates,
        })
    }

    fn determine_severity(&self, bundle_percent: Option<(u8, u8)>) -> IssueSeverity {
        match bundle_percent {
            Some((_, max)) if max >= 30 => IssueSeverity::Critical,
            Some((_, max)) if max >= 20 => IssueSeverity::High,
            Some((_, max)) if max >= 10 => IssueSeverity::Medium,
            _ => IssueSeverity::Low,
        }
    }

    fn format_suggestion(&self, alternative: &super::heavy_deps::DependencyAlternative) -> String {
        match alternative.alt_type {
            AlternativeType::Replacement => {
                format!(
                    "Replace with {} ({})",
                    alternative.crate_name.unwrap_or("alternative"),
                    alternative.description
                )
            }
            AlternativeType::FeatureMinimization => {
                format!("Use default-features = false ({})", alternative.description)
            }
            AlternativeType::Split => {
                format!("Split into components ({})", alternative.description)
            }
            AlternativeType::Optional => {
                format!(
                    "Make optional via feature flag ({})",
                    alternative.description
                )
            }
            AlternativeType::WasmFix => {
                format!("Enable WASM features ({})", alternative.description)
            }
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;
    #[test]
    fn test_analyze_deps_with_workspace_members() {
        let analyzer = DependencyAnalyzer::new(".");

        // Should handle workspace structure without panicking
        let result = analyzer.analyze();
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_deps_with_missing_metadata() {
        let analyzer = DependencyAnalyzer::new("/nonexistent/path");

        // Should return error for missing project
        let result = analyzer.analyze();
        assert!(result.is_err());
    }

    #[test]
    fn test_determine_severity_edge_cases() {
        let analyzer = DependencyAnalyzer::new(".");

        // Test boundary values
        assert_eq!(
            analyzer.determine_severity(Some((30, 30))),
            IssueSeverity::Critical
        );
        assert_eq!(
            analyzer.determine_severity(Some((0, 0))),
            IssueSeverity::Low
        );
        assert_eq!(
            analyzer.determine_severity(Some((100, 200))),
            IssueSeverity::Critical
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_severity_returns_correct_level_for_size_ranges() {
        let analyzer = DependencyAnalyzer::new(".");

        assert_eq!(
            analyzer.determine_severity(Some((30, 40))),
            IssueSeverity::Critical
        );
        assert_eq!(
            analyzer.determine_severity(Some((20, 25))),
            IssueSeverity::High
        );
        assert_eq!(
            analyzer.determine_severity(Some((10, 15))),
            IssueSeverity::Medium
        );
        assert_eq!(
            analyzer.determine_severity(Some((5, 8))),
            IssueSeverity::Low
        );
        assert_eq!(analyzer.determine_severity(None), IssueSeverity::Low);
    }
}
