# wasm-slim Benchmark Guide

Comprehensive guide to running, interpreting, and tracking performance benchmarks for wasm-slim.

## Quick Start

### 5-Minute Setup

```bash
# 1. Run all benchmarks
cargo bench

# 2. Create baseline for tracking
cargo run --bin bench-tracker baseline --version "v0.1.0"

# 3. Make changes, then check for regressions
cargo run --bin bench-tracker run
```

### Common Commands

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench asset_scanning
cargo bench --bench dependency_analysis
cargo bench --bench twiggy_parsing
cargo bench --bench cargo_optimizer
cargo bench --bench backup_operations
cargo bench --bench config_parsing

# Run specific test within a benchmark
cargo bench --bench asset_scanning "scan project"

# Criterion baseline management
cargo bench -- --save-baseline main
cargo bench -- --baseline main

# bench-tracker tool
cargo run --bin bench-tracker run              # Check for regressions
cargo run --bin bench-tracker show             # View current baseline
cargo run --bin bench-tracker baseline         # Save new baseline
```

## Available Benchmarks

### 1. Asset Scanning (`asset_scanning.rs`) ✅

Benchmarks the asset detection system:
- `scan project for assets` - Full project scan for embedded assets
- `full asset detection workflow` - Complete workflow including initialization

**Target performance:** <100ms for ~50 files

**Datasets:**
- Small (10 assets): Simple web apps - PNG images (1KB each)
- Medium (50 assets): Typical web apps - PNG, JPG, SVG, WOFF2, TTF, ICO
- Large (200 assets): Complex apps - Organized in subdirectories

### 2. Dependency Analysis (`dependency_analysis.rs`) ✅

Benchmarks dependency analysis features:
- `analyze dependencies` - Full dependency tree analysis
- `parse cargo metadata` - Cargo metadata parsing overhead

**Target performance:** <200ms for typical projects

**Note:** Dominated by cargo subprocess spawn time (~80-100ms baseline)

**Datasets:**
- Small (8 deps): Minimal CLI tools - clap, serde, anyhow
- Medium (25 deps): Web services - ~150 transitive dependencies
- Large (45 deps): Full-stack apps - 300+ transitive dependencies

### 3. Twiggy Parsing (`twiggy_parsing.rs`) ✅

Benchmarks WASM analysis output parsing:
- `parse twiggy output (10 lines)` - Small output parsing
- `parse large twiggy output (1000 lines)` - Large output stress test

**Target performance:** <1µs per line

**Note:** Linear scaling with item count, dominated by string operations

### 4. Cargo Optimizer (`cargo_optimizer.rs`) ✅

Benchmarks Cargo.toml optimization operations:
- `optimize single cargo toml` - Single file optimization
- `optimize workspace cargo tomls` - Multiple files (5 crates)
- `dry run optimization` - Read-only mode performance

**Target performance:** <5ms per file

### 5. Backup Operations (`backup_operations.rs`) ✅

Benchmarks backup/restore functionality:
- `create backup` - Backup creation (1KB, 10KB, 100KB files)
- `restore backup` - Backup restoration
- `list backups` - Directory scanning (1, 5, 20 backups)
- `concurrent backup creation` - Multiple files

**Target performance:** <3ms per backup (10KB file)

### 6. Config Parsing (`config_parsing.rs`) ✅

Benchmarks configuration file operations:
- `load config from file` - File I/O + TOML parsing
- `resolve template` - Template lookup and merging
- `validate config` - Budget validation
- `load and resolve config` - Full workflow
- `template library lookup` - HashMap access

**Target performance:** <2ms for full workflow

## Interpreting Benchmark Results

### Understanding Criterion Output

```
scan project for assets
                        time:   [44.234 µs 45.123 µs 46.011 µs]
                        change: [-2.3451% -1.2345% -0.1234%] (improvement)
