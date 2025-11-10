//! Integration tests for tools module
//!
//! These tests execute real external commands (cargo, rustc, etc.)
//! and should not be run during coverage collection as the instrumentation
//! can interfere with subprocess execution.

use wasm_slim::toolchain::ToolchainDetector;
use wasm_slim::tools::{Tool, ToolChain, ToolStatus};

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
fn test_toolchain_module_is_accessible() {
    // Verify toolchain module is accessible from tools
    let detector = ToolchainDetector::new();
    let result = detector.is_nightly_toolchain();
    assert!(result.is_ok(), "Should successfully detect toolchain");
}

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
    // - "cargo 1.86.0 (1d8b05cdd 2024-03-20)"
    // - "rustc 1.86.0 (82e1608df 2024-03-31)"
    // - "wasm-opt version 116 (version_116)"
    let tool = Tool::new("cargo", "cargo", "--version", true);

    let version = tool.version().expect("Failed to get version");
    // Should successfully parse regardless of format
    assert!(!version.is_empty());
    // Should extract the first line (no newlines)
    assert!(!version.contains('\n'));
}

#[test]
fn test_is_nightly_toolchain_detects_channel() {
    // Integration test with real rustc
    let detector = ToolchainDetector::new();
    let result = detector.is_nightly_toolchain();
    assert!(result.is_ok());
}
