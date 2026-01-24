//! Build pipeline executor
//!
//! Orchestrates the complete WASM optimization pipeline:
//! 1. cargo build --release --target wasm32-unknown-unknown
//! 2. wasm-bindgen with optimized flags
//! 3. wasm-opt -Oz for aggressive size optimization
//! 4. wasm-snip to remove panic infrastructure (optional)

use crate::infra::{CommandExecutor, FileSystem, RealCommandExecutor, RealFileSystem};
use std::path::Path;

use crate::tools::ToolChain;

use super::build_orchestrator::BuildOrchestrator;
use super::config::PipelineConfig;
use super::error::PipelineError;
use super::metrics::SizeMetrics;

/// Main build pipeline orchestrator
///
/// Coordinates cargo → wasm-bindgen → wasm-opt → wasm-snip workflow.
pub struct BuildPipeline<FS: FileSystem = RealFileSystem, CE: CommandExecutor = RealCommandExecutor>
{
    orchestrator: BuildOrchestrator<FS, CE>,
}

impl BuildPipeline {
    /// Create a new build pipeline with the given configuration
    ///
    /// # Arguments
    /// * `project_root` - Path to the WASM project root (where Cargo.toml is located)
    /// * `config` - Pipeline configuration (use `PipelineConfig::default()` for sensible defaults)
    ///
    /// # Examples
    ///
    /// Basic usage with default configuration:
    /// ```no_run
    /// use wasm_slim::pipeline::{BuildPipeline, PipelineConfig};
    /// use std::path::Path;
    ///
    /// let config = PipelineConfig::default();
    /// let pipeline = BuildPipeline::new("./my-wasm-project", config);
    /// ```
    ///
    /// Custom configuration for Node.js target:
    /// ```no_run
    /// use wasm_slim::pipeline::{BindgenTarget, BuildPipeline, PipelineConfig, WasmOptLevel};
    ///
    /// let mut config = PipelineConfig::default();
    /// config.bindgen_target = BindgenTarget::NodeJs;
    /// config.opt_level = WasmOptLevel::O3;
    /// config.run_wasm_snip = true; // Enable dead code removal
    ///
    /// let pipeline = BuildPipeline::new("./backend-wasm", config);
    /// ```
    ///
    /// Minimal optimization for faster builds:
    /// ```no_run
    /// use wasm_slim::pipeline::{BuildPipeline, PipelineConfig, WasmOptLevel};
    ///
    /// let mut config = PipelineConfig::default();
    /// config.opt_level = WasmOptLevel::O1;
    /// config.run_wasm_snip = false;
    ///
    /// let pipeline = BuildPipeline::new("./dev-build", config);
    /// ```
    pub fn new(project_root: impl AsRef<Path>, config: PipelineConfig) -> Self {
        Self::with_executors(project_root, config, RealFileSystem, RealCommandExecutor)
    }
}

impl<FS: FileSystem + Clone, CE: CommandExecutor + Clone> BuildPipeline<FS, CE> {
    /// Create a new build pipeline with custom filesystem and command executor implementations
    pub fn with_executors(
        project_root: impl AsRef<Path>,
        config: PipelineConfig,
        fs: FS,
        cmd_executor: CE,
    ) -> Self {
        let orchestrator = BuildOrchestrator::new(
            project_root.as_ref().to_path_buf(),
            config,
            ToolChain::with_executor(cmd_executor.clone()),
            fs,
            cmd_executor,
        );
        Self { orchestrator }
    }

