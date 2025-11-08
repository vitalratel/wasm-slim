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
