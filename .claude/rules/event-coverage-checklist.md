---
paths:
  - "src/components/**"
  - "src/hooks/**"
  - "src/stores/**"
  - "src/lib/chat-context-registry.ts"
  - "src/types/chat-conversation.ts"
  - "src-tauri/src/commands/**"
  - "src-tauri/src/application/chat_service/**"
  - "src-tauri/src/application/task_transition_service.rs"
  - "src-tauri/src/http_server/**"
  - "plugins/app/agents/**"
  - "plugins/app/ralphx-mcp-server/src/**"
---

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Event Coverage Checklist

**Archetype addressed:** #5 (Incomplete event coverage)

Every feature or fix that adds a new pipeline stage, MCP tool, agent type, or event-bearing context MUST review the checks below before the proposal is accepted.

## NON-NEGOTIABLE Checklist

| # | Check | Question | Pass? |
|---|-------|----------|-------|
| 1 | Happy path event | Does the success path emit a UI event? | yes / no |
| 2 | Error path event | Does every error exit emit a UI event? | yes / no |
| 3 | Timeout event | Does the timeout path emit a UI event? | yes / no |
| 4 | Cancel event | Does user cancellation emit a UI event? | yes / no |
| 5 | Store key | Is the UI store key registered for this context type? | yes / no |
| 6 | Agent status cycle | Does the agent status cycle through idle→generating→idle? | yes / no |
| 7 | Session switch | Does switching away and back preserve correct state? | yes / no |

Core rule: all checks relevant to the affected context must pass before merge. Do not force irrelevant UI/session checks onto pure backend or non-switchable contexts.

## When This Applies

| Trigger | Apply checklist? |
|---------|-----------------|
| New pipeline stage added | yes |
| New MCP tool added | yes |
| New agent type added | yes |
| New context type (ChatContextType) | yes |
| Modifying existing event handlers | yes |
| Pure Rust backend (no UI events) | skip |
| Documentation-only changes | skip |

## How To Apply

| Check | Apply when |
|-------|------------|
| Happy path event / Error path event / Timeout event / Cancel event | The feature has those exit paths |
| Store key / Agent status cycle | A UI-visible agent or context state is introduced or changed |
| Session switch | The UI can navigate away and return to the affected stateful view |

## How to Reference in Proposals

Every proposal that adds a pipeline stage, MCP tool, or agent type MUST include an acceptance criterion that names the relevant checks:

> **Event Coverage** — Relevant checks in `.claude/rules/event-coverage-checklist.md` pass for this context. Success and failure exits emit required events, and any UI-visible state wiring stays consistent.
