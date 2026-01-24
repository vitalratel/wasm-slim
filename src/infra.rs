//! Infrastructure traits for abstracting I/O operations.
//!
//! This module provides trait abstractions for filesystem and command execution operations,
//! enabling better testability and adherence to the Dependency Inversion Principle.

use std::fs::{Metadata, ReadDir};
use std::io;
use std::path::Path;
use std::process::{Command, ExitStatus, Output};

/// Trait for abstracting filesystem operations.
///
/// This trait allows for dependency injection of filesystem operations,
/// making code more testable and allowing for alternative implementations
/// (e.g., in-memory filesystems for testing, cloud storage, etc.).
pub trait FileSystem {
    /// Copy a file from one location to another.
    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64>;

    /// Create a directory and all missing parent directories.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Read the contents of a directory.
    fn read_dir(&self, path: &Path) -> io::Result<ReadDir>;

    /// Get metadata for a file or directory.
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;

    /// Read the entire contents of a file into a string.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Write a slice of bytes to a file.
    fn write(&self, path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()>;
}

/// Real filesystem implementation that delegates to std::fs.
#[derive(Clone, Copy)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        std::fs::copy(from, to)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<ReadDir> {
        std::fs::read_dir(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        std::fs::metadata(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
        std::fs::write(path, contents)
    }
}

/// Trait for abstracting command execution.
///
/// This trait allows for dependency injection of command execution operations,
/// enabling testing without running real commands and allowing for alternative
/// implementations (e.g., mocked execution, remote execution, etc.).
pub trait CommandExecutor {
    /// Execute a command and return its exit status.
    /// This is the primary method used by the pipeline for running external tools.
    fn status(&self, cmd: &mut Command) -> io::Result<ExitStatus>;

    /// Execute a command and return its output (stdout, stderr, status).
    /// Useful for commands where we need to capture output.
    fn output(&self, cmd: &mut Command) -> io::Result<Output>;

    /// Execute a command built with a closure and return its output.
    ///
    /// This provides a more ergonomic API for building and executing commands:
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::infra::{CommandExecutor, RealCommandExecutor};
    /// use std::process::Command;
    ///
    /// let executor = RealCommandExecutor;
    /// let output = executor.execute(|cmd| {
    ///     cmd.arg("--version")
    ///        .env("RUST_LOG", "debug")
    /// }, "cargo")?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn execute<F>(&self, builder: F, program: &str) -> io::Result<Output>
    where
        F: FnOnce(&mut Command) -> &mut Command,
    {
        let mut cmd = Command::new(program);
        builder(&mut cmd);
        self.output(&mut cmd)
    }

    /// Execute a command built with a closure and return its exit status.
    ///
    /// Similar to `execute()` but only returns the exit status without capturing output.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::infra::{CommandExecutor, RealCommandExecutor};
    /// use std::process::Command;
    ///
    /// let executor = RealCommandExecutor;
    /// let status = executor.run(|cmd| {
    ///     cmd.arg("build")
    ///        .arg("--release")
    /// }, "cargo")?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn run<F>(&self, builder: F, program: &str) -> io::Result<ExitStatus>
    where
        F: FnOnce(&mut Command) -> &mut Command,
    {
        let mut cmd = Command::new(program);
        builder(&mut cmd);
        self.status(&mut cmd)
    }
}

/// Real command executor that delegates to std::process::Command.
#[derive(Debug, Clone, Copy)]
pub struct RealCommandExecutor;

impl CommandExecutor for RealCommandExecutor {
    fn status(&self, cmd: &mut Command) -> io::Result<ExitStatus> {
        cmd.status()
    }

    fn output(&self, cmd: &mut Command) -> io::Result<Output> {
        cmd.output()
    }
}

/// Create an ExitStatus with the given exit code for use in test mocks.
///
/// This avoids spawning actual processes (like `Command::new("true")`) in tests.
#[cfg(all(test, unix))]
pub fn mock_exit_status(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(code << 8) // Unix stores exit code in upper bits
}

#[cfg(all(test, windows))]
pub fn mock_exit_status(code: i32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(code as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    // FileSystem tests

    #[test]
    fn test_real_filesystem_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let fs = RealFileSystem;

        // Write content
        let content = b"Hello, World!";
        fs.write(&file_path, content).unwrap();

        // Read content back
        let read_content = fs.read_to_string(&file_path).unwrap();
        assert_eq!(read_content, "Hello, World!");
    }

    #[test]
    fn test_real_filesystem_copy() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let dest = temp_dir.path().join("dest.txt");

        let fs = RealFileSystem;

        // Create source file
        fs.write(&source, b"test content").unwrap();

        // Copy file
        let bytes_copied = fs.copy(&source, &dest).unwrap();
        assert_eq!(bytes_copied, 12); // "test content" is 12 bytes

        // Verify destination exists and has same content
        let dest_content = fs.read_to_string(&dest).unwrap();
        assert_eq!(dest_content, "test content");
    }

