//! Cross-platform integration tests
//!
//! Tests cross-platform behavior including path separators, executable extensions,
//! line endings, temp directories, file locking, and filesystem case sensitivity.
//!
//! Covers Windows, macOS, and Linux platform-specific behaviors.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn test_path_separator_handling() {
    // Test that using std::path::Path APIs works on all platforms
    let path_str = "src/lib.rs";
    let path = PathBuf::from(path_str);

    // Should have 2 components: src and lib.rs
    assert_eq!(path.components().count(), 2);

    // Building path with join should work consistently
    let built_path = PathBuf::from("src").join("lib.rs");
    assert_eq!(
        path, built_path,
        "Paths built differently should be equivalent"
    );
}

#[test]
fn test_pathbuf_join_cross_platform() {
    // Test that PathBuf::join produces correct paths on all platforms
    let base = PathBuf::from("project");
    let path = base.join("src").join("lib.rs");

    assert!(path.to_string_lossy().contains("src"));
    assert!(path.to_string_lossy().contains("lib.rs"));

    // Path should be valid on current platform
    assert!(path.is_relative());
}

#[test]
fn test_temp_directory_cross_platform() {
    // Test that temp directories work on all platforms
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    assert!(temp_path.exists());
    assert!(temp_path.is_dir());

    // Should be able to create files in temp directory
    let test_file = temp_path.join("test.txt");
    fs::write(&test_file, "test content").unwrap();
    assert!(test_file.exists());
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_path_separator() {
    // Windows-specific: Test backslash separator
    let path = PathBuf::from("C:\\Users\\test\\project");

    assert!(path.to_string_lossy().contains("\\"));
    // Windows paths have: Prefix(C:), RootDir(\), Users, test, project = 5 components
    assert_eq!(path.components().count(), 5);
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_executable_extension() {
    // Windows-specific: Executables should have .exe extension
    let cargo_exe = which::which("cargo").unwrap_or_else(|_| PathBuf::from("cargo"));

    // If cargo is found, it should have .exe extension on Windows
    if cargo_exe.exists() {
        assert!(
            cargo_exe.extension().map_or(false, |ext| ext == "exe"),
            "Windows executables should have .exe extension"
        );
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_unix_executable_no_extension() {
    // Unix-specific: Executables typically don't have extensions
    let cargo_exe = which::which("cargo").unwrap_or_else(|_| PathBuf::from("cargo"));

    if cargo_exe.exists() && cargo_exe.file_name().unwrap() == "cargo" {
        assert!(
            cargo_exe.extension().is_none(),
            "Unix executables typically don't have extensions"
        );
    }
}

#[test]
fn test_line_ending_agnostic_reading() {
    // Test that file reading handles different line endings
    let temp_dir = TempDir::new().unwrap();

    // Create file with Unix line endings
    let unix_file = temp_dir.path().join("unix.txt");
    fs::write(&unix_file, "line1\nline2\nline3\n").unwrap();

    // Create file with Windows line endings
    let windows_file = temp_dir.path().join("windows.txt");
    fs::write(&windows_file, "line1\r\nline2\r\nline3\r\n").unwrap();

    // Both should be readable
    let unix_content = fs::read_to_string(&unix_file).unwrap();
    let windows_content = fs::read_to_string(&windows_file).unwrap();

    // Count lines in both files (using lines() which is line-ending agnostic)
    let unix_lines: Vec<_> = unix_content.lines().collect();
    let windows_lines: Vec<_> = windows_content.lines().collect();

    assert_eq!(unix_lines.len(), 3, "Unix file should have 3 lines");
    assert_eq!(windows_lines.len(), 3, "Windows file should have 3 lines");
    assert_eq!(unix_lines, windows_lines, "Line content should match");
}

#[test]
fn test_absolute_vs_relative_paths() {
    // Test distinguishing absolute from relative paths
    let relative = PathBuf::from("src/lib.rs");
    assert!(relative.is_relative());

    let temp_dir = TempDir::new().unwrap();
    let absolute = temp_dir.path().join("test.rs");
    assert!(absolute.is_absolute());
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_drive_letter_handling() {
    // Windows-specific: Test drive letter paths
    let path = PathBuf::from("C:\\Users\\test");

    assert!(path.is_absolute());
    assert!(path.to_string_lossy().starts_with("C:"));
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_unc_path_handling() {
    // Windows-specific: Test UNC path handling
    let unc_path = PathBuf::from("\\\\server\\share\\file.txt");

    // UNC paths should be recognized as absolute
    assert!(unc_path.is_absolute());
}

#[test]
fn test_file_creation_cross_platform() {
    // Test file creation works on all platforms
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_create.txt");

    fs::write(&test_file, "test content").unwrap();

    assert!(test_file.exists());
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "test content");
}

#[test]
fn test_directory_creation_cross_platform() {
    // Test directory creation works on all platforms
    let temp_dir = TempDir::new().unwrap();
    let nested_dir = temp_dir.path().join("level1").join("level2").join("level3");

    fs::create_dir_all(&nested_dir).unwrap();

    assert!(nested_dir.exists());
    assert!(nested_dir.is_dir());
}

#[test]
fn test_path_canonicalization() {
    // Test path canonicalization (resolves symlinks, makes absolute)
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "content").unwrap();

    let canonical = test_file.canonicalize().unwrap();

    assert!(canonical.is_absolute());
    assert!(canonical.exists());
}

#[test]
fn test_filename_extraction() {
    // Test extracting filename from path
    let path = PathBuf::from("src/module/file.rs");

    assert_eq!(path.file_name().unwrap(), "file.rs");
    assert_eq!(path.extension().unwrap(), "rs");
    assert_eq!(path.file_stem().unwrap(), "file");
}

#[test]
fn test_parent_directory_navigation() {
    // Test navigating to parent directories
    let path = PathBuf::from("project/src/lib.rs");

    let parent = path.parent().unwrap();
    assert_eq!(parent, Path::new("project/src"));

    let grandparent = parent.parent().unwrap();
    assert_eq!(grandparent, Path::new("project"));
}

#[test]
fn test_filesystem_case_sensitivity() {
    // Test that detects actual filesystem behavior rather than assuming based on OS
    let temp_dir = TempDir::new().unwrap();
    let lowercase_file = temp_dir.path().join("test.txt");
    fs::write(&lowercase_file, "content").unwrap();

    // Try to access with different case
    let uppercase_file = temp_dir.path().join("TEST.TXT");
    let can_read = fs::read_to_string(&uppercase_file).is_ok();

    // Expected behavior by platform:
    // - Windows: case-insensitive (can_read = true)
    // - macOS: typically case-insensitive (can_read = true)
    // - Linux: case-sensitive (can_read = false)
    #[cfg(target_os = "windows")]
    assert!(
        can_read,
        "Windows filesystems are typically case-insensitive"
    );

    #[cfg(target_os = "macos")]
    {
        // macOS can be either case-sensitive or case-insensitive
        // Default APFS is case-insensitive, but don't enforce
        // Just document the behavior found
        if can_read {
            println!("macOS filesystem is case-insensitive (default APFS behavior)");
        } else {
            println!("macOS filesystem is case-sensitive (APFS case-sensitive variant)");
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    assert!(!can_read, "Linux filesystems are typically case-sensitive");
}

#[test]
fn test_path_comparison_cross_platform() {
    // Test path comparison works correctly
    let path1 = PathBuf::from("src/lib.rs");
    let path2 = PathBuf::from("src").join("lib.rs");

    assert_eq!(path1, path2, "Equivalent paths should be equal");
}

#[test]
fn test_relative_path_resolution() {
    // Test resolving relative paths
    let base = PathBuf::from("project/src");
    let relative = PathBuf::from("../tests/test.rs");

    let joined = base.join(&relative);

    // Should contain both components
    assert!(joined.to_string_lossy().contains("project"));
    assert!(joined.to_string_lossy().contains("tests"));
}

#[test]
fn test_strip_prefix_cross_platform() {
    // Test stripping path prefixes
    let full_path = PathBuf::from("project/src/module/file.rs");
    let prefix = PathBuf::from("project/src");

    let stripped = full_path.strip_prefix(&prefix).unwrap();
    assert_eq!(stripped, Path::new("module/file.rs"));
}
