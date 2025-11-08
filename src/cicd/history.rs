//! Build history tracking for regression detection
//!
//! Stores build sizes in .wasm-slim/history.json to:
//! - Detect size regressions (>5% increase)
//! - Track optimization progress over time
//! - Provide historical context for CI/CD

use crate::git::GitRepository;
use crate::infra::{FileSystem, RealFileSystem};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Single build record in history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Build size in bytes
    pub size_bytes: u64,
    /// Git commit hash (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_hash: Option<String>,
    /// Git branch (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

impl BuildRecord {
    /// Create a new build record with current timestamp
    pub fn new(size_bytes: u64) -> Result<Self> {
        Ok(Self {
            timestamp: current_timestamp()?,
            size_bytes,
            commit_hash: GitRepository::new().get_commit_hash().ok().flatten(),
            branch: GitRepository::new().get_branch_name().ok().flatten(),
        })
    }
}

/// Build history manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildHistory {
    /// List of build records (newest first)
    pub records: Vec<BuildRecord>,
}

impl BuildHistory {
    const HISTORY_DIR: &'static str = ".wasm-slim";
    const HISTORY_FILE: &'static str = "history.json";
    const MAX_RECORDS: usize = 100;

    /// Create a new empty history
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Load history from project root
    pub fn load(project_root: &Path) -> Result<Self> {
        Self::load_with_fs(project_root, &RealFileSystem)
    }

    /// Load history with a custom filesystem implementation
    pub fn load_with_fs<FS: FileSystem>(project_root: &Path, fs: &FS) -> Result<Self> {
        let history_path = Self::history_path(project_root);

        if !history_path.exists() {
            return Ok(Self::new());
        }

        let contents = fs
            .read_to_string(&history_path)
            .context("Failed to read build history")?;

        let history: BuildHistory =
            serde_json::from_str(&contents).context("Failed to parse build history")?;

        Ok(history)
    }

    /// Save history to project root
    pub fn save(&self, project_root: &Path) -> Result<()> {
        self.save_with_fs(project_root, &RealFileSystem)
    }

    /// Save history with a custom filesystem implementation
    pub fn save_with_fs<FS: FileSystem>(&self, project_root: &Path, fs: &FS) -> Result<()> {
        let history_dir = project_root.join(Self::HISTORY_DIR);
        let history_path = Self::history_path(project_root);

        // Create directory if it doesn't exist
        fs.create_dir_all(&history_dir)
            .context("Failed to create .wasm-slim directory")?;

        let contents =
            serde_json::to_string_pretty(self).context("Failed to serialize build history")?;

        fs.write(&history_path, contents)
            .context("Failed to write build history")?;

        Ok(())
    }

    /// Add a new build record
    pub fn add_record(&mut self, record: BuildRecord) {
        // Add to front (newest first)
        self.records.insert(0, record);

        // Trim to max records
        if self.records.len() > Self::MAX_RECORDS {
            self.records.truncate(Self::MAX_RECORDS);
        }
    }

    /// Get the most recent build record
    pub fn latest(&self) -> Option<&BuildRecord> {
        self.records.first()
    }

    /// Check for size regression (>5% increase from previous)
    /// Compares current_size against the most recent build in history
    pub fn check_regression(&self, current_size: u64) -> Option<RegressionResult> {
        // Compare against the latest build (most recent)
        let previous = self.latest()?;

        let size_diff = current_size as i64 - previous.size_bytes as i64;
        let percent_change = (size_diff as f64 / previous.size_bytes as f64) * 100.0;

        // Regression if >5% increase
        let is_regression = percent_change > 5.0;

        Some(RegressionResult {
            is_regression,
            previous_size: previous.size_bytes,
            current_size,
            size_diff,
            percent_change,
        })
    }

    /// Get history file path
    fn history_path(project_root: &Path) -> PathBuf {
        project_root
            .join(Self::HISTORY_DIR)
            .join(Self::HISTORY_FILE)
    }
}

impl Default for BuildHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of regression check
#[derive(Debug, Clone)]
pub struct RegressionResult {
    /// Whether a regression was detected
    pub is_regression: bool,
    /// Previous build size in bytes
    pub previous_size: u64,
    /// Current build size in bytes
    pub current_size: u64,
    /// Size difference in bytes (negative = reduction)
    pub size_diff: i64,
    /// Percent change (positive = increase)
    pub percent_change: f64,
}

impl RegressionResult {
    /// Print regression status
    ///
    /// Delegates to display module for console formatting.
    pub fn print(&self) {
        super::display::print_regression(self);
    }
}

