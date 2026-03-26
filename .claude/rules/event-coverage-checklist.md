> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Event Coverage Checklist

**Archetype addressed:** #5 (Incomplete event coverage — 5 same-day finalize_proposals fixes)

Every feature or fix that adds a new pipeline stage, MCP tool, or agent type MUST pass all checks below before the proposal is accepted.

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

**All 7 must be yes before merging.** Partial coverage = incomplete feature.

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

## How to Reference in Proposals

Every proposal that adds a pipeline stage, MCP tool, or agent type MUST include an acceptance criterion:

> **Event Coverage** — All 7 checks in `.claude/rules/event-coverage-checklist.md` pass. Happy path, error, timeout, cancel events all emit. Store key registered. Agent status cycles idle→generating→idle. Session switch preserves state.
