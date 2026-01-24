//! Enhanced error types with contextual suggestions
//!
//! Provides structured error types that include:
//! - Actionable error messages
//! - Suggested fixes and recovery actions
//! - Documentation links
//! - Proper exit codes for CI/CD
//!
//! # Examples
//!
//! ```no_run
//! use wasm_slim::error::WasmSlimError;
//! use wasm_slim::pipeline::{BuildPipeline, PipelineConfig};
//!
//! let pipeline = BuildPipeline::new(".", PipelineConfig::default());
//!
//! match pipeline.build() {
//!     Ok(metrics) => {
//!         println!("Build successful: {} bytes", metrics.after_bytes);
//!     }
//!     Err(e) => {
//!         eprintln!("Build failed: {}", e);
//!         // Errors contain actionable context
//!         std::process::exit(1);
//!     }
//! }
//! ```

use std::path::PathBuf;
use thiserror::Error;

use crate::pipeline::PipelineError;

/// Enhanced wasm-slim errors with contextual suggestions
#[derive(Error, Debug)]
pub enum WasmSlimError {
    /// Required tool is not installed
    #[error("Tool not installed: {tool}")]
    ToolMissing {
        /// Tool name
        tool: String,
        /// Installation command
        install_cmd: String,
        /// Optional documentation URL
        docs_url: Option<String>,
    },

    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    ConfigNotFound {
        /// Path to config file
        path: PathBuf,
        #[source]
        /// IO error source
        source: std::io::Error,
    },

    /// Invalid template name
    #[error("Invalid template: '{name}'")]
    InvalidTemplate {
        /// Invalid template name
        name: String,
        /// List of valid template names
        available: Vec<String>,
    },

    /// File not found during operation
    #[error("File not found: {path}")]
    FileNotFound {
        /// Path to missing file
        path: PathBuf,
        /// Operation that required the file
        operation: String,
    },

    /// WASM file required but not provided
    #[error("WASM file required for {operation}")]
    WasmFileRequired {
        /// Operation requiring WASM file
        operation: String,
    },

    /// Size budget exceeded
    #[error("WASM bundle size ({actual} bytes) exceeds maximum ({max_allowed} bytes)")]
    BudgetExceeded {
        /// Actual bundle size
        actual: u64,
        /// Maximum allowed size
        max_allowed: u64,
        /// Percentage over budget
        percentage_over: f64,
    },

    /// Build command failed
    #[error("Build command failed: {command}")]
    BuildFailed {
        /// Command that failed
        command: String,
        /// Error output
        stderr: String,
    },

    /// Invalid analysis mode
    #[error("Unknown analysis mode: '{mode}'")]
    InvalidAnalysisMode {
        /// Invalid mode name
        mode: String,
        /// List of valid modes
        valid_modes: Vec<String>,
    },

    /// Generic I/O error with context
    #[error("I/O error: {context}")]
    Io {
        /// Context about where the error occurred
        context: String,
        #[source]
        /// IO error source
        source: std::io::Error,
    },

