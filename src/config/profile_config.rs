//! Cargo profile configuration for WASM optimization

use serde::{Deserialize, Serialize};

/// Cargo profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Optimization level: "s" (size), "z" (more size), "3" (speed)
    pub opt_level: String,
    /// Link-time optimization: "fat", "thin", true, false
    pub lto: String,
    /// Strip debug symbols
    pub strip: bool,
    /// Number of codegen units (1 = better optimization)
    pub codegen_units: u32,
    /// Panic strategy: "abort" (smaller) or "unwind"
    pub panic: String,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        // Production-tested defaults from Warp.dev
        Self {
            lto: "fat".to_string(),     // 15-30% reduction
            codegen_units: 1,           // Better optimization
            opt_level: "s".to_string(), // Size-optimized (balanced)
            strip: true,                // Remove debug symbols
            panic: "abort".to_string(), // Smaller panic handler
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_config_default_has_expected_values() {
        let config = ProfileConfig::default();

        assert_eq!(config.lto, "fat");
        assert_eq!(config.codegen_units, 1);
        assert_eq!(config.opt_level, "s");
        assert!(config.strip);
        assert_eq!(config.panic, "abort");
    }
}
