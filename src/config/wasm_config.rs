//! WASM tooling configuration (wasm-opt, wasm-bindgen)

use serde::{Deserialize, Serialize};

/// wasm-opt configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmOptConfig {
    /// Optimization flags for wasm-opt
    pub flags: Vec<String>,
}

impl Default for WasmOptConfig {
    fn default() -> Self {
        // Production-tested flags from real-world projects
        Self {
            flags: vec![
                "-Oz".to_string(),
                "--enable-mutable-globals".to_string(),
                "--enable-bulk-memory".to_string(),
                "--enable-sign-ext".to_string(),
                "--enable-nontrapping-float-to-int".to_string(),
                "--strip-debug".to_string(),
                "--strip-dwarf".to_string(),
                "--strip-producers".to_string(),
            ],
        }
    }
}

/// wasm-bindgen configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmBindgenConfig {
    /// Enable debug info
    pub debug: bool,
    /// Remove producers section
    pub remove_producers_section: bool,
    /// Additional flags
    pub flags: Vec<String>,
}

impl Default for WasmBindgenConfig {
    fn default() -> Self {
        Self {
            debug: false,
            remove_producers_section: true,
            flags: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_opt_config_default_has_expected_flags() {
        let config = WasmOptConfig::default();

        assert!(!config.flags.is_empty());
        assert!(config.flags.contains(&"-Oz".to_string()));
        assert!(config.flags.contains(&"--strip-debug".to_string()));
    }
}
