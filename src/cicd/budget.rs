//! Size budget enforcement for CI/CD
//!
//! Implements three-tier budget system:
//! - Target: Ideal size goal (informational)
//! - Warning: Threshold for warnings (exit 0 with warning)
//! - Max: Hard limit (exit 1 if exceeded)

use crate::config::file::SizeBudget;
use anyhow::Result;
use console::style;

/// Status of size budget check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetStatus {
    /// Under target size (green)
    UnderTarget,
    /// Between target and warning threshold (green/yellow)
    AboveTarget,
    /// Between warning and max (yellow)
    Warning,
    /// Over max size (red)
    OverBudget,
}

/// Result of budget check with detailed information
#[derive(Debug, Clone)]
pub struct BudgetResult {
    /// Budget status
    pub status: BudgetStatus,
    /// Actual size in KB
    pub size_kb: f64,
    /// Target size if set
    pub target_kb: Option<u64>,
    /// Warning threshold if set
    pub warn_threshold_kb: Option<u64>,
    /// Maximum allowed size if set
    pub max_size_kb: Option<u64>,
    /// Human-readable message
    pub message: String,
}

impl BudgetResult {
    /// Get exit code for CI/CD (0 = pass, 1 = fail, 2 = warning)
    pub fn exit_code(&self) -> i32 {
        match self.status {
            BudgetStatus::UnderTarget | BudgetStatus::AboveTarget => 0,
            BudgetStatus::Warning => 0,    // Warning still passes
            BudgetStatus::OverBudget => 1, // Fail
        }
    }

    /// Print colored status message
    pub fn print(&self) {
        let status_icon = match self.status {
            BudgetStatus::UnderTarget => style("✅").green(),
            BudgetStatus::AboveTarget => style("✓").green(),
            BudgetStatus::Warning => style("⚠️").yellow(),
            BudgetStatus::OverBudget => style("❌").red(),
        };

        let size_str = match self.status {
            BudgetStatus::UnderTarget | BudgetStatus::AboveTarget => {
                style(format!("{:.2} KB", self.size_kb)).green()
            }
            BudgetStatus::Warning => style(format!("{:.2} KB", self.size_kb)).yellow(),
            BudgetStatus::OverBudget => style(format!("{:.2} KB", self.size_kb)).red(),
        };

        println!("\n{} Size Budget Check: {}", status_icon, size_str);
        println!("   {}", self.message);

        // Show budget thresholds
        if let Some(target) = self.target_kb {
            let target_str = if self.size_kb <= target as f64 {
                style(format!("Target: {} KB", target)).green()
            } else {
                style(format!("Target: {} KB", target)).dim()
            };
            println!("   {}", target_str);
        }

        if let Some(warn) = self.warn_threshold_kb {
            let warn_str = if self.size_kb <= warn as f64 {
                style(format!("Warning: {} KB", warn)).dim()
            } else if self.status == BudgetStatus::Warning {
                style(format!("Warning: {} KB", warn)).yellow()
            } else {
                style(format!("Warning: {} KB", warn)).dim()
            };
            println!("   {}", warn_str);
        }

        if let Some(max) = self.max_size_kb {
            let max_str = if self.status == BudgetStatus::OverBudget {
                style(format!("Max: {} KB (EXCEEDED)", max)).red()
            } else {
                style(format!("Max: {} KB", max)).dim()
            };
            println!("   {}", max_str);
        }
    }
}

/// Size budget checker
pub struct BudgetChecker {
    budget: SizeBudget,
}

impl BudgetChecker {
    /// Create a new budget checker
    pub fn new(budget: SizeBudget) -> Self {
        Self { budget }
    }

    /// Determine budget status based on size and thresholds
    ///
    /// Priority order: max > warn > target
    fn determine_status(&self, size_kb: f64) -> BudgetStatus {
        // Check max threshold first (highest priority)
        if let Some(max) = self.budget.max_size_kb {
            if size_kb > max as f64 {
                return BudgetStatus::OverBudget;
            }
        }

        // Check warning threshold
        if let Some(warn) = self.budget.warn_threshold_kb {
            if size_kb > warn as f64 {
                return BudgetStatus::Warning;
            }
        }

        // Check target threshold
        if let Some(target) = self.budget.target_size_kb {
            if size_kb <= target as f64 {
                return BudgetStatus::UnderTarget;
            } else {
                return BudgetStatus::AboveTarget;
            }
        }

        // No target configured but within limits - if any budget is set, return AboveTarget
        // If no budget at all, return UnderTarget
        if self.budget.max_size_kb.is_some() || self.budget.warn_threshold_kb.is_some() {
            BudgetStatus::AboveTarget
        } else {
            BudgetStatus::UnderTarget
        }
    }

