//! Criterion benchmark results parsing

use super::storage::BenchmarkResult;
use crate::infra::FileSystem;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

/// Parses criterion benchmark results
pub struct BenchmarkParser<FS: FileSystem> {
    fs: FS,
}

impl<FS: FileSystem> BenchmarkParser<FS> {
    /// Create a new benchmark parser
    pub fn new(fs: FS) -> Self {
        Self { fs }
    }

    /// Parse criterion benchmark output from a directory
    pub fn parse_criterion_results(
        &self,
        criterion_dir: &Path,
    ) -> Result<HashMap<String, BenchmarkResult>> {
        let mut results = HashMap::new();

        // Criterion stores results in target/criterion/<benchmark_name>/base/estimates.json
        for entry in self
            .fs
            .read_dir(criterion_dir)
            .context("Failed to read criterion directory")?
        {
            let entry = entry?;
            let bench_name = entry.file_name().to_string_lossy().to_string();

            let estimates_path = entry.path().join("base").join("estimates.json");
            if !estimates_path.exists() {
                continue;
            }

            let estimates_json = self
                .fs
                .read_to_string(&estimates_path)
                .context(format!("Failed to read estimates for {}", bench_name))?;
            let estimates: serde_json::Value = serde_json::from_str(&estimates_json)
                .context(format!("Failed to parse estimates for {}", bench_name))?;

            // Extract mean estimate
            if let Some(mean) = estimates.get("mean") {
                if let (Some(point_estimate), Some(std_err)) =
                    (mean.get("point_estimate"), mean.get("standard_error"))
                {
                    if let (Some(mean_ns), Some(stddev_ns)) =
                        (point_estimate.as_f64(), std_err.as_f64())
                    {
                        let result = BenchmarkResult {
                            name: bench_name.clone(),
                            mean_ns: mean_ns as u64,
                            stddev_ns: stddev_ns as u64,
                            min_ns: mean_ns as u64, // Approximation
                            max_ns: mean_ns as u64, // Approximation
                            iterations: 100,        // Default
                            timestamp: SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .context("System clock is before Unix epoch")?
                                .as_secs(),
                        };

                        results.insert(bench_name, result);
                    }
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::RealFileSystem;
    use tempfile::TempDir;

    #[test]
    fn test_parse_criterion_results_handles_missing_data() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        let bench_dir = criterion_dir.join("test_bench");
        std::fs::create_dir_all(&bench_dir).expect("Failed to create bench dir");

        let results = parser
            .parse_criterion_results(&criterion_dir)
            .expect("Should handle missing estimates gracefully");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_criterion_results_with_valid_data() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        let bench_dir = criterion_dir.join("test_bench");
        let base_dir = bench_dir.join("base");
        std::fs::create_dir_all(&base_dir).expect("Failed to create base dir");

        let estimates_json = r#"{
            "mean": {
                "point_estimate": 12345.67,
                "standard_error": 123.45
            }
        }"#;
        std::fs::write(base_dir.join("estimates.json"), estimates_json)
            .expect("Failed to write estimates");

        let results = parser
            .parse_criterion_results(&criterion_dir)
            .expect("Should parse valid estimates");
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("test_bench"));

        let result = &results["test_bench"];
        assert_eq!(result.name, "test_bench");
        assert_eq!(result.mean_ns, 12345);
        assert_eq!(result.stddev_ns, 123);
    }

    #[test]
    fn test_parse_criterion_results_multiple_benchmarks() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        // Create two benchmark results
        for bench_name in &["bench1", "bench2"] {
            let bench_dir = criterion_dir.join(bench_name);
            let base_dir = bench_dir.join("base");
            std::fs::create_dir_all(&base_dir).expect("Failed to create base dir");

            let estimates_json = r#"{
                "mean": {
                    "point_estimate": 5000.0,
                    "standard_error": 50.0
                }
            }"#;
            std::fs::write(base_dir.join("estimates.json"), estimates_json)
                .expect("Failed to write estimates");
        }