```

**Key Components:**
- **time:** [lower bound, estimate, upper bound] with 95% confidence
- **change:** Percentage change from previous run
- **Mean:** Average time per iteration
- **Median:** Middle value (more robust to outliers)
- **Std Dev:** Measure of variability

**Key Indicators:**
- **Green:** Improvement
- **Red:** Regression
- **± values:** Confidence interval (usually 95%)

**What to Look For:**
- Changes >10%: Likely significant (investigate)
- Changes <5%: Often noise (unless consistent)
- Wide confidence intervals: Unstable benchmark (fix environment)

### bench-tracker Output

```
📊 Performance Comparison
================================================================================
Benchmark                                    Baseline      Current       Change
--------------------------------------------------------------------------------
🟢 scan project for assets                   50.23 µs     45.11 µs       -10.19%
⚪ full asset detection workflow             53.76 µs     54.01 µs        +0.47%
🔴 analyze dependencies                     120.45 ms    135.23 ms       +12.27%
================================================================================
```

- 🟢 Green = Improvement (>5% faster)
- ⚪ White = No significant change
- 🔴 Red = Regression (>10% slower)

## Performance Budgets

Current performance budgets (maximum acceptable times):

| Benchmark | Budget | Current |
|-----------|--------|---------|
| Asset scanning | 100ms | ~45µs ✓ |
| Dependency analysis | 200ms | ~123ms ✓ |
| Twiggy parsing (10 lines) | 10µs | ~2.3µs ✓ |
| Twiggy parsing (1000 lines) | 1ms | ~145µs ✓ |
| Cargo optimizer | 5ms | ~2ms ✓ |
| Backup operations (10KB) | 3ms | ~1.5ms ✓ |
| Config parsing | 2ms | ~800µs ✓ |

Regression threshold: **10%** (configurable)

## Performance Tracking with bench-tracker

### Typical Workflow

#### Daily Development
```bash
# After making changes
cargo bench
cargo run --bin bench-tracker compare
```

#### Before Commit
```bash
# Ensure no regressions
cargo run --bin bench-tracker run --max-regression 10.0
```

#### After Release
```bash
# Update baseline
cargo run --bin bench-tracker baseline --version "v1.0.0"
git add .bench-tracker/baseline.json
git commit -m "chore: update performance baseline for v1.0.0"
```

### bench-tracker Commands

```bash
# View current baseline
cargo run --bin bench-tracker show

# Run with custom threshold
cargo run --bin bench-tracker run --max-regression 15.0

# Fail build on regression (for CI)
cargo run --bin bench-tracker run --fail-on-regression

# Reset baseline
cargo run --bin bench-tracker reset
```

See [PERFORMANCE_TRACKING.md](PERFORMANCE_TRACKING.md) for complete documentation.

## Baseline Management (Criterion)

Criterion provides its own baseline management independent of bench-tracker:

### Save Baseline

```bash
# Save baseline for all benchmarks
cargo bench -- --save-baseline main

# Save baseline for specific benchmark
cargo bench --bench asset_scanning -- --save-baseline v0.1.0
```

### Compare Against Baseline

```bash
# Compare current code against 'main' baseline
cargo bench -- --baseline main

