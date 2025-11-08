//! Backup Operations Benchmarks
//!
//! **Purpose:** Measure performance of backup creation and restoration
//!
//! **Baseline Metrics (2025-11-03, Rust 1.86):**
//! - Single backup creation: ~1-3ms (depends on file size)
//! - Backup restoration: ~1-2ms
//! - List backups: <1ms
//!
//! **Regression Threshold:** >20% slower than baseline
//!
//! **How to Run:**
//! ```bash
//! cargo bench --bench backup_operations
//! ```
//!
//! **What's Being Measured:**
//! 1. `create backup` - File copy to backup directory
//! 2. `concurrent backup creation` - Multiple files backed up sequentially
//!
//! **Performance Notes:**
//! - File I/O is the primary bottleneck
//! - Backup creation includes timestamp formatting
//! - List operation depends on number of existing backups

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use std::hint::black_box;
use tempfile::TempDir;
use wasm_slim::optimizer::backup::BackupManager;

fn create_test_file(dir: &TempDir, name: &str, size_kb: usize) -> std::path::PathBuf {
    let file = dir.path().join(name);
    let content = vec![0u8; size_kb * 1024];
    fs::write(&file, content).unwrap();
    file
}

fn bench_create_backup(c: &mut Criterion) {
    let mut group = c.benchmark_group("create backup");

    for size_kb in [1, 10, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            size_kb,
            |b, &size_kb| {
                b.iter_batched(
                    || {
                        let temp_dir = TempDir::new().unwrap();
                        let manager = BackupManager::new(temp_dir.path());
                        let test_file = create_test_file(&temp_dir, "test.toml", size_kb);
                        (temp_dir, manager, test_file)
                    },
                    |(temp_dir, manager, test_file)| {
                        black_box(manager.create_backup(&test_file)).unwrap();
                        drop(temp_dir);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn bench_concurrent_backup_creation(c: &mut Criterion) {
    c.bench_function("concurrent backup creation (3 files)", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let manager = BackupManager::new(temp_dir.path());
                let files = vec![
                    create_test_file(&temp_dir, "file1.toml", 5),
                    create_test_file(&temp_dir, "file2.toml", 5),
                    create_test_file(&temp_dir, "file3.toml", 5),
                ];
                (temp_dir, manager, files)
            },
            |(temp_dir, manager, files)| {
                for file in &files {
                    let _ = black_box(manager.create_backup(file));
                }
                drop(temp_dir);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_create_backup,
    bench_concurrent_backup_creation
);
criterion_main!(benches);
