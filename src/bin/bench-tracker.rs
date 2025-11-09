//! Benchmark tracking tool for wasm-slim
//!
//! This tool runs criterion benchmarks and tracks performance over time,
//! detecting regressions and managing baselines.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::Command;
use wasm_slim::bench_tracker::{BenchmarkTracker, PerformanceBudget};

#[derive(Parser)]
#[command(name = "bench-tracker")]
#[command(about = "Performance tracking tool for wasm-slim benchmarks")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to project root directory (defaults to current directory)
    ///
    /// This should be the directory containing Cargo.toml, not the file itself.
    /// Symlinks are resolved to their target paths.
    #[arg(short, long, default_value = ".")]
    project_root: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Run benchmarks and compare with baseline
    Run {
        /// Maximum allowed regression percentage (default: 10%)
        #[arg(long, default_value = "10.0")]
        max_regression: f64,

        /// Fail if regressions are detected
        #[arg(long)]
        fail_on_regression: bool,

        /// Specific benchmark to run (runs all if not specified)
        #[arg(short, long)]
        bench: Option<String>,
    },

    /// Save current benchmark results as baseline
    Baseline {
        /// Version/tag for this baseline
        #[arg(short, long, default_value = "current")]
        version: String,
    },

    /// Show comparison with baseline without running benchmarks
    Compare,

    /// Show current baseline information
    Show,

    /// Reset/delete current baseline
    Reset,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let project_root = cli
        .project_root
        .canonicalize()
        .context("Failed to resolve project root path")?;

    // Validate that project_root is a directory
    if !project_root.is_dir() {
        anyhow::bail!(
            "Project root must be a directory, got: {}",
            project_root.display()
        );
    }

    match cli.command {
        Commands::Run {
            max_regression,
            fail_on_regression,
            bench,
        } => {
            run_benchmarks(&project_root, max_regression, fail_on_regression, bench)?;
        }
        Commands::Baseline { version } => {
            save_baseline(&project_root, version)?;
        }
        Commands::Compare => {
            compare_with_baseline(&project_root)?;
        }
        Commands::Show => {
            show_baseline(&project_root)?;
        }
        Commands::Reset => {
            reset_baseline(&project_root)?;
        }
    }

    Ok(())
}

/// Run benchmarks and compare with baseline
fn run_benchmarks(
    project_root: &Path,
    max_regression: f64,
    fail_on_regression: bool,
    bench_name: Option<String>,
) -> Result<()> {
    println!("🔧 Running benchmarks...");

    // Build the cargo bench command
    let mut cmd = Command::new("cargo");
    cmd.arg("bench")
        .arg("--message-format=json")
        .current_dir(project_root);

    if let Some(name) = bench_name {
        cmd.arg("--bench").arg(name);
    }

    // Run benchmarks
    let output = cmd.output().context("Failed to run cargo bench")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Benchmark execution failed:\n{}", stderr);
        anyhow::bail!("Benchmark execution failed");
    }

    println!("✓ Benchmarks completed");

    // Parse results and compare
    let criterion_dir = project_root.join("target").join("criterion");
    if !criterion_dir.exists() {
        println!("⚠️  No criterion results found. Make sure benchmarks use criterion.");
        return Ok(());
    }

    let budget = PerformanceBudget {
        max_regression_percent: max_regression,
        max_time_ns: None,
        fail_on_violation: fail_on_regression,
    };

    let tracker = BenchmarkTracker::with_budget(project_root, budget);
    let current_results = tracker.parse_criterion_results(&criterion_dir)?;

    if current_results.is_empty() {
        println!("⚠️  No benchmark results parsed. Check criterion output.");
        return Ok(());
    }

    println!("✓ Parsed {} benchmark results", current_results.len());

    // Compare with baseline if it exists
    if let Some(baseline) = tracker.load_baseline()? {
        let comparisons = tracker.compare_with_baseline(&current_results, &baseline);
        tracker.print_comparison(&comparisons);

        if tracker.has_regressions(&comparisons) {
            println!("\n⚠️  Performance regressions detected!");
            if fail_on_regression {
                anyhow::bail!("Performance regressions exceed threshold");
            }
        } else {
            println!("\n✓ No significant performance regressions");
        }
    } else {
        println!("\n📝 No baseline found. Run 'bench-tracker baseline' to save current results.");
    }

    Ok(())
}

/// Save current benchmark results as baseline
fn save_baseline(project_root: &Path, version: String) -> Result<()> {
    println!("💾 Saving baseline...");

    let criterion_dir = project_root.join("target").join("criterion");
    if !criterion_dir.exists() {
        anyhow::bail!(
            "No criterion results found. Run benchmarks first with 'cargo bench' or 'bench-tracker run'"
        );
    }

    let tracker = BenchmarkTracker::new(project_root);
    let results = tracker.parse_criterion_results(&criterion_dir)?;

    if results.is_empty() {
        anyhow::bail!("No benchmark results found to save");
    }

    let baseline = tracker.create_baseline(results, version)?;
    tracker.save_baseline(baseline)?;

    println!("✓ Baseline saved successfully");
    Ok(())
}

