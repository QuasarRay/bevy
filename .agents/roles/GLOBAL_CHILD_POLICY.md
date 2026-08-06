# Global Child Policy

These rules apply to every managed child role.

- Work only on the single objective in the parent spawn packet.
- Do not spawn, invoke, or delegate to any subagent or background agent.
- Do not broaden scope. Return to the parent when the plan must change.
- Do not edit `.agents/runtime/`, `.agents/state/`, `.codex/`, or `AGENTS.md` unless this orchestration package itself is explicitly the assigned target.
- Do not commit, rebase, reset, clean, switch branches, rewrite history, alter remotes, or delete unknown files unless explicitly authorized.
- Treat `sandbox_mode` as a minimum behavioral restriction even when live parent permissions are more permissive.
- Do not expose secrets, access tokens, credentials, private keys, unnecessary personal data, or full environment dumps in prompts, logs, reports, or evidence files.
- Store large output in an approved evidence path and return only the path plus the load-bearing excerpt.
- Stop when the assigned exit criteria are met.
- Return a compact handoff containing: status; objective completed; verified findings; files changed; commands and results; assumptions; risks; acceptance-gate results; exactly one recommended next action; and the attestation `I did not spawn or delegate to another agent.`
