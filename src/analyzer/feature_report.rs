//! Feature analysis report formatting

use super::features::FeatureAnalysisResults;
use super::report_utils;
use console::style;
use std::fmt::{self, Write as _};

/// Format feature analysis results for console output
pub fn format_console_report(results: &FeatureAnalysisResults) -> Result<String, fmt::Error> {
    let mut output = String::new();

    // Header
    writeln!(output, "\n{} Feature Flag Analysis", style("🔍").bold())?;
    writeln!(
        output,
        "   Total Features Analyzed: {}",
        style(results.total_features).cyan()
    )?;
    writeln!(
        output,
        "   Potentially Unused: {}",
        style(results.unused_features.len()).yellow()
    )?;
    writeln!(
        output,
        "   Estimated Savings: {} KB\n",
        style(results.estimated_savings_kb).green()
    )?;

    // Unused features table
    if !results.unused_features.is_empty() {
        writeln!(output, "{} Potentially Unused Features", style("⚠️").bold())?;
        writeln!(
            output,
            "   {:<20} {:<15} {:<12} {:<10} Confidence",
            "Package", "Feature", "Enabled By", "Impact"
        )?;
        output.push_str("   ─────────────────────────────────────────────────────────────────\n");

        for feature in &results.unused_features {
            let confidence_str = match feature.confidence.as_str() {
                "High" => style(&feature.confidence).green(),
                "Medium" => style(&feature.confidence).yellow(),
                _ => style(&feature.confidence).dim(),
            };

            writeln!(
                output,
                "   {:<20} {:<15} {:<12} {:<10} {}",
                truncate(&feature.package, 20),
                truncate(&feature.feature, 15),
                &feature.enabled_by,
                format!("~{}KB", feature.estimated_impact_kb),
                confidence_str
            )?;
        }
    }

    // Recommendations
    if !results.recommendations.is_empty() {
        writeln!(output, "\n{} Recommendations", style("💡").bold())?;

        for rec in &results.recommendations {
            if rec.starts_with('✅') {
                writeln!(output, "   {}", style(rec).green())?;
            } else {
                writeln!(output, "   • {}", rec)?;
            }
        }
    }

    output.push_str(
        "\n💡 Tip: Use 'default-features = false' and explicitly enable only needed features\n",
    );
    output.push_str("   Example: serde = { version = \"1.0\", default-features = false, features = [\"derive\"] }\n\n");

    Ok(output)
}

/// Format feature analysis results as JSON
pub fn format_json_report(results: &FeatureAnalysisResults) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(results)
}

// Use shared formatting utilities from report_utils
fn truncate(s: &str, max_len: usize) -> String {
    report_utils::truncate_str(s, max_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_with_long_string_adds_ellipsis() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("very_long_string_here", 10), "very_lo...");
    }

    #[test]
    fn test_format_json_report_serializes_feature_analysis_results() {
        let results = FeatureAnalysisResults {
            total_features: 10,
            unused_features: vec![],
            estimated_savings_kb: 0,
            recommendations: vec![],
        };
        let json = format_json_report(&results).unwrap();
        assert!(json.contains("total_features"));
    }
}
