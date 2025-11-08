//! Build result formatting and display

use console::{style, Emoji};

use super::metrics::SizeMetrics;
use crate::fmt::format_bytes;

static CHART: Emoji = Emoji("📊", "~");
static SPARKLES: Emoji = Emoji("✨", "*");

/// Formats and displays build results
pub struct ResultFormatter;

impl ResultFormatter {
    /// Print a formatted summary of build size metrics
    pub fn print_summary(metrics: &SizeMetrics) {
        println!("\n{} {} Build Summary", CHART, style("📈").bold());
        println!(
            "   {} Before: {}",
            style("→").dim(),
            style(format_bytes(metrics.before_bytes)).yellow()
        );
        println!(
            "   {} After:  {}",
            style("→").dim(),
            style(format_bytes(metrics.after_bytes)).green().bold()
        );

        let reduction = metrics.reduction_bytes();
        let reduction_pct = metrics.reduction_percent();

        if reduction > 0 {
            println!(
                "   {} Saved:  {} ({:.1}% reduction)",
                style("→").dim(),
                style(format_bytes(reduction as u64)).green().bold(),
                reduction_pct
            );
        } else {
            println!("   {} No size reduction", style("→").dim());
        }

        println!(
            "\n{} {} Build complete!",
            SPARKLES,
            style("Success!").green().bold()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_formatter_exists() {
        // ResultFormatter is a unit struct, just verify it exists
        ResultFormatter::print_summary(&SizeMetrics {
            before_bytes: 1000,
            after_bytes: 500,
        });
    }

    #[test]
    fn test_format_with_unicode_content() {
        // Test that formatter handles emojis and unicode properly
        let metrics = SizeMetrics {
            before_bytes: 500_000,
            after_bytes: 250_000,
        };
        // Should not panic with unicode emojis in output
        ResultFormatter::print_summary(&metrics);
    }

    #[test]
    fn test_format_with_extremely_large_numbers() {
        // Test with very large file sizes (GB range)
        let metrics = SizeMetrics {
            before_bytes: 5_000_000_000, // ~5GB
            after_bytes: 2_500_000_000,  // ~2.5GB
        };
        // Should handle large numbers without overflow
        ResultFormatter::print_summary(&metrics);
    }

    #[test]
    fn test_format_with_zero_reduction() {
        // Test case where size doesn't change
        let metrics = SizeMetrics {
            before_bytes: 1000,
            after_bytes: 1000,
        };
        // Should display "No size reduction" message
        ResultFormatter::print_summary(&metrics);
    }

    #[test]
    fn test_format_with_size_increase() {
        // Test case where size actually increases (should show negative reduction)
        let metrics = SizeMetrics {
            before_bytes: 1000,
            after_bytes: 1500,
        };
        // Should handle negative reduction gracefully
        ResultFormatter::print_summary(&metrics);
    }

    #[test]
    fn test_format_with_very_small_sizes() {
        // Test with very small file sizes (bytes)
        let metrics = SizeMetrics {
            before_bytes: 100,
            after_bytes: 50,
        };
        ResultFormatter::print_summary(&metrics);
    }

    #[test]
    fn test_format_with_zero_before_size() {
        // Edge case: zero before size
        let metrics = SizeMetrics {
            before_bytes: 0,
            after_bytes: 0,
        };
        // Should not panic with division by zero
        ResultFormatter::print_summary(&metrics);
    }

    #[test]
    fn test_format_with_one_byte() {
        // Edge case: minimal sizes
        let metrics = SizeMetrics {
            before_bytes: 1,
            after_bytes: 0,
        };
        ResultFormatter::print_summary(&metrics);
    }

    #[test]
    fn test_format_with_exact_half_reduction() {
        // Test 50% reduction for clean percentage display
        let metrics = SizeMetrics {
            before_bytes: 2000,
            after_bytes: 1000,
        };
        ResultFormatter::print_summary(&metrics);
        assert_eq!(metrics.reduction_percent(), 50.0);
    }

    #[test]
    fn test_format_with_99_percent_reduction() {
        // Test near-total reduction
        let metrics = SizeMetrics {
            before_bytes: 100_000,
            after_bytes: 1_000,
        };
        ResultFormatter::print_summary(&metrics);
        assert!(metrics.reduction_percent() > 90.0);
    }
}
