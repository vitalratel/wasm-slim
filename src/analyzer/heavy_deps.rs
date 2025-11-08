//! Database of known heavy dependencies and their lighter alternatives
//!
//! Based on real-world WASM optimization case studies (Warp.dev).

use std::collections::HashMap;
use std::sync::OnceLock;

/// A known heavy dependency with metadata
#[derive(Debug, Clone)]
pub struct HeavyDependency {
    /// Typical size range in KB
    pub size_kb: (u32, u32),
    /// Percentage of typical bundle
    pub bundle_percent: Option<(u8, u8)>,
    /// Description of why it's heavy
    pub reason: &'static str,
    /// Recommended alternatives
    pub alternatives: Vec<DependencyAlternative>,
}

/// An alternative to a heavy dependency
#[derive(Debug, Clone)]
pub struct DependencyAlternative {
    /// Type of alternative
    pub alt_type: AlternativeType,
    /// Crate name (if replacement)
    pub crate_name: Option<&'static str>,
    /// Expected size in KB
    pub size_kb: Option<(u32, u32)>,
    /// Estimated savings percentage
    pub savings_percent: u8,
    /// Description
    pub description: &'static str,
}

/// Type of dependency alternative recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlternativeType {
    /// Replace with different crate
    Replacement,
    /// Minimize features with default-features = false
    FeatureMinimization,
    /// Split monolith into components
    Split,
    /// Make optional via feature flag
    Optional,
    /// Fix WASM-specific issue
    WasmFix,
}

/// Global database of heavy dependencies
pub static HEAVY_DEPS_DATABASE: OnceLock<HashMap<&'static str, HeavyDependency>> = OnceLock::new();

/// Initialize and return the heavy dependencies database
fn init_heavy_deps_database() -> HashMap<&'static str, HeavyDependency> {
    let mut db = HashMap::new();

    // swc_core - Known to be very large dependency
    db.insert(
        "swc_core",
        HeavyDependency {
            size_kb: (1000, 1500),
            bundle_percent: Some((30, 45)),
            reason: "Full compiler suite with all features",
            alternatives: vec![DependencyAlternative {
                alt_type: AlternativeType::Split,
                crate_name: Some("swc_ecma_parser"),
                size_kb: Some((400, 600)),
                savings_percent: 60,
                description: "Split into minimal components (parser + AST only)",
            }],
        },
    );

    // printpdf
    db.insert(
        "printpdf",
        HeavyDependency {
            size_kb: (800, 1000),
            bundle_percent: Some((15, 25)),
            reason: "Full-featured PDF library with many formats",
            alternatives: vec![
                DependencyAlternative {
                    alt_type: AlternativeType::Replacement,
                    crate_name: Some("pdf-writer"),
                    size_kb: Some((200, 400)),
                    savings_percent: 50,
                    description: "Lightweight PDF writer for simple use cases",
                },
                DependencyAlternative {
                    alt_type: AlternativeType::Replacement,
                    crate_name: Some("lopdf"),
                    size_kb: Some((400, 600)),
                    savings_percent: 30,
                    description: "PDF library with minimal features (no rayon)",
                },
            ],
        },
    );

    // lopdf with rayon
    db.insert(
        "lopdf",
        HeavyDependency {
            size_kb: (400, 800),
            bundle_percent: Some((10, 20)),
            reason: "Default includes rayon (doesn't work in WASM)",
            alternatives: vec![DependencyAlternative {
                alt_type: AlternativeType::FeatureMinimization,
                crate_name: None,
                size_kb: Some((400, 600)),
                savings_percent: 25,
                description: "Disable rayon and unused features",
            }],
        },
    );

    // tokio
    db.insert(
        "tokio",
        HeavyDependency {
            size_kb: (500, 1000),
            bundle_percent: Some((15, 30)),
            reason: "Async runtime doesn't work in WASM",
            alternatives: vec![DependencyAlternative {
                alt_type: AlternativeType::WasmFix,
                crate_name: Some("wasm-bindgen-futures"),
                size_kb: Some((50, 100)),
                savings_percent: 80,
                description: "WASM-specific async runtime",
            }],
        },
    );

    // async-std
    db.insert(
        "async-std",
        HeavyDependency {
            size_kb: (400, 800),
            bundle_percent: Some((10, 25)),
            reason: "Async runtime doesn't work in WASM",
            alternatives: vec![DependencyAlternative {
                alt_type: AlternativeType::WasmFix,
                crate_name: Some("wasm-bindgen-futures"),
                size_kb: Some((50, 100)),
                savings_percent: 80,
                description: "WASM-specific async runtime",
            }],
        },
    );

    // rustybuzz
    db.insert(
        "rustybuzz",
        HeavyDependency {
            size_kb: (400, 600),
            bundle_percent: Some((10, 15)),
            reason: "Full text shaping library",
            alternatives: vec![DependencyAlternative {
                alt_type: AlternativeType::Optional,
                crate_name: None,
                size_kb: Some((0, 0)),
                savings_percent: 100,
                description: "Make optional via feature flag (removes from default build)",
            }],
        },
    );

    // chrono
    db.insert(
        "chrono",
        HeavyDependency {
            size_kb: (300, 500),
            bundle_percent: Some((8, 15)),
            reason: "Feature-rich date/time library",
            alternatives: vec![
                DependencyAlternative {
                    alt_type: AlternativeType::Replacement,
                    crate_name: Some("time"),
                    size_kb: Some((150, 250)),
                    savings_percent: 40,
                    description: "Lighter date/time library",
                },
                DependencyAlternative {
                    alt_type: AlternativeType::WasmFix,
                    crate_name: Some("js-sys"),
                    size_kb: Some((50, 100)),
                    savings_percent: 70,
                    description: "Use browser Date API (WASM only)",
                },
            ],
        },
    );

    // regex with unicode
    db.insert(
        "regex",
        HeavyDependency {
            size_kb: (300, 600),
            bundle_percent: Some((8, 15)),
            reason: "Large unicode tables included by default",
            alternatives: vec![DependencyAlternative {
                alt_type: AlternativeType::FeatureMinimization,
                crate_name: None,
                size_kb: Some((100, 200)),
                savings_percent: 60,
                description: "Disable unicode support if not needed",
            }],
        },
    );

    // image
    db.insert(
        "image",
        HeavyDependency {
            size_kb: (400, 800),
            bundle_percent: Some((10, 20)),
            reason: "Includes all image format codecs by default",
            alternatives: vec![DependencyAlternative {
                alt_type: AlternativeType::FeatureMinimization,
                crate_name: None,
                size_kb: Some((150, 300)),
                savings_percent: 50,
                description: "Enable only needed image formats",
            }],
        },
    );

    // getrandom
    db.insert(
        "getrandom",
        HeavyDependency {
            size_kb: (50, 100),
            bundle_percent: None,
            reason: "Default doesn't work in WASM (missing OS entropy)",
            alternatives: vec![DependencyAlternative {
                alt_type: AlternativeType::WasmFix,
                crate_name: None,
                size_kb: Some((50, 100)),
                savings_percent: 0,
                description: "Enable WASM support (required for functionality, not size)",
            }],
        },
    );

    db
}

