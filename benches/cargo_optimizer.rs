//! Cargo.toml Optimizer Benchmarks
//!
//! **Purpose:** Measure performance of Cargo.toml optimization operations
//!
//! **Baseline Metrics (2025-11-03, Rust 1.75):**
//! - Single Cargo.toml optimization: ~1-5ms
//! - Workspace optimization (10 crates): ~10-30ms
//! - Parse + modify + write cycle: ~2-8ms
//!
//! **Regression Threshold:** >20% slower than baseline
//!
//! **How to Run:**
//! ```bash
//! cargo bench --bench cargo_optimizer
//! cargo bench --bench cargo_optimizer -- --save-baseline main
//! ```
//!
//! **What's Being Measured:**
//! 1. `optimize single cargo toml` - Single file optimization
//! 2. `optimize workspace cargo tomls` - Multiple files in workspace
//! 3. `dry run optimization` - Read-only operations
//!
//! **Performance Tips:**
//! - TOML parsing dominates (toml_edit preserves formatting)
//! - File I/O is second bottleneck
//! - Dry-run mode should be faster (no writes)

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use wasm_slim::optimizer::{CargoTomlEditor, OptimizationConfig};

fn create_test_cargo_toml(dir: &TempDir) -> PathBuf {
    let cargo_toml = dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = "1"
wasm-bindgen = "0.2"

[profile.release]
opt-level = 3
"#,
    )
    .unwrap();
    cargo_toml
}

fn bench_optimize_single_cargo_toml(c: &mut Criterion) {
    c.bench_function("optimize single cargo toml", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let cargo_toml = create_test_cargo_toml(&temp_dir);
                (temp_dir, cargo_toml)
            },
            |(_temp_dir, cargo_toml)| {
                let editor = CargoTomlEditor::new();
                let config = OptimizationConfig::default();
                black_box(editor.optimize_cargo_toml(&cargo_toml, &config, None, false))
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_optimize_workspace(c: &mut Criterion) {
    c.bench_function("optimize workspace cargo tomls", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                // Create 5 crates in workspace
                let mut tomls = Vec::new();
                for i in 0..5 {
                    let crate_dir = temp_dir.path().join(format!("crate{}", i));
                    fs::create_dir(&crate_dir).unwrap();
                    let toml = crate_dir.join("Cargo.toml");
                    fs::write(
                        &toml,
                        format!(
                            r#"
[package]
name = "crate{}"
version = "0.1.0"
edition = "2021"

[profile.release]
opt-level = 2
"#,
                            i
                        ),
                    )
                    .unwrap();
                    tomls.push(toml);
                }
                (temp_dir, tomls)
            },
            |(_temp_dir, tomls)| {
                let editor = CargoTomlEditor::new();
                let config = OptimizationConfig::default();
                for toml in &tomls {
                    let _ = black_box(editor.optimize_cargo_toml(toml, &config, None, false));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_dry_run_optimization(c: &mut Criterion) {
    c.bench_function("dry run optimization", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let cargo_toml = create_test_cargo_toml(&temp_dir);
                (temp_dir, cargo_toml)
            },
            |(_temp_dir, cargo_toml)| {
                let editor = CargoTomlEditor::new();
                let config = OptimizationConfig::default();
                black_box(editor.optimize_cargo_toml(&cargo_toml, &config, None, true))
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_optimize_single_cargo_toml,
    bench_optimize_workspace,
    bench_dry_run_optimization
);
criterion_main!(benches);