    /// Pipeline error during build
    #[error("pipeline error: {0}")]
    Pipeline(#[from] PipelineError),
}

impl WasmSlimError {
    /// Get actionable suggestion for resolving this error.
    ///
    /// Returns a user-friendly suggestion for how to fix the error, if available.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::error::WasmSlimError;
    ///
    /// let error = WasmSlimError::ToolMissing {
    ///     tool: "wasm-opt".to_string(),
    ///     install_cmd: "cargo install wasm-opt".to_string(),
    ///     docs_url: None,
    /// };
    ///
    /// let suggestion = error.suggestion();
    /// assert!(suggestion.is_some());
    /// assert!(suggestion.unwrap().contains("cargo install"));
    /// ```
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::ToolMissing { install_cmd, .. } => Some(format!("Install with: {}", install_cmd)),
            Self::ConfigNotFound { .. } => {
                Some("Run 'wasm-slim init' to create a configuration file".to_string())
            }
            Self::InvalidTemplate { available, .. } => Some(format!(
                "Available templates: {}\nRun 'wasm-slim init --list' to see all templates",
                available.join(", ")
            )),
            Self::FileNotFound { path, operation } => Some(format!(
                "Ensure {} exists before running {}",
                path.display(),
                operation
            )),
            Self::WasmFileRequired { operation } => Some(format!(
                "Provide WASM file path: wasm-slim analyze {} <path/to/file.wasm>",
                operation
            )),
            Self::BudgetExceeded {
                percentage_over, ..
            } => Some(format!(
                "Bundle is {:.1}% over budget. Consider:\n  \
                     - Using 'aggressive' template\n  \
                     - Running 'wasm-slim analyze assets' to find large embedded resources\n  \
                     - Running 'wasm-slim analyze deps' to identify heavy dependencies",
                percentage_over
            )),
            Self::BuildFailed { stderr, .. } => {
                if stderr.contains("wasm32-unknown-unknown") {
                    Some(
                        "Install WASM target: rustup target add wasm32-unknown-unknown".to_string(),
                    )
                } else {
                    Some("Check the build errors above and fix compilation issues".to_string())
                }
            }
            Self::InvalidAnalysisMode { valid_modes, .. } => {
                Some(format!("Valid modes: {}", valid_modes.join(", ")))
            }
            Self::Io { context, .. } => Some(format!(
                "Check file permissions and that {} is accessible",
                context
            )),
            Self::Pipeline(e) => {
                let msg = e.to_string();
                if msg.contains("wasm32-unknown-unknown") {
                    Some(
                        "Install WASM target: rustup target add wasm32-unknown-unknown".to_string(),
                    )
                } else if msg.contains("wasm-opt") || msg.contains("wasm-bindgen") {
                    Some("Ensure required tools are installed. Run: cargo install wasm-bindgen-cli && cargo install wasm-opt".to_string())
                } else {
                    Some("Check the build errors above and fix compilation issues".to_string())
                }
            }
        }
    }

    /// Get documentation URL for this error.
    ///
    /// Returns a URL to relevant documentation for resolving this error type.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::error::WasmSlimError;
    ///
    /// let error = WasmSlimError::ToolMissing {
    ///     tool: "wasm-bindgen".to_string(),
    ///     install_cmd: "cargo install wasm-bindgen-cli".to_string(),
    ///     docs_url: Some("https://rustwasm.github.io/wasm-bindgen/".to_string()),
    /// };
    ///
    /// assert!(error.docs_url().is_some());
    /// assert_eq!(error.docs_url().unwrap(), "https://rustwasm.github.io/wasm-bindgen/");
    /// ```
    pub fn docs_url(&self) -> Option<&str> {
        match self {
            Self::ToolMissing { docs_url, .. } => docs_url.as_deref(),
            Self::ConfigNotFound { .. } => {
                Some("https://github.com/vitalratel/wasm-slim#configuration")
            }
            Self::BudgetExceeded { .. } => {
                Some("https://github.com/vitalratel/wasm-slim#ci-cd-integration")
            }
            Self::Pipeline(_) => Some("https://github.com/vitalratel/wasm-slim#build-pipeline"),
            _ => None,
        }
    }

    /// Get appropriate exit code for this error.
    ///
    /// Returns Unix-style exit codes based on the error type, following sysexits.h conventions.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::error::WasmSlimError;
    ///
    /// let error = WasmSlimError::ToolMissing {
    ///     tool: "wasm-opt".to_string(),
    ///     install_cmd: "cargo install wasm-opt".to_string(),
    ///     docs_url: None,
    /// };
    ///
    /// assert_eq!(error.exit_code(), 127); // Command not found
    ///
    /// let budget_error = WasmSlimError::BudgetExceeded {
    ///     actual: 1500000, // 1500 KB in bytes
    ///     max_allowed: 1000000, // 1000 KB in bytes
    ///     percentage_over: 50.0,
    /// };
    ///
    /// assert_eq!(budget_error.exit_code(), 1); // Generic error for CI failure
    /// ```
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::ToolMissing { .. } => 127, // Command not found (Unix convention)
            Self::ConfigNotFound { .. } => 66, // EX_NOINPUT (sysexits.h)
            Self::InvalidTemplate { .. } => 65, // EX_DATAERR
            Self::FileNotFound { .. } => 66, // EX_NOINPUT
            Self::WasmFileRequired { .. } => 64, // EX_USAGE
            Self::BudgetExceeded { .. } => 1, // Generic error (CI should fail)
            Self::BuildFailed { .. } => 1,   // Generic error
            Self::InvalidAnalysisMode { .. } => 64, // EX_USAGE
            Self::Io { .. } => 74,           // EX_IOERR
            Self::Pipeline(_) => 1,          // Generic error (build failed)
        }
    }
}

