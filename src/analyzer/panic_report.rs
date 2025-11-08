//! Panic analysis report formatting
//!
//! Provides console output formatters for panic detection results.

use crate::analyzer::panics::PanicResults;
use console::style;
use std::collections::HashMap;

/// Print panic analysis report to console
pub fn print_panic_report(results: &PanicResults) {
    println!();
    println!("{}", style("Panic Pattern Analysis").bold().underlined());
    println!();

    // Summary
    println!(
        "🔍 {} {} panic sites detected",
        style("Total:").bold(),
        style(results.total_panics).cyan().bold()
    );
    println!(
        "📦 {} ~{} KB (estimated impact on WASM size)",
        style("Size Impact:").bold(),
        style(results.estimated_size_kb).yellow().bold()
    );
    println!();

    // Breakdown by pattern
    if !results.by_pattern.is_empty() {
        println!("{}", style("BREAKDOWN BY PATTERN:").bold());
        println!("{}", style("─".repeat(70)).dim());
        println!();

        for (pattern, count) in &results.by_pattern {
            let size_kb = (pattern.size_per_occurrence() * (*count as u64)) / 1024;

            println!(
                "  {} {:>4} occurrences (~{} KB)",
                style(format!("{:20}", pattern.name())).cyan(),
                style(count).bold(),
                style(size_kb).yellow()
            );
            println!(
                "       {} Use {}",
                style("→").dim(),
                style(pattern.alternative()).green()
            );
            println!();
        }

        println!("{}", style("─".repeat(70)).dim());
        println!();
    }

    // Top files with most panics
    let files_with_panics = group_by_file(&results.panic_sites);
    if !files_with_panics.is_empty() {
        println!("{}", style("TOP FILES WITH PANICS:").bold());
        println!("{}", style("─".repeat(70)).dim());

        let mut sorted_files: Vec<_> = files_with_panics.iter().collect();
        sorted_files.sort_by(|a, b| b.1.cmp(a.1));

        for (file, count) in sorted_files.iter().take(10) {
            println!(
                "  {:>4}  {}",
                style(count).yellow().bold(),
                style(file.display()).dim()
            );
        }

        if files_with_panics.len() > 10 {
            println!(
                "\n      {} {} more files...",
                style("...").dim(),
                files_with_panics.len() - 10
            );
        }

        println!();
        println!("{}", style("─".repeat(70)).dim());
        println!();
    }

    // Recommendations
    if !results.recommendations.is_empty() {
        println!("{}", style("RECOMMENDATIONS:").bold().yellow());
        println!("{}", style("─".repeat(70)).dim());
        println!();

        for rec in &results.recommendations {
            if rec.trim().is_empty() {
                println!();
                continue;
            }

            // Color-code by priority
            if rec.starts_with("[P0]") {
                println!("  {}", style(rec).red().bold());
            } else if rec.starts_with("[P1]") {
                println!("  {}", style(rec).yellow().bold());
            } else if rec.starts_with("[P2]") {
                println!("  {}", style(rec).blue());
            } else if rec.starts_with("[P3]") {
                println!("  {}", style(rec).green());
            } else {
                println!("  {}", style(rec).dim());
            }
        }

        println!();
        println!("{}", style("─".repeat(70)).dim());
        println!();
    } else {
        println!(
            "{}",
            style("✨ No major panic issues detected. Code is well-optimized!").green()
        );
        println!();
    }

    // Footer
    println!(
        "{}",
        style("💡 Tip: Use 'wasm-slim analyze --mode monos' to check for generic bloat").dim()
    );
    println!();
}

/// Print panic analysis report as JSON
pub fn print_json_report(results: &PanicResults) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{}", json);
    Ok(())
}

/// Group panic sites by file
fn group_by_file(
    panics: &[crate::analyzer::panics::DetectedPanic],
) -> HashMap<std::path::PathBuf, usize> {
    let mut map = HashMap::new();

    for panic in panics {
        *map.entry(panic.file.clone()).or_insert(0) += 1;
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::panics::{DetectedPanic, PanicPattern};
    use std::path::PathBuf;

    #[test]
    fn test_group_by_file_counts_correctly() {
        let panics = vec![
            DetectedPanic {
                file: PathBuf::from("src/main.rs"),
                line: 10,
                pattern: PanicPattern::Unwrap,
                snippet: None,
            },
            DetectedPanic {
                file: PathBuf::from("src/main.rs"),
                line: 20,
                pattern: PanicPattern::Expect,
                snippet: None,
            },
            DetectedPanic {
                file: PathBuf::from("src/lib.rs"),
                line: 5,
                pattern: PanicPattern::Index,
                snippet: None,
            },
        ];

        let grouped = group_by_file(&panics);

        assert_eq!(grouped.get(&PathBuf::from("src/main.rs")), Some(&2));
        assert_eq!(grouped.get(&PathBuf::from("src/lib.rs")), Some(&1));
    }
}
