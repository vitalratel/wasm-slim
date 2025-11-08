//! Configuration validation system
//!
//! Provides pluggable validation for configuration files, enabling custom
//! validation rules, conflict detection, and auto-fix suggestions.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Validation severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity {
    /// Informational message
    Info,
    /// Warning - should be addressed but not blocking
    Warning,
    /// Error - must be fixed
    Error,
}

impl ValidationSeverity {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationSeverity::Info => "INFO",
            ValidationSeverity::Warning => "WARNING",
            ValidationSeverity::Error => "ERROR",
        }
    }
}

/// A validation issue found in configuration
///
/// # Examples
///
/// ```
/// use wasm_slim::config::validator::{ValidationIssue, ValidationSeverity};
///
/// // Create an error issue
/// let issue = ValidationIssue::error("opt_level", "Invalid optimization level");
/// assert_eq!(issue.severity, ValidationSeverity::Error);
/// assert_eq!(issue.field, "opt_level");
///
/// // Create a warning with a suggestion
/// let warning = ValidationIssue::warning("codegen_units", "Value may be too high")
///     .with_suggestion("Try setting codegen_units to 1 for better optimization");
/// assert!(warning.suggestion.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Severity level
    pub severity: ValidationSeverity,
    /// Field or section that has the issue
    pub field: String,
    /// Description of the issue
    pub message: String,
    /// Suggested fix (if available)
    pub suggestion: Option<String>,
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(
        severity: ValidationSeverity,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            field: field.into(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Create an error issue
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ValidationSeverity::Error, field, message)
    }

    /// Create a warning issue
    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ValidationSeverity::Warning, field, message)
    }

    /// Create an info issue
    pub fn info(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ValidationSeverity::Info, field, message)
    }
}

/// Result of configuration validation
///
/// # Examples
///
/// ```
/// use wasm_slim::config::validator::{ValidationResult, ValidationIssue};
///
/// // Successful validation
/// let result = ValidationResult::success();
/// assert!(result.valid);
/// assert!(result.issues.is_empty());
///
/// // Failed validation with issues
/// let issues = vec![
///     ValidationIssue::error("profile", "Invalid profile name"),
///     ValidationIssue::warning("lto", "LTO is disabled"),
/// ];
/// let result = ValidationResult::failure(issues);
/// assert!(!result.valid);
/// assert_eq!(result.issues.len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed (no errors)
    pub valid: bool,
    /// Issues found during validation
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            valid: true,
            issues: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn failure(issues: Vec<ValidationIssue>) -> Self {
        let has_errors = issues
            .iter()
            .any(|i| i.severity == ValidationSeverity::Error);
        Self {
            valid: !has_errors,
            issues,
        }
    }

    /// Add an issue
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        if issue.severity == ValidationSeverity::Error {
            self.valid = false;
        }
        self.issues.push(issue);
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|i| i.severity == ValidationSeverity::Error)
    }

    /// Get only errors
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect()
    }

    /// Get only warnings
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Warning)
            .collect()
    }
}

/// Trait for pluggable configuration validators
pub trait ConfigValidator: Send + Sync {
    /// Validator name
    fn name(&self) -> &str;

    /// Validate configuration data
    ///
    /// The config is provided as a key-value map for flexibility.
    /// Validators can check specific fields they care about.
    fn validate(&self, config: &HashMap<String, String>) -> ValidationResult;

    /// Attempt to automatically fix issues
    ///
    /// Returns a map of field -> fixed value suggestions.
    fn auto_fix(&self, config: &HashMap<String, String>) -> HashMap<String, String> {
        // Default: no auto-fixes
        let _ = config;
        HashMap::new()
    }

    /// Get validator priority (lower runs first)
    fn priority(&self) -> u32 {
        100
    }
}

/// Registry for managing configuration validators
#[derive(Default)]
pub struct ValidatorRegistry {
    validators: Mutex<Vec<Arc<dyn ConfigValidator>>>,
}

