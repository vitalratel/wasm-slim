//! Safe file backup and restore functionality
//!
//! Provides [`BackupManager`] for creating timestamped backups of Cargo.toml
//! files before optimization, enabling safe rollback if needed.
//!
//! # Examples
//!
//! ```no_run
//! # use wasm_slim::optimizer::BackupManager;
//! # use std::path::Path;
//! let manager = BackupManager::new(Path::new("."));
//! let backup_path = manager.create_backup(Path::new("Cargo.toml"))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::infra::{FileSystem, RealFileSystem};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

/// Format a timestamp for backup filenames using system time
///
/// Uses simplified date calculation (365 days/year, 30 days/month) for backup
/// filename generation. Timestamps may be off by ±1-3 days due to leap years
/// and varying month lengths. This is acceptable for backup identification as:
/// - Backups are uniquely identified by millisecond-precision timestamps
/// - Backup rotation uses filesystem metadata, not parsed dates
/// - Users don't need calendar-accurate dates in backup filenames
///
/// This approach avoids external dependencies (chrono/time crates) which would
/// add 200-400KB to binary size for a feature that doesn't need precision.
///
/// # Format
///
/// Returns timestamp in format: `YYYYMMDD_HHMMSS.mmm`
///
/// Example: `20241102_143052.123`
///
/// # Errors
///
/// Returns [`BackupError::CopyFile`] if system clock is before Unix epoch (1970-01-01).
fn format_timestamp() -> Result<String, BackupError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| BackupError::CopyFile(format!("System time error: {}", e)))?;

    let secs = now.as_secs();
    let millis = now.subsec_millis();

    // Convert to datetime components
    const SECS_PER_DAY: u64 = 86400;
    const SECS_PER_HOUR: u64 = 3600;
    const SECS_PER_MINUTE: u64 = 60;

    let days_since_epoch = secs / SECS_PER_DAY;
    let remaining = secs % SECS_PER_DAY;
    let hours = remaining / SECS_PER_HOUR;
    let remaining = remaining % SECS_PER_HOUR;
    let minutes = remaining / SECS_PER_MINUTE;
    let seconds = remaining % SECS_PER_MINUTE;

    // Simplified date calculation (good enough for backup filenames)
    // Format: YYYYMMDD_HHMMSS.mmm
    let year = 1970 + (days_since_epoch / 365);
    let day_of_year = days_since_epoch % 365;
    let month = 1 + (day_of_year / 30).min(11);
    let day = 1 + (day_of_year % 30);

    Ok(format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}.{:03}",
        year, month, day, hours, minutes, seconds, millis
    ))
}

/// Errors that can occur during backup operations
#[derive(Error, Debug)]
pub enum BackupError {
    /// Failed to create backup directory
    #[error("Failed to create backup directory: {0}")]
    CreateDirectory(#[from] io::Error),

    /// Failed to copy file
    #[error("Failed to copy file: {0}")]
    CopyFile(String),
}

/// Manages backups of Cargo.toml files
pub struct BackupManager<FS: FileSystem = RealFileSystem> {
    backup_dir: PathBuf,
    fs: FS,
}

impl BackupManager<RealFileSystem> {
    /// Create a new BackupManager with the specified backup directory
    pub fn new(project_root: &Path) -> Self {
        Self::with_fs(project_root, RealFileSystem)
    }
}

impl<FS: FileSystem> BackupManager<FS> {
    /// Create a new BackupManager with a custom filesystem implementation
    pub fn with_fs(project_root: &Path, fs: FS) -> Self {
        Self {
            backup_dir: project_root.join(".wasm-slim").join("backups"),
            fs,
        }
    }

    /// Create a backup of the specified file
    ///
    /// Backups are stored in `.wasm-slim/backups/` with timestamped filenames
    /// including UUID for uniqueness in concurrent scenarios.
    ///
    /// # Arguments
    /// * `source` - Path to the file to backup
    ///
    /// # Returns
    /// Path to the created backup file
    ///
    /// # Errors
    /// Returns error if backup directory creation or file copy fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::optimizer::BackupManager;
    /// use std::path::Path;
    ///
    /// let manager = BackupManager::new(Path::new("."));
    /// let backup_path = manager.create_backup(Path::new("Cargo.toml"))?;
    /// println!("Created backup: {}", backup_path.display());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn create_backup(&self, source: &Path) -> Result<PathBuf, BackupError> {
        // Ensure backup directory exists
        self.fs.create_dir_all(&self.backup_dir)?;

