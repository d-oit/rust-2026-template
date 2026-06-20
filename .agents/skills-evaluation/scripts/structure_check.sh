#!/bin/bash
# .agents/skills-evaluation/scripts/structure_check.sh

SKILLS_DIR=".agents/skills"
ITERATION_DIR=${1:-".agents/skills-evaluation/iterations/baseline"}
RESULTS="$ITERATION_DIR/structure_check.json"

mkdir -p "$ITERATION_DIR"

echo "[" > "$RESULTS"
first=true

for skill_dir in "$SKILLS_DIR"/*/; do
    skill=$(basename "$skill_dir")
    skill_md="$skill_dir/SKILL.md"
    evals_json="$skill_dir/evals/evals.json"

    if [ ! -f "$skill_md" ]; then
        continue
    fi

    # Frontmatter checks (Handle indentation)
    has_name=$(grep -E "^  ?name:" "$skill_md" | wc -l)
    has_desc=$(grep -E "^  ?description:" "$skill_md" | wc -l)
    has_cat=$(grep -E "^  ?category:" "$skill_md" | wc -l)
    has_ver=$(grep -E "^  ?version:" "$skill_md" | wc -l)

    # Section checks
    has_when=$(grep -Ei "^## When [tT]o Use" "$skill_md" | wc -l)
    has_rational=$(grep -E "^## Rationalizations" "$skill_md" | wc -l)
    has_flags=$(grep -E "^## Red Flags" "$skill_md" | wc -l)

    # Evals
    eval_count=0
    assertion_count=0
    if [ -f "$evals_json" ]; then
        eval_count=$(jq '.evals | length' "$evals_json" 2>/dev/null)
        [ -z "$eval_count" ] && eval_count=0
        assertion_count=$(jq '[.evals[].assertions // [] | length] | add // 0' "$evals_json" 2>/dev/null)
        [ -z "$assertion_count" ] && assertion_count=0
    fi

    # Calculate score
    score=0
    [ "$has_name" -ge 1 ] && score=$((score + 1))
    [ "$has_desc" -ge 1 ] && score=$((score + 1))
    [ "$has_cat" -ge 1 ] && score=$((score + 1))
    [ "$has_ver" -ge 1 ] && score=$((score + 1))
    [ "$has_when" -ge 1 ] && score=$((score + 1))
    [ "$has_rational" -ge 1 ] && score=$((score + 1))
    [ "$has_flags" -ge 1 ] && score=$((score + 1))
    [ "$eval_count" -ge 3 ] && score=$((score + 1))

    verdict="PASS"
    [ "$score" -lt 8 ] && verdict="NEEDS_WORK"
    [ "$score" -lt 5 ] && verdict="FAIL"

    if [ "$first" = true ]; then
        first=false
    else
        echo "," >> "$RESULTS"
    fi

    cat >> "$RESULTS" << EOF
  {
    "skill": "$skill",
    "score": $score,
    "max_score": 8,
    "verdict": "$verdict",
    "has_skill_md": true,
    "has_evals_json": $([ -f "$evals_json" ] && echo "true" || echo "false"),
    "eval_count": $eval_count,
    "assertion_count": $assertion_count
  }
EOF
done

echo "]" >> "$RESULTS"
echo "Structure check complete. Results in $RESULTS"
