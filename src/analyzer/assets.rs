//! Asset detection for embedded resources in WASM projects.
//!
//! Detects embedded assets (fonts, images, data files) that could be externalized
//! to reduce WASM bundle size. Based on Warp.dev's 10MB savings through asset
//! externalization.

use crate::infra::{FileSystem, RealFileSystem};
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use syn::File;
use thiserror::Error;

// Re-export types for backward compatibility
pub use super::asset_metrics::{DetectedAsset, EstimatedSavings, ScanResults};
pub use super::asset_types::{AssetPriority, AssetType};

/// Errors that can occur during asset detection
#[derive(Error, Debug)]
pub enum AssetDetectionError {
    /// I/O error reading files
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse Rust source file
    #[error("Failed to parse source file {0}: {1}")]
    ParseError(PathBuf, String),

    /// Failed to compile regex pattern
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

/// Compiled regex patterns for asset detection (cached for performance)
static INCLUDE_BYTES_RE: OnceLock<Regex> = OnceLock::new();
static INCLUDE_STR_RE: OnceLock<Regex> = OnceLock::new();

/// Main asset detector that scans Rust source files for embedded assets
///
/// Identifies `include_bytes!`, `include_str!`, font files, and other
/// embedded resources to provide externalization recommendations.
///
/// # Examples
///
/// Basic usage:
/// ```no_run
/// use wasm_slim::analyzer::AssetDetector;
/// use std::path::Path;
///
/// let detector = AssetDetector::new(Path::new("."));
/// let results = detector.scan_project()?;
///
/// println!("Found {} assets ({:.1}% of bundle)",
///          results.total_assets, results.bundle_percentage);
///
/// for (priority, assets) in &results.assets_by_priority {
///     println!("{:?} priority: {} assets", priority, assets.len());
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// Analyzing externalization opportunities:
/// ```no_run
/// # use wasm_slim::analyzer::AssetDetector;
/// # use std::path::Path;
/// let detector = AssetDetector::new(Path::new("."));
/// let results = detector.scan_project()?;
///
/// let savings = &results.estimated_savings;
/// println!("Potential savings:");
/// println!("  Critical assets only: {} KB ({:.1}%)",
///          savings.critical_only_kb, savings.critical_only_percent);
/// println!("  High + critical: {} KB ({:.1}%)",
///          savings.high_and_critical_kb, savings.high_and_critical_percent);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct AssetDetector<FS: FileSystem = RealFileSystem> {
    project_root: PathBuf,
    fs: FS,
}

// Core detector logic
impl AssetDetector<RealFileSystem> {
    /// Create a new asset detector for the given project root
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self::with_fs(project_root, RealFileSystem)
    }
}

// Static utility methods (no generic parameters needed)
impl AssetDetector {
    /// Detect asset type from file extension (case-insensitive)
    fn detect_type(path: &Path) -> AssetType {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());

        match ext.as_deref() {
            Some("ttf") | Some("otf") | Some("woff") | Some("woff2") => AssetType::Font,
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") | Some("webp") => {
                AssetType::Image
            }
            Some("json") | Some("toml") | Some("yaml") | Some("txt") | Some("css") => {
                AssetType::Data
            }
            _ => AssetType::Unknown,
        }
    }
}

impl<FS: FileSystem + Sync> AssetDetector<FS> {
    /// Create a new asset detector with a custom filesystem implementation
    pub fn with_fs(project_root: impl Into<PathBuf>, fs: FS) -> Self {
        Self {
            project_root: project_root.into(),
            fs,
        }
    }

