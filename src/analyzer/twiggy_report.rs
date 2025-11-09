//! Twiggy analysis report formatting
//!
//! Provides console output formatters for twiggy analysis results.

use crate::analyzer::twiggy::{AnalysisResults, ComparisonResults};
use console::style;

/// Print twiggy analysis report to console
pub fn print_analysis_report(results: &AnalysisResults) {
    println!();
    println!("{}", style("WASM Size Analysis").bold().underlined());
    println!();

    // Summary
    let total_mb = results.total_size_bytes as f64 / (1024.0 * 1024.0);
    println!(
        "📦 {} {:.2} MB ({} bytes)",
        style("Total Size:").bold(),
        total_mb,
        format_number(results.total_size_bytes)
    );
    println!("🔍 {} {}", style("Analysis Mode:").bold(), results.mode);
    println!();

    // Top contributors
    if !results.items.is_empty() {
        println!(
            "{}",
            style(format!("TOP CONTRIBUTORS ({} items):", results.items.len())).bold()
        );
        println!("{}", style("─".repeat(70)).dim());

        let display_count = results.items.len().min(20);
        for (i, item) in results.items.iter().take(display_count).enumerate() {
            let size_kb = item.size_bytes / 1024;
            let size_str = if size_kb >= 1024 {
                format!("{:.2} MB", size_kb as f64 / 1024.0)
            } else {
                format!("{} KB", size_kb)
            };

            println!(
                "  {:2}. {:>10} ({:>5.1}%)  {}",
                i + 1,
                style(size_str).cyan().bold(),
                item.percentage,
                style(&item.name).dim()
            );
        }

        if results.items.len() > display_count {
            println!(
                "\n      {} {} more items...",
                style("...").dim(),
                results.items.len() - display_count
            );
        }

        println!();
    }

    // Monomorphization groups (for monos mode)
    if let Some(ref mono_groups) = results.mono_groups {
        if !mono_groups.is_empty() {
            println!("{}", style("MONOMORPHIZATION ANALYSIS:").bold().cyan());
            println!("{}", style("─".repeat(70)).dim());
            println!();

            // Summary
            let total_groups = mono_groups.len();
            let total_instantiations: usize =
                mono_groups.iter().map(|g| g.instantiation_count).sum();
            let total_savings: u64 = mono_groups.iter().map(|g| g.potential_savings_bytes).sum();
            let total_savings_kb = total_savings / 1024;
            let savings_percent = (total_savings as f64 / results.total_size_bytes as f64) * 100.0;

            println!(
                "  {} {} generic functions with {} total instantiations",
                style("Found:").bold(),
                style(total_groups).cyan().bold(),
                style(total_instantiations).cyan().bold()
            );
            println!(
                "  {} ~{} KB ({:.1}% of bundle)",
                style("Potential savings:").bold(),
                style(format_number(total_savings_kb)).green().bold(),
                savings_percent
            );
            println!();

            // Top groups
            let display_count = mono_groups.len().min(10);
            println!(
                "{}",
                style(format!("TOP {} GENERIC FUNCTIONS:", display_count)).bold()
            );
            println!("{}", style("─".repeat(70)).dim());

            for (i, group) in mono_groups.iter().take(display_count).enumerate() {
                let size_kb = group.total_size_bytes / 1024;
                let savings_kb = group.potential_savings_bytes / 1024;

                println!();
                println!(
                    "  {:2}. {}",
                    i + 1,
                    style(&group.function_name).yellow().bold()
                );
                println!(
                    "      {} {} instantiations, {} KB total (avg {} KB each)",
                    style("→").dim(),
                    style(group.instantiation_count).cyan(),
                    style(format_number(size_kb)).cyan(),
                    style(format_number(group.avg_size_bytes / 1024)).dim()
                );
                println!(
                    "      {} ~{} KB using trait objects (Box<dyn Trait>)",
                    style("→ Savings:").dim(),
                    style(format_number(savings_kb)).green().bold()
                );
            }

            if mono_groups.len() > display_count {
                println!();
                println!(
                    "      {} {} more generic functions...",
                    style("...").dim(),
                    mono_groups.len() - display_count
                );
            }

            println!();
            println!("{}", style("─".repeat(70)).dim());
            println!();
        }
    }

    // Recommendations
    if !results.recommendations.is_empty() {
        println!("{}", style("RECOMMENDATIONS:").bold().yellow());
        println!("{}", style("─".repeat(70)).dim());
        println!();

        for rec in &results.recommendations {
            let priority_color = match rec.priority.as_str() {
                "P0" => console::Color::Red,
                "P1" => console::Color::Yellow,
                "P2" => console::Color::Blue,
                _ => console::Color::White,
            };

            println!(
                "  {} {}",
                style(format!("[{}]", rec.priority))
                    .fg(priority_color)
                    .bold(),
                rec.description
            );

            if rec.estimated_savings_kb > 0 {
                println!(
                    "      {} ~{} KB ({:.1}%)",
                    style("→ Potential savings:").dim(),
                    format_number(rec.estimated_savings_kb),
                    rec.estimated_savings_percent
                );
            }

            println!();
        }

        // Calculate total potential savings
        let total_savings_kb: u64 = results
            .recommendations
            .iter()
            .map(|r| r.estimated_savings_kb)
            .sum();

        if total_savings_kb > 0 {
            let total_savings_percent: f64 = results
                .recommendations
                .iter()
                .map(|r| r.estimated_savings_percent)
                .sum();

            println!("{}", style("─".repeat(70)).dim());
            println!(
                "  {} ~{} KB ({:.1}%)",
                style("Total optimization potential:").bold(),
                format_number(total_savings_kb),
                total_savings_percent
            );
            println!();
        }
    } else {
        println!(
            "{}",
            style("✨ No major optimization opportunities detected. Bundle is well-optimized!")
                .green()
        );
        println!();
    }

    // Footer
    println!(
        "{}",
        style("💡 Tip: Run 'wasm-slim analyze assets' to check for embedded assets").dim()
    );
    println!();
}

