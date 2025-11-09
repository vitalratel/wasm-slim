//! Tool execution for WASM build pipeline
//!
//! Handles execution of individual tools: cargo, wasm-bindgen, wasm-opt, wasm-snip

use crate::infra::{CommandExecutor, FileSystem};
use std::fs;
use std::path::{Path, PathBuf};

use super::config::PipelineConfig;
use super::error::PipelineError;

/// Executes individual build tools
pub struct ToolRunner<FS: FileSystem, CE: CommandExecutor> {
    project_root: PathBuf,
    config: PipelineConfig,
    fs: FS,
    cmd_executor: CE,
}

impl<FS: FileSystem, CE: CommandExecutor> ToolRunner<FS, CE> {
    /// Create a new tool runner with the given configuration
    pub fn new(project_root: PathBuf, config: PipelineConfig, fs: FS, cmd_executor: CE) -> Self {
        Self {
            project_root,
            config,
            fs,
            cmd_executor,
        }
    }

    /// Execute cargo build for wasm32-unknown-unknown target
    pub fn cargo_build(&self) -> Result<PathBuf, PipelineError> {
        let project_root = self.project_root.clone();
        let target = self.config.target.as_str();
        let target_dir = self.config.target_dir.clone();

        let status = self.cmd_executor.run(
            |cmd| {
                cmd.current_dir(&project_root)
                    .arg("build")
                    .arg("--release")
                    .arg("--target")
                    .arg(target);

                if let Some(ref target_dir) = target_dir {
                    cmd.arg("--target-dir").arg(target_dir);
                }

                // Strip code coverage instrumentation environment variables to prevent
                // "can't find crate for `profiler_builtins`" errors when building under
                // cargo llvm-cov (e.g., in CI coverage runs or integration tests).
                // These vars cause the subprocess build to inherit profiling flags.
                cmd.env_remove("CARGO_INCREMENTAL")
                    .env_remove("RUSTFLAGS")
                    .env_remove("CARGO_ENCODED_RUSTFLAGS")
                    .env_remove("LLVM_PROFILE_FILE")
                    .env_remove("CARGO_LLVM_COV")
                    .env_remove("CARGO_LLVM_COV_TARGET_DIR");

                cmd
            },
            "cargo",
        )?;

        if !status.success() {
            return Err(PipelineError::BuildFailed("cargo build failed".to_string()));
        }

        // Find the built WASM file
        let target_dir = self
            .config
            .target_dir
            .clone()
            .unwrap_or_else(|| self.project_root.join("target"));

        let build_dir = target_dir
            .join(self.config.target.as_str())
            .join(&self.config.profile);

        // Look for .wasm files
        let wasm_files: Vec<_> = self
            .fs
            .read_dir(&build_dir)
            .map_err(PipelineError::Io)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|s| s == "wasm").unwrap_or(false))
            .collect();

        if wasm_files.is_empty() {
            return Err(PipelineError::FileNotFound(format!(
                "No .wasm file found in {}",
                build_dir.display()
            )));
        }

        if wasm_files.len() > 1 {
            use console::style;
            println!(
                "   {} Multiple .wasm files found, using first one",
                style("⚠️")
            );
        }

        Ok(wasm_files[0].path())
    }

    /// Run wasm-bindgen on the WASM file
    pub fn run_wasm_bindgen(&self, wasm_file: &Path) -> Result<PathBuf, PipelineError> {
        let out_dir = self.project_root.join("pkg");
        let wasm_file = wasm_file.to_path_buf();
        let bindgen_target = self.config.bindgen_target.as_str();

        let status = self.cmd_executor.run(
            |cmd| {
                cmd.arg(&wasm_file)
                    .arg("--out-dir")
                    .arg(&out_dir)
                    .arg("--target")
                    .arg(bindgen_target)
            },
            "wasm-bindgen",
        )?;

        if !status.success() {
            return Err(PipelineError::ToolFailed("wasm-bindgen failed".to_string()));
        }

        // Find the output WASM file
        let wasm_files: Vec<_> = self
            .fs
            .read_dir(&out_dir)
            .map_err(PipelineError::Io)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|s| s == "wasm").unwrap_or(false))
            .collect();

        if wasm_files.is_empty() {
            return Err(PipelineError::FileNotFound(format!(
                "No .wasm file found in {}",
                out_dir.display()
            )));
        }

        Ok(wasm_files[0].path())
    }

    /// Run wasm-opt optimization on the WASM file
    pub fn run_wasm_opt(&self, wasm_file: &Path) -> Result<(), PipelineError> {
        let wasm_file = wasm_file.to_path_buf();
        let opt_level_arg = self.config.opt_level.as_arg().to_string();

        let status = self.cmd_executor.run(
            |cmd| {
                cmd.arg(&wasm_file)
                    .arg(&opt_level_arg)
                    .arg("-o")
                    .arg(&wasm_file)
                    // Add Warp-recommended flags
                    .arg("--enable-mutable-globals")
                    .arg("--enable-bulk-memory")
                    .arg("--enable-sign-ext")
                    .arg("--enable-nontrapping-float-to-int")
            },
            "wasm-opt",
        )?;

        if !status.success() {
            return Err(PipelineError::ToolFailed("wasm-opt failed".to_string()));
        }

        Ok(())
    }

    /// Run wasm-snip to remove unnecessary code
    pub fn run_wasm_snip(&self, wasm_file: &Path) -> Result<(), PipelineError> {
        // Create a temporary file for output
        let temp_file = wasm_file.with_extension("wasm.tmp");
        let wasm_file = wasm_file.to_path_buf();

        let status = self.cmd_executor.run(
            |cmd| {
                cmd.arg(&wasm_file)
                    .arg("-o")
                    .arg(&temp_file)
                    .arg("--snip-rust-panicking-code")
            },
            "wasm-snip",
        )?;

        if !status.success() {
            return Err(PipelineError::ToolFailed("wasm-snip failed".to_string()));
        }

        // Replace original with snipped version
        // Note: Using copy instead of rename for testability with mocked filesystem
        self.fs
            .copy(&temp_file, &wasm_file)
            .map_err(PipelineError::Io)?;
        // Clean up temp file (best effort, ignore errors)
        let _ = fs::remove_file(&temp_file);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::{RealCommandExecutor, RealFileSystem};
    use std::io;
    use std::process::Command;
    use std::sync::{Arc, Mutex};

    // Mock FileSystem for testing
    #[derive(Clone)]
    struct MockFileSystem {
        read_dir_result: Arc<Mutex<Option<io::Result<Vec<std::fs::DirEntry>>>>>,
    }

    impl MockFileSystem {
        fn new() -> Self {
            Self {
                read_dir_result: Arc::new(Mutex::new(None)),
            }
        }

        fn set_read_dir_result(&self, result: io::Result<Vec<std::fs::DirEntry>>) {
            *self
                .read_dir_result
                .lock()
                .expect("MockFileSystem lock should never be poisoned in tests") = Some(result);
        }
    }

    impl FileSystem for MockFileSystem {
        fn read_dir(&self, path: &Path) -> io::Result<std::fs::ReadDir> {
            // For mock purposes, return actual empty dir
            let temp_dir = std::env::temp_dir().join(format!("mock_{}", path.display()));
            std::fs::create_dir_all(&temp_dir)?;
            std::fs::read_dir(&temp_dir)
        }

        fn metadata(&self, _path: &Path) -> io::Result<std::fs::Metadata> {
            unimplemented!("metadata not needed for these tests")
        }

        fn read_to_string(&self, _path: &Path) -> io::Result<String> {
            unimplemented!("read_to_string not needed for these tests")
        }

        fn write(&self, _path: &Path, _contents: impl AsRef<[u8]>) -> io::Result<()> {
            unimplemented!("write not needed for these tests")
        }

        fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
            unimplemented!("create_dir_all not needed for these tests")
        }

        fn copy(&self, _from: &Path, _to: &Path) -> io::Result<u64> {
            // Mock copy - just return success with fake size
            Ok(1024)
        }
    }

    // Mock CommandExecutor for testing
    #[derive(Clone)]
    struct MockCommandExecutor {
        exit_code: Arc<Mutex<i32>>,
        should_fail: Arc<Mutex<bool>>,
    }

    impl MockCommandExecutor {
        fn new() -> Self {
            Self {
                exit_code: Arc::new(Mutex::new(0)),
                should_fail: Arc::new(Mutex::new(false)),
            }
        }

        fn set_exit_code(&self, code: i32) {
            *self
                .exit_code
                .lock()
                .expect("MockCommandExecutor lock should never be poisoned in tests") = code;
        }

        fn set_should_fail(&self, fail: bool) {
            *self
                .should_fail
                .lock()
                .expect("MockCommandExecutor lock should never be poisoned in tests") = fail;
        }
    }

    impl CommandExecutor for MockCommandExecutor {
        fn status(&self, _cmd: &mut Command) -> io::Result<std::process::ExitStatus> {
            if *self
                .should_fail
                .lock()
                .expect("MockCommandExecutor lock should never be poisoned in tests")
            {
                return Err(io::Error::new(io::ErrorKind::NotFound, "command not found"));
            }

            // Create a fake exit status
            let code = *self
                .exit_code
                .lock()
                .expect("MockCommandExecutor lock should never be poisoned in tests");
            // This is a hack to create an ExitStatus for testing
            // In real tests, we'd use Command::new("true") or Command::new("false")
            if code == 0 {
                Command::new("true").status()
            } else {
                Command::new("false").status()
            }
        }

        fn output(&self, _cmd: &mut Command) -> io::Result<std::process::Output> {
            unimplemented!("output not needed for these tests")
        }
    }

    #[test]
    fn test_tool_runner_stores_config() {
        let config = PipelineConfig::default();
        let runner = ToolRunner::new(
            PathBuf::from("/test"),
            config.clone(),
            RealFileSystem,
            RealCommandExecutor,
        );
        assert_eq!(runner.config.target.as_str(), config.target.as_str());
    }

    #[test]
    fn test_cargo_build_with_nonexistent_tool_returns_error() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new();
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_should_fail(true);

        let runner = ToolRunner::new(PathBuf::from("/test"), config, fs, cmd_executor);

        let result = runner.cargo_build();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PipelineError::Io(_)));
    }

    #[test]
    fn test_cargo_build_with_non_zero_exit_returns_build_failed() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new();
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_exit_code(1);

        let runner = ToolRunner::new(PathBuf::from("/test"), config, fs, cmd_executor);

        let result = runner.cargo_build();
        assert!(result.is_err());
        if let Err(PipelineError::BuildFailed(msg)) = result {
            assert!(msg.contains("cargo build failed"));
        } else {
            panic!("Expected BuildFailed error");
        }
    }

    #[test]
    fn test_cargo_build_with_no_wasm_files_returns_file_not_found() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new();
        fs.set_read_dir_result(Ok(vec![]));
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_exit_code(0);

        let runner = ToolRunner::new(PathBuf::from("/test"), config, fs, cmd_executor);

        let result = runner.cargo_build();
        assert!(result.is_err());
        if let Err(PipelineError::FileNotFound(msg)) = result {
            assert!(msg.contains("No .wasm file found"));
        } else {
            panic!("Expected FileNotFound error");
        }
    }

    #[test]
    fn test_run_wasm_bindgen_with_non_zero_exit_returns_tool_failed() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new();
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_exit_code(1);

        let runner = ToolRunner::new(PathBuf::from("/test"), config, fs, cmd_executor);

        let result = runner.run_wasm_bindgen(Path::new("/test/input.wasm"));
        assert!(result.is_err());
        if let Err(PipelineError::ToolFailed(msg)) = result {
            assert!(msg.contains("wasm-bindgen failed"));
        } else {
            panic!("Expected ToolFailed error");
        }
    }

    #[test]
    fn test_run_wasm_opt_with_non_zero_exit_returns_tool_failed() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new();
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_exit_code(1);

        let runner = ToolRunner::new(PathBuf::from("/test"), config, fs, cmd_executor);

        let result = runner.run_wasm_opt(Path::new("/test/input.wasm"));
        assert!(result.is_err());
        if let Err(PipelineError::ToolFailed(msg)) = result {
            assert!(msg.contains("wasm-opt failed"));
        } else {
            panic!("Expected ToolFailed error");
        }
    }

    #[test]
    fn test_run_wasm_snip_with_non_zero_exit_returns_tool_failed() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new();
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_exit_code(1);

        let runner = ToolRunner::new(PathBuf::from("/test"), config, fs, cmd_executor);

        let result = runner.run_wasm_snip(Path::new("/test/input.wasm"));
        assert!(result.is_err());
        if let Err(PipelineError::ToolFailed(msg)) = result {
            assert!(msg.contains("wasm-snip failed"));
        } else {
            panic!("Expected ToolFailed error");
        }
    }

    #[test]
    fn test_run_wasm_opt_with_success_returns_ok() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new();
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_exit_code(0);

        let runner = ToolRunner::new(PathBuf::from("/test"), config, fs, cmd_executor);

        let result = runner.run_wasm_opt(Path::new("/test/input.wasm"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_wasm_snip_with_success_returns_ok() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new();
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_exit_code(0);

        let runner = ToolRunner::new(PathBuf::from("/test"), config, fs, cmd_executor);

        let result = runner.run_wasm_snip(Path::new("/test/input.wasm"));
        assert!(result.is_ok());
    }
}
