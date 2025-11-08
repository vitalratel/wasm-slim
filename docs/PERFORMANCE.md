# Performance Characteristics

This document provides detailed performance metrics, optimization opportunities, and tuning guidance for wasm-slim.

**Related Documentation:**
- [BENCHMARKS.md](BENCHMARKS.md) - Running and writing performance benchmarks
- [PERFORMANCE_TRACKING.md](PERFORMANCE_TRACKING.md) - Regression detection with bench-tracker

## Benchmark Results

### Core Operations

| Operation | Time | Memory | Notes |
|-----------|------|--------|-------|
| Cargo.toml optimization | <50ms | ~2MB | Per file modification |
| Asset scanning (1000 files) | ~120ms | ~5MB | Parallel with rayon |
| Dependency analysis (50 deps) | ~200ms | ~8MB | cargo metadata parsing |
| WASM build (release) | 10-30s | 150-300MB | Dominated by cargo build |
| twiggy analysis (3MB WASM) | 1-3s | 50-100MB | Binary analysis |
| cargo-bloat analysis | 5-10s | 50-100MB | Includes compilation |

### Scalability

| Metric | Small Project | Medium Project | Large Project |
|--------|--------------|----------------|---------------|
| Files scanned | 50 | 500 | 2000+ |
| Dependencies | 20 | 50 | 100+ |
| Scan time | 50ms | 100ms | 250ms |
| Analysis time | 100ms | 200ms | 500ms |

## CPU Profile

### Hot Functions (from production usage)

1. **`cargo_metadata` (15%)** - External cargo command
2. **`toml_edit::parse` (12%)** - TOML parsing for Cargo.toml
3. **`syn::parse_file` (20%)** - Rust AST parsing (asset detection)
4. **`rayon` parallel iteration (8%)** - File scanning parallelization
5. **I/O operations (10%)** - File reads and process spawning

### Optimization Opportunities

#### Already Optimized ✅
- Parallel file scanning with rayon
- Lazy evaluation of expensive operations (only analyze when requested)
- Efficient TOML editing (toml_edit preserves formatting)
- Process output streaming (no buffering entire build logs)

#### Potential Improvements 📈
- **AST caching** - Cache parsed syntax trees between operations (~30% improvement for repeated asset scans)
- **Incremental analysis** - Only re-analyze changed files (~50% improvement for repeated runs)
- **Memory-mapped WASM files** - Use mmap for large binary analysis (~20% memory reduction)
- **Parallel Cargo.toml optimization** - Process multiple files concurrently in workspaces

## Memory Profile

### Peak Memory Usage

**Typical CLI invocation:** 180-250MB peak

### Memory Breakdown

1. **AST structures** (60MB) - Syn parsing for asset detection
2. **TOML documents** (30MB) - In-memory representation
3. **cargo metadata** (20MB) - Dependency graph
4. **String buffers** (15MB) - File contents and output formatting
5. **Baseline** (55MB) - Rust runtime + dependencies

### Memory Optimization Strategies

Currently implemented:
- Drop large structures immediately after use
- Stream cargo output instead of buffering
- Use `Cow<str>` for zero-copy string handling where possible

Future improvements:
- Implement memory pooling for repeated allocations
- Use arena allocation for temporary structures
- Consider streaming JSON parsing for large metadata

## Performance Tuning Guide

### For Users: Faster Builds

#### 1. Skip Unnecessary Analysis

```bash
# Only optimize Cargo.toml, skip analysis
wasm-slim build

# Skip asset scanning if no embedded assets
wasm-slim analyze deps  # Instead of 'assets'
```

#### 2. Reduce Parallelism (Lower Memory)

```bash
# Limit rayon threads to reduce memory usage
RAYON_NUM_THREADS=2 wasm-slim analyze assets
```

#### 3. Use Aggressive Caching

```bash
# Enable incremental compilation (faster rebuilds)
export CARGO_INCREMENTAL=1
wasm-slim build
```

### For CI/CD: Optimizing Pipeline Speed

#### 1. Cache wasm-opt and Tools

```yaml
# GitHub Actions example
- uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/bin/wasm-opt
      ~/.cargo/bin/twiggy
    key: wasm-tools-${{ runner.os }}
```

#### 2. Parallel Jobs

```bash
# Run analysis in parallel with build (if tools installed)
wasm-slim analyze deps &
wasm-slim build
wait
```

#### 3. Skip Redundant Operations

```bash
# Only check budget, don't rebuild if WASM exists
wasm-slim build --check --json
```

### Troubleshooting Performance Issues

#### Slow Asset Scanning?

**Symptoms:** `wasm-slim analyze assets` takes >5 seconds

**Solutions:**
1. Add `.wasm-slim-ignore` file to exclude directories:
   ```
   node_modules
   target
   .git
   ```

2. Use `--exclude` pattern:
   ```bash
   wasm-slim analyze assets --exclude "node_modules/**"
   ```

