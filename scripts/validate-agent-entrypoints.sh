#!/usr/bin/env bash
set -euo pipefail

# scripts/validate-agent-entrypoints.sh
# Validates that assistant-specific entrypoints follow the 3-layer model.

# Root-level adapters
AGENT_FILES=("CLAUDE.md" "GEMINI.md" "QWEN.md")
# Subdirectory adapters
SUB_ADAPTERS=(".claude/rules.md" ".gemini/rules.md" ".qwen/rules.md" ".windsurf/rules.md" ".opencode/rules/rust-rules.md")

EXPECTED_REF="@AGENTS.md"
EXIT_CODE=0

echo "Checking agent entrypoints..."

for file in "${AGENT_FILES[@]}" "${SUB_ADAPTERS[@]}"; do
    if [[ ! -f "$file" ]]; then
        echo "❌ Error: Required agent adapter '$file' is missing."
        EXIT_CODE=1
        continue
    fi

    # Check if file contains @AGENTS.md
    if grep -q "$EXPECTED_REF" "$file"; then
        echo "✅ $file follows the reference model (contains $EXPECTED_REF)."
    else
        echo "❌ Error: $file is not a valid agent adapter."
        echo "Expected file to contain: '$EXPECTED_REF'"
        EXIT_CODE=1
    fi

done

# Check AGENTS.md line count
AGENTS_LOC=$(wc -l < AGENTS.md)
if [[ "$AGENTS_LOC" -le 200 ]]; then
    echo "✅ AGENTS.md length ($AGENTS_LOC) is within the 200 LOC limit."
else
    echo "❌ Error: AGENTS.md is too long ($AGENTS_LOC lines). Must be <= 200."
    EXIT_CODE=1
fi

if [[ $EXIT_CODE -eq 0 ]]; then
    echo "All agent entrypoints are valid."
else
    echo "Validation failed."
fi

# Return exit code
(( EXIT_CODE == 0 )) || false
