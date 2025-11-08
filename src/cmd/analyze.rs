//! Analysis command implementations
//!
//! Provides various analysis modes:
//! - assets: Embedded asset detection
//! - deps: Dependency analysis with auto-fix
//! - bloat: Binary size analysis
//! - features: Feature flag analysis
//! - panics: Panic pattern detection (unwrap, indexing, division)
//! - top/dominators/dead/monos: WASM binary analysis with twiggy

use anyhow::{Context, Result};
use console::{style, Emoji};
use std::env;

use crate::analyzer;

static MICROSCOPE: Emoji = Emoji("🔍", ">>");
static WRENCH: Emoji = Emoji("🔧", "*");
static WARNING: Emoji = Emoji("⚠️", "!");

/// Main analyze command dispatcher
///
/// Routes to the appropriate analysis mode based on the mode parameter
///
/// # Examples
///
/// ```no_run
/// use wasm_slim::cmd::analyze::cmd_analyze;
///
/// // Analyze embedded assets
/// cmd_analyze(&None, "assets", false, false, false, false)?;
///
/// // Analyze dependencies with auto-fix
/// cmd_analyze(&None, "deps", true, false, false, false)?;
///
/// // Analyze binary bloat
/// cmd_analyze(&None, "bloat", false, false, false, false)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn cmd_analyze(
    file: &Option<String>,
    mode: &str,
    fix: bool,
    dry_run: bool,
    guide: bool,
    json: bool,
) -> Result<()> {
    match mode {
        "assets" => analyze_assets(guide, json),
        "deps" => analyze_dependencies(fix, dry_run, json),
        "bloat" => analyze_bloat(json),
        "features" => analyze_features(json),
        "panics" => analyze_panics(json),
        "top" | "dominators" | "dead" | "monos" => analyze_wasm_binary(file, mode, json),
        _ => {
            anyhow::bail!("Unknown analysis mode: {}. Valid modes: assets, deps, features, bloat, panics, top, dominators, dead, monos", mode);
        }
    }
}

/// Analyze embedded assets (Phase 5)
///
/// Scans the project for embedded assets (include_bytes!, include_str!, fonts, images)
/// and provides externalization recommendations
pub fn analyze_assets(guide: bool, json: bool) -> Result<()> {
    if !json {
        println!(
            "{} {} Asset Detection",
            MICROSCOPE,
            style("wasm-slim").bold()
        );
    }

    let project_root = env::current_dir()?;
    let detector = analyzer::AssetDetector::new(&project_root);
    let results = detector.scan_project()?;

    if json {
        analyzer::print_json_output(&results)?;
    } else {
        analyzer::print_asset_report(&results);
        if guide {
            analyzer::show_externalization_guide(&results);
        }
    }

    Ok(())
}

/// Analyze dependencies with optional auto-fix (Phase 4/4.5)
///
/// Analyzes dependency tree for:
/// - Heavy dependencies with lighter alternatives
/// - Unnecessary feature flags
/// - Optimization opportunities
///
/// Can automatically apply fixes with --fix flag
pub fn analyze_dependencies(fix: bool, dry_run: bool, json: bool) -> Result<()> {
    if !json {
        println!(
            "{} {} Dependency Analysis",
            MICROSCOPE,
            style("wasm-slim").bold()
        );
    }

    let project_root = env::current_dir()?;
    let analyzer = analyzer::DependencyAnalyzer::new(&project_root);
    let report = analyzer.analyze()?;

    if json {
        // Output JSON format
        let json_output = serde_json::to_string_pretty(&report)
            .context("Failed to serialize dependency report to JSON")?;
        println!("{}", json_output);
        return Ok(());
    }

    report.print_report();

    // Apply fixes if requested
    if fix || dry_run {
        println!(
            "\n{} {} Applying Optimizations",
            WRENCH,
            style("Auto-Fix").bold()
        );

        if dry_run {
            println!(
                "   {} {}",
                WARNING,
                style("[DRY RUN] No changes will be made").yellow()
            );
        }

        let applicator = analyzer::SuggestionApplicator::new(&project_root);
        let fixes_applied = applicator.apply_suggestions(&report, dry_run)?;

        if fixes_applied > 0 {
            if dry_run {
                println!(
                    "\n   {} Would apply {} fixes",
                    style("→").dim(),
                    style(fixes_applied).yellow().bold()
                );
                println!(
                    "   {} Run without --dry-run to apply changes",
                    style("💡").bold()
                );
            } else {
                println!(
                    "\n   {} Applied {} fixes successfully!",
                    style("✅").bold(),
                    style(fixes_applied).green().bold()
                );
                println!(
                    "   {} Run `cargo check` to verify changes",
                    style("💡").bold()
                );
            }
        } else {
            println!(
                "\n   {} No automatic fixes available for detected issues",
                style("ℹ️").bold()
            );
            println!(
                "   {} Some fixes require manual intervention (see suggestions above)",
                style("→").dim()
            );
        }
    }

    Ok(())
}