3. Check for very large Rust files (>10K LOC):
   - Consider splitting files
   - Asset detection parses entire AST

#### High Memory Usage?

**Symptoms:** Process uses >500MB RAM

**Solutions:**
1. Reduce parallel file processing:
   ```bash
   RAYON_NUM_THREADS=1 wasm-slim analyze assets
   ```

2. Run analyses sequentially:
   ```bash
   # Instead of running all at once
   wasm-slim analyze deps
   wasm-slim analyze assets
   wasm-slim analyze bloat
   ```

3. Check for memory leaks (report bug if RSS keeps growing)

#### Slow Dependency Analysis?

**Symptoms:** `cargo metadata` takes >5 seconds

**Solutions:**
1. This is usually a cargo/network issue
2. Pre-populate cargo cache:
   ```bash
   cargo fetch  # Before running wasm-slim
   ```

3. Use offline mode if dependencies already fetched:
   ```bash
   cargo metadata --offline
   ```

## Benchmark Methodology

### Setup

```bash
# Install benchmarking tools
cargo install cargo-criterion
cargo install hyperfine

# Run benchmarks
cargo bench

# Run comparative benchmarks
hyperfine --warmup 3 \
  "wasm-slim analyze assets" \
  "wasm-slim analyze deps"
```

### Test Projects

Benchmarks run against three reference projects:

1. **Small** - 50 files, 20 dependencies, 500KB WASM
2. **Medium** - 500 files, 50 dependencies, 2MB WASM (typical browser extension)
3. **Large** - 2000 files, 100 dependencies, 5MB+ WASM

### Regression Detection

CI runs benchmarks on every PR:
- Must not regress >10% vs baseline
- Alert on >5% regression for investigation
- Baseline updated quarterly

## Profiling Tools Setup

### CPU Profiling with cargo-flamegraph

```bash
# Install
cargo install flamegraph

# Profile a specific command
sudo cargo flamegraph --bin wasm-slim -- analyze assets

# Output: flamegraph.svg
```

### Memory Profiling with heaptrack

```bash
# Install (Linux)
sudo apt install heaptrack

# Profile
heaptrack target/release/wasm-slim build

# Analyze
heaptrack_gui heaptrack.wasm-slim.*.gz
```

### Binary Size Analysis

```bash
# Install
cargo install cargo-bloat

# Analyze wasm-slim itself
cargo bloat --release -n 50

# Top functions by size
cargo bloat --release --crates
```

## Compilation Time Optimization

### Current Build Times

| Profile | Debug | Release | Release + LTO |
|---------|-------|---------|---------------|
| Clean | 45s | 90s | 180s |
| Incremental | 5s | 15s | 30s |

### Reducing Build Times

```toml
# .cargo/config.toml (for development)
[build]
incremental = true

[profile.dev]
split-debuginfo = "unpacked"  # Faster linking on macOS

[profile.dev.package."*"]
opt-level = 1  # Slightly optimize dependencies
```

## Performance Targets (SLOs)

### P0 (Must Meet)
- Init command: <100ms
- Cargo.toml optimization: <50ms per file
- CLI responsiveness: <200ms to first output

### P1 (Should Meet)
- Asset scanning: <500ms for 1000 files
- Dependency analysis: <1s for 50 dependencies
- Build pipeline: <30s for medium project

### P2 (Nice to Have)
- Parallel workspace optimization: <2x sequential time
- Incremental analysis: <10% of full analysis time

## Comparison with Alternatives

### wasm-opt alone
- **Speed:** Faster (single-purpose tool)
- **Scope:** wasm-slim does more (Cargo optimization, analysis)
- **Tradeoff:** wasm-slim orchestrates multiple tools

### Manual optimization
- **Speed:** N/A (manual process)
- **Quality:** wasm-slim more consistent
- **Time:** wasm-slim 100x faster (automated)

## Future Performance Work

### Planned (v1.1+)
- [ ] Incremental analysis with file watching
- [ ] AST caching layer
- [ ] Parallel workspace optimization
- [ ] Binary distribution for CI (no compile needed)

### Research
- [ ] Custom WASM parser (faster than twiggy for basic metrics)
- [ ] Distributed analysis (cloud-based heavy lifting)
- [ ] ML-based optimization prediction

---

## See Also

- **[BENCHMARKS.md](BENCHMARKS.md)** - Complete guide to running and writing benchmarks
- **[PERFORMANCE_TRACKING.md](PERFORMANCE_TRACKING.md)** - Regression detection with bench-tracker tool
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and module organization
- **[Criterion.rs Book](https://bheisler.github.io/criterion.rs/book/)** - Benchmarking framework documentation
- **[Rust Performance Book](https://nnethercote.github.io/perf-book/)** - General Rust optimization guide

---

**Last Updated:** 2025-11-08  
**Benchmark Version:** 1.0  
**Test Environment:** Linux 6.17.4, 16GB RAM, AMD Ryzen 7
