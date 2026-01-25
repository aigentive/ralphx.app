# Workflow Management UI Implementation Plan

## Problem

1. **WorkflowsPanel buttons don't work** - "New Workflow", Edit, Duplicate, Delete buttons have no click handlers
2. **WorkflowsPanel uses mock data** - Hardcoded array instead of `useWorkflows()` hook
3. **Kanban view has no workflow selector** - Uses hardcoded `DEFAULT_WORKFLOW_ID = "ralphx-default"`
4. **Existing components not integrated** - `WorkflowSelector`, `WorkflowEditor`, `TaskBoardWithHeader` are built but unused
5. **No "Set as Default" action** - Can't change the default workflow from UI
6. **No "Revert to Built-in" action** - Can't reset to RalphX Default workflow

## Solution

Wire up existing components - **no new components needed**:

1. Connect `WorkflowsPanel` to real data via `useWorkflows()` hook
2. Add state management for edit/create modal
3. Wire up all button handlers to mutations
4. Replace `TaskBoard` with `TaskBoardWithHeader` in App.tsx
5. Add "Set as Default" and "Revert to Built-in" actions

---

## Architecture Overview

### Current State (Broken)

```
┌─────────────────────────────────────────────────────────────────────┐
│ ExtensibilityView → WorkflowsPanel                                  │
│                                                                      │
│   workflows = [{ id: "default", name: "Default Kanban", ... }]     │  ← MOCK DATA
│                                                                      │
│   <Button>New Workflow</Button>           ← NO onClick              │
│   <Button><Edit /></Button>               ← NO onClick              │
│   <Button><Copy /></Button>               ← NO onClick              │
│   <Button><Trash2 /></Button>             ← NO onClick              │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│ App.tsx → Kanban View                                               │
│                                                                      │
│   const DEFAULT_WORKFLOW_ID = "ralphx-default";                     │  ← HARDCODED
│                                                                      │
│   <TaskBoard workflowId={DEFAULT_WORKFLOW_ID} />                    │  ← NO SELECTOR
└─────────────────────────────────────────────────────────────────────┘
```

### Target State (Working)

```
┌─────────────────────────────────────────────────────────────────────┐
│ ExtensibilityView → WorkflowsPanel                                  │
│                                                                      │
│   const { data: workflows } = useWorkflows();                       │  ← REAL DATA
│   const createWorkflow = useCreateWorkflow();                       │
│   const updateWorkflow = useUpdateWorkflow();                       │
│   const deleteWorkflow = useDeleteWorkflow();                       │
│   const setDefault = useSetDefaultWorkflow();                       │
│                                                                      │
│   [editorOpen, setEditorOpen] = useState(false);                    │
│   [editingWorkflow, setEditingWorkflow] = useState(null);           │
│                                                                      │
│   <Button onClick={() => setEditorOpen(true)}>New Workflow</Button> │  ← WORKS
│   <Button onClick={() => openEditor(workflow)}>Edit</Button>        │  ← WORKS
│   <Button onClick={() => duplicate(workflow)}>Copy</Button>         │  ← WORKS
│   <Button onClick={() => confirmDelete(workflow)}>Delete</Button>   │  ← WORKS
│   <Button onClick={() => setDefault(workflow.id)}>Set Default</Button>
│                                                                      │
│   <Dialog open={editorOpen}>                                        │
│     <WorkflowEditor workflow={editingWorkflow} onSave={...} />      │
│   </Dialog>                                                          │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│ App.tsx → Kanban View                                               │
│                                                                      │
│   <TaskBoardWithHeader projectId={currentProjectId} />              │  ← HAS SELECTOR
│       │                                                              │
│       └── <WorkflowSelector />  ← User can switch workflows         │
│       └── <TaskBoard workflowId={selectedWorkflowId} />             │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### Workflow CRUD Flow

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   WorkflowsPanel │     │   useWorkflows   │     │   Tauri Backend  │
│                  │     │   (TanStack)     │     │   (Rust)         │
└────────┬─────────┘     └────────┬─────────┘     └────────┬─────────┘
         │                        │                        │
         │  useWorkflows()        │                        │
         │───────────────────────>│  get_workflows         │
         │                        │───────────────────────>│
         │                        │                        │
         │                        │<───────────────────────│
         │  workflows[]           │                        │
         │<───────────────────────│                        │
         │                        │                        │
         │  createWorkflow.mutate │                        │
         │───────────────────────>│  create_workflow       │
         │                        │───────────────────────>│
         │                        │                        │
         │                        │  invalidateQueries()   │
         │                        │<───────────────────────│
         │  UI re-renders         │                        │
         │<───────────────────────│                        │
```

