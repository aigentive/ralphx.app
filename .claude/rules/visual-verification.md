# Visual Verification Workflow

**Required Context:** Auto-loaded by stream-features.md

> **CRITICAL:** This workflow is MANDATORY for any task that modifies UI files.
> You CANNOT mark a task as `"passes": true` without completing these steps.
> You CANNOT skip screenshot capture for UI tasks.
> You CANNOT delegate visual verification to sub-agents - execute DIRECTLY using the Skill tool.

## Step 5.5: Mock Layer Check

1. Identify Tauri commands used by new/modified UI code
2. Check if src/api-mock/ has matching mock
3. Missing? → Create minimal mock:
   - Add to src/api-mock/{domain}.ts
   - Just enough to render (not all states)
   - Export from src/api-mock/index.ts
4. Verify: web mode renders without undefined errors

## Step 6.5: Visual Verification (NO DELEGATION)

**Execute DIRECTLY using the Skill tool. Do NOT use Task tool to delegate.**

1. Check if dev server running at http://localhost:5173
2. If unavailable → start: `npm run dev:web` (background)
3. Need to reload changes? → restart: stop existing server, then `npm run dev:web`
4. Invoke `/agent-browser-skill` (Skill tool) to:
   a. Open the feature view
   b. Snapshot interactive elements
   c. Take screenshot
   d. AI-judge: Does it match expected design?
5. Visual issues? → Fix before proceeding

## Screenshot Convention

Format: YYYY-MM-DD_HH-MM-SS_[task-name].png
Example: 2026-01-31_14-30-45_kanban-filter-panel.png
Location: screenshots/features/

## Minimal Mock Pattern

```typescript
async get_feature_data(id: string): Promise<FeatureData> {
  return { id, name: "Test Feature", items: [] };
}
```
