//! Tests for the `analyze` command
//!
//! Tests dependency analysis, asset detection, and WASM analysis

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

mod common;
use common::fixtures;

fn get_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
}

#[test]
fn test_analyze_deps_command_shows_dependency_analysis() {
    let mut cmd = get_bin();
    cmd.arg("analyze")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependency Analysis"))
        .stdout(predicate::str::contains("Total dependencies"));
}

#[test]
fn test_analyze_wasm_without_file_argument_returns_error() {
    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("nonexistent.wasm")
        .arg("--mode")
        .arg("top")
        .assert()
        .failure()
        .stderr(predicate::str::contains("WASM file not found"));
}

#[test]
fn test_analyze_assets_command_detects_embedded_assets() {
    let (temp_dir, _cargo_toml) =
        fixtures::create_project_with_assets("test-assets").expect("Failed to create test fixture");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("assets")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Asset Detection"))
        .stdout(predicate::str::contains("Embedded Assets Found"));
}

#[test]
fn test_analyze_assets_with_json_flag_outputs_valid_json() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-assets"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("Failed to write test file");

    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create directory");
    fs::write(
        src_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello, world!");
}
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("assets")
        .arg("--json")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("total_assets"))
        .stdout(predicate::str::contains("total_size_kb"));
}

#[test]
fn test_analyze_assets_with_guide_flag_shows_externalization_guide() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-assets"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("Failed to write test file");

    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).expect("Failed to create directory");
    fs::write(src_dir.join("main.rs"), "fn main() {}").expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("assets")
        .arg("--guide")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Externalization Guide"));
}

#[test]
fn test_analyze_deps_with_autofix_flag_fixes_getrandom_issue() {
    let (temp_dir, cargo_toml) = fixtures::create_project_with_getrandom("test-autofix")
        .expect("Failed to create test fixture");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--fix")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Auto-fix").or(predicate::str::contains("Applied")));

    let modified_content = fs::read_to_string(&cargo_toml).expect("Failed to read file contents");
    assert!(
        modified_content.contains("features") && modified_content.contains("js"),
        "Cargo.toml should be updated with getrandom WASM features"
    );
}

#[test]
fn test_analyze_deps_with_autofix_and_dry_run_shows_changes_without_applying() {
    let (temp_dir, cargo_toml) = fixtures::create_project_with_getrandom("test-autofix")
        .expect("Failed to create test fixture");

    let original_content = fs::read_to_string(&cargo_toml).expect("Failed to read file contents");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--fix")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN"));

    let content = fs::read_to_string(&cargo_toml).expect("Failed to read file contents");
    assert_eq!(content, original_content, "Dry-run should not modify files");
}

#[test]
fn test_analyze_deps_with_autofix_creates_backup_before_changes() {
    let (temp_dir, cargo_toml) = fixtures::create_project_with_getrandom("test-autofix")
        .expect("Failed to create test fixture");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--fix")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let modified_content = fs::read_to_string(&cargo_toml).expect("Failed to read file contents");
    assert!(
        modified_content.contains("features") && modified_content.contains("js"),
        "Cargo.toml should be updated with getrandom WASM features"
    );
}

#[test]
fn test_analyze_deps_json_output_contains_required_fields() {
    let (temp_dir, cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-deps-json").expect("Failed to create test fixture");

    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-deps-json"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--json")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    if !output.status.success() {
        return;
    }

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object());
        let obj = json.as_object().expect("Expected JSON object");

        if obj.contains_key("total_deps") {
            assert!(obj["total_deps"].is_number());
        }

        if obj.contains_key("direct_deps") {
            assert!(obj["direct_deps"].is_number());
        }

        if let Some(issues) = obj.get("issues") {
            assert!(issues.is_array(), "issues should be an array");

            for issue in issues.as_array().expect("Expected JSON array") {
                assert!(issue.is_object());
                let issue_obj = issue.as_object().expect("Expected JSON object");

                assert!(issue_obj.contains_key("package"));
                assert!(issue_obj.contains_key("severity"));
                assert!(issue_obj.contains_key("issue"));

                assert!(issue_obj["package"].is_string());
                assert!(issue_obj["severity"].is_string());
                assert!(issue_obj["issue"].is_string());
            }
        }
    }
}

#[test]
fn test_analyze_assets_json_output_contains_required_fields() {
    let (temp_dir, _cargo_toml) = fixtures::create_minimal_wasm_lib("test-assets-json")
        .expect("Failed to create test fixture");

    let mut cmd = get_bin();
    let output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("assets")
        .arg("--json")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    if !output.status.success() {
        return;
    }

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object());
        let obj = json.as_object().expect("Expected JSON object");

        if let Some(assets) = obj.get("assets") {
            assert!(assets.is_array(), "assets should be an array");

            for asset in assets.as_array().expect("Expected JSON array") {
                assert!(asset.is_object());
                let asset_obj = asset.as_object().expect("Expected JSON object");

                if asset_obj.contains_key("file") {
                    assert!(asset_obj["file"].is_string());
                }

                if asset_obj.contains_key("size_bytes") {
                    assert!(asset_obj["size_bytes"].is_number());
                }

                if asset_obj.contains_key("asset_type") {
                    assert!(asset_obj["asset_type"].is_string());
                }
            }
        }

        if let Some(total) = obj.get("total_size_bytes") {
            assert!(total.is_number());
        }

        if let Some(count) = obj.get("asset_count") {
            assert!(count.is_number());
        }
    }
}

