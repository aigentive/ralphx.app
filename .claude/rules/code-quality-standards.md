---
paths:
  - "src/**/*.{ts,tsx,js,jsx}"
  - "src-tauri/src/**/*.rs"
  - "ralphx-plugin/**/*.{ts,js}"
---

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
| Validate | `cargo clippy --all-targets --all-features -- -D warnings` / `npm run typecheck` before commit |
| Hook for logic | Complex state→hook, component only renders |
| Re-export on extract | `export { New as Old }` — don't break imports |
| Extract = delete original | When moving functions to new modules, fully remove original code (not just copy) |
| Named constants | Magic numbers → `TIMEOUT_MS = 300` |
| DRY | 2+ times → helper |

## Tauri API Layer
See api-layer.md for complete API patterns.

## Database

**Migrations:** `src-tauri/src/infrastructure/sqlite/migrations/`

| Step | Action |
|------|--------|
| 1 | Run `python3 scripts/new_sqlite_migration.py <description>` to create `vYYYYMMDDHHMMSS_description.rs` + matching tests |
| 2 | Register in `MIGRATIONS` array |
| 3 | Bump `SCHEMA_VERSION` |
| 4 | Run `python3 scripts/validate_sqlite_migrations.py` before commit |

**Rule:** Legacy numeric versions stay as-is; any new migration after schema `81` must use a UTC timestamp version (`YYYYMMDDHHMMSS`) so parallel branches do not race on hand-picked integers.

**Helpers:** `column_exists`, `table_exists`, `add_column_if_not_exists(conn, table, col, "TYPE DEFAULT x")`

**Datetime:** RFC3339 UTC only. Column=`TEXT`, use `strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')`, read via `parse_datetime` helper.
