//! Command handlers for wasm-slim CLI
//!
//! This module contains all command implementations, organized by functionality.
//! Each submodule handles a specific CLI command.

pub mod analyze;
pub mod build;
pub mod compare;
pub mod completions;
pub mod init;
pub mod workflow;

// Re-export command functions for convenient access
pub use analyze::{
    analyze_assets, analyze_bloat, analyze_dependencies, analyze_features, analyze_wasm_binary,
    cmd_analyze,
};
pub use build::cmd_build;
pub use compare::cmd_compare;
pub use completions::cmd_completions;
pub use init::cmd_init;
pub use workflow::BuildWorkflow;
