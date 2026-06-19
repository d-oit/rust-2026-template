#!/bin/bash
# .agents/skills-evaluation/scripts/structure_check.sh

SKILLS_DIR=".agents/skills"
ITERATION_DIR=${1:-".agents/skills-evaluation/iterations/iteration-1"}
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

    # Frontmatter checks
    has_name=$(grep -c "^name:" "$skill_md" 2>/dev/null | head -n 1 | awk '{print $1}')
    [ -z "$has_name" ] && has_name=0
    has_desc=$(grep -c "^description:" "$skill_md" 2>/dev/null | head -n 1 | awk '{print $1}')
    [ -z "$has_desc" ] && has_desc=0
    has_cat=$(grep -c "^category:" "$skill_md" 2>/dev/null | head -n 1 | awk '{print $1}')
    [ -z "$has_cat" ] && has_cat=0
    has_ver=$(grep -c "^version:" "$skill_md" 2>/dev/null | head -n 1 | awk '{print $1}')
    [ -z "$has_ver" ] && has_ver=0

    # Section checks
    has_when=$(grep -ci "## When to Use" "$skill_md" 2>/dev/null | head -n 1 | awk '{print $1}')
    [ -z "$has_when" ] && has_when=0
    has_rational=$(grep -c "^## Rationalizations" "$skill_md" 2>/dev/null | head -n 1 | awk '{print $1}')
    [ -z "$has_rational" ] && has_rational=0
    has_flags=$(grep -c "^## Red Flags" "$skill_md" 2>/dev/null | head -n 1 | awk '{print $1}')
    [ -z "$has_flags" ] && has_flags=0

    # Evals
    eval_count=0
    assertion_count=0
    if [ -f "$evals_json" ]; then
        eval_count=$(jq '.evals | length' "$evals_json" 2>/dev/null | head -n 1 | awk '{print $1}')
        [ -z "$eval_count" ] && eval_count=0
        assertion_count=$(jq '[.evals[].assertions // [] | length] | add // 0' "$evals_json" 2>/dev/null | head -n 1 | awk '{print $1}')
        [ -z "$assertion_count" ] && assertion_count=0
    fi

    # Calculate score
    score=0
    [ "$has_name" -gt 0 ] && score=$((score + 1))
    [ "$has_desc" -gt 0 ] && score=$((score + 1))
    [ "$has_cat" -gt 0 ] && score=$((score + 1))
    [ "$has_ver" -gt 0 ] && score=$((score + 1))
    [ "$has_when" -gt 0 ] && score=$((score + 1))
    [ "$has_rational" -gt 0 ] && score=$((score + 1))
    [ "$has_flags" -gt 0 ] && score=$((score + 1))
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
