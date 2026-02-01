# Code Quality Standards

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## File Size Limits

| Area | Type | Max | Extract To |
|------|------|-----|------------|
| Backend | File | 500 (refactor@400) | — |
| | Helpers/Validation | 100/30 | `{mod}_helpers.rs`, `{mod}_validation.rs` |
| | >5 structs | — | `{mod}_types.rs` |
| | Service method | 50 | helper fn |
| Frontend | Component | 500 (refactor@400) | — |
| | Hook | 300 | — |
| | Presentational | 200 | pure display |
| Plugin | Component/Hook/Agent | 100 | — |
| | Store/Skill | 150 | — |

**Triggers:** >3 useState→hook | >4 props→composition | >3 branches→sub-components | handler>10 lines→hook

## Core Rules

| Rule | Details |
|------|---------|
| Atomic commits | New files + deletions in same commit |
| No .bak | Git is backup |
| Copy don't rewrite | Read original, copy signatures, verify types |
| Validate | `cargo check` / `npm run typecheck` before commit |
| Hook for logic | Complex state→hook, component only renders |
| Re-export on extract | `export { New as Old }` — don't break imports |
| Named constants | Magic numbers → `TIMEOUT_MS = 300` |
| DRY | 2+ times → helper |

## Tauri API Layer
See @.claude/rules/api-layer.md for complete API patterns.

## Database

**Migrations:** `src-tauri/src/infrastructure/sqlite/migrations/`

| Step | Action |
|------|--------|
| 1 | Create `vN_description.rs` with `IF NOT EXISTS` |
| 2 | Register in `MIGRATIONS` array |
| 3 | Bump `SCHEMA_VERSION` |
| 4 | Add tests to `vN_description_tests.rs` |

**Helpers:** `column_exists`, `table_exists`, `add_column_if_not_exists(conn, table, col, "TYPE DEFAULT x")`

**Datetime:** RFC3339 UTC only. Column=`TEXT`, use `strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')`, read via `parse_datetime` helper.
