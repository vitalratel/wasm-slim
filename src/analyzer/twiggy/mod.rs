//! Twiggy WASM analyzer module
//!
//! Provides size analysis using the twiggy tool to identify
//! the largest contributors to WASM bundle size.
//!
//! # Examples
//!
//! ```no_run
//! use wasm_slim::analyzer::{TwiggyAnalyzer, AnalysisMode};
//! use std::path::Path;
//!
//! let analyzer = TwiggyAnalyzer::new(Path::new("pkg/app_bg.wasm"));
//! let results = analyzer.analyze(AnalysisMode::Top)?;
//!
//! println!("Total size: {} bytes", results.total_size_bytes);
//! for item in results.items.iter().take(10) {
//!     println!("  {} bytes ({:.1}%) - {}",
//!              item.size_bytes, item.percentage, item.name);
//! }
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod analysis_types;
pub mod comparison;
pub mod error;
pub mod executor;
pub mod parser;
pub mod recommendation;
pub mod recommendations;

pub use analysis_types::*;
pub use comparison::*;
pub use error::*;
pub use recommendation::*;

use crate::infra::{CommandExecutor, FileSystem, RealCommandExecutor, RealFileSystem};
use std::path::Path;

/// Main twiggy analyzer for WASM binaries
pub struct TwiggyAnalyzer<
    FS: FileSystem = RealFileSystem,
    CE: CommandExecutor = RealCommandExecutor,
> {
    wasm_file: std::path::PathBuf,
    fs: FS,
    cmd_executor: CE,
}

impl TwiggyAnalyzer {
    /// Create a new analyzer for the given WASM file
    pub fn new(wasm_file: impl Into<std::path::PathBuf>) -> Self {
        Self::with_executors(wasm_file, RealFileSystem, RealCommandExecutor)
    }

    /// Check if twiggy is installed
    pub fn check_installation() -> Result<bool, TwiggyAnalysisError> {
        Self::check_installation_with_executor(&RealCommandExecutor)
    }

    /// Get installation instructions
    pub fn installation_instructions() -> &'static str {
        "twiggy is not installed. Install it with:\n  cargo install twiggy"
    }

    /// Compare two WASM files
    ///
    /// Uses FileSystem and CommandExecutor traits for better testability.
    pub fn compare<FS: FileSystem, CE: CommandExecutor>(
        before: &Path,
        after: &Path,
        fs: &FS,
        cmd_executor: &CE,
    ) -> Result<ComparisonResults, TwiggyAnalysisError> {
        // Get file sizes
        let before_size_bytes = fs.metadata(before)?.len();
        let after_size_bytes = fs.metadata(after)?.len();

        let delta_bytes = after_size_bytes as i64 - before_size_bytes as i64;
        let delta_percent = (delta_bytes as f64 / before_size_bytes as f64) * 100.0;

        // Run twiggy diff
        let before = before.to_path_buf();
        let after = after.to_path_buf();
        let output =
            cmd_executor.execute(|cmd| cmd.args(["diff"]).arg(&before).arg(&after), "twiggy")?;

        let diff_output = String::from_utf8_lossy(&output.stdout);

        // Parse top changes
        let top_changes = Self::parse_diff_output(&diff_output)?;

        Ok(ComparisonResults {
            before_size_bytes,
            after_size_bytes,
            delta_bytes,
            delta_percent,
            top_changes,
        })
    }
}

impl<FS: FileSystem, CE: CommandExecutor> TwiggyAnalyzer<FS, CE> {
    /// Create a new analyzer with custom executors
    pub fn with_executors(
        wasm_file: impl Into<std::path::PathBuf>,
        fs: FS,
        cmd_executor: CE,
    ) -> Self {
        Self {
            wasm_file: wasm_file.into(),
            fs,
            cmd_executor,
        }
    }

    /// Create a new analyzer with a custom command executor (deprecated, use with_executors)
    #[deprecated(since = "0.1.0", note = "Use with_executors instead")]
    pub fn with_executor(wasm_file: impl Into<std::path::PathBuf>, cmd_executor: CE) -> Self
    where
        FS: Default,
    {
        Self::with_executors(wasm_file, FS::default(), cmd_executor)
    }

