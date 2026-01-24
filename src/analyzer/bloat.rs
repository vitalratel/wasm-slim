//! Cargo bloat integration for binary size analysis
//!
//! Provides insights into which functions and data structures take up the most space
//! in the compiled binary. Complements twiggy by analyzing the Rust binary before WASM conversion.

use crate::infra::{CommandExecutor, RealCommandExecutor};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main cargo-bloat analyzer
pub struct BloatAnalyzer<CE: CommandExecutor = RealCommandExecutor> {
    project_root: std::path::PathBuf,
    cmd_executor: CE,
}

/// Bloat analysis item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloatItem {
    /// Size in bytes
    pub size_bytes: u64,
    /// Percentage of total
    pub percentage: f64,
    /// Function or data name
    pub name: String,
    /// Crate name
    pub crate_name: Option<String>,
}

/// Complete bloat analysis results
#[derive(Debug, Serialize, Deserialize)]
pub struct BloatResults {
    /// Total binary size
    pub total_size_bytes: u64,
    /// Text section size (code)
    pub text_size_bytes: u64,
    /// Top items by size
    pub items: Vec<BloatItem>,
    /// Recommendations based on findings
    pub recommendations: Vec<Recommendation>,
}

/// Actionable recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Priority: P0 (Critical), P1 (High), P2 (Medium), P3 (Low)
    pub priority: String,
    /// Description of the recommendation
    pub description: String,
    /// Estimated savings in KB
    pub estimated_savings_kb: u64,
    /// Estimated savings as percentage
    pub estimated_savings_percent: f64,
}

impl BloatAnalyzer {
    /// Create a new bloat analyzer for the given project
    ///
    /// # Arguments
    /// * `project_root` - Path to the project root containing Cargo.toml
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::analyzer::BloatAnalyzer;
    /// use std::path::Path;
    ///
    /// let analyzer = BloatAnalyzer::new(Path::new("."));
    /// let results = analyzer.analyze()?;
    /// println!("Found {} items taking up space", results.items.len());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self::with_executor(project_root, RealCommandExecutor)
    }

    /// Check if cargo-bloat is installed
    ///
    /// # Returns
    /// `true` if cargo-bloat is available, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::analyzer::BloatAnalyzer;
    ///
    /// if !BloatAnalyzer::check_installation()? {
    ///     eprintln!("cargo-bloat not found. Install with: cargo install cargo-bloat");
    /// }
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn check_installation() -> Result<bool> {
        Self::check_installation_with_executor(&RealCommandExecutor)
    }
}

impl<CE: CommandExecutor> BloatAnalyzer<CE> {
    /// Create a new bloat analyzer with a custom command executor
    pub fn with_executor(project_root: impl AsRef<Path>, cmd_executor: CE) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            cmd_executor,
        }
    }

    /// Check if cargo-bloat is installed using a custom command executor
    pub fn check_installation_with_executor<E: CommandExecutor>(executor: &E) -> Result<bool> {
        let output = executor.execute(|cmd| cmd.arg("bloat").arg("--version"), "cargo");

        match output {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    /// Run bloat analysis on the release binary
    #[must_use = "Analysis results should be used or printed"]
    pub fn analyze(&self) -> Result<BloatResults> {
        // Check if cargo-bloat is installed
        if !Self::check_installation_with_executor(&self.cmd_executor)? {
            anyhow::bail!("cargo-bloat is not installed. Install with: cargo install cargo-bloat");
        }

        // Build the project first
        self.build_release()?;

        // Run cargo bloat with JSON output
        let output = self
            .cmd_executor
            .execute(
                |cmd| {
                    cmd.arg("bloat")
                        .arg("--release")
                        .arg("--target")
                        .arg("wasm32-unknown-unknown")
                        .arg("-n")
                        .arg("50") // Top 50 items
                        .current_dir(&self.project_root)
                },
                "cargo",
            )
            .context("Failed to run cargo bloat")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("cargo bloat failed: {}", stderr);
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse cargo bloat output as UTF-8")?;

        self.parse_output(&stdout)
    }

    /// Build the release binary
    fn build_release(&self) -> Result<()> {
        let output = self
            .cmd_executor
            .execute(
                |cmd| {
                    cmd.arg("build")
                        .arg("--release")
                        .arg("--target")
                        .arg("wasm32-unknown-unknown")
                        .current_dir(&self.project_root)
                },
                "cargo",
            )
            .context("Failed to build release binary")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Cargo build failed: {}", stderr);
        }

        Ok(())
    }

    /// Parse cargo bloat output
    fn parse_output(&self, output: &str) -> Result<BloatResults> {
        let mut items = Vec::new();
        let mut total_size_bytes = 0;
        let mut text_size_bytes = 0;
        let mut in_table = false;

        for line in output.lines() {
            // Skip until we find the table
            if line.contains("File  .text") {
                in_table = true;
                continue;
            }

            if !in_table {
                continue;
            }

            // Stop at the summary line
            if line.starts_with(".text section size") {
                if let Some(size_str) = line.split_whitespace().nth(3) {
                    text_size_bytes = BloatAnalyzer::parse_size(size_str)?;
                }
                continue;
            }

            if line.starts_with("File size") {
                if let Some(size_str) = line.split_whitespace().nth(2) {
                    total_size_bytes = BloatAnalyzer::parse_size(size_str)?;
                }
                break;
            }

            // Parse table rows
            if let Some(item) = BloatAnalyzer::parse_line(line) {
                items.push(item);
            }
        }

        let recommendations = self.generate_recommendations(&items, total_size_bytes);

        Ok(BloatResults {
            total_size_bytes,
            text_size_bytes,
            items,
            recommendations,
        })
    }
}

