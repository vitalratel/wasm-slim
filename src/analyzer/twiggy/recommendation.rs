//! Recommendation types for twiggy analysis

use serde::{Deserialize, Serialize};

/// Actionable recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Priority: P0 (Critical), P1 (High), P2 (Medium), P3 (Low)
    pub priority: String,
    /// Description of the recommendation
    pub description: String,
    /// Estimated savings in KB
    pub estimated_savings_kb: u64,
    /// Estimated savings as percentage
    pub estimated_savings_percent: f64,
}