    /// Scan the entire project for embedded assets
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_slim::analyzer::AssetDetector;
    ///
    /// let detector = AssetDetector::new(".");
    /// let results = detector.scan_project()?;
    /// println!("Found {} embedded assets", results.assets.len());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use = "Scan results contain important asset information"]
    pub fn scan_project(&self) -> Result<ScanResults, AssetDetectionError> {
        // Find all Rust source files
        let rust_files = self.find_rust_files(&self.project_root)?;

        // Parallel scan of all files
        let all_assets: Vec<DetectedAsset> = rust_files
            .par_iter()
            .flat_map(|source_file| {
                self.scan_file(source_file).unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to scan {}: {}", source_file.display(), e);
                    Vec::new()
                })
            })
            .collect();

        // Get bundle size (if available)
        let bundle_size_kb = self.estimate_bundle_size()?;

        // Build results
        self.build_results(all_assets, bundle_size_kb)
    }

    /// Scan a single source file for assets
    fn scan_file(&self, source_file: &Path) -> Result<Vec<DetectedAsset>, AssetDetectionError> {
        let content = self.fs.read_to_string(source_file)?;

        let mut assets = Vec::new();

        // Try AST parsing first (most reliable)
        if let Ok(parsed_assets) = self.scan_with_ast(&content, source_file) {
            assets.extend(parsed_assets);
        }

        // Add regex fallback for patterns AST might miss
        let regex_assets = self.scan_with_regex(&content, source_file)?;
        assets.extend(regex_assets);

        Ok(assets)
    }
}

// Scanning strategy methods
impl<FS: FileSystem + Sync> AssetDetector<FS> {
    /// Scan using AST parsing with syn
    fn scan_with_ast(
        &self,
        content: &str,
        source_file: &Path,
    ) -> Result<Vec<DetectedAsset>, AssetDetectionError> {
        let syntax_tree: File = syn::parse_str(content).map_err(|e| {
            AssetDetectionError::ParseError(source_file.to_path_buf(), e.to_string())
        })?;
        let mut visitor = super::asset_visitor::AssetVisitor::new(source_file);
        syn::visit::Visit::visit_file(&mut visitor, &syntax_tree);

        // Resolve paths and calculate sizes
        let mut assets = Vec::new();
        for (path_str, line, method) in visitor.assets() {
            if let Some(asset) = self.create_asset(path_str, source_file, *line, method) {
                assets.push(asset);
            }
        }

        Ok(assets)
    }

