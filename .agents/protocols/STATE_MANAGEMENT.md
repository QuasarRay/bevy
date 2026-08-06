# Runtime State Management

Mutable state lives under `.agents/runtime/tasks/<task-id>/`, which is ignored by Git.

Each task contains:

- `TASK.md`: resolved objective, constraints, gates, sequence, budget;
- `CURRENT.md`: current stage/status and exact next action;
- `DECISIONS.md`;
- `RISKS.md`;
- `EVIDENCE.md`;
- `HANDOFFS/`;
- `evidence/`.

`.agents/runtime/ACTIVE` atomically points to the active task.

Rules:

- Only the parent writes canonical state.
- Children return handoffs through chat or an explicitly assigned noncanonical artifact.
- Update state before and after every child.
- Remove stale hypotheses from `CURRENT.md`; preserve superseded decisions in `DECISIONS.md`.
- Reference long logs by path and hash rather than pasting them.
- Close tasks explicitly; never reuse a closed task as current state.
