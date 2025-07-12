#!/bin/bash

# Script to run code coverage analysis using cargo-tarpaulin

echo "🔍 Running code coverage analysis..."

# Check if tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "❌ cargo-tarpaulin is not installed."
    echo "📦 Installing cargo-tarpaulin..."
    cargo install cargo-tarpaulin
fi

# Run tarpaulin with various output formats
echo "🚀 Running tests with coverage..."

# Default coverage run
cargo tarpaulin \
    --verbose \
    --all-features \
    --workspace \
    --timeout 120 \
    --out Html \
    --out Lcov \
    --output-dir ./target/coverage

# Calculate coverage percentage
COVERAGE=$(cargo tarpaulin --print-summary 2>&1 | grep -oP '\d+\.\d+%' | head -1)

echo ""
echo "✅ Coverage analysis complete!"
echo "📊 Coverage: $COVERAGE"
echo ""
echo "📁 Coverage reports generated:"
echo "   - HTML report: target/coverage/tarpaulin-report.html"
echo "   - LCOV report: target/coverage/lcov.info"
echo ""
echo "💡 Open the HTML report in your browser to see detailed coverage:"
echo "   open target/coverage/tarpaulin-report.html" 