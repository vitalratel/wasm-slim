# Performance Tracking Infrastructure

This document describes the performance tracking system for wasm-slim benchmarks, including baseline management, regression detection, and CI integration.

**Related Documentation:**
- [BENCHMARKS.md](BENCHMARKS.md) - Running and writing performance benchmarks
- [PERFORMANCE.md](PERFORMANCE.md) - Runtime performance characteristics and optimization

## Overview

The performance tracking system provides:

- **Baseline tracking**: Store and manage benchmark baselines
- **Regression detection**: Automatically detect performance regressions
- **Performance budgets**: Set thresholds for acceptable performance changes
- **CI integration**: Ready for continuous integration workflows
- **Historical tracking**: Track performance over time with git commits

## Architecture

### Components

1. **`bench_tracker` module** (`src/bench_tracker.rs`): Core tracking infrastructure
   - `BenchmarkTracker`: Main tracking manager
   - `BenchmarkBaseline`: Baseline storage format
   - `BenchmarkComparison`: Comparison result
   - `PerformanceBudget`: Threshold configuration

2. **`bench-tracker` binary** (`src/bin/bench-tracker.rs`): CLI tool for benchmark management
   - Run benchmarks and detect regressions
   - Save and manage baselines
   - Compare results

3. **Baseline storage**: `.wasm-slim/benchmarks/baseline.json`
   - Stores benchmark results
   - Tracks version and git commit
   - JSON format for easy inspection

## Usage

### Quick Start

```bash
# 1. Run benchmarks and establish baseline
cargo bench
cargo run --bin bench-tracker baseline --version "v0.1.0"

# 2. Make changes to your code

# 3. Run benchmarks and check for regressions
cargo run --bin bench-tracker run

# 4. If no regressions, update the baseline
cargo run --bin bench-tracker baseline --version "v0.1.1"
```

### Commands

#### `bench-tracker run`

Run benchmarks and compare with baseline.

```bash
# Run all benchmarks with default settings (10% regression threshold)
cargo run --bin bench-tracker run

# Run with custom regression threshold
cargo run --bin bench-tracker run --max-regression 5.0

# Fail CI build on regression
cargo run --bin bench-tracker run --fail-on-regression

# Run specific benchmark
cargo run --bin bench-tracker run --bench asset_scanning
```

**Output:**
```
🔧 Running benchmarks...
✓ Benchmarks completed
✓ Parsed 6 benchmark results

📊 Performance Comparison
================================================================================
Benchmark                                  Baseline      Current       Change
--------------------------------------------------------------------------------
🟢 scan project for assets                 45.23 µs      42.11 µs      -6.89%
⚪ full asset detection workflow           48.76 µs      49.01 µs      +0.51%
🟢 analyze dependencies                    123.45 ms     115.23 ms     -6.66%
⚪ parse cargo metadata                    98.12 ms      99.45 ms      +1.36%
🟢 parse twiggy output (10 lines)          2.34 µs       2.21 µs       -5.56%
⚪ parse large twiggy output (1000 lines)  145.67 µs     148.23 µs     +1.76%
================================================================================

✓ No significant performance regressions
```

**Status indicators:**
- 🟢 Improvement (>5% faster)
- ⚪ Neutral (<threshold change)
- 🔴 Regression (>threshold slower)

#### `bench-tracker baseline`

Save current benchmark results as baseline.

```bash
# Save with version tag
cargo run --bin bench-tracker baseline --version "v0.1.0"

# Save with git commit (auto-detected)
cargo run --bin bench-tracker baseline --version "post-optimization"
```

**Output:**
```
💾 Saving baseline...
✓ Saved baseline to .wasm-slim/benchmarks/baseline.json
✓ Baseline saved successfully
```

#### `bench-tracker compare`

Compare existing results with baseline without running benchmarks.

```bash
cargo run --bin bench-tracker compare
```

Useful for checking results after running `cargo bench` separately.

#### `bench-tracker show`

Display current baseline information.

```bash
cargo run --bin bench-tracker show
```

**Output:**
```
📊 Current Baseline
Version: v0.1.0
Timestamp: 2 hours ago
Git commit: 7e75719abc123...

Benchmarks (6):
  • scan project for assets - 45.23 µs (±1.23 µs)
  • full asset detection workflow - 48.76 µs (±2.01 µs)
  • analyze dependencies - 123.45 ms (±5.67 ms)
  • parse cargo metadata - 98.12 ms (±3.45 ms)
  • parse twiggy output (10 lines) - 2.34 µs (±0.12 µs)
  • parse large twiggy output (1000 lines) - 145.67 µs (±6.78 µs)
```

#### `bench-tracker reset`

Delete current baseline.

```bash
cargo run --bin bench-tracker reset
```

### Performance Budgets

Performance budgets define acceptable performance thresholds.

**Configuration options:**

```rust
use wasm_slim::bench_tracker::PerformanceBudget;

let budget = PerformanceBudget {
    max_regression_percent: 10.0,  // Max 10% slower
    max_time_ns: Some(100_000_000), // Max 100ms absolute
    fail_on_violation: true,         // Fail CI on violation
};
```

**Budget behavior:**
- `max_regression_percent`: Percentage slower than baseline (e.g., 10.0 = 10%)
- `max_time_ns`: Absolute time limit in nanoseconds (optional)
- `fail_on_violation`: Whether to exit with error code on violation

## CI Integration

