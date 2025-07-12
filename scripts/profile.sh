#!/bin/bash

# Script to profile grok-code performance using cargo flamegraph

echo "🔥 Performance Profiling for grok-code"
echo ""

# Check if flamegraph is installed
if ! command -v cargo-flamegraph &> /dev/null; then
    echo "❌ cargo-flamegraph is not installed."
    echo "📦 Installing cargo-flamegraph..."
    cargo install flamegraph
fi

# Check if perf is available (Linux only)
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    if ! command -v perf &> /dev/null; then
        echo "⚠️  Warning: 'perf' is not installed. Flamegraph may not work properly."
        echo "   Install perf with: sudo apt-get install linux-tools-common linux-tools-generic"
    fi
fi

# Create a test scenario for profiling
echo "📝 Creating test scenario..."
cat > /tmp/grok_profile_test.txt << 'EOF'
Analyze the codebase and find all error handling patterns.
Then search for TODO comments.
Finally, read the main.rs file and summarize its structure.
EOF

echo "🚀 Running performance profiling..."
echo ""

# Build in release mode with debug symbols
echo "Building release binary with debug symbols..."
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release

# Run different profiling scenarios
echo ""
echo "Choose a profiling scenario:"
echo "1. Codebase analysis (file scanning)"
echo "2. API interaction simulation"
echo "3. Tool execution (file operations)"
echo "4. Complete workflow (all tools)"
read -p "Enter choice (1-4): " choice

case $choice in
    1)
        echo "Profiling codebase analysis..."
        cargo flamegraph --release -- prompt -p "Analyze the entire codebase structure"
        ;;
    2)
        echo "Profiling API interaction..."
        # Use dry-run to avoid actual API calls
        cargo flamegraph --release -- --dry-run prompt -p "Explain how to implement a REST API"
        ;;
    3)
        echo "Profiling tool execution..."
        cargo flamegraph --release -- --dry-run prompt -p "Read all Rust files and count lines of code"
        ;;
    4)
        echo "Profiling complete workflow..."
        cargo flamegraph --release -- --dry-run automate -p "$(cat /tmp/grok_profile_test.txt)"
        ;;
    *)
        echo "Invalid choice. Running default codebase analysis..."
        cargo flamegraph --release -- prompt -p "Analyze the entire codebase structure"
        ;;
esac

echo ""
echo "✅ Profiling complete!"
echo "📊 Flamegraph generated: flamegraph.svg"
echo ""
echo "💡 Tips for analyzing the flamegraph:"
echo "   - Wide bars indicate functions that take more time"
echo "   - Click on bars to zoom in"
echo "   - Look for unexpected hotspots"
echo "   - Common bottlenecks: file I/O, regex parsing, API serialization"
echo ""
echo "🌐 Open the flamegraph in your browser:"
echo "   open flamegraph.svg"

# Clean up
rm -f /tmp/grok_profile_test.txt 