# Compare against specific version
cargo bench -- --baseline v0.1.0
```

### List Saved Baselines

```bash
ls target/criterion/*/base/
```

### View HTML Reports

```bash
# Criterion generates detailed HTML reports
open target/criterion/<benchmark-name>/report/index.html
```

## Adding New Benchmarks

### 1. Create Benchmark File

```rust
// benches/my_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wasm_slim::my_module::MyType;

fn bench_my_operation(c: &mut Criterion) {
    c.bench_function("my operation", |b| {
        b.iter(|| {
            // Your code to benchmark
            black_box(my_operation());
        });
    });
}

criterion_group!(benches, bench_my_operation);
criterion_main!(benches);
```

### 2. Add to `Cargo.toml`

```toml
[[bench]]
name = "my_benchmark"
harness = false
```

### 3. Run and Establish Baseline

```bash
cargo bench --bench my_benchmark
cargo run --bin bench-tracker baseline
```

### 4. Document

Add to this file:
- Purpose of the benchmark
- Target performance
- What datasets/scenarios are tested
- Notes on performance characteristics

## Guidelines for Writing Benchmarks

### Test Data Selection

**Good test data characteristics:**

1. **Realistic**: Match production workloads
   ```rust
   // ✓ Good: Realistic Cargo.toml with ~20 dependencies
   let cargo_toml = include_str!("../test-fixtures/realistic-cargo.toml");
   
   // ✗ Bad: Minimal test case that doesn't represent real usage
   let cargo_toml = "[package]\nname = \"test\"";
   ```

2. **Size variability**: Test multiple sizes
   ```rust
   c.bench_function("parse small file (10 lines)", |b| { /* ... */ });
   c.bench_function("parse medium file (100 lines)", |b| { /* ... */ });
   c.bench_function("parse large file (1000 lines)", |b| { /* ... */ });
   ```

3. **Edge cases**: Include boundary conditions
   ```rust
   // Empty input
   c.bench_function("scan empty project", |b| { /* ... */ });
   
   // Maximum supported size
   c.bench_function("scan project with 1000 files", |b| { /* ... */ });
   ```

### Benchmark Organization

**Group related benchmarks:**

```rust
fn bench_optimization_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cargo_optimization");
    
    // Configure sampling
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("optimize single file", |b| { /* ... */ });
    group.bench_function("optimize workspace", |b| { /* ... */ });
    group.bench_function("dry run mode", |b| { /* ... */ });
    
    group.finish();
}
```

### Common Pitfalls to Avoid

1. **Don't benchmark I/O without context**
   ```rust
   // ✗ Bad: I/O dominates, optimization not measured
   b.iter(|| {
       let content = fs::read_to_string("file.txt").unwrap();
       parse(content)
   });
   
   // ✓ Good: Separate I/O from parsing
   let content = fs::read_to_string("file.txt").unwrap();
   b.iter(|| parse(black_box(&content)));
   ```

2. **Don't forget black_box for returned values**
   ```rust
   // ✗ Bad: Compiler may optimize away entire function
   b.iter(|| expensive_computation());
   
   // ✓ Good: Force evaluation
   b.iter(|| black_box(expensive_computation()));
   ```

3. **Don't use tiny inputs that don't scale**
   ```rust
   // ✗ Bad: Too small to measure overhead
   b.iter(|| parse(black_box("x")));
   
   // ✓ Good: Representative size
   b.iter(|| parse(black_box(&realistic_500_line_input)));
   ```

## Best Practices

### DO:
✅ Use `black_box()` to prevent compiler optimizations  
✅ Include realistic test data  
✅ Document baseline metrics  
✅ Set regression thresholds  
✅ Run on consistent hardware/conditions  
✅ Compare against baselines  
✅ Investigate significant changes  
✅ Run benchmarks multiple times before concluding  
✅ Prioritize correctness over performance  

### DON'T:
❌ Benchmark debug builds  
❌ Include I/O in hot path measurements  
❌ Compare across different machines  
❌ Ignore persistent regressions  
❌ Over-optimize for benchmarks  
❌ Benchmark with background load  
❌ Use absolute times for cross-machine comparison  

## Performance Regression Analysis Examples

### Example 1: Dependency Analysis Regression

**Scenario:** After adding async dependency resolution, benchmark shows 15% slowdown.

```bash
$ cargo bench --bench dependency_analysis

analyze dependencies    time:   [141.23 ms 143.87 ms 146.51 ms]
                        change: [+12.234% +15.123% +18.012%] (regression)
```

**Analysis:**
1. Regression is 15% (exceeds 10% threshold)
2. Trade-off: Async adds 20ms but enables concurrent analysis
3. Decision: Accept regression, update baseline with justification

**Resolution:**
```bash
# Update baseline with notes
git commit -m "feat: Add async dependency resolution

Accepts 15% performance regression (20ms) in dependency analysis
to enable concurrent dependency tree traversal.

Benchmark: 123ms -> 144ms
"

cargo run --bin bench-tracker baseline --version "v0.2.0"
```

### Example 2: Twiggy Parsing Optimization

**Scenario:** Optimized regex compilation shows 45% improvement.

```bash
$ cargo bench --bench twiggy_parsing

parse twiggy output     time:   [1.2345 µs 1.2789 µs 1.3234 µs]
                        change: [-47.234% -45.123% -43.012%] (improvement)
```

**Analysis:**
1. Significant improvement (45%)
2. Validate improvement is real (run multiple times)
3. Check that output correctness is maintained

**Resolution:**
```bash
# Verify correctness first
cargo test

# Update baseline
cargo run --bin bench-tracker baseline

git commit -m "perf: Optimize twiggy regex compilation (45% faster)

Cache compiled regexes to avoid per-line recompilation.

Benchmark: 2.3µs -> 1.3µs per line
"
```

### Example 3: False Positive Investigation

**Scenario:** Asset scanning shows 8% regression, but code unchanged.

```bash
$ cargo bench --bench asset_scanning

scan project for assets time:   [48.234 µs 49.123 µs 50.011 µs]
                        change: [+6.234% +8.123% +10.012%] (possible regression)
