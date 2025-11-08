//! Twiggy Output Parsing Benchmarks
//!
//! **Purpose:** Measure performance of parsing twiggy WASM size profiler output
//!
//! **Baseline Metrics (2025-11-02, Rust 1.86, AMD Ryzen/Intel i7):**
//! - Small output (50 items): ~5-15μs
//! - Medium output (500 items): ~50-150μs  
//! - Large output (2000 items): ~200-600μs
//! - With comma formatting (200 items): ~30-80μs
//!
//! **Regression Threshold:** >20% slower than baseline
//!
//! **How to Run:**
//! ```bash
//! cargo bench --bench twiggy_parsing
//! cargo bench --bench twiggy_parsing -- --save-baseline main
//! cargo bench --bench twiggy_parsing -- --baseline main
//! ```
//!
//! **What's Being Measured:**
//! 1. `parse twiggy output (10 lines)` - Minimal real-world example
//! 2. `parse large twiggy output (1000 lines)` - Synthetic stress test
//! 3. `parse realistic small/medium/large project` - Real-world size distributions
//! 4. `parse with comma formatting` - Number parsing with thousand separators
//!
//! **Performance Notes:**
//! - Parsing is dominated by string splits and allocations
//! - Number parsing (with comma removal) is ~10-15% of total time
//! - Linear scaling with item count
//! - Realistic projects: small ~50 items, medium ~500, large ~2000

use criterion::{criterion_group, criterion_main, Criterion};
use std::fmt::Write as _;
use std::hint::black_box;

// Sample twiggy output for benchmarking
const SAMPLE_TWIGGY_OUTPUT: &str = r#"
 Shallow Bytes │ Shallow % │ Item
───────────────┼───────────┼──────────────────────────────────────────────────────────────────────────
        524288 ┊    50.00% ┊ data[0]
        262144 ┊    25.00% ┊ "function names" subsection
        131072 ┊    12.50% ┊ <wasm_bindgen::JsValue as core::convert::From<&str>>::from
         65536 ┊     6.25% ┊ wasm_bindgen::throw_str
         32768 ┊     3.12% ┊ <&T as core::fmt::Display>::fmt
         16384 ┊     1.56% ┊ core::fmt::write
          8192 ┊     0.78% ┊ <core::str::Utf8Error as core::fmt::Debug>::fmt
          4096 ┊     0.39% ┊ core::panicking::panic_fmt
          2048 ┊     0.19% ┊ rust_begin_unwind
          1024 ┊     0.09% ┊ __rust_alloc
           512 ┊     0.04% ┊ __rust_dealloc
"#;

fn bench_twiggy_output_parsing(c: &mut Criterion) {
    c.bench_function("parse twiggy output (10 lines)", |b| {
        b.iter(|| {
            // Simulate parsing twiggy output
            let lines: Vec<&str> = black_box(SAMPLE_TWIGGY_OUTPUT)
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.contains("─"))
                .collect();

            // Parse each line (simplified version of actual parsing)
            for line in lines {
                let parts: Vec<&str> = line.split('┊').collect();
                if parts.len() >= 3 {
                    let _size = parts[0].trim();
                    let _percentage = parts[1].trim();
                    let _name = parts[2].trim();
                }
            }
        });
    });
}

fn bench_large_twiggy_parsing(c: &mut Criterion) {
    // Create a larger synthetic dataset
    let mut large_output = String::with_capacity(100_000);
    for i in 0..1000 {
        writeln!(
            large_output,
            "       {} ┊     {:.2}% ┊ function_name_{}",
            1024 * i,
            100.0 / 1000.0,
            i
        )
        .unwrap();
    }

    c.bench_function("parse large twiggy output (1000 lines)", |b| {
        b.iter(|| {
            let lines: Vec<&str> = black_box(&large_output)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect();

            for line in lines {
                let parts: Vec<&str> = line.split('┊').collect();
                if parts.len() >= 3 {
                    let _size = parts[0].trim();
                    let _percentage = parts[1].trim();
                    let _name = parts[2].trim();
                }
            }
        });
    });
}

// P2-TEST-BENCH-001: Add realistic benchmark datasets

fn bench_realistic_small_project(c: &mut Criterion) {
    // Small project: ~50 functions, typical for simple WASM apps
    let mut output = String::with_capacity(10_000);
    output.push_str(" Shallow Bytes │ Shallow % │ Item\n");
    output.push_str("───────────────┼───────────┼────────\n");

    for i in 0..50 {
        writeln!(
            output,
            "       {:6} ┊     {:.2}% ┊ module::function_{}",
            50000 - i * 1000,
            (50000.0 - i as f64 * 1000.0) / 1000000.0 * 100.0,
            i
        )
        .unwrap();
    }

    c.bench_function("parse realistic small project (50 items)", |b| {
        b.iter(|| {
            let lines: Vec<&str> = black_box(&output)
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.contains("─"))
                .collect();

            for line in lines {
                let parts: Vec<&str> = line.split('┊').collect();
                if parts.len() >= 3 {
                    let _size = parts[0].trim().replace(",", "").parse::<u64>();
                    let _percentage = parts[1].trim().trim_end_matches('%').parse::<f64>();
                    let _name = parts[2].trim();
                }
            }
        });
    });
}

