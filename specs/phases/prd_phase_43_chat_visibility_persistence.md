# RalphX - Phase 43: Per-Page Chat Visibility Persistence

## Overview

Currently, chat panel visibility is managed inconsistently across views: Kanban uses `chatCollapsed` in uiStore while other views use `isOpen` in chatStore. Neither persists to localStorage, so users lose their preferred chat state on refresh or when switching views.

This phase unifies chat visibility into a single `Record<ViewType, boolean>` in uiStore with localStorage persistence, allowing each view to remember its own chat collapsed/expanded state independently.

**Reference Plan:**
- `specs/plans/per_page_chat_visibility_persistence.md` - Detailed implementation plan with code snippets and file locations

## Goals

1. Unify chat visibility state into a single `chatVisibleByView: Record<ViewType, boolean>` in uiStore
2. Persist visibility state to localStorage so it survives app refresh
3. Allow each view to maintain independent chat visibility (e.g., Kanban open, Settings closed)
4. Remove deprecated dual-state architecture (`chatCollapsed` in uiStore, `isOpen` in chatStore)

## Dependencies

### Phase 41 (Streaming Tool Call Deduplication) - Required

| Dependency | Why Needed |
|------------|------------|
| Stable chat panel | Chat visibility changes should not interfere with streaming tool display |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/per_page_chat_visibility_persistence.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol:**

Reference: `.claude/rules/commit-lock.md`

1. Establish project root: `PROJECT_ROOT="$(git rev-parse --show-toplevel)"`
2. Acquire lock before `git add` (see commit-lock.md § Protocol)
3. Stage and commit using `git -C "$PROJECT_ROOT"`
4. Release lock after commit: `rm -f "$PROJECT_ROOT/.commit-lock"`

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/per_page_chat_visibility_persistence.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Add chatVisibleByView state with localStorage persistence to uiStore",
    "plan_section": "1. `src/stores/uiStore.ts` (BLOCKING)",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "feat(stores): add per-view chat visibility with localStorage persistence",
    "steps": [
      "Read specs/plans/per_page_chat_visibility_persistence.md section '1. src/stores/uiStore.ts'",
      "Add CHAT_VISIBILITY_KEY constant and DEFAULT_CHAT_VISIBILITY object at top of file",
      "Add loadChatVisibility() and saveChatVisibility() helper functions",
      "Add chatVisibleByView: Record<ViewType, boolean> to UiState interface",
      "Add setChatVisible and toggleChatVisible to UiActions interface",
      "Initialize chatVisibleByView with loadChatVisibility() in store",
      "Implement setChatVisible and toggleChatVisible actions with saveChatVisibility call",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(stores): add per-view chat visibility with localStorage persistence"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Update App.tsx to use unified per-view chat visibility",
    "plan_section": "2. `src/App.tsx`",
    "blocking": [4, 5],
    "blockedBy": [1],
    "atomic_commit": "feat(app): use unified per-view chat visibility",
    "steps": [
      "Read specs/plans/per_page_chat_visibility_persistence.md section '2. src/App.tsx'",
      "Replace chatIsOpen/toggleChatPanel from chatStore with chatVisibleByView/toggleChatVisible from uiStore",
      "Update isExpanded logic to use chatVisibleByView[currentView]",
      "Update handleToggle to use () => toggleChatVisible(currentView)",
      "Remove chatCollapsed and toggleChatCollapsed references",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(app): use unified per-view chat visibility"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Update useAppKeyboardShortcuts hook for unified chat visibility",
    "plan_section": "3. `src/hooks/useAppKeyboardShortcuts.ts`",
    "blocking": [4, 5],
    "blockedBy": [1],
    "atomic_commit": "feat(hooks): update keyboard shortcuts for unified chat visibility",
    "steps": [
      "Read specs/plans/per_page_chat_visibility_persistence.md section '3. src/hooks/useAppKeyboardShortcuts.ts'",
      "Update AppKeyboardShortcutsProps interface to use toggleChatVisible and chatVisibleByView",
      "Remove toggleChatPanel and toggleChatCollapsed from interface",
      "Update ⌘K handler to call toggleChatVisible(currentView)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): update keyboard shortcuts for unified chat visibility"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Remove deprecated isOpen/togglePanel from chatStore",
    "plan_section": "4. `src/stores/chatStore.ts`",
    "blocking": [],
    "blockedBy": [2, 3],
    "atomic_commit": "refactor(stores): remove deprecated chat visibility state from chatStore",
    "steps": [
      "Read specs/plans/per_page_chat_visibility_persistence.md section '4. src/stores/chatStore.ts'",
      "Remove isOpen: boolean from ChatState interface",
      "Remove togglePanel() and setOpen() from ChatActions interface",
      "Remove isOpen initialization from store",
      "Remove togglePanel and setOpen implementations",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(stores): remove deprecated chat visibility state from chatStore"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Remove deprecated chatCollapsed/toggleChatCollapsed from uiStore",
    "plan_section": "5. `src/stores/uiStore.ts` (cleanup)",
    "blocking": [],
    "blockedBy": [2, 3],
    "atomic_commit": "refactor(stores): remove deprecated chatCollapsed state from uiStore",
    "steps": [
      "Read specs/plans/per_page_chat_visibility_persistence.md section '5. src/stores/uiStore.ts (cleanup)'",
      "Remove chatCollapsed: boolean from UiState interface",
      "Remove toggleChatCollapsed() from UiActions interface",
      "Remove chatCollapsed initialization from store",
      "Remove toggleChatCollapsed implementation",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(stores): remove deprecated chatCollapsed state from uiStore"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Single source of truth in uiStore** | Avoid split-brain between uiStore.chatCollapsed and chatStore.isOpen |
| **Record<ViewType, boolean> structure** | Naturally maps view → visibility, O(1) lookup |
| **localStorage persistence** | Survives page refresh, standard browser storage |
| **Default visibility per view type** | Kanban/Ideation default open (integrated chat), others default closed |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] chatVisibleByView correctly initializes from localStorage
- [ ] toggleChatVisible correctly toggles and persists state
- [ ] setChatVisible correctly sets and persists state

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] **Per-view persistence**: Toggle chat on Kanban → go to Settings → return to Kanban → chat should still be in same state
- [ ] **Cross-view independence**: Close chat on Kanban → go to Activity → open chat → return to Kanban → chat should be closed
- [ ] **App refresh**: Toggle states → refresh app → states restored from localStorage
- [ ] **Keyboard shortcut**: ⌘K works on all views (except Ideation)
- [ ] **Button toggle**: Header chat button works on all views

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (header chat button, ⌘K keyboard shortcut)
- [ ] toggleChatVisible is called with correct view parameter
- [ ] chatVisibleByView state updates trigger re-render
- [ ] localStorage is updated on every state change

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
