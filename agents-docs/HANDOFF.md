# Agent Handoff Patterns

## When to Hand Off

Hand off work when:
- The task exceeds your context window or expertise
- A specialized agent can handle the task more efficiently
- The task requires access to tools or permissions you don't have
- You've hit a blocker that another agent can resolve

## Handoff Protocol

### 1. Write Handoff State
Before handing off, write the current state to the event file:

```json
{
  "event_type": "handoff",
  "task_id": "T123",
  "agent_id": "code-agent",
  "target_agent": "quality-agent",
  "context": {
    "progress": "Implemented feature X, tests passing",
    "remaining": "Need security review and performance benchmarks",
    " blockers": []
  }
}
```

### 2. Update Workflow State
Update `.agents/context/workflow-state.json` to reflect the handoff:

```json
{
  "current_task": "T123",
  "assigned_to": "quality-agent",
  "handoff_from": "code-agent",
  "handoff_reason": "Security review required"
}
```

### 3. Notify Target Agent
Use the `send` operation to notify the target agent with a brief summary.

## Agent Roles

| Agent | Responsibilities |
|-------|-----------------|
| code-agent | Implementation, refactoring, bug fixes |
| quality-agent | Linting, testing, security review |
| release-agent | Version management, changelog, publishing |
| meta-agent | Skill management, documentation, coordination |

## Context Preservation

When handing off:
- Include relevant file paths and line numbers
- Reference any ADRs or design decisions made
- Note any test failures or warnings encountered
- Specify the exact next action expected

## Anti-Patterns

- **Don't** hand off without writing state first
- **Don't** hand off tasks you could complete with available tools
- **Don't** create circular handoffs (A -> B -> A)
- **Don't** hand off without clear success criteria
