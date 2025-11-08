//! Core analysis types for twiggy WASM analyzer

use serde::{Deserialize, Serialize};

use super::recommendation::Recommendation;

/// Analysis mode for twiggy
#[derive(Debug, Clone, Copy)]
pub enum AnalysisMode {
    /// Top contributors by shallow size
    Top,
    /// Dominator analysis (retained size)
    Dominators,
    /// Dead code detection
    Dead,
    /// Monomorphization analysis
    Monos,
}

/// Single analysis item from twiggy output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisItem {
    /// Size in bytes
    pub size_bytes: u64,
    /// Percentage of total
    pub percentage: f64,
    /// Symbol name or description
    pub name: String,
}

/// Grouped monomorphization analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonomorphizationGroup {
    /// Base function name (demangled)
    pub function_name: String,
    /// Number of instantiations
    pub instantiation_count: usize,
    /// Total size across all instantiations
    pub total_size_bytes: u64,
    /// Average size per instantiation
    pub avg_size_bytes: u64,
    /// Individual instantiations
    pub instantiations: Vec<AnalysisItem>,
    /// Estimated savings if using trait objects (keeps 1 copy)
    pub potential_savings_bytes: u64,
}

/// Complete analysis results
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResults {
    /// Total WASM file size
    pub total_size_bytes: u64,
    /// Analysis mode used
    pub mode: String,
    /// Top items
    pub items: Vec<AnalysisItem>,
    /// Recommendations based on findings
    pub recommendations: Vec<Recommendation>,
    /// Monomorphization groups (only populated for Monos mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mono_groups: Option<Vec<MonomorphizationGroup>>,
}
