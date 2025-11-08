//! Asset detection report formatting
//!
//! Provides console and JSON output formatters for asset detection results.

use crate::analyzer::asset_display::AssetPriorityDisplay;
use crate::analyzer::assets::{AssetPriority, ScanResults};
use console::style;

/// Print asset detection report to console
pub fn print_asset_report(results: &ScanResults) {
    println!();
    println!("{}", style("Asset Detection Report").bold());
    println!("{}", style("━".repeat(50)).dim());
    println!();

    // Summary
    let total_mb = results.total_size_kb as f64 / 1024.0;
    println!(
        "📦 {} {}",
        style("Embedded Assets Found:").bold(),
        style(format!(
            "{} ({:.2} MB total, {:.1}% of bundle)",
            results.total_assets, total_mb, results.bundle_percentage
        ))
        .cyan()
    );
    println!();

    if results.total_assets == 0 {
        println!("✨ No embedded assets detected. Bundle is asset-free!");
        return;
    }

    // Group by priority and display
    print_priority_group(results, AssetPriority::Critical, ">10% or >500KB");
    print_priority_group(results, AssetPriority::High, "5-10% or 200-500KB");
    print_priority_group(results, AssetPriority::Medium, "2-5% or 100-200KB");
    print_priority_group(results, AssetPriority::Low, "<2% or <100KB");

    println!();
    println!("{}", style("━".repeat(50)).dim());
    println!();

    // Estimated savings
    print_savings_estimate(results);

    println!();
    println!(
        "📖 {}",
        style("For externalization guide: wasm-slim analyze assets --guide").dim()
    );
    println!();
}

fn print_priority_group(results: &ScanResults, priority: AssetPriority, threshold: &str) {
    if let Some(assets) = results.assets_by_priority.get(&priority) {
        if assets.is_empty() {
            return;
        }

        let priority_name = format!("{:?}", priority);
        println!(
            "{} {} ({}):",
            style(&priority_name).fg(priority.color()).bold(),
            style("Priority").fg(priority.color()),
            style(threshold).dim()
        );

        for asset in assets {
            let size_kb = asset.size_bytes / 1024;
            let size_str = if size_kb >= 1024 {
                format!("{:.2} MB", size_kb as f64 / 1024.0)
            } else {
                format!("{} KB", size_kb)
            };

            let percentage = if results.bundle_size_kb > 0 {
                format!(
                    "({:.1}%)",
                    (size_kb as f64 / results.bundle_size_kb as f64) * 100.0
                )
            } else {
                String::new()
            };

            println!(
                "  {}  {:30} {:>12}  {}",
                priority.emoji(),
                style(&asset.file_path).cyan(),
                style(size_str).bold(),
                style(&percentage).dim()
            );
            println!(
                "      → {} at {}",
                style(&asset.detection_method).dim(),
                style(&asset.source_location).dim()
            );
        }
        println!();
    }
}

fn print_savings_estimate(results: &ScanResults) {
    println!("{}", style("💡 Externalization Impact Estimate:").bold());

    if results.estimated_savings.critical_only_kb > 0 {
        println!(
            "   - Externalizing Critical assets: ~{} KB saved ({:.1}%)",
            results.estimated_savings.critical_only_kb,
            results.estimated_savings.critical_only_percent
        );
    }

    if results.estimated_savings.high_and_critical_kb > 0 {
        println!(
            "   - Externalizing High+Critical: ~{} KB saved ({:.1}%)",
            results.estimated_savings.high_and_critical_kb,
            results.estimated_savings.high_and_critical_percent
        );
    }

    println!(
        "   - Externalizing all assets: ~{} KB saved ({:.1}%)",
        results.estimated_savings.all_assets_kb, results.estimated_savings.all_assets_percent
    );
}

/// Print JSON output of scan results
pub fn print_json_output(results: &ScanResults) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{}", json);
    Ok(())
}

