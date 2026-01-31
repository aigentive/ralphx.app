# Plan: Fix Chat Context Bleeding on Rapid Context Switches

## Problem Statement

When switching between chat contexts (task → ideation → project → review) while a message is streaming:
1. The stop button from the old conversation appears in the new context
2. Streaming tool call bubbles from the old conversation appear in the new context
3. Messages may not be received properly in the new context

## Root Cause Analysis

There's a **race condition** between context cleanup effects and asynchronous event listeners:

1. User switches context (e.g., from task A to ideation session B)
2. React schedules effects: `useChatPanelContext` cleanup + `useIntegratedChatEvents` ref update
3. **Before effects run**, Tauri events from the OLD conversation arrive
4. Event listeners check `conversation_id === activeConversationIdRef.current`
5. The ref still has the **OLD value** → filter passes → tool calls added to UI
6. Effects finally run → `setStreamingToolCalls([])` clears, BUT new events may have already populated it!

**Key Files Affected:**
- `src/hooks/useIntegratedChatEvents.ts` - Event listeners for IntegratedChatPanel
- `src/hooks/useChatPanelHandlers.ts` - Event listeners for ChatPanel/TaskChatPanel (same pattern)

**The Bug Pattern:**
```typescript
// In useIntegratedChatEvents.ts
useEffect(() => {
  activeConversationIdRef.current = activeConversationId;
}, [activeConversationId]);

useEffect(() => {
  // Event listeners use activeConversationIdRef.current for filtering
  // BUT this ref may have stale value during context transitions
}, [queryClient, messagesEndRef, setStreamingToolCalls]); // ← activeConversationId NOT in deps!
```

## Solution

**Add `activeConversationId` to the event listener effect's dependency array.** This causes:
1. Old listeners to be unsubscribed (cleanup function runs)
2. New listeners to be created with correct closure/ref values
3. Any in-flight events to be ignored (old listeners gone)

The performance overhead is minimal since conversation changes are user-initiated and infrequent.

## Implementation

### Step 1: Fix `useIntegratedChatEvents.ts` (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(hooks): add activeConversationId to event listener deps in useIntegratedChatEvents`

Add `activeConversationId` to the event listener effect dependency array and clear `streamingToolCalls` in the cleanup function:

```typescript
// Before:
}, [queryClient, messagesEndRef, setStreamingToolCalls]);

// After:
}, [queryClient, messagesEndRef, setStreamingToolCalls, activeConversationId]);
```

Also add cleanup to clear streaming state when re-subscribing:
```typescript
return () => {
  unlisteners.forEach((unlisten) => unlisten());
  setStreamingToolCalls([]); // Clear on cleanup
};
```

Files: `src/hooks/useIntegratedChatEvents.ts`

### Step 2: Fix `useChatPanelHandlers.ts` (Same Pattern)
**Dependencies:** None
**Atomic Commit:** `fix(hooks): add activeConversationId to event listener deps in useChatPanelHandlers`

Add `activeConversationId` to the dependency array and clear streaming state in cleanup:

```typescript
// Before:
}, [queryClient, logError, messagesEndRef]);

// After:
}, [queryClient, logError, messagesEndRef, activeConversationId]);
```

Add cleanup:
```typescript
return () => {
  unlisteners.forEach((unlisten) => unlisten());
  setStreamingToolCalls([]); // Clear on cleanup
};
```

Note: `useChatPanelHandlers` already receives `activeConversationId` as a prop, so we just need to add it to deps.

Files: `src/hooks/useChatPanelHandlers.ts`

## Files to Modify

1. **`src/hooks/useIntegratedChatEvents.ts`**
   - Add `activeConversationId` to event listener effect deps (line 140)
   - Add `setStreamingToolCalls([])` to cleanup function (before line 138)

2. **`src/hooks/useChatPanelHandlers.ts`**
   - Add `activeConversationId` to event listener effect deps (line 366)
   - Add `setStreamingToolCalls([])` to cleanup function (before line 364)

## Verification

1. **Manual Testing:**
   - Open task A with an active agent streaming
   - Quickly switch to ideation session
   - Verify: No stop button appears, no tool call bubbles appear
   - Verify: New context loads correctly

2. **Test Scenarios:**
   - Task → Ideation (while agent streaming)
   - Ideation → Task (while agent streaming)
   - Task (execution mode) → Task (regular chat) (same task)
   - Task → Project chat (while agent streaming)

3. **Check for regressions:**
   - Tool calls still appear correctly in the active conversation
   - Stop button still works
   - Messages are still received properly
   - Run completion still clears streaming state

## Risks and Mitigations

**Risk:** Re-subscribing to events on every conversation change might cause brief gaps in event reception.

**Mitigation:** The cleanup → re-subscribe cycle is synchronous within React's effect phase. Tauri events are buffered and won't be lost during this brief window.

**Risk:** Additional `setStreamingToolCalls([])` calls might cause flicker.

**Mitigation:** The clear only happens when conversation actually changes, which is already triggering a re-render anyway. The clear prevents stale data, not causes new renders.

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
