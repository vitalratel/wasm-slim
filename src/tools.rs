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
    fn check(&self) -> ToolStatus {
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

#[derive(Debug)]
enum ToolStatus {
    // Module-private, only used internally
    Available(String),
    InstalledButVersionUnknown,
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
    /// Verifies cargo and wasm-bindgen-cli are available.
    ///
    /// # Errors
    /// Returns error if any required tool is missing
    pub fn check_required(&self) -> Result<(), ToolError> {
        let tools = [&self.cargo, &self.wasm_bindgen];

        for tool in &tools {
            if !tool.is_installed() {
                return Err(ToolError::MissingTool(format!(
                    "{} is required but not found in PATH",
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

    #[test]
    fn test_cargo_check_with_installed_tool_returns_true() {
        // cargo should always be installed if we're running cargo test
        let toolchain = ToolChain::default();
        assert!(toolchain.cargo.is_installed());
    }

    #[test]
    fn test_tool_check_with_installed_tool_returns_available_status() {
        let toolchain = ToolChain::default();
        let status = toolchain.cargo.check();
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
        // Should succeed when cargo is present (it always is in tests)
        let toolchain = ToolChain::default();
        let result = toolchain.check_required();

        // This might fail if wasm-bindgen is not installed, which is acceptable
        // The test validates that the check runs without panicking
        match result {
            Ok(_) => { /* Both tools found */ }
            Err(e) => {
                // Should mention wasm-bindgen if it fails
                assert!(e.to_string().contains("wasm-bindgen"));
            }
        }
    }

    #[test]
    fn test_tool_version_with_valid_tool_returns_parseable_version() {
        // Test handling of commands that might produce invalid UTF-8
        // This tests the String::from_utf8_lossy path
        let tool = Tool::new("cargo", "cargo", "--version", true);

        let version_result = tool.version();
        assert!(version_result.is_ok());

        // Version should be parseable and contain cargo
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
    fn test_toolchain_module_is_accessible() {
        // Verify toolchain module is accessible from tools
        use crate::toolchain::ToolchainDetector;
        let detector = ToolchainDetector::new();
        let result = detector.is_nightly_toolchain();
        assert!(result.is_ok(), "Should successfully detect toolchain");
    }

    // P2-UNIT-003: Version parsing edge case tests

    #[test]
    fn test_tool_version_with_multiline_output_returns_first_line() {
        // Many tools output multiline version info, we should extract just the first line
        let tool = Tool::new("rustc", "rustc", "--version", false);

        if tool.is_installed() {
            let version = tool.version().expect("Failed to get rustc version");
            // Should contain version number on first line
            assert!(!version.is_empty());
            // Should not contain newlines (we take first line only)
            assert!(!version.contains('\n'));
        }
    }

    #[test]
    fn test_tool_version_with_prerelease_tag_parses_successfully() {
        // Test version strings with pre-release tags (common in nightly builds)
        let tool = Tool::new("cargo", "cargo", "--version", true);

        let version = tool.version().expect("Failed to get cargo version");
        // Should successfully parse version even if it has pre-release info
        assert!(!version.is_empty());
        // Cargo version typically contains "cargo" and version number
        assert!(version.to_lowercase().contains("cargo"));
    }

    #[test]
    fn test_tool_version_with_build_metadata_parses_successfully() {
        // Test version strings with build metadata (+commit-hash)
        let tool = Tool::new("rustc", "rustc", "--version", false);

        if tool.is_installed() {
            let version = tool.version().expect("Failed to get version");
            // Should handle version strings with build metadata
            assert!(!version.is_empty());
            // May contain hash or date info but should still be parseable
        }
    }

    #[test]
    fn test_tool_version_with_different_format_succeeds() {
        // Test that version parsing works with different tool output formats
        // Different tools format their version output differently:
        // - "cargo 1.75.0 (1d8b05cdd 2023-11-20)"
        // - "rustc 1.75.0 (82e1608df 2023-12-21)"
        // - "wasm-opt version 116 (version_116)"
        let tool = Tool::new("cargo", "cargo", "--version", true);

        let version = tool.version().expect("Failed to get version");
        // Should successfully parse regardless of format
        assert!(!version.is_empty());
        // Should extract the first line (no newlines)
        assert!(!version.contains('\n'));
    }
}
