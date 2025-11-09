# wasm-slim

> Rust CLI tool for automated WASM binary size optimization

[![Crates.io](https://img.shields.io/crates/v/wasm-slim.svg)](https://crates.io/crates/wasm-slim)
[![Documentation](https://docs.rs/wasm-slim/badge.svg)](https://docs.rs/wasm-slim)
[![codecov](https://codecov.io/gh/vitalratel/wasm-slim/branch/main/graph/badge.svg)](https://codecov.io/gh/vitalratel/wasm-slim)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Status: Beta](https://img.shields.io/badge/Status-Beta-orange)

## 🎯 Goal

Reduce WASM bundle sizes by 60%+ without requiring deep optimization expertise.

**Before:**
```bash
# Manual optimization (complex, error-prone)
cargo build --release
wasm-bindgen ...
wasm-opt -Oz ...
wasm-snip ...
# Result: Maybe 500KB → 200KB if you know what you're doing
```

**After:**
```bash
cargo wasm-slim build
# Output: 60% size reduction, zero config ✨
```

## 📊 Research Foundation

Built on production learnings from real-world WASM optimization projects:

### Warp.dev (Dec 2024)
- **21.4MB → 8MB** (62% reduction)
- Technique: Asset management + compiler optimization
- [Blog post](https://www.warp.dev/blog/reducing-wasm-binary-size)

### Additional Validation
- Internal projects validating dependency optimization techniques
- Proven patterns for feature flag optimization and LTO configuration
- Real-world verification of 60%+ size reduction strategies

## 🚀 Features (Planned)

- ✅ Automated Cargo.toml optimization
- ✅ Integrated build pipeline (wasm-bindgen + wasm-opt + wasm-snip)
- ✅ Dependency analysis and suggestions
- ✅ Size profiling with twiggy integration
- ✅ CI/CD integration with size budgets
- ✅ Template system for common frameworks (Yew, Leptos, Dioxus)
- ✅ **NEW:** Nightly Rust build-std support (10-20% additional reduction)

## 🌙 Advanced: Nightly Rust Optimizations

For **maximum size reduction**, use nightly Rust with the `build-std` feature:

```bash
# Switch to nightly toolchain
rustup default nightly

# Build with automatic build-std configuration
cargo wasm-slim build
```

**What is build-std?**
- Rebuilds the standard library with your project's optimization settings
- Enables `panic_immediate_abort` feature (eliminates panic infrastructure)
- **Provides an additional 10-20% size reduction** on top of other optimizations
- Based on [Leptos binary size guide](https://book.leptos.dev/deployment/binary_size.html)

**How it works:**
1. `wasm-slim` detects your nightly toolchain automatically
2. Creates `.cargo/config.toml` with optimal build-std settings
3. Subsequent builds use the custom-built standard library

**Requirements:**
- Nightly Rust toolchain (install with `rustup default nightly`)
- Opt-in feature (automatically enabled when nightly is detected)

**Note:** On stable Rust, `wasm-slim` will provide a tip about nightly benefits.

## 📦 Installation

### From crates.io
```bash
cargo install wasm-slim
```

### From source
```bash
git clone https://github.com/vitalratel/wasm-slim.git
cd wasm-slim
cargo install --path .
```

## 📚 Examples

The `examples/` directory contains production-tested scripts and workflows:

- **size-tracking-script.sh**: Drop-in CI script for size budget enforcement
- **cargo-toml-optimizations.toml**: Battle-tested Cargo.toml configurations
- **twiggy-analysis-workflow.md**: Step-by-step WASM size analysis guide

See [examples/README.md](examples/README.md) for details.

## 📖 Documentation

- [Architecture Guide](docs/ARCHITECTURE.md) - System design and module organization
- [Performance Guide](docs/PERFORMANCE.md) - Optimization techniques and profiling
- [Benchmark Guide](docs/BENCHMARKS.md) - Running and writing performance benchmarks
- [Testing Guide](docs/TESTING.md) - Contributing test guidelines

## 🧪 Quality

- **Tests**: 861 tests across 20 modules (unit, integration, doc tests)
- **Benchmarks**: 3 benchmark suites with regression detection
- **CI/CD**: Automated testing, security audits, performance tracking
- **Coverage**: Comprehensive test coverage for core functionality

## 🔧 Development

### Prerequisites

**Required:**
- Rust 1.86+ (MSRV)
- `cargo` (comes with Rust)

**For full integration tests:**
```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli (required for build integration tests)
cargo install wasm-bindgen-cli
```

**Note:** Unit tests (~355 tests) run without any additional tools. Integration tests that exercise the build pipeline will skip gracefully if `wasm-bindgen-cli` is not installed.

### Running Tests

```bash
# Run all tests (unit + integration, some may skip without tools)
cargo test

# Run only unit tests (no external tools required)
cargo test --lib

# Run integration tests (requires wasm-bindgen-cli)
cargo test --test integration_tests

# Run with coverage
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

### Test Requirements

- **Unit tests**: No external dependencies
- **Integration tests**: Some tests require `wasm-bindgen-cli` for full pipeline testing
- **Doc tests**: No external dependencies
- **Benchmarks**: No external dependencies

Integration tests will display a helpful message if tools are missing:
```
⚠️  Skipping test: wasm-bindgen-cli not found in PATH
   Install with: cargo install wasm-bindgen-cli
```

## 🤝 Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Ways to contribute:**
- Report bugs and suggest features
- Improve documentation
- Add test coverage
- Share WASM optimization experiences

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details

## 🙏 Acknowledgments

- **Warp.dev team** for publishing their WASM optimization journey
- **Rust WASM community** for tooling (twiggy, wasm-bindgen, wasm-opt)
- Internal projects that validated optimization techniques

## 📬 Contact

Issues and questions: [GitHub Issues](https://github.com/vitalratel/wasm-slim/issues)

---

**Version**: 0.1.0 (Pre-1.0 - API may change)
