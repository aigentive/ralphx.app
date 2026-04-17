# Codex Non-Interactive Mode

Official docs: `https://developers.openai.com/codex/noninteractive`

Snapshot notes:

- `codex exec` is the documented non-interactive entry point for scripts and CI.
- Official docs explicitly document `codex exec resume`.
- `-c/--config key=value` is repeatable and is the main inline override mechanism.

RalphX notes:

- RalphX background process spawning will likely use the non-interactive `codex exec` path, not a human-facing TUI session.
- Resume semantics must be audited carefully because current RalphX recovery assumes Claude-style session ids and stale-session behavior.
