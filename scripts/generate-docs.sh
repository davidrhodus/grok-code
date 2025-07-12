#!/bin/bash

# Script to generate API documentation for grok-code

set -e

echo "📚 Generating API documentation..."

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Build documentation
echo -e "${BLUE}Building documentation...${NC}"
RUSTDOCFLAGS="--html-in-header scripts/doc-header.html" cargo doc --no-deps --all-features

# Open documentation in browser if requested
if [ "$1" = "--open" ]; then
    echo -e "${GREEN}Opening documentation in browser...${NC}"
    cargo doc --open --no-deps
fi

# Generate documentation with private items if requested
if [ "$1" = "--private" ]; then
    echo -e "${YELLOW}Building documentation with private items...${NC}"
    cargo doc --no-deps --all-features --document-private-items
fi

# If GitHub Pages deployment is requested
if [ "$1" = "--deploy" ]; then
    echo -e "${YELLOW}Preparing for GitHub Pages deployment...${NC}"
    
    # Create docs directory if it doesn't exist
    mkdir -p docs
    
    # Copy generated documentation
    cp -r target/doc/* docs/
    
    # Create index.html redirect to main crate
    cat > docs/index.html << EOF
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url=grok_code/index.html">
    <title>Redirecting to grok-code documentation</title>
</head>
<body>
    <p>Redirecting to <a href="grok_code/index.html">grok-code documentation</a>...</p>
</body>
</html>
EOF
    
    echo -e "${GREEN}Documentation prepared in 'docs' directory${NC}"
    echo "To deploy:"
    echo "1. Commit the 'docs' directory"
    echo "2. Enable GitHub Pages in repository settings"
    echo "3. Set source to 'docs' directory"
fi

echo -e "${GREEN}✅ Documentation generation complete!${NC}"
echo ""
echo "Documentation location: target/doc/grok_code/index.html"
echo ""
echo "Usage:"
echo "  ./scripts/generate-docs.sh          # Generate docs"
echo "  ./scripts/generate-docs.sh --open   # Generate and open in browser"
echo "  ./scripts/generate-docs.sh --private # Include private items"
echo "  ./scripts/generate-docs.sh --deploy  # Prepare for GitHub Pages" 