//! Display formatting for CI/CD results

use super::history::RegressionResult;
use console::style;

/// Print regression result to console with formatted output
pub fn print_regression(result: &RegressionResult) {
    if result.is_regression {
        println!("\n{} Size Regression Detected!", style("⚠️").red());
        println!(
            "   Previous: {:.2} KB",
            result.previous_size as f64 / 1024.0
        );
        println!(
            "   Current:  {:.2} KB ({})",
            result.current_size as f64 / 1024.0,
            style(format!("+{:.1}%", result.percent_change)).red()
        );
        println!("   Increase: {:.2} KB", result.size_diff as f64 / 1024.0);
    } else if result.percent_change < -1.0 {
        // Size reduction
        println!("\n{} Size Improvement!", style("✨").green());
        println!(
            "   Previous: {:.2} KB",
            result.previous_size as f64 / 1024.0
        );
        println!(
            "   Current:  {:.2} KB ({})",
            result.current_size as f64 / 1024.0,
            style(format!("{:.1}%", result.percent_change)).green()
        );
        println!("   Reduction: {:.2} KB", -result.size_diff as f64 / 1024.0);
    } else {
        // No significant change
        println!(
            "\n{} Size stable ({})",
            style("✓").dim(),
            style(format!("{:+.1}%", result.percent_change)).dim()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_regression_with_size_increase() {
        let result = RegressionResult {
            is_regression: true,
            previous_size: 100_000,
            current_size: 110_000,
            size_diff: 10_000,
            percent_change: 10.0,
        };

        // Should not panic
        print_regression(&result);
    }

    #[test]
    fn test_print_regression_with_size_decrease() {
        let result = RegressionResult {
            is_regression: false,
            previous_size: 100_000,
            current_size: 90_000,
            size_diff: -10_000,
            percent_change: -10.0,
        };

        // Should not panic
        print_regression(&result);
    }

    #[test]
    fn test_print_regression_with_stable_size() {
        let result = RegressionResult {
            is_regression: false,
            previous_size: 100_000,
            current_size: 100_500,
            size_diff: 500,
            percent_change: 0.5,
        };

        // Should not panic
        print_regression(&result);
    }

    #[test]
    fn test_print_regression_boundary_small_reduction() {
        // Exactly -1.0% should still show stable
        let result = RegressionResult {
            is_regression: false,
            previous_size: 100_000,
            current_size: 99_000,
            size_diff: -1_000,
            percent_change: -1.0,
        };

        print_regression(&result);
    }

    #[test]
    fn test_print_regression_boundary_significant_reduction() {
        // Just over -1.0% should show improvement
        let result = RegressionResult {
            is_regression: false,
            previous_size: 100_000,
            current_size: 98_900,
            size_diff: -1_100,
            percent_change: -1.1,
        };

        print_regression(&result);
    }

    #[test]
    fn test_print_regression_zero_change() {
        let result = RegressionResult {
            is_regression: false,
            previous_size: 100_000,
            current_size: 100_000,
            size_diff: 0,
            percent_change: 0.0,
        };

        print_regression(&result);
    }

    #[test]
    fn test_print_regression_large_numbers() {
        let result = RegressionResult {
            is_regression: true,
            previous_size: 10_000_000,
            current_size: 11_000_000,
            size_diff: 1_000_000,
            percent_change: 10.0,
        };

        print_regression(&result);
    }

    #[test]
    fn test_print_regression_small_numbers() {
        let result = RegressionResult {
            is_regression: false,
            previous_size: 1_000,
            current_size: 900,
            size_diff: -100,
            percent_change: -10.0,
        };

        print_regression(&result);
    }
}
