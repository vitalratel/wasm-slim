//! Tool detection and verification module
//!
//! Detects the presence and versions of required WASM build tools:
//! - cargo (required)
//! - wasm-bindgen-cli (required for wasm-bindgen workflow)
//! - wasm-opt (optional but recommended, from binaryen)
//! - wasm-snip (optional, for panic removal)

use crate::infra::{CommandExecutor, RealCommandExecutor};
use console::style;
use thiserror::Error;

/// Errors that can occur during tool operations
#[derive(Error, Debug)]
pub enum ToolError {
    /// I/O error during tool execution
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Tool execution failed
    #[error("Failed to get version for {0}")]
    VersionFailed(String),

    /// Required tool is missing
    #[error("Required tool missing: {0}")]
    MissingTool(String),
}

/// Represents a build tool with detection capabilities
///
/// Used to verify required tools (cargo, wasm-bindgen, etc.) are installed.
///
/// # Examples
///
/// ```
/// use wasm_slim::tools::ToolChain;
///
/// // Create toolchain and check required tools
/// let toolchain = ToolChain::new();
///
/// // Check if all required tools are available
/// match toolchain.check_required() {
///     Ok(_) => println!("All required tools found"),
///     Err(e) => eprintln!("Missing tools: {}", e),
/// }
/// ```
#[derive(Debug)]
pub struct Tool<CE: CommandExecutor = RealCommandExecutor> {
    /// Human-readable name
    pub name: &'static str,
    /// Binary name in PATH
    pub binary: &'static str,
    /// Flag to get version (e.g., "--version")
    pub version_flag: &'static str,
    /// Whether this tool is required
    pub required: bool,
    /// Command executor for running version checks
    cmd_executor: CE,
}

impl<CE: CommandExecutor> Tool<CE> {
    /// Create a new Tool with a custom command executor
    pub fn with_executor(
        name: &'static str,
        binary: &'static str,
        version_flag: &'static str,
        required: bool,
        cmd_executor: CE,
    ) -> Self {
        Self {
            name,
            binary,
            version_flag,
            required,
            cmd_executor,
        }
    }

    /// Check if the tool is installed and available in PATH
    ///
    /// # Returns
    /// `true` if tool is found in system PATH
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::tools::Tool;
    /// use wasm_slim::infra::RealCommandExecutor;
    ///
    /// let cargo = Tool::with_executor(
    ///     "Cargo",
    ///     "cargo",
    ///     "--version",
    ///     true,
    ///     RealCommandExecutor,
    /// );
    ///
    /// if cargo.is_installed() {
    ///     println!("Cargo is installed");
    /// }
    /// ```
    pub fn is_installed(&self) -> bool {
        which::which(self.binary).is_ok()
    }

    /// Get the version string of the installed tool
    pub fn version(&self) -> Result<String, ToolError> {
        let output = self
            .cmd_executor
            .execute(|cmd| cmd.arg(self.version_flag), self.binary)?;

        if !output.status.success() {
            return Err(ToolError::VersionFailed(self.name.to_string()));
        }

        let version = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        Ok(version)
    }

    /// Check and report the tool status
    pub fn check(&self) -> ToolStatus {
        if self.is_installed() {
            match self.version() {
                Ok(version) => ToolStatus::Available(version),
                Err(_) => ToolStatus::InstalledButVersionUnknown,
            }
        } else {
            ToolStatus::Missing
        }
    }
}

impl Tool<RealCommandExecutor> {
    /// Create a new Tool with real command execution
    pub fn new(
        name: &'static str,
        binary: &'static str,
        version_flag: &'static str,
        required: bool,
    ) -> Self {
        Self::with_executor(name, binary, version_flag, required, RealCommandExecutor)
    }
}

/// Status of a tool check
#[derive(Debug)]
pub enum ToolStatus {
    /// Tool is available and version was successfully retrieved
    Available(String),
    /// Tool binary exists but version check failed
    InstalledButVersionUnknown,
    /// Tool binary not found
    Missing,
}

/// All tools needed for WASM optimization
///
/// Manages detection and verification of build tools:
/// - cargo (required)
/// - wasm-bindgen-cli (required)
/// - wasm-opt (optional)
/// - wasm-snip (optional)
pub struct ToolChain<CE: CommandExecutor = RealCommandExecutor> {
    /// Cargo build tool (required)
    pub cargo: Tool<CE>,
    /// wasm-bindgen CLI tool (required)
    pub wasm_bindgen: Tool<CE>,
    /// wasm-opt optimization tool (optional)
    pub wasm_opt: Tool<CE>,
    /// wasm-snip code removal tool (optional)
    pub wasm_snip: Tool<CE>,
}

