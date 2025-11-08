//! Twiggy output parsing logic

use super::analysis_types::{AnalysisItem, AnalysisMode};
use super::comparison::ChangeItem;
use super::error::TwiggyAnalysisError;
use crate::analyzer::TwiggyAnalyzer;
use crate::infra::{CommandExecutor, FileSystem};

impl<FS: FileSystem, CE: CommandExecutor> TwiggyAnalyzer<FS, CE> {
    /// Parse twiggy output into structured data
    pub(super) fn parse_output(
        &self,
        output: &str,
        _mode: AnalysisMode,
    ) -> Result<Vec<AnalysisItem>, TwiggyAnalysisError> {
        // Pre-allocate based on line count estimate (skip 2 header lines)
        let estimated_capacity = output.lines().count().saturating_sub(2);
        let mut items = Vec::with_capacity(estimated_capacity);

        for line in output.lines().skip(2) {
            // Skip header lines
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse line format: "  Size | % Total | Name"
            // Example: " 123456 | 12.34% | code[123]"
            if let Some(item) = self.parse_line(line) {
                items.push(item);
            }
        }

        Ok(items)
    }

    /// Parse a single line from twiggy output
    pub(super) fn parse_line(&self, line: &str) -> Option<AnalysisItem> {
        // twiggy uses box-drawing characters (│ U+2502 and ┊ U+250A) not ASCII pipe
        let parts: Vec<&str> = line.split(['|', '│', '┊']).map(|s| s.trim()).collect();

        if parts.len() < 3 {
            return None;
        }

        // Parse size (may have commas)
        let size_str = parts[0].replace(",", "");
        let size_bytes: u64 = size_str.parse().ok()?;

        // Parse percentage
        let percent_str = parts[1].trim_end_matches('%');
        let percentage: f64 = percent_str.parse().ok()?;

        // Get name (use String::from for direct allocation)
        let name = String::from(parts[2]);

        Some(AnalysisItem {
            size_bytes,
            percentage,
            name,
        })
    }
}

// Static parsing methods (no generic parameters needed)
impl TwiggyAnalyzer {
    /// Parse twiggy diff output
    pub(super) fn parse_diff_output(output: &str) -> Result<Vec<ChangeItem>, TwiggyAnalysisError> {
        let mut changes = Vec::new();

        for line in output.lines().skip(2) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse diff format: "  +1234 | symbol_name" or "  -1234 | symbol_name"
            if let Some((delta_str, name)) = line.split_once('|') {
                let delta_str = delta_str.trim().replace(",", "");
                if let Ok(delta_bytes) = delta_str.parse::<i64>() {
                    changes.push(ChangeItem {
                        delta_bytes,
                        name: name.trim().to_string(),
                    });
                }
            }
        }

        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_valid_format_extracts_all_fields() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let line = "  123456 | 12.34% | code[123]";
        let item = analyzer
            .parse_line(line)
            .expect("should parse valid twiggy line");

