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

```typescript
async get_feature_data(id: string): Promise<FeatureData> {
  return { id, name: "Test Feature", items: [] };
}
```
