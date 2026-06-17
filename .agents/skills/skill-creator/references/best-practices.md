# Best Practices for Skill Creators

## Skill Design

### Keep It Focused
- One skill = one capability. Don't bundle unrelated workflows.
- If a skill needs >250 lines in SKILL.md, move details to `references/`.

### Write for the Agent, Not the Human
- Agents read instructions literally. Be explicit about when to trigger and when not to.
- Use imperative phrasing: "Run `cargo clippy`" not "You should run clippy".

### Progressive Disclosure
- SKILL.md: core workflow and trigger conditions (required reading)
- `references/`: supplementary docs (read on demand)
- `scripts/`: executable helpers (run when needed)

## Directory Structure

```
skill-name/
├── SKILL.md          # Required: metadata + instructions
├── scripts/          # Optional: executable code
├── references/       # Optional: documentation
├── assets/           # Optional: templates, resources
└── evals/            # Optional: test cases
```

## Frontmatter

### Required Fields
- `name`: Lowercase, hyphens only, max 64 chars
- `description`: Max 1024 chars. Describe what AND when to trigger.

### Optional Fields
- `license`: License name or reference
- `compatibility`: Environment requirements (max 500 chars)
- `metadata`: Arbitrary key-value pairs
- `allowed-tools`: Space-delimited pre-approved tools

## Description Writing

1. **Use imperative phrasing** — "Use this skill when..." not "This skill does..."
2. **Focus on user intent** — What is the user trying to achieve?
3. **Be pushy** — Explicitly list where the skill applies
4. **Stay concise** — A few sentences, max 1024 characters

## Eval Design

- Write 3+ eval cases per skill
- Include both should-trigger and should-not-trigger scenarios
- Use realistic prompts (file paths, casual language, specific details)
- Make assertions concrete and checkable
- Run evals 3 times (nondeterministic models)

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Vague description | Add specific trigger contexts |
| No evals | Write evals alongside the skill |
| Over-engineered | Start simple, iterate based on usage |
| Deeply nested references | Keep docs flat and scannable |
| Missing edge cases | Add evals for failure modes |
