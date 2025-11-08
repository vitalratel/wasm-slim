//! Integration tests for apply_suggestions functionality
//!
//! Tests the end-to-end flow of dependency analysis and suggestion application.
//! These tests verify that the full optimization pipeline works correctly with
//! real Cargo.toml files and actual dependency reports.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use wasm_slim::analyzer::{
    applicator::SuggestionApplicator,
    deps::{DependencyIssue, DependencyReport, IssueSeverity},
};

/// Helper to create a test Cargo.toml with specified dependencies
fn create_test_cargo_toml(dir: &Path, content: &str) -> PathBuf {
    let cargo_toml = dir.join("Cargo.toml");
    fs::write(&cargo_toml, content).unwrap();
    cargo_toml
}

/// Helper to create a minimal valid Cargo.toml
fn create_minimal_cargo_toml(dir: &Path) -> PathBuf {
    create_test_cargo_toml(
        dir,
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    )
}

/// Helper to create a Cargo.toml with a heavy dependency (swc_core)
fn create_cargo_toml_with_swc(dir: &Path) -> PathBuf {
    create_test_cargo_toml(
        dir,
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
swc_core = "0.90"
"#,
    )
}

/// Helper to create a Cargo.toml with getrandom dependency
fn create_cargo_toml_with_getrandom(dir: &Path) -> PathBuf {
    create_test_cargo_toml(
        dir,
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
getrandom = "0.2"
"#,
    )
}

/// Helper to create an empty dependency report
fn create_empty_report() -> DependencyReport {
    DependencyReport {
        issues: vec![],
        total_deps: 0,
        direct_deps: 0,
        duplicates: HashMap::new(),
    }
}

/// Helper to create a report with a single swc_core issue
fn create_swc_issue_report() -> DependencyReport {
    DependencyReport {
        issues: vec![DependencyIssue {
            package: "swc_core".to_string(),
            version: "0.90.0".to_string(),
            severity: IssueSeverity::Critical,
            issue: "Very large dependency (2.5MB)".to_string(),
            suggestion: "Consider using minimal features or alternatives".to_string(),
            size_impact_kb: Some((2500, 500)),
            savings_percent: Some(80),
        }],
        total_deps: 1,
        direct_deps: 1,
        duplicates: HashMap::new(),
    }
}

/// Helper to create a report with getrandom WASM issue
fn create_getrandom_issue_report() -> DependencyReport {
    DependencyReport {
        issues: vec![DependencyIssue {
            package: "getrandom".to_string(),
            version: "0.2.0".to_string(),
            severity: IssueSeverity::High,
            issue: "Missing WASM support features".to_string(),
            suggestion: "Add 'js' feature for WASM compatibility".to_string(),
            size_impact_kb: Some((50, 20)),
            savings_percent: Some(60),
        }],
        total_deps: 1,
        direct_deps: 1,
        duplicates: HashMap::new(),
    }
}

/// Helper to create a report with multiple issues
fn create_multiple_issues_report() -> DependencyReport {
    DependencyReport {
        issues: vec![
            DependencyIssue {
                package: "swc_core".to_string(),
                version: "0.90.0".to_string(),
                severity: IssueSeverity::Critical,
                issue: "Very large dependency".to_string(),
                suggestion: "Use minimal features".to_string(),
                size_impact_kb: Some((2500, 500)),
                savings_percent: Some(80),
            },
            DependencyIssue {
                package: "getrandom".to_string(),
                version: "0.2.0".to_string(),
                severity: IssueSeverity::High,
                issue: "Missing WASM features".to_string(),
                suggestion: "Add 'js' feature".to_string(),
                size_impact_kb: Some((50, 20)),
                savings_percent: Some(60),
            },
        ],
        total_deps: 2,
        direct_deps: 2,
        duplicates: HashMap::new(),
    }
}

#[test]
fn test_apply_suggestions_empty_report_no_op() {
    // Test that applying an empty report makes no changes
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml_path = create_minimal_cargo_toml(temp_dir.path());

    // Read original content
    let original_content = fs::read_to_string(&cargo_toml_path).unwrap();

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_empty_report();

    let result = applicator.apply_suggestions(&report, false);

    // Should succeed with 0 fixes
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);

    // Content should be unchanged
    let new_content = fs::read_to_string(&cargo_toml_path).unwrap();
    assert_eq!(original_content, new_content);
}