/// Analyze binary bloat (Phase 4.5)
///
/// Uses cargo-bloat to identify the largest code contributors
/// in the compiled binary
pub fn analyze_bloat(json: bool) -> Result<()> {
    if !json {
        println!(
            "{} {} Binary Size Analysis",
            MICROSCOPE,
            style("wasm-slim").bold()
        );
    }

    // Check if cargo-bloat is installed
    if !analyzer::BloatAnalyzer::check_installation()? {
        anyhow::bail!("cargo-bloat is not installed. Install with: cargo install cargo-bloat");
    }

    let project_root = env::current_dir()?;
    let bloat_analyzer = analyzer::BloatAnalyzer::new(&project_root);
    let results = bloat_analyzer.analyze()?;

    if json {
        let json_output = analyzer::format_bloat_json(&results)?;
        println!("{}", json_output);
    } else {
        let report = analyzer::format_bloat_console(&results)?;
        print!("{}", report);
    }

    Ok(())
}

/// Analyze feature flags (Phase 4.5)
///
/// Analyzes Cargo feature flag usage and identifies:
/// - Unused features
/// - Feature flag combinations
/// - Optimization opportunities
pub fn analyze_features(json: bool) -> Result<()> {
    if !json {
        println!(
            "{} {} Feature Flag Analysis",
            MICROSCOPE,
            style("wasm-slim").bold()
        );
    }

    let project_root = env::current_dir()?;
    let feature_analyzer = analyzer::FeatureAnalyzer::new(&project_root);
    let results = feature_analyzer.analyze()?;

    if json {
        let json_output = analyzer::format_feature_json(&results)?;
        println!("{}", json_output);
    } else {
        let report = analyzer::format_feature_console(&results)?;
        print!("{}", report);
    }

    Ok(())
}

/// Analyze panic patterns (Rust WASM book optimization)
///
/// Scans the project for panic-inducing code patterns that bloat WASM:
/// - .unwrap() and .expect() calls
/// - Array indexing arr\[i\] (vs .get(i))
/// - Division operators / and % (vs .checked_div())
/// - panic!() and assert!() macros
///
/// Each panic site adds 500-2000 bytes to the WASM binary.
pub fn analyze_panics(json: bool) -> Result<()> {
    if !json {
        println!(
            "{} {} Panic Pattern Analysis",
            MICROSCOPE,
            style("wasm-slim").bold()
        );
        println!();
    }

    let project_root = env::current_dir()?;
    let detector = analyzer::PanicDetector::new(&project_root);
    let results = detector.scan_project()?;

    if json {
        analyzer::print_panic_json(&results)?;
    } else {
        analyzer::print_panic_report(&results);
    }

    Ok(())
}

/// Analyze WASM binary with twiggy (Phase 6)
///
/// Uses the twiggy profiler to analyze WASM binaries with different modes:
/// - top: Show largest code contributors
/// - dominators: Show dominator tree analysis
/// - dead: Identify dead code
/// - monos: Analyze monomorphizations
pub fn analyze_wasm_binary(file: &Option<String>, mode: &str, json: bool) -> Result<()> {
    let f = file.as_ref().ok_or_else(|| {
        anyhow::anyhow!("WASM file required for binary analysis mode (top/dominators/dead/monos)")
    })?;

    // Check if twiggy is installed
    if !analyzer::TwiggyAnalyzer::check_installation()? {
        eprintln!(
            "{}",
            style(analyzer::TwiggyAnalyzer::installation_instructions()).yellow()
        );
        anyhow::bail!("twiggy not installed");
    }

    if !json {
        println!(
            "{} {} WASM Binary Analysis",
            MICROSCOPE,
            style("wasm-slim").bold()
        );
        println!("   File: {}", f);
        println!("   Mode: {}", mode);
        println!();
    }

    let analysis_mode = match mode {
        "top" => analyzer::AnalysisMode::Top,
        "dominators" => analyzer::AnalysisMode::Dominators,
        "dead" => analyzer::AnalysisMode::Dead,
        "monos" => analyzer::AnalysisMode::Monos,
        _ => unreachable!(),
    };

    let wasm_analyzer = analyzer::TwiggyAnalyzer::new(f);
    let results = wasm_analyzer.analyze(analysis_mode)?;

    if json {
        let json_output = serde_json::to_string_pretty(&results)?;
        println!("{}", json_output);
    } else {
        analyzer::print_analysis_report(&results);
    }

    Ok(())
}
