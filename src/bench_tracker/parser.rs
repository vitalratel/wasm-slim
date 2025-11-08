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
}
