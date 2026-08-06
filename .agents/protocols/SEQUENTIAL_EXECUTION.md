# Sequential Execution

Invariant: `open_spawned_child_threads <= 1`.

## Before spawn

- Previous child is closed.
- Effective runtime config was verified this session.
- Active task state is current.
- Role is necessary and within the child budget.
- Objective is singular and falsifiable.
- In/out-of-scope paths are named.
- Child is explicitly forbidden to delegate and to edit orchestration state.
- Required handoff and evidence are specified.

## After child

- Handoff received.
- No-delegation attestation present.
- Important evidence directly checked.
- Claims accepted, rejected, or marked unverified.
- Canonical state updated by parent.
- Child explicitly closed.
- One next action chosen.

Do not spawn if any item is unresolved.
