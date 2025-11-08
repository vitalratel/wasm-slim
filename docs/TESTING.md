# Testing Guide

Comprehensive guide to writing, running, and maintaining tests in wasm-slim.

## TL;DR for Quick Reference

**Test Naming:** `test_<function>_<scenario>_<expected>`

**Assertions:**
- Float comparisons: `assert_approx_eq(actual, expected, 0.01)` from `tests/common/assertions.rs`
- Size checks: `assert_size_within(actual_bytes, expected_bytes, 1024)` (±1KB tolerance)
- Percentages: `assert_percent_within(actual, expected, 2.0)` (±2% tolerance)

**Fixtures:** Use `common::fixtures::create_minimal_wasm_lib(name)` instead of manual setup

**Run Tests:** `cargo test` | `cargo test --test <name>` | `cargo test -- --nocapture`

**Tool Preference:** Use MCP `mcp__serena__*` tools for token efficiency

**Details:** See sections below or [TESTING_BEST_PRACTICES.md](TESTING_BEST_PRACTICES.md) for detailed guidance on assertions

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Test Organization](#test-organization)
3. [Test Naming Conventions](#test-naming-conventions)
4. [Writing Tests](#writing-tests)
5. [Running Tests](#running-tests)
6. [Test Helpers](#test-helpers)
7. [Best Practices](#best-practices)
8. [Contributing](#contributing)

---

## Quick Start

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration_tests
cargo test --test bloat_integration

# Run tests with output
cargo test -- --nocapture

# Run tests sequentially (for debugging)
cargo test -- --test-threads=1

# Run doc tests
cargo test --doc

# Run benchmarks
cargo bench
```

---

## Test Organization

### Test Structure

```
tests/
├── common/                          # Shared test utilities
│   ├── mod.rs                       # Module exports
│   ├── assertions.rs                # Custom assertions (float, size, %)
│   └── fixtures.rs                  # Test project builders
├── integration_tests.rs             # Main integration suite (68 tests)
├── cli_integration.rs               # CLI-specific tests (30 tests)
├── pipeline_integration.rs          # Pipeline integration tests (18 tests)
├── apply_suggestions_integration.rs # Suggestion applicator tests
├── bloat_integration.rs             # Bloat analyzer tests
├── timeout_integration.rs           # Timeout framework tests
├── windows_platform_integration.rs  # Platform-specific tests
├── assertion_helpers_test.rs        # Helper validation tests
# (See docs/TESTING_BEST_PRACTICES.md for assertion best practices)

src/
├── lib.rs                           # Main library
├── analyzer/
│   ├── assets.rs                    # ~60 unit tests
│   ├── bloat.rs                     # ~30 tests
│   └── ...
└── optimizer/
    ├── mod.rs                       # ~40 tests (profile optimization)
    └── backup.rs                    # ~25 tests

benches/
├── asset_scanning.rs                # Asset detection benchmarks
├── dependency_analysis.rs           # Dependency analysis benchmarks
└── twiggy_parsing.rs                # Twiggy parsing benchmarks
```

### Test Categories

| Category | Location | Count | Purpose |
|----------|----------|-------|---------|
| **Unit Tests** | `src/**/*.rs` | 395 | Test individual functions/modules |
| **Integration Tests** | `tests/*.rs` | 179 | Test full workflows and CLI |
| **Doc Tests** | `src/**/*.rs` (comments) | 57 | Executable API documentation |
| **Property Tests** | `src/**/*.rs` | 8 | Validate invariants with random data |
| **Benchmarks** | `benches/*.rs` | 6 | Performance regression detection |

**Total: 645 tests** (as of 2025-01-04)

---

## Test Naming Conventions

### Standard Pattern

**Format:** `test_<function>_<scenario>_<expected>`

**Components:**
- `test_` - Always prefix test functions with `test_`
- `<function>` - Function or feature being tested
- `<scenario>` - Input conditions or context
- `<expected>` - Expected behavior or outcome

### Examples

#### ✅ Good Names

```rust
#[test]
fn test_analyze_valid_module_succeeds() {
    // Tests that analyze() succeeds with valid input
}

#[test]
fn test_analyze_invalid_bytes_returns_error() {
    // Tests that analyze() returns error for invalid bytes
}

#[test]
fn test_apply_suggestions_single_fix_reduces_size() {
    // Tests that applying one suggestion reduces bundle size
}

#[test]
fn test_concurrent_backup_creation_generates_unique_filenames() {
    // Tests that concurrent backups get unique names
}

#[test]
fn test_timeout_operation_exceeding_limit_fails_gracefully() {
    // Tests timeout handling when limit exceeded
}

#[test]
fn test_parse_percentage_with_decimal_returns_float() {
    // Tests parsing "2.5%" returns 2.5
}
```

#### ❌ Bad Names

```rust
// Too vague
#[test]
fn test_basic() { }

// Missing scenario
#[test]
fn test_analyze() { }

// Unclear expectation
#[test]
fn test_suggestions() { }

// Not descriptive
#[test]
fn test_flow() { }

// Inconsistent style
#[test]
fn analyze_works() { }  // Missing test_ prefix

#[test]
fn should_apply_suggestions() { }  // Wrong prefix style
```

### Special Cases

#### Integration Tests
```rust
#[test]
fn test_cli_optimize_with_valid_project_succeeds() {
    // Full CLI workflow test
}

#[test]
fn test_cli_analyze_without_cargo_toml_shows_error() {
    // CLI error handling test
}
```

#### Property-Based Tests
```rust
#[test]
fn prop_priority_monotonic_with_size() {
    // Property: larger assets always have >= priority
}

#[test]
fn prop_percentage_always_between_zero_and_hundred() {
    // Property: percentages stay in valid range
}
```

#### Benchmark Tests
```rust
fn bench_asset_scanning_real_project(c: &mut Criterion) {
    // Benchmark realistic asset scanning
}

fn bench_dependency_analysis_large_tree(c: &mut Criterion) {
    // Benchmark worst-case dependency analysis
}
```

---

## Writing Tests

### Unit Tests

**Location:** Same file as code under test, in `#[cfg(test)] mod tests { }`

```rust
// src/analyzer/example.rs

pub fn calculate_percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    (part as f64 / total as f64) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_percentage_with_half_returns_fifty() {
        let result = calculate_percentage(50, 100);
        assert_eq!(result, 50.0);
    }

    #[test]
    fn test_calculate_percentage_with_zero_total_returns_zero() {
        let result = calculate_percentage(10, 0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_percentage_with_larger_part_exceeds_hundred() {
        let result = calculate_percentage(150, 100);
        assert!(result > 100.0);
    }
}
```

### Integration Tests

**Location:** `tests/*.rs` files

```rust
// tests/example_integration.rs

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_cli_help_displays_usage_information() {
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_optimize_creates_backup_before_changes() {
    let temp = TempDir::new().unwrap();
    // Setup test project
    // ...
    
    let mut cmd = Command::cargo_bin("wasm-slim").unwrap();
    cmd.current_dir(&temp).arg("optimize");
    
    cmd.assert().success();
    
    // Verify backup exists
    assert!(temp.path().join(".wasm-slim/backups").exists());
}
```

### Doc Tests

**Location:** Documentation comments in `src/**/*.rs`

```rust
/// Calculate the reduction percentage between two sizes
///
/// # Arguments
/// * `before` - Original size in bytes
/// * `after` - Optimized size in bytes
///
/// # Examples
///
/// ```
/// use wasm_slim::calculate_reduction;
///
/// // 50% reduction: 1MB → 512KB
/// let reduction = calculate_reduction(1048576, 524288);
/// assert_eq!(reduction, 50.0);
/// ```
///
/// Edge case - zero before size:
/// ```
/// # use wasm_slim::calculate_reduction;
/// let reduction = calculate_reduction(0, 100);
/// assert_eq!(reduction, 0.0);  // Avoid division by zero
/// ```
pub fn calculate_reduction(before: u64, after: u64) -> f64 {
    // Implementation
}
```

### Property-Based Tests

**Location:** Module tests using `proptest`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_priority_monotonic_with_size(
            size1 in 1u64..1000000,
            size2 in 1u64..1000000,
            bundle in 1000000u64..10000000
        ) {
            let p1 = AssetPriority::from_size(size1, bundle);
            let p2 = AssetPriority::from_size(size2, bundle);
            
            // Property: larger assets should have >= priority
            if size1 > size2 {
                assert!(p1 >= p2);
            }
        }
    }
}
```

---

## Running Tests

### Basic Commands

```bash
# Run all tests (unit + integration + doc)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific integration test file
cargo test --test bloat_integration

# Run tests matching pattern
cargo test backup

# Run with output visible
cargo test -- --nocapture

# Run sequentially (useful for debugging)
cargo test -- --test-threads=1
```

### Advanced Commands

```bash
# Run tests with timing information
cargo test -- --show-output --test-threads=1

# Run ignored tests
cargo test -- --ignored

# Run doc tests only
cargo test --doc

# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench -- asset_scanning

# Generate test coverage (requires cargo-llvm-cov)
cargo llvm-cov --html

# Run tests with memory sanitizer (nightly)
RUSTFLAGS="-Z sanitizer=address" cargo +nightly test
```

### CI Commands

```bash
# Run with all features
cargo test --all-features

# Run without default features
cargo test --no-default-features

# Run with verbose output for CI logs
cargo test -- --nocapture --test-threads=1

# Run with timeout
timeout 300 cargo test
```

---

## Test Helpers

### Available Helpers

Located in `tests/common/assertions.rs`:

#### Floating-Point Comparisons

```rust
use common::assertions::*;

// Instead of: assert_eq!(ratio, 0.75);
assert_approx_eq(ratio, 0.75, 0.01);  // ±1% tolerance
```

#### Size Comparisons

```rust
// Instead of: assert_eq!(size, 1048576);
assert_size_within(size, 1048576, 1024);  // ±1KB tolerance
```

#### Percentage Validations

```rust
assert_percent_within(reduction, 30.0, 2.0);  // ±2% tolerance
```

See [TESTING_BEST_PRACTICES.md](TESTING_BEST_PRACTICES.md) for comprehensive guide on when to use each helper.

### Creating Test Fixtures

**Use the `common::fixtures` module** for consistent test project setup:

```rust
mod common;
use common::fixtures::*;

#[test]
fn test_example() {
    // Use pre-built fixtures instead of manual setup
    let (temp_dir, cargo_toml) = create_minimal_wasm_lib("test-project")
        .expect("Failed to create test fixture");
    
    // Test code here
    // temp_dir is automatically cleaned up on drop
}
```

**Available Fixtures** (in `tests/common/fixtures.rs`):

- `create_minimal_wasm_lib(name)` - Basic WASM library with cdylib
- `create_minimal_bin(name)` - Binary project with main.rs
- `create_wasm_bindgen_project(name)` - WASM lib with wasm-bindgen

**When to Create Custom Fixtures:**

Only when you need specialized project configurations not covered by existing fixtures. Add new fixtures to `tests/common/fixtures.rs` for reuse.

---

## Best Practices

### 1. Test One Thing

```rust
// ❌ Bad: Tests multiple things
#[test]
fn test_analyzer() {
    let analyzer = Analyzer::new();
    analyzer.load_module();
    let results = analyzer.analyze();
    assert!(results.is_ok());
    assert_eq!(results.unwrap().items.len(), 5);
    assert!(results.unwrap().total_size > 0);
}

// ✅ Good: One assertion per test
#[test]
fn test_analyze_valid_module_succeeds() {
    let analyzer = Analyzer::new();
    analyzer.load_module();
    assert!(analyzer.analyze().is_ok());
}

#[test]
fn test_analyze_returns_expected_item_count() {
    let analyzer = Analyzer::new();
    analyzer.load_module();
    let results = analyzer.analyze().unwrap();
    assert_eq!(results.items.len(), 5);
}
```

### 2. Use Descriptive Assertions

```rust
// ❌ Bad: Unclear failure message
assert!(size > 1000);

// ✅ Good: Clear context
assert!(
    size > 1000,
    "Optimized size {} should be > 1KB for this test case",
    size
);
```

### 3. Test Error Cases

```rust
#[test]
fn test_parse_invalid_format_returns_error() {
    let result = parse_percentage("invalid");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("invalid format"));
}

#[test]
fn test_analyze_missing_file_returns_file_not_found() {
    let result = analyze(Path::new("/nonexistent"));
    assert!(matches!(result, Err(AnalysisError::FileNotFound(_))));
}
```

### 4. Avoid Sleeps

```rust
// ❌ Bad: Flaky and slow
thread::sleep(Duration::from_millis(100));
assert!(operation_completed());

// ✅ Good: Use synchronization
let (tx, rx) = mpsc::channel();
thread::spawn(move || {
    perform_operation();
    tx.send(()).unwrap();
});
rx.recv_timeout(Duration::from_secs(1)).unwrap();
```

### 5. Clean Up Resources

```rust
#[test]
fn test_with_cleanup() {
    let temp = TempDir::new().unwrap();
    // temp automatically cleaned up when dropped
    
    // Do test work
}

// Or use Drop guard
struct TestGuard {
    path: PathBuf,
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
```

---

## Contributing

### Adding New Tests

1. **Determine test type:**
   - Unit test? → Add to `src/**/*.rs` in `#[cfg(test)] mod tests`
   - Integration test? → Create/add to `tests/*.rs`
   - Doc test? → Add to function documentation

2. **Follow naming convention:**
   - Use `test_<function>_<scenario>_<expected>` pattern
   - Make name descriptive and searchable

3. **Write the test:**
   - Arrange: Set up test data
   - Act: Call function under test
   - Assert: Verify expected behavior

4. **Run tests locally:**
   ```bash
   cargo test
   cargo test --doc
   cargo clippy -- -D warnings
   ```

5. **Submit PR:**
   - Include test count change in description
   - Explain what scenarios are covered
   - Show test output in PR

### Test Review Checklist

- [ ] Test name follows convention
- [ ] One logical assertion per test
- [ ] Error cases tested
- [ ] Edge cases covered
- [ ] No hardcoded paths or sleeps
- [ ] Resources cleaned up
- [ ] Test passes consistently
- [ ] Documentation updated if needed

---

## References

- **Best Practices:** [TESTING_BEST_PRACTICES.md](TESTING_BEST_PRACTICES.md)
- **Benchmarks:** `BENCHMARKS.md`
- **Contributing:** `CONTRIBUTING.md`
- **Rust Testing:** https://doc.rust-lang.org/book/ch11-00-testing.html
- **Property Testing:** https://docs.rs/proptest/
