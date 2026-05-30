# Agent Skills Index

Reusable skill runbooks for working with this Rust template repository.
Skills are self-contained and can be followed by Claude Code, Gemini CLI, OpenCode, Qwen Code, and similar agents.

## Available Skills

| Skill | Path | Description |
|-------|------|-------------|
| `build-rust` | [skills/build-rust/SKILL.md](skills/build-rust/SKILL.md) | Compile, build, and verify Rust code |
| `lint-rust` | [skills/lint-rust/SKILL.md](skills/lint-rust/SKILL.md) | Run Clippy, format checks, cargo-audit, cargo-deny |
| `test-rust` | [skills/test-rust/SKILL.md](skills/test-rust/SKILL.md) | Run test suite with cargo-nextest |
| `release-rust` | [skills/release-rust/SKILL.md](skills/release-rust/SKILL.md) | Safe release workflow for crates.io |
| `crates-io-name-check` | [skills/crates-io-name-check/SKILL.md](skills/crates-io-name-check/SKILL.md) | Verify crate name availability before publishing |
| `anti-ai-slop` | [skills/anti-ai-slop/SKILL.md](skills/anti-ai-slop/SKILL.md) | Audit and fix generic AI-generated Rust code patterns |
| `privacy-first` | [skills/privacy-first/SKILL.md](skills/privacy-first/SKILL.md) | Prevent email/personal data from entering the codebase |
| `skill-creator` | [skills/skill-creator/SKILL.md](skills/skill-creator/SKILL.md) | Create and optimize new agent skills |
| `skill-evaluator` | [skills/skill-evaluator/SKILL.md](skills/skill-evaluator/SKILL.md) | Evaluate skill quality with structure checks |
| `dora-report` | [skills/dora-report/SKILL.md](skills/dora-report/SKILL.md) | Generate automated DORA and agentic metrics reports |

## Skill Format

Each skill follows this structure:

```markdown
# Skill: <name>
## Purpose
## Trigger Conditions
## Prerequisites
## Steps
## Success Criteria
## Common Issues
## Related Skills
## References
```

## Usage by AI Agents

When an AI agent needs to perform a task:

1. Check this index for the relevant skill
2. Follow the skill's steps exactly
3. Report results against the success criteria
4. Escalate to the user if common issues can't be resolved

## Adding New Skills

1. Create a new directory: `.agents/skills/<skill-name>/`
2. Add `SKILL.md` following the format above
3. Update this index
4. Reference the skill in `AGENTS.md` if applicable

## Related Files

- [AGENTS.md](../AGENTS.md) — General agent guidelines
- [CLAUDE.md](../CLAUDE.md) — Claude-specific instructions
- [GEMINI.md](../GEMINI.md) — Gemini-specific instructions
- [agents-docs/workflow.md](../agents-docs/workflow.md) — Skill + CLI usage pattern
