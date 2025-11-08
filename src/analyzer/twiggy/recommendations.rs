//! Recommendation generation and monomorphization analysis

use super::analysis_types::{AnalysisItem, AnalysisMode, MonomorphizationGroup};
use super::recommendation::Recommendation;
use crate::analyzer::TwiggyAnalyzer;
use crate::infra::{CommandExecutor, FileSystem};
use std::collections::HashMap;

impl<FS: FileSystem, CE: CommandExecutor> TwiggyAnalyzer<FS, CE> {
    /// Generate actionable recommendations based on analysis
    pub(super) fn generate_recommendations(
        &self,
        items: &[AnalysisItem],
        total_size: u64,
        mode: AnalysisMode,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        match mode {
            AnalysisMode::Top => {
                self.generate_top_recommendations(items, total_size, &mut recommendations);
            }
            AnalysisMode::Dominators => {
                self.generate_dominator_recommendations(items, total_size, &mut recommendations);
            }
            AnalysisMode::Dead => {
                self.generate_dead_code_recommendations(items, total_size, &mut recommendations);
            }
            AnalysisMode::Monos => {
                self.generate_monos_recommendations(items, &mut recommendations);
            }
        }

        recommendations
    }

    /// Generate recommendations for top contributors
    pub(super) fn generate_top_recommendations(
        &self,
        items: &[AnalysisItem],
        total_size: u64,
        recommendations: &mut Vec<Recommendation>,
    ) {
        // Check for large data segments (>50KB)
        for item in items {
            if item.name.starts_with("data[") && item.size_bytes > 50 * 1024 {
                recommendations.push(Recommendation {
                    priority: "P1".to_string(),
                    description: format!(
                        "Large data segment '{}' detected. Consider externalizing embedded assets or generating data at runtime.",
                        item.name
                    ),
                    estimated_savings_kb: item.size_bytes / 1024,
                    estimated_savings_percent: item.percentage,
                });
            }
        }

        // Check if top 20 items dominate
        if items.len() >= 20 {
            let top_20_size: u64 = items.iter().take(20).map(|i| i.size_bytes).sum();
            let top_20_percent = (top_20_size as f64 / total_size as f64) * 100.0;

            if top_20_percent > 30.0 {
                recommendations.push(Recommendation {
                    priority: "P0".to_string(),
                    description: format!(
                        "Top 20 items contribute {:.1}% of bundle. Focus optimization efforts here for maximum impact.",
                        top_20_percent
                    ),
                    estimated_savings_kb: (top_20_size as f64 * 0.5) as u64 / 1024, // Estimate 50% reduction potential
                    estimated_savings_percent: top_20_percent * 0.5,
                });
            }
        }
    }

    /// Generate recommendations for dominators
    pub(super) fn generate_dominator_recommendations(
        &self,
        items: &[AnalysisItem],
        _total_size: u64,
        recommendations: &mut Vec<Recommendation>,
    ) {
        for item in items {
            if item.percentage > 20.0 {
                recommendations.push(Recommendation {
                    priority: "P0".to_string(),
                    description: format!(
                        "Symbol '{}' dominates {:.1}% of bundle. Making this optional via feature flag could provide significant savings.",
                        item.name, item.percentage
                    ),
                    estimated_savings_kb: item.size_bytes / 1024,
                    estimated_savings_percent: item.percentage,
                });
            }
        }
    }

    /// Generate recommendations for dead code
    pub(super) fn generate_dead_code_recommendations(
        &self,
        items: &[AnalysisItem],
        total_size: u64,
        recommendations: &mut Vec<Recommendation>,
    ) {
        let total_dead: u64 = items.iter().map(|i| i.size_bytes).sum();
        let dead_percent = (total_dead as f64 / total_size as f64) * 100.0;

        if dead_percent > 10.0 {
            recommendations.push(Recommendation {
                priority: "P1".to_string(),
                description: format!(
                    "{:.1}% of bundle is potentially removable. Enable lto = 'fat' and strip = true in Cargo.toml.",
                    dead_percent
                ),
                estimated_savings_kb: total_dead / 1024,
                estimated_savings_percent: dead_percent,
            });
        } else if dead_percent < 1.0 {
            recommendations.push(Recommendation {
                priority: "P3".to_string(),
                description: "Minimal dead code detected (<1%). Bundle is well-optimized. Focus on other areas.".to_string(),
                estimated_savings_kb: 0,
                estimated_savings_percent: 0.0,
            });
        }
    }