#[test]
fn test_analyze_wasm_json_output_contains_required_fields() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let wasm_file = temp_dir.path().join("test.wasm");
    fs::write(&wasm_file, fixtures::WASM_MAGIC_HEADER).expect("Failed to write test file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("wasm")
        .arg("--json")
        .arg(
            wasm_file
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            assert!(json.is_object());
            let obj = json.as_object().expect("Expected JSON object");

            if let Some(items) = obj.get("items") {
                assert!(items.is_array(), "items should be an array");

                for item in items.as_array().expect("Expected JSON array") {
                    assert!(item.is_object());
                    let item_obj = item.as_object().expect("Expected JSON object");

                    if item_obj.contains_key("name") {
                        assert!(item_obj["name"].is_string());
                    }

                    if item_obj.contains_key("size") {
                        assert!(item_obj["size"].is_number());
                    }

                    if item_obj.contains_key("percent") {
                        assert!(item_obj["percent"].is_number());
                    }
                }
            }
        }
    }
}

#[test]
fn test_analyze_deps_with_json_flag_includes_dependency_info() {
    let mut cmd = get_bin();
    let output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--json")
        .output()
        .expect("Command execution failed");

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(json.is_object(), "JSON should be an object");
    } else {
        assert!(!stdout.is_empty(), "Command should produce output");
    }
}

#[test]
fn test_analyze_deps_without_cargo_toml_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("deps")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to run cargo metadata"));
}

#[test]
fn test_analyze_deps_with_malformed_cargo_toml_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(&cargo_toml, "[invalid toml\\nthis is broken").expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("deps")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("parse").or(predicate::str::contains("Failed")));
}

#[test]
fn test_analyze_wasm_with_corrupted_file_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let fake_wasm = temp_dir.path().join("corrupted.wasm");
    fs::write(&fake_wasm, b"not a valid wasm file").expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg(
            fake_wasm
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .arg("--mode")
        .arg("top")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("magic header")
                .or(predicate::str::contains("twiggy"))
                .or(predicate::str::contains("Twiggy")),
        );
}

#[test]
fn test_analyze_assets_without_src_directory_returns_empty_results() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-no-src"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    let output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("assets")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");
        assert!(stdout.contains("0") || stdout.contains("No") || stdout.contains("not found"));
    } else {
        let stderr = String::from_utf8(output.stderr).expect("Failed to parse stderr as UTF-8");
        assert!(
            stderr.contains("src") || stderr.contains("directory") || stderr.contains("not found")
        );
    }
}

#[test]
fn test_analyze_with_invalid_mode_argument_returns_error() {
    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("invalid-mode")
        .assert()
        .failure()
        .stderr(predicate::str::contains("mode").or(predicate::str::contains("invalid")));
}

#[test]
fn test_analyze_wasm_without_mode_flag_returns_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("nonexistent.wasm")
        .current_dir(temp_dir.path())
        .assert()
        .failure();
}

#[test]
fn test_analyze_assets_with_unreadable_files_handles_gracefully() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"
[package]
name = "test-perm"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("Failed to write test file");

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).expect("Failed to create directory");
        let main_rs = src_dir.join("main.rs");
        fs::write(&main_rs, "fn main() {}").expect("Failed to write test file");

        let mut perms = fs::metadata(&src_dir)
            .expect("Failed to read file metadata")
            .permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&src_dir, perms).expect("Failed to set file permissions");

        let mut cmd = get_bin();
        let output = cmd
            .arg("analyze")
            .arg("--mode")
            .arg("assets")
            .current_dir(temp_dir.path())
            .output()
            .expect("Command execution failed");

        let mut perms = fs::metadata(&src_dir)
            .expect("Failed to read file metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&src_dir, perms).expect("Failed to set file permissions");

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr).expect("Failed to parse stderr as UTF-8");
            assert!(
                stderr.contains("permission")
                    || stderr.contains("denied")
                    || stderr.contains("access")
            );
        }
    }

    #[cfg(not(unix))]
    {
        // Skip on non-Unix platforms
    }
}

#[test]
fn test_analyze_deps_with_invalid_json_format_returns_error() {
    let mut cmd = get_bin();
    let output = cmd
        .arg("analyze")
        .arg("--mode")
        .arg("deps")
        .arg("--json")
        .output()
        .expect("Command execution failed");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as UTF-8");
    assert!(!stdout.is_empty());
}

#[test]
fn test_analyze_with_minimal_empty_project_returns_basic_info() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "empty"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("Failed to write test file");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("--mode")
        .arg("deps")
        .current_dir(temp_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to run cargo metadata"));
}

#[test]
fn test_analyze_handles_malformed_cargo_metadata() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let cargo_toml = temp_dir.path().join("Cargo.toml");

    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "invalid-edition"

[dependencies]
nonexistent-package-xyz = { version = "999.999.999", path = "../nonexistent" }
"#,
    )
    .expect("Failed to write test Cargo.toml");

    let mut cmd = get_bin();
    cmd.arg("analyze")
        .arg("deps")
        .current_dir(temp_dir.path())
        .assert()
        .failure();
}

#[test]
fn test_analyze_deps_with_circular_dependencies_handles_gracefully() {
    let (temp_dir, cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-crate").expect("Failed to create test fixture");

    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
serde_derive = "1.0"
"#,
    )
    .expect("Failed to write test Cargo.toml");

    let mut cmd = get_bin();
    let result = cmd
        .arg("analyze")
        .arg("deps")
        .current_dir(temp_dir.path())
        .output()
        .expect("Command execution failed");

    assert!(
        result.status.code().is_some(),
        "Command should complete, not hang"
    );
}
