---
name: goap-agent
description: >
  Invoke for complex multi-step tasks requiring intelligent planning and
  multi-agent coordination. Use when tasks need decomposition, dependency
  mapping, parallel/sequential/swarm/iterative execution strategies, or
  coordination of multiple specialized agents with quality gates.
  Triggers: "complex task", "plan implementation", "multi-step", "coordinate agents".

license: MIT
metadata:
  category: workflow
  author: d-oit
  version: "0.2.10"
  adapted-from: d-o-hub/github-template-ai-agents
---

# GOAP Agent Skill: Goal-Oriented Action Planning

Enable intelligent planning and execution of complex multi-step tasks through systematic decomposition, dependency mapping, and coordinated multi-agent execution.

Always use the `plans/` folder for all files. Use `plans/GOAP_STATE.md` to track persistent state.

`GOAP_STATE.md` is a tracked pointer file (at most 20 lines: goal, active-plan
pointer, last-verified stamp, next steps) — see ADR 0005. Per-task detail lives
in `plans/<nn>-<slug>.md` and is indexed by `plans/_status.json:active_plan`.
Never paste commit hashes, PR/issue numbers, CI logs, or host-local notes into
the pointer file; update it on handover only.

## Core GOAP Methodology

### The GOAP Planning Cycle

```
1. ANALYZE → 2. DECOMPOSE → 3. STRATEGIZE → 4. COORDINATE → 5. EXECUTE → 6. SYNTHESIZE
```

## Phase 1: Task Analysis

1. **Analyze**: Use `triz-analysis` or `triz-solver` to evaluate the problem and resolve contradictions.
2. **Decide**: Formulate an **ADR** (Architecture Decision Record) in `plans/adr/` using the existing
   four-digit numbering convention (`NNNN-<slug>.md`, next free number) with a `## Status` section.
3. **Gate**: Wait until the ADR's status is `Accepted` before proceeding to decomposition.

```markdown
## Task Analysis

**Primary Goal**: [Clear statement of what success looks like]
**Constraints**: [Time, Resources]
**Complexity**: Simple/Medium/Complex
**ADR Link**: [Link to approved Architecture Decision Record]
```

## Phase 2: Task Decomposition

Use **task-decomposition** skill:

```markdown
### Sub-Goals
1. [Component 1] - Priority: P0, Deps: none
2. [Component 2] - Priority: P1, Deps: Component 1
```

Principles: Atomic, Testable, Independent, Assigned

## Phase 3: Strategy Selection

| Strategy | When | Speed |
|----------|------|-------|
| Parallel | Independent tasks | Nx |
| Sequential | Dependent tasks | 1x |
| Swarm | Many similar tasks | ~Nx |
| Hybrid | Mixed requirements | 2-4x |

## Phase 4: Agent Assignment

Match the task type to the spawned worker's capability:

| Capability selector | Best For |
|---------------------|----------|
| `scout` (read-only) | Research, code archaeology, map unfamiliar subsystems |
| `sonic` | Mechanical/bulk updates, data collection |
| `reviewer` / `security-reviewer` (read-only) | Quality and security review gates |
| general-purpose worker | Multi-step implementation (needs editing tools) |

Use a specialist only when one fits; otherwise use the general-purpose worker. Read-only
goes to read-only agents — they cannot edit, so never assign them an edit slice.

## Phase 5: Coordinated Execution

**Parallel**: single batch of task calls with one shared context block (per this
repo's harness: the `task` tool takes a batch of slices with a shared contract).
**Sequential**: Phases with quality gates between
**Monitor**: Track progress, validate results

## Phase 6: Result Synthesis

```markdown
## Summary

✓ Completed: [Tasks]
📦 Deliverables: [List]
✅ Quality: [Status]
```

## Common Patterns

- **Research → Implement → Validate**
- **Investigate → Diagnose → Fix → Verify**
- **Audit → Improve → Validate**

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "I can just execute all tasks in parallel — it's faster." | Parallel execution without dependency analysis causes race conditions and wasted tokens. |
| "Quality gates slow things down — skip them for speed." | Skipping quality gates compounds errors across agents, requiring far more rework. |

## Red Flags

- [ ] Executing tasks without mapping dependencies first
- [ ] Skipping ADR approval gate before decomposition
- [ ] Assigning agents without matching skills to task requirements