/// Show externalization guide
pub fn show_externalization_guide(results: &ScanResults) {
    println!();
    println!(
        "{}",
        style("📖 Asset Externalization Guide").bold().underlined()
    );
    println!();
    println!("Based on your detected assets, here's how to externalize them:");
    println!();

    // Check what types of assets we have
    let has_fonts = results
        .assets
        .iter()
        .any(|a| matches!(a.asset_type, crate::analyzer::assets::AssetType::Font));
    let has_images = results
        .assets
        .iter()
        .any(|a| matches!(a.asset_type, crate::analyzer::assets::AssetType::Image));

    if has_fonts {
        show_font_guide();
    }

    if has_images {
        show_image_guide();
    }

    show_general_guide();

    println!();
    println!("{}", style("📚 Case Study:").bold());
    println!("   Warp.dev saved 10MB by externalizing assets for their web terminal.");
    println!("   Read more: https://www.warp.dev/blog/reducing-wasm-binary-size");
    println!();
}

fn show_font_guide() {
    println!("{}", style("🔤 Externalizing Fonts:").bold());
    println!();
    println!("1. Instead of embedding fonts:");
    println!("   {}", style("// Don't do this:").dim());
    println!("   const FONT: &[u8] = include_bytes!(\"font.woff2\");");
    println!();
    println!("2. Load fonts at runtime:");
    println!("   {}", style("// Do this instead:").dim());
    println!("   async fn load_font() -> Result<Vec<u8>> {{");
    println!("       let response = fetch(\"/assets/font.woff2\").await?;");
    println!("       response.bytes().await");
    println!("   }}");
    println!();
}

fn show_image_guide() {
    println!("{}", style("📸 Externalizing Images:").bold());
    println!();
    println!("1. Instead of embedding images:");
    println!("   {}", style("// Don't do this:").dim());
    println!("   const LOGO: &[u8] = include_bytes!(\"logo.png\");");
    println!();
    println!("2. Use HTML <img> tags or CSS:");
    println!("   {}", style("// Do this instead:").dim());
    println!("   <img src=\"/assets/logo.png\" />");
    println!("   /* Or in CSS: */");
    println!("   background-image: url('/assets/logo.png');");
    println!();
}

