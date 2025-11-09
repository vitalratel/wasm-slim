//! Twiggy tool execution logic

use super::error::TwiggyAnalysisError;
use crate::analyzer::TwiggyAnalyzer;
use crate::infra::{CommandExecutor, FileSystem};

impl<FS: FileSystem, CE: CommandExecutor> TwiggyAnalyzer<FS, CE> {
    /// Run top contributors analysis
    pub(super) fn run_top_analysis(&self) -> Result<String, TwiggyAnalysisError> {
        let output = self.cmd_executor.execute(
            |cmd| cmd.args(["top", "-n", "50"]).arg(&self.wasm_file),
            "twiggy",
        )?;

        if !output.status.success() {
            return Err(TwiggyAnalysisError::CommandFailed(
                "top".to_string(),
                output.status.code().unwrap_or(-1),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run dominators analysis
    pub(super) fn run_dominators_analysis(&self) -> Result<String, TwiggyAnalysisError> {
        let output = self.cmd_executor.execute(
            |cmd| cmd.args(["dominators"]).arg(&self.wasm_file),
            "twiggy",
        )?;

        if !output.status.success() {
            return Err(TwiggyAnalysisError::CommandFailed(
                "dominators".to_string(),
                output.status.code().unwrap_or(-1),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run dead code analysis
    pub(super) fn run_dead_code_analysis(&self) -> Result<String, TwiggyAnalysisError> {
        let output = self.cmd_executor.execute(
            |cmd| {
                cmd.args(["garbage", "--max-items", "100"])
                    .arg(&self.wasm_file)
            },
            "twiggy",
        )?;

        if !output.status.success() {
            return Err(TwiggyAnalysisError::CommandFailed(
                "garbage".to_string(),
                output.status.code().unwrap_or(-1),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run monomorphization analysis
    pub(super) fn run_monos_analysis(&self) -> Result<String, TwiggyAnalysisError> {
        let output = self
            .cmd_executor
            .execute(|cmd| cmd.args(["monos"]).arg(&self.wasm_file), "twiggy")?;

        if !output.status.success() {
            return Err(TwiggyAnalysisError::CommandFailed(
                "monos".to_string(),
                output.status.code().unwrap_or(-1),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::{CommandExecutor, FileSystem};
    use std::path::{Path, PathBuf};
    use std::process::{Command, ExitStatus, Output};
    use std::sync::{Arc, Mutex};

    struct MockFileSystem;
    impl FileSystem for MockFileSystem {
        fn copy(&self, _from: &Path, _to: &Path) -> std::io::Result<u64> {
            unimplemented!("MockFileSystem::copy not needed for these tests")
        }

        fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
            unimplemented!("MockFileSystem::create_dir_all not needed for these tests")
        }

        fn read_dir(&self, _path: &Path) -> std::io::Result<std::fs::ReadDir> {
            unimplemented!("MockFileSystem::read_dir not needed for these tests")
        }

        fn metadata(&self, _path: &Path) -> std::io::Result<std::fs::Metadata> {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Mock filesystem",
            ))
        }

        fn read_to_string(&self, _path: &Path) -> std::io::Result<String> {
            unimplemented!("MockFileSystem::read_to_string not needed for these tests")
        }

        fn write(&self, _path: &Path, _contents: impl AsRef<[u8]>) -> std::io::Result<()> {
            unimplemented!("MockFileSystem::write not needed for these tests")
        }
    }

    struct MockCommandExecutor {
        output: Arc<Mutex<Option<Output>>>,
    }

    impl MockCommandExecutor {
        fn new() -> Self {
            Self {
                output: Arc::new(Mutex::new(None)),
            }
        }

        fn set_output(&self, output: Output) {
            *self.output.lock().unwrap() = Some(output);
        }
    }

    impl CommandExecutor for MockCommandExecutor {
        fn status(&self, _cmd: &mut Command) -> std::io::Result<ExitStatus> {
            Ok(self
                .output
                .lock()
                .unwrap()
                .as_ref()
                .map(|o| o.status)
                .unwrap_or_default())
        }

        fn output(&self, _cmd: &mut Command) -> std::io::Result<Output> {
            self.output
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| std::io::Error::other("No output set"))
        }
    }

    fn create_success_output(stdout: &[u8]) -> Output {
        Output {
            status: ExitStatus::default(),
            stdout: stdout.to_vec(),
            stderr: vec![],
        }
    }

    #[test]
    fn test_run_top_analysis_with_successful_execution() {
        let mock_executor = MockCommandExecutor::new();
        mock_executor.set_output(create_success_output(b"Top contributors output"));

        let analyzer = TwiggyAnalyzer::with_executors(
            PathBuf::from("test.wasm"),
            MockFileSystem,
            mock_executor,
        );

        let result = analyzer.run_top_analysis();
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Top contributors"));
    }

    #[test]
    fn test_run_dominators_analysis_with_successful_execution() {
        let mock_executor = MockCommandExecutor::new();
        mock_executor.set_output(create_success_output(b"Dominators output"));

        let analyzer = TwiggyAnalyzer::with_executors(
            PathBuf::from("test.wasm"),
            MockFileSystem,
            mock_executor,
        );

        let result = analyzer.run_dominators_analysis();
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Dominators"));
    }

    #[test]
    fn test_run_dead_code_analysis_with_successful_execution() {
        let mock_executor = MockCommandExecutor::new();
        mock_executor.set_output(create_success_output(b"Garbage collection output"));

        let analyzer = TwiggyAnalyzer::with_executors(
            PathBuf::from("test.wasm"),
            MockFileSystem,
            mock_executor,
        );

        let result = analyzer.run_dead_code_analysis();
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Garbage"));
    }

    #[test]
    fn test_run_monos_analysis_with_successful_execution() {
        let mock_executor = MockCommandExecutor::new();
        mock_executor.set_output(create_success_output(b"Monomorphization output"));

        let analyzer = TwiggyAnalyzer::with_executors(
            PathBuf::from("test.wasm"),
            MockFileSystem,
            mock_executor,
        );

        let result = analyzer.run_monos_analysis();
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Monomorphization"));
    }
}
