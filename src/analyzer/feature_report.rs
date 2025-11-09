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

    #[test]
    fn test_format_console_report_with_empty_results() {
        let results = FeatureAnalysisResults {
            total_features: 5,
            unused_features: vec![],
            estimated_savings_kb: 0,
            recommendations: vec![],
        };

        let output = format_console_report(&results);
        assert!(output.is_ok());
        let text = output.unwrap();
        assert!(text.contains("Feature Flag Analysis"));
        assert!(text.contains("Total Features Analyzed: 5"));
        assert!(text.contains("Potentially Unused: 0"));
    }

    #[test]
    fn test_format_console_report_with_unused_features() {
        use super::super::features::UnusedFeature;

        let results = FeatureAnalysisResults {
            total_features: 10,
            unused_features: vec![
                UnusedFeature {
                    package: "serde".to_string(),
                    feature: "derive".to_string(),
                    enabled_by: "default".to_string(),
                    estimated_impact_kb: 50,
                    confidence: "High".to_string(),
                },
                UnusedFeature {
                    package: "tokio".to_string(),
                    feature: "full".to_string(),
                    enabled_by: "explicit".to_string(),
                    estimated_impact_kb: 100,
                    confidence: "Medium".to_string(),
                },
            ],
            estimated_savings_kb: 150,
            recommendations: vec![],
        };

        let output = format_console_report(&results);
        assert!(output.is_ok());
        let text = output.unwrap();
        assert!(text.contains("serde"));
        assert!(text.contains("derive"));
        assert!(text.contains("tokio"));
        assert!(text.contains("High"));
        assert!(text.contains("Medium"));
    }

    #[test]
    fn test_format_console_report_with_recommendations() {
        let results = FeatureAnalysisResults {
            total_features: 10,
            unused_features: vec![],
            estimated_savings_kb: 20,
            recommendations: vec![
                "✅ Use default-features = false".to_string(),
                "Consider splitting dependencies".to_string(),
                "Review feature usage".to_string(),
            ],
        };

        let output = format_console_report(&results);
        assert!(output.is_ok());
        let text = output.unwrap();
        assert!(text.contains("Recommendations"));
        assert!(text.contains("default-features = false"));
        assert!(text.contains("splitting dependencies"));
    }

    #[test]
    fn test_format_console_report_with_all_confidence_levels() {
        use super::super::features::UnusedFeature;

        let results = FeatureAnalysisResults {
            total_features: 15,
            unused_features: vec![
                UnusedFeature {
                    package: "pkg1".to_string(),
                    feature: "feat1".to_string(),
                    enabled_by: "default".to_string(),
                    estimated_impact_kb: 10,
                    confidence: "High".to_string(),
                },
                UnusedFeature {
                    package: "pkg2".to_string(),
                    feature: "feat2".to_string(),
                    enabled_by: "default".to_string(),
                    estimated_impact_kb: 20,
                    confidence: "Medium".to_string(),
                },
                UnusedFeature {
                    package: "pkg3".to_string(),
                    feature: "feat3".to_string(),
                    enabled_by: "default".to_string(),
                    estimated_impact_kb: 5,
                    confidence: "Low".to_string(),
                },
            ],
            estimated_savings_kb: 35,
            recommendations: vec![],
        };

        let output = format_console_report(&results);
        assert!(output.is_ok());
        let text = output.unwrap();
        assert!(text.contains("High"));
        assert!(text.contains("Medium"));
        assert!(text.contains("Low"));
    }

    #[test]
    fn test_format_json_report_with_unused_features() {
        use super::super::features::UnusedFeature;

        let results = FeatureAnalysisResults {
            total_features: 8,
            unused_features: vec![UnusedFeature {
                package: "test".to_string(),
                feature: "test_feature".to_string(),
                enabled_by: "default".to_string(),
                estimated_impact_kb: 25,
                confidence: "High".to_string(),
            }],
            estimated_savings_kb: 25,
            recommendations: vec!["Test recommendation".to_string()],
        };

        let json = format_json_report(&results).unwrap();
        assert!(json.contains("total_features"));
        assert!(json.contains("unused_features"));
        assert!(json.contains("test"));
        assert!(json.contains("test_feature"));
        assert!(json.contains("recommendations"));
    }
}
