#!/bin/bash
# export-codebase.sh — Export a codebase for LLM context
# Usage: ./export-codebase.sh [OPTIONS] [PROJECT_DIR]
#
# Options:
#   -o, --output FILE    Output file (default: {project}_codebase_{timestamp}.txt)
#   -d, --docs-only      Only export documentation (README, docs/, *.md)
#   -t, --tree-only      Only show project structure, no file contents
#   -e, --exclude GLOB   Exclude pattern (can be used multiple times)
#   -i, --include GLOB   Include only matching patterns
#   -m, --max-size KB    Skip files larger than KB (default: 100)
#   -h, --help           Show this help

set -euo pipefail

# Defaults
PROJECT_DIR="."
OUTPUT_FILE=""
DOCS_ONLY=false
TREE_ONLY=false
MAX_SIZE_KB=100
EXCLUDES=()
INCLUDES=()

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        -d|--docs-only)
            DOCS_ONLY=true
            shift
            ;;
        -t|--tree-only)
            TREE_ONLY=true
            shift
            ;;
        -e|--exclude)
            EXCLUDES+=("$2")
            shift 2
            ;;
        -i|--include)
            INCLUDES+=("$2")
            shift 2
            ;;
        -m|--max-size)
            MAX_SIZE_KB="$2"
            shift 2
            ;;
        -h|--help)
            head -15 "$0" | tail -13
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
elif [[ -f package.json ]]; then
    PROJECT_NAME=$(grep -m1 '"name"' package.json | sed 's/.*: *"\([^"]*\)".*/\1/' || basename "$(pwd)")
else
    PROJECT_NAME=$(basename "$(pwd)")
fi

# Set output file
if [[ -z "$OUTPUT_FILE" ]]; then
    SUFFIX="codebase"
    $DOCS_ONLY && SUFFIX="docs"
    $TREE_ONLY && SUFFIX="tree"
    OUTPUT_FILE="${PROJECT_NAME}_${SUFFIX}_$(date +%Y%m%d_%H%M%S).txt"
fi

# Build find exclusions (always exclude these)
FIND_EXCLUDES="-type d \( -name target -o -name node_modules -o -name .git -o -name __pycache__ -o -name .venv -o -name dist -o -name build \) -prune -o"

# File patterns based on mode
if $DOCS_ONLY; then
    FILE_PATTERNS="-name '*.md' -o -name '*.txt' -o -name '*.rst'"
else
    FILE_PATTERNS="-name '*.rs' -o -name '*.toml' -o -name '*.md' -o -name '*.json' -o -name '*.yaml' -o -name '*.yml' -o -name '*.sh' -o -name '*.py' -o -name '*.js' -o -name '*.ts' -o -name '*.go'"
fi

# Collect files
collect_files() {
    eval "find . $FIND_EXCLUDES -type f \( $FILE_PATTERNS \) -print" 2>/dev/null | \
        grep -v '\.lock$' | \
        grep -v 'package-lock\.json' | \
        sort
}

# Filter by user excludes/includes
filter_files() {
    local files
    files=$(cat)
    
    # Apply excludes
    for pattern in "${EXCLUDES[@]:-}"; do
        [[ -n "$pattern" ]] && files=$(echo "$files" | grep -v "$pattern" || true)
    done
    
    # Apply includes (if any specified, only keep matching)
    if [[ ${#INCLUDES[@]} -gt 0 ]]; then
        local filtered=""
        for pattern in "${INCLUDES[@]}"; do
            filtered+=$(echo "$files" | grep "$pattern" || true)
            filtered+=$'\n'
        done
        files=$(echo "$filtered" | sort -u | grep -v '^$')
    fi
    
    echo "$files"
}

# Get file list
FILES=$(collect_files | filter_files)

# Start output
{
    echo "# $PROJECT_NAME — Codebase Export"
    echo ""
    echo "Generated: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
    echo "Directory: $(pwd)"
    echo ""
    
    # Project structure
    echo "## Project Structure"
    echo ""
    echo '```'
    if command -v tree &>/dev/null; then
        tree -I 'target|node_modules|.git|__pycache__|.venv|dist|build' -L 3 --noreport 2>/dev/null || find . -type d | head -50
    else
        find . -type d $FIND_EXCLUDES -print 2>/dev/null | head -50 | sed 's|^\./||'
    fi
    echo '```'
    echo ""
    
    # Table of contents with sizes
    echo "## Files ($(echo "$FILES" | wc -l | tr -d ' ') total)"
    echo ""
    echo "| File | Size | Lines |"
    echo "|------|------|-------|"
    
    TOTAL_LINES=0
    TOTAL_SIZE=0
    
    while IFS= read -r file; do
        [[ -z "$file" ]] && continue
        [[ ! -f "$file" ]] && continue
        
        size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
        lines=$(wc -l < "$file" 2>/dev/null | tr -d ' ' || echo 0)
        
        # Skip if over max size
        size_kb=$((size / 1024))
        if [[ $size_kb -gt $MAX_SIZE_KB ]]; then
            echo "| ${file#./} | ${size_kb}KB (skipped) | $lines |"
            continue
        fi
        
        TOTAL_LINES=$((TOTAL_LINES + lines))
        TOTAL_SIZE=$((TOTAL_SIZE + size))
        
        if [[ $size -gt 1024 ]]; then
            echo "| ${file#./} | ${size_kb}KB | $lines |"
        else
            echo "| ${file#./} | ${size}B | $lines |"
        fi
    done <<< "$FILES"
    
    echo ""
    echo "**Total:** $TOTAL_LINES lines, $((TOTAL_SIZE / 1024))KB"
    echo "**Estimated tokens:** ~$((TOTAL_LINES * 2)) (rough: 2 tokens/line)"
    echo ""
    
    # Stop here for tree-only mode
    if $TREE_ONLY; then
        echo "---"
        echo "*Tree-only mode — file contents not included*"
        exit 0
    fi
    
    echo "---"
    echo ""
    echo "## File Contents"
    echo ""
    
    # Add each file
    while IFS= read -r file; do
        [[ -z "$file" ]] && continue
        [[ ! -f "$file" ]] && continue
        
        size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
        size_kb=$((size / 1024))
        
        # Skip if over max size
        if [[ $size_kb -gt $MAX_SIZE_KB ]]; then
            continue
        fi
        
        # Determine language for syntax highlighting
        ext="${file##*.}"
        case "$ext" in
            rs) lang="rust" ;;
            toml) lang="toml" ;;
            md) lang="markdown" ;;
            json) lang="json" ;;
            yaml|yml) lang="yaml" ;;
            sh) lang="bash" ;;
            py) lang="python" ;;
            js) lang="javascript" ;;
            ts) lang="typescript" ;;
            go) lang="go" ;;
            *) lang="" ;;
        esac
        
        echo "### ${file#./}"
        echo ""
        echo "\`\`\`$lang"
        cat "$file"
        echo ""
        echo "\`\`\`"
        echo ""
    done <<< "$FILES"
    
    echo "---"
    echo "*End of export*"
    
} > "$OUTPUT_FILE"

# Summary
echo "✓ Exported to: $OUTPUT_FILE"
echo "  Size: $(du -h "$OUTPUT_FILE" | cut -f1)"
echo "  Lines: $(wc -l < "$OUTPUT_FILE" | tr -d ' ')"
echo "  Est. tokens: ~$((TOTAL_LINES * 2))"
