//! Pipeline configuration types

use std::path::PathBuf;

/// WebAssembly compilation target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WasmTarget {
    /// wasm32-unknown-unknown (default, for web)
    #[default]
    Wasm32UnknownUnknown,
    /// wasm32-wasi (WASI support)
    Wasm32Wasi,
    /// wasm32-unknown-emscripten (Emscripten)
    Wasm32UnknownEmscripten,
}

impl WasmTarget {
    /// Get the target triple as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wasm32UnknownUnknown => "wasm32-unknown-unknown",
            Self::Wasm32Wasi => "wasm32-wasi",
            Self::Wasm32UnknownEmscripten => "wasm32-unknown-emscripten",
        }
    }
}

/// wasm-bindgen output target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BindgenTarget {
    /// Web browser (ES modules)
    #[default]
    Web,
    /// Node.js (CommonJS)
    NodeJs,
    /// Bundler (webpack, rollup, etc.)
    Bundler,
    /// Deno runtime
    Deno,
    /// No ES modules (legacy)
    NoModules,
}

impl BindgenTarget {
    /// Get the bindgen target as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::NodeJs => "nodejs",
            Self::Bundler => "bundler",
            Self::Deno => "deno",
            Self::NoModules => "no-modules",
        }
    }
}

/// wasm-opt optimization levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmOptLevel {
    /// -O1: Quick optimization
    O1,
    /// -O2: Default optimization
    O2,
    /// -O3: Aggressive optimization
    O3,
    /// -O4: Very aggressive optimization
    O4,
    /// -Oz: Optimize for size (recommended)
    Oz,
}

impl WasmOptLevel {
    pub(super) fn as_arg(&self) -> &str {
        match self {
            WasmOptLevel::O1 => "-O1",
            WasmOptLevel::O2 => "-O2",
            WasmOptLevel::O3 => "-O3",
            WasmOptLevel::O4 => "-O4",
            WasmOptLevel::Oz => "-Oz",
        }
    }
}

