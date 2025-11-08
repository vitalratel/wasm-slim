//! Asset Scanning Benchmarks
//!
//! **Purpose:** Measure performance of asset detection and scanning operations
//!
//! **Baseline Metrics (2025-11-02, Rust 1.86, AMD Ryzen/Intel i7):**
//! - Real project scan (~50 files): ~10-20ms
//! - Small project (10 assets): ~5-10ms
//! - Medium project (50 assets): ~15-30ms
//! - Large project (200 assets): ~50-100ms
//!
//! **Regression Threshold:** >15% slower than baseline
//!
//! **How to Run:**
//! ```bash
//! cargo bench --bench asset_scanning
//! cargo bench --bench asset_scanning -- --save-baseline main
//! cargo bench --bench asset_scanning -- --baseline main
//! ```
//!
//! **What's Being Measured:**
//! 1. `scan project for assets` - Real project scanning (wasm-slim repo)
//! 2. `full asset detection workflow` - Complete workflow including initialization
//! 3. `scan small/medium/large project` - Synthetic workloads with known asset counts
//!
//! **Performance Tips:**
//! - Asset scanning is I/O bound - disk speed affects results
//! - Parser performance dominates for large Rust codebases
//! - Regex matching is optimized (no unicode tables)

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::path::PathBuf;
use wasm_slim::analyzer::AssetDetector;

fn bench_asset_scanning(c: &mut Criterion) {
    // Use the actual project as a benchmark target
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let detector = AssetDetector::new(&project_root);

    c.bench_function("scan project for assets", |b| {
        b.iter(|| {
            // Scan the project (should be <100ms for ~50 files)
            let _ = black_box(detector.scan_project());
        });
    });
}

fn bench_asset_detection_full_workflow(c: &mut Criterion) {
    // Benchmark the full asset detection workflow including result generation
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    c.bench_function("full asset detection workflow", |b| {
        b.iter(|| {
            let detector = black_box(AssetDetector::new(&project_root));
            let _ = black_box(detector.scan_project());
        });
    });
}

// P2-TEST-BENCH-001: Add realistic benchmark datasets

fn bench_realistic_small_project_assets(c: &mut Criterion) {
    // Small project: ~10 asset files (images, fonts)
    use std::fs;
    use tempfile::TempDir;

    c.bench_function("scan small project with 10 assets", |b| {
        b.iter_batched(
            || {
                let temp = TempDir::new().unwrap();
                let src = temp.path().join("src");
                fs::create_dir(&src).unwrap();

                // Create realistic asset files
                for i in 0..10 {
                    let file = src.join(format!("asset_{}.png", i));
                    fs::write(file, vec![0u8; 1024]).unwrap(); // 1KB each
                }
                temp
            },
            |temp| {
                let detector = black_box(AssetDetector::new(temp.path()));
                let _ = black_box(detector.scan_project());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_realistic_medium_project_assets(c: &mut Criterion) {
    // Medium project: ~50 asset files (typical for web apps)
    use std::fs;
    use tempfile::TempDir;

    c.bench_function("scan medium project with 50 assets", |b| {
        b.iter_batched(
            || {
                let temp = TempDir::new().unwrap();
                let assets = temp.path().join("assets");
                fs::create_dir(&assets).unwrap();

                // Mix of file types
                let extensions = ["png", "jpg", "svg", "woff2", "ttf", "ico"];
                for i in 0..50 {
                    let ext = extensions[i % extensions.len()];
                    let size = match ext {
                        "png" | "jpg" => 50 * 1024,    // 50KB
                        "woff2" | "ttf" => 100 * 1024, // 100KB
                        _ => 5 * 1024,                 // 5KB
                    };
                    let file = assets.join(format!("asset_{}.{}", i, ext));
                    fs::write(file, vec![0u8; size]).unwrap();
                }
                temp
            },
            |temp| {
                let detector = black_box(AssetDetector::new(temp.path()));
                let _ = black_box(detector.scan_project());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_realistic_large_project_assets(c: &mut Criterion) {
    // Large project: ~200 asset files (complex apps)
    use std::fs;
    use tempfile::TempDir;

    c.bench_function("scan large project with 200 assets", |b| {
        b.iter_batched(
            || {
                let temp = TempDir::new().unwrap();
                let assets = temp.path().join("assets");
                fs::create_dir(&assets).unwrap();

                // Create subdirectories
                let subdirs = ["images", "fonts", "icons", "data"];
                for dir in &subdirs {
                    fs::create_dir(assets.join(dir)).unwrap();
                }

                let extensions = ["png", "jpg", "svg", "woff2", "ttf", "json", "csv"];
                for i in 0..200 {
                    let ext = extensions[i % extensions.len()];
                    let subdir = subdirs[i % subdirs.len()];
                    let size = match ext {
                        "png" | "jpg" => 100 * 1024,
                        "woff2" | "ttf" => 150 * 1024,
                        "json" | "csv" => 20 * 1024,
                        _ => 10 * 1024,
                    };
                    let file = assets.join(subdir).join(format!("asset_{}.{}", i, ext));
                    fs::write(file, vec![0u8; size]).unwrap();
                }
                temp
            },
            |temp| {
                let detector = black_box(AssetDetector::new(temp.path()));
                let _ = black_box(detector.scan_project());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_asset_scanning,
    bench_asset_detection_full_workflow,
    bench_realistic_small_project_assets,
    bench_realistic_medium_project_assets,
    bench_realistic_large_project_assets
);
criterion_main!(benches);
