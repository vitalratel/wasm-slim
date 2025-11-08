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