### Kanban Workflow Selection Flow

```
┌──────────────────────────────────────────────────────────────────────┐
│                     TaskBoardWithHeader                              │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   const { data: workflows } = useWorkflows();                       │
│   const defaultWorkflow = workflows.find(w => w.isDefault);         │
│   const [selectedId, setSelectedId] = useState(null);               │
│   const currentId = selectedId ?? defaultWorkflow?.id;              │
│                                                                      │
│   ┌────────────────────────────────────────────────────────────┐    │
│   │ Header                                                      │    │
│   │  ┌─────────────────────────────────────────────────────┐   │    │
│   │  │ WorkflowSelector                                     │   │    │
│   │  │  [RalphX Default ▼] [Default badge]                 │   │    │
│   │  │                                                      │   │    │
│   │  │  Dropdown:                                           │   │    │
│   │  │  ├─ RalphX Default (7 columns) [Default]            │   │    │
│   │  │  ├─ Jira Compatible (5 columns)                     │   │    │
│   │  │  └─ My Custom (4 columns)                           │   │    │
│   │  └─────────────────────────────────────────────────────┘   │    │
│   └────────────────────────────────────────────────────────────┘    │
│                                                                      │
│   ┌────────────────────────────────────────────────────────────┐    │
│   │ TaskBoard (workflowId={currentId})                          │    │
│   │                                                              │    │
│   │  Columns derived from selected workflow                     │    │
│   └────────────────────────────────────────────────────────────┘    │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Steps

### 1. Refactor WorkflowsPanel to Use Real Data

**File:** `src/components/ExtensibilityView.tsx`

Replace mock data with hooks:

```typescript
function WorkflowsPanel() {
  // Real data from backend
  const { data: workflowsData, isLoading, error } = useWorkflows();
  const createWorkflow = useCreateWorkflow();
  const updateWorkflow = useUpdateWorkflow();
  const deleteWorkflow = useDeleteWorkflow();
  const setDefaultWorkflow = useSetDefaultWorkflow();

  // Convert API response (snake_case) to frontend format (camelCase)
  const workflows = useMemo(() => {
    if (!workflowsData) return [];
    return workflowsData.map(toWorkflowSchema);
  }, [workflowsData]);

  // Editor modal state
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [editingWorkflow, setEditingWorkflow] = useState<WorkflowSchema | null>(null);

  // Delete confirmation state
  const [deleteConfirmId, setDeleteConfirmId] = useState<string | null>(null);

  // Handlers
  const handleNewWorkflow = useCallback(() => {
    setEditingWorkflow(null); // null = create mode
    setIsEditorOpen(true);
  }, []);

  const handleEditWorkflow = useCallback((workflow: WorkflowSchema) => {
    setEditingWorkflow(workflow);
    setIsEditorOpen(true);
  }, []);

  const handleDuplicateWorkflow = useCallback((workflow: WorkflowSchema) => {
    // Create a copy with modified name
    const duplicate = {
      ...workflow,
      id: undefined, // Let backend generate new ID
      name: `${workflow.name} (Copy)`,
      isDefault: false,
    };
    setEditingWorkflow(duplicate);
    setIsEditorOpen(true);
  }, []);

  const handleDeleteWorkflow = useCallback(async (workflowId: string) => {
    await deleteWorkflow.mutateAsync(workflowId);
    setDeleteConfirmId(null);
  }, [deleteWorkflow]);

  const handleSetDefault = useCallback(async (workflowId: string) => {
    await setDefaultWorkflow.mutateAsync(workflowId);
  }, [setDefaultWorkflow]);

  const handleSaveWorkflow = useCallback(async (workflowData: WorkflowInput) => {
    if (editingWorkflow?.id) {
      // Update existing
      await updateWorkflow.mutateAsync({
        id: editingWorkflow.id,
        input: toUpdateInput(workflowData),
      });
    } else {
      // Create new
      await createWorkflow.mutateAsync(toCreateInput(workflowData));
    }
    setIsEditorOpen(false);
    setEditingWorkflow(null);
  }, [editingWorkflow, createWorkflow, updateWorkflow]);

  // ... rest of component
}
```

### 2. Add Editor Dialog to WorkflowsPanel

**File:** `src/components/ExtensibilityView.tsx`

Add Dialog with WorkflowEditor:

```typescript
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { WorkflowEditor } from "@/components/workflows/WorkflowEditor";

