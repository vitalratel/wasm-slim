# GitHub Actions CI/CD Workflows

This directory contains comprehensive CI/CD workflows for the wasm-slim project.

## Workflows Overview

### 1. Tests (`test.yml`)

**Purpose**: Run the full test suite across multiple platforms and Rust versions.

**Triggers**: Push to main, Pull requests

**Key Features**:
- Multi-platform testing (Ubuntu, macOS, Windows)
- Tests both stable Rust and MSRV (1.75)
- Runs tests with all features and no default features
- Includes doc tests and example builds
- Separate CLI integration tests
- Comprehensive cargo caching for faster builds
- Concurrency cancellation for PRs

**Platforms**: ubuntu-latest, macos-latest, windows-latest

### 2. Code Coverage (`coverage.yml`)

**Purpose**: Generate and upload code coverage reports.

**Triggers**: Push to main, Pull requests

**Key Features**:
- Uses cargo-llvm-cov for accurate coverage
- Uploads to codecov.io
- Alternative tarpaulin configuration (disabled by default)
- Generates HTML coverage reports for main branch
- Stores coverage artifacts for 30 days

**Platform**: ubuntu-latest only

**Requirements**:
- Set `CODECOV_TOKEN` secret in repository settings (optional but recommended)

### 3. Performance Benchmarks (`benchmarks.yml`)

**Purpose**: Track performance using the bench-tracker CLI tool and detect regressions.

**Triggers**: Push to main, Pull requests

**Key Features**:
- Runs criterion benchmarks
- Uses bench-tracker for regression detection
- Automatically comments on PRs with results
- Stores baseline from main branch
- Fails on regressions exceeding 10% threshold
- Uploads criterion results as artifacts
- Optional scheduled benchmarks (commented out)

**Platform**: ubuntu-latest only

**How it works**:
- On PRs: Compares against cached baseline from main, fails if regression > 10%
- On main: Updates baseline for future comparisons
- Comments on PRs with detailed benchmark results

### 4. Code Quality (`quality.yml`)

**Purpose**: Enforce code quality standards and security checks.

**Triggers**: Push to main, Pull requests

**Key Features**:
- **Clippy**: Linting with `-D warnings` (fail on warnings)
- **Rustfmt**: Code formatting checks
- **Security Audit**: cargo-audit for vulnerability scanning
- **Documentation**: Doc generation with warnings as errors
- **cargo-deny**: Check for banned dependencies, licenses, and sources
- **Minimal Versions**: Test with minimal dependency versions
- **Multi-platform checks**: Verify compilation on all platforms

**Platform**: Runs different jobs on Ubuntu (primary) and all platforms (compilation check)

### 5. Scheduled Maintenance (`maintenance.yml`)

**Purpose**: Automated weekly maintenance checks for long-term project health.

**Triggers**: 
- Scheduled: Every Monday at 00:00 UTC
- Manual: workflow_dispatch

**Key Features**:
- **Security Audit**: Weekly vulnerability scanning with automatic issue creation
- **Dependency Updates**: Check for outdated dependencies with cargo-outdated
- **Coverage Regression**: Verify 80% coverage threshold is maintained
- **Benchmark Stability**: Monitor for performance regressions over time
- **License Compliance**: Scan for problematic licenses (GPL, AGPL, unknown)
- **MSRV Compatibility**: Ensure code still compiles with minimum Rust version
- **Automatic Issue Creation**: Creates GitHub issues when problems are detected
- **Maintenance Summary**: Generates comprehensive status report

**Platform**: ubuntu-latest only

**Issue Labels**: Automatically tags issues with:
- `security`, `dependencies`, `coverage`, `quality`, `legal`, `compatibility`, `automated`

**How it works**:
- Runs all checks in parallel (6 jobs)
- Only creates issues if they don't already exist (prevents spam)
- Summary job aggregates results from all checks
- Non-blocking: failures create issues but don't stop workflow

## Status Badges

Add these badges to your README.md:

```markdown
[![Tests](https://github.com/vitalratel/wasm-slim/workflows/Tests/badge.svg)](https://github.com/vitalratel/wasm-slim/actions/workflows/test.yml)
[![Coverage](https://github.com/vitalratel/wasm-slim/workflows/Code%20Coverage/badge.svg)](https://github.com/vitalratel/wasm-slim/actions/workflows/coverage.yml)
[![Benchmarks](https://github.com/vitalratel/wasm-slim/workflows/Performance%20Benchmarks/badge.svg)](https://github.com/vitalratel/wasm-slim/actions/workflows/benchmarks.yml)
[![Quality](https://github.com/vitalratel/wasm-slim/workflows/Code%20Quality/badge.svg)](https://github.com/vitalratel/wasm-slim/actions/workflows/quality.yml)
[![Maintenance](https://github.com/vitalratel/wasm-slim/workflows/Scheduled%20Maintenance/badge.svg)](https://github.com/vitalratel/wasm-slim/actions/workflows/maintenance.yml)
[![codecov](https://codecov.io/gh/vitalratel/wasm-slim/branch/main/graph/badge.svg)](https://codecov.io/gh/vitalratel/wasm-slim)
```

