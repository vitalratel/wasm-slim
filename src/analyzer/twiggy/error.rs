//! Error types for twiggy WASM analyzer

use thiserror::Error;

/// Errors that can occur during twiggy analysis
#[derive(Error, Debug)]
pub enum TwiggyAnalysisError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// WASM file not found
    #[error("WASM file not found: {0}")]
    WasmFileNotFound(String),

    /// Twiggy command failed
    #[error("Twiggy {0} command failed with status {1}")]
    CommandFailed(String, i32),

    /// UTF-8 conversion error
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}
