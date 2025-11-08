# Architecture Overview

wasm-slim is a Rust CLI tool for WASM bundle size optimization with a modular, layered architecture.

**Related Documentation:**
- [TESTING.md](TESTING.md) - Test organization and best practices
- [PERFORMANCE.md](PERFORMANCE.md) - Runtime performance characteristics
- [BENCHMARKS.md](BENCHMARKS.md) - Performance benchmarking guide

## Design Principles

### 1. Layered Architecture

The codebase follows clean separation of concerns across modules:

| Layer | Modules | Responsibility |
|-------|---------|----------------|
| **CLI** | `cmd/` | User interface, command routing (init, build, analyze, compare) |
| **Orchestration** | `pipeline/` | Build pipeline coordination (cargo, wasm-bindgen, wasm-opt) |
| **Analysis** | `analyzer/` | Dependency trees, asset detection, bloat/twiggy integration |
| **Optimization** | `optimizer/` | Cargo.toml modifications, backups, build-std |
| **Configuration** | `config/` | Templates (minimal/balanced/aggressive), validation |
| **Infrastructure** | `tools.rs`, `toolchain.rs`, `cicd/`, `bench_tracker/` | Tool detection, CI/CD integration, performance tracking |

Each layer has minimal coupling to others and clear responsibilities. For detailed module structure, run `cargo doc --open`.

### 2. Dependency Injection
- Tools and analyzers receive configuration via constructors
- No global state or singletons
- Testable components with clear boundaries

### 3. Error Transparency
- Uses `anyhow` for error propagation with context
- Errors include actionable information (file paths, line numbers)
- Clear error chains from low-level to high-level operations

### 4. Modularity
- Clear module boundaries with single responsibilities
- Minimal cross-module coupling
- Extensible analyzer and tool system

## Data Flow

### 1. Initialization Flow (`wasm-slim init`)
```
User invokes CLI
    ↓
cmd::init::cmd_init()
    ↓
config::Template::from_type()
    ↓
Select template (minimal/balanced/aggressive)
    ↓
Write .wasm-slim.toml
```

### 2. Build Flow (`wasm-slim build`)
```
User invokes CLI
    ↓
cmd::build::cmd_build()
    ↓
cmd::BuildWorkflow::execute()
    ↓
Load config::ConfigFile
    ↓
optimizer::CargoFileFinder::find_cargo_tomls()
    ↓
optimizer::CargoTomlEditor::optimize_cargo_toml()
    ↓
optimizer::BuildStdOptimizer::apply_build_std() (if nightly)
    ↓
pipeline::BuildPipeline::execute()
    ├── cargo build --release --target wasm32-unknown-unknown
    ├── wasm-bindgen (if configured)
    ├── wasm-opt -Oz (if configured)
    └── wasm-snip (if configured)
    ↓
Calculate size metrics
    ↓
cicd::BudgetChecker::check() (if CI mode)
```

### 3. Analysis Flow (`wasm-slim analyze`)
```
User invokes CLI
    ↓
cmd::analyze::cmd_analyze()
    ↓
Match analysis type:
    ├── dependencies → analyzer::DependencyAnalyzer
    ├── assets → analyzer::AssetDetector
    ├── bloat → analyzer::BloatAnalyzer
    ├── features → analyzer::FeatureAnalyzer
    └── wasm → analyzer::TwiggyAnalyzer
    ↓
Format and print report
```

## Key Abstractions

### 1. Configuration Templates
Templates encapsulate optimization strategies:
- **Minimal**: Basic size reduction (opt-level="z", LTO=thin)
- **Balanced**: Production-ready (opt-level="z", LTO=fat, strip=true)
- **Aggressive**: Maximum size reduction (opt-level="z", LTO=fat, codegen-units=1, panic="abort")

See `src/config/template.rs` for implementation.

### 2. Pipeline Configuration
Configures build tools and optimization levels. Key fields:
- `target`: WasmTarget (wasm32-unknown-unknown, wasm32-wasi)
- `profile`: Build profile (release/debug)
- `bindgen_target`: BindgenTarget (web/nodejs/bundler/deno)
- `run_wasm_opt`: Enable wasm-opt post-processing
- `opt_level`: WasmOptLevel (-Oz/-O3/-O2/-O1)

See `src/pipeline/config.rs` for full definition.

### 3. Analyzer Reports
Each analyzer produces domain-specific reports:
- **DependencyReport**: Heavy dependencies, size impact estimates
- **AssetReport**: Embedded assets detected in source code
- **FeatureReport**: Feature flag analysis and recommendations
- **BloatReport**: cargo-bloat output with top size contributors
- **TwiggyReport**: WASM binary analysis with function sizes

See `src/analyzer/*_report.rs` for implementations.

## Performance Characteristics