// Inside WorkflowsPanel return:
return (
  <div data-testid="workflows-panel" className="space-y-4">
    {/* ... existing header and cards ... */}

    {/* Workflow Editor Dialog */}
    <Dialog open={isEditorOpen} onOpenChange={setIsEditorOpen}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {editingWorkflow?.id ? "Edit Workflow" : "Create Workflow"}
          </DialogTitle>
        </DialogHeader>
        <WorkflowEditor
          workflow={editingWorkflow ?? undefined}
          onSave={handleSaveWorkflow}
          onCancel={() => setIsEditorOpen(false)}
          isSaving={createWorkflow.isPending || updateWorkflow.isPending}
        />
      </DialogContent>
    </Dialog>

    {/* Delete Confirmation Dialog */}
    <AlertDialog open={!!deleteConfirmId} onOpenChange={() => setDeleteConfirmId(null)}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete Workflow?</AlertDialogTitle>
          <AlertDialogDescription>
            This action cannot be undone. Tasks using this workflow will fall back to the default.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={() => deleteConfirmId && handleDeleteWorkflow(deleteConfirmId)}
            className="bg-red-500 hover:bg-red-600"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
);
```

### 3. Wire Up Button Handlers

**File:** `src/components/ExtensibilityView.tsx`

Update buttons to call handlers:

```typescript
{/* Header */}
<div className="flex items-center justify-between">
  <h2 className="text-lg font-semibold" style={{ color: "var(--text-primary)" }}>
    Workflow Schemas
  </h2>
  <Button
    variant="secondary"
    size="sm"
    className="gap-1.5"
    onClick={handleNewWorkflow}
  >
    <Plus className="w-4 h-4" />
    New Workflow
  </Button>
</div>

{/* Card action buttons */}
<div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
  <Tooltip>
    <TooltipTrigger asChild>
      <Button
        variant="ghost"
        size="sm"
        className="h-7 w-7 p-0"
        onClick={() => handleEditWorkflow(workflow)}
      >
        <Edit className="w-4 h-4" />
      </Button>
    </TooltipTrigger>
    <TooltipContent>Edit</TooltipContent>
  </Tooltip>

  <Tooltip>
    <TooltipTrigger asChild>
      <Button
        variant="ghost"
        size="sm"
        className="h-7 w-7 p-0"
        onClick={() => handleDuplicateWorkflow(workflow)}
      >
        <Copy className="w-4 h-4" />
      </Button>
    </TooltipTrigger>
    <TooltipContent>Duplicate</TooltipContent>
  </Tooltip>

  {!workflow.isDefault && (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={() => handleSetDefault(workflow.id)}
          >
            <Star className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Set as Default</TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0 text-red-400 hover:text-red-300"
            onClick={() => setDeleteConfirmId(workflow.id)}
          >
            <Trash2 className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Delete</TooltipContent>
      </Tooltip>
    </>
  )}
</div>
```

### 4. Replace TaskBoard with TaskBoardWithHeader in App.tsx

**File:** `src/App.tsx`

Change import and usage:

```typescript
// Change import
import { TaskBoardWithHeader } from "@/components/tasks/TaskBoard";

