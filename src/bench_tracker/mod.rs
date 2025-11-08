//! Performance tracking infrastructure for benchmark results
//!
//! This module provides baseline tracking, regression detection, and performance
//! budget management for criterion benchmarks.

mod comparator;
mod parser;
mod reporter;
mod storage;

pub use comparator::{BenchmarkComparator, BenchmarkComparison, PerformanceBudget};
pub use parser::BenchmarkParser;
pub use reporter::BenchmarkReporter;
pub use storage::{BenchmarkBaseline, BenchmarkResult, BenchmarkStorage};

use crate::infra::{FileSystem, RealFileSystem};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

/// Performance tracking manager
pub struct BenchmarkTracker<FS: FileSystem = RealFileSystem> {
    storage: BenchmarkStorage<FS>,
    parser: BenchmarkParser<FS>,
    comparator: BenchmarkComparator,
    reporter: BenchmarkReporter,
}

impl BenchmarkTracker<RealFileSystem> {
    /// Create a new benchmark tracker with the real filesystem
    pub fn new(project_root: &Path) -> Self {
        Self::with_budget(project_root, PerformanceBudget::default())
    }

    /// Create a new benchmark tracker with custom budget and real filesystem
    pub fn with_budget(project_root: &Path, budget: PerformanceBudget) -> Self {
        Self::with_fs_and_budget(project_root, RealFileSystem, budget)
    }
}

impl<FS: FileSystem> BenchmarkTracker<FS> {
    /// Create a new benchmark tracker with a custom filesystem implementation
    pub fn with_fs(project_root: &Path, fs: FS) -> Self
    where
        FS: Clone,
    {
        Self::with_fs_and_budget(project_root, fs, PerformanceBudget::default())
    }

    /// Create a new benchmark tracker with custom filesystem and budget
    pub fn with_fs_and_budget(project_root: &Path, fs: FS, budget: PerformanceBudget) -> Self
    where
        FS: Clone,
    {
        Self {
            storage: BenchmarkStorage::new(project_root, fs.clone()),
            parser: BenchmarkParser::new(fs),
            comparator: BenchmarkComparator::new(budget),
            reporter: BenchmarkReporter::new(),
        }
    }

    /// Load the current baseline
    pub fn load_baseline(&self) -> Result<Option<BenchmarkBaseline>> {
        self.storage.load_baseline()
    }

    /// Save a new baseline
    pub fn save_baseline(&self, baseline: BenchmarkBaseline) -> Result<()> {
        self.storage.save_baseline(baseline)
    }

    /// Parse criterion benchmark output from a directory
    pub fn parse_criterion_results(
        &self,
        criterion_dir: &Path,
    ) -> Result<HashMap<String, BenchmarkResult>> {
        self.parser.parse_criterion_results(criterion_dir)
    }

    /// Compare current results against baseline
    pub fn compare_with_baseline(
        &self,
        current: &HashMap<String, BenchmarkResult>,
        baseline: &BenchmarkBaseline,
    ) -> Vec<BenchmarkComparison> {
        self.comparator.compare_with_baseline(current, baseline)
    }

    /// Print comparison results
    pub fn print_comparison(&self, comparisons: &[BenchmarkComparison]) {
        self.reporter.print_comparison(comparisons)
    }

    /// Check if any regressions were detected
    pub fn has_regressions(&self, comparisons: &[BenchmarkComparison]) -> bool {
        self.comparator.has_regressions(comparisons)
    }

    /// Get the current git commit hash
    fn get_git_commit() -> Option<String> {
        crate::git::GitRepository::new()
            .get_commit_hash()
            .ok()
            .flatten()
    }

