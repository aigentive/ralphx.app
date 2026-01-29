# RalphX - Phase 25: Ideation UI Improvements

## Overview

This phase adds two key features to improve the ideation workflow. First, users can start ideation sessions seeded with existing draft tasks - allowing exploration of tasks that need more fleshing out before execution. Second, users can drag-and-drop markdown files into the proposals panel to import external documentation as plan artifacts.

**Reference Plan:**
- `specs/plans/ideation-ui-improvements.md` - Detailed implementation plan with data model changes, entry points, and visual design specifications

## Goals

1. Enable starting ideation sessions from draft tasks via multiple entry points (context menu, task detail, ideation panel)
2. Support drag-and-drop markdown file import into the proposals panel
3. Provide visual feedback during drag operations consistent with RalphX design system

## Dependencies

### Phase 24 (Tmux-Based Multi-Stream Orchestration) - Completed

| Dependency | Why Needed |
|------------|------------|
| Ideation system (Phase 10, 16) | Session creation, plan artifacts |
| Task CRUD (Phase 18) | TaskCardContextMenu, TaskDetailOverlay |
| Design system (Phase 13, 14) | shadcn/ui components, accent color |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/ideation-ui-improvements.md`
2. Understand the data model changes and entry point architecture
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run `npm run lint && npm run typecheck` and `cargo clippy --all-targets --all-features -- -D warnings`
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/ideation-ui-improvements.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "frontend",
    "description": "Extend IdeationSession type with seedTaskId field",
    "plan_section": "Feature 1: Data Model Changes",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Data Model Changes'",
      "Edit src/types/ideation.ts:",
      "  - Add seedTaskId?: string to IdeationSessionSchema",
      "  - Add seedTaskId to CreateSessionInput type",
      "Run npm run typecheck",
      "Commit: feat(ideation): add seedTaskId to session schema"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Update ideation API to pass seed_task_id to backend",
    "plan_section": "Feature 1: Data Model Changes",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Data Model Changes'",
      "Edit src/api/ideation.ts:",
      "  - Update sessions.create to accept seedTaskId parameter",
      "  - Pass seed_task_id to invoke call",
      "Edit src/hooks/useIdeation.ts:",
      "  - Update CreateSessionInput type if needed",
      "  - Pass seedTaskId through to API",
      "Run npm run typecheck",
      "Commit: feat(ideation): pass seedTaskId through API layer"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Update create_ideation_session command to accept seed_task_id",
    "plan_section": "Feature 1: Data Model Changes",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Data Model Changes'",
      "Edit src-tauri/src/commands/ideation_commands.rs:",
      "  - Add seed_task_id: Option<String> parameter to create_ideation_session",
      "  - Store in session metadata or inject as system message context",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Run cargo test",
      "Commit: feat(ideation): accept seed_task_id in create_ideation_session"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add 'Start Ideation' menu item to TaskCardContextMenu",
    "plan_section": "Feature 1: Entry Points",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Entry Points'",
      "Edit src/components/tasks/TaskCardContextMenu.tsx:",
      "  - Add 'Start Ideation' menu item with Lightbulb icon",
      "  - Only show for draft status tasks",
      "  - On click: navigate to /ideation with seedTaskId in state",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add Start Ideation to context menu"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add 'Start Ideation' button to TaskDetailOverlay for draft tasks",
    "plan_section": "Feature 1: Entry Points",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Entry Points'",
      "Edit src/components/tasks/TaskDetailOverlay.tsx:",
      "  - Add 'Start Ideation' button in header (before Edit button)",
      "  - Only show for draft status tasks",
      "  - Use Lightbulb icon consistent with context menu",
      "  - On click: same navigation as context menu",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add Start Ideation button to task detail"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create TaskPickerDialog component for selecting draft tasks",
    "plan_section": "Feature 1: New Components",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'New Components'",
      "Create src/components/Ideation/TaskPickerDialog.tsx:",
      "  - Use shadcn/ui Dialog component",
      "  - Fetch and display list of draft tasks",
      "  - Allow search/filter by task title",
      "  - On select: return selected task, close dialog",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): create TaskPickerDialog component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add 'Seed from Draft Task' link to StartSessionPanel",
    "plan_section": "Feature 1: Entry Points",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Entry Points'",
      "Edit src/components/Ideation/StartSessionPanel.tsx:",
      "  - Add 'Seed from Draft Task' link below main CTA",
      "  - On click: open TaskPickerDialog",
      "  - On task selected: create session with seedTaskId, title from task",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add seed from draft task option"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create useFileDrop hook for reusable drag-and-drop",
    "plan_section": "Feature 2: New Components",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Feature 2'",
      "Create src/hooks/useFileDrop.ts:",
      "  - Track isDragging state",
      "  - Handle dragenter, dragover, dragleave, drop events",
      "  - Validate file type (.md) and size (<1MB)",
      "  - Return { isDragging, dropProps, error }",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): create useFileDrop hook"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create DropZoneOverlay component for visual feedback",
    "plan_section": "Feature 2: New Components",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Visual Design'",
      "Create src/components/Ideation/DropZoneOverlay.tsx:",
      "  - Pulsing orange (#ff6b35) border animation",
      "  - Dimmed background overlay",
      "  - Centered 'Drop to import plan' message with icon",
      "  - Only visible when isDragging=true",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): create DropZoneOverlay component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Integrate drag-and-drop into IdeationView proposals panel",
    "plan_section": "Feature 2: Files to Modify",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Feature 2'",
      "Edit src/components/Ideation/IdeationView.tsx:",
      "  - Use useFileDrop hook on proposals panel container",
      "  - Render DropZoneOverlay when dragging",
      "  - On drop: call create_plan_artifact API with file contents",
      "  - Show success/error toast",
      "Extract handleFileImport to useIdeationHandlers.ts if needed",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add drag-and-drop to proposals panel"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Enhance ProposalsEmptyState with drop hint",
    "plan_section": "Feature 2: Visual Design - Empty State Enhancement",
    "steps": [
      "Read specs/plans/ideation-ui-improvements.md section 'Empty State Enhancement'",
      "Edit or create src/components/Ideation/ProposalsEmptyState.tsx:",
      "  - Add divider with 'or' text",
      "  - Add 'Drag a markdown file here to import a plan' hint",
      "  - Style consistent with existing empty states",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add drop hint to proposals empty state"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Multiple entry points for seed task** | Different user workflows: from Kanban (task needs exploration) vs from Ideation (pull in existing drafts) |
| **Proposals panel as drop zone** | Large target, works in both empty and populated states, consistent with macOS patterns |
| **Reusable useFileDrop hook** | Can be extended for other drag-and-drop scenarios in future |
| **seedTaskId as optional field** | Backwards compatible, doesn't affect existing sessions |
| **File validation client-side** | Fast feedback, prevent invalid files from reaching backend |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] useFileDrop hook handles drag events correctly
- [ ] TaskPickerDialog filters draft tasks

### Build Verification
- [ ] `npm run lint` passes
- [ ] `npm run typecheck` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `npm run build` succeeds
- [ ] `cargo build --release` succeeds

### Integration Testing

**Feature 1 - Start Ideation from Draft Task:**
- [ ] Right-click draft task → "Start Ideation" visible in context menu
- [ ] Click → navigates to Ideation view with new session
- [ ] Session title includes task name
- [ ] Open draft task detail → "Start Ideation" button visible
- [ ] Click → same behavior as context menu
- [ ] In StartSessionPanel → "Seed from Draft Task" link visible
- [ ] Click → TaskPickerDialog opens with draft tasks listed
- [ ] Select task → session created with context

**Feature 2 - Drag-and-Drop Markdown:**
- [ ] Drag .md file over proposals panel → orange border pulses
- [ ] Drop valid .md file → file imported as plan artifact
- [ ] Drop invalid file → error toast displayed
- [ ] Empty proposals state → drop hint text visible

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (click handler, route, event listener)
- [ ] New component is imported AND rendered (not behind disabled flag)
- [ ] API wrappers call backend commands
- [ ] State changes reflect in UI

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
