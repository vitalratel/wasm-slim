//! Asset type definitions

use serde::{Deserialize, Serialize};

/// Type of detected asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    /// Font file (TTF, OTF, WOFF, etc.)
    Font,
    /// Image file (PNG, JPG, SVG, etc.)
    Image,
    /// Generic data file
    Data,
    /// Unknown or unrecognized asset type
    Unknown,
}

/// Priority level for externalization recommendation
/// Ordered from lowest to highest priority for correct Ord comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetPriority {
    /// <2% of bundle or <100KB
    Low,
    /// 2-5% of bundle or 100-200KB
    Medium,
    /// 5-10% of bundle or 200-500KB
    High,
    /// >10% of bundle or >500KB
    Critical,
}

impl AssetPriority {
    /// Priority thresholds
    pub const CRITICAL_PERCENTAGE: f64 = 10.0;
    /// Critical size threshold in KB
    pub const CRITICAL_SIZE_KB: u64 = 500;
    /// High priority percentage threshold
    pub const HIGH_PERCENTAGE: f64 = 5.0;
    /// High size threshold in KB
    pub const HIGH_SIZE_KB: u64 = 200;
    /// Medium priority percentage threshold
    pub const MEDIUM_PERCENTAGE: f64 = 2.0;
    /// Medium size threshold in KB
    pub const MEDIUM_SIZE_KB: u64 = 100;

    /// Calculate priority based on asset size and bundle size
    ///
    /// Priority is determined by either percentage of bundle or absolute size:
    /// - **Critical**: >10% of bundle OR >500KB
    /// - **High**: >5% of bundle OR >200KB
    /// - **Medium**: >2% of bundle OR >100KB
    /// - **Low**: Everything else
    ///
    /// # Arguments
    /// * `asset_kb` - Size of the asset in kilobytes
    /// * `bundle_kb` - Total bundle size in kilobytes
    ///
    /// # Examples
    ///
    /// Large asset triggers critical priority:
    /// ```
    /// use wasm_slim::analyzer::asset_types::AssetPriority;
    ///
    /// // 600KB asset is critical regardless of percentage
    /// let priority = AssetPriority::from_size(600, 10_000);
    /// assert_eq!(priority, AssetPriority::Critical);
    /// ```
    ///
    /// Percentage-based priority:
    /// ```
    /// use wasm_slim::analyzer::asset_types::AssetPriority;
    ///
    /// // 12% of 1MB bundle = 120KB, high percentage triggers critical
    /// let priority = AssetPriority::from_size(120, 1_000);
    /// assert_eq!(priority, AssetPriority::Critical);
    ///
    /// // 6% of 1MB bundle = 60KB, triggers high priority
    /// let priority = AssetPriority::from_size(60, 1_000);
    /// assert_eq!(priority, AssetPriority::High);
    /// ```
    pub fn from_size(asset_kb: u64, bundle_kb: u64) -> Self {
        let percentage = if bundle_kb > 0 {
            (asset_kb as f64 / bundle_kb as f64) * 100.0
        } else {
            0.0
        };

        if percentage > Self::CRITICAL_PERCENTAGE || asset_kb > Self::CRITICAL_SIZE_KB {
            AssetPriority::Critical
        } else if percentage > Self::HIGH_PERCENTAGE || asset_kb > Self::HIGH_SIZE_KB {
            AssetPriority::High
        } else if percentage > Self::MEDIUM_PERCENTAGE || asset_kb > Self::MEDIUM_SIZE_KB {
            AssetPriority::Medium
        } else {
            AssetPriority::Low
        }
    }
}
