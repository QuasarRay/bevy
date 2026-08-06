# Official Codex sources used for v3.1

- Subagents and custom-agent schema: https://developers.openai.com/codex/subagents
- Config reference: https://developers.openai.com/codex/config-reference
- Config precedence and project trust: https://developers.openai.com/codex/config-basic
- Strict configuration and inspecting effective settings: https://developers.openai.com/codex/developer-settings
- AGENTS.md discovery: https://developers.openai.com/codex/agent-configuration/agents-md
- Skills structure and invocation: https://developers.openai.com/codex/build-skills
- Codex models and Max reasoning: https://developers.openai.com/codex/models

The package intentionally distinguishes static file validation from effective session configuration because CLI overrides have higher precedence and project config is ignored for untrusted projects.