    /// Generate recommendations for monomorphization
    pub(super) fn generate_monos_recommendations(
        &self,
        items: &[AnalysisItem],
        recommendations: &mut Vec<Recommendation>,
    ) {
        // Check for excessive instantiations
        for item in items {
            // Parse instantiation count from name if possible
            // twiggy monos output typically shows: "generic_function: 15 instantiations"
            if item.name.contains("instantiation") {
                recommendations.push(Recommendation {
                    priority: "P2".to_string(),
                    description: format!(
                        "Excessive monomorphization detected in '{}'. Consider using trait objects or limiting generic types.",
                        item.name
                    ),
                    estimated_savings_kb: item.size_bytes / 1024,
                    estimated_savings_percent: item.percentage,
                });
            }
        }
    }

    /// Group monomorphizations by base function
    ///
    /// Analyzes items to identify related generic instantiations and calculate
    /// potential savings from using trait objects (dynamic dispatch).
    pub(super) fn group_monomorphizations(
        &self,
        items: &[AnalysisItem],
    ) -> Vec<MonomorphizationGroup> {
        let mut groups: HashMap<String, Vec<&AnalysisItem>> = HashMap::new();

        // Group items by base function name
        for item in items {
            let base_name = self.extract_base_function_name(&item.name);
            groups.entry(base_name).or_default().push(item);
        }

        // Convert to MonomorphizationGroup and calculate savings
        let mut result: Vec<MonomorphizationGroup> = groups
            .into_iter()
            .filter(|(_, items)| items.len() > 1) // Only groups with multiple instantiations
            .map(|(function_name, instantiations)| {
                let instantiation_count = instantiations.len();
                let total_size_bytes: u64 = instantiations.iter().map(|i| i.size_bytes).sum();
                let avg_size_bytes = total_size_bytes / instantiation_count as u64;

                // Estimate savings: keep largest instantiation, save the rest
                // (trait object has small vtable overhead, so we keep the largest)
                let max_size = instantiations
                    .iter()
                    .map(|i| i.size_bytes)
                    .max()
                    .unwrap_or(0);
                let potential_savings_bytes = total_size_bytes.saturating_sub(max_size);

                MonomorphizationGroup {
                    function_name,
                    instantiation_count,
                    total_size_bytes,
                    avg_size_bytes,
                    instantiations: instantiations.into_iter().cloned().collect(),
                    potential_savings_bytes,
                }
            })
            .collect();

        // Sort by potential savings (highest first)
        result.sort_by(|a, b| b.potential_savings_bytes.cmp(&a.potential_savings_bytes));

        result
    }

    /// Extract base function name from mangled/demangled symbol
    ///
    /// Handles both mangled Rust symbols and demangled names.
    /// Groups similar instantiations by removing type parameters.
    pub(super) fn extract_base_function_name(&self, symbol: &str) -> String {
        // For impl blocks like "<Type as Trait>::method" - check this FIRST
        if symbol.starts_with('<') {
            if let Some(pos) = symbol.rfind(">::") {
                let method_name = symbol[pos + 3..].split('<').next().unwrap_or(symbol);
                return String::from(method_name);
            }
        }

        // If already demangled, extract base name
        if symbol.contains("<") {
            // Extract function name before type parameters
            // e.g., "serde_json::ser::Serializer::serialize<T>" -> "serde_json::ser::Serializer::serialize"
            if let Some(pos) = symbol.find('<') {
                return String::from(symbol[..pos].trim());
            }
        }

        // Try to demangle if it looks like a mangled symbol
        if symbol.starts_with('_') && symbol.contains("ZN") {
            // Basic demangling hint: extract visible text
            // For production, consider using rustc-demangle crate
            let demangled = symbol
                .split("ZN")
                .last()
                .unwrap_or(symbol)
                .split("E")
                .next()
                .unwrap_or(symbol);
            return String::from(demangled);
        }

        // Fallback: use first part before any special chars
        let base_name = symbol.split(&['<', '(', ' '][..]).next().unwrap_or(symbol);
        String::from(base_name)
    }

