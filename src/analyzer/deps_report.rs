//! Dependency analysis report formatting
//!
//! Provides console output formatting for dependency analysis results.
//!
//! # Overview
//!
//! This module handles the presentation layer for dependency analysis,
//! including:
//! - Colored terminal output
//! - Issue grouping by severity
//! - Size impact formatting
//! - Duplicate version detection display
//!
//! # Examples
//!
//! ```no_run
//! use wasm_slim::analyzer::deps::DependencyReport;
//! use wasm_slim::analyzer::deps_report::print_dependency_report;
//!
//! # fn get_report() -> DependencyReport { unimplemented!() }
//! let report = get_report();
//! print_dependency_report(&report);
//! ```

use console::style;

use super::deps::{DependencyIssue, DependencyReport, IssueSeverity};

/// Print formatted dependency analysis report to console
///
/// Displays:
/// - Total and direct dependency counts
/// - Issues grouped by severity (P0-P3)
/// - Size impact estimates
/// - Duplicate version warnings
///
/// # Arguments
///
/// * `report` - The dependency analysis report to print
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::analyzer::deps::DependencyReport;
/// use wasm_slim::analyzer::deps_report::print_dependency_report;
///
/// # fn get_report() -> DependencyReport { unimplemented!() }
/// let report = get_report();
/// print_dependency_report(&report);
/// ```
pub fn print_dependency_report(report: &DependencyReport) {
    println!(
        "\n{} {} Dependency Analysis Report",
        style("📦").bold(),
        style("WASM").cyan().bold()
    );
    println!(
        "   {} Total dependencies: {}",
        style("→").dim(),
        style(report.total_deps).yellow()
    );
    println!(
        "   {} Direct dependencies: {}",
        style("→").dim(),
        style(report.direct_deps).yellow()
    );

    if !report.issues.is_empty() {
        println!(
            "\n{} {} Optimization Opportunities Found",
            style("🔍").bold(),
            style(report.issues.len()).yellow().bold()
        );

        let (min_kb, max_kb) = report.total_estimated_savings_kb();
        if max_kb > 0 {
            println!(
                "   {} Estimated savings: {}-{} KB",
                style("→").dim(),
                style(min_kb).green().bold(),
                style(max_kb).green().bold()
            );
        }

        // Group by severity
        for severity in &[
            IssueSeverity::Critical,
            IssueSeverity::High,
            IssueSeverity::Medium,
            IssueSeverity::Low,
        ] {
            let severity_issues = report.issues_by_severity(*severity);
            if !severity_issues.is_empty() {
                println!(
                    "\n{} {} Issues ({} found):",
                    style(format!("[{}]", severity)).bold(),
                    match severity {
                        IssueSeverity::Critical => style("Critical").red().bold(),
                        IssueSeverity::High => style("High").red(),
                        IssueSeverity::Medium => style("Medium").yellow(),
                        IssueSeverity::Low => style("Low").cyan(),
                    },
                    severity_issues.len()
                );

                for issue in severity_issues {
                    print_issue(issue);
                }
            }
        }
    } else {
        println!(
            "\n{} {} All dependencies optimized!",
            style("✅").bold(),
            style("Great!").green().bold()
        );
    }

    // Duplicate versions
    if !report.duplicates.is_empty() {
        println!(
            "\n{} {} Duplicate Versions Detected",
            style("⚠️").bold(),
            style("Warning").yellow().bold()
        );
        for (name, versions) in &report.duplicates {
            println!(
                "   {} {}: {}",
                style("→").dim(),
                style(name).yellow(),
                versions.join(", ")
            );
        }
    }
}

/// Print a single dependency issue with formatting
///
/// Displays:
/// - Package name and version
/// - Issue description
/// - Size impact (if available)
/// - Suggested fix
///
/// # Arguments
///
/// * `issue` - The dependency issue to print
fn print_issue(issue: &DependencyIssue) {
    println!(
        "\n   {} {}@{}",
        style("Package:").dim(),
        style(&issue.package).bold(),
        style(&issue.version).dim()
    );
    println!("   {} {}", style("Issue:").dim(), issue.issue);
    if let Some((min_kb, max_kb)) = issue.size_impact_kb {
        println!(
            "   {} {}-{} KB ({}% of bundle)",
            style("Impact:").dim(),
            style(min_kb).red(),
            style(max_kb).red(),
            issue.savings_percent.unwrap_or(0)
        );
    }
    println!(
        "   {} {}",
        style("Fix:").dim(),
        style(&issue.suggestion).green()
    );
}