// Remove the hardcoded constant
// const DEFAULT_WORKFLOW_ID = "ralphx-default";  // DELETE THIS

// Update the Kanban view section
{currentView === "kanban" && (
  <TaskBoardWithHeader projectId={currentProjectId} />
)}
```

### 5. Add "Seed Built-in Workflows" on App Startup

**File:** `src/App.tsx` or `src/providers/WorkflowProvider.tsx`

Ensure built-in workflows exist:

```typescript
import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// In App.tsx setup or a dedicated provider
useEffect(() => {
  // Seed built-in workflows if they don't exist
  invoke("seed_builtin_workflows").catch(console.error);
}, []);
```

### 6. Add "Revert to Built-in Default" Button

**File:** `src/components/ExtensibilityView.tsx`

Add a button in the header area:

```typescript
const handleRevertToDefault = useCallback(async () => {
  // First ensure built-ins exist
  await invoke("seed_builtin_workflows");
  // Then set RalphX Default as the active default
  await setDefaultWorkflow.mutateAsync("ralphx-default");
}, [setDefaultWorkflow]);

// In the header area
<div className="flex items-center gap-2">
  <Button
    variant="ghost"
    size="sm"
    onClick={handleRevertToDefault}
    disabled={workflows.find(w => w.isDefault)?.id === "ralphx-default"}
  >
    <RotateCcw className="w-4 h-4 mr-1.5" />
    Reset to Default
  </Button>
  <Button variant="secondary" size="sm" className="gap-1.5" onClick={handleNewWorkflow}>
    <Plus className="w-4 h-4" />
    New Workflow
  </Button>
</div>
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/components/ExtensibilityView.tsx` | Replace mock data with hooks, add handlers, add dialogs |
| `src/App.tsx` | Replace `TaskBoard` with `TaskBoardWithHeader`, remove `DEFAULT_WORKFLOW_ID`, add seed on startup |
| `src/components/workflows/WorkflowEditor.tsx` | Minor: ensure props match expected interface |

## Files Already Complete (No Changes Needed)

| File | Status |
|------|--------|
| `src/components/workflows/WorkflowSelector.tsx` | ✅ Complete |
| `src/components/workflows/WorkflowEditor.tsx` | ✅ Complete |
| `src/components/tasks/TaskBoard/TaskBoardWithHeader.tsx` | ✅ Complete |
| `src/hooks/useWorkflows.ts` | ✅ Complete |
| `src/lib/api/workflows.ts` | ✅ Complete |
| `src-tauri/src/commands/workflow_commands.rs` | ✅ Complete |

---

## Conversion Helpers

Add these helper functions to convert between API response format and frontend format:

```typescript
// src/components/ExtensibilityView.tsx or src/lib/workflow-utils.ts

import type { WorkflowResponse, CreateWorkflowInput, UpdateWorkflowInput } from "@/lib/api/workflows";
import type { WorkflowSchema, WorkflowColumn } from "@/types/workflow";

/**
 * Convert WorkflowResponse (snake_case from API) to WorkflowSchema (camelCase)
 */
export function toWorkflowSchema(response: WorkflowResponse): WorkflowSchema {
  return {
    id: response.id,
    name: response.name,
    description: response.description ?? undefined,
    columns: response.columns.map((col) => ({
      id: col.id,
      name: col.name,
      mapsTo: col.maps_to,
      color: col.color ?? undefined,
      icon: col.icon ?? undefined,
      behavior: {
        skipReview: col.skip_review ?? undefined,
        autoAdvance: col.auto_advance ?? undefined,
        agentProfile: col.agent_profile ?? undefined,
      },
    })),
    isDefault: response.is_default,
  };
}

/**
 * Convert frontend WorkflowSchema to CreateWorkflowInput
 */
export function toCreateInput(schema: Omit<WorkflowSchema, "id">): CreateWorkflowInput {
  return {
    name: schema.name,
    description: schema.description,
    columns: schema.columns.map((col) => ({
      id: col.id,
      name: col.name,
      maps_to: col.mapsTo,
      color: col.color,
      icon: col.icon,
      skip_review: col.behavior?.skipReview,
      auto_advance: col.behavior?.autoAdvance,
      agent_profile: col.behavior?.agentProfile,
    })),
    is_default: schema.isDefault,
  };
}

