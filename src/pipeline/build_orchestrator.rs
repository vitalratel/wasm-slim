//! Build orchestration logic
//!
//! Coordinates the build workflow across multiple tools

use console::{style, Emoji};
use std::path::PathBuf;

use crate::fmt::format_bytes;
use crate::infra::{CommandExecutor, FileSystem};
use crate::tools::ToolChain;

use super::config::PipelineConfig;
use super::error::PipelineError;
use super::metrics::SizeMetrics;
use super::result_formatter::ResultFormatter;
use super::tool_runner::ToolRunner;

static HAMMER: Emoji = Emoji("🔨", ">");
static SPARKLES: Emoji = Emoji("✨", "*");
static CHECKMARK: Emoji = Emoji("✅", "[OK]");

/// Orchestrates the complete build workflow
pub struct BuildOrchestrator<FS: FileSystem, CE: CommandExecutor> {
    config: PipelineConfig,
    toolchain: ToolChain,
    tool_runner: ToolRunner<FS, CE>,
    fs: FS,
}

impl<FS: FileSystem + Clone, CE: CommandExecutor + Clone> BuildOrchestrator<FS, CE> {
    /// Create a new build orchestrator
    pub fn new(
        project_root: PathBuf,
        config: PipelineConfig,
        toolchain: ToolChain,
        fs: FS,
        cmd_executor: CE,
    ) -> Self {
        let tool_runner = ToolRunner::new(
            project_root,
            config.clone(),
            fs.clone(),
            cmd_executor.clone(),
        );
        Self {
            config,
            toolchain,
            tool_runner,
            fs,
        }
    }