/// Print comparison report to console
pub fn print_comparison_report(results: &ComparisonResults) {
    println!();
    println!("{}", style("WASM Build Comparison").bold().underlined());
    println!();

    // Size comparison
    let before_mb = results.before_size_bytes as f64 / (1024.0 * 1024.0);
    let after_mb = results.after_size_bytes as f64 / (1024.0 * 1024.0);

    println!("📊 {} {:.2} MB", style("Before:").bold(), before_mb);
    println!("📊 {} {:.2} MB", style("After:").bold(), after_mb);
    println!();

    // Delta
    let delta_color = if results.delta_bytes < 0 {
        console::Color::Green
    } else {
        console::Color::Red
    };

    let delta_symbol = if results.delta_bytes < 0 { "-" } else { "+" };
    let delta_kb = results.delta_bytes.abs() / 1024;

    println!(
        "{}  {} {} KB ({}{:.1}%)",
        if results.delta_bytes < 0 {
            "📉"
        } else {
            "📈"
        },
        style("Delta:").bold(),
        style(format!(
            "{}{}",
            delta_symbol,
            format_number(delta_kb as u64)
        ))
        .fg(delta_color)
        .bold(),
        delta_symbol,
        results.delta_percent.abs()
    );

    println!();

    // Top changes
    if !results.top_changes.is_empty() {
        println!("{}", style("TOP CHANGES:").bold());
        println!("{}", style("─".repeat(70)).dim());
        println!();

        let display_count = results.top_changes.len().min(15);
        for change in results.top_changes.iter().take(display_count) {
            let delta_symbol = if change.delta_bytes < 0 { "-" } else { "+" };
            let delta_kb = change.delta_bytes.abs() / 1024;
            let delta_color = if change.delta_bytes < 0 {
                console::Color::Green
            } else {
                console::Color::Red
            };

            println!(
                "  {}  {:>8} KB  {}",
                style(delta_symbol).fg(delta_color),
                style(format_number(delta_kb as u64)).fg(delta_color).bold(),
                style(&change.name).dim()
            );
        }

        if results.top_changes.len() > display_count {
            println!(
                "\n      {} {} more changes...",
                style("...").dim(),
                results.top_changes.len() - display_count
            );
        }
    }

    println!();
}

