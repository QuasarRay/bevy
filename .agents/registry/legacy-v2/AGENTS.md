# Codex Repository Instructions

For difficult, high-value, or multi-step tasks, use the
`sequential-max-orchestration` skill in `.agents/skills/`.

## Multi-agent rules

- Use `gpt-5.6-sol` with `max` reasoning.
- At most one spawned child thread may be open.
- Never run subagents in parallel.
- A child must never spawn another child.
- The parent owns canonical state and final claims.
- Delegate only when an independent context materially improves correctness.
- Verify child evidence directly before accepting it.
- Prefer few narrow agents over many overlapping agents.
- Use one adversarial review and one final verifier for risky changes.
- Store long evidence in files and pass paths, not large prompt copies.
- Preserve user work and inspect repository state before mutation.

Read `.agents/ORCHESTRATOR.md` and the activated skill for full rules.