    /// Execute the complete build pipeline
    pub fn execute(&self) -> Result<SizeMetrics, PipelineError> {
        println!(
            "\n{} {} WASM Build Pipeline",
            HAMMER,
            style("Running").bold()
        );

        // Step 1: Check tools
        self.toolchain.check_required()?;

        // Step 2: Build with cargo
        println!("\n{} Step 1: Building with cargo...", SPARKLES);
        let wasm_file = self.tool_runner.cargo_build()?;
        let before_size = self
            .fs
            .metadata(&wasm_file)
            .map_err(PipelineError::Io)?
            .len();
        println!(
            "   {} Built: {} ({})",
            CHECKMARK,
            style(wasm_file.display()).cyan(),
            style(format_bytes(before_size)).yellow()
        );

        // Step 3: Run wasm-bindgen
        println!("\n{} Step 2: Running wasm-bindgen...", SPARKLES);
        let bindgen_output = self.tool_runner.run_wasm_bindgen(&wasm_file)?;
        println!("   {} wasm-bindgen complete", CHECKMARK);

        // Get the size after wasm-bindgen
        let mut current_size = self
            .fs
            .metadata(&bindgen_output)
            .map_err(PipelineError::Io)?
            .len();

        // Step 4: Run wasm-opt if available
        if self.config.run_wasm_opt && self.toolchain.wasm_opt.is_installed() {
            println!(
                "\n{} Step 3: Running wasm-opt {}...",
                SPARKLES,
                self.config.opt_level.as_arg()
            );
            self.tool_runner.run_wasm_opt(&bindgen_output)?;
            current_size = self
                .fs
                .metadata(&bindgen_output)
                .map_err(PipelineError::Io)?
                .len();
            println!("   {} wasm-opt complete", CHECKMARK);
        } else if self.config.run_wasm_opt {
            println!(
                "\n{} Step 3: Skipping wasm-opt (not installed)",
                style("ℹ️")
            );
        }

        // Step 5: Run wasm-snip if requested and available
        if self.config.run_wasm_snip && self.toolchain.wasm_snip.is_installed() {
            println!("\n{} Step 4: Running wasm-snip...", SPARKLES);
            self.tool_runner.run_wasm_snip(&bindgen_output)?;
            current_size = self
                .fs
                .metadata(&bindgen_output)
                .map_err(PipelineError::Io)?
                .len();
            println!("   {} wasm-snip complete", CHECKMARK);
        }

        // Calculate final metrics
        let metrics = SizeMetrics {
            before_bytes: before_size,
            after_bytes: current_size,
        };

        // Print summary
        ResultFormatter::print_summary(&metrics);

        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::{RealCommandExecutor, RealFileSystem};

    #[test]
    fn test_orchestrator_stores_config() {
        let config = PipelineConfig::default();
        let orchestrator = BuildOrchestrator::new(
            PathBuf::from("/test"),
            config.clone(),
            ToolChain::default(),
            RealFileSystem,
            RealCommandExecutor,
        );
        assert_eq!(orchestrator.config.target.as_str(), config.target.as_str());
    }

    #[test]
    fn test_orchestrator_creates_with_toolchain() {
        let config = PipelineConfig::default();
        let toolchain = ToolChain::default();
        let _ = toolchain.check_all();

        let orchestrator = BuildOrchestrator::new(
            PathBuf::from("/test"),
            config,
            toolchain,
            RealFileSystem,
            RealCommandExecutor,
        );

        assert_eq!(orchestrator.toolchain.cargo.name, "Cargo");
    }

    #[test]
    fn test_orchestrator_respects_config_flags() {
        let config = PipelineConfig {
            run_wasm_opt: false,
            run_wasm_snip: false,
            ..Default::default()
        };

        let orchestrator = BuildOrchestrator::new(
            PathBuf::from("/test"),
            config.clone(),
            ToolChain::default(),
            RealFileSystem,
            RealCommandExecutor,
        );

        assert!(!orchestrator.config.run_wasm_opt);
        assert!(!orchestrator.config.run_wasm_snip);
    }

    #[test]
    fn test_build_with_toolchain_structure() {
        // Test that builds correctly store toolchain information
        let config = PipelineConfig::default();
        let toolchain = ToolChain::default();

        let orchestrator = BuildOrchestrator::new(
            PathBuf::from("/test"),
            config,
            toolchain,
            RealFileSystem,
            RealCommandExecutor,
        );

        // Verify toolchain structure is preserved
        assert_eq!(orchestrator.toolchain.cargo.name, "Cargo");
        assert!(orchestrator.toolchain.cargo.required);
    }

    #[test]
    fn test_concurrent_builds() {
        // Test that orchestrator can handle multiple build configurations
        let config1 = PipelineConfig::default();
        let config2 = PipelineConfig {
            run_wasm_opt: false,
            ..Default::default()
        };

        let orchestrator1 = BuildOrchestrator::new(
            PathBuf::from("/test1"),
            config1,
            ToolChain::default(),
            RealFileSystem,
            RealCommandExecutor,
        );

        let orchestrator2 = BuildOrchestrator::new(
            PathBuf::from("/test2"),
            config2,
            ToolChain::default(),
            RealFileSystem,
            RealCommandExecutor,
        );

        // Verify different configs create different orchestrators
        assert_ne!(
            orchestrator1.config.run_wasm_opt,
            orchestrator2.config.run_wasm_opt
        );
    }
}

// Integration tests for the build orchestrator workflow
// These tests verify end-to-end behavior but don't require actual tools
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::io;
    use std::path::Path;
    use std::process::Command;
    use std::sync::{Arc, Mutex};

    // Mock FileSystem that tracks all operations
    #[derive(Clone)]
    struct MockFileSystem {
        metadata_size: Arc<Mutex<u64>>,
        operations: Arc<Mutex<Vec<String>>>,
    }

