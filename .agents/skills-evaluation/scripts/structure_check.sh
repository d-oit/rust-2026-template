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
    has_name=$(grep -cE "^  ?name:" "$skill_md" || true)
    has_desc=$(grep -cE "^  ?description:" "$skill_md" || true)
    has_cat=$(grep -cE "^  ?category:" "$skill_md" || true)

    # Section checks. Rationalizations/Red Flags score because the 2026
    # SKILL.md smell study (arXiv:2607.01456, 238 skills) found the absence of
    # rationalization-suppression guidance to be the single most prevalent
    # defect at 94%. A "When to Use" section is deliberately NOT scored: the
    # body loads only after the tier-1 description has already selected the
    # skill, so it cannot influence triggering.
    has_rational=$(grep -cE "^## Rationalizations" "$skill_md" || true)
    has_flags=$(grep -cE "^## Red Flags" "$skill_md" || true)

    # Spec conformance: name must match the directory, description <= 1024 chars.
    fm_name=$(awk '/^---$/{n++} n==1 && /^name:/{sub(/^name:[[:space:]]*/,""); gsub(/"/,""); print; exit}' "$skill_md")
    desc_len=$(awk '/^---$/{n++} n==1 && /^description:/{sub(/^description:[[:space:]]*/,""); print; exit}' "$skill_md" | wc -c)
    name_matches_dir=$([ "$fm_name" = "$skill" ] && echo 1 || echo 0)
    desc_in_spec=$([ "$desc_len" -le 1025 ] && echo 1 || echo 0)

    # Body size. The open spec recommends <500 lines; this repo budgets 250 so
    # the always-loaded instruction set stays small.
    body_lines=$(awk 'NR==1 && /^---[[:space:]]*$/ {fm=1; next} fm && !seen && /^---[[:space:]]*$/ {seen=1; next} seen {c++} END {print c+0}' "$skill_md")

    # Evals
    eval_count=0
    assertion_count=0
    if [ -f "$evals_json" ]; then
        eval_count=$(jq '.evals | length' "$evals_json" 2>/dev/null)
        [ -z "$eval_count" ] && eval_count=0
        assertion_count=$(jq '[.evals[].assertions // [] | length] | add // 0' "$evals_json" 2>/dev/null)
        [ -z "$assertion_count" ] && assertion_count=0
    fi
    # Calculate score (9 signals)
    score=0
    [ "$has_name" -ge 1 ] && score=$((score + 1))
    [ "$has_desc" -ge 1 ] && score=$((score + 1))
    [ "$has_cat" -ge 1 ] && score=$((score + 1))
    [ "$has_rational" -ge 1 ] && score=$((score + 1))
    [ "$has_flags" -ge 1 ] && score=$((score + 1))
    [ "$name_matches_dir" -eq 1 ] && score=$((score + 1))
    [ "$desc_in_spec" -eq 1 ] && score=$((score + 1))
    [ "$body_within_budget" -eq 1 ] && score=$((score + 1))
    [ "$eval_count" -ge 3 ] && score=$((score + 1))

    verdict="PASS"
    [ "$score" -lt 9 ] && verdict="NEEDS_WORK"
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
    "max_score": 9,
    "verdict": "$verdict",
    "has_skill_md": true,
    "has_evals_json": $([ -f "$evals_json" ] && echo "true" || echo "false"),
    "eval_count": $eval_count,
    "assertion_count": $assertion_count,
    "body_lines": $body_lines,
    "body_within_budget": $([ "$body_within_budget" -eq 1 ] && echo "true" || echo "false"),
    "name_matches_dir": $([ "$name_matches_dir" -eq 1 ] && echo "true" || echo "false"),
    "description_chars": $desc_len
  }
EOF
done

echo "]" >> "$RESULTS"
echo "Structure check complete. Results in $RESULTS"