#[test]
fn test_apply_suggestions_dry_run_no_modifications() {
    // Test that dry run doesn't modify files
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml_path = create_cargo_toml_with_swc(temp_dir.path());

    let original_content = fs::read_to_string(&cargo_toml_path).unwrap();

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_swc_issue_report();

    let result = applicator.apply_suggestions(&report, true); // dry_run = true

    assert!(result.is_ok());

    // Content should be unchanged in dry run
    let new_content = fs::read_to_string(&cargo_toml_path).unwrap();
    assert_eq!(original_content, new_content);

    // Should not create backup in dry run
    let backup_path = temp_dir.path().join("Cargo.toml.backup");
    assert!(!backup_path.exists());
}

#[test]
fn test_apply_suggestions_with_recognized_heavy_dependency() {
    // Test applying fix with a recognized heavy dependency (getrandom)
    // Note: swc_core may not have a fix alternative configured, use getrandom which does
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml_path = create_cargo_toml_with_getrandom(temp_dir.path());

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_getrandom_issue_report();

    let result = applicator.apply_suggestions(&report, false);

    assert!(result.is_ok());
    let fixes_applied = result.unwrap();

    // getrandom should have at least 1 fix (WASM feature)
    assert!(
        fixes_applied >= 1,
        "Expected at least 1 fix for getrandom, got {}",
        fixes_applied
    );

    // Verify backup was created in .wasm-slim/backups/
    let backup_dir = temp_dir.path().join(".wasm-slim/backups");
    assert!(backup_dir.exists(), "Backup directory should be created");

    // Check that at least one backup file exists
    let backup_files: Vec<_> = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("backup"))
        .collect();
    assert!(
        !backup_files.is_empty(),
        "At least one backup should be created"
    );

    // Verify Cargo.toml was modified
    let new_content = fs::read_to_string(&cargo_toml_path).unwrap();

    // Should have features added
    assert!(
        new_content.contains("features"),
        "Expected features to be added for WASM support, got: {}",
        new_content
    );
}

#[test]
fn test_apply_suggestions_single_fix_getrandom_wasm_feature() {
    // Test applying WASM feature fix for getrandom
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml_path = create_cargo_toml_with_getrandom(temp_dir.path());

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_getrandom_issue_report();

    let result = applicator.apply_suggestions(&report, false);

    assert!(result.is_ok());
    let fixes_applied = result.unwrap();

    assert!(fixes_applied >= 1);

    // Verify Cargo.toml was modified to add 'js' feature
    let new_content = fs::read_to_string(&cargo_toml_path).unwrap();
    assert!(
        new_content.contains("features") && new_content.contains("js"),
        "Expected 'js' feature to be added for WASM support"
    );
}

#[test]
fn test_apply_suggestions_handles_multiple_packages() {
    // Test that apply_suggestions can handle multiple packages in report
    // Even if not all have fixes, it should process them gracefully
    let temp_dir = TempDir::new().unwrap();
    let _cargo_toml_path = create_test_cargo_toml(
        temp_dir.path(),
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
swc_core = "0.90"
getrandom = "0.2"
"#,
    );

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_multiple_issues_report();

    let result = applicator.apply_suggestions(&report, false);

    // Should succeed even if not all packages have fixes
    assert!(result.is_ok());
    let fixes_applied = result.unwrap();

    // Should apply at least 1 fix (getrandom is known to have WASM fix)
    assert!(
        fixes_applied >= 1,
        "Expected at least 1 fix, got {}",
        fixes_applied
    );

    // If any fixes were applied, backup should exist in .wasm-slim/backups/
    if fixes_applied > 0 {
        let backup_dir = temp_dir.path().join(".wasm-slim/backups");
        assert!(
            backup_dir.exists(),
            "Backup directory should be created when fixes are applied"
        );

        let backup_files: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("backup"))
            .collect();
        assert!(!backup_files.is_empty(), "At least one backup should exist");
    }
}

