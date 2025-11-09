//! Build command implementation
//!
//! Thin presentation layer for the build command.
//! Business logic lives in `workflow::BuildWorkflow`.

use anyhow::Result;
use console::style;
use std::env;

use crate::cmd::workflow::BuildWorkflow;
use crate::fmt::{format_bytes, CHECKMARK, ROCKET};

/// Main build command handler (presentation layer)
///
/// Delegates to BuildWorkflow for orchestration and focuses on
/// formatting and displaying results to the user.
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::cmd::build::cmd_build;
///
/// // Build with default settings
/// cmd_build(false, false, false, None)?;
///
/// // Dry-run to preview changes
/// cmd_build(true, false, false, None)?;
///
/// // Build with JSON output for CI/CD
/// cmd_build(false, false, true, None)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn cmd_build(
    dry_run: bool,
    check: bool,
    json_output: bool,
    target_dir: Option<&str>,
) -> Result<()> {
    println!("{} {} Build Pipeline", ROCKET, style("wasm-slim").bold());
    println!();

    let project_root = env::current_dir()?;
    let workflow = BuildWorkflow::new(&project_root);

    // Execute workflow
    let result = workflow.execute(dry_run, check, target_dir)?;

    // Present results
    if result.dry_run {
        present_dry_run_info(&result.dry_run_files);
    } else {
        present_cargo_changes(&result.cargo_changes);
    }
    present_build_results(&result.metrics);
    present_budget_check(result.budget_check_passed, result.budget_threshold);

    // JSON output for CI/CD
    if json_output {
        present_json_report(&result.metrics)?;
    }

    Ok(())
}

/// Present dry-run information
fn present_dry_run_info(files: &[String]) {
    if !files.is_empty() {
        println!();
        println!("[DRY RUN] Would optimize {} file(s):", files.len());
        for file in files {
            println!("   {} Would optimize: {}", style("→").dim(), file);
        }
        println!();
    }
}

/// Present Cargo.toml optimization results
fn present_cargo_changes(changes: &[String]) {
    if !changes.is_empty() {
        println!();
        println!("{} Applied {} optimizations:", CHECKMARK, changes.len());
        for change in changes {
            println!("   {} {}", style("•").dim(), change);
        }
        println!();
    }
}

/// Present build completion and size metrics
fn present_build_results(metrics: &crate::pipeline::SizeMetrics) {
    println!();
    println!("{} Build completed successfully!", CHECKMARK);
    println!(
        "   Final size: {}",
        style(format_bytes(metrics.after_bytes)).green().bold()
    );

    let reduction = metrics.before_bytes.saturating_sub(metrics.after_bytes);
    let percent = (reduction as f64 / metrics.before_bytes as f64) * 100.0;
    println!(
        "   Reduction: {} ({:.1}%)",
        style(format_bytes(reduction)).green(),
        percent
    );
    println!();
}

/// Present budget check results
fn present_budget_check(passed: Option<bool>, threshold: Option<u64>) {
    if let (Some(passed), Some(threshold)) = (passed, threshold) {
        if passed {
            println!(
                "   {} Size within threshold ({})",
                CHECKMARK,
                format_bytes(threshold)
            );
        }
    }
}

/// Present JSON report for CI/CD systems
fn present_json_report(metrics: &crate::pipeline::SizeMetrics) -> Result<()> {
    let reduction = metrics.before_bytes.saturating_sub(metrics.after_bytes);
    let percent = (reduction as f64 / metrics.before_bytes as f64) * 100.0;

    let report = serde_json::json!({
        "final_size": metrics.after_bytes,
        "original_size": metrics.before_bytes,
        "reduction_bytes": reduction,
        "reduction_percent": percent,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });

    println!();
    println!("{}", serde_json::to_string_pretty(&report)?);
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::pipeline::SizeMetrics;

    #[test]
    fn test_present_dry_run_info_with_empty_list() {
        // Should not panic with empty list
        present_dry_run_info(&[]);
    }

    #[test]
    fn test_present_dry_run_info_with_files() {
        let files = vec!["Cargo.toml".to_string(), "src/lib/Cargo.toml".to_string()];
        // Should not panic with files
        present_dry_run_info(&files);
    }

    #[test]
    fn test_present_cargo_changes_with_empty_list() {
        present_cargo_changes(&[]);
    }

    #[test]
    fn test_present_cargo_changes_with_changes() {
        let changes = vec![
            "Added opt-level = 'z'".to_string(),
            "Added lto = true".to_string(),
        ];
        present_cargo_changes(&changes);
    }

    #[test]
    fn test_present_build_results_displays_metrics() {
        let metrics = SizeMetrics {
            before_bytes: 2000,
            after_bytes: 1000,
        };
        present_build_results(&metrics);
    }

    #[test]
    fn test_present_build_results_with_zero_reduction() {
        let metrics = SizeMetrics {
            before_bytes: 1000,
            after_bytes: 1000,
        };
        present_build_results(&metrics);
    }

    #[test]
    fn test_present_budget_check_with_passed() {
        present_budget_check(Some(true), Some(2000));
    }

    #[test]
    fn test_present_budget_check_with_none() {
        present_budget_check(None, None);
    }

    #[test]
    fn test_present_json_report_generates_valid_json() {
        let metrics = SizeMetrics {
            before_bytes: 2000,
            after_bytes: 1500,
        };
        let result = present_json_report(&metrics);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_bytes_via_shared_module() {
        use crate::fmt::format_bytes;
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(2_621_440), "2.50 MB");
    }
}
