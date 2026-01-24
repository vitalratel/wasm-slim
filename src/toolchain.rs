//! Rust toolchain detection and management

use crate::infra::{CommandExecutor, RealCommandExecutor};
use crate::tools::ToolError;

/// Rust toolchain detector
pub struct ToolchainDetector<CE: CommandExecutor = RealCommandExecutor> {
    cmd_executor: CE,
}

impl ToolchainDetector<RealCommandExecutor> {
    /// Create a new toolchain detector with real command execution
    pub fn new() -> Self {
        Self {
            cmd_executor: RealCommandExecutor,
        }
    }
}

impl Default for ToolchainDetector<RealCommandExecutor> {
    fn default() -> Self {
        Self::new()
    }
}

impl<CE: CommandExecutor> ToolchainDetector<CE> {
    /// Create a toolchain detector with a custom command executor (for testing)
    pub fn with_executor(cmd_executor: CE) -> Self {
        Self { cmd_executor }
    }

    /// Check if the active Rust toolchain is nightly
    ///
    /// Detects nightly by running `rustc --version` and checking for "nightly" in output.
    /// Required for build-std feature which provides additional 10-20% size reduction.
    ///
    /// # Returns
    /// - `Ok(true)` if nightly toolchain is active
    /// - `Ok(false)` if stable/beta toolchain is active  
    /// - `Err` if rustc command fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::toolchain::ToolchainDetector;
    ///
    /// let detector = ToolchainDetector::new();
    /// match detector.is_nightly_toolchain() {
    ///     Ok(true) => println!("Nightly detected - build-std available"),
    ///     Ok(false) => println!("Stable toolchain - build-std unavailable"),
    ///     Err(e) => eprintln!("Failed to check toolchain: {}", e),
    /// }
    /// ```
    pub fn is_nightly_toolchain(&self) -> Result<bool, ToolError> {
        let output = self
            .cmd_executor
            .execute(|cmd| cmd.arg("--version"), "rustc")?;

        if !output.status.success() {
            return Err(ToolError::VersionFailed("rustc".to_string()));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.contains("nightly"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::CommandExecutor;
    use std::process::{Command, ExitStatus, Output};

    // Mock CommandExecutor for testing
    struct MockCommandExecutor {
        stdout: Vec<u8>,
    }

    impl CommandExecutor for MockCommandExecutor {
        fn status(&self, _cmd: &mut Command) -> std::io::Result<ExitStatus> {
            unimplemented!()
        }

        fn output(&self, _cmd: &mut Command) -> std::io::Result<Output> {
            Ok(Output {
                status: ExitStatus::default(),
                stdout: self.stdout.clone(),
                stderr: vec![],
            })
        }
    }

    #[test]
    fn test_is_nightly_toolchain_detects_nightly() {
        let mock = MockCommandExecutor {
            stdout: b"rustc 1.86.0-nightly (abc123 2024-03-01)\n".to_vec(),
        };
        let detector = ToolchainDetector::with_executor(mock);

        let result = detector.is_nightly_toolchain().unwrap();
        assert!(result);
    }

    #[test]
    fn test_is_nightly_toolchain_detects_stable() {
        let mock = MockCommandExecutor {
            stdout: b"rustc 1.86.0 (stable 2024-03-31)\n".to_vec(),
        };
        let detector = ToolchainDetector::with_executor(mock);

        let result = detector.is_nightly_toolchain().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_is_nightly_toolchain_returns_bool() {
        // Integration test with real rustc
        let detector = ToolchainDetector::new();
        if let Ok(is_nightly) = detector.is_nightly_toolchain() {
            // is_nightly is either true or false - just verify it's a bool
            let _: bool = is_nightly;
        }
    }
}