/**
 * Convert frontend WorkflowSchema to UpdateWorkflowInput
 */
export function toUpdateInput(schema: Partial<WorkflowSchema>): UpdateWorkflowInput {
  const input: UpdateWorkflowInput = {};

  if (schema.name !== undefined) input.name = schema.name;
  if (schema.description !== undefined) input.description = schema.description;
  if (schema.isDefault !== undefined) input.is_default = schema.isDefault;
  if (schema.columns !== undefined) {
    input.columns = schema.columns.map((col) => ({
      id: col.id,
      name: col.name,
      maps_to: col.mapsTo,
      color: col.color,
      icon: col.icon,
      skip_review: col.behavior?.skipReview,
      auto_advance: col.behavior?.autoAdvance,
      agent_profile: col.behavior?.agentProfile,
    }));
  }

  return input;
}
```

---

## UI Behavior Details

### WorkflowsPanel States

| State | Display |
|-------|---------|
| Loading | Show skeleton cards |
| Empty (no workflows) | Show empty state with "Create Workflow" CTA |
| Error | Show error message with retry button |
| Has workflows | Show workflow cards with actions |

### Workflow Card Actions

| Action | Visibility | Behavior |
|--------|------------|----------|
| Edit | Always | Opens WorkflowEditor in dialog |
| Duplicate | Always | Opens WorkflowEditor with copy of workflow |
| Set as Default | Non-default only | Calls `setDefaultWorkflow`, updates badge |
| Delete | Non-default only | Shows confirmation dialog, then deletes |

### WorkflowEditor Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| Create | "New Workflow" button | Empty form, calls `createWorkflow` |
| Edit | Edit button on card | Pre-filled form, calls `updateWorkflow` |
| Duplicate | Copy button on card | Pre-filled with "(Copy)" suffix, calls `createWorkflow` |

---

## Verification Checklist

### WorkflowsPanel
- [ ] Displays real workflows from backend (not mock data)
- [ ] Shows loading state while fetching
- [ ] Shows empty state when no workflows exist
- [ ] "New Workflow" button opens editor dialog
- [ ] Edit button opens editor with workflow data
- [ ] Duplicate button opens editor with copy
- [ ] Delete button shows confirmation, then deletes
- [ ] "Set as Default" button works for non-default workflows
- [ ] "Reset to Default" restores RalphX Default as active
- [ ] DEFAULT badge appears on default workflow

### Kanban View
- [ ] Shows WorkflowSelector in header
- [ ] Defaults to the default workflow
- [ ] Can switch between workflows
- [ ] Columns update when workflow changes
- [ ] Tasks display correctly in mapped columns

### Integration
- [ ] Creating workflow in ExtensibilityView shows in Kanban selector
- [ ] Deleting active workflow falls back gracefully
- [ ] Setting new default updates both views
- [ ] Built-in workflows seeded on app startup

---

## Error Handling

| Scenario | Handling |
|----------|----------|
| Create fails | Show toast error, keep dialog open |
| Update fails | Show toast error, keep dialog open |
| Delete fails | Show toast error, close confirmation |
| Set default fails | Show toast error |
| Fetch fails | Show error state with retry button |
| No workflows exist | Trigger `seed_builtin_workflows` automatically |

---

## Methodology-Workflow Relationship

### Overview

Methodologies (BMAD, GSD) embed their own workflow schemas. When a methodology is activated, its embedded workflow becomes the active Kanban layout. This creates a coupling that must be managed carefully.

### Data Model Relationship

```
┌─────────────────────────────────────────────────────────────────────────┐
│ MethodologyExtension                                                     │
├─────────────────────────────────────────────────────────────────────────┤
│ id: "bmad"                                                               │
│ name: "BMAD Method"                                                      │
│ is_active: true                                                          │
│                                                                          │
│ workflow: WorkflowSchema {                      ← EMBEDDED WORKFLOW      │
│   id: "bmad-workflow"                                                    │
│   name: "BMAD Workflow"                                                  │
│   columns: [10 columns...]                                               │
│   is_default: false                                                      │
│ }                                                                        │
│                                                                          │
│ agents: [8 agent configurations...]                                      │
│ skills: [...]                                                            │
│ phases: [...]                                                            │
└─────────────────────────────────────────────────────────────────────────┘
```

### Key Behaviors

#### 1. Workflows Tab Shows ALL Workflows

The Workflows tab displays:
- Standalone workflows (RalphX Default, Jira Compatible, user-created)
- Methodology-embedded workflows (BMAD Workflow, GSD Workflow)

Methodology workflows are marked with an indicator showing which methodology owns them.

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Workflow Schemas                                        [Reset] [+ New] │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ RalphX Default                                     [DEFAULT] [⚙️]  │ │
│  │ 7 columns: Backlog → Approved                                      │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ Jira Compatible                                              [⚙️]  │ │
│  │ 5 columns: To Do → Done                                            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ BMAD Workflow                            [Part of BMAD Method] [⚙️] │ │
│  │ 10 columns: Business Analysis → Approved                           │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ GSD Workflow                              [Part of GSD Method] [⚙️] │ │
│  │ 11 columns: Gather → Done                                          │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 2. Methodology Activation Flow

When user activates a methodology (e.g., BMAD):

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     USER CLICKS "ACTIVATE BMAD"                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    CONFIRMATION DIALOG                            │   │
│  │                                                                    │   │
│  │  ⚠️ Activating "BMAD Method" will change your Kanban layout       │   │
│  │     to "BMAD Workflow" (10 columns).                              │   │
│  │                                                                    │   │
│  │  Current workflow: RalphX Default (7 columns)                     │   │
│  │  New workflow: BMAD Workflow (10 columns)                         │   │
│  │                                                                    │   │
│  │  Continue?                                                         │   │
│  │                                                                    │   │
│  │                              [Cancel]  [Activate]                  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────┐                     ┌─────────────────────────┐    │
│  │ User clicks     │                     │ User clicks             │    │
│  │ [Cancel]        │                     │ [Activate]              │    │
│  └────────┬────────┘                     └────────────┬────────────┘    │
│           │                                           │                  │
│           ▼                                           ▼                  │
│  ┌─────────────────┐                     ┌─────────────────────────┐    │
│  │ NO CHANGES      │                     │ 1. Activate methodology │    │
│  │ - Methodology   │                     │ 2. Set BMAD workflow    │    │
│  │   stays inactive│                     │    as active            │    │
│  │ - Workflow      │                     │ 3. Kanban updates       │    │
│  │   unchanged     │                     └─────────────────────────┘    │
│  └─────────────────┘                                                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 3. Workflow Selection with Active Methodology

When a methodology is active and user selects a different workflow (not owned by the methodology):

```
┌─────────────────────────────────────────────────────────────────────────┐
│             USER SELECTS "JIRA COMPATIBLE" WHILE BMAD IS ACTIVE          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Current state:                                                          │
│  - Active methodology: BMAD                                              │
│  - Active workflow: BMAD Workflow                                        │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    CONFIRMATION DIALOG                            │   │
│  │                                                                    │   │
│  │  ⚠️ Selecting "Jira Compatible" will deactivate the               │   │
│  │     "BMAD Method" methodology.                                    │   │
│  │                                                                    │   │
│  │  The BMAD methodology requires its own workflow layout.           │   │
│  │  Switching to a different workflow will turn off BMAD features.   │   │
│  │                                                                    │   │
│  │  Continue?                                                         │   │
│  │                                                                    │   │
│  │                              [Cancel]  [Switch Anyway]             │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────┐                     ┌─────────────────────────┐    │
│  │ User clicks     │                     │ User clicks             │    │
│  │ [Cancel]        │                     │ [Switch Anyway]         │    │
│  └────────┬────────┘                     └────────────┬────────────┘    │
│           │                                           │                  │
│           ▼                                           ▼                  │
│  ┌─────────────────┐                     ┌─────────────────────────┐    │
│  │ NO CHANGES      │                     │ 1. Deactivate BMAD      │    │
│  │ - BMAD stays    │                     │ 2. Switch to Jira       │    │
│  │   active        │                     │    Compatible workflow  │    │
│  │ - BMAD workflow │                     │ 3. Kanban updates       │    │
│  │   remains       │                     └─────────────────────────┘    │
│  └─────────────────┘                                                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Implementation Details