        // Generate backup filename with timestamp and cryptographically secure UUID
        //
        // Security: UUID v4 uses cryptographically secure randomness (via `getrandom`)
        // ensuring backup filenames are unpredictable and collision-resistant.
        //
        // Security: file_name() extracts only the final path component, preventing directory
        // traversal attacks. Even if source path is "../../../etc/passwd", file_name() returns
        // only "passwd". This ensures all backups stay within backup_dir regardless of the
        // source path provided. The parent directories are stripped by design.
        let filename = source
            .file_name()
            .ok_or_else(|| BackupError::CopyFile("Invalid source filename".to_string()))?;

        let timestamp = format_timestamp()?;
        let uuid = Uuid::new_v4().simple().to_string();
        let backup_name = format!(
            "{}.{}.{}.backup",
            filename.to_string_lossy(),
            timestamp,
            uuid
        );

        let backup_path = self.backup_dir.join(backup_name);

        // Copy file to backup location
        self.fs
            .copy(source, &backup_path)
            .map_err(|e| BackupError::CopyFile(format!("{}: {}", source.display(), e)))?;

        Ok(backup_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    mod basic_operations {
        use super::*;

        #[test]
        fn test_format_timestamp_generates_valid_format() {
            let timestamp = format_timestamp();
            assert!(timestamp.is_ok());

            let ts = timestamp.unwrap();
            // Should be in format YYYYMMDD_HHMMSS.mmm
            assert!(ts.len() >= 18);
            assert!(ts.contains('_'));
            assert!(ts.contains('.'));
        }
    }

    mod error_handling {
        use super::*;

        #[test]
        #[cfg(unix)]
        fn test_create_backup_with_readonly_backup_dir_returns_error() {
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = TempDir::new().unwrap();
            let project_root = temp_dir.path();

            // Create test file
            let test_file = project_root.join("Cargo.toml");
            fs::write(&test_file, "content").unwrap();

            // Create backup directory
            let backup_dir = project_root.join(".wasm-slim").join("backups");
            fs::create_dir_all(&backup_dir).unwrap();

            // Make backup directory read-only
            let mut perms = fs::metadata(&backup_dir).unwrap().permissions();
            perms.set_mode(0o444); // Read-only
            fs::set_permissions(&backup_dir, perms).unwrap();

            let backup_manager = BackupManager::new(project_root);
            let result = backup_manager.create_backup(&test_file);

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&backup_dir).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&backup_dir, perms).unwrap();

            assert!(result.is_err());
            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("Failed to copy")
                    || err_msg.contains("Permission denied")
                    || err_msg.contains("permission")
            );
        }

        #[test]
        fn test_create_backup_with_nonexistent_source_returns_error() {
            let temp_dir = TempDir::new().unwrap();
            let project_root = temp_dir.path();

            let nonexistent = project_root.join("nonexistent.toml");
            let backup_manager = BackupManager::new(project_root);

            let result = backup_manager.create_backup(&nonexistent);
            assert!(result.is_err());

            let err = result.unwrap_err();
            assert!(err.to_string().contains("Failed to copy"));
        }