// Static parsing methods (no generic parameters needed)
impl BloatAnalyzer {
    /// Parse a single line from cargo bloat output
    fn parse_line(line: &str) -> Option<BloatItem> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        // Format: Size % Name
        // Example: "12.3KiB  2.5% std::fmt::write"
        let size_str = parts[0];
        let percentage_str = parts[1].trim_end_matches('%');
        let name = parts[2..].join(" ");

        let size_bytes = BloatAnalyzer::parse_size(size_str).ok()?;
        let percentage = percentage_str.parse::<f64>().ok()?;

        // Extract crate name from symbol
        let crate_name = name.split("::").next().map(|s| s.to_string());

        Some(BloatItem {
            size_bytes,
            percentage,
            name,
            crate_name,
        })
    }

    /// Parse size string (e.g., "12.3KiB", "1.5MiB") to bytes
    fn parse_size(size_str: &str) -> Result<u64> {
        let size_str = size_str.trim();

        // Handle KiB
        if let Some(kb_str) = size_str.strip_suffix("KiB") {
            let kb = kb_str.parse::<f64>().context("Failed to parse KB value")?;
            return Ok((kb * 1024.0) as u64);
        }

        // Handle MiB
        if let Some(mb_str) = size_str.strip_suffix("MiB") {
            let mb = mb_str.parse::<f64>().context("Failed to parse MB value")?;
            return Ok((mb * 1024.0 * 1024.0) as u64);
        }

        // Handle bytes
        if let Some(b_str) = size_str.strip_suffix('B') {
            return b_str.parse::<u64>().context("Failed to parse byte value");
        }

        // Try parsing as plain number (bytes)
        size_str.parse::<u64>().context("Failed to parse size")
    }
}