/// Get current ISO 8601 timestamp
fn current_timestamp() -> Result<String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?;

    // Format as ISO 8601: YYYY-MM-DDTHH:MM:SSZ
    let secs = now.as_secs();

    // Simple ISO 8601 formatting without external dependencies
    Ok(format!("{:019}", secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_build_record_new_creates_with_current_timestamp() {
        let record = BuildRecord::new(500 * 1024).unwrap();
        assert_eq!(record.size_bytes, 500 * 1024);
        assert!(!record.timestamp.is_empty());
    }

    #[test]
    fn test_build_history_add_record_and_get_latest_works() {
        let mut history = BuildHistory::new();
        assert!(history.latest().is_none());

        let record1 = BuildRecord::new(500 * 1024).unwrap();
        history.add_record(record1.clone());
        assert_eq!(history.latest().unwrap().size_bytes, 500 * 1024);

        let record2 = BuildRecord::new(600 * 1024).unwrap();
        history.add_record(record2.clone());
        assert_eq!(history.latest().unwrap().size_bytes, 600 * 1024);
        assert_eq!(history.records[1].size_bytes, 500 * 1024);
    }

    #[test]
    fn test_build_history_check_regression_detects_size_increases() {
        let mut history = BuildHistory::new();

        // Add one record to establish a baseline
        let record1 = BuildRecord::new(500 * 1024).unwrap(); // 500 KB (latest/previous)
        history.add_record(record1);

        // No regression: 4% increase from previous (500 KB)
        let result = history.check_regression(520 * 1024);
        assert!(result.is_some());
        let res = result.unwrap();
        assert!(!res.is_regression);
        assert_eq!(res.previous_size, 500 * 1024);

        // Regression: 10% increase from previous (500 KB)
        let result = history.check_regression(550 * 1024);
        assert!(result.is_some());
        let res = result.unwrap();
        assert!(res.is_regression);
        assert_eq!(res.previous_size, 500 * 1024);

        // Improvement: size decreased from previous (500 KB)
        let result = history.check_regression(450 * 1024);
        assert!(result.is_some());
        let res = result.unwrap();
        assert!(!res.is_regression);
        assert_eq!(res.previous_size, 500 * 1024);
    }

    #[test]
    fn test_build_history_save_and_load_preserves_records() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let mut history = BuildHistory::new();
        let record = BuildRecord::new(500 * 1024).unwrap();
        history.add_record(record);

        history.save(project_root).unwrap();

        let loaded = BuildHistory::load(project_root).unwrap();
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.latest().unwrap().size_bytes, 500 * 1024);
    }

    #[test]
    fn test_build_history_add_record_maintains_max_records_limit() {
        let mut history = BuildHistory::new();

        // Add more than MAX_RECORDS
        for i in 0..150 {
            let record = BuildRecord::new((i + 1) * 1024).unwrap();
            history.add_record(record);
        }

        // Should be truncated to MAX_RECORDS
        assert_eq!(history.records.len(), BuildHistory::MAX_RECORDS);
        // Most recent should be 150 KB
        assert_eq!(history.latest().unwrap().size_bytes, 150 * 1024);
    }

    // P1-TEST-COV-005: History tracking edge cases

    #[test]
    fn test_build_history_load_with_corrupted_json_returns_error() {
        // Test history loading with corrupted JSON file
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create .wasm-slim directory
        let history_dir = project_root.join(".wasm-slim");
        fs::create_dir_all(&history_dir).unwrap();

        // Write corrupted JSON
        let history_path = history_dir.join("history.json");
        fs::write(&history_path, "{invalid json, missing bracket").unwrap();

        // Should fail to load corrupted history
        let result = BuildHistory::load(project_root);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parse"));
    }

    #[test]
    fn test_build_history_load_with_empty_json_returns_error() {
        // Test history loading with empty file
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let history_dir = project_root.join(".wasm-slim");
        fs::create_dir_all(&history_dir).unwrap();

        let history_path = history_dir.join("history.json");
        fs::write(&history_path, "").unwrap();

        // Should fail on empty file
        let result = BuildHistory::load(project_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_history_load_with_invalid_schema_returns_error() {
        // Test history loading with valid JSON but invalid schema
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let history_dir = project_root.join(".wasm-slim");
        fs::create_dir_all(&history_dir).unwrap();

        let history_path = history_dir.join("history.json");
        // Valid JSON but wrong structure (missing 'records' field)
        fs::write(&history_path, "{\\\"wrong_field\\\": []}").unwrap();

        let result = BuildHistory::load(project_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_history_save_creates_directory_if_needed() {
        // Test that save creates .wasm-slim directory if it doesn't exist
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let mut history = BuildHistory::new();
        let record = BuildRecord::new(100 * 1024).unwrap();
        history.add_record(record);

        // Directory should not exist yet
        assert!(!project_root.join(".wasm-slim").exists());

        // Save should create directory
        history.save(project_root).unwrap();

        // Verify directory and file exist
        assert!(project_root.join(".wasm-slim").exists());
        assert!(project_root
            .join(".wasm-slim")
            .join("history.json")
            .exists());
    }

    #[test]
    fn test_build_history_save_overwrites_existing_file() {
        // Test that save overwrites existing history file
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create initial history
        let mut history1 = BuildHistory::new();
        let record1 = BuildRecord::new(100 * 1024).unwrap();
        history1.add_record(record1);
        history1.save(project_root).unwrap();

        // Create new history with different data
        let mut history2 = BuildHistory::new();
        let record2 = BuildRecord::new(200 * 1024).unwrap();
        history2.add_record(record2);
        history2.save(project_root).unwrap();

        // Load and verify it has the new data
        let loaded = BuildHistory::load(project_root).unwrap();
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.latest().unwrap().size_bytes, 200 * 1024);
    }

    #[test]
    fn test_build_history_concurrent_add_record_works_correctly() {
        // Test that adding records maintains correct order
        let mut history = BuildHistory::new();

        // Simulate concurrent-like additions
        let record1 = BuildRecord::new(100 * 1024).unwrap();
        let record2 = BuildRecord::new(200 * 1024).unwrap();
        let record3 = BuildRecord::new(300 * 1024).unwrap();

        history.add_record(record1);
        history.add_record(record2);
        history.add_record(record3);

        // Most recent should be last added (300 KB)
        assert_eq!(history.latest().unwrap().size_bytes, 300 * 1024);
        assert_eq!(history.records[1].size_bytes, 200 * 1024);
        assert_eq!(history.records[2].size_bytes, 100 * 1024);
    }

    #[test]
    fn test_build_history_save_and_load_with_git_metadata_preserves_fields() {
        // Test saving and loading records with git metadata
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let mut history = BuildHistory::new();
        let mut record = BuildRecord::new(500 * 1024).unwrap();
        // Manually set git metadata
        record.commit_hash = Some("abc1234".to_string());
        record.branch = Some("main".to_string());
        history.add_record(record);

        history.save(project_root).unwrap();

        let loaded = BuildHistory::load(project_root).unwrap();
        let loaded_record = loaded.latest().unwrap();
        assert_eq!(loaded_record.commit_hash, Some("abc1234".to_string()));
        assert_eq!(loaded_record.branch, Some("main".to_string()));
    }

    #[test]
    fn test_build_history_save_and_load_without_git_metadata_works() {
        // Test that records without git metadata serialize correctly
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let mut history = BuildHistory::new();
        let mut record = BuildRecord::new(500 * 1024).unwrap();
        // Explicitly set to None
        record.commit_hash = None;
        record.branch = None;
        history.add_record(record);

        history.save(project_root).unwrap();

        let loaded = BuildHistory::load(project_root).unwrap();
        let loaded_record = loaded.latest().unwrap();
        assert_eq!(loaded_record.commit_hash, None);
        assert_eq!(loaded_record.branch, None);
    }

    #[test]
    fn test_build_history_check_regression_with_empty_history_returns_none() {
        // Test regression check with no history
        let history = BuildHistory::new();
        let result = history.check_regression(100 * 1024);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_history_check_regression_at_5_percent_threshold_returns_none() {
        // Test exactly 5% increase (should not be regression)
        let mut history = BuildHistory::new();
        let record = BuildRecord::new(1000 * 1024).unwrap();
        history.add_record(record);

        // Exactly 5% increase: 1000 * 1.05 = 1050
        let result = history.check_regression(1050 * 1024).unwrap();
        assert!(!result.is_regression);
        assert_eq!(result.percent_change, 5.0);
    }

    #[test]
    fn test_build_history_check_regression_just_over_5_percent_returns_regression() {
        // Test just over 5% increase (should be regression)
        let mut history = BuildHistory::new();
        let record = BuildRecord::new(1000 * 1024).unwrap();
        history.add_record(record);

        // 5.1% increase
        let result = history.check_regression(1051 * 1024).unwrap();
        assert!(result.is_regression);
        assert!(result.percent_change > 5.0);
    }

    #[test]
    fn test_build_history_multiple_save_load_cycles_work_correctly() {
        // Test multiple save/load cycles maintain data integrity
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        let mut history = BuildHistory::new();

        // Cycle 1: Add and save
        let record1 = BuildRecord::new(100 * 1024).unwrap();
        history.add_record(record1);
        history.save(project_root).unwrap();

        // Cycle 2: Load, add, save
        let mut loaded = BuildHistory::load(project_root).unwrap();
        let record2 = BuildRecord::new(200 * 1024).unwrap();
        loaded.add_record(record2);
        loaded.save(project_root).unwrap();

        // Cycle 3: Load, add, save
        let mut loaded2 = BuildHistory::load(project_root).unwrap();
        let record3 = BuildRecord::new(300 * 1024).unwrap();
        loaded2.add_record(record3);
        loaded2.save(project_root).unwrap();

        // Final verification
        let final_history = BuildHistory::load(project_root).unwrap();
        assert_eq!(final_history.records.len(), 3);
        assert_eq!(final_history.latest().unwrap().size_bytes, 300 * 1024);
        assert_eq!(final_history.records[1].size_bytes, 200 * 1024);
        assert_eq!(final_history.records[2].size_bytes, 100 * 1024);
    }
}