/// Configuration for the build pipeline
///
/// Controls target platform, optimization levels, and tool invocation.
///
/// # Examples
///
/// Using defaults:
/// ```no_run
/// use wasm_slim::pipeline::{PipelineConfig, WasmTarget};
///
/// let config = PipelineConfig::default();
/// assert_eq!(config.target, WasmTarget::Wasm32UnknownUnknown);
/// assert_eq!(config.profile, "release");
/// assert!(config.run_wasm_opt);
/// ```
///
/// Custom configuration:
/// ```no_run
/// use wasm_slim::pipeline::{PipelineConfig, BindgenTarget};
///
/// let mut config = PipelineConfig::default();
/// config.bindgen_target = BindgenTarget::NodeJs;
/// config.run_wasm_snip = true; // Enable dead code removal
/// ```
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Target triple (default: wasm32-unknown-unknown)
    pub target: WasmTarget,
    /// Build profile (default: release)
    pub profile: String,
    /// Output directory (default: target/wasm32-unknown-unknown/release)
    pub target_dir: Option<PathBuf>,
    /// wasm-bindgen target (web, nodejs, bundler, deno)
    pub bindgen_target: BindgenTarget,
    /// Whether to run wasm-opt
    pub run_wasm_opt: bool,
    /// Whether to run wasm-snip
    pub run_wasm_snip: bool,
    /// wasm-opt optimization level
    pub opt_level: WasmOptLevel,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            target: WasmTarget::default(),
            profile: "release".to_string(),
            target_dir: None,
            bindgen_target: BindgenTarget::default(),
            run_wasm_opt: true,
            run_wasm_snip: false,
            opt_level: WasmOptLevel::Oz,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_target_default() {
        let target = WasmTarget::default();
        assert_eq!(target, WasmTarget::Wasm32UnknownUnknown);
        assert_eq!(target.as_str(), "wasm32-unknown-unknown");
    }

    #[test]
    fn test_wasm_target_as_str_all_variants() {
        assert_eq!(
            WasmTarget::Wasm32UnknownUnknown.as_str(),
            "wasm32-unknown-unknown"
        );
        assert_eq!(WasmTarget::Wasm32Wasi.as_str(), "wasm32-wasi");
        assert_eq!(
            WasmTarget::Wasm32UnknownEmscripten.as_str(),
            "wasm32-unknown-emscripten"
        );
    }

    #[test]
    fn test_wasm_target_equality() {
        let target1 = WasmTarget::Wasm32UnknownUnknown;
        let target2 = WasmTarget::Wasm32UnknownUnknown;
        let target3 = WasmTarget::Wasm32Wasi;

        assert_eq!(target1, target2);
        assert_ne!(target1, target3);
    }

    #[test]
    fn test_bindgen_target_default() {
        let target = BindgenTarget::default();
        assert_eq!(target, BindgenTarget::Web);
        assert_eq!(target.as_str(), "web");
    }

    #[test]
    fn test_bindgen_target_as_str_all_variants() {
        assert_eq!(BindgenTarget::Web.as_str(), "web");
        assert_eq!(BindgenTarget::NodeJs.as_str(), "nodejs");
        assert_eq!(BindgenTarget::Bundler.as_str(), "bundler");
        assert_eq!(BindgenTarget::Deno.as_str(), "deno");
        assert_eq!(BindgenTarget::NoModules.as_str(), "no-modules");
    }

    #[test]
    fn test_bindgen_target_equality() {
        let target1 = BindgenTarget::NodeJs;
        let target2 = BindgenTarget::NodeJs;
        let target3 = BindgenTarget::Web;

        assert_eq!(target1, target2);
        assert_ne!(target1, target3);
    }

    #[test]
    fn test_wasm_opt_level_as_arg() {
        assert_eq!(WasmOptLevel::O1.as_arg(), "-O1");
        assert_eq!(WasmOptLevel::O2.as_arg(), "-O2");
        assert_eq!(WasmOptLevel::O3.as_arg(), "-O3");
        assert_eq!(WasmOptLevel::O4.as_arg(), "-O4");
        assert_eq!(WasmOptLevel::Oz.as_arg(), "-Oz");
    }

    #[test]
    fn test_pipeline_config_default_values() {
        let config = PipelineConfig::default();

        assert_eq!(config.target, WasmTarget::Wasm32UnknownUnknown);
        assert_eq!(config.profile, "release");
        assert_eq!(config.target_dir, None);
        assert_eq!(config.bindgen_target, BindgenTarget::Web);
        assert!(config.run_wasm_opt);
        assert!(!config.run_wasm_snip);
        assert_eq!(config.opt_level, WasmOptLevel::Oz);
    }

    #[test]
    fn test_pipeline_config_builder_pattern() {
        let config = PipelineConfig {
            bindgen_target: BindgenTarget::NodeJs,
            run_wasm_snip: true,
            opt_level: WasmOptLevel::O3,
            ..Default::default()
        };

        assert_eq!(config.bindgen_target, BindgenTarget::NodeJs);
        assert!(config.run_wasm_snip);
        assert_eq!(config.opt_level, WasmOptLevel::O3);
    }

    #[test]
    fn test_pipeline_config_custom_profile() {
        let config = PipelineConfig {
            profile: "dev".to_string(),
            ..Default::default()
        };

        assert_eq!(config.profile, "dev");
    }

    #[test]
    fn test_pipeline_config_custom_target_dir() {
        let config = PipelineConfig {
            target_dir: Some(PathBuf::from("/custom/path")),
            ..Default::default()
        };

        assert_eq!(config.target_dir, Some(PathBuf::from("/custom/path")));
    }

    #[test]
    fn test_pipeline_config_clone() {
        let config1 = PipelineConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.target, config2.target);
        assert_eq!(config1.profile, config2.profile);
        assert_eq!(config1.bindgen_target, config2.bindgen_target);
    }
}
