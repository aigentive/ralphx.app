# Fix: Duplicate Tool Call Display in Streaming Indicator

## Context

The live tool call indicator shows each tool call **multiple times** during agent execution. Root cause: the backend emits 2-3 `agent:tool_call` events per tool call (Started + Completed + Result), and `useIntegratedChatEvents.ts` blindly appends every event to the array without deduplication.

The sibling hook `useChatPanelHandlers.ts:226-266` already has the correct upsert-by-tool_id pattern — this is simply a port of that proven fix.

## Event Lifecycle (per tool call)

| # | StreamEvent | Payload | Frontend today |
|---|-------------|---------|----------------|
| 1 | `ToolCallStarted` | `tool_id`, name, args=Null | Appends (shows "Editing file") |
| 2 | `ToolCallCompleted` | `tool_id`, name, args=real | Appends AGAIN (shows "src/Foo.tsx") |
| 3 | `ToolResultReceived` | name="result:toolu_*" | Appends, but filtered by render |

Result: **2 visible entries per tool call** in `StreamingToolIndicator`.

## Fix: 1 file, ~20 lines changed (SINGLE TASK)

**Dependencies:** None
**Atomic Commit:** `fix(chat-streaming): deduplicate tool call events in streaming indicator`

**File:** `src/hooks/useIntegratedChatEvents.ts` (lines 46-68)

Port the dedup pattern from `src/hooks/useChatPanelHandlers.ts:226-266`:

1. **Extract `tool_id`** from the event payload (already sent by backend, currently ignored)
2. **Early-return on `result:toolu*`** events (never useful for the streaming indicator)
3. **Upsert by `tool_id`**: if entry exists with same `tool_id`, update in-place; otherwise append new

### Before (buggy)
```typescript
setStreamingToolCalls((prev) => [
  ...prev,
  {
    id: `streaming-agent-${Date.now()}-${prev.length}`,
    name: tool_name,
    arguments: args,
    result,
  },
]);
```

### After (fixed)
```typescript
// Skip result events early
if (tool_name.startsWith("result:toolu")) return;

const id = tool_id ?? `streaming-agent-${Date.now()}`;

setStreamingToolCalls((prev) => {
  const existing = prev.find((tc) => tc.id === id);
  if (existing) {
    // Update in-place (Started → Completed lifecycle)
    return prev.map((tc) =>
      tc.id === id
        ? { ...tc, name: tool_name, arguments: args ?? tc.arguments, result: result ?? tc.result }
        : tc
    );
  }
  return [...prev, { id, name: tool_name, arguments: args, result }];
});
```

## Phase 91 Compatibility

Phase 91 (Chat Diff View, in progress) adds `diff_context` to the `AgentToolCallPayload`. This fix is forward-compatible — Phase 91 Task 6 can extend the upsert to merge `diffContext` when it wires up the streaming split. No conflicts.

## No Other Files Need Changes

- `StreamingToolIndicator.tsx` — Keep existing `result:toolu*` filter as defense-in-depth (harmless, free safety net)
- Backend — No changes needed (`tool_id` is already emitted)
- `ToolCallIndicator.tsx` — Type unchanged (Phase 91 Task 3 will add `diffContext` separately)

## Verification

1. Trigger agent execution in IntegratedChatPanel (Kanban/Ideation chat)
2. Watch the streaming tool indicator — each tool call should appear exactly once
3. Tool call should show immediately on start (generic text), then update with real args
4. Verify ChatPanel (non-Kanban chat) still works (untouched code)

## Compilation Unit Analysis

**Single task, single file** — no compilation unit concerns:
- No renames or signature changes
- Adding `tool_id?` to the event payload type is additive (optional field)
- `ToolCall` type's `id: string` field is unchanged
- No cross-file dependencies introduced

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
