# Code Quality Standards

> **Maintainer note:** Keep this file concise. Tables > prose. Examples only when critical. Goal: low cognitive load, high signal density.

## File Size Limits

| Area | Type | Max | Refactor At | Extract To |
|------|------|-----|-------------|------------|
| Backend | Any file | 500 | 400 | — |
| | Helpers | 100 | — | `{module}_helpers.rs` |
| | >5 structs/enums | — | — | `{module}_types.rs` |
| | Service method | 50 | — | helper fn |
| | Validation | 30 | — | `{module}_validation.rs` |
| Frontend | Component | 500 | 400 | — |
| | Hook | 300 | — | — |
| | Presentational | 200 | — | pure display only |
| Plugin | Component/Hook/Agent | 100 | — | — |
| | Store/Skill | 150 | — | — |

**Frontend triggers:** >3 useState → hook | >4 props → composition | >3 branches → sub-components | handler >10 lines → hook

**Rule:** Exceeds limit = must extract. "Well-organized" is not an excuse.

## Extraction Rules

1. **Atomic commits** — new files + deletion in same commit
2. **No .bak files** — git is backup
3. **Copy, don't rewrite** — read original, copy exact signatures, verify types
4. **Validate before commit** — `cargo check` / `npm run typecheck`

## Separation of Concerns

| Pattern | Rule |
|---------|------|
| Hook for logic | Complex state → hook. Component only renders. |
| Behavior with container | Scroll in scroll component, animation in animated component. |
| Re-export on extract | `export { New as Old } from "./New"` — don't break imports |

## Code Clarity

| Pattern | Rule |
|---------|------|
| Named constants | Magic numbers → `TIMEOUT_MS = 300` |
| DRY | 2+ times → helper function |
| Style constants | Repeated inline → `const style: CSSProperties` |
| Co-locate deps | Component owns CSS/animations via `<style>` |
| Loading states | Cover entire async: fetch + auto-select + settling |

## API Serialization

| Layer | Convention | Example |
|-------|-----------|---------|
| Rust backend | snake_case | `session_id` |
| Zod schema | snake_case | `z.object({ session_id })` |
| Transform | converts | `sessionId: raw.session_id` |
| TS types | camelCase | `sessionId: string` |

**Backend:** NEVER `#[serde(rename_all = "camelCase")]` on responses. Input structs may use it.
**Frontend:** Schemas expect snake_case → transforms → camelCase types.

## Database Migrations

**Location:** `src-tauri/src/infrastructure/sqlite/migrations/`

```
migrations/
├── mod.rs           # Runner, MIGRATIONS array, SCHEMA_VERSION
├── helpers.rs       # column_exists, table_exists, add_column_if_not_exists
├── v1_*.rs          # Initial schema (from production dump)
├── v2_*.rs          # Future migrations
└── tests.rs         # Migration tests
```

### Adding Migration

1. Create `vN_description.rs`
2. Implement: `pub fn migrate(conn: &Connection) -> AppResult<()>` with `IF NOT EXISTS`
3. Register in `MIGRATIONS` array in mod.rs
4. Bump `SCHEMA_VERSION`
5. Add tests

### Guidelines

| Rule | Notes |
|------|-------|
| Idempotent | `IF NOT EXISTS` for tables/indexes |
| Columns | `helpers::add_column_if_not_exists()` |
| Atomic | One change per migration |
| Immutable | Never modify existing migrations |

### Helpers

```rust
helpers::column_exists(conn, "table", "col") -> bool
helpers::table_exists(conn, "table") -> bool
helpers::add_column_if_not_exists(conn, "table", "col", "TEXT DEFAULT ''")
```

### Test Pattern

```rust
#[test]
fn test_creates_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    assert!(conn.execute("INSERT INTO table ...", []).is_ok());
}
```

### Checklist

- [ ] New file `vN_description.rs`
- [ ] `IF NOT EXISTS` / helpers for idempotency
- [ ] Registered in `MIGRATIONS`
- [ ] `SCHEMA_VERSION` bumped
- [ ] Tests added

## Datetime Format (SQLite)

**Standard:** RFC3339 with UTC timezone (`2026-01-31T10:30:45+00:00`)

| Wrong | Correct |
|-------|---------|
| `DATETIME DEFAULT CURRENT_TIMESTAMP` | `TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))` |
| `datetime('now')` | `strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')` |

**Rules:**
- Column type: `TEXT` (not `DATETIME`)
- Always include timezone: `+00:00`
- Use `parse_datetime` helper for reading (handles legacy formats gracefully)
