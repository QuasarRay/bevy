<!-- BEGIN SEQMAX MANAGED BLOCK v3 -->
## Sequential Max orchestration

For difficult, high-value, multi-step work, use `$sequential-max-orchestration` from `.agents/skills/`.

- Use managed `seqmax_*` children only when isolation materially improves correctness.
- Parent and managed children target `gpt-5.6-sol` with `max` reasoning.
- Never have more than one spawned child thread open.
- Wait for, validate, and explicitly close a child before spawning another.
- A child must never delegate.
- The parent owns canonical state and final claims.
- Children must not edit `.agents/runtime/`, `.agents/state/`, `.codex/`, or `AGENTS.md` unless this package itself is the assigned target.
- Verify effective settings with `/status` and `/debug-config`; static files alone are not proof.
- Prefer direct deterministic tools over an unnecessary child.
- Preserve user work and run repository preflight before mutation.

Read `.agents/ORCHESTRATOR.md` for the full contract.
<!-- END SEQMAX MANAGED BLOCK v3 -->