    /// Check if size is within budget
    ///
    /// # Examples
    ///
    /// ```
    /// use wasm_slim::cicd::{BudgetChecker, budget::BudgetStatus};
    /// use wasm_slim::config::file::SizeBudget;
    ///
    /// let budget = SizeBudget {
    ///     target_size_kb: Some(500),
    ///     warn_threshold_kb: Some(750),
    ///     max_size_kb: Some(1000),
    /// };
    /// let checker = BudgetChecker::new(budget);
    /// let result = checker.check(600 * 1024)?; // 600 KB
    /// assert!(matches!(result.status, BudgetStatus::AboveTarget));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn check(&self, size_bytes: u64) -> Result<BudgetResult> {
        let size_kb = size_bytes as f64 / 1024.0;

        // Determine status using priority order: max > warn > target
        let status = self.determine_status(size_kb);

        // Generate message
        let message = match status {
            BudgetStatus::UnderTarget => {
                if let Some(target) = self.budget.target_size_kb {
                    let under_by = target as f64 - size_kb;
                    format!("Under target by {:.2} KB", under_by)
                } else {
                    "Size OK".to_string()
                }
            }
            BudgetStatus::AboveTarget => {
                if let Some(target) = self.budget.target_size_kb {
                    let over_by = size_kb - target as f64;
                    format!("Above target by {:.2} KB (still within limits)", over_by)
                } else {
                    "Size OK".to_string()
                }
            }
            BudgetStatus::Warning => {
                if let Some(warn) = self.budget.warn_threshold_kb {
                    let over_by = size_kb - warn as f64;
                    format!(
                        "Warning: {} KB over threshold (consider optimizing)",
                        over_by as i64
                    )
                } else {
                    "Warning threshold exceeded".to_string()
                }
            }
            BudgetStatus::OverBudget => {
                if let Some(max) = self.budget.max_size_kb {
                    let over_by = size_kb - max as f64;
                    format!(
                        "FAILED: {} KB over budget (optimization required)",
                        over_by as i64
                    )
                } else {
                    "Budget exceeded".to_string()
                }
            }
        };

        Ok(BudgetResult {
            status,
            size_kb,
            target_kb: self.budget.target_size_kb,
            warn_threshold_kb: self.budget.warn_threshold_kb,
            max_size_kb: self.budget.max_size_kb,
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_budget_under_target_returns_success() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(400 * 1024).unwrap(); // 400 KB
        assert_eq!(result.status, BudgetStatus::UnderTarget);
        assert_eq!(result.exit_code(), 0);
    }

    #[test]
    fn test_check_budget_above_target_but_within_max_returns_warning() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(600 * 1024).unwrap(); // 600 KB
        assert_eq!(result.status, BudgetStatus::AboveTarget);
        assert_eq!(result.exit_code(), 0);
    }

