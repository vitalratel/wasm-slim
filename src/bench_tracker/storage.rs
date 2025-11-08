//! Benchmark storage operations (I/O)

use crate::infra::FileSystem;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Collection of benchmark results
#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkBaseline {
    /// Version identifier
    pub version: String,
    /// Unix timestamp of baseline
    pub timestamp: u64,
    /// Git commit hash if available
    pub git_commit: Option<String>,
    /// Map of benchmark name to result
    pub results: HashMap<String, BenchmarkResult>,
}

/// A single benchmark measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Name of the benchmark
    pub name: String,
    /// Mean execution time in nanoseconds
    pub mean_ns: u64,
    /// Standard deviation in nanoseconds
    pub stddev_ns: u64,
    /// Minimum execution time in nanoseconds
    pub min_ns: u64,
    /// Maximum execution time in nanoseconds
    pub max_ns: u64,
    /// Number of iterations performed
    pub iterations: u64,
    /// Unix timestamp when measured
    pub timestamp: u64,
}

/// Handles benchmark baseline persistence
pub struct BenchmarkStorage<FS: FileSystem> {
    baseline_dir: PathBuf,
    fs: FS,
}

impl<FS: FileSystem> BenchmarkStorage<FS> {
    /// Create a new benchmark storage
    pub fn new(project_root: &Path, fs: FS) -> Self {
        let baseline_dir = project_root.join(".wasm-slim").join("benchmarks");
        Self { baseline_dir, fs }
    }

    /// Ensure the baseline directory exists
    fn ensure_baseline_dir(&self) -> Result<()> {
        self.fs
            .create_dir_all(&self.baseline_dir)
            .context("Failed to create benchmark baseline directory")?;
        Ok(())
    }

    /// Get the path to the baseline file
    fn baseline_path(&self) -> PathBuf {
        self.baseline_dir.join("baseline.json")
    }

    /// Load the current baseline
    pub fn load_baseline(&self) -> Result<Option<BenchmarkBaseline>> {
        let path = self.baseline_path();
        if !path.exists() {
            return Ok(None);
        }

        let contents = self
            .fs
            .read_to_string(&path)
            .context("Failed to read baseline file")?;
        let baseline: BenchmarkBaseline =
            serde_json::from_str(&contents).context("Failed to parse baseline JSON")?;
        Ok(Some(baseline))
    }

    /// Save a new baseline
    pub fn save_baseline(&self, baseline: BenchmarkBaseline) -> Result<()> {
        self.ensure_baseline_dir()?;

        let path = self.baseline_path();
        let contents =
            serde_json::to_string_pretty(&baseline).context("Failed to serialize baseline")?;
        self.fs
            .write(&path, contents)
            .context("Failed to write baseline file")?;

        println!("✓ Saved baseline to {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::RealFileSystem;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_baseline_preserves_data() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = BenchmarkStorage::new(temp_dir.path(), RealFileSystem);

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

        let baseline = BenchmarkBaseline {
            version: "v1.0.0".to_string(),
            timestamp: 1234567890,
            git_commit: None,
            results,
        };

        storage
            .save_baseline(baseline)
            .expect("Failed to save baseline");

        let loaded = storage
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
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let storage = BenchmarkStorage::new(temp_dir.path(), RealFileSystem);

        let result = storage
            .load_baseline()
            .expect("Should not error on missing file");
        assert!(result.is_none());
    }
}
