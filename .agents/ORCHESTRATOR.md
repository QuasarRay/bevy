# Sequential Max Parent Orchestrator Contract

The parent owns the user objective, sequencing, canonical state, repository safety, and final claims.

## Hard invariants

1. At most one spawned child thread may be open.
2. Before another child is spawned, the previous child must finish, return a handoff, have load-bearing evidence checked, and be explicitly closed.
3. Every child receives one objective and an explicit instruction not to delegate.
4. Managed children use `gpt-5.6-sol` with `max` reasoning.
5. Do not silently downgrade model or reasoning effort. If Max is unavailable, report the limitation and continue directly only when the user accepts the loss of the requested guarantee.
6. Do not use parallel subagents. Do not coordinate simultaneous write-capable workers.
7. Child reports are claims, not proof. Verify important claims directly.
8. Never let a child edit `.agents/runtime/`, `.agents/state/`, `.codex/`, or `AGENTS.md` unless the user's task is explicitly to modify this orchestration package.
9. Never claim that static config files prove the effective Codex session configuration.
10. Stop once the acceptance gates are proven; do not spend budget on speculative improvements.

## Effective-runtime preflight

Before the first delegation in a session:

1. Confirm the repository is trusted so project `.codex/` layers load.
2. Run `/status` and verify the active model and reasoning effort.
3. Run `/debug-config` and verify the project config layer is active and not superseded by CLI, profile, user, system, or managed policy.
4. Confirm `agents.max_concurrent_threads_per_session = 1` in the effective layer diagnostics.
5. If any point cannot be verified, do not claim sequential-Max enforcement. Work directly or ask the user to correct the environment.

## Parent responsibilities

- Resolve the request without broadening it.
- Perform repository/environment preflight when mutation is possible.
- Create a task ledger and falsifiable acceptance gates.
- Classify the work as S, M, L, or XL.
- Choose the smallest useful child sequence.
- Send a bounded spawn packet.
- Validate the handoff.
- Update only the active task's canonical state.
- Resolve contradictions before they propagate.
- Run or witness final verification.
- Report what was observed, inferred, not tested, or blocked.

## Complexity classes and default child caps

| Class | Typical shape | Default child cap |
|---|---|---:|
| S | Direct parent execution | 0 |
| M | One focused worker plus optional verifier | 1–2 |
| L | Explorer/architect, implementer, reviewer, verifier | 3–5 |
| XL | Multiple separately completed milestones | 5 per milestone |

Exceeding the cap requires new evidence of a distinct high-risk subsystem and must be recorded in the task ledger.

## Default state machine

```text
PRECHECK
  -> TRIAGE
  -> optional EXPLORE or RESEARCH
  -> optional ARCHITECT
  -> IMPLEMENT
  -> conditional DIAGNOSE
  -> conditional REVIEW
  -> conditional REMEDIATE
  -> VERIFY
  -> FINALIZE
```

Skip stages that do not change a decision or improve evidence.

## Delegation gate

Delegate only when an isolated context materially improves reliability, such as:

- unfamiliar repository mapping;
- consequential architecture selection;
- a large bounded implementation;
- failure after one direct bounded diagnosis attempt;
- independent adversarial review;
- justified security, performance, test, or migration expertise;
- clean-context final verification.

Do not delegate trivial edits, one obvious compiler error, formatting, a known command, reassurance, or repeated consensus.

## Contradiction handling

1. Freeze the disputed fact.
2. Record it as a risk.
3. Prefer runtime/test evidence over summaries.
4. Prefer direct source and manifests over memory.
5. Prefer versioned primary documentation over secondary sources.
6. Use one diagnostician only when deterministic parent inspection is insufficient.
7. Record the resolution and what would falsify it.

## Final claim standard

Every important claim maps to at least one of:

- file path and relevant lines/symbols;
- exact command and exit code;
- named test and result;
- observed runtime behavior;
- versioned authoritative source;
- clearly labeled inference with confidence and falsifier.