### Time Complexity
| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Dependency analysis | O(n) | n = number of dependencies |
| Asset scanning | O(m) | m = number of source files |
| Cargo.toml optimization | O(1) | Single file modification |
| WASM binary analysis (twiggy) | O(k log k) | k = WASM function count |
| Build pipeline | O(1) | Fixed sequence of external commands |

### Space Complexity
| Component | Memory Usage | Notes |
|-----------|--------------|-------|
| Dependency metadata | ~5-10 MB | cargo metadata JSON |
| AST parsing | ~20-50 MB | For asset detection across codebase |
| WASM analysis | ~50-100 MB | twiggy analysis structures |
| Backup storage | ~100 KB per file | Timestamped Cargo.toml copies |

### Typical Performance
Based on production usage:
- **Init**: <100ms
- **Cargo.toml optimization**: <50ms per file
- **Asset scanning**: ~120ms for 1000 files
- **Dependency analysis**: ~200ms for 50 dependencies
- **Full build pipeline**: 10-30 seconds (dominated by cargo build)
- **twiggy analysis**: 1-3 seconds for 3MB WASM file

## Concurrency Model

- **Single-threaded CLI** - Commands execute sequentially
- **Parallel file scanning** - Uses `rayon` for asset detection
- **External tool invocation** - Blocking process spawns (cargo, wasm-opt, etc.)
- **No async runtime** - Synchronous I/O throughout

## Error Handling Strategy

### Error Categories
1. **User Errors** - Invalid input, missing config → Helpful error with suggestion
2. **System Errors** - Missing tools, file access → Clear diagnostic with install instructions
3. **Build Errors** - Compilation failures → Preserve and display cargo output
4. **Internal Errors** - Unexpected states → Bug report prompt

### Error Context Chain
```rust
fn optimize() -> Result<()> {
    backup_file(&path)
        .context("Failed to create backup")?;  // Adds context
    
    modify_toml(&path)
        .context(format!("Failed to modify {}", path.display()))?;
    
    Ok(())
}
```

## Testing Strategy

### Unit Tests
- Co-located with implementation (`#[cfg(test)] mod tests`)
- Test individual functions and error cases
- Mock file system using `tempfile` crate

### Integration Tests
- Located in `tests/` directory
- Test complete command flows
- Use real Cargo.toml files and project structures

### Benchmarks
- Located in `benches/` directory
- Criterion.rs benchmarks for performance-critical paths
- Regression detection via CI

## Extension Points

### Adding a New Analyzer
1. Create `src/analyzer/my_analyzer.rs`
2. Implement analysis logic with public struct
3. Export from `src/analyzer/mod.rs`
4. Add command in `src/cmd/analyze.rs`
5. Add tests in analyzer module

### Adding a New Template
1. Add variant to `TemplateType` enum in `src/config/template.rs`
2. Implement `Template::from_type()` for the new variant
3. Document in README.md and config examples
4. Add integration test

### Adding CI/CD Integration
1. Extend `src/cicd/` with new integration (e.g., GitHub Actions)
2. Add JSON schema for configuration
3. Implement output formatter
4. Document in CI/CD guide

## Dependency Graph

### External Dependencies

**Core functionality:**
- **clap** - CLI argument parsing with derive macros
- **anyhow** + **thiserror** - Error handling and context
- **toml_edit** + **serde** - Configuration parsing with structure preservation
- **cargo_metadata** - Cargo.toml and dependency tree analysis
- **syn** + **quote** - Rust AST parsing for asset detection
- **regex** - Pattern matching (unicode-perl feature)

**Performance:**
- **rayon** - Parallel file scanning
- **parking_lot** - Fast synchronization primitives

**User experience:**
- **console** - Terminal colors and formatting
- **indicatif** - Progress bars and spinners

**Infrastructure:**
- **which** - External tool detection (cargo, wasm-opt, etc.)
- **uuid** - Unique backup file naming
- **env_logger** + **log** - Logging infrastructure
- **tempfile** - Test utilities and temporary directories

For complete list, see `Cargo.toml`.

### Internal Dependencies

Module dependency flow:
- **cmd/** uses: config, optimizer, analyzer, pipeline, cicd, tools
- **pipeline/** uses: optimizer, tools, toolchain, config
- **analyzer/** uses: tools, fmt (for reporting)
- **optimizer/** uses: git (for backups), toolchain (for build-std)
- **cicd/** uses: git (for history tracking)

All modules can use: error, fmt for shared utilities

## Security Considerations

1. **File System Access** - Limited to project directory and `.wasm-slim/` folder
2. **Command Injection** - No shell expansion, all args passed via Command::arg()
3. **Backup Safety** - UUID + timestamp prevents overwriting backups
4. **Config Validation** - Strict parsing with validation (e.g., budget thresholds)

---

**Document Version:** 1.0  
**Last Updated:** 2025-11-08  
**Authors:** wasm-slim maintainers

For implementation details, see individual module documentation via `cargo doc --open`.
