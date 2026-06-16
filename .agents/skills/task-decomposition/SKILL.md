---
name: task-decomposition
description: >
  Break down complex tasks into atomic, actionable goals with clear dependencies
  and success criteria. Use when planning multi-step projects, coordinating agents,
  or decomposing complex requests.
  Triggers: "break down task", "decompose", "plan steps", "task breakdown".
category: agent
license: MIT
metadata:
  author: d-oit
  version: "0.2.10"
  adapted-from: d-o-hub/github-template-ai-agents
---

# Task Decomposition

Decompose high-level objectives into manageable, testable sub-tasks.

## When to Use

- Complex requests with multiple components
- Multi-phase projects requiring coordination
- Tasks benefiting from parallel execution

## Framework

### 1. Requirements Analysis

Extract: Primary objective, implicit requirements, constraints, success criteria.

### 2. Goal Hierarchy

```
Main Goal
├─ Sub-goal 1 → Tasks 1.1, 1.2
├─ Sub-goal 2 → Task 2.1
└─ Sub-goal 3
```

**Atomic Criteria**: Single action, defined inputs/outputs, one agent, testable.

### 3. Dependency Mapping

- **Sequential**: A → B → C
- **Parallel**: A, B, C (independent)
- **Converging**: A, B, C → D

### 4. Success Criteria

Define: Inputs, outputs, quality standards.

## Process

### Step 1: Understand Goal

```
Request: [Original]
Goal: [Main objective]
Type: [Implementation/Debug/Refactor]
Complexity: [Simple/Medium/Complex]
```

### Step 2: Identify Components (3-7)

### Step 3: Decompose Components

### Step 4: Map Dependencies

### Step 5: Assign Priorities

- P0 (Critical), P1 (Important), P2 (Nice-to-have)

### Step 6: Estimate Complexity

- Low (<30min), Medium (30min-2hr), High (>2hr)

## Patterns

### Layer-Based (Rust)

1. Data types / schemas
2. Business logic
3. API / CLI interface
4. Integration tests
5. Documentation

### Feature-Based

1. Core (MVP)
2. Error handling (`thiserror`/`anyhow`)
3. Performance (benches)
4. Integration
5. Testing
6. Docs

### Problem-Solution

1. Reproduce
2. Diagnose
3. Design
4. Fix
5. Verify
6. Prevent

## Quality Checklist

- Atomic and actionable tasks
- Dependencies identified
- Measurable success criteria
- Appropriate complexity estimates
- No task >4 hours
- Parallelization opportunities found

## Rationalizations

| Rationalization | Reality |
|-----------------|---------|
| "I already know what needs to be done, no need to decompose" | Skipping decomposition leads to scope creep, missed dependencies, and rework. |
| "The task is too simple to break down" | Even simple tasks benefit from explicit success criteria and dependency mapping. |

## Red Flags

- [ ] Defining tasks that are too vague to test or verify
- [ ] Skipping dependency mapping between decomposed tasks
- [ ] Creating tasks larger than 4 hours without further breakdown
