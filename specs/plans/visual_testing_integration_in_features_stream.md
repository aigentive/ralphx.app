# Plan: Visual Testing Integration in Features Stream

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Browser tool | **agent-browser** | AI-judged verification, no baseline images needed |
| Dev server | **Start if needed** | Agent starts `npm run dev:web` (background) if unavailable |
| Mock scope | **Minimal** | Just enough to render, fast iteration |

## Implementation (Progressive Discovery)

**Key Principle:** Agent only loads visual verification details when it reaches that step. Keep main workflow lean.

### Task 1: Create `.claude/rules/visual-verification.md` (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `docs(rules): add visual verification workflow`

This file is ONLY read when step 5.5-6.5 triggers. Contains all the detail:

```markdown
# Visual Verification Workflow

**Required Context:** Read ONLY when stream-features.md step 5.5-6.5 triggers.

## Step 5.5: Mock Layer Check

1. Identify Tauri commands used by new/modified UI code
2. Check if src/api-mock/ has matching mock
3. Missing? → Create minimal mock:
   - Add to src/api-mock/{domain}.ts
   - Just enough to render (not all states)
   - Export from src/api-mock/index.ts
4. Verify: web mode renders without undefined errors

## Step 6.5: Visual Verification

1. Check if dev server running at http://localhost:5173
2. If unavailable → start: npm run dev:web (background)
3. Use agent-browser:
   a. Open the feature view
   b. Snapshot interactive elements
   c. Take screenshot
   d. AI-judge: Does it match expected design?
4. Visual issues? → Fix before proceeding

## Screenshot Convention

Format: YYYY-MM-DD_HH-MM-SS_[task-name].png
Example: 2026-01-31_14-30-45_kanban-filter-panel.png
Location: screenshots/features/

## Minimal Mock Pattern

async get_feature_data(id: string): Promise<FeatureData> {
  return { id, name: "Test Feature", items: [] };
}
```

**Files:** `.claude/rules/visual-verification.md` [NEW]

---

### Task 2: Update `stream-features.md`
**Dependencies:** Task 1
**Atomic Commit:** `docs(rules): add visual verification pointer to features stream`

Add a lightweight pointer after step 5:

```markdown
5. Execute task following PRD steps

5.5-6.5. Visual Verification (if affects UI):
   - Could this task change what the user sees?
     (Components, views, API responses rendered in UI, stores, types, styles, etc.)
   - YES → Read .claude/rules/visual-verification.md and follow its workflow
   - NO → Skip to step 6
```

**Note:** Use plain path (not `@` notation) to avoid auto-loading. Let agent judge "affects UI" naturally rather than hard-coding paths - API changes, store changes, type changes can all affect rendering.

**Files:** `.claude/rules/stream-features.md`

---

### Updated Workflow Summary

```
1-5. (unchanged - check P0, read PRD, execute task)

5.5-6.5. [NEW] Visual Verification pointer:
         - Is this a UI task?
         - YES → read visual-verification.md → execute its workflow
         - NO → skip

6-10. (unchanged - lint, log, mark passes, commit, stop)
```

## Files to Modify

| File | Change | Task |
|------|--------|------|
| `.claude/rules/visual-verification.md` | [NEW] Full workflow (mock check, browser verification, screenshot convention) | Task 1 |
| `.claude/rules/stream-features.md` | Add minimal pointer: "if UI task → read visual-verification.md" | Task 2 |

## Verification

Test the integration by:
1. Running features stream on a UI task
2. Confirming mock check runs (or skips for non-UI)
3. Confirming agent-browser verification runs
4. Confirming visual issues block completion

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
