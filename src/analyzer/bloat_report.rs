//! Bloat analysis report formatting

use super::bloat::BloatResults;
use super::report_utils;
use console::style;
use std::fmt::{self, Write as _};

/// Format bloat analysis results for console output
pub fn format_console_report(results: &BloatResults) -> Result<String, fmt::Error> {
    let mut output = String::new();

    // Header
    writeln!(output, "\n{} Binary Size Analysis", style("📊").bold())?;
    writeln!(
        output,
        "   Total Size: {}",
        style(format_bytes(results.total_size_bytes)).cyan()
    )?;
    writeln!(
        output,
        "   Code (.text): {}\n",
        style(format_bytes(results.text_size_bytes)).cyan()
    )?;

    // Top contributors
    writeln!(output, "{} Top Contributors by Size", style("🔍").bold())?;
    writeln!(output, "   {:<12} {:<8} Symbol", "Size", "Percent")?;
    output.push_str("   ─────────────────────────────────────────────────────────\n");

    for (i, item) in results.items.iter().take(20).enumerate() {
        let size_str = format_bytes(item.size_bytes);
        let percent_str = format!("{:.1}%", item.percentage);
        let name = truncate_symbol(&item.name, 60);

        let color = match i {
            0..=4 => style(size_str).red(),
            5..=9 => style(size_str).yellow(),
            _ => style(size_str).dim(),
        };

        writeln!(
            output,
            "   {:<12} {:<8} {}",
            color,
            percent_str,
            style(name).dim()
        )?;
    }

    // Recommendations
    if !results.recommendations.is_empty() {
        writeln!(
            output,
            "\n{} Optimization Opportunities",
            style("💡").bold()
        )?;

        for rec in &results.recommendations {
            let priority_str = match rec.priority.as_str() {
                "P0" => style(&rec.priority).red().bold(),
                "P1" => style(&rec.priority).yellow().bold(),
                "P2" => style(&rec.priority).blue(),
                _ => style(&rec.priority).dim(),
            };

            writeln!(
                output,
                "   {} {} (save ~{}KB / {:.1}%)",
                priority_str,
                rec.description,
                rec.estimated_savings_kb,
                rec.estimated_savings_percent
            )?;
        }
    }

    output.push('\n');
    Ok(output)
}

/// Format bloat results as JSON
pub fn format_json_report(results: &BloatResults) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(results)
}

// Use shared formatting utilities from report_utils
fn format_bytes(bytes: u64) -> String {
    report_utils::format_bytes(bytes)
}

fn truncate_symbol(symbol: &str, max_len: usize) -> String {
    report_utils::truncate_str(symbol, max_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_converts_to_readable_units() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1572864), "1.50 MB");
    }

    #[test]
    fn test_truncate_symbol_with_long_name_adds_ellipsis() {
        assert_eq!(truncate_symbol("short", 10), "short");
        assert_eq!(
            truncate_symbol("very_long_symbol_name_that_exceeds_limit", 20),
            "very_long_symbol_..."
        );
    }

    #[test]
    fn test_format_json_report_serializes_bloat_results() {
        let results = BloatResults {
            total_size_bytes: 1024000,
            text_size_bytes: 512000,
            items: vec![],
            recommendations: vec![],
        };
        let json = format_json_report(&results).expect("Failed to serialize bloat results to JSON");
        assert!(json.contains("total_size_bytes"));
    }
}
