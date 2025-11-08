//! Build pipeline orchestration module
//!
//! Implements the complete WASM optimization pipeline:
//! 1. cargo build --release --target wasm32-unknown-unknown
//! 2. wasm-bindgen with optimized flags
//! 3. wasm-opt -Oz for aggressive size optimization
//! 4. wasm-snip to remove panic infrastructure (optional)

pub mod build_orchestrator;
pub mod config;
pub mod error;
pub mod executor;
pub mod metrics;
pub mod result_formatter;
pub mod telemetry;
pub mod tool_runner;

pub use build_orchestrator::BuildOrchestrator;
pub use config::{BindgenTarget, PipelineConfig, WasmOptLevel, WasmTarget};
pub use error::PipelineError;
pub use executor::BuildPipeline;
pub use metrics::SizeMetrics;
pub use result_formatter::ResultFormatter;
pub use telemetry::{
    BuildEvent, MemoryCollector, MetricData, MetricsCollector, NoOpCollector, StdoutCollector,
};
pub use tool_runner::ToolRunner;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_size_metrics_calculates_reduction_correctly() {
        let metrics = SizeMetrics {
            before_bytes: 1024 * 1024, // 1 MB
            after_bytes: 512 * 1024,   // 512 KB
        };

        assert_eq!(metrics.reduction_bytes(), 512 * 1024);
        assert_eq!(metrics.reduction_percent(), 50.0);
    }

    #[test]
    fn test_format_bytes_converts_to_readable_units() {
        use crate::fmt::format_bytes;
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn test_wasm_opt_level_returns_correct_arguments() {
        assert_eq!(WasmOptLevel::Oz.as_arg(), "-Oz");
        assert_eq!(WasmOptLevel::O3.as_arg(), "-O3");
    }

    // P2-TEST-COV-012: Pipeline error propagation tests

    #[test]
    fn test_size_metrics_with_zero_before_size_handles_division_by_zero() {
        // Test edge case: zero before size
        let metrics = SizeMetrics {
            before_bytes: 0,
            after_bytes: 100,
        };

        assert_eq!(metrics.reduction_bytes(), -100);
        assert_eq!(metrics.reduction_percent(), 0.0); // Should handle division by zero
    }

    #[test]
    fn test_size_metrics_with_size_increase_returns_negative_reduction() {
        // Test when size increases (negative reduction)
        let metrics = SizeMetrics {
            before_bytes: 512 * 1024,
            after_bytes: 1024 * 1024,
        };

        assert_eq!(metrics.reduction_bytes(), -(512 * 1024));
        assert!(metrics.reduction_percent() < 0.0);
    }

    #[test]
    fn test_size_metrics_with_no_change_returns_zero_reduction() {
        // Test when size doesn't change
        let metrics = SizeMetrics {
            before_bytes: 1024,
            after_bytes: 1024,
        };

        assert_eq!(metrics.reduction_bytes(), 0);
        assert_eq!(metrics.reduction_percent(), 0.0);
    }

    #[test]
    fn test_format_bytes_at_unit_boundaries_formats_correctly() {
        use crate::fmt::format_bytes;
        // Test boundary cases
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1023), "1023.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 10), "10.00 MB");
    }

    #[test]
    fn test_pipeline_config_default_has_expected_values() {
        // Test default configuration
        let config = PipelineConfig::default();

        assert_eq!(config.target, WasmTarget::Wasm32UnknownUnknown);
        assert_eq!(config.profile, "release");
        assert_eq!(config.bindgen_target, BindgenTarget::Web);
        assert!(config.run_wasm_opt);
        assert!(!config.run_wasm_snip);
        assert!(config.target_dir.is_none());
    }

    #[test]
    fn test_wasm_opt_level_all_variants_have_correct_args() {
        // Ensure all optimization levels have correct args
        assert_eq!(WasmOptLevel::O1.as_arg(), "-O1");
        assert_eq!(WasmOptLevel::O2.as_arg(), "-O2");
        assert_eq!(WasmOptLevel::O3.as_arg(), "-O3");
        assert_eq!(WasmOptLevel::O4.as_arg(), "-O4");
        assert_eq!(WasmOptLevel::Oz.as_arg(), "-Oz");
    }

    #[test]
    fn test_pipeline_config_with_custom_values_sets_correctly() {
        // Test custom configuration
        let config = PipelineConfig {
            target: WasmTarget::Wasm32Wasi,
            profile: "dev".to_string(),
            target_dir: Some(PathBuf::from("/custom/target")),
            bindgen_target: BindgenTarget::NodeJs,
            run_wasm_opt: false,
            run_wasm_snip: true,
            opt_level: WasmOptLevel::O3,
        };

        assert_eq!(config.target, WasmTarget::Wasm32Wasi);
        assert_eq!(config.profile, "dev");
        assert_eq!(config.bindgen_target, BindgenTarget::NodeJs);
        assert!(!config.run_wasm_opt);
        assert!(config.run_wasm_snip);
        assert_eq!(config.target_dir.unwrap(), PathBuf::from("/custom/target"));
        assert_eq!(config.opt_level.as_arg(), "-O3");
    }

    #[test]
    fn test_size_metrics_with_large_values_above_1gb_calculates_correctly() {
        // Test with large file sizes (>1GB)
        let metrics = SizeMetrics {
            before_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
            after_bytes: 1024 * 1024 * 1024,      // 1 GB
        };

        assert_eq!(metrics.reduction_bytes(), 1024 * 1024 * 1024);
        assert_eq!(metrics.reduction_percent(), 50.0);
    }

    #[test]
    fn test_format_bytes_with_gigabyte_sizes_formats_as_megabytes() {
        use crate::fmt::format_bytes;
        // Test formatting of large sizes
        let gb = 1024 * 1024 * 1024;
        assert_eq!(format_bytes(gb), "1024.00 MB");
        assert_eq!(format_bytes(gb * 2), "2048.00 MB");
    }

    // P2-UNIT-002: Pipeline edge case tests

    #[test]
    fn test_pipeline_config_with_empty_strings_sets_correctly() {
        // Test configuration with edge case empty profile string
        let config = PipelineConfig {
            target: WasmTarget::Wasm32UnknownUnknown,
            profile: String::new(),
            target_dir: None,
            bindgen_target: BindgenTarget::Web,
            run_wasm_opt: false,
            run_wasm_snip: false,
            opt_level: WasmOptLevel::Oz,
        };

        assert_eq!(config.target, WasmTarget::Wasm32UnknownUnknown);
        assert_eq!(config.profile, "");
        assert_eq!(config.bindgen_target, BindgenTarget::Web);
    }

    #[test]
    fn test_size_metrics_with_large_values_near_i64_max() {
        // Test with large values that don't overflow i64 (i64::MAX is 9,223,372,036,854,775,807)
        let metrics = SizeMetrics {
            before_bytes: 5_000_000_000, // ~5GB before
            after_bytes: 2_000_000_000,  // ~2GB after
        };

        assert_eq!(metrics.reduction_bytes(), 3_000_000_000);
        assert!(metrics.reduction_percent() > 59.0 && metrics.reduction_percent() < 61.0);

        // Test edge case: after_bytes larger than before_bytes (regression)
        let regression_metrics = SizeMetrics {
            before_bytes: 1_000_000,
            after_bytes: 2_000_000,
        };

        assert_eq!(regression_metrics.reduction_bytes(), -1_000_000);
        assert!(regression_metrics.reduction_percent() < 0.0);
    }

    #[test]
    fn test_pipeline_config_with_special_characters_in_paths() {
        // Test configuration with special characters
        let config = PipelineConfig {
            target: WasmTarget::Wasm32UnknownUnknown,
            profile: "release".to_string(),
            target_dir: Some(PathBuf::from("/path/with spaces/and-dashes")),
            bindgen_target: BindgenTarget::Web,
            run_wasm_opt: true,
            run_wasm_snip: false,
            opt_level: WasmOptLevel::Oz,
        };

        assert!(config.target_dir.is_some());
        let path = config.target_dir.unwrap();
        assert!(path.to_string_lossy().contains(" "));
    }

    #[test]
    fn test_size_metrics_reduction_percent_precision() {
        // Test precision of percentage calculation
        let metrics = SizeMetrics {
            before_bytes: 1000,
            after_bytes: 333,
        };

        let percent = metrics.reduction_percent();
        assert!(percent > 66.0 && percent < 67.0);
        // Verify precision is reasonable
        assert!((percent - 66.7).abs() < 0.1);
    }

    #[test]
    fn test_format_bytes_with_fractional_kb_shows_precision() {
        use crate::fmt::format_bytes;
        // Test byte formatting with fractional KB
        let bytes_1_5_kb = 1536; // 1.5 KB
        let formatted = format_bytes(bytes_1_5_kb);

        assert!(formatted.contains("1.50") || formatted.contains("1.5"));
        assert!(formatted.contains("KB"));
    }
}
