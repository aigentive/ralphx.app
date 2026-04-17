# Codex Config Reference

Official docs: `https://developers.openai.com/codex/config-reference`

Snapshot notes:

- Codex config is documented as a large typed keyspace under `config.toml`.
- Official docs confirm `approval_policy` values include `untrusted`, `on-request`, `never`, and granular approval maps.
- Official docs confirm MCP server config under `mcp_servers.*`, including `enabled_tools`, `disabled_tools`, `startup_timeout_sec`, `tool_timeout_sec`, and `enabled`.
- Official docs confirm model defaults can be configured in `config.toml`.

RalphX notes:

- `model_reasoning_effort = "xhigh"` is vendor-supported in official Codex docs and should back the RalphX default for Codex ideation unless runtime capability checks say otherwise.
- MCP allowlisting is config-driven, not Claude plugin-driven.
- Approval/sandbox policy should be expressed through explicit Codex config rather than ad hoc CLI flags only.
