//! Shared formatting utilities for size display and console output

use console::Emoji;

/// Wrench emoji for build/tool operations
pub const WRENCH: Emoji = Emoji("🔧", "*");

/// Rocket emoji for launch/start operations
pub const ROCKET: Emoji = Emoji("🚀", ">");

/// Checkmark emoji for success
pub const CHECKMARK: Emoji = Emoji("✅", "[OK]");

/// Crossmark emoji for failure
pub const CROSSMARK: Emoji = Emoji("❌", "[FAIL]");

/// Hammer emoji for build operations
pub const HAMMER: Emoji = Emoji("🔨", ">");

/// Sparkles emoji for completion/success
pub const SPARKLES: Emoji = Emoji("✨", "*");

/// Info emoji for informational messages
pub const INFO: Emoji = Emoji("ℹ️", "i");

/// Chart emoji for metrics/statistics
pub const CHART: Emoji = Emoji("📊", "~");

/// Microscope emoji for analysis/inspection
pub const MICROSCOPE: Emoji = Emoji("🔍", ">>");

/// Warning emoji for caution/alerts
pub const WARNING: Emoji = Emoji("⚠️", "!");

/// Format bytes as human-readable size string
///
/// # Examples
///
/// ```
/// use wasm_slim::fmt::format_bytes;
///
/// assert_eq!(format_bytes(512), "512 B");
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(1_048_576), "1.00 MB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_various_sizes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(2_621_440), "2.50 MB");
    }
}
