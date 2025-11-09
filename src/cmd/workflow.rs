//! Build workflow orchestration
//! Build workflow orchestration module.
//!
//! This module provides the core build workflow logic that orchestrates the complete
//! WASM optimization pipeline. It separates business logic from presentation concerns,
//! allowing the build process to be used programmatically or via CLI.
//!
//! # Architecture
//!
//! The workflow follows a three-phase approach:
//!
//! 1. **Cargo.toml Optimization**: Analyzes and applies optimizations to the Cargo.toml
//!    file, including profile settings, dependency features, and build-std configuration.
//!
//! 2. **WASM Build Pipeline**: Executes the actual WASM build with optimized settings,
//!    runs wasm-opt for further size reduction, and tracks build metrics.
//!
//! 3. **CI/CD Metrics Validation**: Validates build outputs against configured size budgets
//!    and records historical build data for regression detection.
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```no_run
//! use std::path::Path;
//! use wasm_slim::cmd::workflow::BuildWorkflow;
//!
//! let workflow = BuildWorkflow::new(Path::new("."));
//! let result = workflow.execute(false, false, None).expect("Build failed");
//! println!("Build completed: {} -> {} bytes",
//!          result.metrics.before_bytes, result.metrics.after_bytes);
//! ```
//!
//! Dry-run mode (preview changes without applying):
//!
//! ```no_run
//! # use std::path::Path;
//! # use wasm_slim::cmd::workflow::BuildWorkflow;
//! let workflow = BuildWorkflow::new(Path::new("."));
//! let result = workflow.execute(true, false, None).expect("Dry-run failed");
//! for file in &result.dry_run_files {
//!     println!("Would modify: {}", file);
//! }
//! ```
//!
//! With budget checking:
//!
//! ```no_run
//! # use std::path::Path;
//! # use wasm_slim::cmd::workflow::BuildWorkflow;
//! let workflow = BuildWorkflow::new(Path::new("."));
//! let result = workflow.execute(false, true, None).expect("Build failed");
//! if let Some(passed) = result.budget_check_passed {
//!     assert!(passed, "Build exceeded size budget!");
//! }
//! ```

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::{config, optimizer, pipeline};

/// Result of the complete build workflow
#[derive(Debug)]
pub struct BuildResult {
    /// Changes made during Cargo.toml optimization
    pub cargo_changes: Vec<String>,
    /// Build metrics (sizes, reduction)
    pub metrics: pipeline::SizeMetrics,
    /// Whether size budget check passed (if applicable)
    pub budget_check_passed: Option<bool>,
    /// Size budget threshold (if configured)
    pub budget_threshold: Option<u64>,
    /// Whether this was a dry-run
    pub dry_run: bool,
    /// Files that would be optimized in dry-run mode
    pub dry_run_files: Vec<String>,
}

/// Result type for Cargo.toml optimization with backup information
///
/// Returns: (changes_made, dry_run_files, backup_paths_with_content)
type OptimizationResult = (Vec<String>, Vec<String>, Vec<(PathBuf, String)>);

/// Build workflow orchestrator
///
/// Coordinates the three-phase build process:
/// 1. Cargo.toml optimization
/// 2. WASM build pipeline
/// 3. CI/CD metrics validation
pub struct BuildWorkflow {
    project_root: PathBuf,
}