impl WasmSlimError {
    /// Returns the pipeline error if this is a `Pipeline` variant.
    pub fn as_pipeline_error(&self) -> Option<&PipelineError> {
        match self {
            Self::Pipeline(e) => Some(e),
            _ => None,
        }
    }
}

/// Error formatter with colors and structured output
pub struct ErrorFormatter;

impl ErrorFormatter {
    /// Format error with suggestions and documentation links
    pub fn format(error: &anyhow::Error) -> String {
        use console::style;

        let mut output = String::new();

        // Main error message
        output.push_str(&format!("{} {}\n", style("error:").red().bold(), error));

        // Error chain (caused by)
        let mut source = error.source();
        let mut indent = 1;
        while let Some(err) = source {
            output.push_str(&format!(
                "{}{} {}\n",
                "  ".repeat(indent),
                style("caused by:").yellow(),
                err
            ));
            source = err.source();
            indent += 1;
        }

        // Try to downcast to WasmSlimError for suggestions
        if let Some(ws_error) = error.downcast_ref::<WasmSlimError>() {
            // Suggestions
            if let Some(suggestion) = ws_error.suggestion() {
                output.push_str(&format!(
                    "\n{} {}\n",
                    style("help:").cyan().bold(),
                    suggestion
                ));
            }

            // Documentation link
            if let Some(docs) = ws_error.docs_url() {
                output.push_str(&format!("{} {}\n", style("docs:").blue(), docs));
            }
        }

        output
    }