fn bench_realistic_medium_project(c: &mut Criterion) {
    // Medium project: ~500 functions, typical for moderate WASM apps
    let mut output = String::with_capacity(100_000);
    output.push_str(" Shallow Bytes │ Shallow % │ Item\n");
    output.push_str("───────────────┼───────────┼────────\n");

    // Simulate realistic distribution with some large items and many small ones
    for i in 0..500 {
        let size = if i < 20 {
            // Top 20 items are large (data segments, core functions)
            100000 - i * 4000
        } else {
            // Rest are smaller
            5000 - (i - 20) * 10
        };
        writeln!(
            output,
            "       {:6} ┊     {:.2}% ┊ crate_{}::module_{}::function_{}",
            size,
            size as f64 / 3000000.0 * 100.0,
            i / 100,
            (i / 10) % 10,
            i % 10
        )
        .unwrap();
    }

    c.bench_function("parse realistic medium project (500 items)", |b| {
        b.iter(|| {
            let lines: Vec<&str> = black_box(&output)
                .lines()
                .skip(2)
                .filter(|line| !line.trim().is_empty())
                .collect();

            for line in lines {
                let parts: Vec<&str> = line.split('┊').collect();
                if parts.len() >= 3 {
                    let _size = parts[0].trim().replace(",", "").parse::<u64>();
                    let _percentage = parts[1].trim().trim_end_matches('%').parse::<f64>();
                    let _name = parts[2].trim();
                }
            }
        });
    });
}

fn bench_realistic_large_project(c: &mut Criterion) {
    // Large project: ~2000 functions, typical for complex WASM apps
    let mut output = String::with_capacity(400_000);
    output.push_str(" Shallow Bytes │ Shallow % │ Item\n");
    output.push_str("───────────────┼───────────┼────────\n");

    for i in 0..2000 {
        let size = if i < 50 {
            200000 - i * 3000
        } else if i < 200 {
            50000 - (i - 50) * 200
        } else {
            10000 - (i - 200) * 5
        };
        let crate_name = if i % 3 == 0 {
            "std"
        } else if i % 3 == 1 {
            "core"
        } else {
            "app"
        };
        let module_name = if i % 7 == 0 {
            "fmt"
        } else if i % 7 == 1 {
            "collections"
        } else {
            "utils"
        };
        writeln!(
            output,
            "       {:6} ┊     {:.2}% ┊ {}::{}::fn_{}",
            size,
            size as f64 / 10000000.0 * 100.0,
            crate_name,
            module_name,
            i
        )
        .unwrap();
    }

    c.bench_function("parse realistic large project (2000 items)", |b| {
        b.iter(|| {
            let lines: Vec<&str> = black_box(&output)
                .lines()
                .skip(2)
                .filter(|line| !line.trim().is_empty())
                .collect();

            for line in lines {
                let parts: Vec<&str> = line.split('┊').collect();
                if parts.len() >= 3 {
                    let _size = parts[0].trim().replace(",", "").parse::<u64>();
                    let _percentage = parts[1].trim().trim_end_matches('%').parse::<f64>();
                    let _name = parts[2].trim();
                }
            }
        });
    });
}

fn bench_realistic_with_formatting(c: &mut Criterion) {
    // Test with realistic comma-formatted numbers
    let mut output = String::with_capacity(50_000);
    for i in 0..200 {
        let size = 1_234_567 - i * 5_000;
        // Manually format with commas for realistic test data
        let formatted_size = {
            let s = size.to_string();
            let mut result = String::new();
            for (i, c) in s.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    result.push(',');
                }
                result.push(c);
            }
            result.chars().rev().collect::<String>()
        };
        writeln!(
            output,
            "     {:>9} ┊     {:.2}% ┊ function_{}",
            formatted_size,
            size as f64 / 50_000_000.0 * 100.0,
            i
        )
        .unwrap();
    }

    c.bench_function("parse with comma formatting (200 items)", |b| {
        b.iter(|| {
            let lines: Vec<&str> = black_box(&output)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect();

            for line in lines {
                let parts: Vec<&str> = line.split('┊').collect();
                if parts.len() >= 3 {
                    let _size = parts[0].trim().replace(",", "").parse::<u64>();
                    let _percentage = parts[1].trim().trim_end_matches('%').parse::<f64>();
                    let _name = parts[2].trim();
                }
            }
        });
    });
}

criterion_group!(
    benches,
    bench_twiggy_output_parsing,
    bench_large_twiggy_parsing,
    bench_realistic_small_project,
    bench_realistic_medium_project,
    bench_realistic_large_project,
    bench_realistic_with_formatting
);
criterion_main!(benches);
