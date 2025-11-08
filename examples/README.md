# wasm-slim Examples

This directory contains example projects demonstrating different use cases for wasm-slim.

## Examples

### 1. [basic-usage](./basic-usage/)
The simplest usage of wasm-slim for a basic WASM project.
- Demonstrates default optimization settings
- Shows before/after size comparison
- **Estimated size reduction**: 60-70%

### 2. [with-assets](./with-assets/)
Optimizing a WASM project with embedded assets (fonts, images).
- Demonstrates asset detection and externalization recommendations
- Shows how to configure size budgets
- **Estimated size reduction**: 70-80% (with asset externalization)

### 3. [ci-integration](./ci-integration/)
Integrating wasm-slim into CI/CD pipelines with size budgets.
- GitHub Actions workflow configuration
- Size budget enforcement
- Build history tracking
- JSON output for automation

## Quick Start

Each example includes a README with:
- Project description
- Prerequisites
- Step-by-step usage instructions
- Expected results
- Troubleshooting tips

## Running Examples

```bash
# Navigate to an example
cd examples/basic-usage

# Follow the README instructions
cat README.md

# Run wasm-slim
cargo install --path ../..  # Install wasm-slim from repo root
wasm-slim build
```

## Contributing Examples

Want to add your own example? Please:
1. Create a new directory under `examples/`
2. Include a complete `README.md` with usage instructions
3. Add a minimal working `Cargo.toml` and source code
4. Document expected results and size reductions
5. Submit a PR!