impl BuildWorkflow {
    /// Create a new build workflow for the given project
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
        }
    }

    /// Execute the complete build workflow
    pub fn execute(
        &self,
        dry_run: bool,
        check_budget: bool,
        _target_dir: Option<&str>,
    ) -> Result<BuildResult> {
        // Phase 1: Optimize Cargo.toml files and save backups
        let (cargo_changes, dry_run_files, backups) =
            self.optimize_cargo_tomls_with_backup(dry_run)?;

        // Phase 2: Run build pipeline
        let metrics = match self.run_build_pipeline() {
            Ok(m) => m,
            Err(e) => {
                // Rollback on build failure
                if !dry_run && !backups.is_empty() {
                    let _ = self.rollback_cargo_tomls(&backups);
                }
                return Err(e);
            }
        };

        // Phase 3: Check CI/CD metrics
        let (budget_check_passed, budget_threshold) = if check_budget {
            self.check_budget(&metrics)?
        } else {
            (None, None)
        };

        Ok(BuildResult {
            cargo_changes,
            metrics,
            budget_check_passed,
            budget_threshold,
            dry_run,
            dry_run_files,
        })
    }

    /// Phase 1: Optimize Cargo.toml files with backup support
    fn optimize_cargo_tomls_with_backup(&self, dry_run: bool) -> Result<OptimizationResult> {
        let _config = config::ConfigLoader::load(&self.project_root)
            .unwrap_or_else(|_| config::ConfigFile::default());

        let mut changes = Vec::new();
        let mut dry_run_files = Vec::new();
        let mut backups = Vec::new();

        // Optimize Cargo.toml files
        let (cargo_changes, cargo_dry_run, cargo_backups) = self.optimize_cargo_files(dry_run)?;
        changes.extend(cargo_changes);
        dry_run_files.extend(cargo_dry_run);
        backups.extend(cargo_backups);

        // Apply build-std optimization if on nightly
        let (build_std_changes, build_std_dry_run) = self.optimize_build_std_if_needed(dry_run)?;
        changes.extend(build_std_changes);
        dry_run_files.extend(build_std_dry_run);

        Ok((changes, dry_run_files, backups))
    }

    /// Optimize all Cargo.toml files in the workspace
    fn optimize_cargo_files(&self, dry_run: bool) -> Result<OptimizationResult> {
        let cargo_finder = optimizer::CargoFileFinder::new(&self.project_root);
        let cargo_editor = optimizer::CargoTomlEditor::new();
        let opt_config = optimizer::OptimizationConfig::default();

        let mut changes = Vec::new();
        let mut dry_run_files = Vec::new();
        let mut backups = Vec::new();
        let cargo_tomls = cargo_finder.find_cargo_tomls()?;

        for toml_path in cargo_tomls {
            let relative = toml_path
                .strip_prefix(&self.project_root)
                .unwrap_or(&toml_path)
                .display()
                .to_string();

            if dry_run {
                dry_run_files.push(relative);
            } else {
                // Create backup before modification
                if let Ok(original_content) = std::fs::read_to_string(&toml_path) {
                    backups.push((toml_path.clone(), original_content));
                }

                match cargo_editor.optimize_cargo_toml(&toml_path, &opt_config, None, dry_run) {
                    Ok(change_list) => {
                        changes.extend(change_list);
                    }
                    Err(_) => {
                        // Skip files that fail to optimize
                        continue;
                    }
                }
            }
        }

        Ok((changes, dry_run_files, backups))
    }

    /// Apply build-std optimization if nightly toolchain is available
    fn optimize_build_std_if_needed(&self, dry_run: bool) -> Result<(Vec<String>, Vec<String>)> {
        let mut changes = Vec::new();
        let mut dry_run_files = Vec::new();

        // Check for nightly toolchain and build-std optimization
        if let Ok(true) = crate::toolchain::ToolchainDetector::new().is_nightly_toolchain() {
            let build_std_optimizer = optimizer::BuildStdOptimizer::new(&self.project_root);

            if !build_std_optimizer.is_configured().unwrap_or(false) {
                if dry_run {
                    dry_run_files.push("build-std configuration".to_string());
                } else {
                    let build_std_config = optimizer::BuildStdConfig::minimal();

                    if let Ok(build_std_changes) =
                        build_std_optimizer.apply_build_std(&build_std_config, dry_run)
                    {
                        changes.extend(build_std_changes);
                    }
                }
            }
        }

        Ok((changes, dry_run_files))
    }

    /// Rollback Cargo.toml files from backups
    fn rollback_cargo_tomls(&self, backups: &[(PathBuf, String)]) -> Result<()> {
        for (path, content) in backups {
            std::fs::write(path, content)?;
        }
        Ok(())
    }

    /// Phase 2: Run the build pipeline
    fn run_build_pipeline(&self) -> Result<pipeline::SizeMetrics> {
        let _config = config::ConfigLoader::load(&self.project_root)
            .unwrap_or_else(|_| config::ConfigFile::default());

        let pipeline_config = pipeline::PipelineConfig {
            opt_level: pipeline::WasmOptLevel::Oz,
            run_wasm_snip: true,
            ..Default::default()
        };

        let build_pipeline = pipeline::BuildPipeline::new(&self.project_root, pipeline_config);
        build_pipeline
            .build()
            .map_err(|e| anyhow::anyhow!("Build pipeline failed: {}", e))
    }

    /// Phase 3: Check CI/CD budget
    fn check_budget(&self, metrics: &pipeline::SizeMetrics) -> Result<(Option<bool>, Option<u64>)> {
        let config = config::ConfigLoader::load(&self.project_root)
            .unwrap_or_else(|_| config::ConfigFile::default());

        if let Some(budget) = &config.size_budget {
            if let Some(max_size_kb) = budget.max_size_kb {
                let max_size = max_size_kb * 1024;
                let passed = metrics.after_bytes <= max_size;

                if !passed {
                    anyhow::bail!(
                        "WASM bundle size ({} bytes) exceeds maximum ({} bytes)",
                        metrics.after_bytes,
                        max_size
                    );
                }

                return Ok((Some(passed), Some(max_size)));
            }
        }

        Ok((None, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_build_workflow_new_with_different_paths() {
        let paths = vec![
            Path::new("/tmp"),
            Path::new("/home/user/project"),
            Path::new("."),
            Path::new("./relative/path"),
        ];

        for path in paths {
            let workflow = BuildWorkflow::new(path);
            assert_eq!(workflow.project_root, PathBuf::from(path));
        }
    }

    #[test]
    fn test_build_workflow_stores_project_root() {
        let root = Path::new("/test/project");
        let workflow = BuildWorkflow::new(root);
        assert_eq!(workflow.project_root, root);
    }

    #[test]
    fn test_build_result_can_be_created() {
        use crate::pipeline::SizeMetrics;

        let result = BuildResult {
            cargo_changes: vec!["test".to_string()],
            metrics: SizeMetrics {
                before_bytes: 1000,
                after_bytes: 800,
            },
            budget_check_passed: None,
            budget_threshold: None,
            dry_run: false,
            dry_run_files: vec![],
        };
        assert_eq!(result.cargo_changes.len(), 1);
        assert_eq!(result.metrics.before_bytes, 1000);
        assert_eq!(result.metrics.after_bytes, 800);
    }

    #[test]
    fn test_build_result_with_budget_check() {
        use crate::pipeline::SizeMetrics;

        let result = BuildResult {
            cargo_changes: vec![],
            metrics: SizeMetrics {
                before_bytes: 2000,
                after_bytes: 1500,
            },
            budget_check_passed: Some(true),
            budget_threshold: Some(2000),
            dry_run: false,
            dry_run_files: vec![],
        };
        assert_eq!(result.budget_check_passed, Some(true));
        assert_eq!(result.budget_threshold, Some(2000));
    }

    #[test]
    fn test_build_result_with_failed_budget() {
        use crate::pipeline::SizeMetrics;

        let result = BuildResult {
            cargo_changes: vec![],
            metrics: SizeMetrics {
                before_bytes: 1000,
                after_bytes: 2500,
            },
            budget_check_passed: Some(false),
            budget_threshold: Some(2000),
            dry_run: false,
            dry_run_files: vec![],
        };
        assert_eq!(result.budget_check_passed, Some(false));
        assert!(result.metrics.after_bytes > result.budget_threshold.unwrap());
    }

    #[test]
    fn test_build_result_dry_run_mode() {
        use crate::pipeline::SizeMetrics;

        let result = BuildResult {
            cargo_changes: vec![],
            metrics: SizeMetrics {
                before_bytes: 1000,
                after_bytes: 1000,
            },
            budget_check_passed: None,
            budget_threshold: None,
            dry_run: true,
            dry_run_files: vec!["file1.toml".to_string(), "file2.toml".to_string()],
        };
        assert!(result.dry_run);
        assert_eq!(result.dry_run_files.len(), 2);
    }

    #[test]
    fn test_build_result_with_multiple_cargo_changes() {
        use crate::pipeline::SizeMetrics;

        let result = BuildResult {
            cargo_changes: vec![
                "change1".to_string(),
                "change2".to_string(),
                "change3".to_string(),
            ],
            metrics: SizeMetrics {
                before_bytes: 2000,
                after_bytes: 1000,
            },
            budget_check_passed: None,
            budget_threshold: None,
            dry_run: false,
            dry_run_files: vec![],
        };
        assert_eq!(result.cargo_changes.len(), 3);
        assert!(result.metrics.before_bytes > result.metrics.after_bytes);
    }

    #[test]
    fn test_optimization_result_empty_tuple() {
        let result: OptimizationResult = (vec![], vec![], vec![]);
        assert!(result.0.is_empty());
        assert!(result.1.is_empty());
        assert!(result.2.is_empty());
    }

    #[test]
    fn test_optimization_result_with_changes() {
        let result: OptimizationResult = (
            vec!["change1".to_string(), "change2".to_string()],
            vec![],
            vec![],
        );
        assert_eq!(result.0.len(), 2);
        assert!(result.1.is_empty());
        assert!(result.2.is_empty());
    }

    #[test]
    fn test_optimization_result_with_dry_run_files() {
        let result: OptimizationResult = (
            vec![],
            vec!["file1.toml".to_string(), "file2.toml".to_string()],
            vec![],
        );
        assert!(result.0.is_empty());
        assert_eq!(result.1.len(), 2);
        assert!(result.2.is_empty());
    }

    #[test]
    fn test_optimization_result_with_backups() {
        let result: OptimizationResult = (
            vec![],
            vec![],
            vec![
                (PathBuf::from("/path/backup1"), "content1".to_string()),
                (PathBuf::from("/path/backup2"), "content2".to_string()),
            ],
        );
        assert!(result.0.is_empty());
        assert!(result.1.is_empty());
        assert_eq!(result.2.len(), 2);
    }

    #[test]
    fn test_optimization_result_complete() {
        let result: OptimizationResult = (
            vec!["change".to_string()],
            vec!["file.toml".to_string()],
            vec![(PathBuf::from("/backup"), "content".to_string())],
        );
        assert_eq!(result.0.len(), 1);
        assert_eq!(result.1.len(), 1);
        assert_eq!(result.2.len(), 1);
    }

    #[test]
    fn test_build_workflow_can_be_created() {
        let workflow = BuildWorkflow::new(Path::new("/tmp"));
        assert_eq!(workflow.project_root, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_rollback_cargo_tomls_restores_content() {
        use std::fs;
        use tempfile::NamedTempFile;

        let workflow = BuildWorkflow::new(Path::new("/tmp"));

        // Create temporary files with original content
        let temp_file1 = NamedTempFile::new().unwrap();
        let temp_file2 = NamedTempFile::new().unwrap();

        let original_content1 = "original content 1";
        let original_content2 = "original content 2";

        fs::write(temp_file1.path(), "modified").unwrap();
        fs::write(temp_file2.path(), "modified").unwrap();

        let backups = vec![
            (
                temp_file1.path().to_path_buf(),
                original_content1.to_string(),
            ),
            (
                temp_file2.path().to_path_buf(),
                original_content2.to_string(),
            ),
        ];

        // Test rollback
        let result = workflow.rollback_cargo_tomls(&backups);
        assert!(result.is_ok());

        // Verify content restored
        assert_eq!(
            fs::read_to_string(temp_file1.path()).unwrap(),
            original_content1
        );
        assert_eq!(
            fs::read_to_string(temp_file2.path()).unwrap(),
            original_content2
        );
    }

    #[test]
    fn test_rollback_cargo_tomls_with_empty_backups() {
        let workflow = BuildWorkflow::new(Path::new("/tmp"));
        let backups = [];

        let result = workflow.rollback_cargo_tomls(&backups);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_result_debug_format() {
        use crate::pipeline::SizeMetrics;

        let result = BuildResult {
            cargo_changes: vec!["test".to_string()],
            metrics: SizeMetrics {
                before_bytes: 1000,
                after_bytes: 800,
            },
            budget_check_passed: None,
            budget_threshold: None,
            dry_run: false,
            dry_run_files: vec![],
        };

        // Verify Debug trait is implemented
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("BuildResult"));
    }

    #[test]
    fn test_build_result_all_fields() {
        use crate::pipeline::SizeMetrics;

        let result = BuildResult {
            cargo_changes: vec!["opt-level=z".to_string(), "lto=true".to_string()],
            metrics: SizeMetrics {
                before_bytes: 5000,
                after_bytes: 3000,
            },
            budget_check_passed: Some(true),
            budget_threshold: Some(4000),
            dry_run: true,
            dry_run_files: vec!["Cargo.toml".to_string()],
        };

        assert_eq!(result.cargo_changes.len(), 2);
        assert_eq!(result.metrics.before_bytes, 5000);
        assert_eq!(result.metrics.after_bytes, 3000);
        assert_eq!(result.budget_check_passed, Some(true));
        assert_eq!(result.budget_threshold, Some(4000));
        assert!(result.dry_run);
        assert_eq!(result.dry_run_files.len(), 1);
    }

    #[test]
    fn test_optimization_result_type_alias() {
        // Test that OptimizationResult type is correctly defined
        let _result: OptimizationResult = (
            vec!["change".to_string()],
            vec!["file.toml".to_string()],
            vec![(PathBuf::from("/backup"), "content".to_string())],
        );

        // Type checking passes means the alias is correct
    }
}
