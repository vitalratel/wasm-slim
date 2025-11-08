//! CI/CD integration module (Phase 8)
//!
//! Provides:
//! - Size budget enforcement with configurable thresholds
//! - Build history tracking for regression detection
//! - JSON output for CI/CD tool integration
//! - Exit codes for automated workflows

pub mod budget;
pub mod display;
pub mod history;
pub mod output;

pub use budget::BudgetChecker;
pub use history::{BuildHistory, BuildRecord};
pub use output::JsonOutput;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cicd_module_exports_are_accessible() {
        // Ensure all exports compile and are accessible
        let _: Option<BudgetChecker> = None;
        let _: Option<BuildHistory> = None;
        let _: Option<BuildRecord> = None;
        let _: Option<JsonOutput> = None;
    }
}
