//! Test assertion helpers
//!
//! Provides utilities for robust test assertions, particularly for floating-point
//! comparisons and size validations that may vary slightly across platforms or
//! optimization levels.

/// Assert that two floating-point values are approximately equal
///
/// Use this instead of `assert_eq!` for f64/f32 comparisons to avoid
/// floating-point precision issues.
///
/// # Arguments
/// * `actual` - The actual value
/// * `expected` - The expected value
/// * `epsilon` - Maximum allowed difference (default: 0.01 or 1%)
///
/// # Examples
///
/// ```
/// # use wasm_slim_tests::assertions::assert_approx_eq;
/// // Allow ±1% tolerance for percentages
/// assert_approx_eq(ratio, 0.75, 0.01);
///
/// // Allow ±0.5% for tighter comparisons
/// assert_approx_eq(percentage, 12.5, 0.0625);
/// ```
#[allow(dead_code)]
pub fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff < epsilon,
        "Float values not approximately equal:\n  actual: {}\n  expected: {}\n  diff: {} (epsilon: {})",
        actual, expected, diff, epsilon
    );
}

/// Assert that a size value is within tolerance of expected
///
/// Useful for size calculations that may vary slightly due to:
/// - Platform differences (32-bit vs 64-bit)
/// - Compiler optimization levels
/// - Small code changes not affecting functionality
///
/// # Arguments
/// * `actual_bytes` - The actual size in bytes
/// * `expected_bytes` - The expected size in bytes
/// * `tolerance_bytes` - Maximum allowed difference
///
/// # Examples
///
/// ```
/// # use wasm_slim_tests::assertions::assert_size_within;
/// // Allow ±1KB variance for bundle sizes
/// assert_size_within(actual_size, expected_size, 1024);
///
/// // Allow ±10KB for larger bundles
/// assert_size_within(bundle_size, 1048576, 10240);
/// ```
#[allow(dead_code)]
pub fn assert_size_within(actual_bytes: u64, expected_bytes: u64, tolerance_bytes: u64) {
    let diff = actual_bytes.abs_diff(expected_bytes);

    assert!(
        diff <= tolerance_bytes,
        "Size outside tolerance:\n  actual: {} bytes ({:.2} KB)\n  expected: {} bytes ({:.2} KB)\n  diff: {} bytes (tolerance: {} bytes)",
        actual_bytes, actual_bytes as f64 / 1024.0,
        expected_bytes, expected_bytes as f64 / 1024.0,
        diff, tolerance_bytes
    );
}

/// Assert that a percentage value is within tolerance
///
/// Convenience wrapper for percentage comparisons.
///
/// # Arguments
/// * `actual_percent` - The actual percentage (0-100 scale)
/// * `expected_percent` - The expected percentage
/// * `tolerance_percent` - Maximum allowed difference (default: 1.0 for ±1%)
///
/// # Examples
///
/// ```
/// # use wasm_slim_tests::assertions::assert_percent_within;
/// // Allow ±2% tolerance
/// assert_percent_within(reduction, 50.0, 2.0);
/// ```
#[allow(dead_code)]
pub fn assert_percent_within(actual_percent: f64, expected_percent: f64, tolerance_percent: f64) {
    assert_approx_eq(actual_percent, expected_percent, tolerance_percent);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_approx_eq_pass() {
        assert_approx_eq(0.75, 0.75, 0.01);
        assert_approx_eq(0.75, 0.7501, 0.01);
        assert_approx_eq(12.5, 12.45, 0.1);
    }

    #[test]
    #[should_panic(expected = "Float values not approximately equal")]
    fn test_assert_approx_eq_fail() {
        assert_approx_eq(0.75, 0.80, 0.01);
    }

    #[test]
    fn test_assert_size_within_pass() {
        assert_size_within(1024000, 1024000, 1024);
        assert_size_within(1024000, 1024512, 1024);
        assert_size_within(1024512, 1024000, 1024);
    }

    #[test]
    #[should_panic(expected = "Size outside tolerance")]
    fn test_assert_size_within_fail() {
        assert_size_within(1024000, 1030000, 1024);
    }

    #[test]
    fn test_assert_percent_within_pass() {
        assert_percent_within(50.0, 50.0, 1.0);
        assert_percent_within(50.0, 50.5, 1.0);
    }
}
