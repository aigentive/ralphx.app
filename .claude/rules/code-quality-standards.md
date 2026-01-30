# Code Quality Standards

## File Size Limits

| Area | Type | Max | Refactor At |
|------|------|-----|-------------|
| **Backend** | Any file | 500 | 400 |
| | Helper functions | 100 | Extract to `{module}_helpers.rs` |
| | >5 structs/enums | — | Extract to `{module}_types.rs` |
| | Service method | 50 | Extract helper |
| | Validation | 30 | Extract to `{module}_validation.rs` |
| **Frontend** | Component | 500 | 400 |
| | Custom Hook | 300 | — |
| | Presentational | 200 | Pure display only |
| **Plugin** | Component/Hook/Agent | 100 | — |
| | Store/Skill | 150 | — |

**Frontend extraction triggers:** >3 useState → hook | >4 props → composition | >3 branches → sub-components | handler >10 lines → hook

## Extraction Rules

1. **Atomic commits** — new files + deletion in same commit
2. **No .bak files** — git is backup
3. **Copy, don't rewrite** — read original first, copy exact signatures, verify types exist
4. **Validate before commit** — `cargo check` or `npm run typecheck`

**Exceeds limit = must extract. "Well-organized" is not an excuse.**

## Separation of Concerns

| Pattern | Rule |
|---------|------|
| **Hook for logic, component for display** | Complex state logic → custom hook. Component only renders. |
| **Behavior with container** | Scroll logic in scroll component, not parent. Animation in animated component. |
| **Re-export on extract** | `export { NewName as OldName } from "./NewFile"` — don't break imports |

## Code Clarity

| Pattern | Rule |
|---------|------|
| **Named constants** | Magic numbers → `TIMEOUT_MS = 300` with JSDoc |
| **Extract repeated logic** | Same code 2+ times → helper function |
| **Shared style constants** | Repeated inline styles → `const style: React.CSSProperties = {...}` |
| **Co-locate dependencies** | Component owns its CSS/animations via `<style>` tag, not parent injection |
| **Complete loading states** | Cover entire async window: fetch + auto-select + settling |

## API Serialization Convention

### The snake_case Boundary Pattern

| Layer | Convention | Example |
|-------|-----------|---------|
| Rust backend | snake_case | `session_id`, `created_at` |
| Frontend Zod schema | snake_case | `z.object({ session_id: z.string() })` |
| Transform function | converts | `sessionId: raw.session_id` |
| Frontend types | camelCase | `interface { sessionId: string }` |

### Backend Rules
- **NEVER** use `#[serde(rename_all = "camelCase")]` on response structs
- Rust structs serialize to snake_case by default (correct)
- Input structs may use `#[serde(rename_all = "camelCase")]` for Tauri param convenience

### Frontend Rules
- API schemas in `src/api/*.schemas.ts` expect **snake_case**
- Display types in `src/types/*.ts` use **camelCase**
- Transform functions in `src/api/*.transforms.ts` bridge the gap
- Every API wrapper must apply transforms before returning