    /// Generate enhanced recommendations from monomorphization groups
    ///
    /// Creates actionable recommendations with concrete size impact estimates.
    pub(super) fn generate_monos_recommendations_enhanced(
        &self,
        groups: &[MonomorphizationGroup],
        total_size_bytes: u64,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // Calculate total monomorphization overhead
        let total_mono_size: u64 = groups.iter().map(|g| g.total_size_bytes).sum();
        let total_savings: u64 = groups.iter().map(|g| g.potential_savings_bytes).sum();
        let mono_percent = (total_mono_size as f64 / total_size_bytes as f64) * 100.0;

        // Overall assessment
        if mono_percent > 15.0 {
            recommendations.push(Recommendation {
                priority: "P0".to_string(),
                description: format!(
                    "Significant monomorphization bloat detected ({:.1}% of bundle). {} KB across {} generic functions.",
                    mono_percent,
                    total_mono_size / 1024,
                    groups.len()
                ),
                estimated_savings_kb: total_savings / 1024,
                estimated_savings_percent: (total_savings as f64 / total_size_bytes as f64) * 100.0,
            });
        } else if mono_percent > 5.0 {
            recommendations.push(Recommendation {
                priority: "P2".to_string(),
                description: format!(
                    "Moderate monomorphization detected ({:.1}% of bundle). Consider optimization for top contributors.",
                    mono_percent
                ),
                estimated_savings_kb: total_savings / 1024,
                estimated_savings_percent: (total_savings as f64 / total_size_bytes as f64) * 100.0,
            });
        } else {
            recommendations.push(Recommendation {
                priority: "P3".to_string(),
                description: format!(
                    "Minimal monomorphization overhead ({:.1}%). Not a priority optimization target.",
                    mono_percent
                ),
                estimated_savings_kb: 0,
                estimated_savings_percent: 0.0,
            });
            return recommendations; // No need for per-function recommendations
        }

        // Top offenders (>10 instantiations or >50KB total)
        for group in groups.iter().take(10) {
            if group.instantiation_count >= 10 || group.total_size_bytes > 50 * 1024 {
                let priority = if group.potential_savings_bytes > 100 * 1024 {
                    "P0"
                } else if group.potential_savings_bytes > 30 * 1024 {
                    "P1"
                } else {
                    "P2"
                };

                recommendations.push(Recommendation {
                    priority: priority.to_string(),
                    description: format!(
                        "Function '{}' has {} instantiations ({} KB total). Consider using 'Box<dyn Trait>' or limiting type parameters.",
                        group.function_name,
                        group.instantiation_count,
                        group.total_size_bytes / 1024
                    ),
                    estimated_savings_kb: group.potential_savings_bytes / 1024,
                    estimated_savings_percent: (group.potential_savings_bytes as f64 / total_size_bytes as f64) * 100.0,
                });
            }
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_recommendations_empty_items_returns_empty() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Empty items
        let recommendations = analyzer.generate_recommendations(&[], 1000, AnalysisMode::Top);
        assert!(recommendations.is_empty() || recommendations.len() < 3);

        // Single small item
        let small_item = vec![AnalysisItem {
            size_bytes: 100,
            percentage: 0.01,
            name: "tiny_function".to_string(),
        }];
        let recommendations =
            analyzer.generate_recommendations(&small_item, 1000000, AnalysisMode::Top);
        // Should not generate recommendations for tiny items
        assert!(recommendations.is_empty() || recommendations.iter().all(|r| r.priority == "P3"));
    }

    #[test]
    fn test_generate_recommendations_top_mode_large_data_segment() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let items = vec![AnalysisItem {
            size_bytes: 100 * 1024,
            percentage: 10.0,
            name: "data[0]".to_string(),
        }];

        let mut recs = Vec::new();
        analyzer.generate_top_recommendations(&items, 1024 * 1024, &mut recs);

        assert!(!recs.is_empty());
        assert!(recs.iter().any(|r| r.description.contains("data segment")));
    }