### GitHub Actions Example

```yaml
name: Performance Testing

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-git-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache target directory
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-target-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Cache baseline
        uses: actions/cache@v3
        with:
          path: .wasm-slim/benchmarks
          key: benchmark-baseline-${{ github.ref }}
          restore-keys: |
            benchmark-baseline-refs/heads/main
      
      - name: Run benchmarks
        run: |
          cargo bench
          cargo run --bin bench-tracker run --fail-on-regression --max-regression 10.0
      
      - name: Update baseline (on main)
        if: github.ref == 'refs/heads/main'
        run: |
          cargo run --bin bench-tracker baseline --version "${{ github.sha }}"
      
      - name: Upload benchmark results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: target/criterion
```

### Key CI features:

1. **Baseline caching**: Restores baseline from main branch for PRs
2. **Regression detection**: Fails PR if performance degrades >10%
3. **Baseline updates**: Updates baseline on main branch commits
4. **Result artifacts**: Uploads full criterion results for inspection

## Baseline Storage Format

Baselines are stored in `.wasm-slim/benchmarks/baseline.json`:

```json
{
  "version": "v0.1.0",
  "timestamp": 1730491200,
  "git_commit": "7e75719abc123...",
  "results": {
    "scan project for assets": {
      "name": "scan project for assets",
      "mean_ns": 45230,
      "stddev_ns": 1230,
      "min_ns": 45230,
      "max_ns": 45230,
      "iterations": 100,
      "timestamp": 1730491200
    }
  }
}
```

**Fields:**
- `version`: User-defined version/tag
- `timestamp`: Unix timestamp when baseline was created
- `git_commit`: Git commit hash (auto-detected)
- `results`: Map of benchmark name to result data

## Best Practices

### 1. Establish Baseline Early

```bash
# After initial benchmark implementation
cargo bench
cargo run --bin bench-tracker baseline --version "initial"
```

### 2. Regular Baseline Updates

Update baselines after:
- Intentional optimizations
- Major refactoring
- Version releases

```bash
cargo run --bin bench-tracker baseline --version "v0.2.0"
```

### 3. PR Workflow

For pull requests:
1. Run benchmarks against main branch baseline
2. Review any regressions
3. Document intentional performance changes
4. Only update baseline after merge

### 4. Set Appropriate Thresholds

- **Conservative** (5%): Critical performance paths
- **Standard** (10%): Most code
- **Relaxed** (20%): Non-critical paths

### 5. Track Historical Performance

Keep baseline history in git:

```bash
# Add baseline to git
git add .wasm-slim/benchmarks/baseline.json
git commit -m "chore: update performance baseline"
```

## Troubleshooting

### No criterion results found

**Problem:** `bench-tracker` can't find benchmark results.

**Solution:** Run benchmarks first:
```bash
cargo bench
cargo run --bin bench-tracker baseline
```

### Inconsistent results

**Problem:** Benchmark results vary significantly between runs.

**Solutions:**
- Close resource-intensive applications
- Run on same machine/environment
- Increase criterion sample size in benchmark code
- Use dedicated CI runners for consistent environment

### False positive regressions

**Problem:** Small performance variations trigger false alarms.

**Solutions:**
- Increase `max_regression_percent` threshold
- Run benchmarks multiple times
- Check for system load during benchmarks
- Use statistical significance (criterion's built-in analysis)

## Integration with Existing Benchmarks

The tracker automatically works with all criterion benchmarks:

```rust
// benches/my_benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn my_benchmark(c: &mut Criterion) {
    c.bench_function("my operation", |b| {
        b.iter(|| {
            // Your code here
        });
    });
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
```

No changes needed - `bench-tracker` automatically discovers and tracks all criterion benchmarks.

## Programmatic Usage

Use the tracking infrastructure in your own tools:

```rust
use wasm_slim::bench_tracker::{BenchmarkTracker, PerformanceBudget};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let project_root = Path::new(".");
    
    // Create tracker with budget
    let budget = PerformanceBudget {
        max_regression_percent: 5.0,
        max_time_ns: None,
        fail_on_violation: true,
    };
    let tracker = BenchmarkTracker::with_budget(project_root, budget);
    
    // Load baseline
    let baseline = tracker.load_baseline()?.expect("No baseline found");
    
    // Parse current results
    let criterion_dir = project_root.join("target/criterion");
    let current = tracker.parse_criterion_results(&criterion_dir)?;
    
    // Compare
    let comparisons = tracker.compare_with_baseline(&current, &baseline);
    tracker.print_comparison(&comparisons);
    
    // Check for regressions
    if tracker.has_regressions(&comparisons) {
        eprintln!("Performance regressions detected!");
        std::process::exit(1);
    }
    
    Ok(())
}
```

## Future Enhancements

Planned improvements:
- Historical trend visualization
- Multiple baseline comparison
- Benchmark result export (CSV, JSON)
- Integration with benchmark dashboards
- Automatic baseline selection (by git tag/branch)
- Performance regression bisection

## See Also

- **[BENCHMARKS.md](BENCHMARKS.md)** - Complete guide to running and writing benchmarks
- **[PERFORMANCE.md](PERFORMANCE.md)** - Runtime performance characteristics and optimization
- **[Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)** - Benchmarking framework documentation
- **[GitHub Actions Cache](https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows)** - CI caching for faster builds

---

**Last Updated:** 2025-11-08