/// Compare with baseline without running benchmarks
fn compare_with_baseline(project_root: &Path) -> Result<()> {
    let criterion_dir = project_root.join("target").join("criterion");
    if !criterion_dir.exists() {
        anyhow::bail!("No criterion results found. Run benchmarks first.");
    }

    let tracker = BenchmarkTracker::new(project_root);
    let current_results = tracker.parse_criterion_results(&criterion_dir)?;

    if let Some(baseline) = tracker.load_baseline()? {
        let comparisons = tracker.compare_with_baseline(&current_results, &baseline);
        tracker.print_comparison(&comparisons);

        if tracker.has_regressions(&comparisons) {
            println!("\n⚠️  Performance regressions detected!");
        } else {
            println!("\n✓ No significant performance regressions");
        }
    } else {
        println!("❌ No baseline found. Run 'bench-tracker baseline' first.");
    }

    Ok(())
}

/// Show current baseline information
fn show_baseline(project_root: &Path) -> Result<()> {
    let tracker = BenchmarkTracker::new(project_root);

    if let Some(baseline) = tracker.load_baseline()? {
        println!("📊 Current Baseline");
        println!("Version: {}", baseline.version);
        println!("Timestamp: {}", format_timestamp(baseline.timestamp));
        if let Some(commit) = &baseline.git_commit {
            println!("Git commit: {}", commit);
        }
        println!("\nBenchmarks ({}):", baseline.results.len());

        for (name, result) in &baseline.results {
            println!(
                "  • {} - {} (±{})",
                name,
                format_ns(result.mean_ns),
                format_ns(result.stddev_ns)
            );
        }
    } else {
        println!("❌ No baseline found.");
    }

    Ok(())
}

/// Reset/delete current baseline
fn reset_baseline(project_root: &Path) -> Result<()> {
    let baseline_path = project_root
        .join(".wasm-slim")
        .join("benchmarks")
        .join("baseline.json");

    if baseline_path.exists() {
        std::fs::remove_file(&baseline_path).context("Failed to remove baseline file")?;
        println!("✓ Baseline reset successfully");
    } else {
        println!("ℹ️  No baseline to reset");
    }

    Ok(())
}

/// Format timestamp as human-readable date
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let time = UNIX_EPOCH + Duration::from_secs(timestamp);
    match time.duration_since(SystemTime::now()) {
        Ok(_) => "future".to_string(),
        Err(elapsed) => {
            let elapsed = elapsed.duration();
            if elapsed.as_secs() < 60 {
                "just now".to_string()
            } else if elapsed.as_secs() < 3600 {
                format!("{} minutes ago", elapsed.as_secs() / 60)
            } else if elapsed.as_secs() < 86400 {
                format!("{} hours ago", elapsed.as_secs() / 3600)
            } else {
                format!("{} days ago", elapsed.as_secs() / 86400)
            }
        }
    }
}

/// Format nanoseconds as human-readable time
fn format_ns(ns: u64) -> String {
    if ns < 1_000 {
        format!("{} ns", ns)
    } else if ns < 1_000_000 {
        format!("{:.2} µs", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.2} ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.2} s", ns as f64 / 1_000_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_format_timestamp_future() {
        // Far future timestamp
        let future = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 86400; // 1 day in future
        assert_eq!(format_timestamp(future), "future");
    }

    #[test]
    fn test_format_timestamp_just_now() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let result = format_timestamp(now);
        assert_eq!(result, "just now");
    }

    #[test]
    fn test_format_timestamp_minutes() {
        let two_minutes_ago = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 120;
        let result = format_timestamp(two_minutes_ago);
        assert!(result.contains("minutes ago"));
    }

    #[test]
    fn test_format_timestamp_hours() {
        let two_hours_ago = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 7200;
        let result = format_timestamp(two_hours_ago);
        assert!(result.contains("hours ago"));
    }

    #[test]
    fn test_format_timestamp_days() {
        let two_days_ago = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 172800;
        let result = format_timestamp(two_days_ago);
        assert!(result.contains("days ago"));
    }

    #[test]
    fn test_format_ns_nanoseconds() {
        assert_eq!(format_ns(500), "500 ns");
        assert_eq!(format_ns(999), "999 ns");
    }

    #[test]
    fn test_format_ns_microseconds() {
        assert_eq!(format_ns(1_000), "1.00 µs");
        assert_eq!(format_ns(1_500), "1.50 µs");
        assert_eq!(format_ns(999_999), "1000.00 µs");
    }

    #[test]
    fn test_format_ns_milliseconds() {
        assert_eq!(format_ns(1_000_000), "1.00 ms");
        assert_eq!(format_ns(1_500_000), "1.50 ms");
        assert_eq!(format_ns(999_999_999), "1000.00 ms");
    }

    #[test]
    fn test_format_ns_seconds() {
        assert_eq!(format_ns(1_000_000_000), "1.00 s");
        assert_eq!(format_ns(2_500_000_000), "2.50 s");
    }

    #[test]
    fn test_format_ns_boundary_values() {
        assert_eq!(format_ns(0), "0 ns");
        assert_eq!(format_ns(1), "1 ns");
    }
}
