# Effective Codex Configuration

Static TOML validation is necessary but insufficient.

Before delegation in a new session, start with strict configuration checking where supported so unknown keys fail rather than being ignored:

1. Trust the project explicitly if appropriate.
2. Run `/status` for model, reasoning, permissions, and workspace.
3. Run `/debug-config` for layer precedence.
4. Confirm no CLI `--model`, `--config`, profile, user, system, or managed-policy layer changes the intended settings.
5. Confirm the project agent role declarations loaded.
6. Confirm one-thread concurrency.

If the UI/client cannot expose these facts, state that the guarantee is unverified and do not claim enforcement.