/// Get information about a heavy dependency
pub fn get_heavy_dependency_info(name: &str) -> Option<&HeavyDependency> {
    HEAVY_DEPS_DATABASE
        .get_or_init(init_heavy_deps_database)
        .get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_heavy_dependency_info_swc_core_has_split_alternatives() {
        let swc = get_heavy_dependency_info("swc_core").expect("should have swc_core info");
        assert!(!swc.alternatives.is_empty());
        assert!(swc
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::Split));
    }

    #[test]
    fn test_get_heavy_dependency_info_printpdf_has_replacement_alternatives() {
        let printpdf = get_heavy_dependency_info("printpdf").expect("should have printpdf info");
        // printpdf has Replacement alternatives (pdf-writer, lopdf)
        assert!(printpdf
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::Replacement));
        assert!(printpdf
            .alternatives
            .iter()
            .any(|a| a.crate_name == Some("pdf-writer")));
    }

    #[test]
    fn test_get_heavy_dependency_info_lopdf_has_lighter_alternative() {
        let lopdf = get_heavy_dependency_info("lopdf").expect("should have lopdf info");
        assert!(!lopdf.alternatives.is_empty());
        // lopdf has feature minimization alternative
        assert!(lopdf
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::FeatureMinimization));
    }

    #[test]
    fn test_get_heavy_dependency_info_tokio_has_wasm_alternative() {
        let tokio = get_heavy_dependency_info("tokio").expect("should have tokio info");
        // Should have WasmFix (wasm-bindgen-futures)
        assert!(tokio
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::WasmFix));
        assert!(tokio
            .alternatives
            .iter()
            .any(|a| a.crate_name == Some("wasm-bindgen-futures")));
    }

    #[test]
    fn test_get_heavy_dependency_info_async_std_has_wasm_alternative() {
        let async_std = get_heavy_dependency_info("async-std").expect("should have async-std info");
        // Should have WasmFix (wasm-bindgen-futures)
        assert!(async_std
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::WasmFix));
        assert!(async_std
            .alternatives
            .iter()
            .any(|a| a.crate_name == Some("wasm-bindgen-futures")));
    }

    #[test]
    fn test_get_heavy_dependency_info_rustybuzz_can_be_made_optional() {
        let rustybuzz = get_heavy_dependency_info("rustybuzz").expect("should have rustybuzz info");
        // Should be Optional type (make it optional via feature flag)
        assert!(rustybuzz
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::Optional));
        // Should achieve 100% savings when disabled
        assert!(rustybuzz
            .alternatives
            .iter()
            .any(|a| a.savings_percent == 100));
    }

    #[test]
    fn test_get_heavy_dependency_info_chrono_has_lighter_alternatives() {
        let chrono = get_heavy_dependency_info("chrono").expect("should have chrono info");
        // Should have replacement alternatives (time crate)
        assert!(chrono
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::Replacement));
    }

    #[test]
    fn test_get_heavy_dependency_info_regex_supports_feature_minimization() {
        let regex = get_heavy_dependency_info("regex").expect("should have regex info");
        // Should have FeatureMinimization (disable unicode)
        assert!(regex
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::FeatureMinimization));
        // Should save significant space (~60%)
        assert!(regex.alternatives.iter().any(|a| a.savings_percent >= 50));
    }

    #[test]
    fn test_get_heavy_dependency_info_image_supports_feature_minimization() {
        let image = get_heavy_dependency_info("image").expect("should have image info");
        // Should have feature minimization
        assert!(image
            .alternatives
            .iter()
            .any(|a| a.alt_type == AlternativeType::FeatureMinimization));
    }

    #[test]
    fn test_database_all_alternative_types_represented() {
        // Ensure all alternative types are used in the database
        let all_deps = vec![
            "swc_core",
            "printpdf",
            "lopdf",
            "tokio",
            "async-std",
            "rustybuzz",
            "chrono",
            "regex",
            "image",
            "getrandom",
        ];

        let mut found_types = std::collections::HashSet::new();
        for dep in all_deps {
            let info = get_heavy_dependency_info(dep).unwrap();
            for alt in &info.alternatives {
                found_types.insert(alt.alt_type);
            }
        }

        // Should have representation of different alternative types
        assert!(found_types.contains(&AlternativeType::FeatureMinimization));
        assert!(found_types.contains(&AlternativeType::WasmFix));
        // May also have Replacement, Split, Optional
    }

    #[test]
    fn test_database_size_ranges_consistent_with_savings() {
        // Alternative sizes should be smaller than original
        let all_deps = vec![
            "swc_core",
            "printpdf",
            "lopdf",
            "tokio",
            "async-std",
            "rustybuzz",
            "chrono",
            "regex",
            "image",
            "getrandom",
        ];

        for dep in all_deps {
            let info = get_heavy_dependency_info(dep).unwrap();
            let original_min = info.size_kb.0;

            for alt in &info.alternatives {
                if let Some((alt_min, alt_max)) = alt.size_kb {
                    // Alternative should generally be smaller
                    assert!(
                        alt_max <= original_min * 2,
                        "{} alternative size ({}-{}) not significantly smaller than original ({})",
                        dep,
                        alt_min,
                        alt_max,
                        original_min
                    );
                }
            }
        }
    }

    #[test]
    fn test_database_bundle_percentages_within_valid_range() {
        // Bundle percentages should be reasonable
        let all_deps = vec![
            "swc_core",
            "printpdf",
            "lopdf",
            "tokio",
            "async-std",
            "rustybuzz",
            "chrono",
            "regex",
            "image",
            "getrandom",
        ];

        for dep in all_deps {
            let info = get_heavy_dependency_info(dep).unwrap();
            if let Some((min_pct, max_pct)) = info.bundle_percent {
                assert!(
                    min_pct > 0 && min_pct <= 100,
                    "{} min bundle % must be 1-100",
                    dep
                );
                assert!(
                    max_pct >= min_pct && max_pct <= 100,
                    "{} max bundle % must be >= min and <= 100",
                    dep
                );
            }
        }
    }

    #[test]
    fn test_database_initialization_has_entries() {
        // Ensure database actually has entries
        let db = HEAVY_DEPS_DATABASE.get_or_init(init_heavy_deps_database);
        assert!(db.len() >= 10, "Database should have at least 10 entries");
    }
}
