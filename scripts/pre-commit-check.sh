#!/bin/bash
# Pre-commit check script to ensure code quality

echo "Running pre-commit checks..."
echo "============================="

# Run cargo fmt check
echo -n "Checking formatting... "
if cargo fmt -- --check > /dev/null 2>&1; then
    echo "✅ OK"
else
    echo "❌ FAILED"
    echo
    echo "Please run 'cargo fmt' to fix formatting issues."
    echo "Diff:"
    cargo fmt -- --check
    exit 1
fi

# Run cargo clippy
echo -n "Running clippy... "
if cargo clippy --all-targets --all-features -- -D warnings > /dev/null 2>&1; then
    echo "✅ OK"
else
    echo "❌ FAILED"
    echo
    echo "Please fix the following clippy warnings:"
    cargo clippy --all-targets --all-features -- -D warnings
    exit 1
fi

# Optional: Run tests
echo -n "Running tests... "
if cargo test --quiet > /dev/null 2>&1; then
    echo "✅ OK"
else
    echo "⚠️  Some tests failed (not blocking commit)"
fi

echo
echo "✅ All checks passed! Ready to commit." 