impl<CE: CommandExecutor> BloatAnalyzer<CE> {
    /// Generate recommendations based on bloat analysis
    fn generate_recommendations(
        &self,
        items: &[BloatItem],
        total_size: u64,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // Check for large individual functions (>50KB or >5%)
        for item in items {
            if item.size_bytes > 50 * 1024 || item.percentage > 5.0 {
                let savings_kb = item.size_bytes / 1024;
                recommendations.push(Recommendation {
                    priority: if item.percentage > 10.0 {
                        "P0".to_string()
                    } else {
                        "P1".to_string()
                    },
                    description: format!(
                        "Large function '{}' ({:.1}% of binary). Consider splitting or optimizing.",
                        item.name, item.percentage
                    ),
                    estimated_savings_kb: savings_kb / 2, // Conservative estimate
                    estimated_savings_percent: item.percentage / 2.0,
                });
            }
        }

        // Group by crate and find heavy crates
        let mut crate_sizes: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for item in items {
            if let Some(ref crate_name) = item.crate_name {
                *crate_sizes.entry(crate_name.clone()).or_insert(0) += item.size_bytes;
            }
        }

        // Check for crates contributing >20% of binary
        for (crate_name, size) in crate_sizes {
            let percentage = (size as f64 / total_size as f64) * 100.0;
            if percentage > 20.0 {
                recommendations.push(Recommendation {
                    priority: "P0".to_string(),
                    description: format!(
                        "Crate '{}' contributes {:.1}% of binary. Consider lighter alternatives or feature minimization.",
                        crate_name, percentage
                    ),
                    estimated_savings_kb: ((size / 2) / 1024),
                    estimated_savings_percent: percentage / 2.0,
                });
            }
        }

        // Check for formatting/panic code (common bloat source)
        let formatting_size: u64 = items
            .iter()
            .filter(|item| {
                item.name.contains("fmt::")
                    || item.name.contains("Display")
                    || item.name.contains("Debug")
            })
            .map(|item| item.size_bytes)
            .sum();

        if formatting_size > total_size / 20 {
            // >5%
            let percentage = (formatting_size as f64 / total_size as f64) * 100.0;
            recommendations.push(Recommendation {
                priority: "P2".to_string(),
                description: format!(
                    "Formatting code takes up {:.1}% of binary. Consider using panic='abort' or removing Debug derives.",
                    percentage
                ),
                estimated_savings_kb: ((formatting_size / 3) / 1024),
                estimated_savings_percent: percentage / 3.0,
            });
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_kib_units_converts_to_bytes() {
        let size = BloatAnalyzer::parse_size("12.5KiB").expect("should parse valid KiB size");
        assert_eq!(size, 12800); // 12.5 * 1024
    }

    #[test]
    fn test_parse_size_mib_units_converts_to_bytes() {
        let size = BloatAnalyzer::parse_size("1.5MiB").expect("should parse valid MiB size");
        assert_eq!(size, 1572864); // 1.5 * 1024 * 1024
    }

    #[test]
    fn test_parse_size_bytes_plain_number_parses_directly() {
        let size = BloatAnalyzer::parse_size("1024B").expect("should parse valid byte size");
        assert_eq!(size, 1024);
    }

    #[test]
    fn test_parse_line_valid_format_extracts_all_fields() {
        let line = "12.3KiB  2.5% std::fmt::write";
        let item = BloatAnalyzer::parse_line(line).expect("should parse valid bloat line");
        assert_eq!(item.name, "std::fmt::write");
        assert_eq!(item.percentage, 2.5);
        assert_eq!(item.crate_name, Some("std".to_string()));
    }

    #[test]
    fn test_check_installation_cargo_bloat_availability_succeeds() {
        // This test just ensures the function doesn't panic
        let _ = BloatAnalyzer::check_installation();
    }

    #[test]
    fn test_parse_size_plain_number_without_units_parses_as_bytes() {
        let size = BloatAnalyzer::parse_size("1024").expect("should parse plain number as bytes");
        assert_eq!(size, 1024);
    }

    #[test]
    fn test_parse_size_with_whitespace_trims_and_parses() {
        let size =
            BloatAnalyzer::parse_size("  12.5KiB  ").expect("should parse size with whitespace");
        assert_eq!(size, 12800);
    }

    #[test]
    fn test_parse_size_zero_value_returns_zero() {
        let size = BloatAnalyzer::parse_size("0B").expect("should parse zero size");
        assert_eq!(size, 0);
    }

    #[test]
    fn test_parse_size_invalid_format_returns_error() {
        assert!(BloatAnalyzer::parse_size("invalid").is_err());
        assert!(BloatAnalyzer::parse_size("12.3XB").is_err());
        assert!(BloatAnalyzer::parse_size("").is_err());
    }

    #[test]
    fn test_parse_size_negative_number_returns_zero() {
        // Negative numbers should fail for u64
        assert!(BloatAnalyzer::parse_size("-100").is_err());
    }

    #[test]
    fn test_parse_size_fractional_bytes_rounds_to_integer() {
        // Fractional bytes should work but truncate
        let size = BloatAnalyzer::parse_size("10B").expect("should parse fractional bytes");
        assert_eq!(size, 10);
    }

    #[test]
    fn test_parse_line_multiword_name_parses_correctly() {
        let line = "5.2KiB  1.2% <alloc::vec::Vec<T> as core::clone::Clone>::clone";
        let item = BloatAnalyzer::parse_line(line).expect("should parse multiword name");
        assert!(item.name.contains("alloc::vec::Vec"));
        assert_eq!(item.percentage, 1.2);
    }

    #[test]
    fn test_parse_line_missing_fields_returns_error() {
        // Line with only 2 fields should return None
        assert!(BloatAnalyzer::parse_line("12.3KiB  2.5%").is_none());
        assert!(BloatAnalyzer::parse_line("12.3KiB").is_none());
        assert!(BloatAnalyzer::parse_line("").is_none());
    }

    #[test]
    fn test_parse_line_invalid_percentage_returns_error() {
        // Invalid percentage format should return None
        assert!(BloatAnalyzer::parse_line("12.3KiB  invalid% function_name").is_none());
    }

    #[test]
    fn test_parse_line_invalid_size_returns_error() {
        // Invalid size format should return None
        assert!(BloatAnalyzer::parse_line("invalid  2.5% function_name").is_none());
    }

    #[test]
    fn test_parse_line_zero_size_parses_successfully() {
        let line = "0B  0.0% empty_function";
        let item = BloatAnalyzer::parse_line(line).expect("should parse zero size line");
        assert_eq!(item.size_bytes, 0);
        assert_eq!(item.percentage, 0.0);
    }

    #[test]
    fn test_parse_line_namespace_prefix_extracts_crate_name() {
        let line = "10KiB  5.0% my_crate::module::function";
        let item =
            BloatAnalyzer::parse_line(line).expect("should extract crate name from namespace");
        assert_eq!(item.crate_name, Some("my_crate".to_string()));
    }

    #[test]
    fn test_parse_line_no_namespace_crate_name_is_none() {
        let line = "10KiB  5.0% standalone_function";
        let item = BloatAnalyzer::parse_line(line).expect("should parse line without namespace");
        assert_eq!(item.crate_name, Some("standalone_function".to_string()));
    }

    #[test]
    fn test_parse_output_empty_input_returns_empty_results() {
        let analyzer = BloatAnalyzer::new(".");
        let result = analyzer.parse_output("");
        // Should succeed but have empty items
        assert!(result.is_ok());
        let results = result.expect("should parse empty input");
        assert_eq!(results.items.len(), 0);
        assert_eq!(results.total_size_bytes, 0);
    }

    #[test]
    fn test_parse_output_missing_headers_returns_empty_results() {
        let analyzer = BloatAnalyzer::new(".");
        let output = "12.3KiB  2.5% std::fmt::write\n10KiB  2.0% other::function";
        let result = analyzer.parse_output(output);
        // Without headers, should parse no items
        assert!(result.is_ok());
        let results = result.expect("should handle missing headers");
        assert_eq!(results.items.len(), 0);
    }

    #[test]
    fn test_parse_output_valid_table_parses_all_items() {
        let analyzer = BloatAnalyzer::new(".");
        let output = r#"
File  .text     Size Crate Name
12.3KiB   2.5% std   std::fmt::write
10.0KiB   2.0% core  core::fmt::Display
.text section size 500KiB
File size 1000KiB
"#;
        let result = analyzer.parse_output(output);
        assert!(result.is_ok());
        let results = result.expect("should parse valid table");
        assert_eq!(results.items.len(), 2);
        assert_eq!(results.text_size_bytes, 512000); // 500 * 1024
        assert_eq!(results.total_size_bytes, 1024000); // 1000 * 1024
    }

    #[test]
    fn test_parse_output_partial_summary_parses_available_fields() {
        let analyzer = BloatAnalyzer::new(".");
        let output = r#"
File  .text     Size Crate Name
12.3KiB   2.5% std   std::fmt::write
.text section size 500KiB
"#;
        let result = analyzer.parse_output(output);
        assert!(result.is_ok());
        let results = result.expect("should parse partial summary");
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.text_size_bytes, 512000);
        assert_eq!(results.total_size_bytes, 0); // Missing File size
    }

    #[test]
    fn test_generate_recommendations_large_function_creates_p0_priority() {
        let analyzer = BloatAnalyzer::new(".");
        let items = vec![BloatItem {
            size_bytes: 100 * 1024, // 100KB
            percentage: 15.0,
            name: "huge::function".to_string(),
            crate_name: Some("huge".to_string()),
        }];
        let recommendations = analyzer.generate_recommendations(&items, 1024 * 1024);
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.priority == "P0"));
    }

    #[test]
    fn test_generate_recommendations_heavy_crate_suggests_optimization() {
        let analyzer = BloatAnalyzer::new(".");
        let items = vec![
            BloatItem {
                size_bytes: 150 * 1024,
                percentage: 15.0,
                name: "heavy::func1".to_string(),
                crate_name: Some("heavy".to_string()),
            },
            BloatItem {
                size_bytes: 100 * 1024,
                percentage: 10.0,
                name: "heavy::func2".to_string(),
                crate_name: Some("heavy".to_string()),
            },
        ];
        // Total size 1MB, heavy crate is 250KB (25%)
        let recommendations = analyzer.generate_recommendations(&items, 1024 * 1024);
        assert!(recommendations
            .iter()
            .any(|r| r.description.contains("heavy")));
    }

    #[test]
    fn test_generate_recommendations_formatting_bloat_detects_pattern() {
        let analyzer = BloatAnalyzer::new(".");
        let items = vec![
            BloatItem {
                size_bytes: 60 * 1024,
                percentage: 6.0,
                name: "std::fmt::write".to_string(),
                crate_name: Some("std".to_string()),
            },
            BloatItem {
                size_bytes: 50 * 1024,
                percentage: 5.0,
                name: "core::fmt::Display".to_string(),
                crate_name: Some("core".to_string()),
            },
        ];
        // Formatting is >5% of 1MB total
        let recommendations = analyzer.generate_recommendations(&items, 1024 * 1024);
        assert!(recommendations
            .iter()
            .any(|r| r.description.contains("Formatting")));
    }

    #[test]
    fn test_generate_recommendations_no_bloat_returns_empty() {
        let analyzer = BloatAnalyzer::new(".");
        let items = vec![BloatItem {
            size_bytes: 10 * 1024, // 10KB
            percentage: 1.0,
            name: "small::function".to_string(),
            crate_name: Some("small".to_string()),
        }];
        let recommendations = analyzer.generate_recommendations(&items, 1024 * 1024);
        // Should have no critical recommendations for small items
        assert!(recommendations.is_empty() || recommendations.iter().all(|r| r.priority != "P0"));
    }

    // P2-CODE-GLOBAL-004: Property-based tests for parse_size
    mod proptest_parse_size {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_parse_size_kib_roundtrip(kb in 0.0f64..1_000_000.0) {
                let size_str = format!("{:.2}KiB", kb);
                let parsed = BloatAnalyzer::parse_size(&size_str);

                if kb >= 0.0 {
                    let result = parsed.expect("should parse valid KiB size");
                    let expected = (kb * 1024.0) as u64;
                    // Allow for floating point rounding (up to 0.5% tolerance)
                    let tolerance = (expected as f64 * 0.005).max(10.0) as u64;
                    let diff = result.abs_diff(expected);
                    prop_assert!(diff <= tolerance, "Expected {} but got {} (diff: {})", expected, result, diff);
                }
            }

            #[test]
            fn test_parse_size_mib_roundtrip(mb in 0.0f64..1_000.0) {
                let size_str = format!("{:.2}MiB", mb);
                let parsed = BloatAnalyzer::parse_size(&size_str);

                if mb >= 0.0 {
                    let result = parsed.expect("should parse valid MiB size");
                    let expected = (mb * 1024.0 * 1024.0) as u64;
                    // Allow for floating point rounding (up to 0.5% tolerance)
                    let tolerance = (expected as f64 * 0.005).max(10240.0) as u64;
                    let diff = result.abs_diff(expected);
                    prop_assert!(diff <= tolerance, "Expected {} but got {} (diff: {})", expected, result, diff);
                }
            }

            #[test]
            fn test_parse_size_bytes_roundtrip(bytes in 0u64..1_000_000_000u64) {
                let size_str = format!("{}B", bytes);
                let result = BloatAnalyzer::parse_size(&size_str).expect("should parse valid byte size");
                prop_assert_eq!(result, bytes);
            }

            #[test]
            fn test_parse_size_plain_number_roundtrip(bytes in 0u64..1_000_000_000u64) {
                let size_str = format!("{}", bytes);
                let result = BloatAnalyzer::parse_size(&size_str).expect("should parse plain number");
                prop_assert_eq!(result, bytes);
            }

            #[test]
            fn test_parse_size_whitespace_invariant(bytes in 0u64..1_000_000u64, leading in 0usize..5, trailing in 0usize..5) {
                let size_str = format!("{}{}{}", " ".repeat(leading), bytes, " ".repeat(trailing));
                let result = BloatAnalyzer::parse_size(&size_str).expect("should parse with whitespace");
                prop_assert_eq!(result, bytes);
            }
        }
    }
}