#### Additional State Needed

```typescript
// src/stores/methodologyStore.ts (or similar)
interface MethodologyState {
  activeMethodology: MethodologyExtension | null;
  methodologies: MethodologyExtension[];
}

// Check if workflow belongs to active methodology
function isMethodologyWorkflow(workflowId: string, methodology: MethodologyExtension | null): boolean {
  return methodology?.workflow?.id === workflowId;
}
```

#### Updated WorkflowSelector Logic

```typescript
// src/components/workflows/WorkflowSelector.tsx

const handleWorkflowSelect = async (workflowId: string) => {
  const activeMethodology = useMethodologyStore.getState().activeMethodology;

  // If no methodology active, just switch
  if (!activeMethodology) {
    setSelectedWorkflowId(workflowId);
    return;
  }

  // If selecting the methodology's own workflow, just switch
  if (activeMethodology.workflow?.id === workflowId) {
    setSelectedWorkflowId(workflowId);
    return;
  }

  // Otherwise, warn about deactivating methodology
  const confirmed = await showDeactivateMethodologyConfirmation(activeMethodology.name);

  if (confirmed) {
    // Deactivate methodology first
    await deactivateMethodology.mutateAsync(activeMethodology.id);
    // Then switch workflow
    setSelectedWorkflowId(workflowId);
  }
  // If not confirmed, do nothing (workflow stays the same)
};
```

