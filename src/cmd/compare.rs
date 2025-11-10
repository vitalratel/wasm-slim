//! Compare command implementation
//!
//! Handles the `wasm-slim compare` command which compares two WASM builds
//! to show size differences and optimization impact

use anyhow::Result;
use console::style;
use std::path::Path;

use crate::analyzer;

/// Compare two WASM builds to show optimization impact
///
/// Uses twiggy to analyze both files and show:
/// - Size differences
/// - Symbol-level changes
/// - Optimization effectiveness
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::cmd::compare::cmd_compare;
///
/// // Compare baseline WASM with optimized version
/// cmd_compare("baseline.wasm", "optimized.wasm")?;
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Either file doesn't exist
/// - twiggy is not installed
/// - Files are not valid WASM binaries
pub fn cmd_compare(before: &str, after: &str) -> Result<()> {
    cmd_compare_impl(before, after, true)
}

/// Internal implementation that allows skipping twiggy check for testing
fn cmd_compare_impl(before: &str, after: &str, check_twiggy: bool) -> Result<()> {
    let before_path = Path::new(before);
    let after_path = Path::new(after);

    // Verify files exist first (before checking for twiggy)
    if !before_path.exists() {
        anyhow::bail!(
            "Baseline file not found: {}. Run a build first to create a baseline.",
            before
        );
    }
    if !after_path.exists() {
        anyhow::bail!("Comparison file not found: {}", after);
    }

    // Check if twiggy is installed (unless we're in test mode)
    if check_twiggy && !analyzer::TwiggyAnalyzer::check_installation()? {
        eprintln!(
            "{}",
            style(analyzer::TwiggyAnalyzer::installation_instructions()).yellow()
        );
        anyhow::bail!("twiggy not installed");
    }

    println!("📊 {} WASM Build Comparison", style("wasm-slim").bold());
    println!();

    // Run comparison
    use crate::infra::{RealCommandExecutor, RealFileSystem};
    let results = analyzer::TwiggyAnalyzer::compare(
        before_path,
        after_path,
        &RealFileSystem,
        &RealCommandExecutor,
    )?;

    // Print report
    analyzer::print_comparison_report(&results);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_compare_with_missing_baseline_file() {
        let temp_dir = TempDir::new().unwrap();
        let baseline = temp_dir.path().join("nonexistent_baseline.wasm");
        let after = temp_dir.path().join("after.wasm");

        // Create after file but not baseline
        fs::write(&after, b"dummy wasm content").unwrap();

        let result = cmd_compare_impl(baseline.to_str().unwrap(), after.to_str().unwrap(), false);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Baseline file not found"));
        assert!(err_msg.contains("nonexistent_baseline.wasm"));
    }

    #[test]
    fn test_compare_with_missing_comparison_file() {
        let temp_dir = TempDir::new().unwrap();
        let baseline = temp_dir.path().join("baseline.wasm");
        let after = temp_dir.path().join("nonexistent_after.wasm");

        // Create baseline file but not comparison
        fs::write(&baseline, b"dummy wasm content").unwrap();

        let result = cmd_compare_impl(baseline.to_str().unwrap(), after.to_str().unwrap(), false);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Comparison file not found"));
        assert!(err_msg.contains("nonexistent_after.wasm"));
    }

    #[test]
    fn test_compare_with_both_files_missing() {
        let temp_dir = TempDir::new().unwrap();
        let baseline = temp_dir.path().join("nonexistent1.wasm");
        let after = temp_dir.path().join("nonexistent2.wasm");

        let result = cmd_compare_impl(baseline.to_str().unwrap(), after.to_str().unwrap(), false);

        assert!(result.is_err());
        // Should fail on baseline first
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Baseline file not found"));
    }

    #[test]
    fn test_compare_validates_baseline_before_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let baseline = temp_dir.path().join("missing_baseline.wasm");
        let after = temp_dir.path().join("missing_after.wasm");

        // Neither file exists, but baseline should be checked first
        let result = cmd_compare_impl(baseline.to_str().unwrap(), after.to_str().unwrap(), false);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Should mention baseline, not comparison
        assert!(err_msg.contains("Baseline"));
        assert!(!err_msg.contains("Comparison file not found"));
    }

    #[test]
    fn test_compare_error_message_includes_file_path() {
        let baseline_path = "/some/path/to/baseline.wasm";
        let after_path = "/some/path/to/after.wasm";

        let result = cmd_compare_impl(baseline_path, after_path, false);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains(baseline_path));
    }
}