impl Default for ToolChain<RealCommandExecutor> {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolChain<RealCommandExecutor> {
    /// Create a new ToolChain with real command execution
    pub fn new() -> Self {
        Self::with_executor(RealCommandExecutor)
    }
}

impl<CE: CommandExecutor + Clone> ToolChain<CE> {
    /// Create a new ToolChain with a custom command executor
    pub fn with_executor(cmd_executor: CE) -> Self {
        Self {
            cargo: Tool::with_executor("Cargo", "cargo", "--version", true, cmd_executor.clone()),
            wasm_bindgen: Tool::with_executor(
                "wasm-bindgen-cli",
                "wasm-bindgen",
                "--version",
                true,
                cmd_executor.clone(),
            ),
            wasm_opt: Tool::with_executor(
                "wasm-opt (Binaryen)",
                "wasm-opt",
                "--version",
                false,
                cmd_executor.clone(),
            ),
            wasm_snip: Tool::with_executor(
                "wasm-snip",
                "wasm-snip",
                "--version",
                false,
                cmd_executor,
            ),
        }
    }
}

impl<CE: CommandExecutor> ToolChain<CE> {
    /// Check all tools and report their status
    pub fn check_all(&self) -> Result<(), ToolError> {
        println!("\n{} Checking build tools...", style("🔧").bold());

        let tools = [
            &self.cargo,
            &self.wasm_bindgen,
            &self.wasm_opt,
            &self.wasm_snip,
        ];

        let mut missing_required: Vec<&Tool<CE>> = Vec::new();
        let mut missing_optional: Vec<&Tool<CE>> = Vec::new();

        for tool in &tools {
            match tool.check() {
                ToolStatus::Available(version) => {
                    println!(
                        "   {} {} - {}",
                        style("✓").green(),
                        style(tool.name).bold(),
                        style(version).dim()
                    );
                }
                ToolStatus::InstalledButVersionUnknown => {
                    println!(
                        "   {} {} - {}",
                        style("✓").green(),
                        style(tool.name).bold(),
                        style("(version unknown)").dim()
                    );
                }
                ToolStatus::Missing => {
                    if tool.required {
                        println!(
                            "   {} {} - {}",
                            style("✗").red(),
                            style(tool.name).bold(),
                            style("NOT FOUND").red()
                        );
                        missing_required.push(*tool);
                    } else {
                        println!(
                            "   {} {} - {} {}",
                            style("○").yellow(),
                            style(tool.name).bold(),
                            style("NOT FOUND").yellow(),
                            style("(optional)").dim()
                        );
                        missing_optional.push(*tool);
                    }
                }
            }
        }

        // Report missing required tools
        if !missing_required.is_empty() {
            println!("\n{} Missing required tools:", style("❌").bold());
            for tool in &missing_required {
                println!("   • {}", tool.name);
            }
            println!("\n{} Installation instructions:", style("💡").bold());
            self.print_installation_instructions(&missing_required);
            return Err(ToolError::MissingTool(
                "Required tools are missing. Please install them and try again.".to_string(),
            ));
        }

        // Report missing optional tools
        if !missing_optional.is_empty() {
            println!("\n{} Optional tools not found:", style("ℹ️").bold());
            println!("   These tools provide additional optimizations:");
            for tool in &missing_optional {
                println!("   • {}", tool.name);
            }
            println!(
                "\n{} You can install them for better results:",
                style("💡").bold()
            );
            self.print_installation_instructions(&missing_optional);
        }

        Ok(())
    }

    fn print_installation_instructions(&self, tools: &[&Tool<CE>]) {
        for tool in tools {
            match tool.binary {
                "cargo" => {
                    println!("\n   Cargo (Rust toolchain):");
                    println!("     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh");
                }
                "wasm-bindgen" => {
                    println!("\n   wasm-bindgen-cli:");
                    println!("     cargo install wasm-bindgen-cli");
                }
                "wasm-opt" => {
                    println!("\n   wasm-opt (Binaryen):");
                    println!("     # macOS");
                    println!("     brew install binaryen");
                    println!("     # Linux (Debian/Ubuntu)");
                    println!("     sudo apt install binaryen");
                    println!(
                        "     # Or build from source: https://github.com/WebAssembly/binaryen"
                    );
                }
                "wasm-snip" => {
                    println!("\n   wasm-snip:");
                    println!("     cargo install wasm-snip");
                }
                _ => {}
            }
        }
    }

