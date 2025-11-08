# With Assets Example

This example demonstrates how to use wasm-slim with a project that has embedded assets (fonts, images).

## What This Example Shows

- Asset detection for embedded files (include_bytes!, include_str!)
- Externalization recommendations based on size impact
- Size budget configuration
- Asset analysis reporting

## Prerequisites

```bash
# Install wasm-slim
cd ../..
cargo install --path .

# Install WASM target
rustup target add wasm32-unknown-unknown
```

## Project Structure

```
with-assets/
├── Cargo.toml              # WASM library with dependencies
├── .wasm-slim.toml         # Size budget configuration
├── src/
│   └── lib.rs              # Code with embedded assets
├── assets/
│   ├── font.txt            # Dummy font file (simulates real font)
│   └── data.json           # Dummy data file
└── README.md               # This file
```

## Usage

### Step 1: Build and analyze assets

```bash
cd examples/with-assets
wasm-slim analyze --mode assets
```

**Expected output**:
```
📦 Embedded Assets Detected (2 assets, 48.5 KB total)

🔴 Critical Priority (0 assets)
🟡 High Priority (1 asset)
   assets/font.txt - 32.0 KB (66% of bundle) [include_bytes!]
🔵 Medium Priority (1 asset)
   assets/data.json - 16.5 KB (34% of bundle) [include_str!]

💡 Externalization Guide:
   For High priority assets:
   - Remove include_bytes!() from source
   - Fetch at runtime with fetch API
   - Estimated savings: 32 KB (66%)
```

### Step 2: Build with size budget

```bash
wasm-slim build --check
```

This will:
1. Build the WASM module
2. Check against size budget in `.wasm-slim.toml`
3. Exit with error code if budget exceeded

### Step 3: View build history

```bash
# Build multiple times
wasm-slim build
# Make a change...
wasm-slim build

# View history
wasm-slim history
```

## Size Budget Configuration

The `.wasm-slim.toml` file shows how to configure size budgets:

```toml
[size-budget]
target-size-kb = 500      # Target size
warn-threshold-kb = 750    # Warning level
max-size-kb = 1000         # Hard limit (fails CI)
```

## Optimization Strategy

For this example with embedded assets:

1. **Phase 1**: Apply standard optimizations (LTO, wasm-opt)
   - Reduction: ~40-50%

2. **Phase 2**: Externalize large assets (>10% of bundle)
   - Reduction: ~60-70% additional
   - **Total**: ~80-85% smaller

### Before Externalization
```
Total bundle: 2.5 MB
├── Code: 500 KB (20%)
└── Assets: 2.0 MB (80%)
```

### After Standard Optimization
```
Total bundle: 1.2 MB
├── Code: 250 KB (optimized)
└── Assets: 950 KB (compressed)
```

### After Asset Externalization
```
Total bundle: 250 KB
└── Code: 250 KB (assets loaded at runtime)
```

## Next Steps

- Externalize the font using fetch API in JavaScript
- Configure CDN for asset delivery
- See [ci-integration](../ci-integration/) for automated size enforcement
