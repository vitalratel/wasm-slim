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

    #[test]
    fn test_format_number_adds_thousand_separators() {
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
    }
}
