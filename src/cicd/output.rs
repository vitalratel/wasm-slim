//! JSON output formatting for CI/CD integration

use crate::cicd::budget::{BudgetResult, BudgetStatus};
use crate::cicd::history::RegressionResult;
use serde::{Deserialize, Serialize};

/// JSON output structure for CI/CD tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonOutput {
    /// Build success status
    pub success: bool,
    /// Size information
    pub size: SizeInfo,
    /// Budget check result (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<BudgetInfo>,
    /// Regression check result (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regression: Option<RegressionInfo>,
}

/// Size information in multiple units
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeInfo {
    /// Size in bytes
    pub bytes: u64,
    /// Size in KB
    pub kb: f64,
    /// Size in MB
    pub mb: f64,
    /// Formatted string (e.g., "500.00 KB")
    pub formatted: String,
}

impl SizeInfo {
    /// Create size info from bytes
    pub fn new(bytes: u64) -> Self {
        let kb = bytes as f64 / 1024.0;
        let mb = kb / 1024.0;
        let formatted = if mb >= 1.0 {
            format!("{:.2} MB", mb)
        } else {
            format!("{:.2} KB", kb)
        };

        Self {
            bytes,
            kb,
            mb,
            formatted,
        }
    }
}

/// Budget check results for JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetInfo {
    /// Budget status: "under_target", "above_target", "warning", "over_budget"
    pub status: String,
    /// Whether budget check passed (true if not over_budget)
    pub passed: bool,
    /// Target size in KB (if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kb: Option<u64>,
    /// Warning threshold in KB (if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warn_threshold_kb: Option<u64>,
    /// Max size in KB (if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size_kb: Option<u64>,
    /// Amount over/under budget in KB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_kb: Option<f64>,
    /// Human-readable message
    pub message: String,
}

impl BudgetInfo {
    /// Convert from BudgetResult
    pub fn from_result(result: &BudgetResult) -> Self {
        let status_str = match result.status {
            BudgetStatus::UnderTarget => "under_target",
            BudgetStatus::AboveTarget => "above_target",
            BudgetStatus::Warning => "warning",
            BudgetStatus::OverBudget => "over_budget",
        };

        let passed = result.status != BudgetStatus::OverBudget;

        // Calculate delta
        let delta_kb = if let Some(max) = result.max_size_kb {
            Some(result.size_kb - max as f64)
        } else if let Some(warn) = result.warn_threshold_kb {
            Some(result.size_kb - warn as f64)
        } else {
            result
                .target_kb
                .map(|target| result.size_kb - target as f64)
        };

        Self {
            status: status_str.to_string(),
            passed,
            target_kb: result.target_kb,
            warn_threshold_kb: result.warn_threshold_kb,
            max_size_kb: result.max_size_kb,
            delta_kb,
            message: result.message.clone(),
        }
    }
}

/// Regression detection results for JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionInfo {
    /// Whether a regression was detected (>5% increase)
    pub is_regression: bool,
    /// Previous build size in bytes
    pub previous_bytes: u64,
    /// Previous build size in KB
    pub previous_kb: f64,
    /// Size difference in bytes (negative = reduction)
    pub diff_bytes: i64,
    /// Size difference in KB (negative = reduction)
    pub diff_kb: f64,
    /// Percent change (negative = reduction)
    pub percent_change: f64,
}

impl RegressionInfo {
    /// Convert from RegressionResult
    pub fn from_result(result: &RegressionResult) -> Self {
        Self {
            is_regression: result.is_regression,
            previous_bytes: result.previous_size,
            previous_kb: result.previous_size as f64 / 1024.0,
            diff_bytes: result.size_diff,
            diff_kb: result.size_diff as f64 / 1024.0,
            percent_change: result.percent_change,
        }
    }
}

impl JsonOutput {
    /// Create a new JSON output
    pub fn new(size_bytes: u64) -> Self {
        Self {
            success: true,
            size: SizeInfo::new(size_bytes),
            budget: None,
            regression: None,
        }
    }

    /// Add budget check result
    pub fn with_budget(mut self, result: &BudgetResult) -> Self {
        self.success = result.status != BudgetStatus::OverBudget;
        self.budget = Some(BudgetInfo::from_result(result));
        self
    }

    /// Add regression check result
    pub fn with_regression(mut self, result: &RegressionResult) -> Self {
        self.regression = Some(RegressionInfo::from_result(result));
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize JSON output: {}", e))
    }

    /// Print JSON to stdout
    pub fn print(&self) {
        match self.to_json() {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error formatting JSON: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cicd::budget::BudgetChecker;
    use crate::config::file::SizeBudget;

    #[test]
    fn test_size_info_formats_sizes_correctly() {
        let info = SizeInfo::new(500 * 1024);
        assert_eq!(info.bytes, 500 * 1024);
        assert_eq!(info.kb, 500.0);
        assert!(info.formatted.contains("500"));
        assert!(info.formatted.contains("KB"));
    }

    #[test]
    fn test_json_output_basic_includes_size_info() {
        let output = JsonOutput::new(500 * 1024);
        assert!(output.success);
        assert_eq!(output.size.kb, 500.0);
        assert!(output.budget.is_none());
        assert!(output.regression.is_none());
    }

    #[test]
    fn test_json_output_with_budget_includes_budget_status() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let budget_result = checker.check(600 * 1024).unwrap();

        let output = JsonOutput::new(600 * 1024).with_budget(&budget_result);

        assert!(output.success);
        assert!(output.budget.is_some());
        let budget_info = output.budget.unwrap();
        assert_eq!(budget_info.status, "above_target");
        assert!(budget_info.passed);
    }

    #[test]
    fn test_json_output_with_budget_failed_shows_failure() {
        let budget = SizeBudget {
            max_size_kb: Some(500),
            warn_threshold_kb: None,
            target_size_kb: None,
        };
        let checker = BudgetChecker::new(budget);
        let budget_result = checker.check(600 * 1024).unwrap();

        let output = JsonOutput::new(600 * 1024).with_budget(&budget_result);

        assert!(!output.success);
        assert!(output.budget.is_some());
        let budget_info = output.budget.unwrap();
        assert_eq!(budget_info.status, "over_budget");
        assert!(!budget_info.passed);
    }

    #[test]
    fn test_json_output_serialization_produces_valid_json() {
        let output = JsonOutput::new(500 * 1024);
        let json = output.to_json().unwrap();
        assert!(json.contains("\"success\":"));
        assert!(json.contains("\"size\":"));
        assert!(json.contains("\"kb\": 500"));
    }
}
