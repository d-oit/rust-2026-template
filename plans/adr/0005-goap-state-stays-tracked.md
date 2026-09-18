# 0005: GOAP Live State Stays Tracked

## Status

Accepted

## Context

Commit `6bfbc78` deleted `plans/GOAP_STATE.md` and `plans/_status.json` on the
grounds that agent planning state must not ship to template adopters. That
broke the `resume` order (`.opencode/commands/resume.md`), which reads
`_status.json` for `active_plan`/`handover_ref` and `GOAP_STATE.md` for world
state, and contradicted `agents-docs/structure.md` and the `goap-agent` skill,
which both document `plans/GOAP_STATE.md` as shipped structure.

At the same time, the previously tracked `GOAP_STATE.md` had grown into a
44-line per-cycle journal (PR numbers, commit hashes, CI log pastes,
host-local "behind origin" notes). Shipping a journal entry as the template
baseline teaches adopters to commit scratch and guarantees merge conflicts
across parallel agent branches.

## Decision

Keep `plans/GOAP_STATE.md` and `plans/_status.json` tracked in the repo. The
world-state file is a short pointer (at most 20 lines), not a journal:
current goal, active-plan pointer, last-verified stamp, next steps. Per-task
detail lives in `plans/<nn>-<slug>.md` and is indexed by `_status.json`.
No commit hashes, PR/issue numbers, CI log pastes, or host-local notes.

Enforcement: `schema/goap-status.schema.json` (closed schema) plus a
`validate-workflows.sh` section that fails on oversize state, hash-like
tokens, or schema-invalid status. Fixtures under `tests/fixtures/goap/`
pin the red behavior.

## Consequences

- `resume` works on a fresh clone; the skill contract matches the tree.
- Adopters inherit a stable anchor instead of someone else's cycle log.
- Tracked mutable state still conflicts under parallel agents; mitigated by
  the size cap and handover-only writes, not eliminated.
- Supersedes the `6bfbc78` deletion for these two paths only; generated
  telemetry (`.agents/ci/*`, `benchmarks/events/*`) stays generated.

## Alternatives Considered

| Option | Reason Rejected |
|--------|-----------------|
| Gitignore live state | Breaks `resume` and contradicts shipped docs/skill contract |
| Keep journal-style state | Ships stale context; conflict magnet; fails template bar |
| Reuse ADR number 0004 | Breaks monotonic ADR numbering; 0004 was an unmerged draft |