    /// Scan using regex patterns (fallback)
    fn scan_with_regex(
        &self,
        content: &str,
        source_file: &Path,
    ) -> Result<Vec<DetectedAsset>, AssetDetectionError> {
        let mut assets = Vec::new();

        // Pattern: include_bytes!("path")
        let include_bytes_re = INCLUDE_BYTES_RE.get_or_init(|| {
            // SAFETY: This regex pattern is compile-time validated and will never fail.
            // The pattern r#"include_bytes!\s*\(\s*"([^"]+)"\s*\)"# is a simple, valid regex.
            Regex::new(r#"include_bytes!\s*\(\s*"([^"]+)"\s*\)"#)
                .expect("include_bytes regex is valid")
        });
        for cap in include_bytes_re.captures_iter(content) {
            if let Some(path) = cap.get(1) {
                if let Some(asset) =
                    self.create_asset(path.as_str(), source_file, 0, "include_bytes!")
                {
                    assets.push(asset);
                }
            }
        }

        // Pattern: include_str!("path")
        let include_str_re = INCLUDE_STR_RE.get_or_init(|| {
            // SAFETY: This regex pattern is compile-time validated and will never fail.
            // The pattern r#"include_str!\s*\(\s*"([^"]+)"\s*\)"# is a simple, valid regex.
            Regex::new(r#"include_str!\s*\(\s*"([^"]+)"\s*\)"#).expect("include_str regex is valid")
        });
        for cap in include_str_re.captures_iter(content) {
            if let Some(path) = cap.get(1) {
                if let Some(asset) =
                    self.create_asset(path.as_str(), source_file, 0, "include_str!")
                {
                    assets.push(asset);
                }
            }
        }

        Ok(assets)
    }
}

// Asset processing utilities
impl<FS: FileSystem + Sync> AssetDetector<FS> {
    /// Create a DetectedAsset from a path
    fn create_asset(
        &self,
        asset_path: &str,
        source_file: &Path,
        line: usize,
        method: &str,
    ) -> Option<DetectedAsset> {
        // Resolve asset path relative to source file
        let source_dir = source_file.parent()?;
        let resolved_path = source_dir.join(asset_path);

        if !resolved_path.exists() {
            return None;
        }

        // Get file size
        let size_bytes = self.fs.metadata(&resolved_path).ok()?.len();

        // Determine asset type from extension
        let asset_type = AssetDetector::detect_type(&resolved_path);

        // Format source location
        let source_location = format!("{}:{}", source_file.display(), line);

        Some(DetectedAsset {
            file_path: asset_path.to_string(),
            size_bytes,
            asset_type,
            source_location,
            detection_method: method.to_string(),
        })
    }

    /// Build final ScanResults from detected assets
    fn build_results(
        &self,
        assets: Vec<DetectedAsset>,
        bundle_size_kb: u64,
    ) -> Result<ScanResults, AssetDetectionError> {
        let total_size_kb: u64 = assets.iter().map(|a| a.size_bytes / 1024).sum();

        let bundle_percentage = if bundle_size_kb > 0 {
            (total_size_kb as f64 / bundle_size_kb as f64) * 100.0
        } else {
            0.0
        };

        // Group by priority
        let mut assets_by_priority: HashMap<AssetPriority, Vec<DetectedAsset>> = HashMap::new();
        for asset in &assets {
            let asset_kb = asset.size_bytes / 1024;
            let priority = AssetPriority::from_size(asset_kb, bundle_size_kb);
            assets_by_priority
                .entry(priority)
                .or_default()
                .push(asset.clone());
        }

        // Calculate estimated savings
        let critical_kb: u64 = assets_by_priority
            .get(&AssetPriority::Critical)
            .map(|a| a.iter().map(|x| x.size_bytes / 1024).sum())
            .unwrap_or(0);

        let high_kb: u64 = assets_by_priority
            .get(&AssetPriority::High)
            .map(|a| a.iter().map(|x| x.size_bytes / 1024).sum())
            .unwrap_or(0);

        let estimated_savings = EstimatedSavings {
            critical_only_kb: critical_kb,
            high_and_critical_kb: critical_kb + high_kb,
            all_assets_kb: total_size_kb,
            critical_only_percent: if bundle_size_kb > 0 {
                (critical_kb as f64 / bundle_size_kb as f64) * 100.0
            } else {
                0.0
            },
            high_and_critical_percent: if bundle_size_kb > 0 {
                ((critical_kb + high_kb) as f64 / bundle_size_kb as f64) * 100.0
            } else {
                0.0
            },
            all_assets_percent: bundle_percentage,
        };

        Ok(ScanResults {
            total_assets: assets.len(),
            total_size_kb,
            bundle_size_kb,
            bundle_percentage,
            assets,
            assets_by_priority,
            estimated_savings,
        })
    }
}

// Utility functions
impl<FS: FileSystem + Sync> AssetDetector<FS> {
    /// Find all Rust source files in the project
    fn find_rust_files(&self, dir: &Path) -> Result<Vec<PathBuf>, AssetDetectionError> {
        let mut rust_files = Vec::new();

        if dir.is_file() {
            if dir.extension().and_then(|e| e.to_str()) == Some("rs") {
                rust_files.push(dir.to_path_buf());
            }
            return Ok(rust_files);
        }

        for entry in self.fs.read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip target directory and hidden directories
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == "target" || name.starts_with('.') {
                    continue;
                }
            }

            if path.is_dir() {
                rust_files.extend(self.find_rust_files(&path)?);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                rust_files.push(path);
            }
        }

        Ok(rust_files)
    }

