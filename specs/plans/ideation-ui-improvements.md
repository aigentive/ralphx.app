# Ideation UI Improvements Plan

## Overview

Two features to improve the ideation flow:
1. **Start Ideation from Draft Task** - Seed ideation sessions with existing draft tasks
2. **Drag-and-Drop Markdown** - Import documentation by dragging files into proposals panel

---

## Feature 1: Start Ideation from Draft Task

### Recommended Approach: Multiple Entry Points

Different user workflows justify different entry points:
- **From Kanban:** User sees a draft task and thinks "this needs more exploration"
- **From Ideation:** User is already ideating and wants to pull in existing drafts

### Entry Points

| Location | Trigger | UX |
|----------|---------|-----|
| TaskCardContextMenu | Right-click draft task | "Start Ideation" menu item with lightbulb icon |
| TaskDetailOverlay | Click draft task | "Start Ideation" button in header (before Edit) |
| StartSessionPanel | In Ideation, no session | "Seed from Draft Task" link → opens TaskPickerDialog |

### User Flow (from Kanban)

```
Right-click draft task in Kanban
    ↓
Click "Start Ideation" in context menu
    ↓
Navigate to Ideation view
    ↓
Create session with seedTaskId
    ↓
Session title: "Ideation: {task.title}"
Task context injected as system message
```

### Data Model Changes

```typescript
// Extend IdeationSession
interface IdeationSession {
  // ... existing fields
  seedTaskId?: string;  // NEW: reference to source draft task
}

// Extend CreateSessionInput
interface CreateSessionInput {
  projectId: string;
  title?: string;
  seedTaskId?: string;  // NEW
}
```

### Files to Modify

| File | Change |
|------|--------|
| `src/types/ideation.ts` | Add `seedTaskId` field to schema |
| `src/api/ideation.ts` | Pass `seed_task_id` to backend |
| `src/hooks/useIdeation.ts` | Extend `CreateSessionInput` |
| `src/components/tasks/TaskCardContextMenu.tsx` | Add "Start Ideation" menu item |
| `src/components/tasks/TaskDetailOverlay.tsx` | Add "Start Ideation" button |
| `src/components/Ideation/StartSessionPanel.tsx` | Add "Seed from Draft" link |
| `src-tauri/src/commands/ideation_commands.rs` | Accept `seed_task_id` param |

### New Components

| Component | Purpose |
|-----------|---------|
| `src/components/Ideation/TaskPickerDialog.tsx` | Modal to select draft task from list |

---

## Feature 2: Drag-and-Drop Markdown Files

### Recommended Approach: Proposals Panel as Drop Zone

- Large drop target (entire middle panel)
- Works in both empty and populated states
- Consistent with macOS file import patterns

### User Flow

```
Drag .md file from Finder
    ↓
Enter proposals panel area
    ↓
Visual feedback:
  - Pulsing orange (#ff6b35) border
  - Overlay: "Drop to import plan"
  - Background dims
    ↓
Drop file
    ↓
Validate: single file, .md extension, <1MB
    ↓
Call existing /api/create_plan_artifact
    ↓
Load artifact, show success toast
```

### Visual Design

**During Drag:**
```
┌━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  ← Pulsing orange border
│ Proposals                      [3] │
├────────────────────────────────────┤
│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│
│░░░░░░░░  ╔═══════════════╗  ░░░░░░░│
│░░░░░░░░  ║  📄 ↓         ║  ░░░░░░░│
│░░░░░░░░  ║ Drop to import║  ░░░░░░░│
│░░░░░░░░  ╚═══════════════╝  ░░░░░░░│
│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│
└━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

**Empty State Enhancement:**
```
        💡 No proposals yet
   Ideas from chat appear here

   ─────────── or ───────────

   Drag a markdown file here
   to import a plan
```

### Files to Modify

| File | Change |
|------|--------|
| `src/components/Ideation/IdeationView.tsx` | Add drag handlers to proposals panel |
| `src/components/Ideation/useIdeationHandlers.ts` | Extract reusable `handleFileImport()` |
| `src/components/Ideation/ProposalsEmptyState.tsx` | Add drop hint text |

### New Components

| Component | Purpose |
|-----------|---------|
| `src/hooks/useFileDrop.ts` | Reusable drag-and-drop hook |
| `src/components/Ideation/DropZoneOverlay.tsx` | Visual overlay during drag |

---

## Implementation Sequence

### Phase 1: Data Model & API
1. Extend `IdeationSessionSchema` with `seedTaskId`
2. Update `ideationApi.sessions.create` to pass parameter
3. Update backend `create_ideation_session` command

### Phase 2: Entry Points (Feature 1)
4. Add "Start Ideation" to `TaskCardContextMenu`
5. Add "Start Ideation" button to `TaskDetailOverlay`
6. Create `TaskPickerDialog` component
7. Update `StartSessionPanel` with "Seed from Draft" link

### Phase 3: Drag-and-Drop (Feature 2)
8. Create `useFileDrop` hook
9. Create `DropZoneOverlay` component
10. Integrate into `IdeationView` proposals panel
11. Enhance `ProposalsEmptyState` with drop hint

---

## Verification

1. **Feature 1 - From Kanban:**
   - Right-click draft task → "Start Ideation" visible
   - Click → navigates to Ideation with new session
   - Session title shows task name

2. **Feature 1 - From Task Detail:**
   - Open draft task → "Start Ideation" button visible
   - Click → same behavior as context menu

3. **Feature 1 - From Ideation:**
   - In StartSessionPanel → "Seed from Draft" link visible
   - Click → TaskPickerDialog opens with draft tasks
   - Select task → session created with context

4. **Feature 2 - Drag-and-Drop:**
   - Drag .md file → orange border pulses
   - Drop → file imported as plan artifact
   - Invalid file → error toast

5. **Feature 2 - Empty State:**
   - No proposals → drop hint text visible
