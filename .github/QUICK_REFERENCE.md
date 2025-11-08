# CI Workflows Quick Reference

Quick reference card for wasm-slim CI workflows.

## Workflow Status

| Workflow | Badge | Description |
|----------|-------|-------------|
| Tests | `[![Tests](https://github.com/USER/wasm-slim/workflows/Tests/badge.svg)]` | Multi-platform test suite |
| Coverage | `[![Coverage](https://codecov.io/gh/USER/wasm-slim/branch/main/graph/badge.svg)]` | Code coverage tracking |
| Benchmarks | `[![Benchmarks](https://github.com/USER/wasm-slim/workflows/Performance%20Benchmarks/badge.svg)]` | Performance regression detection |
| Quality | `[![Quality](https://github.com/USER/wasm-slim/workflows/Code%20Quality/badge.svg)]` | Code quality checks |

## Common Commands

### Run Locally Before Pushing

```bash
# Format code
cargo fmt --all

# Check with clippy
cargo clippy --all-features --all-targets -- -D warnings

# Run tests
cargo test --all-features

# Run benchmarks
cargo bench

# Check for regressions
cargo run --release --bin bench-tracker -- run --fail-on-regression
```

### Workflow-Specific

```bash
# Generate coverage report (like CI)
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --workspace --html
open target/llvm-cov/html/index.html

# Run security audit (like CI)
cargo install cargo-audit
cargo audit

# Check documentation (like CI)
cargo doc --all-features --no-deps
```

## Workflow Triggers

| Event | test.yml | coverage.yml | benchmarks.yml | quality.yml |
|-------|----------|--------------|----------------|-------------|
| Push to main | ✓ | ✓ | ✓ | ✓ |
| Pull request | ✓ | ✓ | ✓ | ✓ |
| Schedule | - | - | Optional | - |

## What Each Workflow Does

### test.yml - Tests
- Runs on: Ubuntu, macOS, Windows
- Rust versions: stable, 1.75 (MSRV)
- Tests: All features, no features, docs, examples
- Duration: ~8 min per platform

### coverage.yml - Code Coverage
- Runs on: Ubuntu only
- Tool: cargo-llvm-cov
- Uploads to: codecov.io
- Duration: ~4 min

### benchmarks.yml - Performance
- Runs on: Ubuntu only
- Tool: bench-tracker CLI
- Fails if: Regression > 10%
- Posts: PR comment with results
- Duration: ~3 min

### quality.yml - Quality Checks
- Clippy (warnings as errors)
- Rustfmt (formatting check)
- cargo-audit (security)
- cargo-deny (dependencies)
- Documentation generation
- Minimal versions check
- Duration: ~5 min

## Benchmark Workflow Details

### On Pull Requests
1. Run benchmarks
2. Compare with baseline from main
3. Post results as PR comment
4. Fail if regression > 10%

### On Main Branch
1. Run benchmarks
2. Update baseline for future PRs
3. Upload artifacts

### Manual Override
Skip benchmark check in PR:
```yaml
# In .github/workflows/benchmarks.yml, temporarily set:
if: false  # On the "Check for regressions" step
```

## Fixing Common CI Failures

| Error | Solution |
|-------|----------|
| Formatting check failed | `cargo fmt --all` |
| Clippy warnings | Fix warnings or add `#[allow(...)]` |
| Test failures | Debug with `cargo test --all-features` |
| Benchmark regression | Review performance changes or adjust threshold |
| Security audit failed | Update vulnerable dependencies |
| Coverage upload failed | Add `CODECOV_TOKEN` secret |

## Cache Information

Workflows cache:
- Cargo registry: `~/.cargo/registry/index`
- Cargo git: `~/.cargo/git`
- Build artifacts: `target/`
- Benchmark baseline: `.wasm-slim/benchmarks/`

Cache keys based on:
- Operating system
- Cargo.lock hash
- Workflow name

Clear cache: Settings → Actions → Caches → Delete

## Secrets Required

| Secret | Required For | How to Get |
|--------|-------------|------------|
| CODECOV_TOKEN | coverage.yml | https://codecov.io/ → Add repo → Copy token |

## File Locations

```
.github/
├── CI_SETUP.md                  # Setup instructions
├── IMPLEMENTATION_SUMMARY.md    # Full implementation details
├── QUICK_REFERENCE.md          # This file
└── workflows/
    ├── README.md               # Workflow documentation
    ├── test.yml               # Tests
    ├── coverage.yml           # Coverage
    ├── benchmarks.yml         # Benchmarks
    └── quality.yml            # Quality checks
```

## Debugging Workflows

### Enable Debug Logging
Add to repository secrets:
- Name: `ACTIONS_STEP_DEBUG`
- Value: `true`

### View Workflow Run
1. Go to "Actions" tab
2. Click on workflow run
3. Click on specific job
4. View logs for each step

### Download Artifacts
- Criterion results: 30 days retention
- Benchmark baseline: 90 days retention
- Coverage report: 30 days retention

## Customization

### Change Benchmark Threshold
Edit `benchmarks.yml` line ~72:
```yaml
--max-regression 10.0  # Adjust this value
```

### Add Test Platform
Edit `test.yml` matrix:
```yaml
os: [ubuntu-latest, macos-latest, windows-latest, ubuntu-20.04]
```

### Skip CI for Commit
```bash
git commit -m "docs: update README [skip ci]"
```

## Performance Optimization

Current setup with caching:
- First run: ~60-70 min total
- Subsequent runs: ~15-25 min total

Tips:
- Keep dependencies lean
- Incremental compilation enabled
- Parallel job execution
- Aggressive caching

## Branch Protection Settings

Recommended required checks:
- [x] Test Suite (from test.yml)
- [x] Clippy Lints (from quality.yml)
- [x] Code Formatting (from quality.yml)
- [x] Run Benchmarks (from benchmarks.yml) - Optional
- [x] Generate Coverage Report (from coverage.yml) - Optional

Configure at: Settings → Branches → main → Add rule

## Getting Help

- **Workflow issues**: Check `.github/workflows/README.md`
- **Setup help**: Check `.github/CI_SETUP.md`
- **Full details**: Check `.github/IMPLEMENTATION_SUMMARY.md`
- **GitHub Actions docs**: https://docs.github.com/en/actions
- **Project issues**: Create issue in repository

## Quick Checks

Before creating a PR:
```bash
# All-in-one check
cargo fmt --all && \
  cargo clippy --all-features --all-targets -- -D warnings && \
  cargo test --all-features && \
  echo "✓ All checks passed!"
```

After merging to main:
- Check Actions tab for green checkmarks
- Verify baseline was updated (benchmarks)
- Verify coverage was uploaded (codecov.io)