```

**Analysis:**
1. Code unchanged since last baseline
2. System load was high during benchmark
3. Re-run shows normal performance

**Resolution:**
```bash
# Re-run in clean environment
cargo clean
sudo cpupower frequency-set --governor performance  # Linux only

# Close background applications
# Re-run benchmark
cargo bench --bench asset_scanning

# If results normalize, no action needed
# If regression persists, investigate system changes
```

## Troubleshooting

### Inconsistent Results

Benchmarks can be affected by:
- System load (close other applications)
- CPU frequency scaling (use performance governor)
- Background processes
- Thermal throttling

**Solution:** 
```bash
# Reduce system noise
# Close browser, IDE, background apps

# Increase sample size
cargo bench -- --sample-size 200

# Increase measurement time
cargo bench -- --measurement-time 10

# Run benchmarks multiple times and use median values
```

### Missing Benchmark Results

If `bench-tracker` reports no results:
1. Ensure benchmarks ran successfully: `cargo bench`
2. Check `target/criterion/` exists
3. Verify benchmarks use `criterion_group!` and `criterion_main!`

### False Positive Regressions

Small variations are normal. Adjust the regression threshold:

```bash
cargo run --bin bench-tracker run --max-regression 15.0
```

### Slow Benchmarks

```bash
# Ensure running in release mode (default)
cargo bench

# Check optimization level in Cargo.toml
[profile.bench]
opt-level = 3
```

### Comparing Across Machines

Absolute times vary by hardware. Focus on:
- Relative performance (change %)
- Scaling characteristics (small vs large)
- Consistency within machine

## CI Integration

### Current Status

Benchmarks run automatically in CI on:
- Push to main branch (updates baseline)
- Pull requests (checks for regressions)

Performance regressions >10% will fail the CI build.

### Manual CI Workflow

1. Run benchmarks on main branch
2. Save baseline: `cargo run --bin bench-tracker baseline`
3. On feature branch, compare: `cargo run --bin bench-tracker run`
4. Review any regressions >10%

## Historical Performance Data

Performance trends for wasm-slim (tracked since v0.1.0):

| Version | Asset Scan | Dep Analysis | Twiggy Parse | Notes |
|---------|------------|--------------|--------------|-------|
| v0.1.0  | 45µs       | 123ms        | 2.3µs        | Initial baseline |
| v0.2.0  | 45µs       | 144ms (+17%) | 2.3µs        | Added async deps |
| v0.3.0  | 42µs (-7%) | 141ms (-2%)  | 1.3µs (-43%) | Regex caching |
| current | 45µs       | 123ms        | 145µs        | Latest benchmarks |

*Note: Historical data stored in `.bench-tracker/history.json`*

### Viewing Historical Trends

```bash
# View benchmark history
cat .bench-tracker/history.json | jq '.history[] | {version, date, benchmarks}'

# Compare two versions
cargo run --bin bench-tracker compare v0.1.0 v0.3.0

# Generate performance report
cargo run --bin bench-tracker report --format markdown > docs/performance-report.md
```

## Benchmark Results Location

Criterion stores detailed results in:
```
target/criterion/
├── scan project for assets/
│   ├── base/
│   │   ├── estimates.json
│   │   ├── sample.json
│   │   └── ...
│   └── report/
│       └── index.html
└── ...
```

View HTML reports: `target/criterion/<benchmark-name>/report/index.html`

## Resources

- [Criterion.rs Book](https://bheisler.github.io/criterion.rs/book/) - Comprehensive benchmarking guide
- [Performance Tracking Documentation](PERFORMANCE_TRACKING.md) - Project-specific tracking
- [Rust Performance Book](https://nnethercote.github.io/perf-book/) - General Rust optimization
- [Benchmarking Best Practices](https://www.algolia.com/blog/engineering/benchmarking-best-practices/) - Statistical considerations
- [Flamegraph Profiling](https://github.com/flamegraph-rs/flamegraph) - For deeper performance analysis

## Maintenance

### Quarterly Review
- Update baseline metrics
- Verify regression thresholds
- Add benchmarks for new features
- Remove obsolete benchmarks

### After Major Changes
- Run full benchmark suite
- Update baselines if intentional
- Document performance changes in changelog
- Adjust targets if architecture changed

---

**Last Updated:** 2025-11-08  
**Baseline Version:** main @ commit 9d25b9a
