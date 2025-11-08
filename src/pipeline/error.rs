//! Error types for the build pipeline

use thiserror::Error;

/// Errors that can occur during pipeline execution
#[derive(Error, Debug)]
pub enum PipelineError {
    /// I/O error during build
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Tool error
    #[error("Tool error: {0}")]
    Tool(#[from] crate::tools::ToolError),

    /// Build command failed
    #[error("Build failed: {0}")]
    BuildFailed(String),

    /// Tool execution failed
    #[error("Tool execution failed: {0}")]
    ToolFailed(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),
}
