# GOAP World State

Template pointer file. Tracked in the repo so agent sessions resume from a
known anchor. Per-task detail lives in `plans/<nn>-<slug>.md` and is indexed
by `plans/_status.json:active_plan` — never pasted here.

- **Current goal:** none (idle template baseline; set on handover)
- **Active plan:** see `plans/_status.json` (`active_plan`, `phases`)
- **Last verified:** template baseline (update date + gate name on handover)
- **Next steps:** pick up `resume` order — status, handover, plan, world state

Rules: keep this file at most 20 lines; no commit hashes, no PR or issue
numbers, no CI log pastes, no host-local notes. Update on handover only.
