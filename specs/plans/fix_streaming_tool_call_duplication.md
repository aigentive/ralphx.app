# Plan: Fix Streaming Tool Call Duplication Bug

## Root Cause Analysis

**The bug:** Same tool call appears multiple times in the streaming tooltip during live agent execution.

**Why it happens:**

1. **Backend emits multiple events per tool call:** The backend emits `agent:tool_call` for 3 lifecycle stages:
   - `ToolCallStarted` - with `tool_id`, but arguments are null
   - `ToolCallCompleted` - with `tool_id` AND full arguments
   - `ToolResultReceived` - as `result:{tool_use_id}` with the result

2. **Frontend ignores backend `tool_id`:** The frontend event listener doesn't extract `tool_id` from the payload. Instead it generates a weak client-side ID: `streaming-${Date.now()}-${prev.length}`

3. **No deduplication:** Each event blindly appends to the array, causing the same tool call to appear 2-3 times.

**Evidence from code:**

Backend payload (`chat_service_types.rs:86-93`):
```rust
pub struct AgentToolCallPayload {
    pub tool_name: String,
    pub tool_id: Option<String>,  // <-- EXISTS but frontend ignores it
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    ...
}
```

Frontend listener (`useChatPanelHandlers.ts:214-233`):
```typescript
const toolCallUnlisten = await listen<{
  tool_name: string;
  arguments: unknown;
  result: unknown;
  conversation_id: string;
  // Missing: tool_id ❌
}>("agent:tool_call", (event) => {
  setStreamingToolCalls((prev) => [
    ...prev,
    {
      id: `streaming-${Date.now()}-${prev.length}`, // Weak ID ❌
      ...
    }
  ]);
});
```

## Implementation Plan

### Task 1: Update event listener to use backend `tool_id` (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(chat): use backend tool_id for streaming deduplication`

**File:** `src/hooks/useChatPanelHandlers.ts`

Changes:
1. Add `tool_id` to the TypeScript event payload interface
2. Use `tool_id` as the unique identifier (fall back to timestamp if null)
3. Use `Map`-based state update to deduplicate - update existing entry if tool_id exists, append only if new

**Logic:**
```typescript
setStreamingToolCalls((prev) => {
  const toolId = event.payload.tool_id ?? `streaming-${Date.now()}`;
  const existingIndex = prev.findIndex(tc => tc.id === toolId);

  if (existingIndex >= 0) {
    // Update existing entry (started → completed → result)
    const updated = [...prev];
    updated[existingIndex] = { ...updated[existingIndex], ...newData };
    return updated;
  }

  // New tool call - append
  return [...prev, { id: toolId, ...newData }];
});
```

### Task 2: Filter out result events in the listener
**Dependencies:** Task 1
**Atomic Commit:** `fix(chat): filter result events early in tool call listener`

**File:** `src/hooks/useChatPanelHandlers.ts`

The `result:toolu*` events are already filtered in rendering (`StreamingToolIndicator.tsx:204`), but we can filter them earlier in the listener to avoid unnecessary state updates.

```typescript
// Skip result events - they don't add new tool calls
if (tool_name.startsWith("result:toolu")) {
  return;
}
```

### Task 3: Update ToolCall type if needed
**Dependencies:** None
**Atomic Commit:** `chore(types): ensure ToolCall type supports lifecycle tracking`

**File:** `src/components/Chat/ToolCallIndicator.tsx` (or types file)

Ensure the `ToolCall` type includes all fields we need for proper lifecycle tracking.

## Critical Files

| File | Change |
|------|--------|
| `src/hooks/useChatPanelHandlers.ts:214-239` | Add `tool_id` to interface, implement deduplication |
| `src/components/Chat/StreamingToolIndicator.tsx` | No changes needed (already uses `tc.id` as key) |

## Verification

1. Start the app with dev server running
2. Open chat panel and send a message that triggers tool calls
3. Observe the streaming tooltip - each tool call should appear exactly once
4. After run completes, verify final message shows correct tool count

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
