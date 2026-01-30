# Plan: Inline Version Selector for Plan History

## Summary
Replace the modal-based plan version history with an inline version selector dropdown in `PlanDisplay`. When a version is selected, display its markdown content directly in the collapsible area. Also auto-expand the plan when there are no proposals.

## Changes

### 1. PlanDisplay.tsx — Add Version Selector (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(ideation): add inline version selector to PlanDisplay`

**Location:** `src/components/ideation/PlanDisplay.tsx`

**Add state and fetch logic:**
```typescript
const [selectedVersion, setSelectedVersion] = useState(plan.metadata.version);
const [historicalContent, setHistoricalContent] = useState<string | null>(null);
const [isLoadingVersion, setIsLoadingVersion] = useState(false);

// Fetch historical version when selection changes
useEffect(() => {
  if (selectedVersion === plan.metadata.version) {
    setHistoricalContent(null);
    return;
  }
  // Fetch via artifactApi.getAtVersion(plan.id, selectedVersion)
}, [plan.id, selectedVersion, plan.metadata.version]);
```

**Replace History button (lines 281-291) with dropdown:**
- Use DropdownMenu (shadcn/ui) pattern like ConversationSelector
- Trigger: `v{selectedVersion}` with History icon + ChevronDown
- Items: List versions from 1 to `plan.metadata.version` (newest first)
- Active state: dot indicator + left border accent

**Update content display:**
- Use `historicalContent ?? planContent` for display
- Show loading state during fetch
- Add "Viewing version X of Y" banner with "Back to latest" button when viewing historical

**Remove:** `onViewHistory` prop (no longer needed)

### 2. IdeationView.tsx — Remove Modal + Auto-expand
**Dependencies:** Task 1
**Atomic Commit:** `refactor(ideation): remove PlanHistoryDialog, add auto-expand`

**Location:** `src/components/ideation/IdeationView.tsx`

**Remove modal:**
- Remove `PlanHistoryDialog` import (line 28)
- Remove `planHistoryDialog`, `handleViewHistoricalPlan`, `handleClosePlanHistoryDialog` from destructured handlers (lines 174, 186-187)
- Delete `<PlanHistoryDialog>` render block (lines 442-448)
- Remove `onViewHistory` prop from `<PlanDisplay>` (line 363)

**Add auto-expand effect:**
```typescript
// Auto-expand plan when there are no proposals
useEffect(() => {
  if (planArtifact && proposals.length === 0 && !isPlanExpanded) {
    setIsPlanExpanded(true);
  }
}, [planArtifact, proposals.length, isPlanExpanded, setIsPlanExpanded]);
```

### 3. useIdeationHandlers.ts — Remove Modal State
**Dependencies:** Task 2
**Atomic Commit:** `refactor(ideation): remove plan history modal state`

**Location:** `src/components/ideation/useIdeationHandlers.ts`

**Remove:**
- `planHistoryDialog` state (line 22)
- `handleViewHistoricalPlan` callback (lines 61-63)
- `handleClosePlanHistoryDialog` callback (line 65)
- Remove from return object (lines 167, 179-180)

### 4. Delete PlanHistoryDialog.tsx
**Dependencies:** Task 2, Task 3
**Atomic Commit:** `chore(ideation): delete unused PlanHistoryDialog`

**Location:** `src/components/ideation/PlanHistoryDialog.tsx`

Delete entire file (171 LOC) — no longer needed.

## Files Modified

| File | Action | Delta |
|------|--------|-------|
| `PlanDisplay.tsx` | Add version selector, fetch logic | +80 LOC → ~415 |
| `IdeationView.tsx` | Remove modal, add auto-expand | -15 LOC → ~447 |
| `useIdeationHandlers.ts` | Remove modal state | -15 LOC → ~174 |
| `PlanHistoryDialog.tsx` | DELETE | -171 LOC |

**Net:** ~100 LOC reduction

## UI Pattern Reference

From `ConversationSelector.tsx`:
- DropdownMenu with `align="end"`
- Active item: `bg-[var(--accent-muted)]` + `border-l-2 border-[var(--accent-primary)]`
- Dot indicator for selected item
- Item padding: `px-3 py-2.5`

## Verification

1. **Version selector appears** when plan has `version > 1`
2. **Selecting a version** fetches and displays content inline
3. **Markdown renders** correctly (not plain text like old modal)
4. **"Back to latest"** button returns to current version
5. **Auto-expand works** when no proposals exist but plan exists
6. **Modal fully removed** (no PlanHistoryDialog references)

## Commit Lock Workflow (Parallel Agent Coordination)

Reference: `.claude/rules/commit-lock.md`

### Before Committing
```bash
# 1. Establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Check/acquire lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Read lock content, wait 3s, retry up to 30s
  # If stale (same content >30s), delete and proceed
fi

# 3. Create lock
echo "<stream-name> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 4. Stage and commit
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

### After Committing
```bash
# ALWAYS release lock (success or failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Lock Rules
1. Acquire lock BEFORE `git add`
2. Release lock AFTER commit (success OR failure)
3. Stale = same content + >30 sec old
4. Never force-delete active lock from another agent
