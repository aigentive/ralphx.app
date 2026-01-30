# Plan: Fix Session Browser Selection Flash Bug

## Problem Statement
When clicking on a session in the ideation session browser, the **topmost (most recently updated) session** briefly appears selected before the clicked session becomes selected.

## Root Cause Analysis

The bug stems from a **dual-state source race condition** combined with **non-atomic Zustand updates**.

### Key Files
- `src/App.tsx:341-348` — `handleSelectSession` handler
- `src/App.tsx:664` — Session prop with dual-state fallback
- `src/stores/ideationStore.ts:102-109` — `setActiveSession` action
- `src/stores/ideationStore.ts:111-129` — `addSession` action

### The Race Condition

**In `App.tsx:664`:**
```tsx
session={sessionData?.session ?? activeSession}
```

This creates two sources of truth:
1. `sessionData?.session` — TanStack Query (async, cached)
2. `activeSession` — Zustand store (sync)

**In `handleSelectSession` (App.tsx:341-348):**
```tsx
const handleSelectSession = useCallback((sessionId: string) => {
  const session = allSessions.find((s) => s.id === sessionId);
  if (session) {
    addSession(session);      // Zustand set() #1
    setActiveSession(sessionId);  // Zustand set() #2
  }
}, [allSessions, addSession, setActiveSession]);
```

Two separate `set()` calls can trigger **two React renders**:
1. **Render 1:** `sessions` updated, but `activeSessionId` still old/null
2. **Render 2:** `activeSessionId` updated

During Render 1, if TanStack Query has cached data for a different session (could be the first/most-recently-accessed session), that cached data briefly shows as selected.

## Solution: Atomic State Update

Combine both updates into a single Zustand action to ensure atomic state changes.

## Implementation Tasks

### Task 1: Add `selectSession` action to ideationStore (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(stores): add atomic selectSession action`

**Files:** `src/stores/ideationStore.ts`

Add to `IdeationActions` interface:
```typescript
/** Select a session atomically (adds to store + sets active in one update) */
selectSession: (session: IdeationSession) => void;
```

Add implementation:
```typescript
selectSession: (session) =>
  set((state) => {
    // Add session to store
    state.sessions[session.id] = session;

    // Set as active
    state.activeSessionId = session.id;

    // Clear session-specific state
    state.planArtifact = null;
    state.syncNotification = null;
    state.error = null;

    // LRU eviction if needed
    const sessionIds = Object.keys(state.sessions);
    if (sessionIds.length > MAX_CACHED_SESSIONS) {
      const oldest = sessionIds
        .filter((id) => id !== session.id)
        .sort((a, b) => {
          const aTime = new Date(state.sessions[a]?.updatedAt ?? 0).getTime();
          const bTime = new Date(state.sessions[b]?.updatedAt ?? 0).getTime();
          return aTime - bTime;
        })[0];
      if (oldest) {
        delete state.sessions[oldest];
      }
    }
  }),
```

### Task 2: Update handler in App.tsx
**Dependencies:** Task 1
**Atomic Commit:** `fix(ideation): use atomic selectSession for flash-free selection`

**Files:** `src/App.tsx`

Replace:
```typescript
const addSession = useIdeationStore((s) => s.addSession);
const setActiveSession = useIdeationStore((s) => s.setActiveSession);

const handleSelectSession = useCallback((sessionId: string) => {
  const session = allSessions.find((s) => s.id === sessionId);
  if (session) {
    addSession(session);
    setActiveSession(sessionId);
  }
}, [allSessions, addSession, setActiveSession]);
```

With:
```typescript
const selectSession = useIdeationStore((s) => s.selectSession);

const handleSelectSession = useCallback((sessionId: string) => {
  const session = allSessions.find((s) => s.id === sessionId);
  if (session) {
    selectSession(session);  // Single atomic update
  }
}, [allSessions, selectSession]);
```

**Note:** Keep `addSession` and `setActiveSession` imports if they're used elsewhere in the file.

## Files to Modify

| File | Change |
|------|--------|
| `src/stores/ideationStore.ts` | Add `selectSession` action + type |
| `src/App.tsx` | Replace `addSession` + `setActiveSession` with `selectSession` |

## Verification

1. Click through multiple sessions rapidly — no flash
2. Click on a session that was never visited — correct session shows immediately
3. Click on the second session in list — first session does NOT flash
4. Ensure proposals and messages load correctly for selected session

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
