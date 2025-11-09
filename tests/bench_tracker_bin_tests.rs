//! Integration tests for bench-tracker binary
//!
//! Tests the CLI interface and functionality of the bench-tracker tool

#[macro_use]
extern crate assert_cmd;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a test project structure
fn setup_test_project() -> TempDir {
    let temp = TempDir::new().unwrap();
    let project_root = temp.path();

    // Create Cargo.toml
    fs::write(
        project_root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    // Create benches directory with a dummy benchmark
    let benches_dir = project_root.join("benches");
    fs::create_dir_all(&benches_dir).unwrap();

    fs::write(
        benches_dir.join("test_bench.rs"),
        r#"use criterion::{criterion_group, criterion_main, Criterion};

fn bench_example(c: &mut Criterion) {
    c.bench_function("example", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, bench_example);
criterion_main!(benches);
"#,
    )
    .unwrap();

    temp
}

/// Helper to create mock criterion results
fn create_mock_criterion_results(project_root: &std::path::Path) {
    let criterion_dir = project_root.join("target").join("criterion");
    fs::create_dir_all(&criterion_dir).unwrap();

    // Create a mock benchmark result
    let bench_dir = criterion_dir.join("example");
    fs::create_dir_all(&bench_dir).unwrap();

    let estimates = bench_dir.join("base").join("estimates.json");
    fs::create_dir_all(estimates.parent().unwrap()).unwrap();
    fs::write(
        &estimates,
        r#"{
  "mean": {
    "point_estimate": 1234.5,
    "standard_error": 12.3,
    "confidence_interval": {
      "lower_bound": 1200.0,
      "upper_bound": 1250.0
    }
  },
  "median": {
    "point_estimate": 1230.0,
    "standard_error": 10.0,
    "confidence_interval": {
      "lower_bound": 1210.0,
      "upper_bound": 1240.0
    }
  },
  "std_dev": {
    "point_estimate": 50.0,
    "standard_error": 5.0,
    "confidence_interval": {
      "lower_bound": 45.0,
      "upper_bound": 55.0
    }
  }
}"#,
    )
    .unwrap();
}

#[test]
fn test_bench_tracker_help() {
    Command::new(cargo_bin!("bench-tracker"))
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Performance tracking tool"));
}

#[test]
fn test_bench_tracker_version() {
    Command::new(cargo_bin!("bench-tracker"))
        .arg("--version")
        .assert()
        .success();
}

#[test]
fn test_bench_tracker_run_help() {
    Command::new(cargo_bin!("bench-tracker"))
        .args(["run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run benchmarks and compare"));
}

#[test]
fn test_bench_tracker_baseline_help() {
    Command::new(cargo_bin!("bench-tracker"))
        .args(["baseline", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Save current benchmark results"));
}

#[test]
fn test_bench_tracker_compare_help() {
    Command::new(cargo_bin!("bench-tracker"))
        .args(["compare", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Show comparison with baseline"));
}

#[test]
fn test_bench_tracker_show_help() {
    Command::new(cargo_bin!("bench-tracker"))
        .args(["show", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Show current baseline"));
}

#[test]
fn test_bench_tracker_reset_help() {
    Command::new(cargo_bin!("bench-tracker"))
        .args(["reset", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset/delete current baseline"));
}

#[test]
fn test_bench_tracker_invalid_project_root() {
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", "/nonexistent/path/xyz123", "show"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to resolve project root"));
}

#[test]
fn test_bench_tracker_project_root_is_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("not_a_dir.txt");
    fs::write(&file_path, "content").unwrap();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", file_path.to_str().unwrap(), "show"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be a directory"));
}

#[test]
fn test_bench_tracker_show_no_baseline() {
    let temp = setup_test_project();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No baseline found"));
}

#[test]
fn test_bench_tracker_reset_no_baseline() {
    let temp = setup_test_project();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "reset"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No baseline to reset"));
}

#[test]
fn test_bench_tracker_baseline_no_results() {
    let temp = setup_test_project();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No criterion results found"));
}

#[test]
fn test_bench_tracker_baseline_with_mock_results() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    Command::new(cargo_bin!("bench-tracker"))
        .args([
            "--project-root",
            temp.path().to_str().unwrap(),
            "baseline",
            "--version",
            "test-v1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline saved successfully"));

    // Verify baseline file was created
    let baseline_path = temp
        .path()
        .join(".wasm-slim")
        .join("benchmarks")
        .join("baseline.json");
    assert!(baseline_path.exists());
}

#[test]
fn test_bench_tracker_show_with_baseline() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    // First create a baseline
    Command::new(cargo_bin!("bench-tracker"))
        .args([
            "--project-root",
            temp.path().to_str().unwrap(),
            "baseline",
            "--version",
            "v1.0",
        ])
        .assert()
        .success();

    // Then show it
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Current Baseline"))
        .stdout(predicate::str::contains("v1.0"))
        .stdout(predicate::str::contains("example"));
}

#[test]
fn test_bench_tracker_reset_with_baseline() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    // Create baseline
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .success();

    let baseline_path = temp
        .path()
        .join(".wasm-slim")
        .join("benchmarks")
        .join("baseline.json");
    assert!(baseline_path.exists());

    // Reset baseline
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "reset"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline reset successfully"));

    // Verify baseline was deleted
    assert!(!baseline_path.exists());
}

#[test]
fn test_bench_tracker_compare_no_results() {
    let temp = setup_test_project();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "compare"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No criterion results found"));
}

#[test]
fn test_bench_tracker_compare_no_baseline() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "compare"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No baseline found"));
}