/// Format number with comma separators
fn format_number(n: u64) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or("?"))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::twiggy::analysis_types::{
        AnalysisItem, AnalysisResults, MonomorphizationGroup,
    };
    use crate::analyzer::twiggy::comparison::{ChangeItem, ComparisonResults};

    #[test]
    fn test_format_number_adds_thousand_separators() {
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_format_number_edge_cases() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1), "1");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
    }

    #[test]
    fn test_print_analysis_report_basic() {
        let results = AnalysisResults {
            total_size_bytes: 1_048_576, // 1 MB
            mode: "top".to_string(),
            items: vec![
                AnalysisItem {
                    name: "function_a".to_string(),
                    size_bytes: 524_288,
                    percentage: 50.0,
                },
                AnalysisItem {
                    name: "function_b".to_string(),
                    size_bytes: 262_144,
                    percentage: 25.0,
                },
            ],
            mono_groups: None,
            recommendations: vec![],
        };

        // Should not panic
        print_analysis_report(&results);
    }

    #[test]
    fn test_print_analysis_report_empty_items() {
        let results = AnalysisResults {
            total_size_bytes: 1_048_576,
            mode: "top".to_string(),
            items: vec![],
            mono_groups: None,
            recommendations: vec![],
        };

        print_analysis_report(&results);
    }

    #[test]
    fn test_print_analysis_report_with_mono_groups() {
        let results = AnalysisResults {
            total_size_bytes: 2_097_152, // 2 MB
            mode: "monos".to_string(),
            items: vec![],
            mono_groups: Some(vec![
                MonomorphizationGroup {
                    function_name: "Vec::push".to_string(),
                    instantiation_count: 5,
                    total_size_bytes: 10_240,
                    avg_size_bytes: 2_048,
                    potential_savings_bytes: 8_192,
                    instantiations: vec![],
                },
                MonomorphizationGroup {
                    function_name: "Option::unwrap".to_string(),
                    instantiation_count: 3,
                    total_size_bytes: 6_144,
                    avg_size_bytes: 2_048,
                    potential_savings_bytes: 4_096,
                    instantiations: vec![],
                },
            ]),
            recommendations: vec![],
        };

        print_analysis_report(&results);
    }

    #[test]
    fn test_print_analysis_report_large_item_count() {
        let mut items = Vec::new();
        for i in 0..30 {
            items.push(AnalysisItem {
                name: format!("function_{}", i),
                size_bytes: 1000 * (30 - i),
                percentage: 1.0,
            });
        }

        let results = AnalysisResults {
            total_size_bytes: 100_000,
            mode: "top".to_string(),
            items,
            mono_groups: None,
            recommendations: vec![],
        };

        // Should show "... X more items" message
        print_analysis_report(&results);
    }

    #[test]
    fn test_print_analysis_report_dominators_mode() {
        let results = AnalysisResults {
            total_size_bytes: 500_000,
            mode: "dominators".to_string(),
            items: vec![AnalysisItem {
                name: "dominator_function".to_string(),
                size_bytes: 250_000,
                percentage: 50.0,
            }],
            mono_groups: None,
            recommendations: vec![],
        };

        print_analysis_report(&results);
    }

    #[test]
    fn test_print_analysis_report_garbage_mode() {
        let results = AnalysisResults {
            total_size_bytes: 300_000,
            mode: "garbage".to_string(),
            items: vec![],
            mono_groups: None,
            recommendations: vec![],
        };

        print_analysis_report(&results);
    }

    #[test]
    fn test_print_comparison_report_size_reduction() {
        let results = ComparisonResults {
            before_size_bytes: 2_097_152, // 2 MB
            after_size_bytes: 1_048_576,  // 1 MB
            delta_bytes: -1_048_576,
            delta_percent: -50.0,
            top_changes: vec![
                ChangeItem {
                    delta_bytes: -524_288,
                    name: "function_removed".to_string(),
                },
                ChangeItem {
                    delta_bytes: -262_144,
                    name: "function_optimized".to_string(),
                },
            ],
        };

        print_comparison_report(&results);
    }

    #[test]
    fn test_print_comparison_report_size_increase() {
        let results = ComparisonResults {
            before_size_bytes: 1_048_576,
            after_size_bytes: 2_097_152,
            delta_bytes: 1_048_576,
            delta_percent: 100.0,
            top_changes: vec![ChangeItem {
                delta_bytes: 524_288,
                name: "function_added".to_string(),
            }],
        };

        print_comparison_report(&results);
    }

    #[test]
    fn test_print_comparison_report_no_changes() {
        let results = ComparisonResults {
            before_size_bytes: 1_048_576,
            after_size_bytes: 1_048_576,
            delta_bytes: 0,
            delta_percent: 0.0,
            top_changes: vec![],
        };

        print_comparison_report(&results);
    }

    #[test]
    fn test_print_comparison_report_many_changes() {
        let mut changes = Vec::new();
        for i in 0..20 {
            changes.push(ChangeItem {
                delta_bytes: (i as i64 - 10) * 1000,
                name: format!("change_{}", i),
            });
        }

        let results = ComparisonResults {
            before_size_bytes: 1_000_000,
            after_size_bytes: 1_100_000,
            delta_bytes: 100_000,
            delta_percent: 10.0,
            top_changes: changes,
        };

        // Should show "... X more changes" message
        print_comparison_report(&results);
    }

    #[test]
    fn test_print_comparison_report_small_delta() {
        let results = ComparisonResults {
            before_size_bytes: 1_000_000,
            after_size_bytes: 999_000,
            delta_bytes: -1_000,
            delta_percent: -0.1,
            top_changes: vec![],
        };

        print_comparison_report(&results);
    }

    #[test]
    fn test_print_analysis_report_many_mono_groups() {
        let mut groups = Vec::new();
        for i in 0..15 {
            groups.push(MonomorphizationGroup {
                function_name: format!("generic_function_{}", i),
                instantiation_count: i + 2,
                total_size_bytes: ((i + 1) * 1024) as u64,
                avg_size_bytes: 512,
                potential_savings_bytes: (i * 512) as u64,
                instantiations: vec![],
            });
        }

        let results = AnalysisResults {
            total_size_bytes: 500_000,
            mode: "monos".to_string(),
            items: vec![],
            mono_groups: Some(groups),
            recommendations: vec![],
        };

        print_analysis_report(&results);
    }

    #[test]
    fn test_print_analysis_report_large_file_sizes() {
        let results = AnalysisResults {
            total_size_bytes: 10_485_760, // 10 MB
            mode: "top".to_string(),
            items: vec![AnalysisItem {
                name: "large_function".to_string(),
                size_bytes: 5_242_880, // 5 MB
                percentage: 50.0,
            }],
            mono_groups: None,
            recommendations: vec![],
        };

        print_analysis_report(&results);
    }

    #[test]
    fn test_print_analysis_report_small_file_sizes() {
        let results = AnalysisResults {
            total_size_bytes: 10_240, // 10 KB
            mode: "top".to_string(),
            items: vec![AnalysisItem {
                name: "small_function".to_string(),
                size_bytes: 2_048,
                percentage: 20.0,
            }],
            mono_groups: None,
            recommendations: vec![],
        };

        print_analysis_report(&results);
    }
}
