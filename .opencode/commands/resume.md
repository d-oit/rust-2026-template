# Command: resume

## Description
Resumes the agent session from the last recorded state to ensure continuity across interruptions.

## Execution Protocol
1. **Status Anchor**: Read `plans/_status.json` to identify the `active_plan` and any `handover_ref`.
2. **Handover Recovery**: If `handover_ref` points to a file, read it to restore the previous session's "thought process" and specific context.
3. **Plan Alignment**: Load the markdown file associated with the `active_plan` (typically in the `plans/` directory).
   - Identify the current `phase`.
   - Identify the next pending `todo` item.
4. **World State**: Read `plans/GOAP_STATE.md` to align with the executable truth of the repository.
5. **Validation**: Run `./scripts/code-quality.sh check` to ensure the workspace is in a healthy state for continuation.

## Goal
Restore the agent's mental model to the exact point of the last save or handoff.
