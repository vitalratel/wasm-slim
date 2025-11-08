//! Custom allocator detection and optimization for WASM
//!
//! Analyzes dependency tree to recommend custom allocator usage when beneficial.
//!
//! # Overview
//!
//! WASM bundles using default system allocator carry overhead. Custom allocators
//! like `wee_alloc` trade allocation speed for size (2-5% reduction).
//!
//! This module:
//! - Detects existing custom allocators (wee_alloc, lol_alloc, dlmalloc)
//! - Counts allocation-heavy dependencies
//! - Recommends allocator when ≥5 allocation-heavy deps present
//!
//! # Examples
//!
//! ```no_run
//! use wasm_slim::analyzer::allocator::AllocatorDetector;
//! use std::path::Path;
//!
//! let detector = AllocatorDetector::new(Path::new("."));
//! if let Ok(Some(recommendation)) = detector.check_allocator_optimization() {
//!     println!("Suggestion: {}", recommendation.suggestion);
//! }
//! ```

use cargo_metadata::MetadataCommand;
use std::path::Path;

use super::deps::{DependencyAnalysisError, DependencyIssue, IssueSeverity};

/// Detects allocator optimization opportunities
pub struct AllocatorDetector {
    project_root: std::path::PathBuf,
}

impl AllocatorDetector {
    /// Create a new allocator detector for the given project
    ///
    /// # Arguments
    ///
    /// * `project_root` - Path to the project root containing Cargo.toml
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
        }
    }

    /// Check if custom allocator optimization would benefit this project
    ///
    /// Returns `Some(DependencyIssue)` if:
    /// - No custom allocator is currently used
    /// - Project has ≥5 allocation-heavy dependencies
    ///
    /// Returns `None` if:
    /// - Custom allocator already present (wee_alloc, lol_alloc, dlmalloc)
    /// - Too few allocation-heavy dependencies (<5)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use wasm_slim::analyzer::allocator::AllocatorDetector;
    /// # use std::path::Path;
    /// let detector = AllocatorDetector::new(Path::new("."));
    /// match detector.check_allocator_optimization() {
    ///     Ok(Some(issue)) => println!("Recommendation: {}", issue.suggestion),
    ///     Ok(None) => println!("Allocator already optimized"),
    ///     Err(e) => eprintln!("Analysis failed: {}", e),
    /// }
    /// ```
    pub fn check_allocator_optimization(
        &self,
    ) -> Result<Option<DependencyIssue>, DependencyAnalysisError> {
        let metadata = MetadataCommand::new()
            .current_dir(&self.project_root)
            .exec()?;

        self.check_allocator_optimization_with_metadata(&metadata)
    }

    /// Check allocator optimization with provided metadata (for testing)
    ///
    /// # Arguments
    ///
    /// * `metadata` - Cargo metadata to analyze
    ///
    /// # Returns
    ///
    /// `Some(DependencyIssue)` if custom allocator would benefit the project, `None` otherwise
    pub(crate) fn check_allocator_optimization_with_metadata(
        &self,
        metadata: &cargo_metadata::Metadata,
    ) -> Result<Option<DependencyIssue>, DependencyAnalysisError> {
        // Check if custom allocator already present
        let has_custom_allocator = metadata
            .packages
            .iter()
            .any(|p| matches!(p.name.as_str(), "wee_alloc" | "lol_alloc" | "dlmalloc"));

        if has_custom_allocator {
            return Ok(None); // Already optimized
        }

        // Count allocation-heavy dependencies
        let heavy_alloc_deps = self.count_allocation_heavy_deps(metadata);

        // Recommend if >= 5 allocation-heavy deps (heuristic threshold)
        if heavy_alloc_deps >= 5 {
            let estimated_savings_kb = self.estimate_allocator_savings(heavy_alloc_deps);

            Ok(Some(DependencyIssue {
                package: "wasm-allocator".to_string(),
                version: "n/a".to_string(),
                severity: IssueSeverity::Medium,
                issue: format!(
                    "No custom allocator detected. Project has {} allocation-heavy dependencies using default system allocator.",
                    heavy_alloc_deps
                ),
                suggestion: "Add wee_alloc for 2-5% size reduction:\n\n   [dependencies]\n   wee_alloc = \"0.4.5\"\n\n   And configure global allocator in lib.rs:\n   #[cfg(target_arch = \"wasm32\")]\n   #[global_allocator]\n   static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;\n\n   Note: wee_alloc trades smaller size for slightly slower allocation performance.".to_string(),
                size_impact_kb: Some(estimated_savings_kb),
                savings_percent: Some(3), // Conservative 3% estimate
            }))
        } else {
            Ok(None) // Not enough heap usage to warrant custom allocator
        }
    }

    /// Count dependencies that do heavy heap allocation
    ///
    /// Uses heuristics to identify crates likely to allocate frequently:
    /// - String/text processing (regex, serde_json, toml, etc.)
    /// - Collections (vec-heavy, tree structures)
    /// - Parsing/compiling (parsers, compilers)
    /// - Data structures (btree, linked lists, etc.)
    fn count_allocation_heavy_deps(&self, metadata: &cargo_metadata::Metadata) -> usize {
        metadata
            .packages
            .iter()
            .filter(|p| Self::is_allocation_heavy(&p.name))
            .count()
    }

    /// Check if a crate is known to do heavy heap allocation
    ///
    /// This is a heuristic based on common patterns:
    /// - String manipulation crates
    /// - JSON/serialization
    /// - Parsers and compilers
    /// - Collection-heavy libraries
    pub(crate) fn is_allocation_heavy(crate_name: &str) -> bool {
        matches!(
            crate_name,
            // String/text processing
            "regex" | "regex-syntax" | "fancy-regex" |
            // Serialization (creates many temporary objects)
            "serde_json" | "serde_yaml" | "toml" | "toml_edit" | "ron" |
            // Parsers (build ASTs in memory)
            "syn" | "swc_core" | "swc_ecma_parser" | "nom" | "pest" | "lalrpop-util" |
            // HTML/XML
            "html5ever" | "xml-rs" | "quick-xml" |
            // Data structures
            "petgraph" | "indexmap" | "linked-hash-map" |
            // String formatting
            "format_bytes" | "indoc" |
            // Web frameworks (session management, request buffers)
            "actix-web" | "axum" | "warp" | "rocket" |
            // Database (query builders, ORMs)
            "sqlx" | "diesel" | "sea-orm" |
            // Crypto (work buffers)
            "openssl" | "rustls" | "ring" |
            // Compression (buffers)
            "flate2" | "brotli" | "zstd" |
            // Template engines
            "tera" | "handlebars" | "askama" |
            // CLI (parsing, help text)
            "clap" | "structopt"
        )
    }

    /// Estimate allocator savings based on allocation-heavy dependency count
    ///
    /// Returns (min_kb, max_kb) tuple
    ///
    /// Heuristic formula:
    /// - Base savings: 10-20KB (allocator overhead)
    /// - Per-dependency factor: 2-4KB per allocation-heavy dep
    /// - Capped at reasonable maximum (100KB)
    fn estimate_allocator_savings(&self, heavy_dep_count: usize) -> (u32, u32) {
        let base_min = 10;
        let base_max = 20;
        let per_dep_min = 2;
        let per_dep_max = 4;

        let min_kb = (base_min + (heavy_dep_count as u32 * per_dep_min)).min(50);
        let max_kb = (base_max + (heavy_dep_count as u32 * per_dep_max)).min(100);

        (min_kb, max_kb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_metadata::Metadata;

    #[test]
    fn test_is_allocation_heavy_detects_known_crates() {
        assert!(AllocatorDetector::is_allocation_heavy("regex"));
        assert!(AllocatorDetector::is_allocation_heavy("serde_json"));
        assert!(AllocatorDetector::is_allocation_heavy("syn"));
        assert!(AllocatorDetector::is_allocation_heavy("clap"));
        assert!(!AllocatorDetector::is_allocation_heavy("unknown-crate"));
    }

    #[test]
    fn test_estimate_allocator_savings_scales_with_deps() {
        let detector = AllocatorDetector::new(".");

        // Few deps - minimal savings
        let (min_5, max_5) = detector.estimate_allocator_savings(5);
        assert!(min_5 >= 10);
        assert!(max_5 <= 40);
        assert!(min_5 < max_5);

        // Many deps - more savings
        let (min_20, max_20) = detector.estimate_allocator_savings(20);
        assert!(min_20 > min_5);
        assert!(max_20 > max_5);

        // Cap at reasonable max
        let (min_100, max_100) = detector.estimate_allocator_savings(100);
        assert!(min_100 <= 50);
        assert!(max_100 <= 100);
    }

    #[test]
    fn test_check_allocator_optimization_with_custom_allocator_returns_none() {
        // Create mock metadata with wee_alloc present
        let json = r#"{
            "packages": [
                {
                    "name": "wee_alloc",
                    "version": "0.4.5",
                    "id": "wee_alloc 0.4.5",
                    "license": "MIT",
                    "license_file": null,
                    "description": "Small allocator",
                    "source": null,
                    "dependencies": [],
                    "targets": [],
                    "features": {},
                    "manifest_path": "/fake/Cargo.toml",
                    "metadata": null,
                    "publish": null,
                    "authors": [],
                    "categories": [],
                    "keywords": [],
                    "readme": null,
                    "repository": null,
                    "homepage": null,
                    "documentation": null,
                    "edition": "2021",
                    "links": null,
                    "default_run": null,
                    "rust_version": null
                },
                {
                    "name": "regex",
                    "version": "1.0.0",
                    "id": "regex 1.0.0",
                    "license": "MIT",
                    "license_file": null,
                    "description": "Regex",
                    "source": null,
                    "dependencies": [],
                    "targets": [],
                    "features": {},
                    "manifest_path": "/fake/Cargo.toml",
                    "metadata": null,
                    "publish": null,
                    "authors": [],
                    "categories": [],
                    "keywords": [],
                    "readme": null,
                    "repository": null,
                    "homepage": null,
                    "documentation": null,
                    "edition": "2021",
                    "links": null,
                    "default_run": null,
                    "rust_version": null
                }
            ],
            "workspace_members": [],
            "workspace_default_members": [],
            "resolve": null,
            "target_directory": "/fake/target",
            "version": 1,
            "workspace_root": "/fake",
            "metadata": null
        }"#;

        let metadata: Metadata = serde_json::from_str(json).unwrap();
        let detector = AllocatorDetector::new(".");

        // Should return None since wee_alloc is present
        let result = detector
            .check_allocator_optimization_with_metadata(&metadata)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_check_allocator_optimization_with_few_deps_returns_none() {
        // Create mock metadata with only 2 allocation-heavy deps (below threshold)
        let json = r#"{
            "packages": [
                {
                    "name": "regex",
                    "version": "1.0.0",
                    "id": "regex 1.0.0",
                    "license": "MIT",
                    "license_file": null,
                    "description": "Regex",
                    "source": null,
                    "dependencies": [],
                    "targets": [],
                    "features": {},
                    "manifest_path": "/fake/Cargo.toml",
                    "metadata": null,
                    "publish": null,
                    "authors": [],
                    "categories": [],
                    "keywords": [],
                    "readme": null,
                    "repository": null,
                    "homepage": null,
                    "documentation": null,
                    "edition": "2021",
                    "links": null,
                    "default_run": null,
                    "rust_version": null
                },
                {
                    "name": "serde_json",
                    "version": "1.0.0",
                    "id": "serde_json 1.0.0",
                    "license": "MIT",
                    "license_file": null,
                    "description": "JSON",
                    "source": null,
                    "dependencies": [],
                    "targets": [],
                    "features": {},
                    "manifest_path": "/fake/Cargo.toml",
                    "metadata": null,
                    "publish": null,
                    "authors": [],
                    "categories": [],
                    "keywords": [],
                    "readme": null,
                    "repository": null,
                    "homepage": null,
                    "documentation": null,
                    "edition": "2021",
                    "links": null,
                    "default_run": null,
                    "rust_version": null
                }
            ],
            "workspace_members": [],
            "workspace_default_members": [],
            "resolve": null,
            "target_directory": "/fake/target",
            "version": 1,
            "workspace_root": "/fake",
            "metadata": null
        }"#;

        let metadata: Metadata = serde_json::from_str(json).unwrap();
        let detector = AllocatorDetector::new(".");

        // Should return None since < 5 allocation-heavy deps
        let result = detector
            .check_allocator_optimization_with_metadata(&metadata)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_check_allocator_optimization_with_many_deps_returns_issue() {
        // Create mock metadata with 6 allocation-heavy deps (above threshold)
        let json = r#"{
            "packages": [
                {"name": "regex", "version": "1.0.0", "id": "regex 1.0.0", "license": "MIT", "license_file": null, "description": "Regex", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null},
                {"name": "serde_json", "version": "1.0.0", "id": "serde_json 1.0.0", "license": "MIT", "license_file": null, "description": "JSON", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null},
                {"name": "toml", "version": "1.0.0", "id": "toml 1.0.0", "license": "MIT", "license_file": null, "description": "TOML", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null},
                {"name": "syn", "version": "1.0.0", "id": "syn 1.0.0", "license": "MIT", "license_file": null, "description": "Parser", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null},
                {"name": "clap", "version": "4.0.0", "id": "clap 4.0.0", "license": "MIT", "license_file": null, "description": "CLI", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null},
                {"name": "html5ever", "version": "1.0.0", "id": "html5ever 1.0.0", "license": "MIT", "license_file": null, "description": "HTML parser", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null}
            ],
            "workspace_members": [],
            "workspace_default_members": [],
            "resolve": null,
            "target_directory": "/fake/target",
            "version": 1,
            "workspace_root": "/fake",
            "metadata": null
        }"#;

        let metadata: Metadata = serde_json::from_str(json).unwrap();
        let detector = AllocatorDetector::new(".");

        // Should return Some since >= 5 allocation-heavy deps and no custom allocator
        let result = detector
            .check_allocator_optimization_with_metadata(&metadata)
            .unwrap();
        assert!(result.is_some());

        let issue = result.unwrap();
        assert_eq!(issue.package, "wasm-allocator");
        assert_eq!(issue.severity, IssueSeverity::Medium);
        assert!(issue.suggestion.contains("wee_alloc"));
        assert!(issue.size_impact_kb.is_some());
        assert_eq!(issue.savings_percent, Some(3));
    }

    #[test]
    fn test_allocator_detection_recognizes_all_variants() {
        // Test with lol_alloc (alternative allocator)
        let json_lol = r#"{
            "packages": [
                {
                    "name": "lol_alloc",
                    "version": "0.4.0",
                    "id": "lol_alloc 0.4.0",
                    "license": "MIT",
                    "license_file": null,
                    "description": "Allocator",
                    "source": null,
                    "dependencies": [],
                    "targets": [],
                    "features": {},
                    "manifest_path": "/fake/Cargo.toml",
                    "metadata": null,
                    "publish": null,
                    "authors": [],
                    "categories": [],
                    "keywords": [],
                    "readme": null,
                    "repository": null,
                    "homepage": null,
                    "documentation": null,
                    "edition": "2021",
                    "links": null,
                    "default_run": null,
                    "rust_version": null
                }
            ],
            "workspace_members": [],
            "workspace_default_members": [],
            "resolve": null,
            "target_directory": "/fake/target",
            "version": 1,
            "workspace_root": "/fake",
            "metadata": null
        }"#;

        let metadata: Metadata = serde_json::from_str(json_lol).unwrap();
        let detector = AllocatorDetector::new(".");

        // Should return None - lol_alloc is recognized as custom allocator
        assert!(detector
            .check_allocator_optimization_with_metadata(&metadata)
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_count_allocation_heavy_deps_accuracy() {
        // Mix of heavy and normal deps
        let json = r#"{
            "packages": [
                {"name": "regex", "version": "1.0.0", "id": "regex 1.0.0", "license": "MIT", "license_file": null, "description": "Regex", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null},
                {"name": "serde", "version": "1.0.0", "id": "serde 1.0.0", "license": "MIT", "license_file": null, "description": "Serde", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null},
                {"name": "serde_json", "version": "1.0.0", "id": "serde_json 1.0.0", "license": "MIT", "license_file": null, "description": "JSON", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null},
                {"name": "anyhow", "version": "1.0.0", "id": "anyhow 1.0.0", "license": "MIT", "license_file": null, "description": "Error", "source": null, "dependencies": [], "targets": [], "features": {}, "manifest_path": "/fake/Cargo.toml", "metadata": null, "publish": null, "authors": [], "categories": [], "keywords": [], "readme": null, "repository": null, "homepage": null, "documentation": null, "edition": "2021", "links": null, "default_run": null, "rust_version": null}
            ],
            "workspace_members": [],
            "workspace_default_members": [],
            "resolve": null,
            "target_directory": "/fake/target",
            "version": 1,
            "workspace_root": "/fake",
            "metadata": null
        }"#;

        let metadata: Metadata = serde_json::from_str(json).unwrap();
        let detector = AllocatorDetector::new(".");

        let count = detector.count_allocation_heavy_deps(&metadata);
        assert_eq!(count, 2); // regex and serde_json
    }
}