    #[test]
    fn test_real_filesystem_create_dir_all() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("a").join("b").join("c");

        let fs = RealFileSystem;

        // Create nested directories
        fs.create_dir_all(&nested_path).unwrap();

        // Verify they exist
        assert!(nested_path.exists());
        assert!(nested_path.is_dir());
    }

    #[test]
    fn test_real_filesystem_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let fs = RealFileSystem;

        // Create a file
        fs.write(&file_path, b"content").unwrap();

        // Get metadata
        let metadata = fs.metadata(&file_path).unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.len(), 7); // "content" is 7 bytes
    }

    #[test]
    fn test_real_filesystem_read_dir() {
        let temp_dir = TempDir::new().unwrap();
        let fs = RealFileSystem;

        // Create some files
        fs.write(&temp_dir.path().join("file1.txt"), b"test1")
            .unwrap();
        fs.write(&temp_dir.path().join("file2.txt"), b"test2")
            .unwrap();
        fs.write(&temp_dir.path().join("file3.txt"), b"test3")
            .unwrap();

        // Read directory
        let entries: Vec<_> = fs
            .read_dir(temp_dir.path())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_real_filesystem_read_nonexistent_file_returns_error() {
        let fs = RealFileSystem;
        let result = fs.read_to_string(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_real_filesystem_copy_nonexistent_file_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let fs = RealFileSystem;

        let result = fs.copy(
            Path::new("/nonexistent.txt"),
            &temp_dir.path().join("dest.txt"),
        );
        assert!(result.is_err());
    }

    // CommandExecutor tests

    #[test]
    fn test_real_command_executor_status_success() {
        let executor = RealCommandExecutor;
        let mut cmd = Command::new("echo");
        cmd.arg("test");

        let status = executor.status(&mut cmd).unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_real_command_executor_output_captures_stdout() {
        let executor = RealCommandExecutor;
        let mut cmd = Command::new("echo");
        cmd.arg("hello");

        let output = executor.output(&mut cmd).unwrap();
        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    #[test]
    fn test_real_command_executor_execute_with_builder() {
        let executor = RealCommandExecutor;

        let output = executor
            .execute(|cmd| cmd.arg("test_output"), "echo")
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("test_output"));
    }

    #[test]
    fn test_real_command_executor_run_with_builder() {
        let executor = RealCommandExecutor;

        let status = executor.run(|cmd| cmd.arg("test_arg"), "echo").unwrap();

        assert!(status.success());
    }

    #[test]
    fn test_real_command_executor_nonexistent_command_returns_error() {
        let executor = RealCommandExecutor;
        let mut cmd = Command::new("nonexistent_command_xyz_123");

        let result = executor.output(&mut cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_real_command_executor_failed_command_returns_non_success() {
        let executor = RealCommandExecutor;
        // Run a command that will fail (cat with nonexistent file)
        let mut cmd = Command::new("cat");
        cmd.arg("/nonexistent/file/that/does/not/exist.txt");

        let output = executor.output(&mut cmd).unwrap();
        assert!(!output.status.success());
    }

    #[test]
    fn test_real_filesystem_clone() {
        let fs1 = RealFileSystem;
        let fs2 = fs1;

        // Both should work independently
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");

        fs1.write(&path, b"test1").unwrap();
        let content = fs2.read_to_string(&path).unwrap();
        assert_eq!(content, "test1");
    }

    #[test]
    fn test_real_command_executor_clone() {
        let exec1 = RealCommandExecutor;
        let exec2 = exec1;

        // Both should work independently
        let mut cmd = Command::new("echo");
        cmd.arg("test");

        let status1 = exec1.status(&mut cmd).unwrap();
        assert!(status1.success());

        let mut cmd2 = Command::new("echo");
        cmd2.arg("test");
        let status2 = exec2.status(&mut cmd2).unwrap();
        assert!(status2.success());
    }
}
