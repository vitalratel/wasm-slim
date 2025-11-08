#!/bin/bash
# Comprehensive local CI checks (runs same checks as GitHub Actions)
# Run this before pushing important changes to main branch

set -e  # Exit on first error

echo "🔍 Running comprehensive CI checks locally..."
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Cargo.toml not found. Run this from project root."
    exit 1
fi

# 1. Formatting check
echo "1️⃣  Checking code formatting..."
if cargo fmt --all -- --check; then
    echo "   ✅ Formatting check passed"
else
    echo "   ❌ Formatting check failed"
    echo "   Fix with: cargo fmt --all"
    exit 1
fi
echo ""

# 2. Clippy lints (all targets)
echo "2️⃣  Running Clippy lints (all targets)..."
if cargo clippy --all-features --all-targets -- -D warnings; then
    echo "   ✅ Clippy passed"
else
    echo "   ❌ Clippy found issues"
    exit 1
fi
echo ""

# 3. Unit tests
echo "3️⃣  Running unit tests..."
if cargo test --lib --all-features; then
    echo "   ✅ Unit tests passed"
else
    echo "   ❌ Unit tests failed"
    exit 1
fi
echo ""

# 4. Integration tests
echo "4️⃣  Running integration tests..."
if cargo test --tests 2>&1 | tee /tmp/ci_test_output.log | grep -q "Skipping test"; then
    echo "   ⚠️  Some integration tests skipped (missing wasm-bindgen-cli or other tools)"
    # Check if tests actually failed vs just skipped
    if grep -q "test result:.*FAILED" /tmp/ci_test_output.log; then
        echo "   ❌ Integration tests failed"
        rm -f /tmp/ci_test_output.log
        exit 1
    fi
    echo "   ✅ Integration tests passed (some skipped)"
elif cargo test --tests; then
    echo "   ✅ Integration tests passed"
else
    echo "   ❌ Integration tests failed"
    exit 1
fi
rm -f /tmp/ci_test_output.log
echo ""

# 5. Doc tests
echo "5️⃣  Running doc tests..."
if cargo test --doc --all-features; then
    echo "   ✅ Doc tests passed"
else
    echo "   ❌ Doc tests failed"
    exit 1
fi
echo ""

# 6. Benchmark compilation check
echo "6️⃣  Checking benchmarks compile..."
if cargo bench --no-run; then
    echo "   ✅ Benchmarks compile"
else
    echo "   ❌ Benchmark compilation failed"
    exit 1
fi
echo ""

# 7. Check Cargo.lock is up to date
echo "7️⃣  Checking Cargo.lock is up to date..."
if cargo update --workspace --locked 2>&1 | grep -q "error:"; then
    echo "   ❌ Cargo.lock is out of date"
    echo "   Fix with: cargo update --workspace"
    exit 1
else
    echo "   ✅ Cargo.lock is up to date"
fi
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ All CI checks passed!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 Optional: Run './scripts/full-ci-with-act.sh' for complete platform testing"