    /// Run the complete build pipeline
    ///
    /// Executes: cargo build → wasm-bindgen → wasm-opt → (optional) wasm-snip
    ///
    /// # Returns
    /// Size metrics showing before/after optimization results
    ///
    /// # Errors
    /// Returns error if:
    /// - Cargo.toml not found in project root
    /// - Required tools (wasm-bindgen-cli) not installed
    /// - Compilation fails
    /// - WASM binary not found after build
    ///
    /// # Examples
    ///
    /// Basic build with error handling:
    /// ```no_run
    /// use wasm_slim::pipeline::{BuildPipeline, PipelineConfig};
    ///
    /// let config = PipelineConfig::default();
    /// let pipeline = BuildPipeline::new("./my-wasm-project", config);
    ///
    /// match pipeline.build() {
    ///     Ok(metrics) => {
    ///         println!("Build successful!");
    ///         println!("Original size: {} bytes", metrics.before_bytes);
    ///         println!("Optimized size: {} bytes", metrics.after_bytes);
    ///         println!("Reduction: {:.1}%", metrics.reduction_percent());
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Build failed: {}", e);
    ///         std::process::exit(1);
    ///     }
    /// }
    /// ```
    ///
    /// Build with metrics interpretation:
    /// ```no_run
    /// use wasm_slim::pipeline::{BuildPipeline, PipelineConfig, SizeMetrics};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let config = PipelineConfig::default();
    /// let pipeline = BuildPipeline::new("./app", config);
    ///
    /// let metrics = pipeline.build()?;
    ///
    /// // Check if optimization was effective
    /// if metrics.reduction_percent() > 10.0 {
    ///     println!("✓ Good optimization: {:.1}% reduction", metrics.reduction_percent());
    /// } else {
    ///     println!("⚠ Limited optimization: {:.1}% reduction", metrics.reduction_percent());
    ///     println!("  Consider enabling wasm-snip or using higher opt level");
    /// }
    ///
    /// // Human-readable sizes
    /// use wasm_slim::fmt::format_bytes;
    /// println!("Before: {}", format_bytes(metrics.before_bytes));
    /// println!("After:  {}", format_bytes(metrics.after_bytes));
    /// println!("Saved:  {}", format_bytes(metrics.reduction_bytes() as u64));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Integration with budget checking:
    /// ```no_run
    /// use wasm_slim::pipeline::{BuildPipeline, PipelineConfig};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let config = PipelineConfig::default();
    /// let pipeline = BuildPipeline::new("./app", config);
    ///
    /// let metrics = pipeline.build()?;
    ///
    /// // Enforce size budget for CI/CD
    /// let max_size_kb = 500;
    /// let actual_size_kb = metrics.after_bytes / 1024;
    ///
    /// if actual_size_kb > max_size_kb {
    ///     eprintln!("❌ Size budget exceeded!");
    ///     eprintln!("   Budget: {} KB", max_size_kb);
    ///     eprintln!("   Actual: {} KB", actual_size_kb);
    ///     std::process::exit(1);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Recovery from build failures:
    /// ```no_run
    /// use wasm_slim::pipeline::{BuildPipeline, PipelineConfig, PipelineError, SizeMetrics};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let config = PipelineConfig::default();
    /// let pipeline = BuildPipeline::new("./app", config);
    ///
    /// match pipeline.build() {
    ///     Ok(metrics) => {
    ///         use wasm_slim::fmt::format_bytes;
    ///         println!("✓ Build succeeded: {}", format_bytes(metrics.after_bytes));
    ///     }
    ///     Err(PipelineError::Tool(e)) => {
    ///         eprintln!("❌ Tool error: {}", e);
    ///         eprintln!("   Make sure wasm-bindgen-cli is installed");
    ///         std::process::exit(127);
    ///     }
    ///     Err(PipelineError::BuildFailed(msg)) => {
    ///         eprintln!("❌ Build failed: {}", msg);
    ///         std::process::exit(1);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("❌ Unexpected error: {}", e);
    ///         std::process::exit(1);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(&self) -> Result<SizeMetrics, PipelineError> {
        self.orchestrator.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::config::{BindgenTarget, WasmOptLevel, WasmTarget};

    #[test]
    fn test_new_pipeline_creates_orchestrator() {
        let config = PipelineConfig::default();
        let _pipeline = BuildPipeline::new("/test/project", config);
        // Pipeline successfully created with orchestrator
    }

    #[test]
    fn test_new_pipeline_with_custom_config() {
        let config = PipelineConfig {
            target: WasmTarget::Wasm32Wasi,
            opt_level: WasmOptLevel::O3,
            ..Default::default()
        };

        let _pipeline = BuildPipeline::new("/test/project", config.clone());
        // Pipeline successfully created with custom config
    }

    #[test]
    fn test_pipeline_with_custom_bindgen_target() {
        let config = PipelineConfig {
            bindgen_target: BindgenTarget::NodeJs,
            ..Default::default()
        };

        let _pipeline = BuildPipeline::new("/test/project", config);
        // Pipeline successfully created with custom bindgen target
    }

    #[test]
    fn test_pipeline_with_wasm_snip_enabled() {
        let config = PipelineConfig {
            run_wasm_snip: true,
            ..Default::default()
        };

        let _pipeline = BuildPipeline::new("/test/project", config);
        // Pipeline successfully created with wasm-snip enabled
    }

    #[test]
    fn test_pipeline_with_wasm_snip_disabled() {
        let config = PipelineConfig {
            run_wasm_snip: false,
            ..Default::default()
        };

        let _pipeline = BuildPipeline::new("/test/project", config);
        // Pipeline successfully created with wasm-snip disabled
    }

    #[test]
    fn test_pipeline_default_target_is_wasm32_unknown_unknown() {
        let config = PipelineConfig::default();
        assert_eq!(config.target.as_str(), "wasm32-unknown-unknown");
        let _pipeline = BuildPipeline::new("/test/project", config);
    }

    #[test]
    fn test_pipeline_default_opt_level_is_oz() {
        let config = PipelineConfig::default();
        assert_eq!(config.opt_level, WasmOptLevel::Oz);
        let _pipeline = BuildPipeline::new("/test/project", config);
    }

    #[test]
    fn test_pipeline_default_bindgen_target_is_web() {
        let config = PipelineConfig::default();
        assert_eq!(config.bindgen_target.as_str(), "web");
        let _pipeline = BuildPipeline::new("/test/project", config);
    }
}