    /// Get exit code from error
    pub fn exit_code(error: &anyhow::Error) -> i32 {
        if let Some(ws_error) = error.downcast_ref::<WasmSlimError>() {
            ws_error.exit_code()
        } else {
            1 // Generic error
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_missing_has_suggestion() {
        let err = WasmSlimError::ToolMissing {
            tool: "twiggy".to_string(),
            install_cmd: "cargo install twiggy".to_string(),
            docs_url: Some("https://rustwasm.github.io/twiggy/".to_string()),
        };

        let suggestion = err
            .suggestion()
            .expect("ToolMissing should have suggestion");
        assert!(suggestion.contains("cargo install twiggy"));
    }

    #[test]
    fn test_budget_exceeded_has_actionable_suggestion() {
        let err = WasmSlimError::BudgetExceeded {
            actual: 1_500_000,
            max_allowed: 1_000_000,
            percentage_over: 50.0,
        };

        let suggestion = err
            .suggestion()
            .expect("BudgetExceeded should have suggestion");
        assert!(suggestion.contains("aggressive"));
        assert!(suggestion.contains("analyze assets"));
    }

    #[test]
    fn test_invalid_template_lists_alternatives() {
        let err = WasmSlimError::InvalidTemplate {
            name: "foo".to_string(),
            available: vec![
                "minimal".to_string(),
                "balanced".to_string(),
                "aggressive".to_string(),
            ],
        };

        let suggestion = err
            .suggestion()
            .expect("InvalidTemplate should have suggestion");
        assert!(suggestion.contains("minimal"));
        assert!(suggestion.contains("balanced"));
        assert!(suggestion.contains("aggressive"));
    }

    #[test]
    fn test_exit_codes_follow_conventions() {
        let tool_err = WasmSlimError::ToolMissing {
            tool: "test".to_string(),
            install_cmd: "test".to_string(),
            docs_url: None,
        };
        assert_eq!(tool_err.exit_code(), 127); // Command not found

        let config_err = WasmSlimError::ConfigNotFound {
            path: PathBuf::from("test"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
        };
        assert_eq!(config_err.exit_code(), 66); // No input file
    }

    #[test]
    fn test_config_not_found_has_suggestion() {
        let err = WasmSlimError::ConfigNotFound {
            path: PathBuf::from("wasm-slim.toml"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };

        let suggestion = err
            .suggestion()
            .expect("ConfigNotFound should have suggestion");
        assert!(suggestion.contains("wasm-slim init"));
    }

    #[test]
    fn test_file_not_found_has_context() {
        let err = WasmSlimError::FileNotFound {
            path: PathBuf::from("output.wasm"),
            operation: "analyze".to_string(),
        };

        let suggestion = err
            .suggestion()
            .expect("FileNotFound should have suggestion");
        assert!(suggestion.contains("output.wasm"));
        assert!(suggestion.contains("analyze"));
    }

    #[test]
    fn test_wasm_file_required_has_usage_hint() {
        let err = WasmSlimError::WasmFileRequired {
            operation: "bloat".to_string(),
        };

        let suggestion = err
            .suggestion()
            .expect("WasmFileRequired should have suggestion");
        assert!(suggestion.contains("bloat"));
        assert!(suggestion.contains(".wasm"));
    }

    #[test]
    fn test_build_failed_with_missing_target() {
        let err = WasmSlimError::BuildFailed {
            command: "cargo build".to_string(),
            stderr: "error: target wasm32-unknown-unknown not found".to_string(),
        };

        let suggestion = err
            .suggestion()
            .expect("BuildFailed should have suggestion for missing target");
        assert!(suggestion.contains("rustup target add"));
        assert!(suggestion.contains("wasm32-unknown-unknown"));
    }

    #[test]
    fn test_build_failed_generic_error() {
        let err = WasmSlimError::BuildFailed {
            command: "cargo build".to_string(),
            stderr: "error: could not compile".to_string(),
        };

        let suggestion = err
            .suggestion()
            .expect("BuildFailed should have suggestion");
        assert!(suggestion.contains("build errors"));
    }

    #[test]
    fn test_invalid_analysis_mode_lists_valid_modes() {
        let err = WasmSlimError::InvalidAnalysisMode {
            mode: "invalid".to_string(),
            valid_modes: vec![
                "bloat".to_string(),
                "assets".to_string(),
                "deps".to_string(),
            ],
        };

        let suggestion = err
            .suggestion()
            .expect("InvalidAnalysisMode should have suggestion");
        assert!(suggestion.contains("bloat"));
        assert!(suggestion.contains("assets"));
        assert!(suggestion.contains("deps"));
    }

    #[test]
    fn test_io_error_has_context() {
        let err = WasmSlimError::Io {
            context: "reading Cargo.toml".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };

        let suggestion = err.suggestion().expect("Io error should have suggestion");
        assert!(suggestion.contains("permissions"));
        assert!(suggestion.contains("reading Cargo.toml"));
    }

    #[test]
    fn test_all_error_variants_have_exit_codes() {
        let errors = vec![
            WasmSlimError::ToolMissing {
                tool: "test".to_string(),
                install_cmd: "test".to_string(),
                docs_url: None,
            },
            WasmSlimError::ConfigNotFound {
                path: PathBuf::from("test"),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
            },
            WasmSlimError::InvalidTemplate {
                name: "test".to_string(),
                available: vec![],
            },
            WasmSlimError::FileNotFound {
                path: PathBuf::from("test"),
                operation: "test".to_string(),
            },
            WasmSlimError::WasmFileRequired {
                operation: "test".to_string(),
            },
            WasmSlimError::BudgetExceeded {
                actual: 100,
                max_allowed: 50,
                percentage_over: 100.0,
            },
            WasmSlimError::BuildFailed {
                command: "test".to_string(),
                stderr: "test".to_string(),
            },
            WasmSlimError::InvalidAnalysisMode {
                mode: "test".to_string(),
                valid_modes: vec![],
            },
            WasmSlimError::Io {
                context: "test".to_string(),
                source: std::io::Error::other("test"),
            },
            WasmSlimError::Pipeline(crate::pipeline::PipelineError::BuildFailed(
                "test".to_string(),
            )),
        ];

        for err in errors {
            let exit_code = err.exit_code();
            assert!(
                exit_code > 0,
                "Error {:?} should have non-zero exit code",
                err
            );
            assert!(exit_code < 256, "Exit code should fit in a byte");
        }
    }

    #[test]
    fn test_all_error_variants_have_suggestions() {
        let errors = vec![
            WasmSlimError::ToolMissing {
                tool: "test".to_string(),
                install_cmd: "cargo install test".to_string(),
                docs_url: None,
            },
            WasmSlimError::ConfigNotFound {
                path: PathBuf::from("test"),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
            },
            WasmSlimError::InvalidTemplate {
                name: "test".to_string(),
                available: vec!["minimal".to_string()],
            },
            WasmSlimError::FileNotFound {
                path: PathBuf::from("test.wasm"),
                operation: "analyze".to_string(),
            },
            WasmSlimError::WasmFileRequired {
                operation: "bloat".to_string(),
            },
            WasmSlimError::BudgetExceeded {
                actual: 100,
                max_allowed: 50,
                percentage_over: 100.0,
            },
            WasmSlimError::BuildFailed {
                command: "cargo build".to_string(),
                stderr: "compilation failed".to_string(),
            },
            WasmSlimError::InvalidAnalysisMode {
                mode: "invalid".to_string(),
                valid_modes: vec!["bloat".to_string()],
            },
            WasmSlimError::Io {
                context: "reading file".to_string(),
                source: std::io::Error::other("test"),
            },
            WasmSlimError::Pipeline(crate::pipeline::PipelineError::BuildFailed(
                "build failed".to_string(),
            )),
        ];

        for err in &errors {
            let suggestion = err.suggestion();
            assert!(
                suggestion.is_some(),
                "Error {:?} should have a suggestion",
                err
            );
            assert!(
                !suggestion.unwrap().is_empty(),
                "Suggestion should not be empty"
            );
        }
    }

    #[test]
    fn test_pipeline_error_has_suggestion() {
        let err = WasmSlimError::Pipeline(crate::pipeline::PipelineError::BuildFailed(
            "cargo build failed".to_string(),
        ));

        let suggestion = err
            .suggestion()
            .expect("Pipeline error should have suggestion");
        assert!(!suggestion.is_empty());
    }

    #[test]
    fn test_pipeline_error_with_wasm_target_has_install_suggestion() {
        let err = WasmSlimError::Pipeline(crate::pipeline::PipelineError::BuildFailed(
            "target wasm32-unknown-unknown not found".to_string(),
        ));

        let suggestion = err
            .suggestion()
            .expect("Pipeline error should have suggestion");
        assert!(suggestion.contains("rustup target add"));
        assert!(suggestion.contains("wasm32-unknown-unknown"));
    }

    #[test]
    fn test_pipeline_error_accessor() {
        let pipeline_err = crate::pipeline::PipelineError::BuildFailed("test".to_string());
        let err = WasmSlimError::Pipeline(pipeline_err);

        assert!(err.as_pipeline_error().is_some());

        let other_err = WasmSlimError::Io {
            context: "test".to_string(),
            source: std::io::Error::other("test"),
        };
        assert!(other_err.as_pipeline_error().is_none());
    }
}