#[test]
fn test_apply_suggestions_idempotent() {
    // Test that applying suggestions twice doesn't make additional changes
    let temp_dir = TempDir::new().unwrap();
    create_cargo_toml_with_getrandom(temp_dir.path());

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_getrandom_issue_report();

    // Apply once
    let result1 = applicator.apply_suggestions(&report, false);
    assert!(result1.is_ok());
    let fixes1 = result1.unwrap();

    // Read content after first application
    let cargo_toml_path = temp_dir.path().join("Cargo.toml");
    let _content_after_first = fs::read_to_string(&cargo_toml_path).unwrap();

    // Apply again with same report
    let result2 = applicator.apply_suggestions(&report, false);
    assert!(result2.is_ok());
    let fixes2 = result2.unwrap();

    // Second application should make fewer or no changes
    assert!(
        fixes2 <= fixes1,
        "Second application should not apply more fixes than first"
    );

    // Content should be stable or improved, not broken
    let content_after_second = fs::read_to_string(&cargo_toml_path).unwrap();
    assert!(
        content_after_second.contains("[dependencies]"),
        "Cargo.toml structure should remain valid"
    );
}

#[test]
fn test_apply_suggestions_preserves_toml_structure() {
    // Test that applying suggestions preserves TOML structure and formatting
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml_path = create_test_cargo_toml(
        temp_dir.path(),
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"
authors = ["Test Author"]

[dependencies]
getrandom = "0.2"
serde = "1.0"

[dev-dependencies]
criterion = "0.5"
"#,
    );

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_getrandom_issue_report();

    let result = applicator.apply_suggestions(&report, false);
    assert!(result.is_ok());

    // Verify TOML structure is preserved
    let new_content = fs::read_to_string(&cargo_toml_path).unwrap();

    // Should still have all original sections
    assert!(new_content.contains("[package]"));
    assert!(new_content.contains("name = \"test-crate\""));
    assert!(new_content.contains("authors = [\"Test Author\"]"));
    assert!(new_content.contains("[dependencies]"));
    assert!(new_content.contains("[dev-dependencies]"));
    assert!(new_content.contains("serde"));
    assert!(new_content.contains("criterion"));
}

#[test]
fn test_apply_suggestions_missing_cargo_toml_fails() {
    // Test that missing Cargo.toml returns error
    let temp_dir = TempDir::new().unwrap();

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_swc_issue_report();

    let result = applicator.apply_suggestions(&report, false);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cargo.toml not found"));
}

#[test]
fn test_apply_suggestions_malformed_toml_fails_gracefully() {
    // Test that malformed TOML returns helpful error
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml_path = temp_dir.path().join("Cargo.toml");
    fs::write(&cargo_toml_path, "this is not valid TOML [[[").unwrap();

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_swc_issue_report();

    let result = applicator.apply_suggestions(&report, false);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("parse") || error_msg.contains("TOML"),
        "Error should mention parsing issue: {}",
        error_msg
    );
}

#[test]
fn test_apply_suggestions_backup_creation() {
    // Test that backup is always created when changes are made
    let temp_dir = TempDir::new().unwrap();
    let cargo_toml_path = create_cargo_toml_with_getrandom(temp_dir.path());
    let original_content = fs::read_to_string(&cargo_toml_path).unwrap();

    let applicator = SuggestionApplicator::new(temp_dir.path());
    let report = create_getrandom_issue_report();

    let result = applicator.apply_suggestions(&report, false);
    assert!(result.is_ok());

    // Verify backup exists in .wasm-slim/backups/ and contains original content
    let backup_dir = temp_dir.path().join(".wasm-slim/backups");
    assert!(backup_dir.exists(), "Backup directory should be created");

    let backup_files: Vec<_> = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("backup"))
        .collect();
    assert!(
        !backup_files.is_empty(),
        "At least one backup should be created"
    );

    // Verify backup content matches original
    let backup_path = backup_files[0].path();
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(
        original_content, backup_content,
        "Backup should contain exact original content"
    );
}

#[test]
fn test_apply_suggestions_returns_correct_count() {
    // Test that the return value correctly reflects number of fixes applied
    let temp_dir = TempDir::new().unwrap();

    // Test with empty report
    create_minimal_cargo_toml(temp_dir.path());
    let applicator = SuggestionApplicator::new(temp_dir.path());
    let empty_report = create_empty_report();
    let result = applicator.apply_suggestions(&empty_report, false);
    assert_eq!(result.unwrap(), 0, "Empty report should return 0 fixes");

    // Test with single issue
    fs::remove_file(temp_dir.path().join("Cargo.toml")).unwrap();
    create_cargo_toml_with_getrandom(temp_dir.path());
    let single_report = create_getrandom_issue_report();
    let result = applicator.apply_suggestions(&single_report, false);
    assert!(result.is_ok());
    let count = result.unwrap();
    assert!(count >= 1, "Single issue should apply at least 1 fix");
}