    /// Create a baseline from current results
    pub fn create_baseline(
        &self,
        results: HashMap<String, BenchmarkResult>,
        version: String,
    ) -> Result<BenchmarkBaseline> {
        Ok(BenchmarkBaseline {
            version,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .context("System clock is before Unix epoch")?
                .as_secs(),
            git_commit: Self::get_git_commit(),
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_baseline_preserves_data() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tracker = BenchmarkTracker::new(temp_dir.path());

        let mut results = HashMap::new();
        results.insert(
            "benchmark_1".to_string(),
            BenchmarkResult {
                name: "benchmark_1".to_string(),
                mean_ns: 1_000_000,
                stddev_ns: 50_000,
                min_ns: 900_000,
                max_ns: 1_100_000,
                iterations: 100,
                timestamp: 1234567890,
            },
        );

        let baseline = tracker
            .create_baseline(results.clone(), "v1.0.0".to_string())
            .expect("Failed to create baseline");

        tracker
            .save_baseline(baseline)
            .expect("Failed to save baseline");

        let loaded = tracker
            .load_baseline()
            .expect("Failed to load baseline")
            .expect("Baseline should exist");

        assert_eq!(loaded.version, "v1.0.0");
        assert_eq!(loaded.results.len(), 1);
        assert_eq!(
            loaded.results.get("benchmark_1").unwrap().mean_ns,
            1_000_000
        );
    }

    #[test]
    fn test_load_baseline_returns_none_when_file_missing() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tracker = BenchmarkTracker::new(temp_dir.path());

        let result = tracker
            .load_baseline()
            .expect("Should not error on missing file");
        assert!(result.is_none());
    }

    #[test]
    fn test_regression_detection_at_threshold_boundaries() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let budget = PerformanceBudget {
            max_regression_percent: 10.0,
            max_time_ns: None,
            fail_on_violation: false,
        };
        let tracker = BenchmarkTracker::with_budget(temp_dir.path(), budget);

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

        let comparisons = tracker.compare_with_baseline(&current_at_threshold, &baseline);
        assert_eq!(comparisons.len(), 1);
        assert!(
            !comparisons[0].is_regression,
            "Exactly at threshold should not be regression"
        );

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

        let comparisons = tracker.compare_with_baseline(&current_over_threshold, &baseline);
        assert_eq!(comparisons.len(), 1);
        assert!(
            comparisons[0].is_regression,
            "Over threshold should be regression"
        );
    }

    #[test]
    fn test_compare_with_baseline_handles_new_benchmarks() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tracker = BenchmarkTracker::new(temp_dir.path());

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

        let comparisons = tracker.compare_with_baseline(&current_results, &baseline);

        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].name, "old_benchmark");
    }

    #[test]
    fn test_parse_criterion_results_handles_missing_data() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tracker = BenchmarkTracker::new(temp_dir.path());

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        let bench_dir = criterion_dir.join("test_bench");
        std::fs::create_dir_all(&bench_dir).expect("Failed to create bench dir");

        let results = tracker
            .parse_criterion_results(&criterion_dir)
            .expect("Should handle missing estimates gracefully");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_compare_with_baseline_detects_improvements() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tracker = BenchmarkTracker::new(temp_dir.path());

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
                mean_ns: 800_000,
                stddev_ns: 10_000,
                min_ns: 700_000,
                max_ns: 900_000,
                iterations: 100,
                timestamp: 1234567891,
            },
        );

        let comparisons = tracker.compare_with_baseline(&current_results, &baseline);
        assert_eq!(comparisons.len(), 1);
        assert!(!comparisons[0].is_regression);
        assert!(
            comparisons[0].change_percent < 0.0,
            "Should show negative change for improvement"
        );
    }

    #[test]
    fn test_budget_with_max_time_ns_detects_violations() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let budget = PerformanceBudget {
            max_regression_percent: 10.0,
            max_time_ns: Some(2_000_000),
            fail_on_violation: true,
        };
        let tracker = BenchmarkTracker::with_budget(temp_dir.path(), budget);

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

        let comparisons = tracker.compare_with_baseline(&current_results, &baseline);
        assert_eq!(comparisons.len(), 1);
        assert!(
            comparisons[0].exceeds_budget,
            "Should detect budget violation"
        );
        assert!(
            tracker.has_regressions(&comparisons),
            "Should report regression due to budget violation"
        );
    }

    #[test]
    fn test_has_regressions_with_mixed_results() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tracker = BenchmarkTracker::new(temp_dir.path());

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
            tracker.has_regressions(&comparisons),
            "Should detect regression in mixed results"
        );
    }
}
