//! Type definitions for dependency analysis
//!
//! Extracted from deps.rs to follow Single Responsibility Principle.
//! This module contains only the data structures, while deps.rs handles the analysis logic.

use std::collections::HashMap;

/// Issue severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum IssueSeverity {
    /// Critical - Will break in WASM or cause major bloat (>30%)
    Critical,
    /// High - Significant size impact (20-30%)
    High,
    /// Medium - Moderate size impact (10-20%)
    Medium,
    /// Low - Minor optimization opportunity (<10%)
    Low,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IssueSeverity::Critical => "P0",
            IssueSeverity::High => "P1",
            IssueSeverity::Medium => "P2",
            IssueSeverity::Low => "P3",
        };
        write!(f, "{}", s)
    }
}

/// A detected dependency issue
#[derive(Debug, Clone, serde::Serialize)]
pub struct DependencyIssue {
    /// Package name
    pub package: String,
    /// Version
    pub version: String,
    /// Severity
    pub severity: IssueSeverity,
    /// Issue description
    pub issue: String,
    /// Suggested fix
    pub suggestion: String,
    /// Estimated size impact in KB
    pub size_impact_kb: Option<(u32, u32)>,
    /// Estimated savings percentage
    pub savings_percent: Option<u8>,
}

/// Full dependency analysis report
#[derive(Debug, Clone, serde::Serialize)]
pub struct DependencyReport {
    /// Total number of dependencies
    pub total_deps: usize,
    /// Number of direct dependencies
    pub direct_deps: usize,
    /// Detected issues
    pub issues: Vec<DependencyIssue>,
    /// Duplicate versions
    pub duplicates: HashMap<String, Vec<String>>,
}

impl DependencyReport {
    /// Filter issues by severity level.
    ///
    /// Returns all dependency issues matching the specified severity.
    ///
    /// # Arguments
    ///
    /// * `severity` - The severity level to filter by
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::analyzer::deps::{DependencyAnalyzer, IssueSeverity};
    /// use std::path::Path;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let analyzer = DependencyAnalyzer::new(Path::new("."));
    /// let report = analyzer.analyze()?;
    ///
    /// // Get all critical issues
    /// let critical_issues = report.issues_by_severity(IssueSeverity::Critical);
    /// println!("Found {} critical issues", critical_issues.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn issues_by_severity(&self, severity: IssueSeverity) -> Vec<&DependencyIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .collect()
    }

    /// Calculate total estimated size savings from all identified issues.
    ///
    /// Returns a tuple of (minimum, maximum) estimated savings in kilobytes.
    ///
    /// # Returns
    ///
    /// A tuple `(min_kb, max_kb)` representing the range of potential savings.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::analyzer::deps::DependencyAnalyzer;
    /// use std::path::Path;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let analyzer = DependencyAnalyzer::new(Path::new("."));
    /// let report = analyzer.analyze()?;
    ///
    /// let (min, max) = report.total_estimated_savings_kb();
    /// println!("Potential savings: {}-{} KB", min, max);
    /// # Ok(())
    /// # }
    /// ```
    pub fn total_estimated_savings_kb(&self) -> (u32, u32) {
        let (min, max) = self
            .issues
            .iter()
            .filter_map(|i| i.size_impact_kb)
            .fold((0, 0), |(min_acc, max_acc), (min, max)| {
                (min_acc + min, max_acc + max)
            });
        (min, max)
    }

    /// Print formatted dependency report to console
    ///
    /// Delegates to [`crate::analyzer::deps_report::print_dependency_report`]
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wasm_slim::analyzer::DependencyAnalyzer;
    /// let analyzer = DependencyAnalyzer::new(".");
    /// let report = analyzer.analyze()?;
    /// report.print_report();
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn print_report(&self) {
        super::deps_report::print_dependency_report(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_severity_display_format() {
        assert_eq!(IssueSeverity::Critical.to_string(), "P0");
        assert_eq!(IssueSeverity::High.to_string(), "P1");
        assert_eq!(IssueSeverity::Medium.to_string(), "P2");
        assert_eq!(IssueSeverity::Low.to_string(), "P3");
    }

    #[test]
    fn test_issue_severity_ordering() {
        // Ordering follows declaration order in enum
        assert!(IssueSeverity::Critical < IssueSeverity::High);
        assert!(IssueSeverity::High < IssueSeverity::Medium);
        assert!(IssueSeverity::Medium < IssueSeverity::Low);
    }

    #[test]
    fn test_dependency_report_issues_by_severity() {
        let report = DependencyReport {
            total_deps: 10,
            direct_deps: 5,
            issues: vec![
                DependencyIssue {
                    package: "test1".to_string(),
                    version: "1.0".to_string(),
                    severity: IssueSeverity::Critical,
                    issue: "test".to_string(),
                    suggestion: "test".to_string(),
                    size_impact_kb: None,
                    savings_percent: None,
                },
                DependencyIssue {
                    package: "test2".to_string(),
                    version: "1.0".to_string(),
                    severity: IssueSeverity::Low,
                    issue: "test".to_string(),
                    suggestion: "test".to_string(),
                    size_impact_kb: None,
                    savings_percent: None,
                },
            ],
            duplicates: HashMap::new(),
        };

        let critical = report.issues_by_severity(IssueSeverity::Critical);
        assert_eq!(critical.len(), 1);
        assert_eq!(critical[0].package, "test1");
    }

    #[test]
    fn test_dependency_report_total_estimated_savings() {
        let report = DependencyReport {
            total_deps: 10,
            direct_deps: 5,
            issues: vec![
                DependencyIssue {
                    package: "test1".to_string(),
                    version: "1.0".to_string(),
                    severity: IssueSeverity::Critical,
                    issue: "test".to_string(),
                    suggestion: "test".to_string(),
                    size_impact_kb: Some((100, 200)),
                    savings_percent: Some(50),
                },
                DependencyIssue {
                    package: "test2".to_string(),
                    version: "1.0".to_string(),
                    severity: IssueSeverity::Low,
                    issue: "test".to_string(),
                    suggestion: "test".to_string(),
                    size_impact_kb: Some((50, 100)),
                    savings_percent: Some(20),
                },
            ],
            duplicates: HashMap::new(),
        };

        let (min, max) = report.total_estimated_savings_kb();
        assert_eq!(min, 150);
        assert_eq!(max, 300);
    }
}
