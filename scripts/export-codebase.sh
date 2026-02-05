#!/bin/bash
# export-codebase.sh — Simple codebase export for LLM context
# Usage: ./export-codebase.sh [PROJECT_DIR]

set -e

PROJECT_DIR="${1:-.}"
cd "$PROJECT_DIR"

PROJECT_NAME=$(basename "$(pwd)")
OUTPUT_FILE="${PROJECT_NAME}_codebase_$(date +%Y%m%d_%H%M%S).txt"

{
    echo "# $PROJECT_NAME"
    echo ""
    echo "Generated: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
    echo ""

    # Structure
    echo "## Structure"
    echo ""
    find . -name '*.rs' -type f | grep -v '/target/' | sort | sed 's|^\./||'
    echo ""

    # Cargo.toml
    if [[ -f Cargo.toml ]]; then
        echo "## Cargo.toml"
        echo ""
        echo '```toml'
        cat Cargo.toml
        echo '```'
        echo ""
    fi

    # All Rust files
    echo "## Source"
    echo ""

    find . -name '*.rs' -type f | grep -v '/target/' | sort | while read -r file; do
        echo "### ${file#./}"
        echo ""
        echo '```rust'
        cat "$file"
        echo '```'
        echo ""
    done

} > "$OUTPUT_FILE"

echo "Exported to: $OUTPUT_FILE"
echo "Size: $(du -h "$OUTPUT_FILE" | cut -f1)"