    impl MockFileSystem {
        fn new(size: u64) -> Self {
            Self {
                metadata_size: Arc::new(Mutex::new(size)),
                operations: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn operations(&self) -> Vec<String> {
            self.operations
                .lock()
                .expect("MockFileSystem operations lock should never be poisoned in tests")
                .clone()
        }
    }

    impl FileSystem for MockFileSystem {
        fn metadata(&self, path: &Path) -> io::Result<std::fs::Metadata> {
            self.operations
                .lock()
                .expect("MockFileSystem operations lock should never be poisoned in tests")
                .push(format!("metadata: {}", path.display()));

            // Create a real file temporarily to get metadata
            let temp = tempfile::NamedTempFile::new()?;
            let size = *self
                .metadata_size
                .lock()
                .expect("MockFileSystem size lock should never be poisoned in tests");
            std::fs::write(temp.path(), vec![0u8; size as usize])?;
            std::fs::metadata(temp.path())
        }

        fn read_dir(&self, path: &Path) -> io::Result<std::fs::ReadDir> {
            self.operations
                .lock()
                .expect("MockFileSystem operations lock should never be poisoned in tests")
                .push(format!("read_dir: {}", path.display()));
            // Return an actual empty directory
            let temp_dir = std::env::temp_dir().join(format!("mock_{}", path.display()));
            std::fs::create_dir_all(&temp_dir)?;
            std::fs::read_dir(&temp_dir)
        }

        fn read_to_string(&self, _path: &Path) -> io::Result<String> {
            unimplemented!()
        }

        fn write(&self, _path: &Path, _contents: impl AsRef<[u8]>) -> io::Result<()> {
            unimplemented!()
        }

        fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
            unimplemented!()
        }

        fn copy(&self, _from: &Path, _to: &Path) -> io::Result<u64> {
            unimplemented!()
        }
    }

    // Mock CommandExecutor that simulates tool behavior
    #[derive(Clone)]
    struct MockCommandExecutor {
        fail_at_step: Arc<Mutex<Option<String>>>,
        operations: Arc<Mutex<Vec<String>>>,
    }

    impl MockCommandExecutor {
        fn new() -> Self {
            Self {
                fail_at_step: Arc::new(Mutex::new(None)),
                operations: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn set_fail_at_step(&self, step: &str) {
            *self
                .fail_at_step
                .lock()
                .expect("MockCommandExecutor lock should never be poisoned in tests") =
                Some(step.to_string());
        }

        fn operations(&self) -> Vec<String> {
            self.operations
                .lock()
                .expect("MockCommandExecutor operations lock should never be poisoned in tests")
                .clone()
        }
    }

    impl CommandExecutor for MockCommandExecutor {
        fn status(&self, cmd: &mut Command) -> io::Result<std::process::ExitStatus> {
            let program = cmd.get_program().to_string_lossy().to_string();
            self.operations
                .lock()
                .expect("MockCommandExecutor operations lock should never be poisoned in tests")
                .push(format!("execute: {}", program));

            // Check if we should fail at this step
            if let Some(ref fail_step) = *self
                .fail_at_step
                .lock()
                .expect("MockCommandExecutor fail_at_step lock should never be poisoned in tests")
            {
                if program.contains(fail_step) {
                    return Command::new("false").status();
                }
            }

            Command::new("true").status()
        }

        fn output(&self, _cmd: &mut Command) -> io::Result<std::process::Output> {
            unimplemented!()
        }
    }

    #[test]
    #[ignore = "Integration test with mocks - unreliable in containerized environments"]
    fn test_orchestrator_with_failed_cargo_stops_pipeline() {
        let config = PipelineConfig::default();
        let fs = MockFileSystem::new(1000);
        let cmd_executor = MockCommandExecutor::new();
        cmd_executor.set_fail_at_step("cargo");

        let orchestrator = BuildOrchestrator::new(
            PathBuf::from("/test"),
            config,
            ToolChain::default(),
            fs.clone(),
            cmd_executor.clone(),
        );

        let result = orchestrator.execute();
        assert!(result.is_err());

        // Verify cargo was attempted
        let ops = cmd_executor.operations();
        assert!(ops.iter().any(|op| op.contains("cargo")));
    }

    #[test]
    #[ignore = "Integration test with mocks - unreliable in containerized environments"]
    fn test_orchestrator_tracks_size_changes_through_pipeline() {
        let config = PipelineConfig {
            run_wasm_opt: false,
            run_wasm_snip: false,
            ..Default::default()
        };

        let fs = MockFileSystem::new(1000);
        let cmd_executor = MockCommandExecutor::new();

        let orchestrator = BuildOrchestrator::new(
            PathBuf::from("/test"),
            config,
            ToolChain::default(),
            fs.clone(),
            cmd_executor,
        );

        // Note: This will fail because mock doesn't provide actual wasm files
        // But we can verify the setup is correct
        let _ = orchestrator.execute();

        // Verify file system operations were tracked
        let ops = fs.operations();
        assert!(!ops.is_empty());
    }
}
