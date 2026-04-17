# Codex MCP

Official docs: `https://developers.openai.com/codex/mcp`

Snapshot notes:

- Codex supports MCP via config-managed servers, including HTTP / streamable HTTP and stdio-style setups.
- Official docs confirm per-server `enabled_tools` and `disabled_tools`.
- MCP server config belongs under `mcp_servers.*` in Codex config, not in a Claude plugin dir.

RalphX notes:

- This is the main bridge for RalphX internal tools when running Codex.
- Reefagent injects Codex MCP by passing repeatable `-c mcp_servers.<name>.*` overrides and routing to an HTTP bridge; RalphX should evaluate the same pattern.
- Tool allowlists must be unified at the harness abstraction layer so Claude and Codex grant the same logical tool surface even if vendor wiring differs.