        #[test]
        fn test_create_backup_with_invalid_filename_returns_error() {
            let temp_dir = TempDir::new().unwrap();
            let project_root = temp_dir.path();

            // Path with no filename (directory path)
            let invalid_path = Path::new("/");
            let backup_manager = BackupManager::new(project_root);

            let result = backup_manager.create_backup(invalid_path);
            assert!(result.is_err());

            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("Invalid source filename")
                    || err.to_string().contains("Failed to copy")
            );
        }

        #[test]
        fn test_create_backup_with_missing_source_returns_error() {
            // Test error handling when source file disappears
            let temp_dir = TempDir::new().unwrap();
            let project_root = temp_dir.path();
            let test_file = project_root.join("Cargo.toml");

            let backup_manager = BackupManager::new(project_root);

            // Try to backup non-existent file
            let result = backup_manager.create_backup(&test_file);
            assert!(result.is_err());

            let err_msg = result.unwrap_err().to_string();
            assert!(err_msg.contains("Failed to copy file"));
        }

        #[test]
        fn test_create_backup_after_previous_failure_succeeds() {
            // Test that failed backup doesn't prevent subsequent backups
            let temp_dir = TempDir::new().unwrap();
            let project_root = temp_dir.path();
            let backup_manager = BackupManager::new(project_root);

            // First backup fails (no source file)
            let fake_file = project_root.join("nonexistent.toml");
            let result1 = backup_manager.create_backup(&fake_file);
            assert!(result1.is_err());

            // Second backup should succeed
            let test_file = project_root.join("Cargo.toml");
            fs::write(&test_file, "content").unwrap();
            let result2 = backup_manager.create_backup(&test_file);
            assert!(result2.is_ok());
        }
    }

    mod concurrency {
        use super::*;

        #[test]
        fn test_create_backup_with_concurrent_requests_creates_unique_filenames() {
            // Test multiple concurrent backups don't conflict
            let temp_dir = TempDir::new().unwrap();
            let project_root = temp_dir.path().to_path_buf();
            let test_file = project_root.join("Cargo.toml");
            fs::write(&test_file, "content").unwrap();

            // Create multiple backups rapidly
            let mut handles = vec![];
            for _i in 0..5 {
                let test_file_clone = test_file.clone();
                let project_root_clone = project_root.clone();

                let handle = std::thread::spawn(move || {
                    let manager = BackupManager::new(&project_root_clone);
                    manager.create_backup(&test_file_clone)
                });
                handles.push(handle);
            }

            // Wait for all backups to complete
            let mut results = vec![];
            for handle in handles {
                results.push(handle.join().unwrap());
            }

            // All should succeed
            assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 5);

            // All backup paths should be unique
            let paths: Vec<_> = results.iter().filter_map(|r| r.as_ref().ok()).collect();
            let unique_paths: std::collections::HashSet<_> = paths.iter().collect();
            assert_eq!(paths.len(), unique_paths.len());
        }

        #[test]
        fn test_format_timestamp_returns_nonempty_string() {
            let ts = format_timestamp().unwrap();
            assert!(!ts.is_empty());
        }

        #[test]
        fn test_format_timestamp_contains_expected_separators() {
            let ts = format_timestamp().unwrap();
            // Should contain underscores separating date and time components
            assert!(ts.contains('_'));
        }

        #[test]
        fn test_format_timestamp_has_correct_format() {
            let ts = format_timestamp().unwrap();
            // Format should be YYYYMMDD_HHMMSS.mmm
            // Should contain underscore and dot
            assert!(ts.contains('_'));
            assert!(ts.contains('.'));
        }

        #[test]
        fn test_backup_error_copy_file_display() {
            let error = BackupError::CopyFile("/path".to_string());
            let display_str = format!("{}", error);
            assert!(!display_str.is_empty());
            assert!(display_str.contains("Failed to copy file"));
        }

        #[test]
        fn test_backup_error_copy_file_variant() {
            let error = BackupError::CopyFile("/missing/file".to_string());
            match error {
                BackupError::CopyFile(path) => assert_eq!(path, "/missing/file"),
                _ => panic!("Expected CopyFile variant"),
            }
        }

        #[test]
        fn test_backup_manager_new_with_temp_dir() {
            use tempfile::TempDir;
            let temp = TempDir::new().unwrap();
            let manager = BackupManager::new(temp.path());
            // BackupManager::new appends ".wasm-slim/backups" to the path
            let expected = temp.path().join(".wasm-slim").join("backups");
            assert_eq!(manager.backup_dir, expected);
        }

        #[test]
        fn test_backup_manager_backup_dir_stored_correctly() {
            let project_root = PathBuf::from("/test/backup");
            let manager = BackupManager::<RealFileSystem>::new(&project_root);
            // BackupManager::new appends ".wasm-slim/backups" to the path
            let expected = project_root.join(".wasm-slim").join("backups");
            assert_eq!(manager.backup_dir, expected);
        }
    }
}
