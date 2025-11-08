# Testing Best Practices

**Supplementary guide to [TESTING.md](TESTING.md)** — deep-dive on assertion patterns, anti-patterns, and decision trees.

This document provides detailed guidance on specific testing patterns for robust, maintainable tests in wasm-slim. For general testing information (running tests, organization, quick start), see [TESTING.md](TESTING.md).

## Table of Contents

1. [Floating-Point Comparisons](#floating-point-comparisons)
2. [Size Assertions](#size-assertions)
3. [When to Use Exact vs Approximate Comparisons](#when-to-use-exact-vs-approximate-comparisons)
4. [Test Organization](#test-organization)
5. [Property-Based Testing](#property-based-testing)
6. [Integration Test Structure](#integration-test-structure)
7. [Examples](#examples)

---

## Floating-Point Comparisons

### ❌ Don't: Use exact equality for floats

```rust
// WRONG: Brittle due to floating-point precision
assert_eq!(ratio, 0.75);
assert_eq!(percentage, 12.5);
```

**Why?** Floating-point arithmetic can introduce tiny precision errors:
- `0.75` might be `0.7500000001` or `0.7499999999`
- Different platforms (32-bit vs 64-bit) may produce slightly different results
- Compiler optimization levels can affect precision

### ✅ Do: Use approximate comparisons

```rust
use common::assertions::assert_approx_eq;

// CORRECT: Tolerant of precision errors
assert_approx_eq(ratio, 0.75, 0.01);  // ±1% tolerance
assert_approx_eq(percentage, 12.5, 0.1);  // ±0.1 absolute tolerance
```

---

## Size Assertions

### ❌ Don't: Use exact equality for calculated sizes

```rust
// WRONG: Fragile - small code changes break tests
assert_eq!(bundle_size, 1048576);
assert_eq!(optimized_size, 524288);
```

**Why?** Size calculations may vary due to:
- Platform differences (pointer sizes, alignment)
- Compiler optimization levels
- Small code changes that don't affect functionality
- Metadata or debug info variations

### ✅ Do: Use tolerance-based comparisons

```rust
use common::assertions::assert_size_within;

// CORRECT: Allow reasonable variance
assert_size_within(bundle_size, 1048576, 1024);  // ±1KB tolerance
assert_size_within(optimized_size, 524288, 512);  // ±512 bytes
```

---

## When to Use Exact vs Approximate Comparisons

### Use Exact Comparisons (`assert_eq!`) When:

1. **Testing parsing logic with deterministic inputs:**
   ```rust
   // String "2.5%" should parse to exactly 2.5
   assert_eq!(parse_percentage("2.5%"), 2.5);
   ```

2. **Testing exact mathematical operations:**
   ```rust
   // 50% reduction is mathematically exact
   let before = 1024 * 1024;
   let after = 512 * 1024;
   assert_eq!((before - after) * 100 / before, 50);
   ```

3. **Testing counters and discrete values:**
   ```rust
   assert_eq!(asset_count, 10);
   assert_eq!(error_count, 0);
   ```

### Use Approximate Comparisons When:

1. **Testing real-world measurements:**
   ```rust
   // Actual file sizes from filesystem
   assert_size_within(file_size, expected, 1024);
   ```

2. **Testing calculations involving division:**
   ```rust
   // Division can introduce precision errors
   let ratio = total as f64 / count as f64;
   assert_approx_eq(ratio, 0.75, 0.01);
   ```

3. **Testing platform-dependent code:**
   ```rust
   // Memory layout may vary by platform
   assert_size_within(struct_size, 128, 16);
   ```

4. **Testing performance metrics:**
   ```rust
   // Performance varies by system load
   assert_percent_within(improvement, 30.0, 5.0);  // ±5% tolerance
   ```

**Helper Functions:** See [TESTING.md - Test Helpers](TESTING.md#test-helpers) for the API reference and basic usage examples.

---

## Test Organization

### Best Practice: Organize Complex Modules into Logical Sub-Modules

For modules with extensive test suites (50+ tests), organize tests into focused sub-modules by functionality.

#### ✅ Exemplary Pattern: BackupManager (src/optimizer/backup.rs)

The BackupManager module organizes 73 tests into 4 logical sub-modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod basic_operations {
        use super::*;

        #[test]
        fn test_create_backup() { /* ... */ }

        #[test]
        fn test_list_backups() { /* ... */ }

        #[test]
        fn test_restore_backup() { /* ... */ }
    }

    mod error_handling {
        use super::*;

        #[test]
        fn test_backup_nonexistent_file() { /* ... */ }

        #[test]
        fn test_restore_with_permission_error() { /* ... */ }
    }

    mod concurrency {
        use super::*;

        #[test]
        fn test_concurrent_backups() { /* ... */ }

        #[test]
        fn test_race_condition_handling() { /* ... */ }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_empty_file_backup() { /* ... */ }

        #[test]
        fn test_very_long_filename() { /* ... */ }
    }
}
```

#### Benefits:

- **Discoverability**: Easy to find tests for specific functionality
- **Maintainability**: Related tests are grouped together
- **Readability**: Clear structure shows what aspects are tested
- **Parallel Execution**: Cargo can run modules in parallel

#### When to Use Sub-Modules:

- Module has 20+ tests
- Tests cover distinct functionality areas (CRUD, error handling, edge cases, etc.)
- Multiple developers work on the module

#### When NOT to Use Sub-Modules:

- Module has <20 tests (flat structure is clearer)
- All tests are testing the same type of functionality
- Tests are simple unit tests without clear groupings

---

## Property-Based Testing

### Using Proptest for Invariant Testing

Property-based testing verifies that code maintains invariants across a wide range of inputs, catching edge cases that example-based tests would miss.

#### ✅ Exemplary Pattern: Assets Module (src/analyzer/assets.rs)

The assets module uses `proptest` to verify important invariants:

```rust
use proptest::prelude::*;

proptest! {
    /// Priority should monotonically increase with asset size
    #[test]
    fn prop_priority_monotonic_with_size(
        small in 0u64..1000,
        large in 1000u64..1000000,
        bundle_size in 10000u64..10000000
    ) {
        let small_priority = AssetPriority::from_size(small, bundle_size);
        let large_priority = AssetPriority::from_size(large, bundle_size);

        // Larger assets should have >= priority (never lower)
        prop_assert!(large_priority >= small_priority);
    }

    /// Assets over 500KB should always be critical
    #[test]
    fn prop_critical_above_500kb_large_assets_always_critical(
        size in 500_001u64..10_000_000,
        bundle_size in 500_000u64..100_000_000
    ) {
        let priority = AssetPriority::from_size(size, bundle_size);
        prop_assert_eq!(priority, AssetPriority::Critical);
    }
}
```

#### When to Use Property Tests:

1. **Numerical Logic**: Algorithms with mathematical invariants
2. **Parsing**: Verifying round-trip properties (parse → serialize → parse)
3. **Ordering**: Verifying comparison operators are transitive
4. **Bounds Checking**: Ensuring values stay within valid ranges

#### Common Property Patterns:

**Monotonicity:**
```rust
prop_assert!(larger_input >= smaller_input => larger_output >= smaller_output);
```

**Idempotence:**
```rust
prop_assert_eq!(f(f(x)), f(x));  // Applying twice = applying once
```

**Round-trip:**
```rust
prop_assert_eq!(parse(serialize(x)), x);
```

**Inverse:**
```rust
prop_assert_eq!(encrypt(decrypt(x, key), key), x);
```

#### Recommended Strategies:

- Start with 2-3 property tests per module with complex logic
- Use constrained generators (`0u64..1000`) to avoid overflow
- Test edge cases explicitly, use properties for general behavior
- Keep property tests simple - if logic is complex, add example tests too

---

## Integration Test Structure

### Organizing Integration Tests by Feature

Integration tests are organized into separate files by feature area, with shared utilities in `tests/common/`.

#### Current Structure:

```
tests/
├── common/
│   ├── mod.rs              # Re-exports
│   ├── assertions.rs       # Test helpers (assert_approx_eq, etc.)
│   └── fixtures.rs         # Test fixtures and setup
├── analyze_tests.rs        # Tests for `wasm-slim analyze` command
├── apply_suggestions_integration.rs  # Dependency analysis & fixes
├── bloat_integration.rs    # Bloat analysis integration
├── build_tests.rs          # Tests for `wasm-slim build` command
├── cleanup_tests.rs        # Cleanup and backup tests
├── cli_integration.rs      # CLI argument parsing tests
├── cli_tests.rs            # CLI end-to-end workflows
├── compare_tests.rs        # Tests for `wasm-slim compare` command
├── init_tests.rs           # Tests for `wasm-slim init` command
├── pipeline_integration.rs # Build pipeline integration tests
├── timeout_integration.rs  # Timeout and cancellation tests
├── windows_platform_integration.rs  # Windows-specific tests
└── workflow_tests.rs       # Complete optimization workflows
```

#### Guidelines:

**1. One Feature Per File:**
```rust
// ✅ GOOD: bloat_integration.rs
#[test]
fn test_bloat_analysis_with_real_wasm() { }

#[test]
fn test_bloat_json_output() { }

// ❌ BAD: kitchen_sink_tests.rs with unrelated tests
```

**2. Use Common Test Utilities:**
```rust
mod common;
use common::fixtures::create_minimal_wasm_lib;
use common::assertions::assert_size_within;
```

**3. Skip Gracefully When Tools Missing:**
```rust
macro_rules! require_wasm_tools {
    () => {
        if !has_required_wasm_tools() {
            eprintln!("⚠️  Skipping test: wasm-bindgen-cli not found");
            eprintln!("   Install with: cargo install wasm-bindgen-cli");
            return;
        }
    };
}

#[test]
fn test_wasm_workflow() {
    require_wasm_tools!();
    // Test code...
}
```

**Note:** While test skipping is pragmatic for local development, CI environments should have all required tools installed and fail if tools are missing. See issue P2-TEST-WASM_SLIM-INTEGRATION-004 for recommended CI improvements.

**4. Clean Up Resources:**
```rust
#[test]
fn test_creates_temp_files() {
    let temp_dir = TempDir::new().unwrap();  // Auto-cleaned on drop
    let project_root = temp_dir.path();

    // Test code...

    // temp_dir dropped here, cleaning up automatically
}
```

---

## Examples

### Example 1: Testing Size Optimization

```rust
mod common;
use common::assertions::*;

#[test]
fn test_wasm_optimization() {
    let before_size = measure_bundle_size();
    optimize_bundle();
    let after_size = measure_bundle_size();
    
    // Use approximate comparison for real measurements
    let reduction = ((before_size - after_size) as f64 / before_size as f64) * 100.0;
    assert_percent_within(reduction, 30.0, 2.0);  // Expect ~30% ±2%
}
```

### Example 2: Testing Parsing Logic

```rust
#[test]
fn test_parse_bloat_output() {
    let line = "12.5KiB  2.5% std::fmt::write";
    let item = parse_bloat_line(line).unwrap();
    
    // Exact comparison OK - parsing is deterministic
    assert_eq!(item.percentage, 2.5);
    assert_eq!(item.name, "std::fmt::write");
}
```

### Example 3: Testing Calculated Metrics

```rust
mod common;
use common::assertions::*;

#[test]
fn test_bundle_analysis() {
    let results = analyze_bundle("test.wasm");
    
    // Exact comparison for counts
    assert_eq!(results.function_count, 150);
    
    // Approximate for calculated percentages
    assert_approx_eq(results.dead_code_percent, 5.2, 0.5);  // ±0.5%
    
    // Tolerance for sizes
    assert_size_within(results.total_size, 1048576, 1024);  // ±1KB
}
```

---

## Decision Tree

```
Is this a float comparison?
├─ YES: Is it from parsing deterministic input?
│   ├─ YES: Use assert_eq!
│   └─ NO: Use assert_approx_eq
└─ NO: Is this a size/bytes comparison?
    ├─ YES: Is it from a calculation or measurement?
    │   ├─ YES: Use assert_size_within
    │   └─ NO: Use assert_eq!
    └─ NO: Use assert_eq!
```

---

## Common Pitfalls

### Pitfall 1: Over-using approximate comparisons

```rust
// WRONG: Parsing should be exact
assert_approx_eq(parse_number("42"), 42.0, 0.1);

// RIGHT: Parsing is deterministic
assert_eq!(parse_number("42"), 42.0);
```

### Pitfall 2: Too tight tolerances

```rust
// WRONG: Too tight, will fail on some platforms
assert_approx_eq(calculated_value, 0.75, 0.0001);

// RIGHT: Reasonable tolerance for the domain
assert_approx_eq(calculated_value, 0.75, 0.01);
```

### Pitfall 3: Too loose tolerances

```rust
// WRONG: Tolerance so large it accepts incorrect values
assert_size_within(bundle_size, 1048576, 1000000);  // ±1MB = meaningless

// RIGHT: Tolerance appropriate for the measurement
assert_size_within(bundle_size, 1048576, 1024);  // ±1KB
```

---

## References

- [Floating Point Guide](https://floating-point-gui.de/)
- [Rust API Guidelines on Testing](https://rust-lang.github.io/api-guidelines/)
- [`approx` crate documentation](https://docs.rs/approx/) (if we add it in the future)
