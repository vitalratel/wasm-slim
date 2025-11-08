# Basic Usage Example

This example demonstrates the simplest usage of wasm-slim for optimizing a basic WASM project.

## What This Example Shows

- Default optimization settings (LTO, opt-level="s", strip)
- Automatic wasm-opt integration
- Before/after size comparison
- Basic build output

## Prerequisites

```bash
# Install wasm-slim from the repository root
cd ../..
cargo install --path .

# Install WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-opt (optional, for maximum size reduction)
# macOS: brew install binaryen
# Ubuntu: sudo apt install binaryen
# Or download from: https://github.com/WebAssembly/binaryen/releases
```

## Project Structure

```
basic-usage/
├── Cargo.toml      # Simple WASM library
├── src/
│   └── lib.rs      # Basic greeting function
└── README.md       # This file
```

## Usage

### Step 1: Build without wasm-slim (baseline)

```bash
cd examples/basic-usage
cargo build --release --target wasm32-unknown-unknown
ls -lh target/wasm32-unknown-unknown/release/*.wasm
```

**Expected baseline size**: ~2.5MB

### Step 2: Build with wasm-slim

```bash
wasm-slim build
```

**Expected optimized size**: ~800KB-1.2MB (60-70% reduction)

### Step 3: View detailed analysis

```bash
# Analyze dependencies
wasm-slim analyze --mode deps

# Check for embedded assets
wasm-slim analyze --mode assets

# View top code contributors (requires twiggy)
wasm-slim analyze --mode top
```

## Expected Output

```
🏗️  Building WASM with optimization (balanced template)...
✅ Build complete: target/wasm32-unknown-unknown/release/basic_usage.wasm

📊 Size Analysis:
   Original:  2,485,760 bytes (2.37 MB)
   Optimized:   985,344 bytes (962 KB)
   Reduction: 1,500,416 bytes (1.43 MB) - 60.4% smaller

🎯 Optimizations Applied:
   ✓ LTO enabled (15-30% reduction)
   ✓ opt-level = "s" (size optimization)
   ✓ Debug symbols stripped
   ✓ wasm-opt -O3 (20-30% additional reduction)
```

## Troubleshooting

### Build fails with "wasm-opt not found"

Install binaryen or disable wasm-opt:

```bash
wasm-slim build --no-wasm-opt
```

### Want even more size reduction?

Try the "aggressive" template:

```bash
wasm-slim init --template aggressive
wasm-slim build
```

This uses opt-level="z" and wasm-opt -Oz for maximum size reduction (at the cost of slightly slower build times).

## Next Steps

- See [with-assets](../with-assets/) for optimizing projects with embedded fonts/images
- See [ci-integration](../ci-integration/) for CI/CD size budget enforcement
- Read the main [README](../../README.md) for advanced configuration options
