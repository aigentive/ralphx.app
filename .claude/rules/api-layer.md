# API Layer Patterns

**Required Context:** @.claude/rules/code-quality-standards.md

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## Pipeline (CRITICAL)

```
Rust (snake_case) → Zod schema (snake_case) → Transform fn → TS types (camelCase)
```

| Layer | Format | File | Example |
|-------|--------|------|---------|
| Schema | snake_case | `{domain}.schemas.ts` | `project_id: z.string()` |
| Transform | conversion | `{domain}.transforms.ts` | `projectId: raw.project_id` |
| Types | camelCase | `{domain}.types.ts` | `projectId: string` |

## Invoke Helpers

| Helper | Use Case |
|--------|----------|
| `typedInvoke(cmd, args, schema)` | Validation only |
| `typedInvokeWithTransform(cmd, args, schema, fn)` | Validation + conversion |

## Tauri Params

| Context | JS | Rust | Failure |
|---------|----|----- |---------|
| Direct | `{ taskId }` | `task_id: String` | ❌ `{ task_id }` → silent missing |
| Struct | `{ input: { task_id } }` | serde match | Must match exactly |
| Response | — | default snake_case | ❌ NEVER `rename_all` |

## Domain API Pattern

```typescript
export const domainApi = {
  create: (input) => typedInvokeWithTransform("cmd", { input }, Schema, transform),
  list: (params) => typedInvokeWithTransform("cmd", params, z.array(Schema), xs => xs.map(transform)),
} as const;
```

Files: `{domain}.ts` | `{domain}.schemas.ts` | `{domain}.transforms.ts` | `{domain}.types.ts`

## Web Mode (Phase 55)

| Component | Location | Purpose |
|-----------|----------|---------|
| `isWebMode()` | `src/lib/tauri-detection.ts` | Environment check |
| Mock plugins | `src/mocks/tauri-plugin-*.ts` | Graceful degradation |
| Vite aliases | `vite.config.ts` | Redirect in web mode |

## EventBus

| Mode | Implementation | Use |
|------|----------------|-----|
| Native | `TauriEventBus` | Real Tauri `listen()` |
| Web/Test | `MockEventBus` | In-memory EventEmitter |

Pattern: `bus.subscribe(event, handler)` → returns unsubscribe fn

## New API Checklist

- [ ] Schema (snake_case, match Rust)
- [ ] Types (camelCase)
- [ ] Transform fn
- [ ] Domain API method using typedInvokeWithTransform
- [ ] Re-export in `src/lib/tauri.ts` if widely used