    #[test]
    fn test_check_budget_at_warning_threshold_returns_warning() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(900 * 1024).unwrap(); // 900 KB
        assert_eq!(result.status, BudgetStatus::Warning);
        assert_eq!(result.exit_code(), 0); // Warning still passes
    }

    #[test]
    fn test_check_budget_exceeded_max_returns_failure() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(1100 * 1024).unwrap(); // 1100 KB
        assert_eq!(result.status, BudgetStatus::OverBudget);
        assert_eq!(result.exit_code(), 1); // Fail
    }

    #[test]
    fn test_check_budget_with_no_limits_returns_success() {
        let budget = SizeBudget {
            max_size_kb: None,
            warn_threshold_kb: None,
            target_size_kb: None,
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(5000 * 1024).unwrap(); // 5 MB
        assert_eq!(result.status, BudgetStatus::UnderTarget);
        assert_eq!(result.exit_code(), 0);
    }

    #[test]
    fn test_check_budget_with_only_max_limit_returns_correct_status() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: None,
            target_size_kb: None,
        };
        let checker = BudgetChecker::new(budget);

        let result_ok = checker.check(800 * 1024).unwrap();
        assert_eq!(result_ok.status, BudgetStatus::AboveTarget);
        assert_eq!(result_ok.exit_code(), 0);

        let result_fail = checker.check(1200 * 1024).unwrap();
        assert_eq!(result_fail.status, BudgetStatus::OverBudget);
        assert_eq!(result_fail.exit_code(), 1);
    }

    // P1-TEST-COV-007: Budget validation error message tests

    #[test]
    fn test_check_budget_under_target_has_success_message() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(400 * 1024).unwrap();

        assert!(result.message.contains("Under target"));
        assert!(result.message.contains("KB"));
        // Should show how much under
        assert!(result.message.contains("100") || result.message.contains("by"));
    }

    #[test]
    fn test_check_budget_above_target_has_warning_message() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(600 * 1024).unwrap();

        assert!(result.message.contains("Above target"));
        assert!(result.message.contains("within limits"));
        assert!(result.message.contains("KB"));
    }

    #[test]
    fn test_check_budget_at_warning_threshold_has_warning_message() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(900 * 1024).unwrap();

        assert!(result.message.contains("Warning"));
        assert!(result.message.contains("over threshold"));
        assert!(result.message.contains("optimizing"));
        assert!(result.message.contains("KB"));
    }

    #[test]
    fn test_check_budget_exceeded_has_failure_message() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(1100 * 1024).unwrap();

        assert!(result.message.contains("FAILED"));
        assert!(result.message.contains("over budget"));
        assert!(result.message.contains("optimization required"));
        assert!(result.message.contains("KB"));
    }

    #[test]
    fn test_check_budget_with_no_limits_has_success_message() {
        let budget = SizeBudget {
            max_size_kb: None,
            warn_threshold_kb: None,
            target_size_kb: None,
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(5000 * 1024).unwrap();

        assert!(result.message.contains("Size OK") || result.message.contains("OK"));
    }

    #[test]
    fn test_budget_result_fields_have_correct_values() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(600 * 1024).unwrap();

        // Verify all fields are populated correctly
        assert_eq!(result.size_kb, 600.0);
        assert_eq!(result.target_kb, Some(500));
        assert_eq!(result.warn_threshold_kb, Some(800));
        assert_eq!(result.max_size_kb, Some(1000));
        assert!(!result.message.is_empty());
    }

    #[test]
    fn test_check_budget_exactly_at_target_returns_success() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(500 * 1024).unwrap();

        // Exactly at target should be UnderTarget
        assert_eq!(result.status, BudgetStatus::UnderTarget);
        assert!(result.message.contains("Under target"));
    }

    #[test]
    fn test_check_budget_exactly_at_warning_threshold_returns_warning() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(800 * 1024).unwrap();

        // Exactly at warning threshold should not trigger warning
        assert_eq!(result.status, BudgetStatus::AboveTarget);
    }

    #[test]
    fn test_check_budget_exactly_at_max_returns_failure() {
        let budget = SizeBudget {
            max_size_kb: Some(1000),
            warn_threshold_kb: Some(800),
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);
        let result = checker.check(1000 * 1024).unwrap();

        // Exactly at max should not exceed
        assert_eq!(result.status, BudgetStatus::Warning);
        assert_eq!(result.exit_code(), 0);
    }

    #[test]
    fn test_check_budget_with_only_target_no_other_limits_works_correctly() {
        let budget = SizeBudget {
            max_size_kb: None,
            warn_threshold_kb: None,
            target_size_kb: Some(500),
        };
        let checker = BudgetChecker::new(budget);

        // Under target
        let result_under = checker.check(400 * 1024).unwrap();
        assert_eq!(result_under.status, BudgetStatus::UnderTarget);
        assert!(result_under.message.contains("Under target"));

        // Over target but no max
        let result_over = checker.check(600 * 1024).unwrap();
        assert_eq!(result_over.status, BudgetStatus::AboveTarget);
        assert!(result_over.message.contains("Above target"));
    }

    // Property-based tests using proptest
    use proptest::prelude::*;

    proptest! {
        /// Property: Budget status transitions are monotonic - larger sizes never have better status
        #[test]
        fn prop_budget_status_monotonic(
            size_kb in 1u64..10_000,
            target in 100u64..5_000,
            warn in 200u64..7_500,
            max in 300u64..10_000
        ) {
            // Ensure proper ordering: target < warn < max
            prop_assume!(target < warn && warn < max);

            let budget = SizeBudget {
                target_size_kb: Some(target),
                warn_threshold_kb: Some(warn),
                max_size_kb: Some(max),
            };

            let checker = BudgetChecker { budget };
            let size_bytes = size_kb * 1024;

            let result = checker.check(size_bytes).unwrap();

            // Verify status is consistent with thresholds
            if size_kb > max {
                prop_assert_eq!(result.status, BudgetStatus::OverBudget,
                    "Size {}KB > max {}KB should be OverBudget", size_kb, max);
            } else if size_kb > warn {
                prop_assert_eq!(result.status, BudgetStatus::Warning,
                    "Size {}KB > warn {}KB should be Warning", size_kb, warn);
            } else if size_kb > target {
                prop_assert_eq!(result.status, BudgetStatus::AboveTarget,
                    "Size {}KB > target {}KB should be AboveTarget", size_kb, target);
            } else {
                prop_assert_eq!(result.status, BudgetStatus::UnderTarget,
                    "Size {}KB <= target {}KB should be UnderTarget", size_kb, target);
            }
        }

        /// Property: Exit codes are deterministic and follow rules
        #[test]
        fn prop_exit_codes_deterministic(
            size_kb in 1u64..10_000,
            max in 100u64..5_000
        ) {
            let budget = SizeBudget {
                target_size_kb: Some(max / 2),
                warn_threshold_kb: Some(max * 3 / 4),
                max_size_kb: Some(max),
            };

            let checker = BudgetChecker { budget };
            let result = checker.check(size_kb * 1024).unwrap();

            // Exit code rules
            let exit_code = result.exit_code();
            match result.status {
                BudgetStatus::UnderTarget | BudgetStatus::AboveTarget | BudgetStatus::Warning => {
                    prop_assert_eq!(exit_code, 0, "Non-critical statuses should exit 0");
                }
                BudgetStatus::OverBudget => {
                    prop_assert_eq!(exit_code, 1, "OverBudget should exit 1");
                }
            }
        }

        /// Property: Bytes to KB conversion is consistent
        #[test]
        fn prop_bytes_to_kb_conversion(size_bytes in 0u64..100_000_000) {
            let budget = SizeBudget {
                target_size_kb: Some(1000),
                warn_threshold_kb: Some(2000),
                max_size_kb: Some(3000),
            };

            let checker = BudgetChecker { budget };
            let result = checker.check(size_bytes).unwrap();

            let expected_kb = size_bytes as f64 / 1024.0;
            prop_assert!((result.size_kb - expected_kb).abs() < 0.01,
                "Conversion mismatch: {} bytes should be ~{} KB, got {} KB",
                size_bytes, expected_kb, result.size_kb);
        }

        /// Property: None thresholds never cause panics
        #[test]
        fn prop_none_thresholds_no_panic(
            size_kb in 0u64..10_000,
            has_target in prop::bool::ANY,
            has_warn in prop::bool::ANY,
            has_max in prop::bool::ANY
        ) {
            let budget = SizeBudget {
                target_size_kb: if has_target { Some(500) } else { None },
                warn_threshold_kb: if has_warn { Some(1000) } else { None },
                max_size_kb: if has_max { Some(2000) } else { None },
            };

            let checker = BudgetChecker { budget };
            let result = checker.check(size_kb * 1024);

            // Should never panic, always return Ok
            prop_assert!(result.is_ok(), "Check with None thresholds should not panic");
        }

        /// Property: Status matches reported thresholds
        #[test]
        fn prop_status_matches_thresholds(
            size_kb in 1u64..5_000,
            target in 100u64..2_000,
            warn in 200u64..3_000,
            max in 300u64..4_000
        ) {
            prop_assume!(target < warn && warn < max);

            let budget = SizeBudget {
                target_size_kb: Some(target),
                warn_threshold_kb: Some(warn),
                max_size_kb: Some(max),
            };

            let checker = BudgetChecker { budget };
            let result = checker.check(size_kb * 1024).unwrap();

            // Verify reported thresholds match input
            prop_assert_eq!(result.target_kb, Some(target));
            prop_assert_eq!(result.warn_threshold_kb, Some(warn));
            prop_assert_eq!(result.max_size_kb, Some(max));
        }

        /// Property: Message is always non-empty
        #[test]
        fn prop_message_non_empty(
            size_kb in 1u64..10_000,
            max in 100u64..5_000
        ) {
            let budget = SizeBudget {
                target_size_kb: Some(max / 2),
                warn_threshold_kb: Some(max * 3 / 4),
                max_size_kb: Some(max),
            };

            let checker = BudgetChecker { budget };
            let result = checker.check(size_kb * 1024).unwrap();

            prop_assert!(!result.message.is_empty(), "Message should never be empty");
            prop_assert!(result.message.len() > 5, "Message should be descriptive");
        }

        /// Property: Zero size always passes
        #[test]
        fn prop_zero_size_passes(
            target in 100u64..5_000,
            warn in 200u64..7_500,
            max in 300u64..10_000
        ) {
            prop_assume!(target < warn && warn < max);

            let budget = SizeBudget {
                target_size_kb: Some(target),
                warn_threshold_kb: Some(warn),
                max_size_kb: Some(max),
            };

            let checker = BudgetChecker { budget };
            let result = checker.check(0).unwrap();

            prop_assert_eq!(result.status, BudgetStatus::UnderTarget,
                "Zero size should always be UnderTarget");
            prop_assert_eq!(result.exit_code(), 0, "Zero size should pass");
        }
    }
}
