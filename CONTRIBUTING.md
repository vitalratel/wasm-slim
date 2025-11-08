# Contributing to wasm-slim

Thank you for your interest in contributing to wasm-slim!

## Development Setup

### Prerequisites
- Rust 1.86+ (install via [rustup](https://rustup.rs/))
- Git

### Getting Started

1. Clone the repository:
```bash
git clone https://github.com/vitalratel/wasm-slim.git
cd wasm-slim
```

2. Build the project:
```bash
cargo build
```

3. Run tests:
```bash
cargo test
```

4. Run the CLI:
```bash
cargo run -- --help
cargo run -- build --dry-run
```

## Project Structure

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed system design and module organization.

Quick exploration:
```bash
# Browse module structure
tree src/  # or: ls -R src/

# View detailed documentation
cargo doc --open
```

## Current Status

**Version**: 0.1.0 (Beta)

**Core Features Implemented:**
- ✅ CLI with clap (init, build, analyze, compare, workflow)
- ✅ Cargo.toml profile optimization
- ✅ Build pipeline integration (cargo, wasm-bindgen, wasm-opt)
- ✅ Dependency analysis and heavy dependency detection
- ✅ Asset detection in source code
- ✅ Feature flag analysis
- ✅ Bloat and twiggy WASM analysis integration
- ✅ Template system (minimal, balanced, aggressive)
- ✅ CI/CD integration with size budgets
- ✅ Nightly build-std support
- ✅ Comprehensive test suite (861 tests)
- ✅ Performance benchmarks with regression detection

## How to Contribute

### Bug Reports
- Use GitHub Issues
- Include: OS, Rust version, steps to reproduce

### Feature Requests
- Check the implementation plan first
- Open an issue to discuss before implementing
- Consider which phase it fits into

### Code Contributions

1. **Fork and create a branch**:
```bash
git checkout -b feature/your-feature-name
```

2. **Make your changes**:
- Follow Rust conventions (use `rustfmt`)
- Add tests for new functionality
- Update documentation as needed

3. **Run checks**:
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

4. **Commit with descriptive message**:
```bash
git commit -m "feat: add Cargo.toml parser

Implements parsing logic for reading and modifying Cargo.toml
while preserving formatting using toml_edit.

```

5. **Push and create PR**:
```bash
git push origin feature/your-feature-name
```

## Coding Standards

### Rust Style
- Use `cargo fmt` (rustfmt)
- Use `cargo clippy` and fix warnings
- Prefer `Result<T>` with `anyhow::Result` for errors
- Add doc comments for public items

### Commit Messages
Follow conventional commits:
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Adding tests
- `refactor:` Code refactoring
- `chore:` Maintenance tasks

Example:
```
feat: implement twiggy integration for size analysis

- Add twiggy wrapper in analyzer module
- Parse twiggy output into structured data
- Generate user-friendly size reports

```

### Testing
- Unit tests for individual functions
- Integration tests for CLI commands
- Use `assert_cmd` for testing CLI behavior
- Aim for >80% code coverage

## Documentation

- [Architecture Guide](docs/ARCHITECTURE.md) - System design and module organization
- [Performance Guide](docs/PERFORMANCE.md) - Optimization techniques and profiling
- [Benchmark Guide](docs/BENCHMARKS.md) - Running and writing performance benchmarks
- [Testing Guide](docs/TESTING.md) - Test organization and best practices
- [Testing Best Practices](docs/TESTING_BEST_PRACTICES.md) - Detailed testing patterns

## Questions?

- Check documentation in `docs/`
- Review [examples/](examples/) for reference implementations
- Open an issue for questions

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