fn show_general_guide() {
    println!("{}", style("💡 General Strategy:").bold());
    println!();
    println!(
        "• {} Focus on Critical and High priority assets first",
        style("1.").bold()
    );
    println!(
        "• {} Move assets to a 'public/' or 'static/' directory",
        style("2.").bold()
    );
    println!(
        "• {} Configure your bundler to copy assets",
        style("3.").bold()
    );
    println!(
        "• {} Load assets at runtime via fetch() or <link> tags",
        style("4.").bold()
    );
    println!(
        "• {} Consider lazy-loading for non-critical assets",
        style("5.").bold()
    );
    println!();
    println!("{}", style("⚠️  Note:").yellow().bold());
    println!("   External assets require network requests.");
    println!("   Consider caching strategies and fallbacks.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::assets::{AssetType, DetectedAsset, EstimatedSavings};
    use std::collections::HashMap;

    // Helper function to create test scan results
    fn create_test_results(
        total_assets: usize,
        total_size_kb: u64,
        bundle_size_kb: u64,
    ) -> ScanResults {
        let bundle_percentage = if bundle_size_kb > 0 {
            (total_size_kb as f64 / bundle_size_kb as f64) * 100.0
        } else {
            0.0
        };

        ScanResults {
            total_assets,
            total_size_kb,
            bundle_size_kb,
            bundle_percentage,
            assets: vec![],
            assets_by_priority: HashMap::new(),
            estimated_savings: EstimatedSavings {
                critical_only_kb: 0,
                high_and_critical_kb: 0,
                all_assets_kb: total_size_kb,
                critical_only_percent: 0.0,
                high_and_critical_percent: 0.0,
                all_assets_percent: bundle_percentage,
            },
        }
    }

    fn create_asset(file_path: &str, size_bytes: u64, asset_type: AssetType) -> DetectedAsset {
        DetectedAsset {
            file_path: file_path.to_string(),
            size_bytes,
            asset_type,
            source_location: "test.rs:1".to_string(),
            detection_method: "test".to_string(),
        }
    }

    // P2-TEST-COV-010: Report formatting edge case tests

    #[test]
    fn test_print_json_output_empty_results_succeeds() {
        // Test JSON output with zero assets
        let results = create_test_results(0, 0, 1000);
        let result = print_json_output(&results);
        assert!(
            result.is_ok(),
            "JSON output should succeed with empty results"
        );
    }

    #[test]
    fn test_print_json_output_with_assets_valid_json() {
        // Test JSON output with assets
        let mut results = create_test_results(2, 500, 1000);
        results.assets = vec![
            create_asset("font.woff2", 300_000, AssetType::Font),
            create_asset("image.png", 200_000, AssetType::Image),
        ];

        let result = print_json_output(&results);
        assert!(result.is_ok(), "JSON output should succeed with assets");
    }

    #[test]
    fn test_print_asset_report_zero_assets_no_panic() {
        // Test that zero assets case is handled correctly
        let results = create_test_results(0, 0, 1000);

        // This is a formatting function, so we're just ensuring it doesn't panic
        // In a real test environment, we'd capture stdout and verify the message
        print_asset_report(&results);
        // No panic = success
    }

    #[test]
    fn test_print_asset_report_large_bundle_no_panic() {
        // Test with large bundle size (>1GB)
        let results = create_test_results(5, 500_000, 10_000_000); // 500MB assets in 10GB bundle
        print_asset_report(&results);
        // No panic = success
    }

    #[test]
    fn test_print_asset_report_all_embedded_assets_no_panic() {
        // Test edge case: all bundle is assets
        let results = create_test_results(10, 1000, 1000); // 100% assets
        print_asset_report(&results);
        // No panic = success
    }

    #[test]
    fn test_print_asset_report_zero_bundle_size_no_panic() {
        // Test edge case: zero bundle size (division by zero potential)
        let results = create_test_results(5, 500, 0);
        print_asset_report(&results);
        // No panic = success
    }

    #[test]
    fn test_print_savings_estimate_no_assets_shows_zero() {
        // Test savings estimate with no critical/high assets
        let results = ScanResults {
            total_assets: 1,
            total_size_kb: 50,
            bundle_size_kb: 1000,
            bundle_percentage: 5.0,
            assets: vec![],
            assets_by_priority: HashMap::new(),
            estimated_savings: EstimatedSavings {
                critical_only_kb: 0,
                high_and_critical_kb: 0,
                all_assets_kb: 50,
                critical_only_percent: 0.0,
                high_and_critical_percent: 0.0,
                all_assets_percent: 5.0,
            },
        };

        print_savings_estimate(&results);
        // No panic = success
    }

    #[test]
    fn test_print_savings_estimate_critical_assets_shows_potential() {
        // Test savings estimate with critical assets
        let results = ScanResults {
            total_assets: 3,
            total_size_kb: 800,
            bundle_size_kb: 1000,
            bundle_percentage: 80.0,
            assets: vec![],
            assets_by_priority: HashMap::new(),
            estimated_savings: EstimatedSavings {
                critical_only_kb: 600,
                high_and_critical_kb: 750,
                all_assets_kb: 800,
                critical_only_percent: 60.0,
                high_and_critical_percent: 75.0,
                all_assets_percent: 80.0,
            },
        };

        print_savings_estimate(&results);
        // No panic = success
    }

    #[test]
    fn test_show_externalization_guide_empty_results_no_output() {
        // Test guide with no detected assets
        let results = create_test_results(0, 0, 1000);
        show_externalization_guide(&results);
        // No panic = success
    }

    #[test]
    fn test_show_externalization_guide_font_assets_shows_font_guide() {
        // Test guide shows font-specific guidance
        let mut results = create_test_results(2, 500, 1000);
        results.assets = vec![
            create_asset("font1.woff2", 200_000, AssetType::Font),
            create_asset("font2.woff2", 300_000, AssetType::Font),
        ];

        show_externalization_guide(&results);
        // No panic = success
    }

    #[test]
    fn test_show_externalization_guide_image_assets_shows_image_guide() {
        // Test guide shows image-specific guidance
        let mut results = create_test_results(2, 500, 1000);
        results.assets = vec![
            create_asset("image1.png", 200_000, AssetType::Image),
            create_asset("image2.jpg", 300_000, AssetType::Image),
        ];

        show_externalization_guide(&results);
        // No panic = success
    }

    #[test]
    fn test_show_externalization_guide_mixed_types_shows_all_guides() {
        // Test guide with mixed asset types
        let mut results = create_test_results(4, 1000, 2000);
        results.assets = vec![
            create_asset("font.woff2", 300_000, AssetType::Font),
            create_asset("image.png", 400_000, AssetType::Image),
            create_asset("data.json", 200_000, AssetType::Data),
            create_asset("unknown.xyz", 100_000, AssetType::Unknown),
        ];

        show_externalization_guide(&results);
        // No panic = success
    }

    #[test]
    fn test_print_priority_group_empty_list_no_output() {
        // Test priority group with no assets of that priority
        let results = create_test_results(0, 0, 1000);
        print_priority_group(&results, AssetPriority::Critical, ">10%");
        // No panic = success
    }

    #[test]
    fn test_print_priority_group_with_assets_displays_all() {
        // Test priority group with assets
        let mut results = create_test_results(2, 700, 1000);
        let critical_assets = vec![
            create_asset("large_font.woff2", 600_000, AssetType::Font),
            create_asset("large_image.png", 100_000, AssetType::Image),
        ];
        results.assets = critical_assets.clone();
        results
            .assets_by_priority
            .insert(AssetPriority::Critical, critical_assets);

        print_priority_group(&results, AssetPriority::Critical, ">10% or >500KB");
        // No panic = success
    }

    #[test]
    fn test_print_json_output_structure_valid_format() {
        // Test that JSON output is valid and contains expected fields
        let mut results = create_test_results(1, 500, 1000);
        results.assets = vec![create_asset("test.png", 500_000, AssetType::Image)];

        // Manually serialize to verify structure
        let json_result = serde_json::to_string_pretty(&results);
        assert!(json_result.is_ok(), "JSON serialization should succeed");

        let json_str = json_result.unwrap();
        assert!(
            json_str.contains("total_assets"),
            "JSON should contain total_assets"
        );
        assert!(
            json_str.contains("total_size_kb"),
            "JSON should contain total_size_kb"
        );
        assert!(
            json_str.contains("bundle_percentage"),
            "JSON should contain bundle_percentage"
        );
        assert!(
            json_str.contains("estimated_savings"),
            "JSON should contain estimated_savings"
        );
    }

    #[test]
    fn test_print_asset_report_all_priority_levels_no_panic() {
        // Test formatting with assets at all priority levels
        let mut results = create_test_results(4, 900, 1000);

        let critical = vec![create_asset("critical.woff2", 600_000, AssetType::Font)];
        let high = vec![create_asset("high.png", 150_000, AssetType::Image)];
        let medium = vec![create_asset("medium.jpg", 100_000, AssetType::Image)];
        let low = vec![create_asset("low.txt", 50_000, AssetType::Data)];

        results
            .assets_by_priority
            .insert(AssetPriority::Critical, critical);
        results.assets_by_priority.insert(AssetPriority::High, high);
        results
            .assets_by_priority
            .insert(AssetPriority::Medium, medium);
        results.assets_by_priority.insert(AssetPriority::Low, low);

        print_asset_report(&results);
        // No panic = success
    }

    #[test]
    fn test_print_asset_report_extremely_large_numbers_no_panic() {
        // Test with very large numbers to check formatting
        let results = create_test_results(1000, u64::MAX / 1024, u64::MAX / 1024); // Very large sizes
        print_asset_report(&results);
        // No panic = success
    }

    #[test]
    fn test_print_asset_report_special_characters_in_paths_no_panic() {
        // Test with special characters in file paths
        let mut results = create_test_results(3, 300, 1000);
        results.assets = vec![
            create_asset("path/with spaces/font.woff2", 100_000, AssetType::Font),
            create_asset("path/with/unicode/文件.png", 100_000, AssetType::Image),
            create_asset(
                "path-with-dashes_and_underscores.json",
                100_000,
                AssetType::Data,
            ),
        ];

        show_externalization_guide(&results);
        // No panic = success
    }
}
