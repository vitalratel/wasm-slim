//! Shared utilities for report formatting
//!
//! This module provides common formatting functions used across all report modules
//! (asset_report, bloat_report, twiggy_report, feature_report) to ensure consistency
//! and reduce code duplication.

/// Format bytes with appropriate unit (B, KB, MB, GB)
///
/// # Examples
///
/// ```
/// use wasm_slim::analyzer::report_utils::format_bytes;
///
/// assert_eq!(format_bytes(512), "512 B");
/// assert_eq!(format_bytes(2048), "2.00 KB");
/// assert_eq!(format_bytes(2_097_152), "2.00 MB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Truncate string with ellipsis if exceeds max length
///
/// # Examples
///
/// ```
/// use wasm_slim::analyzer::report_utils::truncate_str;
///
/// assert_eq!(truncate_str("short", 10), "short");
/// assert_eq!(truncate_str("very_long_symbol_name", 12), "very_long...");
/// ```
pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format number with thousand separators
///
/// # Examples
///
/// ```
/// use wasm_slim::analyzer::report_utils::format_number;
///
/// assert_eq!(format_number(1234567), "1,234,567");
/// ```
pub fn format_number(n: u64) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or("?"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Format percentage with 2 decimal places
///
/// # Examples
///
/// ```
/// use wasm_slim::analyzer::report_utils::format_percent;
///
/// assert_eq!(format_percent(42.567), "42.57%");
/// ```
pub fn format_percent(value: f64) -> String {
    format!("{:.2}%", value)
}

/// Serialize to JSON with pretty printing, fallback to empty object on error
///
/// # Examples
///
/// ```
/// use wasm_slim::analyzer::report_utils::to_json_string;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Data { value: i32 }
///
/// let data = Data { value: 42 };
/// let json = to_json_string(&data);
/// assert!(json.contains("\"value\""));
/// ```
pub fn to_json_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_with_bytes_shows_b_unit() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn test_format_bytes_with_kilobytes_shows_kb_unit() {
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(2048), "2.00 KB");
        assert_eq!(format_bytes(1024 * 512), "512.00 KB");
    }

    #[test]
    fn test_format_bytes_with_megabytes_shows_mb_unit() {
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(2 * 1024 * 1024), "2.00 MB");
    }

    #[test]
    fn test_format_bytes_with_gigabytes_shows_gb_unit() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_truncate_str_short_string_returns_unchanged() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("test", 4), "test");
    }

    #[test]
    fn test_truncate_str_long_string_adds_ellipsis() {
        assert_eq!(truncate_str("very_long_symbol_name", 12), "very_long...");
        assert_eq!(truncate_str("abcdefghij", 8), "abcde...");
    }

    #[test]
    fn test_truncate_str_max_len_less_than_3_handles_edge_case() {
        assert_eq!(truncate_str("test", 2), "...");
        assert_eq!(truncate_str("a", 1), "a");
    }

    #[test]
    fn test_format_number_adds_thousand_separators() {
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn test_format_percent_rounds_to_2_decimals() {
        assert_eq!(format_percent(42.567), "42.57%");
        assert_eq!(format_percent(100.0), "100.00%");
        assert_eq!(format_percent(0.123), "0.12%");
    }

    #[test]
    fn test_to_json_string_serializes_struct() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct Test {
            value: i32,
        }

        let test = Test { value: 42 };
        let json = to_json_string(&test);
        assert!(json.contains("\"value\""));
        assert!(json.contains("42"));
    }
}
