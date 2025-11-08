//! Size metrics for build pipeline optimization tracking

/// Size metrics for before/after comparison
///
/// Tracks binary size reduction through the optimization pipeline.
#[derive(Debug, Clone)]
pub struct SizeMetrics {
    /// Binary size before optimization (bytes)
    pub before_bytes: u64,
    /// Binary size after optimization (bytes)
    pub after_bytes: u64,
}

impl SizeMetrics {
    /// Calculate size reduction in bytes
    pub fn reduction_bytes(&self) -> i64 {
        self.before_bytes as i64 - self.after_bytes as i64
    }

    /// Calculate size reduction as percentage
    pub fn reduction_percent(&self) -> f64 {
        if self.before_bytes == 0 {
            return 0.0;
        }
        (self.reduction_bytes() as f64 / self.before_bytes as f64) * 100.0
    }
}
