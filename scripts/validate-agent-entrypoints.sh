#!/usr/bin/env bash
set -euo pipefail

# scripts/validate-agent-entrypoints.sh
# Validates that assistant-specific entrypoints follow the reference model.

AGENT_FILES=("CLAUDE.md" "GEMINI.md" "QWEN.md")
EXPECTED_PREFIX="@AGENTS.md"
EXIT_CODE=0

echo "Checking agent entrypoints..."

for file in "${AGENT_FILES[@]}"; do
    if [[ ! -f "$file" ]]; then
        echo "❌ Error: Required agent entrypoint '$file' is missing."
        EXIT_CODE=1
        continue
    fi

    # Check if file starts with @AGENTS.md
    FIRST_LINE=$(head -n 1 "$file")

    if [[ "$FIRST_LINE" == "$EXPECTED_PREFIX" ]]; then
        echo "✅ $file follows the reference model (starts with $EXPECTED_PREFIX)."

        # Check if the file contains only approved content or at least doesn't duplicate common guidelines
        # We check for basic existence of content after the prefix
        if [[ $(wc -l < "$file") -gt 1 ]]; then
             echo "   (Note: $file contains assistant-specific instructions)"
        fi
    else
        echo "❌ Error: $file is not a valid agent entrypoint."
        echo "Expected first line to be: '$EXPECTED_PREFIX'"
        echo "Actual first line was: '$FIRST_LINE'"
        EXIT_CODE=1
    fi
done

if [[ $EXIT_CODE -eq 0 ]]; then
    echo "All agent entrypoints are valid."
else
    echo "Validation failed. Please ensure assistant-specific files start with '@AGENTS.md'."
fi

# Return exit code
(( EXIT_CODE == 0 )) || false
