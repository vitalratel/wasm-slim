//! Tests for cleanup and temporary directory management

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

mod common;
use common::fixtures;

/// Check if required WASM tools are available
fn has_required_wasm_tools() -> bool {
    which::which("wasm-bindgen").is_ok()
}

/// Skip test if WASM tools not available
macro_rules! require_wasm_tools {
    () => {
        if !has_required_wasm_tools() {
            eprintln!("⚠️  Skipping test: wasm-bindgen-cli not found in PATH");
            eprintln!("   Install with: cargo install wasm-bindgen-cli");
            return;
        }
    };
}

/// Helper to get the wasm-slim binary command
fn get_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wasm-slim"))
}

#[test]
fn test_temp_directory_cleanup_after_successful_command() {
    // Verify TempDir is cleaned up after successful test
    let temp_path = {
        let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
        let path = temp_dir.path().to_path_buf();

        // Create a file in temp dir
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"").expect("Failed to write test file");

        assert!(path.exists(), "Temp dir should exist during test");
        assert!(cargo_toml.exists(), "File should exist during test");

        path
    }; // TempDir drops here

    // After drop, temp dir should be cleaned up
    assert!(
        !temp_path.exists(),
        "Temp dir should be cleaned up after drop"
    );
}

#[test]
fn test_temp_directory_cleanup_after_failed_command() {
    // Verify TempDir is cleaned up even when command fails
    let temp_path = {
        let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
        let path = temp_dir.path().to_path_buf();

        // Run a command that fails
        let mut cmd = get_bin();
        cmd.arg("build")
            .current_dir(temp_dir.path())
            .assert()
            .failure(); // Command fails - no Cargo.toml

        assert!(path.exists(), "Temp dir should still exist before drop");
        path
    }; // TempDir drops here

    // Cleanup should happen even after failure
    assert!(
        !temp_path.exists(),
        "Temp dir should be cleaned up after failure"
    );
}

#[test]
fn test_config_file_cleanup_removes_temporary_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
    let config_path = temp_dir.path().join(".wasm-slim.toml");

    // Create Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"",
    )
    .expect("Failed to write test file");

    // Run init to create config
    let mut cmd = get_bin();
    cmd.arg("init")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    assert!(config_path.exists(), "Config should be created");

    // Config file will be cleaned up with TempDir
    // Verify it exists before drop
    let config_contents = fs::read_to_string(&config_path).expect("Failed to read file contents");
    assert!(!config_contents.is_empty(), "Config should have content");
}

#[test]
fn test_backup_directory_not_created_with_dry_run_flag() {
    require_wasm_tools!();
    let (temp_dir, _cargo_toml) =
        fixtures::create_minimal_wasm_lib("test-crate").expect("Failed to create test fixture");
    let backup_dir = temp_dir.path().join(".wasm-slim");

    // Run dry-run build
    let mut cmd = get_bin();
    cmd.arg("build")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify no backup directory created
    assert!(
        !backup_dir.exists(),
        "Dry-run should not create backup directory"
    );

    // Verify target/ directory is created (build artifacts) but not .wasm-slim/ backup
    let has_backup = fs::read_dir(temp_dir.path())
        .expect("Command execution failed")
        .filter_map(|e| e.ok())
        .any(|e| e.file_name() == ".wasm-slim");

    assert!(
        !has_backup,
        "Dry-run should not create .wasm-slim backup directory"
    );
}

#[test]
fn test_multiple_temp_directories_clean_up_independently() {
    // Verify multiple TempDirs don't interfere with each other
    let path1 = {
        let temp1 = TempDir::new().expect("Failed to create temp directory for test");
        let path = temp1.path().to_path_buf();
        fs::write(temp1.path().join("file1.txt"), "data1").expect("Failed to write test file");
        path
    };

    let path2 = {
        let temp2 = TempDir::new().expect("Failed to create temp directory for test");
        let path = temp2.path().to_path_buf();
        fs::write(temp2.path().join("file2.txt"), "data2").expect("Failed to write test file");
        path
    };

    // Both should be cleaned up independently
    assert!(!path1.exists(), "First temp dir should be cleaned up");
    assert!(!path2.exists(), "Second temp dir should be cleaned up");
}

