---
name: skill-creator
description: Create new skills, modify and improve existing skills, and measure skill performance. Use when users want to create a skill from scratch, edit, or optimize an existing skill, run evals to test a skill, benchmark skill performance with variance analysis, or optimize a skill's description for better triggering accuracy.
category: meta
license: MIT
metadata:
  author: d-oit
  version: "1.0"
  source: d-o-hub/github-template-ai-agents
---

# Skill Creator

Create and improve skills following the Agent Skills specification.
A skill extends agent capabilities with specialized knowledge, workflows, and tools.

## Core Loop

1. **Capture intent** - What should the skill do? When should it trigger?
2. **Write draft** - Create SKILL.md with frontmatter and instructions
3. **Create test cases** - Realistic prompts users would actually say
4. **Run evals** - Test with-skill vs baseline (or old version)
5. **Review results** - Use eval-viewer for human review + benchmarks
6. **Iterate** - Improve based on feedback until satisfied
7. **Optimize description** - Fine-tune frontmatter for better triggering

---

## Skill Specification

### Directory Structure

```
skill-name/
├── SKILL.md          # Required: metadata + instructions
├── scripts/          # Optional: executable code
├── references/       # Optional: documentation
├── assets/           # Optional: templates, resources
└── evals/            # Optional: test cases
```

### Frontmatter Fields

| Field | Required | Constraints |
|-------|----------|-------------|
| `name` | Yes | Max 64 chars. Lowercase letters, numbers, hyphens only. |
| `description` | Yes | Max 1024 chars. Describes what the skill does AND when to use it. |
| `license` | No | License name or reference to bundled license file. |
| `compatibility` | No | Max 500 chars. Environment requirements. |
| `metadata` | No | Arbitrary key-value mapping. |
| `allowed-tools` | No | Space-delimited list of pre-approved tools. |

### SKILL.md Body

- Keep under **250 lines**
- Use progressive disclosure: move detailed content to `references/`
- Include step-by-step instructions, examples, and common edge cases

---

## Optimizing Skill Descriptions

### Core Writing Principles

1. **Use imperative phrasing** — "Use this skill when..." rather than "This skill does..."
2. **Focus on user intent, not implementation** — Describe what the user is trying to achieve
3. **Err on the side of being pushy** — Explicitly list contexts where the skill applies
4. **Keep it concise** — A few sentences; max 1024 characters

### Testing & Evaluation

1. **Design trigger eval queries** — Create ~20 realistic prompts (8-10 should-trigger, 8-10 should-not-trigger)
2. **Vary should-trigger queries** along multiple axes: phrasing, explicitness, detail, complexity
3. **Create strong should-not-trigger queries** — Use near-misses that share keywords but need something different
4. **Run each query multiple times** — Model behavior is nondeterministic; run 3 times
5. **Use train/validation splits** — ~60% train / ~40% validation

### The Optimization Loop

1. **Evaluate on both sets** — Train results guide changes; validation tells if changes generalize
2. **Identify failures in train set only** — Keep validation results hidden during iteration
3. **Revise strategically:**
    - Should-trigger failing → broaden scope or add context
    - Should-not-trigger false-triggering → add specificity about what the skill does *not* do
4. **Select best iteration by validation pass rate**
5. **Check the 1024-character limit**

---

## Creating Test Cases

Store in `evals/evals.json`:

```json
{
  "skill_name": "example-skill",
  "evals": [
    {
      "id": 1,
      "prompt": "User's task prompt",
      "expected_output": "Description of expected result",
      "files": [],
      "assertions": ["The output includes X"]
    }
  ]
}
```

### Test Case Guidelines

- **Realism**: Add file paths, personal context, specific details, casual language
- **Variety**: Mix formal/casual, terse/context-heavy, single-step/multi-step
- **Near-misses**: Include queries that share keywords but need something different

---

## Reference Files

- `references/best-practices.md` - Best practices for skill creators
- `references/evaluating-skills.md` - Evaluating skill output quality
- `references/schemas.md` - JSON structures for evals.json, grading.json

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "The skill works, I don't need evals." | Without evals, you can't prove the skill works or detect regressions. |
| "I'll write evals later." | Later never happens. Write evals alongside the skill. |
| "My description is fine." | Bad descriptions cause false triggers or missed activations. Test them. |

## Red Flags

- [ ] Creating a skill without `evals/evals.json`
- [ ] Description longer than 1024 characters
- [ ] No test cases that verify the skill triggers correctly
