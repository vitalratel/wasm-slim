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

    #[test]
    fn test_reporter_new() {
        let _reporter = BenchmarkReporter::new();
        // Should not panic
    }

    #[test]
    fn test_reporter_default() {
        let _reporter = BenchmarkReporter;
        // Should not panic
    }

    #[test]
    fn test_print_comparison_empty_list() {
        let reporter = BenchmarkReporter::new();
        let comparisons = vec![];
        // Should not panic
        reporter.print_comparison(&comparisons);
    }

    #[test]
    fn test_print_comparison_with_regression() {
        let reporter = BenchmarkReporter::new();
        let comparisons = vec![BenchmarkComparison {
            name: "test_bench".to_string(),
            baseline_mean_ns: 1000,
            current_mean_ns: 1200,
            change_percent: 20.0,
            is_regression: true,
            exceeds_budget: false,
        }];
        reporter.print_comparison(&comparisons);
    }

    #[test]
    fn test_print_comparison_with_improvement() {
        let reporter = BenchmarkReporter::new();
        let comparisons = vec![BenchmarkComparison {
            name: "test_bench".to_string(),
            baseline_mean_ns: 1000,
            current_mean_ns: 800,
            change_percent: -20.0,
            is_regression: false,
            exceeds_budget: false,
        }];
        reporter.print_comparison(&comparisons);
    }

    #[test]
    fn test_print_comparison_with_stable_performance() {
        let reporter = BenchmarkReporter::new();
        let comparisons = vec![BenchmarkComparison {
            name: "test_bench".to_string(),
            baseline_mean_ns: 1000,
            current_mean_ns: 1010,
            change_percent: 1.0,
            is_regression: false,
            exceeds_budget: false,
        }];
        reporter.print_comparison(&comparisons);
    }

    #[test]
    fn test_print_comparison_with_budget_exceeded() {
        let reporter = BenchmarkReporter::new();
        let comparisons = vec![BenchmarkComparison {
            name: "test_bench".to_string(),
            baseline_mean_ns: 1000,
            current_mean_ns: 1100,
            change_percent: 10.0,
            is_regression: false,
            exceeds_budget: true,
        }];
        reporter.print_comparison(&comparisons);
    }

    #[test]
    fn test_print_comparison_multiple_benchmarks() {
        let reporter = BenchmarkReporter::new();
        let comparisons = vec![
            BenchmarkComparison {
                name: "bench1".to_string(),
                baseline_mean_ns: 1000,
                current_mean_ns: 1200,
                change_percent: 20.0,
                is_regression: true,
                exceeds_budget: false,
            },
            BenchmarkComparison {
                name: "bench2".to_string(),
                baseline_mean_ns: 2000,
                current_mean_ns: 1800,
                change_percent: -10.0,
                is_regression: false,
                exceeds_budget: false,
            },
        ];
        reporter.print_comparison(&comparisons);
    }

    #[test]
    fn test_print_comparison_long_benchmark_name() {
        let reporter = BenchmarkReporter::new();
        let comparisons = vec![BenchmarkComparison {
            name: "this_is_a_very_long_benchmark_name_that_should_be_truncated".to_string(),
            baseline_mean_ns: 1000,
            current_mean_ns: 1100,
            change_percent: 10.0,
            is_regression: false,
            exceeds_budget: false,
        }];
        reporter.print_comparison(&comparisons);
    }

    #[test]
    fn test_format_ns_all_units() {
        // Nanoseconds
        assert_eq!(format_ns(0), "0 ns");
        assert_eq!(format_ns(1), "1 ns");
        assert_eq!(format_ns(999), "999 ns");

        // Microseconds
        assert_eq!(format_ns(1_000), "1.00 µs");
        assert_eq!(format_ns(123_456), "123.46 µs");

        // Milliseconds
        assert_eq!(format_ns(1_000_000), "1.00 ms");
        assert_eq!(format_ns(123_456_789), "123.46 ms");

        // Seconds
        assert_eq!(format_ns(1_000_000_000), "1.00 s");
        assert_eq!(format_ns(5_000_000_000), "5.00 s");
    }

    #[test]
    fn test_truncate_edge_cases() {
        assert_eq!(truncate("", 10), "");
        assert_eq!(truncate("abc", 3), "abc");
        assert_eq!(truncate("abcd", 3), "...");
        assert_eq!(truncate("abcdef", 5), "ab...");
    }

    #[test]
    fn test_print_comparison_boundary_change_percent() {
        let reporter = BenchmarkReporter::new();

        // Exactly -5.0% (should show green)
        let comparisons = vec![BenchmarkComparison {
            name: "test_bench".to_string(),
            baseline_mean_ns: 1000,
            current_mean_ns: 950,
            change_percent: -5.0,
            is_regression: false,
            exceeds_budget: false,
        }];
        reporter.print_comparison(&comparisons);

        // Just below -5.0% (should show green)
        let comparisons = vec![BenchmarkComparison {
            name: "test_bench".to_string(),
            baseline_mean_ns: 1000,
            current_mean_ns: 940,
            change_percent: -6.0,
            is_regression: false,
            exceeds_budget: false,
        }];
        reporter.print_comparison(&comparisons);
    }
}