        let results = parser
            .parse_criterion_results(&criterion_dir)
            .expect("Should parse multiple benchmarks");
        assert_eq!(results.len(), 2);
        assert!(results.contains_key("bench1"));
        assert!(results.contains_key("bench2"));
    }

    #[test]
    fn test_parse_criterion_results_with_missing_mean() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        let bench_dir = criterion_dir.join("test_bench");
        let base_dir = bench_dir.join("base");
        std::fs::create_dir_all(&base_dir).expect("Failed to create base dir");

        // Missing "mean" field
        let estimates_json = r#"{"median": {"point_estimate": 1000.0}}"#;
        std::fs::write(base_dir.join("estimates.json"), estimates_json)
            .expect("Failed to write estimates");

        let results = parser
            .parse_criterion_results(&criterion_dir)
            .expect("Should handle missing mean gracefully");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_criterion_results_with_incomplete_mean() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        let bench_dir = criterion_dir.join("test_bench");
        let base_dir = bench_dir.join("base");
        std::fs::create_dir_all(&base_dir).expect("Failed to create base dir");

        // Missing "standard_error" field
        let estimates_json = r#"{"mean": {"point_estimate": 1000.0}}"#;
        std::fs::write(base_dir.join("estimates.json"), estimates_json)
            .expect("Failed to write estimates");

        let results = parser
            .parse_criterion_results(&criterion_dir)
            .expect("Should handle incomplete mean gracefully");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_criterion_results_with_invalid_json() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        let bench_dir = criterion_dir.join("test_bench");
        let base_dir = bench_dir.join("base");
        std::fs::create_dir_all(&base_dir).expect("Failed to create base dir");

        // Invalid JSON
        let estimates_json = "not valid json {";
        std::fs::write(base_dir.join("estimates.json"), estimates_json)
            .expect("Failed to write estimates");

        let result = parser.parse_criterion_results(&criterion_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_criterion_results_empty_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        let results = parser
            .parse_criterion_results(&criterion_dir)
            .expect("Should handle empty directory");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_criterion_results_mixed_valid_invalid() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        // Valid benchmark
        let bench1_dir = criterion_dir.join("valid_bench");
        let base1_dir = bench1_dir.join("base");
        std::fs::create_dir_all(&base1_dir).expect("Failed to create base dir");
        std::fs::write(
            base1_dir.join("estimates.json"),
            r#"{"mean": {"point_estimate": 1000.0, "standard_error": 10.0}}"#,
        )
        .expect("Failed to write estimates");

        // Invalid benchmark (missing estimates.json)
        let bench2_dir = criterion_dir.join("invalid_bench");
        std::fs::create_dir_all(&bench2_dir).expect("Failed to create bench dir");

        let results = parser
            .parse_criterion_results(&criterion_dir)
            .expect("Should parse valid benchmarks and skip invalid ones");
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("valid_bench"));
    }

    #[test]
    fn test_parse_criterion_results_with_non_numeric_values() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let parser = BenchmarkParser::new(RealFileSystem);

        let criterion_dir = temp_dir.path().join("criterion");
        std::fs::create_dir_all(&criterion_dir).expect("Failed to create criterion dir");

        let bench_dir = criterion_dir.join("test_bench");
        let base_dir = bench_dir.join("base");
        std::fs::create_dir_all(&base_dir).expect("Failed to create base dir");

        // Non-numeric values
        let estimates_json =
            r#"{"mean": {"point_estimate": "not a number", "standard_error": 10.0}}"#;
        std::fs::write(base_dir.join("estimates.json"), estimates_json)
            .expect("Failed to write estimates");

        let results = parser
            .parse_criterion_results(&criterion_dir)
            .expect("Should handle non-numeric values gracefully");
        assert_eq!(results.len(), 0);
    }
}
