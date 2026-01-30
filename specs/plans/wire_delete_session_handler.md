# Plan: Wire Up Delete Session Handler in IdeationView

## Summary

Fix the non-working delete button in the session sidebar by wiring up the existing `onDeleteSession` prop through the component hierarchy, and create a reusable confirmation dialog hook.

## Root Cause

The delete flow has all pieces implemented but not connected:
- ✅ `SessionBrowser` has `onDeleteSession` prop and calls it on menu click
- ❌ `IdeationViewProps` missing `onDeleteSession`
- ❌ `IdeationView` doesn't pass prop to `SessionBrowser`
- ❌ `App.tsx` doesn't create handler or pass to `IdeationView`
- ✅ `useDeleteIdeationSession` hook exists and works
- ✅ `ideationApi.sessions.delete` exists and works

## Implementation Steps

### Step 1: Create useConfirmation hook (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(hooks): create useConfirmation hook for reusable dialogs`

**File:** `src/hooks/useConfirmation.tsx` (new file)

Create a reusable confirmation dialog hook:
- Manages internal state with `useState`
- Exposes `confirm(options): Promise<boolean>` that opens dialog and resolves when user decides
- Returns `ConfirmationDialog` component to render
- Supports `destructive` variant for red confirm button
- Uses existing AlertDialog primitives from `src/components/ui/alert-dialog.tsx`

```typescript
interface ConfirmOptions {
  title: string;
  description: string;
  confirmText?: string;      // defaults to "Confirm"
  cancelText?: string;       // defaults to "Cancel"
  variant?: 'default' | 'destructive';
}

interface UseConfirmationReturn {
  confirm: (options: ConfirmOptions) => Promise<boolean>;
  ConfirmationDialog: React.FC;
}
```

The hook pattern:
1. `confirm()` stores options in ref, sets open=true, returns new Promise
2. When user clicks confirm/cancel, resolve the promise and close
3. `ConfirmationDialog` renders AlertDialog with stored options

### Step 2: Add onDeleteSession to IdeationViewProps (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(ideation): add onDeleteSession prop to IdeationView`

**File:** `src/components/Ideation/IdeationView.tsx`

**Line 54** - Add to interface after `onArchiveSession`:
```typescript
onArchiveSession: (sessionId: string) => void;
onDeleteSession?: (sessionId: string) => void;  // ADD THIS
```

**Line 84** - Add to destructured props after `onArchiveSession`:
```typescript
onArchiveSession,
onDeleteSession,  // ADD THIS
```

### Step 3: Pass onDeleteSession to SessionBrowser
**Dependencies:** Step 2
**Atomic Commit:** (combined with Step 2)

**File:** `src/components/Ideation/IdeationView.tsx`

**Lines 235-240** - Update SessionBrowser rendering:
```tsx
<SessionBrowser
  sessions={activeSessions}
  currentSessionId={session?.id ?? null}
  onSelectSession={onSelectSession}
  onNewSession={onNewSession}
  onDeleteSession={onDeleteSession}  // ADD THIS
/>
```

### Step 4: Wire up in App.tsx
**Dependencies:** Step 1, Step 2, Step 3
**Atomic Commit:** `feat(ideation): wire delete session handler in App`

**File:** `src/App.tsx`

**1. Import** (near line 45 with other useIdeation imports):
```typescript
import { useDeleteIdeationSession } from "@/hooks/useIdeation";
import { useConfirmation } from "@/hooks/useConfirmation";
```

**2. Initialize** (near line 156 after archiveSession):
```typescript
const deleteSession = useDeleteIdeationSession();
const { confirm, ConfirmationDialog } = useConfirmation();
```

**3. Create handler** (after line 304, after handleArchiveSession):
```typescript
const handleDeleteSession = useCallback(async (sessionId: string) => {
  const sessionToDelete = allSessions.find(s => s.id === sessionId);

  const confirmed = await confirm({
    title: "Delete session?",
    description: `This will permanently delete "${sessionToDelete?.title || 'this session'}" and all its messages. This action cannot be undone.`,
    confirmText: "Delete",
    variant: "destructive",
  });

  if (!confirmed) return;

  try {
    await deleteSession.mutateAsync(sessionId);
    if (activeSession?.id === sessionId) {
      setActiveSession(null);
    }
    toast.success("Session deleted");
  } catch {
    toast.error("Failed to delete session");
  }
}, [deleteSession, confirm, allSessions, activeSession, setActiveSession]);
```

**4. Pass to IdeationView** (line 634, after onArchiveSession):
```tsx
onArchiveSession={handleArchiveSession}
onDeleteSession={handleDeleteSession}  // ADD THIS
```

**5. Render ConfirmationDialog** (at end of App, before closing fragment):
```tsx
<ConfirmationDialog />
```

## Files Modified

| File | Change |
|------|--------|
| `src/hooks/useConfirmation.tsx` | New file (~60 LOC) |
| `src/components/Ideation/IdeationView.tsx` | Add prop to interface + destructuring + pass to SessionBrowser |
| `src/App.tsx` | Import hook, create handler, pass prop, render dialog |

## Verification

1. **Build:** `npm run typecheck` - no type errors
2. **Lint:** `npm run lint` - no lint errors
3. **Manual test:**
   - Navigate to Ideation view
   - Open session dropdown menu → click "Delete"
   - Verify confirmation dialog shows session name
   - Cancel → dialog closes, session remains
   - Confirm → session removed from list
4. **Edge case:**
   - Delete the active session
   - Verify UI shows start panel (cleared selection)
5. **Error handling:**
   - Verify error toast on failure

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