## Configuration

### Required Secrets

- `CODECOV_TOKEN` (optional): For uploading coverage to codecov.io
  - Generate at: https://codecov.io/
  - Add in: Settings → Secrets and variables → Actions

### Caching Strategy

All workflows use GitHub Actions cache to speed up builds:
- Cargo registry and git indices
- Build artifacts (target directory)
- Benchmark baselines (for benchmarks.yml)

Cache keys are based on:
- OS (runner.os)
- Cargo.lock hash
- Workflow-specific prefixes

### Concurrency Control

All workflows use concurrency groups to:
- Cancel in-progress runs when new commits are pushed to the same PR
- Save CI minutes and provide faster feedback

## Benchmark Workflow Details

The benchmark workflow uses the custom `bench-tracker` CLI tool:

**Commands used**:
- `cargo bench --quiet`: Run all criterion benchmarks
- `bench-tracker run --fail-on-regression --max-regression 10.0`: Check for regressions
- `bench-tracker compare`: Compare with baseline (for PR comments)
- `bench-tracker baseline --version "main-SHA"`: Update baseline on main

**Baseline storage**:
- Location: `.wasm-slim/benchmarks/baseline.json`
- Cached using GitHub Actions cache
- Updated only on main branch pushes
- Uploaded as artifact for 90-day retention

**Regression policy**:
- PRs fail if any benchmark regresses by more than 10%
- Detailed results are posted as PR comments
- Main branch pushes always update the baseline

## Customization

### Adjusting Benchmark Regression Threshold

Edit `benchmarks.yml`, line with `--max-regression`:
```yaml
--max-regression 10.0  # Change to your desired percentage
```

### Enabling Scheduled Benchmarks

Uncomment the schedule trigger in `benchmarks.yml`:
```yaml
on:
  schedule:
    - cron: '0 0 * * 0'  # Every Sunday at midnight UTC
```

### Using Tarpaulin Instead of llvm-cov

In `coverage.yml`, set `if: true` for the `coverage-tarpaulin` job.

### Adding More Platforms

Edit the matrix in `test.yml`:
```yaml
matrix:
  os: [ubuntu-latest, macos-latest, windows-latest, ubuntu-20.04]
```

## Troubleshooting

### Tests Fail on Specific Platform

Check the test.yml workflow run for that platform. Common issues:
- Path separators (use `std::path::PathBuf`)
- Line endings (configure git properly)
- Platform-specific dependencies

### Coverage Upload Fails

If codecov upload fails without `CODECOV_TOKEN`:
- Add the token as a secret
- Or set `fail_ci_if_error: false` (already configured)

### Benchmarks Always Show Regression

- Ensure baseline exists by pushing to main first
- Check cache is being restored properly
- Verify criterion output format hasn't changed

### Clippy Fails on New Code

Either:
- Fix the clippy warnings (recommended)
- Add `#[allow(clippy::lint_name)]` for false positives
- Adjust clippy configuration in `clippy.toml` (create if needed)

## Local Testing

### Run tests like CI:
```bash
# Multi-feature tests
cargo test --all-features
cargo test --no-default-features

# Clippy
cargo clippy --all-features --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check

# Security audit
cargo install cargo-audit
cargo audit

# Coverage (requires llvm-tools-preview)
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --workspace --html
```

### Run benchmarks like CI:
```bash
# Run benchmarks
cargo bench

# Check for regressions
cargo run --release --bin bench-tracker -- run \
  --fail-on-regression \
  --max-regression 10.0

# Update baseline
cargo run --release --bin bench-tracker -- baseline --version "local"
```

## Best Practices

1. **Always run tests locally before pushing**: Use pre-commit hooks
2. **Keep benchmarks fast**: CI has time limits (6 hours per job)
3. **Monitor cache hit rates**: Check Actions tab for cache statistics
4. **Review security advisories**: Check cargo-audit output regularly
5. **Update baselines carefully**: Only update on verified performance improvements
6. **Use draft PRs**: For WIP changes that don't need full CI yet

## CI Costs

GitHub Actions provides:
- 2,000 CI minutes/month for free (public repos)
- Unlimited for public repositories

Workflow typical durations:
- Tests: 5-10 minutes per platform
- Coverage: 3-5 minutes
- Benchmarks: 2-4 minutes
- Quality: 3-5 minutes
- Maintenance: 10-15 minutes (weekly only)

**Total per PR**: ~15-25 minutes (with caching)
**Total per week**: +10-15 minutes (scheduled maintenance)
