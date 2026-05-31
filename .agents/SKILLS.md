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
| `metrics-reporter` | [skills/metrics-reporter/SKILL.md](skills/metrics-reporter/SKILL.md) | Mandatory task completion reporting for DORA |
| `dora-report` | [skills/dora-report/SKILL.md](skills/dora-report/SKILL.md) | Generate automated DORA and agentic metrics reports |

## Skill Format

Each skill **must** follow this full structure. Sections marked with `*` are required for multi-agent compatibility.

```markdown
# Skill: <name>

---
version: <semver e.g. 1.0.0>             # * required
agents: [claude-code, gemini-cli, ...]   # * required — list of validated agents
---

## Purpose
## Trigger Conditions
## Prerequisites

## Input Schema                          # * required
<!-- Structured inputs this skill expects from the calling agent or workflow context. -->
<!-- Always include: workflow_id, task_id, parent_task_id. Add skill-specific fields below. -->

## Output Schema                         # * required
<!-- Structured result this skill emits for downstream agents. -->
<!-- Always include: success, status, artifacts[], next_skill (optional). -->

## Steps
## Success Criteria
## Common Issues

## Agent Compatibility                   # * required
<!-- | agent | min_version | notes | -->

## Related Skills
## References
```

### Common Envelope Fields

All skills share these standard fields in Input/Output schemas:

| Field | Type | Direction | Description |
|-------|------|-----------|-------------|
| `workflow_id` | `string\|null` | in + out | Groups related tasks across agents |
| `task_id` | `string` | in + out | Unique ID for this invocation |
| `parent_task_id` | `string\|null` | in | ID of the triggering task |
| `agent_id` | `string` | out | Agent that ran this skill |
| `skill` | `string` | out | Skill name (matches directory name) |
| `skill_version` | `string` | out | Semver version of SKILL.md used |
| `success` | `bool` | out | Overall outcome |
| `status` | `enum` | out | `success` \| `failure` \| `partial` |
| `artifacts` | `string[]` | out | File paths or URLs produced |
| `next_skill` | `string\|null` | out | Recommended next skill in chain |
| `notes` | `string` | out | Human-readable result summary |

## Usage by AI Agents

When an AI agent needs to perform a task:

1. Read [`.agents/ORCHESTRATION.md`](ORCHESTRATION.md) to understand current workflow phase and pending skills.
2. Check this index for the relevant skill.
3. Read [`.agents/context/workflow-state.json`](context/workflow-state.json) to get `workflow_id` and `task_id` context.
4. Follow the skill's **Input Schema → Steps → Output Schema** exactly.
5. Write a per-event file under `.agents/events/YYYY/MM/DD/` (see [ORCHESTRATION.md § Event File Format](ORCHESTRATION.md#event-file-format)).
6. Update `.agents/context/workflow-state.json` with result and `pending_skill`.
7. Report results against the Success Criteria.
8. Escalate to the user if Common Issues cannot be resolved.

> ⚠️ **Never append directly to `.agents/metrics.jsonl`** — that file is deprecated.
> Use per-event files under `.agents/events/` instead. See ORCHESTRATION.md.

## Adding New Skills

1. Create a new directory: `.agents/skills/<skill-name>/`
2. Add `SKILL.md` following the **full format above**, including `version`, `agents` frontmatter, `Input Schema`, `Output Schema`, and `Agent Compatibility`.
3. Update this index table.
4. Reference the skill in `AGENTS.md` and in [`ORCHESTRATION.md`](ORCHESTRATION.md) if it belongs in a pipeline.

## Related Files

- [AGENTS.md](../AGENTS.md) — General agent guidelines
- [ORCHESTRATION.md](ORCHESTRATION.md) — Multi-agent coordination, skill chains, handoff protocol
- [context/workflow-state.json](context/workflow-state.json) — Live workflow state (read before acting)
- [CLAUDE.md](../CLAUDE.md) — Claude-specific instructions
- [GEMINI.md](../GEMINI.md) — Gemini-specific instructions
- [agents-docs/workflow.md](../agents-docs/workflow.md) — Skill + CLI usage pattern
