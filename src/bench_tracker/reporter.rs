//! Benchmark comparison reporting and display

use super::comparator::BenchmarkComparison;

/// Handles benchmark comparison output
pub struct BenchmarkReporter;

impl BenchmarkReporter {
    /// Create a new benchmark reporter
    pub fn new() -> Self {
        Self
    }

    /// Print comparison results
    pub fn print_comparison(&self, comparisons: &[BenchmarkComparison]) {
        println!("\n📊 Performance Comparison");
        println!("{}", "=".repeat(80));
        println!(
            "{:<40} {:>12} {:>12} {:>12}",
            "Benchmark", "Baseline", "Current", "Change"
        );
        println!("{}", "-".repeat(80));

        for comp in comparisons {
            let status = if comp.is_regression {
                "🔴"
            } else if comp.change_percent < -5.0 {
                "🟢"
            } else {
                "⚪"
            };

            let change_str = format!("{:+.2}%", comp.change_percent);

            println!(
                "{} {:<37} {:>12} {:>12} {:>12}",
                status,
                truncate(&comp.name, 37),
                format_ns(comp.baseline_mean_ns),
                format_ns(comp.current_mean_ns),
                change_str
            );

            if comp.exceeds_budget {
                println!("   ⚠️  Exceeds time budget!");
            }
        }

        println!("{}", "=".repeat(80));
    }
}

impl Default for BenchmarkReporter {
    fn default() -> Self {
        Self::new()
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

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ns_converts_to_readable_time_units() {
        assert_eq!(format_ns(500), "500 ns");
        assert_eq!(format_ns(1_500), "1.50 µs");
        assert_eq!(format_ns(1_500_000), "1.50 ms");
        assert_eq!(format_ns(1_500_000_000), "1.50 s");
    }

    #[test]
    fn test_truncate_with_long_string_adds_ellipsis() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a very long string", 10), "this is...");
    }
}
