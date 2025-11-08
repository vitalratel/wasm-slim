//! Benchmark comparison logic

use super::storage::{BenchmarkBaseline, BenchmarkResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Performance budget thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBudget {
    /// Maximum allowed regression percentage (e.g., 10.0 for 10%)
    pub max_regression_percent: f64,
    /// Optional absolute time budget in nanoseconds
    pub max_time_ns: Option<u64>,
    /// Whether to fail on budget violations
    pub fail_on_violation: bool,
}

impl Default for PerformanceBudget {
    fn default() -> Self {
        Self {
            max_regression_percent: 10.0,
            max_time_ns: None,
            fail_on_violation: false,
        }
    }
}

/// Comparison between current and baseline results
#[derive(Debug)]
pub struct BenchmarkComparison {
    /// Name of the benchmark
    pub name: String,
    /// Baseline mean time in nanoseconds
    pub baseline_mean_ns: u64,
    /// Current mean time in nanoseconds
    pub current_mean_ns: u64,
    /// Percent change from baseline (positive = slower)
    pub change_percent: f64,
    /// True if performance regressed significantly
    pub is_regression: bool,
    /// True if budget limit was exceeded
    pub exceeds_budget: bool,
}

/// Compares benchmark results against baselines
pub struct BenchmarkComparator {
    budget: PerformanceBudget,
}

impl BenchmarkComparator {
    /// Create a new benchmark comparator
    pub fn new(budget: PerformanceBudget) -> Self {
        Self { budget }
    }

    /// Create with default budget
    pub fn with_default_budget() -> Self {
        Self::new(PerformanceBudget::default())
    }

    /// Compare current results against baseline
    pub fn compare_with_baseline(
        &self,
        current: &HashMap<String, BenchmarkResult>,
        baseline: &BenchmarkBaseline,
    ) -> Vec<BenchmarkComparison> {
        let mut comparisons = Vec::new();

        for (name, current_result) in current {
            if let Some(baseline_result) = baseline.results.get(name) {
                let change_percent = ((current_result.mean_ns as f64
                    - baseline_result.mean_ns as f64)
                    / baseline_result.mean_ns as f64)
                    * 100.0;

                let is_regression =
                    change_percent > 0.0 && change_percent > self.budget.max_regression_percent;

                let exceeds_budget = if let Some(max_time) = self.budget.max_time_ns {
                    current_result.mean_ns > max_time
                } else {
                    false
                };

                comparisons.push(BenchmarkComparison {
                    name: name.clone(),
                    baseline_mean_ns: baseline_result.mean_ns,
                    current_mean_ns: current_result.mean_ns,
                    change_percent,
                    is_regression,
                    exceeds_budget,
                });
            }
        }

        comparisons
    }

    /// Check if any regressions were detected
    pub fn has_regressions(&self, comparisons: &[BenchmarkComparison]) -> bool {
        comparisons
            .iter()
            .any(|c| c.is_regression || c.exceeds_budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_budget_default_has_expected_values() {
        let budget = PerformanceBudget::default();
        assert_eq!(budget.max_regression_percent, 10.0);
        assert_eq!(budget.max_time_ns, None);
        assert!(!budget.fail_on_violation);
    }

    #[test]
    fn test_regression_detection_at_threshold_boundaries() {
        let budget = PerformanceBudget {
            max_regression_percent: 10.0,
            max_time_ns: None,
            fail_on_violation: false,
        };
        let comparator = BenchmarkComparator::new(budget);

        let mut baseline_results = HashMap::new();
        baseline_results.insert(
            "test".to_string(),
            BenchmarkResult {
                name: "test".to_string(),
                mean_ns: 1_000_000,
                stddev_ns: 10_000,
                min_ns: 900_000,
                max_ns: 1_100_000,
                iterations: 100,
                timestamp: 1234567890,
            },
        );

        let baseline = BenchmarkBaseline {
            version: "v1.0.0".to_string(),
            timestamp: 1234567890,
            git_commit: None,
            results: baseline_results,
        };

        // Test at exactly threshold (10% slower = 1,100,000 ns)
        let mut current_at_threshold = HashMap::new();
        current_at_threshold.insert(
            "test".to_string(),
            BenchmarkResult {
                name: "test".to_string(),
                mean_ns: 1_100_000,
                stddev_ns: 10_000,
                min_ns: 1_000_000,
                max_ns: 1_200_000,
                iterations: 100,
                timestamp: 1234567891,
            },
        );

        let comparisons = comparator.compare_with_baseline(&current_at_threshold, &baseline);
        assert_eq!(comparisons.len(), 1);
        assert!(
            !comparisons[0].is_regression,
            "Exactly at threshold should not be regression"
        );

        // Test just over threshold (10.1% slower = 1,101,000 ns)
        let mut current_over_threshold = HashMap::new();
        current_over_threshold.insert(
            "test".to_string(),
            BenchmarkResult {
                name: "test".to_string(),
                mean_ns: 1_101_000,
                stddev_ns: 10_000,
                min_ns: 1_000_000,
                max_ns: 1_200_000,
                iterations: 100,
                timestamp: 1234567891,
            },
        );

        let comparisons = comparator.compare_with_baseline(&current_over_threshold, &baseline);
        assert_eq!(comparisons.len(), 1);
        assert!(
            comparisons[0].is_regression,
            "Over threshold should be regression"
        );
    }

    #[test]
    fn test_compare_with_baseline_handles_new_benchmarks() {
        let comparator = BenchmarkComparator::with_default_budget();

        let mut baseline_results = HashMap::new();
        baseline_results.insert(
            "old_benchmark".to_string(),
            BenchmarkResult {
                name: "old_benchmark".to_string(),
                mean_ns: 1_000_000,
                stddev_ns: 10_000,
                min_ns: 900_000,
                max_ns: 1_100_000,
                iterations: 100,
                timestamp: 1234567890,
            },
        );

        let baseline = BenchmarkBaseline {
            version: "v1.0.0".to_string(),
            timestamp: 1234567890,
            git_commit: None,
            results: baseline_results,
        };

        let mut current_results = HashMap::new();
        current_results.insert(
            "old_benchmark".to_string(),
            BenchmarkResult {
                name: "old_benchmark".to_string(),
                mean_ns: 1_000_000,
                stddev_ns: 10_000,
                min_ns: 900_000,
                max_ns: 1_100_000,
                iterations: 100,
                timestamp: 1234567891,
            },
        );
        current_results.insert(
            "new_benchmark".to_string(),
            BenchmarkResult {
                name: "new_benchmark".to_string(),
                mean_ns: 500_000,
                stddev_ns: 5_000,
                min_ns: 450_000,
                max_ns: 550_000,
                iterations: 100,
                timestamp: 1234567891,
            },
        );

        let comparisons = comparator.compare_with_baseline(&current_results, &baseline);

        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].name, "old_benchmark");
    }

    #[test]
    fn test_compare_with_baseline_detects_improvements() {
        let comparator = BenchmarkComparator::with_default_budget();

        let mut baseline_results = HashMap::new();
        baseline_results.insert(
            "test".to_string(),
            BenchmarkResult {
                name: "test".to_string(),
                mean_ns: 1_000_000,
                stddev_ns: 10_000,
                min_ns: 900_000,
                max_ns: 1_100_000,
                iterations: 100,
                timestamp: 1234567890,
            },
        );

        let baseline = BenchmarkBaseline {
            version: "v1.0.0".to_string(),
            timestamp: 1234567890,
            git_commit: None,
            results: baseline_results,
        };

        // 20% faster
        let mut current_results = HashMap::new();
        current_results.insert(
            "test".to_string(),
            BenchmarkResult {
                name: "test".to_string(),
                mean_ns: 800_000,
                stddev_ns: 10_000,
                min_ns: 700_000,
                max_ns: 900_000,
                iterations: 100,
                timestamp: 1234567891,
            },
        );

        let comparisons = comparator.compare_with_baseline(&current_results, &baseline);
        assert_eq!(comparisons.len(), 1);
        assert!(!comparisons[0].is_regression);
        assert!(
            comparisons[0].change_percent < 0.0,
            "Should show negative change for improvement"
        );
    }

    #[test]
    fn test_budget_with_max_time_ns_detects_violations() {
        let budget = PerformanceBudget {
            max_regression_percent: 10.0,
            max_time_ns: Some(2_000_000),
            fail_on_violation: true,
        };
        let comparator = BenchmarkComparator::new(budget);

        let mut baseline_results = HashMap::new();
        baseline_results.insert(
            "test".to_string(),
            BenchmarkResult {
                name: "test".to_string(),
                mean_ns: 1_000_000,
                stddev_ns: 10_000,
                min_ns: 900_000,
                max_ns: 1_100_000,
                iterations: 100,
                timestamp: 1234567890,
            },
        );

        let baseline = BenchmarkBaseline {
            version: "v1.0.0".to_string(),
            timestamp: 1234567890,
            git_commit: None,
            results: baseline_results,
        };

        let mut current_results = HashMap::new();
        current_results.insert(
            "test".to_string(),
            BenchmarkResult {
                name: "test".to_string(),
                mean_ns: 2_500_000,
                stddev_ns: 10_000,
                min_ns: 2_400_000,
                max_ns: 2_600_000,
                iterations: 100,
                timestamp: 1234567891,
            },
        );

        let comparisons = comparator.compare_with_baseline(&current_results, &baseline);
        assert_eq!(comparisons.len(), 1);
        assert!(
            comparisons[0].exceeds_budget,
            "Should detect budget violation"
        );
        assert!(
            comparator.has_regressions(&comparisons),
            "Should report regression due to budget violation"
        );
    }

    #[test]
    fn test_has_regressions_with_mixed_results() {
        let comparator = BenchmarkComparator::with_default_budget();

        let comparisons = vec![
            BenchmarkComparison {
                name: "fast".to_string(),
                baseline_mean_ns: 1_000_000,
                current_mean_ns: 800_000,
                change_percent: -20.0,
                is_regression: false,
                exceeds_budget: false,
            },
            BenchmarkComparison {
                name: "slow".to_string(),
                baseline_mean_ns: 1_000_000,
                current_mean_ns: 1_200_000,
                change_percent: 20.0,
                is_regression: true,
                exceeds_budget: false,
            },
        ];

        assert!(
            comparator.has_regressions(&comparisons),
            "Should detect regression in mixed results"
        );
    }
}
