//! Recommendation engine for panic pattern optimization

use super::panics::{PanicPattern, PanicResults};

/// Generate actionable recommendations based on panic detection results
pub fn generate_recommendations(
    total: usize,
    by_pattern: &[(PanicPattern, usize)],
    size_kb: u64,
) -> Vec<String> {
    let mut recs = Vec::new();

    // Overall assessment
    if total > 100 {
        recs.push(format!(
            "[P0] Critical: {} panic sites detected (~{} KB). Significant WASM bloat.",
            total, size_kb
        ));
    } else if total > 50 {
        recs.push(format!(
            "[P1] High: {} panic sites detected (~{} KB). Consider refactoring hot paths.",
            total, size_kb
        ));
    } else if total > 10 {
        recs.push(format!(
            "[P2] Moderate: {} panic sites detected (~{} KB). Optimize critical sections.",
            total, size_kb
        ));
    } else {
        recs.push(format!(
            "[P3] Low: {} panic sites detected (~{} KB). Well optimized!",
            total, size_kb
        ));
        return recs; // No need for detailed recommendations
    }

    // Top offenders
    for (pattern, count) in by_pattern.iter().take(3) {
        if *count > 10 {
            let savings = (pattern.size_per_occurrence() * (*count as u64)) / 1024;
            recs.push(format!(
                "  → Replace {} {} calls with {} (save ~{} KB)",
                count,
                pattern.name(),
                pattern.alternative(),
                savings
            ));
        }
    }

    // Specific advice
    if total > 50 {
        recs.push(String::from(""));
        recs.push(String::from("Quick wins:"));
        recs.push(String::from(
            "  1. Use .get(i) instead of arr[i] for array access",
        ));
        recs.push(String::from(
            "  2. Replace .unwrap() with match or if let in hot paths",
        ));
        recs.push(String::from(
            "  3. Use .checked_div() for division operations",
        ));
        recs.push(String::from(
            "  4. Enable panic_immediate_abort in Cargo.toml (nightly)",
        ));
    }

    recs
}

/// Build complete panic results with recommendations
pub fn build_results(
    panic_sites: Vec<super::panics::DetectedPanic>,
) -> super::panics::PanicResults {
    let total_panics = panic_sites.len();

    // Count by pattern type
    let mut pattern_counts = std::collections::HashMap::new();
    let mut total_size: u64 = 0;

    for panic in &panic_sites {
        *pattern_counts.entry(panic.pattern.clone()).or_insert(0) += 1;
        total_size += panic.pattern.size_per_occurrence();
    }

    let mut by_pattern: Vec<(PanicPattern, usize)> = pattern_counts.into_iter().collect();
    by_pattern.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

    let estimated_size_kb = total_size / 1024;

    // Generate recommendations
    let recommendations = generate_recommendations(total_panics, &by_pattern, estimated_size_kb);

    PanicResults {
        total_panics,
        by_pattern,
        panic_sites,
        estimated_size_kb,
        recommendations,
    }
}