#### Updated Methodology Activation Logic

```typescript
// src/components/ExtensibilityView.tsx - MethodologiesPanel

const handleActivateMethodology = async (methodology: MethodologyExtension) => {
  const currentWorkflow = getCurrentActiveWorkflow(); // from store or query

  // Show confirmation about workflow change
  const confirmed = await showActivateMethodologyConfirmation({
    methodologyName: methodology.name,
    currentWorkflowName: currentWorkflow?.name ?? "Default",
    currentWorkflowColumns: currentWorkflow?.columns.length ?? 0,
    newWorkflowName: methodology.workflow?.name ?? "Unknown",
    newWorkflowColumns: methodology.workflow?.columns.length ?? 0,
  });

  if (confirmed) {
    // Activate methodology (backend should also set its workflow as active)
    await activateMethodology.mutateAsync(methodology.id);
    // Optionally refresh workflow state
    queryClient.invalidateQueries({ queryKey: ["workflows"] });
  }
  // If not confirmed, methodology stays inactive
};
```

### Files to Modify (Additional)

| File | Changes |
|------|---------|
| `src/components/ExtensibilityView.tsx` | Add confirmation dialogs to MethodologiesPanel activation |
| `src/components/workflows/WorkflowSelector.tsx` | Add methodology-aware selection with confirmation |
| `src/stores/methodologyStore.ts` | Ensure active methodology state is accessible |
| `src/components/ui/confirmation-dialog.tsx` | Create reusable confirmation dialog (if not exists) |

### Verification Checklist (Additional)

#### Methodology-Workflow Coordination
- [ ] Workflows tab shows methodology-embedded workflows
- [ ] Methodology workflows marked with "Part of X Method" indicator
- [ ] Activating methodology shows confirmation about workflow change
- [ ] Rejecting methodology confirmation cancels both actions
- [ ] Selecting non-methodology workflow shows deactivation warning
- [ ] Rejecting workflow switch keeps methodology and workflow unchanged
- [ ] Accepting workflow switch deactivates methodology and switches workflow
- [ ] Kanban view updates correctly after methodology activation
- [ ] Kanban view updates correctly after methodology deactivation via workflow switch
