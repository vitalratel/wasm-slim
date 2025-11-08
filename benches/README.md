# wasm-slim Benchmarks

Performance benchmarks using [Criterion.rs](https://github.com/bheisler/criterion.rs).

## Quick Start

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench asset_scanning
```

## Available Benchmarks

- `asset_scanning` - Asset detection system performance
- `dependency_analysis` - Dependency tree analysis
- `twiggy_parsing` - WASM analysis output parsing
- `cargo_optimizer` - Cargo.toml optimization operations
- `backup_operations` - Backup/restore functionality
- `config_parsing` - Configuration file operations

## Documentation

**📚 Complete Guide:** [docs/BENCHMARKS.md](../docs/BENCHMARKS.md)

Includes:
- How to run and interpret benchmarks
- Performance tracking with bench-tracker
- Adding new benchmarks
- Best practices and examples
- Troubleshooting guide
- Historical performance data

**📊 Performance Tracking:** [docs/PERFORMANCE_TRACKING.md](../docs/PERFORMANCE_TRACKING.md)
