//! Asset scan results and metrics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::asset_types::AssetPriority;

/// A detected embedded asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedAsset {
    /// Path to the embedded asset (relative to source file)
    pub file_path: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Type of asset
    pub asset_type: super::asset_types::AssetType,
    /// Where it was detected (file:line)
    pub source_location: String,
    /// How it was detected (include_bytes!, rustybuzz, etc.)
    pub detection_method: String,
}

/// Complete scan results with all detected assets
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResults {
    /// Total number of detected assets
    pub total_assets: usize,
    /// Total size of all assets in KB
    pub total_size_kb: u64,
    /// Total bundle size in KB
    pub bundle_size_kb: u64,
    /// Percentage of bundle occupied by assets
    pub bundle_percentage: f64,
    /// All detected assets
    pub assets: Vec<DetectedAsset>,
    /// Assets grouped by priority level
    pub assets_by_priority: HashMap<AssetPriority, Vec<DetectedAsset>>,
    /// Estimated savings from externalization
    pub estimated_savings: EstimatedSavings,
}

/// Estimated savings from externalizing assets
#[derive(Debug, Serialize, Deserialize)]
pub struct EstimatedSavings {
    /// Savings from externalizing critical priority assets only (KB)
    pub critical_only_kb: u64,
    /// Savings from externalizing high and critical priority assets (KB)
    pub high_and_critical_kb: u64,
    /// Savings from externalizing all assets (KB)
    pub all_assets_kb: u64,
    /// Savings from critical assets as percentage of bundle
    pub critical_only_percent: f64,
    /// Savings from high+critical assets as percentage of bundle
    pub high_and_critical_percent: f64,
    /// Savings from all assets as percentage of bundle
    pub all_assets_percent: f64,
}
