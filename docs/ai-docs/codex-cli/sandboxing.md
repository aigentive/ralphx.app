# Codex Sandboxing

Official docs: `https://developers.openai.com/codex/concepts/sandboxing`

Snapshot notes:

- Codex has a native sandbox model and does not rely on Claude plugin-dir semantics.
- Official docs distinguish `read-only`, `workspace-write`, and `danger-full-access` style modes.
- Sandbox policy is intertwined with approvals, writable roots, and network access.

RalphX notes:

- Codex sandbox semantics are the biggest vendor-level difference from Claude CLI for backend spawning.
- The RalphX Codex harness must make sandbox mode explicit in spawn metadata and raw logs.
- Reefagent chose HTTP MCP bridging plus `danger-full-access` in order to preserve localhost MCP access; RalphX must validate whether it needs the same tradeoff.
