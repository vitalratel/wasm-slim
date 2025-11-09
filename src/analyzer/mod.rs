//! Dependency and binary analysis module
//!
//! Provides tools for analyzing WASM bundles and their dependencies:
//! - Dependency tree parsing and analysis
//! - Feature flag optimization detection
//! - Heavy dependency identification
//! - Size estimation and reporting

pub mod allocator;
pub mod applicator;
pub mod asset_display;
pub mod asset_metrics;
pub mod asset_report;
pub mod asset_types;
pub mod asset_visitor;
pub mod assets;
pub mod bloat;
pub mod bloat_report;
pub mod deps;
pub mod deps_report;
pub mod deps_types;
pub mod feature_report;
pub mod features;
pub mod heavy_deps;
pub mod panic_advisor;
pub mod panic_report;
pub mod panics;
pub mod report_utils;
pub mod twiggy;
pub mod twiggy_report;

// Public exports for common analyzer types
pub use applicator::SuggestionApplicator;
pub use asset_report::{print_asset_report, print_json_output, show_externalization_guide};
pub use assets::AssetDetector;
pub use bloat::BloatAnalyzer;
pub use bloat_report::{
    format_console_report as format_bloat_console, format_json_report as format_bloat_json,
};
pub use deps::DependencyAnalyzer;
pub use feature_report::{
    format_console_report as format_feature_console, format_json_report as format_feature_json,
};
pub use features::FeatureAnalyzer;
pub use panic_report::{print_json_report as print_panic_json, print_panic_report};
pub use panics::PanicDetector;
pub use twiggy::{AnalysisMode, MonomorphizationGroup, TwiggyAnalyzer};
pub use twiggy_report::{print_analysis_report, print_comparison_report};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_module_exports_are_accessible() {
        // Ensure all key exports compile and are accessible
        let _: Option<DependencyAnalyzer> = None;
        let _: Option<SuggestionApplicator> = None;
        let _: Option<AssetDetector> = None;
        let _: Option<TwiggyAnalyzer> = None;
        let _: Option<BloatAnalyzer> = None;
        let _: Option<FeatureAnalyzer> = None;
        let _: Option<PanicDetector> = None;
    }
}
