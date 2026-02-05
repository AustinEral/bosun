#!/bin/bash
# export-docs.sh — Export project documentation for LLM context
# Gathers: README, SPEC, CHANGELOG, docs/, inline rustdoc, etc.
#
# Usage: ./export-docs.sh [OPTIONS] [PROJECT_DIR]
#
# Options:
#   -o, --output FILE    Output file (default: {project}_docs_{timestamp}.txt)
#   -r, --rustdoc        Include extracted rustdoc comments from source
#   -a, --api            Include API signatures (fn/struct/enum definitions)
#   -h, --help           Show this help

set -euo pipefail

PROJECT_DIR="."
OUTPUT_FILE=""
INCLUDE_RUSTDOC=false
INCLUDE_API=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        -r|--rustdoc)
            INCLUDE_RUSTDOC=true
            shift
            ;;
        -a|--api)
            INCLUDE_API=true
            shift
            ;;
        -h|--help)
            head -13 "$0" | tail -11
            exit 0
            ;;
        *)
            PROJECT_DIR="$1"
            shift
            ;;
    esac
done

cd "$PROJECT_DIR"

# Detect project name
if [[ -f Cargo.toml ]]; then
    PROJECT_NAME=$(grep -m1 '^name' Cargo.toml | sed 's/.*= *"\([^"]*\)".*/\1/' || basename "$(pwd)")
else
    PROJECT_NAME=$(basename "$(pwd)")
fi

[[ -z "$OUTPUT_FILE" ]] && OUTPUT_FILE="${PROJECT_NAME}_docs_$(date +%Y%m%d_%H%M%S).txt"

{
    echo "# $PROJECT_NAME — Documentation Export"
    echo ""
    echo "Generated: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
    echo ""
    
    # Key documentation files
    echo "## Overview Documents"
    echo ""
    
    for doc in README.md README SPEC.md DESIGN.md ARCHITECTURE.md CHANGELOG.md CONTRIBUTING.md LICENSE; do
        if [[ -f "$doc" ]]; then
            echo "### $doc"
            echo ""
            cat "$doc"
            echo ""
            echo "---"
            echo ""
        fi
    done
    
    # docs/ directory
    if [[ -d docs ]]; then
        echo "## Documentation (docs/)"
        echo ""
        find docs -name '*.md' -type f | sort | while read -r file; do
            echo "### ${file}"
            echo ""
            cat "$file"
            echo ""
            echo "---"
            echo ""
        done
    fi
    
    # Cargo.toml metadata
    if [[ -f Cargo.toml ]]; then
        echo "## Cargo.toml"
        echo ""
        echo '```toml'
        cat Cargo.toml
        echo '```'
        echo ""
    fi
    
    # Extract rustdoc comments
    if $INCLUDE_RUSTDOC && [[ -d src ]]; then
        echo "## Rustdoc Comments"
        echo ""
        echo "*Extracted /// and //! comments from source files*"
        echo ""
        
        find src -name '*.rs' -type f | sort | while read -r file; do
            # Extract doc comments
            docs=$(grep -E '^\s*(///|//!)' "$file" 2>/dev/null | sed 's/^\s*//' || true)
            if [[ -n "$docs" ]]; then
                echo "### ${file#./}"
                echo ""
                echo '```'
                echo "$docs"
                echo '```'
                echo ""
            fi
        done
    fi
    
    # Extract API signatures
    if $INCLUDE_API && [[ -d src ]]; then
        echo "## API Overview"
        echo ""
        echo "*Public type and function signatures*"
        echo ""
        
        find src -name '*.rs' -type f | sort | while read -r file; do
            # Extract pub fn, pub struct, pub enum, pub trait signatures
            sigs=$(grep -E '^\s*pub\s+(fn|struct|enum|trait|type|const|static|mod)\s+' "$file" 2>/dev/null || true)
            if [[ -n "$sigs" ]]; then
                echo "### ${file#./}"
                echo ""
                echo '```rust'
                echo "$sigs"
                echo '```'
                echo ""
            fi
        done
    fi
    
    echo "---"
    echo "*End of documentation export*"
    
} > "$OUTPUT_FILE"

echo "✓ Exported to: $OUTPUT_FILE"
echo "  Size: $(du -h "$OUTPUT_FILE" | cut -f1)"
echo "  Lines: $(wc -l < "$OUTPUT_FILE" | tr -d ' ')"
