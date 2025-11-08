//! Display formatting for asset detection results

use super::assets::AssetPriority;
use console::Color;

/// Display helper for AssetPriority
pub trait AssetPriorityDisplay {
    /// Get emoji icon for this priority level
    fn emoji(&self) -> &'static str;

    /// Get color for terminal output
    fn color(&self) -> Color;
}

impl AssetPriorityDisplay for AssetPriority {
    fn emoji(&self) -> &'static str {
        match self {
            AssetPriority::Critical => "⚠️",
            AssetPriority::High => "📸",
            AssetPriority::Medium => "🔤",
            AssetPriority::Low => "📄",
        }
    }

    fn color(&self) -> Color {
        match self {
            AssetPriority::Critical => Color::Red,
            AssetPriority::High => Color::Yellow,
            AssetPriority::Medium => Color::Blue,
            AssetPriority::Low => Color::White,
        }
    }
}
