//! Config File Parsing Benchmarks
//!
//! **Purpose:** Measure performance of configuration file loading and validation
//!
//! **Baseline Metrics (2025-11-03, Rust 1.86):**
//! - Config load + parse: ~0.5-2ms
//! - Template resolution: ~0.1-0.5ms
//! - Config validation: <0.1ms
//!
//! **Regression Threshold:** >20% slower than baseline
//!
//! **How to Run:**
//! ```bash
//! cargo bench --bench config_parsing
//! ```
//!
//! **What's Being Measured:**
//! 1. `load config from file` - File I/O + TOML parsing
//! 2. `resolve template` - Template lookup and merging
//! 3. `validate config` - Budget validation logic
//!
//! **Performance Notes:**
//! - TOML parsing uses toml_edit (preserves formatting)
//! - Template resolution involves HashMap lookup
//! - Validation is primarily arithmetic checks

use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use std::hint::black_box;
use tempfile::TempDir;
use wasm_slim::config::{ConfigLoader, Template, TemplateResolver};

fn create_config_file(dir: &TempDir, template: &str) -> std::path::PathBuf {
    let config_path = dir.path().join(".wasm-slim.toml");
    fs::write(
        &config_path,
        format!(
            r#"
template = "{}"

[size-budget]
target-size-kb = 500
warn-threshold-kb = 750
max-size-kb = 1000

[ci-integration]
enable-history = true
max-history-entries = 50
regression-threshold = 5.0
"#,
            template
        ),
    )
    .unwrap();
    config_path
}

fn bench_load_config(c: &mut Criterion) {
    c.bench_function("load config from file", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                create_config_file(&temp_dir, "balanced");
                temp_dir
            },
            |temp_dir| {
                black_box(ConfigLoader::load(temp_dir.path())).unwrap();
                drop(temp_dir);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_resolve_template(c: &mut Criterion) {
    c.bench_function("resolve template", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                create_config_file(&temp_dir, "balanced");
                let config = ConfigLoader::load(temp_dir.path()).unwrap();
                (temp_dir, config)
            },
            |(temp_dir, config)| {
                black_box(TemplateResolver::resolve(&config)).unwrap();
                drop(temp_dir);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_validate_config(c: &mut Criterion) {
    c.bench_function("validate config", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                create_config_file(&temp_dir, "balanced");
                let config = ConfigLoader::load(temp_dir.path()).unwrap();
                (temp_dir, config)
            },
            |(temp_dir, config)| {
                if let Some(ref budget) = config.size_budget {
                    black_box(budget.validate()).unwrap();
                }
                drop(temp_dir);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_load_and_resolve(c: &mut Criterion) {
    c.bench_function("load and resolve config (full workflow)", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                create_config_file(&temp_dir, "balanced");
                temp_dir
            },
            |temp_dir| {
                let config = ConfigLoader::load(temp_dir.path()).unwrap();
                let template = black_box(TemplateResolver::resolve(&config)).unwrap();
                black_box(template);
                drop(temp_dir);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_template_library_lookup(c: &mut Criterion) {
    c.bench_function("template library lookup", |b| {
        b.iter(|| {
            black_box(Template::get("balanced"));
            black_box(Template::get("aggressive"));
            black_box(Template::get("minimal"));
        });
    });
}

criterion_group!(
    benches,
    bench_load_config,
    bench_resolve_template,
    bench_validate_config,
    bench_load_and_resolve,
    bench_template_library_lookup
);
criterion_main!(benches);
