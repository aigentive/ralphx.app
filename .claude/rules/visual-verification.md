# Visual Verification Workflow

**Required Context:** @.claude/rules/code-quality-standards.md

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

> **CRITICAL:** This workflow is MANDATORY for any task that modifies UI files.
> You CANNOT mark a task as `"passes": true` without completing these steps.
> You CANNOT skip screenshot capture for UI tasks.
> You CANNOT delegate visual verification to sub-agents - execute DIRECTLY using the Skill tool.

## Step 6.0: Mock Layer Check (PRODUCES EVIDENCE)

| # | Action |
|---|--------|
| 1 | Grep modified .tsx files for `invoke(` calls → list all Tauri commands |
| 2 | Check if src/api-mock/ has matching mock for each command |
| 3 | Missing? → Create minimal mock (add to src/api-mock/{domain}.ts, export from index.ts) |
| 4 | Verify: `npm run dev:web` renders without undefined errors |
| 5 | **CREATE EVIDENCE FILE** at `screenshots/features/YYYY-MM-DD_HH-MM-SS_[task-name]_mock-check.md` |

### Mock-Check Evidence Template

```markdown
# Mock Parity Check - [Task Name]

## Commands Found
- `command_name` → ✅ mock exists | ❌ CREATED mock

## Web Mode Test
- URL: http://localhost:5173/[path]
- Renders: ✅ Yes | ❌ No (fixed: [description])

## Result: PASS
```

## Step 6.5: Visual Verification (NO DELEGATION)

**Execute DIRECTLY using the Skill tool. Do NOT use Task tool to delegate.**

| # | Action |
|---|--------|
| 1 | Check dev server at http://localhost:5173 |
| 2 | If unavailable → start: `npm run dev:web` (background) |
| 3 | Need reload? → restart server |
| 4 | Invoke `/agent-browser-skill` (Skill tool) to: open view, snapshot, take screenshot |
| 5 | **Analyze screenshot against PRD** (see criteria below) |
| 6 | Empty/missing data? → Log P0 gap → STOP |
| 7 | Visual issues fixable now? → Fix before proceeding |

### Screenshot Analysis Criteria

| Check | Pass | Fail |
|-------|------|------|
| Data/content matches PRD | ✅ | ❌ Log P0 `[Visual/Mock]` |
| Data populated (not empty/undefined) | ✅ | ❌ Log P0 `[Visual/Mock]` |
| All specified UI elements appear | ✅ | ❌ Log P0 `[Visual/Mock]` |

## Step 6.9: Checkpoint (BLOCKING)

| Evidence | Path | If Missing |
|----------|------|------------|
| Mock-check | `screenshots/features/*_mock-check.md` | STOP → return to 6.0 |
| Screenshot | `screenshots/features/*.png` | STOP → return to 6.5 |
| PRD content | Data visible in screenshot | STOP → log P0 `[Visual/Mock]` |

All conditions pass → Visual verification complete → Proceed to step 7.

## Screenshot Convention

| Field | Value |
|-------|-------|
| Format | `YYYY-MM-DD_HH-MM-SS_[task-name].png` |
| Example | `2026-01-31_14-30-45_kanban-filter-panel.png` |
| Location | `screenshots/features/` |

## Minimal Mock Pattern

```typescript
async get_feature_data(id: string): Promise<FeatureData> {
  return { id, name: "Test Feature", items: [] };
}
```

## P0 Gap Format

When logging gaps to `streams/features/backlog.md`:

```markdown
- [ ] [Visual/Mock] [Component]: Missing mock data for [description] - prevents visual verification
```
