# Codex Hooks

Official docs: `https://developers.openai.com/codex/hooks`

Snapshot notes:

- Codex has a native hooks system separate from MCP and separate from AGENTS/rules.
- Hook configuration is relevant for pre/post tool behavior, auditability, and policy enforcement.

RalphX notes:

- RalphX should not depend on hooks for core harness parity initially.
- Hooks may become useful later for provider-specific tracing, approval instrumentation, or policy guardrails.