impl ValidatorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a validator
    pub fn register(&self, validator: Arc<dyn ConfigValidator>) {
        let mut validators = self.validators.lock();
        validators.push(validator);
        // Sort by priority
        validators.sort_by_key(|v| v.priority());
    }

    /// Run all validators
    pub fn validate_all(&self, config: &HashMap<String, String>) -> ValidationResult {
        let validators = self.validators.lock();

        let mut result = ValidationResult::success();

        for validator in validators.iter() {
            let validator_result = validator.validate(config);
            for issue in validator_result.issues {
                result.add_issue(issue);
            }
        }

        result
    }

    /// Run all validators and collect auto-fix suggestions
    pub fn auto_fix_all(&self, config: &HashMap<String, String>) -> HashMap<String, String> {
        let validators = self.validators.lock();

        let mut fixes = HashMap::new();

        for validator in validators.iter() {
            let validator_fixes = validator.auto_fix(config);
            fixes.extend(validator_fixes);
        }

        fixes
    }

    /// Clear all validators
    pub fn clear(&self) {
        self.validators.lock().clear();
    }

    /// Get validator count
    pub fn count(&self) -> usize {
        self.validators.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestValidator;

    impl ConfigValidator for TestValidator {
        fn name(&self) -> &str {
            "test"
        }

        fn validate(&self, config: &HashMap<String, String>) -> ValidationResult {
            let mut result = ValidationResult::success();
            if config.get("invalid_field").is_some() {
                result.add_issue(ValidationIssue::error("invalid_field", "Test error"));
            }
            result
        }
    }

    #[test]
    fn test_validation_result_detects_errors() {
        let mut result = ValidationResult::success();
        assert!(result.valid);

        result.add_issue(ValidationIssue::warning("field", "warning"));
        assert!(result.valid);

        result.add_issue(ValidationIssue::error("field", "error"));
        assert!(!result.valid);
    }

    #[test]
    fn test_validator_registry_runs_all_validators() {
        let registry = ValidatorRegistry::new();
        registry.register(Arc::new(TestValidator));

        let mut config = HashMap::new();
        config.insert("invalid_field".to_string(), "value".to_string());

        let result = registry.validate_all(&config);
        assert!(!result.valid);
        assert_eq!(result.errors().len(), 1);
    }

    #[test]
    fn test_validation_issue_builder() {
        let issue = ValidationIssue::error("field", "message").with_suggestion("fix");
        assert_eq!(issue.severity, ValidationSeverity::Error);
        assert_eq!(issue.field, "field");
        assert_eq!(issue.message, "message");
        assert_eq!(issue.suggestion, Some("fix".to_string()));
    }

    // Cross-field validation tests
    struct CrossFieldValidator;

    impl ConfigValidator for CrossFieldValidator {
        fn name(&self) -> &str {
            "cross-field"
        }

        fn validate(&self, config: &HashMap<String, String>) -> ValidationResult {
            let mut result = ValidationResult::success();

            // Check opt_level vs size_budget compatibility
            if let (Some(opt_level), Some(size_budget)) =
                (config.get("profile.opt_level"), config.get("size_budget"))
            {
                if opt_level == "0" && size_budget.parse::<u64>().unwrap_or(0) < 100_000 {
                    result.add_issue(
                        ValidationIssue::error(
                            "profile.opt_level + size_budget",
                            "opt-level=0 with small size budget is incompatible",
                        )
                        .with_suggestion("Use opt-level='z' or 's' for size optimization"),
                    );
                }
            }

            // Check wasm_opt flags vs target
            if let (Some(wasm_opt_flags), Some(target)) =
                (config.get("wasm_opt.flags"), config.get("target"))
            {
                if target.contains("wasm32")
                    && wasm_opt_flags.contains("--enable-simd")
                    && !target.contains("simd")
                {
                    result.add_issue(
                        ValidationIssue::warning(
                            "wasm_opt.flags + target",
                            "SIMD flags enabled but target doesn't explicitly support SIMD",
                        )
                        .with_suggestion(
                            "Verify target architecture supports SIMD or remove --enable-simd flag",
                        ),
                    );
                }
            }

            result
        }
    }

    #[test]
    fn test_validator_detects_incompatible_opt_level_and_budget() {
        let validator = CrossFieldValidator;
        let mut config = HashMap::new();
        config.insert("profile.opt_level".to_string(), "0".to_string());
        config.insert("size_budget".to_string(), "50000".to_string());

        let result = validator.validate(&config);

        assert!(
            !result.valid,
            "Should detect incompatible opt_level and size_budget"
        );
        assert_eq!(result.errors().len(), 1);
        assert!(result.errors()[0].message.contains("incompatible"));
        assert!(result.errors()[0].suggestion.is_some());
    }

    #[test]
    fn test_validator_detects_invalid_wasm_opt_for_target() {
        let validator = CrossFieldValidator;
        let mut config = HashMap::new();
        config.insert("wasm_opt.flags".to_string(), "--enable-simd".to_string());
        config.insert("target".to_string(), "wasm32-unknown-unknown".to_string());

        let result = validator.validate(&config);

        // This should be a warning, not an error
        assert!(result.valid, "Should be valid with warnings");
        assert_eq!(result.warnings().len(), 1);
        assert!(result.warnings()[0].message.contains("SIMD"));
    }

    #[test]
    fn test_validator_with_multiple_constraint_violations() {
        let validator = CrossFieldValidator;
        let mut config = HashMap::new();

        // Add multiple violations
        config.insert("profile.opt_level".to_string(), "0".to_string());
        config.insert("size_budget".to_string(), "10000".to_string());
        config.insert("wasm_opt.flags".to_string(), "--enable-simd".to_string());
        config.insert("target".to_string(), "wasm32-unknown-unknown".to_string());

        let result = validator.validate(&config);

        assert!(!result.valid, "Should detect multiple violations");
        assert_eq!(result.errors().len(), 1, "Should have 1 error");
        assert_eq!(result.warnings().len(), 1, "Should have 1 warning");
    }

    #[test]
    fn test_validator_allows_compatible_configurations() {
        let validator = CrossFieldValidator;
        let mut config = HashMap::new();

        // Compatible configuration
        config.insert("profile.opt_level".to_string(), "z".to_string());
        config.insert("size_budget".to_string(), "50000".to_string());

        let result = validator.validate(&config);

        assert!(result.valid);
        assert_eq!(result.errors().len(), 0);
        assert_eq!(result.warnings().len(), 0);
    }

    #[test]
    fn test_validator_handles_missing_fields_gracefully() {
        let validator = CrossFieldValidator;
        let config = HashMap::new();

        let result = validator.validate(&config);

        // Should pass validation when fields are missing (they're optional)
        assert!(result.valid);
        assert_eq!(result.errors().len(), 0);
    }

    #[test]
    fn test_validator_with_partial_cross_field_data() {
        let validator = CrossFieldValidator;
        let mut config = HashMap::new();

        // Only one field from a cross-field constraint
        config.insert("profile.opt_level".to_string(), "0".to_string());

        let result = validator.validate(&config);

        // Should not error when only partial data is present
        assert!(result.valid);
    }
}
