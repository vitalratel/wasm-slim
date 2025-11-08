//! Tests for test assertion helpers
//!
//! Verifies that the common assertion utilities work correctly.

mod common;

use common::assertions::*;

#[test]
fn test_approx_eq_within_tolerance() {
    assert_approx_eq(0.75, 0.75, 0.01);
    assert_approx_eq(0.75, 0.7501, 0.01);
    assert_approx_eq(12.5, 12.45, 0.1);
}

#[test]
fn test_size_within_tolerance() {
    assert_size_within(1024000, 1024000, 1024);
    assert_size_within(1024000, 1024512, 1024);
    assert_size_within(1024512, 1024000, 1024);
}

#[test]
fn test_percent_within_tolerance() {
    assert_percent_within(50.0, 50.0, 1.0);
    assert_percent_within(50.0, 50.5, 1.0);
    assert_percent_within(12.5, 12.3, 0.5);
}

#[test]
fn test_helpers_work_in_real_scenario() {
    // Simulate a size reduction calculation
    let before: u64 = 1048576; // 1MB
    let after: u64 = 524288; // 512KB

    let reduction_percent = ((before - after) as f64 / before as f64) * 100.0;

    // Use approximate comparison for the percentage
    assert_approx_eq(reduction_percent, 50.0, 0.01);

    // Use size comparison for the after size
    assert_size_within(after, 524288, 0); // Exact in this case
}
