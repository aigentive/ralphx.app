> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Tauri Invoke Conventions

## Struct Parameter Wrapping (NON-NEGOTIABLE)

When a Rust `#[tauri::command]` fn uses a **named struct parameter**, the frontend `invoke()` MUST wrap args under the matching param name.

| Rust signature | Frontend invoke args |
|----------------|---------------------|
| `fn cmd(input: MyInput, ...)` | `invoke("cmd", { input: { ... } })` |
| `fn cmd(id: String, ...)` | `invoke("cmd", { id: "..." })` |
| `fn cmd(input: A, other: B, ...)` | `invoke("cmd", { input: {...}, other: {...} })` |

```typescript
// ✅ Struct param — wrap under key name
invoke("rotate_api_key", { input: { id } })

// ❌ Flat args when backend expects struct param
invoke("rotate_api_key", { id })
```

## Serde Casing Rules

| Rust annotation | Serialized field names | Zod schema should use |
|-----------------|------------------------|----------------------|
| `#[serde(rename_all = "camelCase")]` | `projectIds`, `createdAt` | camelCase |
| No annotation | `project_id`, `created_at` | snake_case |

❌ Do NOT assume all Rust structs use camelCase — always check `#[serde(...)]` attributes.

## Field Casing in invoke Args

Tauri input structs use `#[serde(rename_all = "camelCase")]` for request deserialization, so `invoke()` args inside the struct wrapper must also use camelCase:

```typescript
// ✅
invoke("create_api_key", { input: { name, projectIds, permissions } })

// ❌ snake_case in invoke args
invoke("create_api_key", { input: { name, project_ids, permissions } })
```

**Reference:** `src/CLAUDE.md` rule 14 — Tauri invoke: camelCase fields.

## Direct Flat Params (No Struct)

When Rust uses flat params (e.g., `fn cmd(id: String, project_id: String, ...)`), Tauri auto-converts camelCase JS keys to snake_case. Both `{ id }` and `{ projectId }` → `project_id` work, but prefer **camelCase** for consistency.

❌ Inconsistency in codebase: some files use `{ bucket_id: bucketId }` (snake_case), others use `{ projectId }` (camelCase). Both are correct but pick one style per file.

## Commands Without Input Args

Commands that take only `app_state` / `db` (no user input) use `invoke("cmd")` with no args object.

```typescript
// ✅ No args needed
invoke<unknown[]>("list_api_keys")
```

## Audit Results (2026-03-11)

Full audit of 106 `invoke()` calls across 31 files in `src/hooks/`, `src/api/`, `src/lib/api/`, `src/components/`:

| Finding | Result |
|---------|--------|
| Struct-param wrapping bugs | **0 found** — all commands with struct params correctly use `{ paramName: { ... } }` |
| camelCase compliance | ✅ All invoke args use camelCase field names |
| Commands audited | 47 distinct Tauri commands verified against Rust signatures |
| Files with invoke() calls | 31 (hooks, api, lib/api, components detail views) |
