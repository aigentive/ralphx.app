# Codex Config Basics

Official docs: `https://developers.openai.com/codex/config-basic`

Snapshot notes:

- Codex CLI and IDE extension share a `config.toml`.
- The docs position configuration as the main place for default model, reasoning, approvals, sandbox, MCP, and customization.
- Official docs pair config basics with advanced config, reference, and sample pages.

RalphX notes:

- RalphX should inject per-run Codex config with CLI overrides where possible instead of mutating a user-global `config.toml`.
- A managed temp config file or repeatable `-c key=value` overrides are a likely fit for provider-managed spawns.
- Repo-local `AGENTS.md` remains relevant because Codex can auto-include it unless disabled.
