# Codex Authentication

Official docs: `https://developers.openai.com/codex/auth`

Snapshot notes:

- Codex auth is documented separately from CLI usage.
- Authentication setup must be treated as a harness availability prerequisite.

RalphX notes:

- RalphX needs a Codex availability check analogous to current Claude CLI checks.
- Availability must distinguish: binary missing, auth missing, model unavailable, and MCP bridge unavailable.