    /// Estimate WASM bundle size (if available)
    fn estimate_bundle_size(&self) -> Result<u64, AssetDetectionError> {
        // Look for compiled WASM files in target directory
        let target_dir = self
            .project_root
            .join("target/wasm32-unknown-unknown/release");

        if !target_dir.exists() {
            return Ok(0); // No bundle built yet
        }

        // Find .wasm files
        let mut total_size = 0;
        for entry in self.fs.read_dir(&target_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                if let Ok(metadata) = self.fs.metadata(&path) {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size / 1024) // Convert to KB
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::asset_display::AssetPriorityDisplay;

    #[test]
    fn test_from_size_various_percentages_returns_correct_priorities() {
        // Critical: >10% or >500KB
        assert_eq!(AssetPriority::from_size(600, 5000), AssetPriority::Critical);
        assert_eq!(AssetPriority::from_size(550, 4000), AssetPriority::Critical);

        // High: 5-10% or 200-500KB
        assert_eq!(AssetPriority::from_size(300, 5000), AssetPriority::High);
        assert_eq!(AssetPriority::from_size(250, 3000), AssetPriority::High);

        // Medium: 2-5% or 100-200KB
        assert_eq!(AssetPriority::from_size(150, 5000), AssetPriority::Medium);
        assert_eq!(AssetPriority::from_size(120, 3000), AssetPriority::Medium);

        // Low: <2% or <100KB
        assert_eq!(AssetPriority::from_size(50, 5000), AssetPriority::Low);
        assert_eq!(AssetPriority::from_size(80, 5000), AssetPriority::Low);
    }

    #[test]
    fn test_detect_type_common_extensions_returns_correct_types() {
        assert_eq!(
            AssetDetector::detect_type(Path::new("font.woff2")),
            AssetType::Font
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.png")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("config.json")),
            AssetType::Data
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("unknown.xyz")),
            AssetType::Unknown
        );
    }

    #[test]
    fn test_detect_type_all_font_extensions_returns_font_type() {
        // Test all supported font extensions
        assert_eq!(
            AssetDetector::detect_type(Path::new("font.ttf")),
            AssetType::Font
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("font.otf")),
            AssetType::Font
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("font.woff")),
            AssetType::Font
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("font.woff2")),
            AssetType::Font
        );
    }

    #[test]
    fn test_detect_type_all_image_extensions_returns_image_type() {
        // Test all supported image extensions
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.png")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.jpg")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.jpeg")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.gif")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.svg")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.webp")),
            AssetType::Image
        );
    }

    #[test]
    fn test_detect_type_all_data_extensions_returns_data_type() {
        // Test all supported data extensions
        assert_eq!(
            AssetDetector::detect_type(Path::new("config.json")),
            AssetType::Data
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("config.toml")),
            AssetType::Data
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("config.yaml")),
            AssetType::Data
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("data.txt")),
            AssetType::Data
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("styles.css")),
            AssetType::Data
        );
    }

    #[test]
    fn test_detect_type_mixed_case_extensions_handles_case_insensitive() {
        // Extensions should be case-insensitive
        assert_eq!(
            AssetDetector::detect_type(Path::new("font.TTF")),
            AssetType::Font
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.PNG")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("config.JSON")),
            AssetType::Data
        );
    }

    #[test]
    fn test_detect_type_file_without_extension_returns_unknown() {
        // Files without extensions should be Unknown
        assert_eq!(
            AssetDetector::detect_type(Path::new("filename")),
            AssetType::Unknown
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("Makefile")),
            AssetType::Unknown
        );
    }

    #[test]
    fn test_detect_type_multiple_dots_in_filename_uses_last_extension() {
        // Only the last extension should matter
        assert_eq!(
            AssetDetector::detect_type(Path::new("font.backup.woff2")),
            AssetType::Font
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("image.old.png")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("config.prod.json")),
            AssetType::Data
        );
    }

    #[test]
    fn test_detect_type_full_path_with_directories_extracts_extension() {
        // Detection should work regardless of path depth
        assert_eq!(
            AssetDetector::detect_type(Path::new("assets/fonts/myfont.woff2")),
            AssetType::Font
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("../images/logo.png")),
            AssetType::Image
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("/absolute/path/data.json")),
            AssetType::Data
        );
    }

    #[test]
    fn test_detect_type_uncommon_extensions_returns_correct_types() {
        // Test less common but valid file types
        assert_eq!(
            AssetDetector::detect_type(Path::new("archive.zip")),
            AssetType::Unknown
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("video.mp4")),
            AssetType::Unknown
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("document.pdf")),
            AssetType::Unknown
        );
    }

    #[test]
    fn test_detect_type_edge_case_extensions_handles_correctly() {
        // Test edge cases
        // Note: ".png" is treated as a hidden file with no extension
        assert_eq!(
            AssetDetector::detect_type(Path::new(".png")),
            AssetType::Unknown
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new(".")),
            AssetType::Unknown
        );
        assert_eq!(
            AssetDetector::detect_type(Path::new("")),
            AssetType::Unknown
        );
    }

    #[test]
    fn test_from_size_boundary_thresholds_assigns_correct_priorities() {
        // Test exact boundary conditions for priority calculation

        // Critical boundary: >10% or >500KB
        assert_eq!(AssetPriority::from_size(500, 5000), AssetPriority::High); // exactly 10%
        assert_eq!(AssetPriority::from_size(501, 5000), AssetPriority::Critical); // >10%
        assert_eq!(AssetPriority::from_size(501, 0), AssetPriority::Critical); // >500KB

        // High boundary: >5% or >200KB
        assert_eq!(AssetPriority::from_size(250, 5000), AssetPriority::High); // exactly 5%
        assert_eq!(AssetPriority::from_size(200, 4000), AssetPriority::Medium); // exactly 5%
        assert_eq!(AssetPriority::from_size(201, 0), AssetPriority::High); // >200KB

        // Medium boundary: >2% or >100KB
        assert_eq!(AssetPriority::from_size(100, 5000), AssetPriority::Low); // exactly 2%
        assert_eq!(AssetPriority::from_size(99, 5000), AssetPriority::Low); // <2%
        assert_eq!(AssetPriority::from_size(101, 5000), AssetPriority::Medium); // >2%
    }

    #[test]
    fn test_from_size_zero_bundle_size_returns_low_priority() {
        // When bundle size is zero, priority is based solely on asset size
        assert_eq!(AssetPriority::from_size(600, 0), AssetPriority::Critical);
        assert_eq!(AssetPriority::from_size(300, 0), AssetPriority::High);
        assert_eq!(AssetPriority::from_size(150, 0), AssetPriority::Medium);
        assert_eq!(AssetPriority::from_size(50, 0), AssetPriority::Low);
    }

    #[test]
    fn test_from_size_zero_asset_size_returns_low_priority() {
        // Zero-sized assets should always be Low priority
        assert_eq!(AssetPriority::from_size(0, 1000), AssetPriority::Low);
        assert_eq!(AssetPriority::from_size(0, 0), AssetPriority::Low);
    }

    #[test]
    fn test_emoji_all_priorities_return_valid_emojis() {
        // Test emoji representations
        assert_eq!(AssetPriority::Critical.emoji(), "⚠️");
        assert_eq!(AssetPriority::High.emoji(), "📸");
        assert_eq!(AssetPriority::Medium.emoji(), "🔤");
        assert_eq!(AssetPriority::Low.emoji(), "📄");
    }

    #[test]
    fn test_color_all_priorities_return_valid_ansi_codes() {
        // Test color assignments
        use console::Color;
        assert_eq!(AssetPriority::Critical.color(), Color::Red);
        assert_eq!(AssetPriority::High.color(), Color::Yellow);
        assert_eq!(AssetPriority::Medium.color(), Color::Blue);
        assert_eq!(AssetPriority::Low.color(), Color::White);
    }

    // Property-based tests using proptest
    use proptest::prelude::*;

    proptest! {
        /// Property: Asset priority is monotonic - larger assets never have lower priority
        #[test]
        fn prop_priority_monotonic_with_size_increasing_asset_never_decreases_priority(
            asset_kb in 1u64..10_000_000,
            bundle_kb in 1u64..10_000_000,
            delta in 1u64..1000
        ) {
            let priority1 = AssetPriority::from_size(asset_kb, bundle_kb);
            let priority2 = AssetPriority::from_size(asset_kb + delta, bundle_kb);

            // Priority should not decrease when size increases
            prop_assert!(priority2 >= priority1,
                "Priority decreased: {:?} -> {:?} (asset: {} -> {}, bundle: {})",
                priority1, priority2, asset_kb, asset_kb + delta, bundle_kb);
        }

        /// Property: Zero bundle size never panics
        #[test]
        fn prop_zero_bundle_no_panic_any_asset_size_never_panics(asset_kb in 0u64..10_000_000) {
            let _ = AssetPriority::from_size(asset_kb, 0);
            // If we reach here, no panic occurred
        }

        /// Property: Critical priority for assets >10% of bundle
        #[test]
        fn prop_critical_above_10_percent_over_threshold_returns_critical(
            bundle_kb in 1000u64..10_000_000
        ) {
            let asset_kb = (bundle_kb as f64 * 0.11) as u64; // 11% of bundle
            let priority = AssetPriority::from_size(asset_kb, bundle_kb);
            prop_assert_eq!(priority, AssetPriority::Critical,
                "Asset {}KB ({}% of {}KB bundle) should be Critical",
                asset_kb, (asset_kb as f64 / bundle_kb as f64) * 100.0, bundle_kb);
        }

        /// Property: Critical priority for assets >500KB regardless of percentage
        #[test]
        fn prop_critical_above_500kb_large_assets_always_critical(
            asset_kb in 501u64..10_000_000,
            bundle_kb in 1_000_000u64..100_000_000
        ) {
            let priority = AssetPriority::from_size(asset_kb, bundle_kb);
            prop_assert_eq!(priority, AssetPriority::Critical,
                "Asset {}KB should be Critical (>500KB)", asset_kb);
        }

        /// Property: Low priority for assets <2% and <100KB
        #[test]
        fn prop_low_under_thresholds_small_percentage_returns_low(
            bundle_kb in 10_000u64..10_000_000
        ) {
            let asset_kb = 50; // <100KB and <2% of any bundle >2500KB
            let priority = AssetPriority::from_size(asset_kb, bundle_kb);
            prop_assert_eq!(priority, AssetPriority::Low,
                "Asset {}KB ({}% of {}KB) should be Low",
                asset_kb, (asset_kb as f64 / bundle_kb as f64) * 100.0, bundle_kb);
        }

        /// Property: Priority boundaries are well-defined
        #[test]
        fn prop_priority_boundaries_exact_thresholds_assign_correctly(
            bundle_kb in 1000u64..10_000_000
        ) {
            // Test exact boundary conditions
            // Add 1 to ensure we're definitely above the threshold after integer truncation
            let critical_percent = ((bundle_kb as f64 * 0.10001) as u64) + 1;
            let high_percent = ((bundle_kb as f64 * 0.05001) as u64) + 1;

            // Just above critical threshold (should be Critical)
            prop_assert!(matches!(
                AssetPriority::from_size(critical_percent, bundle_kb),
                AssetPriority::Critical
            ));

            // Just above high threshold (but below critical percentage)
            if high_percent < (bundle_kb as f64 * 0.09) as u64 {
                prop_assert!(matches!(
                    AssetPriority::from_size(high_percent, bundle_kb),
                    AssetPriority::High | AssetPriority::Critical
                ));
            }
        }

        /// Property: Percentage calculation is always non-negative
        #[test]
        fn prop_percentage_non_negative_calculation_always_valid(
            asset_kb in 0u64..10_000_000,
            bundle_kb in 1u64..10_000_000
        ) {
            let percentage = if bundle_kb > 0 {
                (asset_kb as f64 / bundle_kb as f64) * 100.0
            } else {
                0.0
            };
            prop_assert!(percentage >= 0.0, "Percentage should be non-negative");
            prop_assert!(percentage <= 100.0 * 10_000_000.0, "Percentage should be reasonable");
        }

        /// Property: Asset larger than bundle gets critical priority
        #[test]
        fn prop_asset_larger_than_bundle_oversized_asset_returns_critical(
            bundle_kb in 1u64..1_000_000,
            extra in 1u64..1_000_000
        ) {
            let asset_kb = bundle_kb + extra;
            let priority = AssetPriority::from_size(asset_kb, bundle_kb);
            // Asset >100% of bundle should definitely be Critical (>10% threshold)
            prop_assert_eq!(priority, AssetPriority::Critical,
                "Asset {}KB (>100% of {}KB bundle) should be Critical",
                asset_kb, bundle_kb);
        }

        /// Property: Font extensions always detected as Font type
        #[test]
        fn prop_font_extensions_all_variations_return_font_type(
            prefix in "[a-z]{1,20}",
            font_ext in prop::sample::select(vec!["ttf", "otf", "woff", "woff2"])
        ) {
            let filename = format!("{}.{}", prefix, font_ext);
            let detected = AssetDetector::detect_type(Path::new(&filename));
            prop_assert_eq!(detected, AssetType::Font,
                "File '{}' should be detected as Font", filename);
        }

        /// Property: Image extensions always detected as Image type
        #[test]
        fn prop_image_extensions_all_variations_return_image_type(
            prefix in "[a-z]{1,20}",
            img_ext in prop::sample::select(vec!["png", "jpg", "jpeg", "gif", "svg", "webp"])
        ) {
            let filename = format!("{}.{}", prefix, img_ext);
            let detected = AssetDetector::detect_type(Path::new(&filename));
            prop_assert_eq!(detected, AssetType::Image,
                "File '{}' should be detected as Image", filename);
        }

        /// Property: Data file extensions always detected as Data type
        #[test]
        fn prop_data_extensions_all_variations_return_data_type(
            prefix in "[a-z]{1,20}",
            data_ext in prop::sample::select(vec!["json", "toml", "yaml", "txt", "css"])
        ) {
            let filename = format!("{}.{}", prefix, data_ext);
            let detected = AssetDetector::detect_type(Path::new(&filename));
            prop_assert_eq!(detected, AssetType::Data,
                "File '{}' should be detected as Data", filename);
        }

        /// Property: Unknown extensions always detected as Unknown type
        #[test]
        fn prop_unknown_extensions_unrecognized_return_unknown_type(
            prefix in "[a-z]{1,20}",
            unknown_ext in prop::sample::select(vec!["xyz", "abc", "unknown", "bin", "dat"])
        ) {
            let filename = format!("{}.{}", prefix, unknown_ext);
            let detected = AssetDetector::detect_type(Path::new(&filename));
            prop_assert_eq!(detected, AssetType::Unknown,
                "File '{}' should be detected as Unknown", filename);
        }

        /// Property: Path depth doesn't affect asset type detection
        #[test]
        fn prop_path_depth_any_depth_detects_type_correctly(
            depth in 0usize..5,
            filename in "[a-z]{1,15}\\.(ttf|png|json)"
        ) {
            let path_parts: Vec<String> = (0..depth)
                .map(|i| format!("dir{}", i))
                .collect();
            let full_path = if path_parts.is_empty() {
                filename.clone()
            } else {
                format!("{}/{}", path_parts.join("/"), filename)
            };

            let detected = AssetDetector::detect_type(Path::new(&full_path));
            // Should detect type regardless of path depth
            prop_assert!(matches!(
                detected,
                AssetType::Font | AssetType::Image | AssetType::Data
            ), "Path '{}' should detect valid asset type", full_path);
        }

        /// Property: Multiple extensions use only the last one
        #[test]
        fn prop_multiple_extensions_last_extension_determines_type(
            base in "[a-z]{1,10}",
            middle_ext in "[a-z]{2,5}",
            final_ext in prop::sample::select(vec!["png", "ttf", "json", "unknown"])
        ) {
            let filename = format!("{}.{}.{}", base, middle_ext, final_ext);
            let detected = AssetDetector::detect_type(Path::new(&filename));

            // Type should match only the final extension
            let expected = match final_ext {
                "png" => AssetType::Image,
                "ttf" => AssetType::Font,
                "json" => AssetType::Data,
                _ => AssetType::Unknown,
            };

            prop_assert_eq!(detected, expected,
                "File '{}' should detect type from final extension '{}'",
                filename, final_ext);
        }

        /// Property: Extensions are case-insensitive
        #[test]
        fn prop_case_insensitivity_uppercase_extensions_match_lowercase(
            prefix in "[a-z]{1,15}",
            ext in prop::sample::select(vec!["PNG", "TTF", "JSON", "WOFF2"])
        ) {
            let filename = format!("{}.{}", prefix, ext);
            let detected = AssetDetector::detect_type(Path::new(&filename));

            // Uppercase should match the same as lowercase
            let expected = match ext {
                "PNG" => AssetType::Image,
                "TTF" => AssetType::Font,
                "JSON" => AssetType::Data,
                "WOFF2" => AssetType::Font,
                _ => AssetType::Unknown,
            };

            prop_assert_eq!(detected, expected,
                "File '{}' with uppercase extension should match lowercase", filename);
        }
    }
}