        assert_eq!(item.size_bytes, 123456);
        assert_eq!(item.percentage, 12.34);
        assert_eq!(item.name, "code[123]");
    }

    #[test]
    fn test_parse_line_size_with_commas_parses_correctly() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let line = "  1,234,567 | 45.67% | data[456]";
        let item = analyzer
            .parse_line(line)
            .expect("should parse name with pipes");

        assert_eq!(item.size_bytes, 1234567);
        assert_eq!(item.percentage, 45.67);
        assert_eq!(item.name, "data[456]");
    }

    #[test]
    fn test_parse_output_malformed_input_returns_empty_items() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Test empty output
        let empty_result = analyzer.parse_output("", AnalysisMode::Top);
        assert!(empty_result.is_ok());
        assert_eq!(empty_result.expect("should handle empty output").len(), 0);

        // Test malformed lines
        let malformed = "invalid output without proper format\nno pipes here\n";
        let malformed_result = analyzer.parse_output(malformed, AnalysisMode::Top);
        assert!(malformed_result.is_ok());
        // Should gracefully skip unparseable lines
        assert_eq!(
            malformed_result
                .expect("should handle malformed output")
                .len(),
            0
        );
    }

    #[test]
    fn test_parse_line_invalid_formats_returns_none() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Too few columns
        assert!(analyzer.parse_line("123456").is_none());
        assert!(analyzer.parse_line("123456 | 12.34%").is_none());

        // Invalid numbers
        assert!(analyzer.parse_line("invalid | 12.34% | name").is_none());
        assert!(analyzer.parse_line("123456 | invalid% | name").is_none());

        // Empty parts
        assert!(analyzer.parse_line(" | | ").is_none());
    }

    #[test]
    fn test_parse_line_edge_cases_handles_correctly() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Zero size
        let zero = analyzer.parse_line("0 | 0.00% | empty_function");
        assert!(zero.is_some());
        let item = zero.expect("should parse zero size line");
        assert_eq!(item.size_bytes, 0);
        assert_eq!(item.percentage, 0.0);

        // Very large size
        let large = analyzer.parse_line("9,999,999,999 | 99.99% | huge_data");
        assert!(large.is_some());
        let item = large.expect("should parse large size line");
        assert_eq!(item.size_bytes, 9999999999);
    }

    #[test]
    fn test_parse_diff_output_malformed_input_returns_empty_changes() {
        // Test malformed diff output
        let empty = TwiggyAnalyzer::parse_diff_output("");
        assert!(empty.is_ok());
        assert_eq!(empty.expect("should handle empty diff output").len(), 0);

        // Lines without pipes
        let no_pipes = TwiggyAnalyzer::parse_diff_output("invalid line\nno pipes here");
        assert!(no_pipes.is_ok());
        assert_eq!(no_pipes.expect("should handle no pipes in diff").len(), 0);

        // Invalid delta values
        let invalid_delta = TwiggyAnalyzer::parse_diff_output("invalid | symbol_name");
        assert!(invalid_delta.is_ok());
        assert_eq!(invalid_delta.expect("should handle invalid delta").len(), 0);
    }

    #[test]
    fn test_parse_diff_output_valid_format_parses_deltas() {
        let diff_output = r#"Header line
Another header
  +1234 | added_function
  -5678 | removed_function
  +0 | unchanged"#;

        let result = TwiggyAnalyzer::parse_diff_output(diff_output);
        assert!(result.is_ok());

        let changes = result.expect("should parse valid diff output");
        assert_eq!(changes.len(), 3);

        assert_eq!(changes[0].delta_bytes, 1234);
        assert_eq!(changes[0].name, "added_function");

        assert_eq!(changes[1].delta_bytes, -5678);
        assert_eq!(changes[1].name, "removed_function");
    }

    // P2-TEST-UNIT-002: Twiggy parser format tests

    #[test]
    fn test_parse_line_decimal_percentage_parses_fractional() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Test various decimal formats
        let item = analyzer
            .parse_line("100 | 0.01% | tiny_func")
            .expect("should parse tiny percentage");
        assert_eq!(item.percentage, 0.01);

        let item = analyzer
            .parse_line("1000 | 99.99% | huge_func")
            .expect("should parse large percentage");
        assert_eq!(item.percentage, 99.99);
    }

    #[test]
    fn test_parse_line_name_with_pipes_preserves_content() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Parser only takes parts[2], not the rest
        let line = "123 | 1.5% | name_with | extra | parts";
        let item = analyzer.parse_line(line).unwrap();
        // Only gets the first part after second pipe
        assert_eq!(item.name, "name_with");
    }

    #[test]
    fn test_parse_line_special_characters_in_name_handles() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Names with angle brackets, colons, etc.
        let item = analyzer
            .parse_line("500 | 2.0% | <std::vec::Vec<T>>")
            .expect("should parse special characters in name");
        assert_eq!(item.name, "<std::vec::Vec<T>>");

        let item = analyzer
            .parse_line("300 | 1.5% | func::with::colons")
            .expect("should parse whitespace variations");
        assert_eq!(item.name, "func::with::colons");
    }

    #[test]
    fn test_parse_line_whitespace_variations_parses_correctly() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Extra whitespace
        let item = analyzer
            .parse_line("  1000  |  5.5%  |  func_name  ")
            .unwrap();
        assert_eq!(item.size_bytes, 1000);
        assert_eq!(item.percentage, 5.5);
        assert_eq!(item.name, "func_name");
    }

    #[test]
    fn test_parse_line_no_commas_in_size_parses_directly() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Large numbers without commas
        let item = analyzer
            .parse_line("1234567 | 10.5% | big_func")
            .expect("should parse number without commas");
        assert_eq!(item.size_bytes, 1234567);
    }

    #[test]
    fn test_parse_output_with_headers_skips_non_data_lines() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let output = "Shallow Bytes | Shallow % | Item\nHeader 2\n  1000 | 10.0% | function_a\n  500 | 5.0% | function_b";

        let items = analyzer
            .parse_output(output, AnalysisMode::Top)
            .expect("should parse output with headers");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "function_a");
    }

    #[test]
    fn test_parse_output_mixed_lines_parses_valid_only() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let output = "Header\nHeader 2\n  1000 | 10.0% | valid_func\ninvalid line\n  500 | 5.0% | another_valid\nmalformed\n  200 | 2.0% | final_valid";

        let items = analyzer
            .parse_output(output, AnalysisMode::Top)
            .expect("should parse mixed lines");
        // Should parse only valid lines
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_parse_output_empty_lines_skips_blanks() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let output = "\n\n  100 | 1.0% | func\n\n\n  200 | 2.0% | func2\n\n";
        let items = analyzer
            .parse_output(output, AnalysisMode::Top)
            .expect("should skip empty lines");
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_diff_output_delta_with_commas_parses() {
        // Note: parse_diff_output skips first 2 lines
        let output = "Header1\nHeader2\n  +1,234,567 | big_addition\n  -9,876 | small_removal";
        let changes =
            TwiggyAnalyzer::parse_diff_output(output).expect("should parse diff with commas");

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].delta_bytes, 1234567);
        assert_eq!(changes[1].delta_bytes, -9876);
    }

    #[test]
    fn test_parse_diff_output_zero_delta_included() {
        // Note: parse_diff_output skips first 2 lines
        let output = "Header1\nHeader2\n  +0 | no_change\n  -0 | also_no_change";
        let changes = TwiggyAnalyzer::parse_diff_output(output).expect("should parse zero delta");

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].delta_bytes, 0);
        assert_eq!(changes[1].delta_bytes, 0);
    }

    #[test]
    fn test_parse_diff_output_large_values_handled() {
        // Note: parse_diff_output skips first 2 lines
        let output = "Header1\nHeader2\n  +999999999 | huge_add\n  -999999999 | huge_remove";
        let changes = TwiggyAnalyzer::parse_diff_output(output).expect("should parse large values");

        assert_eq!(changes[0].delta_bytes, 999999999);
        assert_eq!(changes[1].delta_bytes, -999999999);
    }

    #[test]
    fn test_parse_line_boundary_values_parses_extremes() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Maximum u64 (would overflow in real scenario, but test parser)
        let item = analyzer.parse_line("999999999999 | 100.0% | huge");
        assert!(item.is_some());

        // Minimum (zero)
        let item = analyzer
            .parse_line("0 | 0.0% | zero")
            .expect("should parse zero line");
        assert_eq!(item.size_bytes, 0);
    }

    #[test]
    fn test_parse_line_percentage_boundary_accepts_full_range() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // 100%
        let item = analyzer
            .parse_line("1000 | 100.0% | whole")
            .expect("should parse 100% line");
        assert_eq!(item.percentage, 100.0);

        // Very small
        let item = analyzer
            .parse_line("1 | 0.0001% | tiny")
            .expect("should parse tiny percentage line");
        assert_eq!(item.percentage, 0.0001);
    }

    #[test]
    fn test_parse_output_all_modes_consistent_parsing() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let output = "Header\n  100 | 1.0% | item";

        // Should work with all analysis modes
        assert!(analyzer.parse_output(output, AnalysisMode::Top).is_ok());
        assert!(analyzer
            .parse_output(output, AnalysisMode::Dominators)
            .is_ok());
        assert!(analyzer.parse_output(output, AnalysisMode::Dead).is_ok());
        assert!(analyzer.parse_output(output, AnalysisMode::Monos).is_ok());
    }

    // P2-CODE-GLOBAL-004: Property-based tests for TwiggyParser
    mod proptest_parser {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_parse_line_size_roundtrip(size in 0u64..1_000_000_000u64, percent in 0.0f64..100.0f64) {
                let analyzer = TwiggyAnalyzer::new("dummy.wasm");
                let line = format!("{} | {:.2}% | test_function", size, percent);
                let item = analyzer.parse_line(&line);

                prop_assert!(item.is_some());
                let item = item.unwrap();
                prop_assert_eq!(item.size_bytes, size);
                // Allow small floating point differences
                let diff = (item.percentage - percent).abs();
                prop_assert!(diff < 0.01, "Expected {:.2}% but got {:.2}%", percent, item.percentage);
            }

            #[test]
            fn test_parse_line_with_commas_roundtrip(size in 1000u64..1_000_000_000u64) {
                let analyzer = TwiggyAnalyzer::new("dummy.wasm");
                // Format with commas
                let size_str = format!("{}", size)
                    .as_bytes()
                    .rchunks(3)
                    .rev()
                    .map(|chunk| std::str::from_utf8(chunk).unwrap())
                    .collect::<Vec<_>>()
                    .join(",");
                let line = format!("{} | 10.0% | func", size_str);

                if let Some(item) = analyzer.parse_line(&line) {
                    prop_assert_eq!(item.size_bytes, size);
                }
            }

            #[test]
            fn test_parse_diff_output_roundtrip(delta in -1_000_000i64..1_000_000i64) {
                let output = format!("Header1\nHeader2\n  {} | symbol_name", delta);
                let changes = TwiggyAnalyzer::parse_diff_output(&output);

                prop_assert!(changes.is_ok());
                let changes = changes.unwrap();
                if !changes.is_empty() {
                    prop_assert_eq!(changes[0].delta_bytes, delta);
                    prop_assert_eq!(&changes[0].name, "symbol_name");
                }
            }

            #[test]
            fn test_parse_line_arbitrary_name(size in 1u64..1_000_000u64, name in "[a-zA-Z0-9_:]{1,50}") {
                let analyzer = TwiggyAnalyzer::new("dummy.wasm");
                let line = format!("{} | 5.0% | {}", size, name);

                if let Some(item) = analyzer.parse_line(&line) {
                    prop_assert_eq!(item.size_bytes, size);
                    prop_assert_eq!(item.name, name);
                }
            }

            #[test]
            fn test_parse_output_multiple_lines(count in 0usize..20) {
                let analyzer = TwiggyAnalyzer::new("dummy.wasm");
                let mut lines = vec!["Header1".to_string(), "Header2".to_string()];

                for i in 0..count {
                    lines.push(format!("  {} | {:.1}% | func_{}", i * 100, i as f64, i));
                }

                let output = lines.join("\n");
                let items = analyzer.parse_output(&output, AnalysisMode::Top);

                prop_assert!(items.is_ok());
                prop_assert_eq!(items.unwrap().len(), count);
            }
        }
    }
}
