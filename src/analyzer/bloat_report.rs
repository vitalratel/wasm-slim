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

    use super::super::bloat::BloatItem;

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

    #[test]
    fn test_format_console_report_with_empty_results() {
        let results = BloatResults {
            total_size_bytes: 1024,
            text_size_bytes: 512,
            items: vec![],
            recommendations: vec![],
        };
        let output = format_console_report(&results);
        assert!(output.is_ok());
        let text = output.unwrap();
        assert!(text.contains("Binary Size Analysis"));
        assert!(text.contains("1.00 KB")); // total size
    }

    #[test]
    fn test_format_console_report_with_items() {
        use super::super::bloat::Recommendation;

        let results = BloatResults {
            total_size_bytes: 100000,
            text_size_bytes: 50000,
            items: vec![
                BloatItem {
                    name: "test_function".to_string(),
                    size_bytes: 5000,
                    percentage: 5.0,
                    crate_name: Some("test_crate".to_string()),
                },
                BloatItem {
                    name: "another_function".to_string(),
                    size_bytes: 3000,
                    percentage: 3.0,
                    crate_name: None,
                },
            ],
            recommendations: vec![Recommendation {
                priority: "P0".to_string(),
                description: "Optimize large function".to_string(),
                estimated_savings_kb: 10,
                estimated_savings_percent: 2.0,
            }],
        };

        let output = format_console_report(&results);
        assert!(output.is_ok());
        let text = output.unwrap();
        assert!(text.contains("Binary Size Analysis"));
        assert!(text.contains("Top Contributors"));
        assert!(text.contains("test_function"));
        assert!(text.contains("Optimization Opportunities"));
        assert!(text.contains("P0"));
    }

    #[test]
    fn test_format_console_report_with_many_items() {
        let mut items = vec![];
        for i in 0..30 {
            items.push(BloatItem {
                name: format!("function_{}", i),
                size_bytes: 1000 - (i * 10),
                percentage: (1000 - (i * 10)) as f64 / 10000.0,
                crate_name: Some("crate".to_string()),
            });
        }

        let results = BloatResults {
            total_size_bytes: 100000,
            text_size_bytes: 50000,
            items,
            recommendations: vec![],
        };

        let output = format_console_report(&results);
        assert!(output.is_ok());
        // Should only show top 20
        let text = output.unwrap();
        assert!(text.contains("function_0"));
        assert!(text.contains("function_19"));
    }

    #[test]
    fn test_format_console_report_with_all_priority_levels() {
        use super::super::bloat::Recommendation;

        let results = BloatResults {
            total_size_bytes: 100000,
            text_size_bytes: 50000,
            items: vec![],
            recommendations: vec![
                Recommendation {
                    priority: "P0".to_string(),
                    description: "Critical issue".to_string(),
                    estimated_savings_kb: 50,
                    estimated_savings_percent: 10.0,
                },
                Recommendation {
                    priority: "P1".to_string(),
                    description: "High priority".to_string(),
                    estimated_savings_kb: 20,
                    estimated_savings_percent: 5.0,
                },
                Recommendation {
                    priority: "P2".to_string(),
                    description: "Medium priority".to_string(),
                    estimated_savings_kb: 10,
                    estimated_savings_percent: 2.0,
                },
                Recommendation {
                    priority: "P3".to_string(),
                    description: "Low priority".to_string(),
                    estimated_savings_kb: 5,
                    estimated_savings_percent: 1.0,
                },
            ],
        };

        let output = format_console_report(&results);
        assert!(output.is_ok());
        let text = output.unwrap();
        assert!(text.contains("P0"));
        assert!(text.contains("P1"));
        assert!(text.contains("P2"));
        assert!(text.contains("P3"));
    }

    #[test]
    fn test_format_json_report_with_recommendations() {
        use super::super::bloat::Recommendation;

        let results = BloatResults {
            total_size_bytes: 100000,
            text_size_bytes: 50000,
            items: vec![BloatItem {
                name: "test".to_string(),
                size_bytes: 1000,
                percentage: 1.0,
                crate_name: None,
            }],
            recommendations: vec![Recommendation {
                priority: "P0".to_string(),
                description: "Test".to_string(),
                estimated_savings_kb: 10,
                estimated_savings_percent: 2.0,
            }],
        };

        let json = format_json_report(&results).unwrap();
        assert!(json.contains("total_size_bytes"));
        assert!(json.contains("text_size_bytes"));
        assert!(json.contains("items"));
        assert!(json.contains("recommendations"));
        assert!(json.contains("P0"));
    }
}
