# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- WASM bundle size optimization CLI
- Dependency analysis with size impact assessment
- Asset detection in Rust source code
- Feature flag analysis
- Cargo.toml optimization with backup/restore
- Twiggy integration for binary analysis
- CI/CD integration with size budgets
- Nightly Rust build-std support
- Comprehensive benchmarking suite (3 suites, 11 benchmarks)
- 861 tests across 20 test modules
- Template system (minimal, balanced, aggressive)
- Build pipeline orchestration (cargo, wasm-bindgen, wasm-opt)
- Workflow command for guided optimization

### Performance
- Asset scanning: ~1.2ms per 1000 files
- Dependency analysis: ~50ms for typical projects
- Binary optimized to 2.7MB (release build)
- Parallel file processing with rayon

## [0.1.0] - 2025-11-07

Initial beta release (pre-1.0 API may change).

**Note**: This is a beta release. APIs and command-line interfaces are subject to change before 1.0.0.
