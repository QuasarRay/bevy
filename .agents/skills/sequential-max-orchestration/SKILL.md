---
name: sequential-max-orchestration
description: Orchestrate difficult high-value work with GPT-5.6 Sol at Max reasoning, strictly one sequential child at a time, evidence-gated handoffs, independent review, and budget-aware context isolation. Use for complex multi-step implementation, migration, debugging, or review; do not use for trivial edits.
---

# Sequential Max Orchestration

1. Read `.agents/ORCHESTRATOR.md`.
2. Read `references/effective-config.md` and verify the effective session before delegation.
3. Create or locate the active task under `.agents/runtime/`.
4. Read only the references needed for the current stage.
5. Classify the task and define acceptance gates.
6. Select the smallest useful sequence of `seqmax_*` roles.
7. Spawn exactly one child, wait, validate, update state, close it, then decide whether another is necessary.
8. Use Max reasoning to improve depth, not to justify more agents.
9. Finalize only after gate-by-gate verification.

Relevant references:

- triage: `references/task-triage.md`
- delegation: `references/delegation-policy.md`
- context: `references/context-budgeting.md`
- handoff: `references/handoff-protocol.md`
- evidence: `references/evidence-standards.md`
- failures: `references/failure-recovery.md`
- budget: `references/budget-policy.md`
- stopping: `references/stopping-rules.md`
- repository safety: `references/repository-safety.md`
- external facts: `references/source-verification.md`
- redaction: `references/redaction.md`
- processes: `references/process-lifecycle.md`