    #[test]
    fn test_generate_recommendations_top_mode_prioritizes_largest() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Create 20 items totaling >30% of bundle
        let items: Vec<AnalysisItem> = (0..20)
            .map(|i| AnalysisItem {
                size_bytes: 20 * 1024,
                percentage: 2.0,
                name: format!("func_{}", i),
            })
            .collect();

        let mut recs = Vec::new();
        analyzer.generate_top_recommendations(&items, 1024 * 1024, &mut recs);

        assert!(recs.iter().any(|r| r.description.contains("Top 20")));
    }

    #[test]
    fn test_generate_recommendations_dominators_mode_high_priority() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let items = vec![AnalysisItem {
            size_bytes: 250 * 1024,
            percentage: 25.0,
            name: "dominating_symbol".to_string(),
        }];

        let mut recs = Vec::new();
        analyzer.generate_dominator_recommendations(&items, 1024 * 1024, &mut recs);

        assert!(!recs.is_empty());
        assert_eq!(recs[0].priority, "P0");
        assert!(recs[0].description.contains("dominates"));
    }

    #[test]
    fn test_generate_recommendations_garbage_mode_high_percentage() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let items = vec![AnalysisItem {
            size_bytes: 150 * 1024,
            percentage: 15.0,
            name: "dead_func".to_string(),
        }];

        let mut recs = Vec::new();
        analyzer.generate_dead_code_recommendations(&items, 1024 * 1024, &mut recs);

        assert!(!recs.is_empty());
        assert!(recs.iter().any(|r| r.description.contains("removable")));
    }

    #[test]
    fn test_generate_recommendations_garbage_mode_low_percentage() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let items = vec![AnalysisItem {
            size_bytes: 5 * 1024,
            percentage: 0.5,
            name: "tiny_dead".to_string(),
        }];

        let mut recs = Vec::new();
        analyzer.generate_dead_code_recommendations(&items, 1024 * 1024, &mut recs);

        assert!(!recs.is_empty());
        assert!(recs
            .iter()
            .any(|r| r.description.contains("well-optimized")));
        assert_eq!(recs[0].priority, "P3");
    }

    #[test]
    fn test_generate_recommendations_monos_mode_suggests_generics() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let items = vec![AnalysisItem {
            size_bytes: 50 * 1024,
            percentage: 5.0,
            name: "generic_function: 25 instantiations".to_string(),
        }];

        let mut recs = Vec::new();
        analyzer.generate_monos_recommendations(&items, &mut recs);

        assert!(!recs.is_empty());
    }

    #[test]
    fn test_extract_base_function_name_demangled_symbols() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Function with type parameters
        let name = analyzer.extract_base_function_name("serde_json::ser::Serializer::serialize<T>");
        assert_eq!(name, "serde_json::ser::Serializer::serialize");

        // Multiple type parameters
        let name = analyzer.extract_base_function_name("core::convert::From<T, U>::from");
        assert_eq!(name, "core::convert::From");
    }

    #[test]
    fn test_extract_base_function_name_impl_trait_format() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Impl trait format
        let name = analyzer.extract_base_function_name("<Type as Trait>::method");
        assert_eq!(name, "method");

        // With type parameters
        let name = analyzer.extract_base_function_name("<Vec<T> as IntoIterator>::into_iter");
        assert_eq!(name, "into_iter");
    }

    #[test]
    fn test_extract_base_function_name_mangled_symbols() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Mangled symbol
        let name = analyzer.extract_base_function_name("_ZN4core3fmt3Display3fmtE");
        assert!(name.contains("fmt") || name.contains("Display"));

        // Should handle various mangling patterns
        let name = analyzer
            .extract_base_function_name("_ZN52_impl_std_convert_From_u32_for_i64_17from_abcdefE");
        assert!(!name.is_empty());
    }

    #[test]
    fn test_extract_base_function_name_fallback() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        // Simple function name
        let name = analyzer.extract_base_function_name("simple_function");
        assert_eq!(name, "simple_function");

        // Function with parentheses
        let name = analyzer.extract_base_function_name("function_name(args)");
        assert_eq!(name, "function_name");

        // With space
        let name = analyzer.extract_base_function_name("my_func with_space");
        assert_eq!(name, "my_func");
    }

    #[test]
    fn test_group_monomorphizations_multiple_instantiations() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let items = vec![
            AnalysisItem {
                size_bytes: 1000,
                percentage: 1.0,
                name: "serialize<i32>".to_string(),
            },
            AnalysisItem {
                size_bytes: 1200,
                percentage: 1.2,
                name: "serialize<String>".to_string(),
            },
            AnalysisItem {
                size_bytes: 800,
                percentage: 0.8,
                name: "serialize<Vec<u8>>".to_string(),
            },
        ];

        let groups = analyzer.group_monomorphizations(&items);

        // Should have 1 group for "serialize"
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].function_name, "serialize");
        assert_eq!(groups[0].instantiation_count, 3);
        assert_eq!(groups[0].total_size_bytes, 3000);

        // Savings should be total minus largest
        let max_size = 1200; // String variant
        assert_eq!(groups[0].potential_savings_bytes, 3000 - max_size);
    }

    #[test]
    fn test_group_monomorphizations_filters_single_instantiation() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let items = vec![
            AnalysisItem {
                size_bytes: 1000,
                percentage: 1.0,
                name: "serialize<i32>".to_string(),
            },
            AnalysisItem {
                size_bytes: 2000,
                percentage: 2.0,
                name: "deserialize<String>".to_string(), // Only one
            },
            AnalysisItem {
                size_bytes: 1500,
                percentage: 1.5,
                name: "serialize<String>".to_string(),
            },
        ];

        let groups = analyzer.group_monomorphizations(&items);

        // Should only have serialize group (2 instantiations)
        // deserialize has only 1, so it's filtered out
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].function_name, "serialize");
        assert_eq!(groups[0].instantiation_count, 2);
    }

    #[test]
    fn test_group_monomorphizations_sorts_by_savings() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let items = vec![
            // Small group
            AnalysisItem {
                size_bytes: 100,
                percentage: 0.1,
                name: "small<i32>".to_string(),
            },
            AnalysisItem {
                size_bytes: 100,
                percentage: 0.1,
                name: "small<u32>".to_string(),
            },
            // Large group
            AnalysisItem {
                size_bytes: 5000,
                percentage: 5.0,
                name: "large<String>".to_string(),
            },
            AnalysisItem {
                size_bytes: 5000,
                percentage: 5.0,
                name: "large<Vec>".to_string(),
            },
            AnalysisItem {
                size_bytes: 5000,
                percentage: 5.0,
                name: "large<Box>".to_string(),
            },
        ];

        let groups = analyzer.group_monomorphizations(&items);

        // Should have 2 groups
        assert_eq!(groups.len(), 2);

        // "large" should come first (higher savings)
        assert_eq!(groups[0].function_name, "large");
        assert_eq!(groups[0].potential_savings_bytes, 10000); // 15000 - 5000

        assert_eq!(groups[1].function_name, "small");
        assert_eq!(groups[1].potential_savings_bytes, 100); // 200 - 100
    }

    #[test]
    fn test_generate_monos_recommendations_enhanced_significant_bloat() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let groups = vec![MonomorphizationGroup {
            function_name: "big_generic".to_string(),
            instantiation_count: 20,
            total_size_bytes: 200_000, // 200 KB
            avg_size_bytes: 10_000,
            instantiations: vec![],
            potential_savings_bytes: 190_000, // Save 190 KB
        }];

        let total_size = 1_000_000; // 1 MB total
        let recommendations = analyzer.generate_monos_recommendations_enhanced(&groups, total_size);

        // Should have recommendations (>15% threshold)
        assert!(!recommendations.is_empty());

        // First should be P0 for overall assessment
        assert_eq!(recommendations[0].priority, "P0");
        assert!(recommendations[0].description.contains("Significant"));

        // Should have recommendation for the big_generic function
        let function_rec = recommendations
            .iter()
            .find(|r| r.description.contains("big_generic"));
        assert!(function_rec.is_some());
        assert_eq!(
            function_rec
                .expect("should have function recommendation")
                .priority,
            "P0"
        ); // >100 KB savings
    }

    #[test]
    fn test_generate_monos_recommendations_enhanced_moderate_bloat() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let groups = vec![MonomorphizationGroup {
            function_name: "medium_generic".to_string(),
            instantiation_count: 8,
            total_size_bytes: 80_000, // 80 KB
            avg_size_bytes: 10_000,
            instantiations: vec![],
            potential_savings_bytes: 70_000,
        }];

        let total_size = 1_000_000; // 1 MB total (8% monomorphization)
        let recommendations = analyzer.generate_monos_recommendations_enhanced(&groups, total_size);

        // Should have moderate priority (5-15% threshold)
        assert!(!recommendations.is_empty());
        assert_eq!(recommendations[0].priority, "P2");
        assert!(recommendations[0].description.contains("Moderate"));
    }

    #[test]
    fn test_generate_monos_recommendations_enhanced_minimal_bloat() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let groups = vec![MonomorphizationGroup {
            function_name: "tiny_generic".to_string(),
            instantiation_count: 3,
            total_size_bytes: 10_000, // 10 KB
            avg_size_bytes: 3_333,
            instantiations: vec![],
            potential_savings_bytes: 6_667,
        }];

        let total_size = 1_000_000; // 1 MB total (1% monomorphization)
        let recommendations = analyzer.generate_monos_recommendations_enhanced(&groups, total_size);

        // Should have low priority and stop early
        assert_eq!(recommendations.len(), 1); // Only overall assessment
        assert_eq!(recommendations[0].priority, "P3");
        assert!(recommendations[0].description.contains("Minimal"));
        assert_eq!(recommendations[0].estimated_savings_kb, 0); // Not worth optimizing
    }

    #[test]
    fn test_generate_monos_recommendations_enhanced_priority_thresholds() {
        let analyzer = TwiggyAnalyzer::new("dummy.wasm");

        let total_size = 1_000_000; // 1 MB

        // >100 KB savings = P0
        let groups_p0 = vec![MonomorphizationGroup {
            function_name: "huge".to_string(),
            instantiation_count: 50,
            total_size_bytes: 200_000,
            avg_size_bytes: 4_000,
            instantiations: vec![],
            potential_savings_bytes: 150_000, // 150 KB
        }];
        let recs = analyzer.generate_monos_recommendations_enhanced(&groups_p0, total_size);
        let func_rec = recs
            .iter()
            .find(|r| r.description.contains("huge"))
            .expect("should have huge function recommendation");
        assert_eq!(func_rec.priority, "P0");

        // 30-100 KB savings = P1
        let groups_p1 = vec![MonomorphizationGroup {
            function_name: "medium".to_string(),
            instantiation_count: 15,
            total_size_bytes: 100_000,
            avg_size_bytes: 6_666,
            instantiations: vec![],
            potential_savings_bytes: 50_000, // 50 KB
        }];
        let recs = analyzer.generate_monos_recommendations_enhanced(&groups_p1, total_size);
        let func_rec = recs
            .iter()
            .find(|r| r.description.contains("medium"))
            .expect("should have medium function recommendation");
        assert_eq!(func_rec.priority, "P1");

        // <30 KB savings = P2
        let groups_p2 = vec![MonomorphizationGroup {
            function_name: "small".to_string(),
            instantiation_count: 10,
            total_size_bytes: 60_000,
            avg_size_bytes: 6_000,
            instantiations: vec![],
            potential_savings_bytes: 20_000, // 20 KB
        }];
        let recs = analyzer.generate_monos_recommendations_enhanced(&groups_p2, total_size);
        let func_rec = recs
            .iter()
            .find(|r| r.description.contains("small"))
            .expect("should have small function recommendation");
        assert_eq!(func_rec.priority, "P2");
    }
}
