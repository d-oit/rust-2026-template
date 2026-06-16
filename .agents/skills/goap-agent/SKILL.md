---
name: goap-agent
description: >
  Invoke for complex multi-step tasks requiring intelligent planning and
  multi-agent coordination. Use when tasks need decomposition, dependency
  mapping, parallel/sequential/swarm/iterative execution strategies, or
  coordination of multiple specialized agents with quality gates.
  Triggers: "complex task", "plan implementation", "multi-step", "coordinate agents".
category: workflow
license: MIT
metadata:
  author: d-oit
  version: "0.2.10"
  adapted-from: d-o-hub/github-template-ai-agents
---

# GOAP Agent Skill: Goal-Oriented Action Planning

Enable intelligent planning and execution of complex multi-step tasks through systematic decomposition, dependency mapping, and coordinated multi-agent execution.

Always use the `plans/` folder for all files. Use `plans/GOAP_STATE.md` to track persistent state.

## When to Use This Skill

Use this skill when facing:
- **Complex Multi-Step Tasks**: Tasks requiring 5+ distinct steps
- **Cross-Domain Problems**: Issues spanning multiple areas
- **Optimization Opportunities**: Tasks benefiting from parallel execution
- **Quality-Critical Work**: Projects requiring validation checkpoints

## Core GOAP Methodology

### The GOAP Planning Cycle

```
1. ANALYZE → 2. DECOMPOSE → 3. STRATEGIZE → 4. COORDINATE → 5. EXECUTE → 6. SYNTHESIZE
```

## Phase 1: Task Analysis

1. **Analyze**: Use `triz-analysis` or `triz-solver` to evaluate the problem and resolve contradictions.
2. **Decide**: Formulate an **ADR** (Architecture Decision Record) in `plans/adr/`.
3. **Gate**: Wait for human approval of the ADR before proceeding to decomposition.

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

| Agent | Best For |
|-------|----------|
| feature-implementer | New functionality |
| debugger | Bug fixes |
| test-runner | Test validation |
| refactorer | Code improvements |
| code-reviewer | Quality assurance |

## Phase 5: Coordinated Execution

**Parallel**: Single message, multiple Task tool calls
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