    /// Check if twiggy is installed using a custom command executor
    pub fn check_installation_with_executor<E: CommandExecutor>(
        executor: &E,
    ) -> Result<bool, TwiggyAnalysisError> {
        match executor.execute(|cmd| cmd.arg("--version"), "twiggy") {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    /// Run analysis with the specified mode
    #[must_use = "Analysis results should be used or printed"]
    pub fn analyze(&self, mode: AnalysisMode) -> Result<AnalysisResults, TwiggyAnalysisError> {
        // Verify file exists
        if !self.wasm_file.exists() {
            return Err(TwiggyAnalysisError::WasmFileNotFound(
                self.wasm_file.display().to_string(),
            ));
        }

        // Get total file size
        let total_size_bytes = self.fs.metadata(&self.wasm_file)?.len();

        // Run twiggy command
        let output = match mode {
            AnalysisMode::Top => self.run_top_analysis()?,
            AnalysisMode::Dominators => self.run_dominators_analysis()?,
            AnalysisMode::Dead => self.run_dead_code_analysis()?,
            AnalysisMode::Monos => self.run_monos_analysis()?,
        };

        // Parse output
        let items = self.parse_output(&output, mode)?;

        // Group monomorphizations if in Monos mode
        let mono_groups = if matches!(mode, AnalysisMode::Monos) {
            Some(self.group_monomorphizations(&items))
        } else {
            None
        };

        // Generate recommendations
        let recommendations = if let Some(ref groups) = mono_groups {
            self.generate_monos_recommendations_enhanced(groups, total_size_bytes)
        } else {
            self.generate_recommendations(&items, total_size_bytes, mode)
        };

        Ok(AnalysisResults {
            total_size_bytes,
            mode: format!("{:?}", mode).to_lowercase(),
            items,
            recommendations,
            mono_groups,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_installation_twiggy_availability_succeeds() {
        // This test will pass if twiggy is installed
        let result = TwiggyAnalyzer::check_installation();
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_nonexistent_file_returns_file_error() {
        let analyzer = TwiggyAnalyzer::new("nonexistent-file.wasm");
        let result = analyzer.analyze(AnalysisMode::Top);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not found") || err.to_string().contains("WASM file"));
    }

    #[test]
    fn test_compare_nonexistent_files_returns_file_error() {
        use crate::infra::{RealCommandExecutor, RealFileSystem};
        let result = TwiggyAnalyzer::compare(
            Path::new("nonexistent_before.wasm"),
            Path::new("nonexistent_after.wasm"),
            &RealFileSystem,
            &RealCommandExecutor,
        );

        assert!(result.is_err());
        // Should fail on file not found
    }

    #[test]
    fn test_installation_instructions_provides_cargo_command() {
        let instructions = TwiggyAnalyzer::installation_instructions();
        assert!(!instructions.is_empty());
        assert!(instructions.contains("twiggy") || instructions.contains("cargo install"));
    }

    #[test]
    fn test_analyze_invalid_wasm_file_handles_corrupted_data() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a file with invalid WASM magic bytes
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"NOT_WASM_DATA").unwrap();
        temp_file.flush().unwrap();

        let analyzer = TwiggyAnalyzer::new(temp_file.path());
        let result = analyzer.analyze(AnalysisMode::Top);

        // Should handle twiggy command failure gracefully
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error should indicate command failure or IO error
        assert!(
            matches!(err, TwiggyAnalysisError::CommandFailed(_, _))
                || matches!(err, TwiggyAnalysisError::Io(_))
        );
    }

    #[test]
    fn test_analyze_empty_wasm_file_handles_edge_case() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create an empty file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"").unwrap();
        temp_file.flush().unwrap();

        let analyzer = TwiggyAnalyzer::new(temp_file.path());
        let result = analyzer.analyze(AnalysisMode::Top);

        // Should handle empty file gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_with_unreadable_file_returns_error() {
        #[cfg(unix)]
        {
            use std::fs;
            use std::io::Write;
            use std::os::unix::fs::PermissionsExt;
            use tempfile::NamedTempFile;

            let temp_file = NamedTempFile::new().unwrap();
            temp_file.as_file().write_all(b"test").unwrap();
            let path = temp_file.path().to_path_buf();

            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o000);
            fs::set_permissions(&path, perms).unwrap();

            let analyzer = TwiggyAnalyzer::new(&path);
            let result = analyzer.analyze(AnalysisMode::Top);
            assert!(result.is_err());

            // Cleanup
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o644);
            let _ = fs::set_permissions(&path, perms);
        }
    }

    #[test]
    fn test_compare_with_invalid_wasm_does_not_panic() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut before_file = NamedTempFile::new().unwrap();
        before_file.write_all(b"INVALID").unwrap();
        before_file.flush().unwrap();

        let mut after_file = NamedTempFile::new().unwrap();
        after_file.write_all(b"ALSO_INVALID").unwrap();
        after_file.flush().unwrap();

        use crate::infra::{RealCommandExecutor, RealFileSystem};
        let result = TwiggyAnalyzer::compare(
            before_file.path(),
            after_file.path(),
            &RealFileSystem,
            &RealCommandExecutor,
        );
        let _ = result; // Test passes if no panic occurs
    }

    #[test]
    fn test_analyze_top_mode_with_invalid_wasm_returns_error() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"INVALID_WASM").unwrap();
        temp_file.flush().unwrap();

        let analyzer = TwiggyAnalyzer::new(temp_file.path());
        let result = analyzer.analyze(AnalysisMode::Top);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_dominators_mode_with_invalid_wasm_returns_error() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"INVALID_WASM").unwrap();
        temp_file.flush().unwrap();

        let analyzer = TwiggyAnalyzer::new(temp_file.path());
        let result = analyzer.analyze(AnalysisMode::Dominators);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_dead_mode_with_invalid_wasm_returns_error() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"INVALID_WASM").unwrap();
        temp_file.flush().unwrap();

        let analyzer = TwiggyAnalyzer::new(temp_file.path());
        let result = analyzer.analyze(AnalysisMode::Dead);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_monos_mode_with_invalid_wasm_returns_error() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"INVALID_WASM").unwrap();
        temp_file.flush().unwrap();

        let analyzer = TwiggyAnalyzer::new(temp_file.path());
        let result = analyzer.analyze(AnalysisMode::Monos);
        assert!(result.is_err());
    }
}