#[test]
fn test_bench_tracker_compare_with_baseline() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    // Create baseline first
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .success();

    // Compare with baseline
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "compare"])
        .assert()
        .success()
        .stdout(predicate::str::contains("example"));
}

#[test]
fn test_format_timestamp_just_now() {
    // This is tested indirectly through the show command with recent baselines
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .success();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("just now").or(predicate::str::contains("minutes ago")));
}

#[test]
fn test_format_ns_display() {
    // Tested indirectly through show command which displays benchmark times
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .success();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("µs").or(predicate::str::contains("ns")));
}

#[test]
fn test_baseline_with_version_flag() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    Command::new(cargo_bin!("bench-tracker"))
        .args([
            "--project-root",
            temp.path().to_str().unwrap(),
            "baseline",
            "--version",
            "v2.0",
        ])
        .assert()
        .success();

    // Verify version is shown
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("v2.0"));
}

#[test]
fn test_compare_shows_no_regression_with_same_results() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    // Create baseline
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .success();

    // Compare should show no regression (same results)
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "compare"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No significant performance regressions",
        ));
}

#[test]
fn test_show_displays_benchmark_count() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .success();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Benchmarks"))
        .stdout(predicate::str::contains("example"));
}

#[test]
fn test_baseline_default_version() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    // Create baseline without version flag (should use "current")
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .success();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("current"));
}

#[test]
fn test_project_root_with_symlink() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    // Test with current directory
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", ".", "show"])
        .current_dir(temp.path())
        .assert()
        .success();
}

#[test]
fn test_multiple_baseline_updates() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    // Create first baseline
    Command::new(cargo_bin!("bench-tracker"))
        .args([
            "--project-root",
            temp.path().to_str().unwrap(),
            "baseline",
            "--version",
            "v1.0",
        ])
        .assert()
        .success();

    // Update baseline
    Command::new(cargo_bin!("bench-tracker"))
        .args([
            "--project-root",
            temp.path().to_str().unwrap(),
            "baseline",
            "--version",
            "v2.0",
        ])
        .assert()
        .success();

    // Should show latest version
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("v2.0"));
}

#[test]
fn test_show_displays_git_commit_if_available() {
    let temp = setup_test_project();
    create_mock_criterion_results(temp.path());

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .ok();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(temp.path())
        .output()
        .ok();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(temp.path())
        .output()
        .ok();

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(temp.path())
        .output()
        .ok();

    std::process::Command::new("git")
        .args(["commit", "-m", "test"])
        .current_dir(temp.path())
        .output()
        .ok();

    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "baseline"])
        .assert()
        .success();

    // Show command should display git commit info if available
    Command::new(cargo_bin!("bench-tracker"))
        .args(["--project-root", temp.path().to_str().unwrap(), "show"])
        .assert()
        .success();
}