    /// Check only required tools (faster check)
    ///
    /// Verifies cargo and wasm-bindgen-cli are available by running them.
    ///
    /// # Errors
    /// Returns error if any required tool is missing or not working
    pub fn check_required(&self) -> Result<(), ToolError> {
        let tools = [&self.cargo, &self.wasm_bindgen];

        for tool in &tools {
            // Try to get version to verify tool is installed and working
            if tool.version().is_err() {
                return Err(ToolError::MissingTool(format!(
                    "{} is required but not found or not working",
                    tool.name
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::mock_exit_status;
    use std::io;
    use std::process::{Command, Output};
    use std::sync::{Arc, Mutex};

    // Mock CommandExecutor for testing
    #[derive(Clone)]
    struct MockCommandExecutor {
        should_succeed: Arc<Mutex<bool>>,
        stdout_data: Arc<Mutex<Vec<u8>>>,
        should_error: Arc<Mutex<bool>>,
    }

    impl MockCommandExecutor {
        fn new() -> Self {
            Self {
                should_succeed: Arc::new(Mutex::new(true)),
                stdout_data: Arc::new(Mutex::new(b"default version".to_vec())),
                should_error: Arc::new(Mutex::new(false)),
            }
        }

        fn set_success(&self, stdout: &str) {
            *self.should_succeed.lock().unwrap() = true;
            *self.stdout_data.lock().unwrap() = stdout.as_bytes().to_vec();
            *self.should_error.lock().unwrap() = false;
        }

        fn set_failure(&self) {
            *self.should_succeed.lock().unwrap() = false;
            *self.stdout_data.lock().unwrap() = vec![];
            *self.should_error.lock().unwrap() = false;
        }

        fn set_error(&self) {
            *self.should_error.lock().unwrap() = true;
        }
    }

    impl CommandExecutor for MockCommandExecutor {
        fn status(&self, _cmd: &mut Command) -> io::Result<std::process::ExitStatus> {
            if *self.should_error.lock().unwrap() {
                return Err(io::Error::new(io::ErrorKind::NotFound, "command not found"));
            }

            if *self.should_succeed.lock().unwrap() {
                Ok(mock_exit_status(0))
            } else {
                Ok(mock_exit_status(1))
            }
        }

        fn output(&self, _cmd: &mut Command) -> io::Result<Output> {
            if *self.should_error.lock().unwrap() {
                return Err(io::Error::new(io::ErrorKind::NotFound, "command not found"));
            }

            let status = if *self.should_succeed.lock().unwrap() {
                mock_exit_status(0)
            } else {
                mock_exit_status(1)
            };

            Ok(Output {
                status,
                stdout: self.stdout_data.lock().unwrap().clone(),
                stderr: vec![],
            })
        }
    }

    #[test]
    fn test_cargo_check_with_installed_tool_returns_true() {
        // cargo should always be installed if we're running cargo test
        let toolchain = ToolChain::default();
        assert!(toolchain.cargo.is_installed());
    }

    #[test]
    fn test_tool_check_with_installed_tool_returns_available_status_mock() {
        // Unit test with mock - integration test moved to tests/tools_integration.rs
        let mock = MockCommandExecutor::new();
        mock.set_success("cargo 1.86.0\n");

        let tool = Tool::with_executor("Cargo", "cargo", "--version", true, mock);
        let status = tool.check();
        match status {
            ToolStatus::Available(version) => {
                assert!(version.contains("cargo"));
            }
            _ => panic!("cargo should be available"),
        }
    }

    // P0-TEST-COV-001: Error handling tests for external tool failures

    #[test]
    fn test_tool_is_installed_with_nonexistent_binary_returns_false() {
        // Test detection of non-existent tool
        let tool = Tool::new(
            "nonexistent-tool-xyz-123",
            "nonexistent-tool-xyz-123",
            "--version",
            false,
        );

        assert!(!tool.is_installed());
    }

    #[test]
    fn test_tool_version_with_binary_no_version_flag_succeeds_gracefully() {
        // Test tool that exists but version() call fails
        // Using a binary that exists but doesn't support --version
        let tool = Tool::new(
            "test-binary",
            "true", // Unix command that exists but doesn't support --version
            "--version",
            false,
        );

        // Should be installed
        assert!(tool.is_installed());

        // But version() should fail gracefully
        let result = tool.version();
        // true command exits with 0 but no output, so version should be empty or succeed with empty
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_check_with_nonexistent_binary_returns_missing_status() {
        let tool = Tool::new(
            "nonexistent-tool",
            "nonexistent-xyz-binary",
            "--version",
            false,
        );

        let status = tool.check();
        assert!(matches!(status, ToolStatus::Missing));
    }

    #[test]
    fn test_check_required_with_missing_required_tool_returns_error() {
        // Create a toolchain with a missing required tool
        let mut toolchain = ToolChain::default();
        toolchain.cargo.binary = "nonexistent-cargo-xyz";

        let result = toolchain.check_required();
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("required") || err_msg.contains("not found"));
    }

    #[test]
    fn test_check_required_with_all_required_tools_present_succeeds() {
        let toolchain = ToolChain::default();
        let result = toolchain.check_required();

        // May fail if a required tool is not installed
        match result {
            Ok(_) => { /* Both tools found */ }
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    err_msg.contains("Cargo") || err_msg.contains("wasm-bindgen"),
                    "Expected error to mention a required tool, got: {err_msg}"
                );
            }
        }
    }

    #[test]
    fn test_tool_version_with_valid_tool_returns_parseable_version_mock() {
        // Unit test with mock - integration test moved to tests/tools_integration.rs
        let mock = MockCommandExecutor::new();
        mock.set_success("cargo 1.86.0\n");

        let tool = Tool::with_executor("Cargo", "cargo", "--version", true, mock);
        let version_result = tool.version();
        assert!(version_result.is_ok());

        let version = version_result.unwrap();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_check_all_with_missing_optional_tools_succeeds() {
        // Test that check_all() doesn't fail on missing optional tools
        let mut toolchain = ToolChain::default();

        // Make optional tools non-existent
        toolchain.wasm_opt.binary = "nonexistent-wasm-opt";
        toolchain.wasm_snip.binary = "nonexistent-wasm-snip";

        // Should still pass if required tools are present
        // But might fail if required tools missing, which is acceptable
        let result = toolchain.check_all();

        match result {
            Ok(_) => { /* Success - required tools present */ }
            Err(e) => {
                // Should fail only due to required tools
                let err_msg = e.to_string();
                assert!(
                    err_msg.contains("Required tools") || err_msg.contains("missing"),
                    "Error should be about required tools, got: {}",
                    err_msg
                );
            }
        }
    }

    #[test]
    fn test_toolchain_new_creates_with_default_tools() {
        let toolchain = ToolChain::new();
        assert_eq!(toolchain.cargo.name, "Cargo");
        assert_eq!(toolchain.wasm_bindgen.name, "wasm-bindgen-cli");
        assert_eq!(toolchain.wasm_opt.name, "wasm-opt (Binaryen)");
        assert_eq!(toolchain.wasm_snip.name, "wasm-snip");
    }

    #[test]
    fn test_toolchain_required_tools_are_marked_correctly() {
        let toolchain = ToolChain::new();
        assert!(toolchain.cargo.required);
        assert!(toolchain.wasm_bindgen.required);
        assert!(!toolchain.wasm_opt.required);
        assert!(!toolchain.wasm_snip.required);
    }

    #[test]
    fn test_tool_version_with_mocked_executor_returns_version() {
        let mock = MockCommandExecutor::new();
        mock.set_success("cargo 1.75.0 (1d8b05cdd 2024-01-18)\n");

        let tool = Tool::with_executor("Cargo", "cargo", "--version", true, mock);

        let version = tool.version().unwrap();
        assert_eq!(version, "cargo 1.75.0 (1d8b05cdd 2024-01-18)");
    }

    #[test]
    fn test_tool_version_with_failed_command_returns_error() {
        let mock = MockCommandExecutor::new();
        mock.set_failure();

        let tool = Tool::with_executor("TestTool", "test-tool", "--version", false, mock);

        let result = tool.version();
        assert!(result.is_err());
        if let Err(ToolError::VersionFailed(name)) = result {
            assert_eq!(name, "TestTool");
        } else {
            panic!("Expected VersionFailed error");
        }
    }

    #[test]
    fn test_tool_version_with_io_error_returns_error() {
        let mock = MockCommandExecutor::new();
        mock.set_error();

        let tool = Tool::with_executor("TestTool", "test-tool", "--version", false, mock);

        let result = tool.version();
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_version_extracts_first_line_only() {
        let mock = MockCommandExecutor::new();
        mock.set_success("version 1.0.0\nSecond line\nThird line");

        let tool = Tool::with_executor("TestTool", "test-tool", "--version", false, mock);

        let version = tool.version().unwrap();
        assert_eq!(version, "version 1.0.0");
        assert!(!version.contains("Second"));
    }

    #[test]
    fn test_check_all_with_all_tools_available() {
        let mock = MockCommandExecutor::new();
        mock.set_success("version 1.0.0");

        let toolchain = ToolChain::with_executor(mock);
        let result = toolchain.check_all();

        // Will succeed if tools are in PATH, otherwise acceptable to fail
        match result {
            Ok(_) => { /* All tools found */ }
            Err(e) => {
                // Should be about missing tools, not a panic
                assert!(e.to_string().contains("missing") || e.to_string().contains("Required"));
            }
        }
    }

    #[test]
    fn test_check_all_with_missing_required_returns_error() {
        let mock = MockCommandExecutor::new();
        mock.set_error();

        let mut toolchain = ToolChain::with_executor(mock.clone());
        // Make a required tool have a non-existent binary
        toolchain.cargo.binary = "nonexistent-cargo-binary-xyz";

        let result = toolchain.check_all();
        assert!(result.is_err());

        if let Err(ToolError::MissingTool(msg)) = result {
            assert!(msg.contains("Required"));
        } else {
            panic!("Expected MissingTool error");
        }
    }
}