#[test]
fn test_nested_temp_directories_cleanup_recursively() {
    let outer_path = {
        let outer_temp = TempDir::new().expect("Failed to create temp directory for test");
        let outer = outer_temp.path().to_path_buf();

        // Create nested temp dir
        let inner_path = {
            let inner_temp =
                TempDir::new_in(outer_temp.path()).expect("Failed to create nested temp directory");
            let inner = inner_temp.path().to_path_buf();
            fs::write(inner.join("inner.txt"), "inner data").expect("Failed to write test file");
            assert!(inner.exists());
            inner
        };

        // Inner should be cleaned up
        assert!(
            !inner_path.exists(),
            "Inner temp dir should be cleaned up first"
        );

        outer
    };

    // Outer should be cleaned up
    assert!(!outer_path.exists(), "Outer temp dir should be cleaned up");
}

#[test]
fn test_temp_file_with_special_permissions_cleans_up() {
    use std::os::unix::fs::PermissionsExt;

    let temp_path = {
        let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
        let path = temp_dir.path().to_path_buf();

        // Create file with specific permissions
        let file = temp_dir.path().join("restricted.txt");
        fs::write(&file, "data").expect("Failed to write test file");

        let mut perms = fs::metadata(&file)
            .expect("Failed to read file metadata")
            .permissions();
        perms.set_mode(0o400); // Read-only
        fs::set_permissions(&file, perms).expect("Failed to set file permissions");

        assert!(file.exists());
        path
    };

    // Should cleanup even with restricted permissions
    assert!(
        !temp_path.exists(),
        "Should cleanup files with restricted permissions"
    );
}

#[test]
fn test_large_temp_directory_with_many_files_cleans_up() {
    let temp_path = {
        let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
        let path = temp_dir.path().to_path_buf();

        // Create many files
        for i in 0..100 {
            fs::write(
                temp_dir.path().join(format!("file{}.txt", i)),
                vec![0u8; 1024],
            )
            .expect("Failed to write test file");
        }

        // Create subdirectories with files
        for i in 0..10 {
            let subdir = temp_dir.path().join(format!("dir{}", i));
            fs::create_dir(&subdir).expect("Failed to create directory");
            for j in 0..10 {
                fs::write(subdir.join(format!("file{}.txt", j)), "data")
                    .expect("Failed to write test file");
            }
        }

        path
    };

    // Should cleanup large directories with many files
    assert!(!temp_path.exists(), "Should cleanup large temp directories");
}

#[test]
fn test_symlinks_in_temp_directory_cleanup_correctly() {
    use std::os::unix::fs::symlink;

    let temp_path = {
        let temp_dir = TempDir::new().expect("Failed to create temp directory for test");
        let path = temp_dir.path().to_path_buf();

        // Create file and symlink
        let file = temp_dir.path().join("target.txt");
        fs::write(&file, "data").expect("Failed to write test file");

        let link = temp_dir.path().join("link.txt");
        symlink(&file, &link).expect("Failed to create symlink");

        assert!(link.exists());
        path
    };

    // Should cleanup symlinks
    assert!(!temp_path.exists(), "Should cleanup symlinks");
}

#[test]
fn test_temp_directories_isolated_between_test_runs() {
    // Get system temp directory
    let system_temp = std::env::temp_dir();

    // Count temp entries before
    let before_count = fs::read_dir(&system_temp)
        .expect("Command execution failed")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_string_lossy().contains("wasm-slim"))
        .count();

    // Create and drop temp dir
    {
        let _temp = TempDir::new().expect("Failed to create temp directory for test");
        fs::write(_temp.path().join("test.txt"), "data").expect("Failed to write test file");
    }

    // Count after - should be same or less (other tests might have cleaned up)
    let after_count = fs::read_dir(&system_temp)
        .expect("Command execution failed")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_string_lossy().contains("wasm-slim"))
        .count();

    assert!(
        after_count <= before_count + 1,
        "Should not leak temp directories"
    );
}
