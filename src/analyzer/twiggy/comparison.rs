//! Comparison types for WASM file analysis

use serde::{Deserialize, Serialize};

/// Comparison results between two WASM files
#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonResults {
    /// Before file size
    pub before_size_bytes: u64,
    /// After file size
    pub after_size_bytes: u64,
    /// Delta in bytes (negative = reduction)
    pub delta_bytes: i64,
    /// Delta percentage
    pub delta_percent: f64,
    /// Top differences
    pub top_changes: Vec<ChangeItem>,
}

/// A single change between two builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeItem {
    /// Size delta in bytes
    pub delta_bytes: i64,
    /// Symbol name
    pub name: String,
}
