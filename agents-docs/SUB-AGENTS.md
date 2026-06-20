# Sub-Agent Patterns

## Overview

Sub-agents are specialized workers spawned by a parent agent to handle specific tasks. They operate with isolated context and return structured results.

## Architecture

```
Parent Agent (orchestrator)
    |
    ├── Explore Sub-Agent (read-only code discovery)
    ├── General Sub-Agent (multi-step implementation)
    └── General Sub-Agent (parallel work)
```

## Sub-Agent Types

### Explore Agent

**Purpose**: Fast, read-only codebase exploration
**Context**: Inherits parent's working directory
**Use cases**:
- Finding file patterns (`**/*.rs`)
- Searching for code patterns (`grep` across codebase)
- Answering questions about codebase structure

### General Agent

**Purpose**: Multi-step task execution with tool access
**Context**: Inherits parent's working directory
**Use cases**:
- Implementing features across multiple files
- Running tests and fixing failures
- Complex refactoring operations

## Spawning Sub-Agents

### Inline (Blocking)

```python
# Spawn and wait for result
actor({
    "operation": {
        "action": "run",
        "description": "Find error handling patterns",
        "prompt": "Search src/ for error handling patterns...",
        "subagent_type": "explore"
    }
})
```

### Background (Non-Blocking)

```python
# Spawn and continue working
actor({
    "operation": {
        "action": "spawn",
        "description": "Run tests in background",
        "prompt": "Run cargo test --workspace and report results...",
        "subagent_type": "general"
    }
})
```

## Context Isolation

Each sub-agent operates with isolated context:

| Context Level | What Child Sees |
|---------------|-----------------|
| `none` (default) | Only the prompt |
| `state` | Checkpoint summaries |
| `full` | Full parent conversation |

### When to Use Each Level

- **`none`**: Independent tasks (explore, search, verify)
- **`state`**: Tasks needing project background (implement, fix)
- **`full`**: Tasks requiring full conversation history (review, evaluate)

## Task Binding

Sub-agents can be bound to tracked tasks:

```python
actor({
    "operation": {
        "action": "run",
        "task_id": "T4",  # Bind to task T4
        "prompt": "Implement feature X...",
        "subagent_type": "general"
    }
})
```

Benefits:
- Progress captured to `tasks/<id>/progress.md`
- Findings integrated into next checkpoint
- Task status updated automatically

## Communication

### Parent to Child

Use `send` to deliver messages to running sub-agents:

```python
actor({
    "operation": {
        "action": "send",
        "to_actor_id": "explore-1",
        "content": "Focus on error handling in src/parser.rs"
    }
})
```

### Child to Parent

Sub-agents return results via their final message. Format:

```
**Status**: success | partial | failed | blocked
**Summary**: <one-line description>

<deliverable body>

**Files touched**: <comma-separated paths>
**Findings worth promoting**: <bullet list>
```

## Anti-Patterns

- **Don't** spawn sub-agents for trivial single-file lookups
- **Don't** spawn without clear success criteria in the prompt
- **Don't** create deep nesting (parent -> child -> grandchild)
- **Don't** rely on sub-agents for real-time communication (use `send`)

## Performance

- Sub-agents share the process-wide concurrency ceiling
- Default max concurrent: `min(16, 2x cores)`
- Use `spawn` for long-running tasks to avoid blocking
- Use `run` for short tasks that need immediate results
