# Plan: Per-Page Chat Visibility Persistence

## Goal
Persist chat panel visibility state per-page/view so each screen remembers its own chat collapsed/expanded state.

## Current State
- **Kanban view**: Uses `chatCollapsed: boolean` in `uiStore.ts:88`
- **Other views**: Uses `isOpen: boolean` in `chatStore.ts:54` (shared across all views)
- **Persistence**: None for visibility (only panel widths persist to localStorage)

## Implementation Approach

Unify chat visibility into a single `Record<ViewType, boolean>` in uiStore with localStorage persistence.

---

## Files to Modify

### 1. `src/stores/uiStore.ts` (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(stores): add per-view chat visibility with localStorage persistence`

**Add state** (after line 88):
```typescript
/** Chat visibility per view (persisted to localStorage) */
chatVisibleByView: Record<ViewType, boolean>;
```

**Add actions** (after line 159):
```typescript
/** Set chat visibility for a specific view */
setChatVisible: (view: ViewType, visible: boolean) => void;
/** Toggle chat visibility for a specific view */
toggleChatVisible: (view: ViewType) => void;
```

**Initialize with defaults and localStorage** (after line 191):
```typescript
chatVisibleByView: loadChatVisibility(),
```

**Implement actions** (after line 338):
```typescript
setChatVisible: (view, visible) =>
  set((state) => {
    state.chatVisibleByView[view] = visible;
    saveChatVisibility(state.chatVisibleByView);
  }),

toggleChatVisible: (view) =>
  set((state) => {
    state.chatVisibleByView[view] = !state.chatVisibleByView[view];
    saveChatVisibility(state.chatVisibleByView);
  }),
```

**Add helpers** (top of file):
```typescript
const CHAT_VISIBILITY_KEY = "ralphx-chat-visibility-by-view";

const DEFAULT_CHAT_VISIBILITY: Record<ViewType, boolean> = {
  kanban: true,       // visible by default (integrated layout)
  ideation: true,     // always visible (built-in chat)
  extensibility: false,
  activity: false,
  settings: false,
  task_detail: false,
};

function loadChatVisibility(): Record<ViewType, boolean> {
  try {
    const saved = localStorage.getItem(CHAT_VISIBILITY_KEY);
    if (saved) {
      return { ...DEFAULT_CHAT_VISIBILITY, ...JSON.parse(saved) };
    }
  } catch { /* ignore */ }
  return { ...DEFAULT_CHAT_VISIBILITY };
}

function saveChatVisibility(visibility: Record<ViewType, boolean>): void {
  localStorage.setItem(CHAT_VISIBILITY_KEY, JSON.stringify(visibility));
}
```

**Deprecate `chatCollapsed`**: Keep for now but derive from `chatVisibleByView.kanban` (inverted). Or remove and update all usages.

### 2. `src/App.tsx`
**Dependencies:** Task 1 (uiStore.ts)
**Atomic Commit:** `feat(app): use unified per-view chat visibility`

**Replace lines 101-103**:
```typescript
// OLD:
const chatIsOpen = useChatStore((s) => s.isOpen);
const toggleChatPanel = useChatStore((s) => s.togglePanel);

// NEW:
const chatVisibleByView = useUiStore((s) => s.chatVisibleByView);
const toggleChatVisible = useUiStore((s) => s.toggleChatVisible);
```

**Update line 546-547** (chat toggle logic):
```typescript
// OLD:
const isExpanded = currentView === "kanban" ? !chatCollapsed : chatIsOpen;
const handleToggle = currentView === "kanban" ? toggleChatCollapsed : toggleChatPanel;

// NEW:
const isExpanded = chatVisibleByView[currentView];
const handleToggle = () => toggleChatVisible(currentView);
```

### 3. `src/hooks/useAppKeyboardShortcuts.ts`
**Dependencies:** Task 1 (uiStore.ts)
**Atomic Commit:** `feat(hooks): update keyboard shortcuts for unified chat visibility`

**Update interface** (lines 9-20):
```typescript
// Replace toggleChatPanel and toggleChatCollapsed with:
toggleChatVisible: (view: ViewType) => void;
chatVisibleByView: Record<ViewType, boolean>;
```

**Update ⌘K handler** (lines 72-92):
```typescript
case "k":
case "K": {
  if (currentView === "ideation") return;
  const activeElement = document.activeElement;
  if (activeElement instanceof HTMLInputElement || activeElement instanceof HTMLTextAreaElement) return;
  e.preventDefault();
  toggleChatVisible(currentView);
  break;
}
```

### 4. `src/stores/chatStore.ts`
**Dependencies:** Task 2 (App.tsx), Task 3 (useAppKeyboardShortcuts.ts)
**Atomic Commit:** `refactor(stores): remove deprecated chat visibility state`

**Remove**:
- `isOpen: boolean` from state (line 54)
- `togglePanel()` action (line 75)
- `setOpen()` action (line 77)
- Implementations (lines 115, 128-136)

### 5. `src/stores/uiStore.ts` (cleanup)
**Dependencies:** Task 2 (App.tsx), Task 3 (useAppKeyboardShortcuts.ts)
**Atomic Commit:** `refactor(stores): remove deprecated chatCollapsed state`

**Remove**:
- `chatCollapsed: boolean` from state
- `toggleChatCollapsed()` action
- Related implementations

---

## Implementation Steps

1. Add `chatVisibleByView` state, helpers, and actions to `uiStore.ts` **(BLOCKING)**
2. Update `App.tsx` to use new unified state
3. Update `useAppKeyboardShortcuts.ts` to use new actions
4. Remove deprecated `isOpen`/`togglePanel` from `chatStore.ts`
5. Remove deprecated `chatCollapsed`/`toggleChatCollapsed` from `uiStore.ts`
6. Run `npm run lint && npm run typecheck`

---

## Verification

1. **Per-view persistence**: Toggle chat on Kanban → go to Settings → return to Kanban → chat should still be in same state
2. **Cross-view independence**: Close chat on Kanban → go to Activity → open chat → return to Kanban → chat should be closed
3. **App refresh**: Toggle states → refresh app → states restored from localStorage
4. **Keyboard shortcut**: ⌘K works on all views (except Ideation)
5. **Button toggle**: Header chat button works on all views

---

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
