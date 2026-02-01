# Plan: Link Conversation IDs to Task State History

## Problem

When navigating task history (executing → reviewing → re_executing → reviewing cycles), the UI cannot show the correct conversation for each historical state because:

1. `task_state_history` records transitions but has no `conversation_id`
2. `chat_conversations` links to tasks via `context_type/context_id` but doesn't distinguish between cycles
3. Result: Same conversation list shows for both `executing` and `re_executing` states

## Current Architecture

| Component | Location | Current Behavior |
|-----------|----------|------------------|
| State history | `task_state_history` table | Records transitions with `metadata JSON` column |
| Conversation create | `src/api/chat.ts:242` → `create_agent_conversation` | Creates by context_type + task_id |
| Chat panel context | `src/hooks/useChatPanelContext.ts` | Fetches all conversations for context_type + task_id |
| History navigation | `StateTimelineNav.tsx` | Sets `viewAsStatus` but no conversation selection |

**Gap:** No link between state transition and the conversation created for that transition.

## Proposed Solution

**Store `conversation_id` in `task_state_history.metadata`** when entering states that spawn conversations.

### Complete InternalStatus List (17 States)

| Category | States | Spawns Conversation? |
|----------|--------|---------------------|
| **Idle** | backlog, ready, blocked | ❌ No |
| **Execution** | executing, re_executing | ✅ Yes (task_execution) |
| **QA** | qa_refining, qa_testing, qa_passed, qa_failed | ❌ No (background agents, not conversations) |
| **Review** | pending_review, reviewing, review_passed, escalated, revision_needed | ✅ `reviewing` only |
| **Terminal** | approved, failed, cancelled | ❌ No |

### States That Spawn Conversations

| State | Context Type | Behavior |
|-------|--------------|----------|
| `executing` | task_execution | **NEW** conversation |
| `re_executing` | task_execution | **NEW or REUSE** existing TaskExecution conversation |
| `reviewing` | review | **NEW or REUSE** existing Review conversation |

### The REUSE Problem

`get_or_create_conversation()` reuses an "active" conversation for the same context_type + task_id:
- Multiple `re_executing` cycles may share the same TaskExecution conversation
- Multiple `reviewing` cycles may share the same Review conversation
- **Result:** Same conversation_id appears for multiple state transitions

### Solution: Track agent_run_id

Each state entry creates a **new AgentRun** even when reusing a conversation. The `agent_runs` table has:
- `id` (unique per execution)
- `conversation_id` (may be shared)
- `started_at`, `completed_at`

**Metadata Format:**
```json
{
  "conversation_id": "conv_abc123",
  "agent_run_id": "run_xyz789"
}
```

This allows the UI to:
1. Show the correct conversation
2. Scroll to or filter messages from that specific agent run

No schema migration needed - uses existing `metadata JSON` column.

## Orchestration Flow Analysis

**Current sequence:**

1. `task_transition_service.rs`:
   - State persisted to DB
   - State history recorded via `persist_status_change()` ← **NO conversation_id yet**
   - Entry actions executed via `on_enter()`

2. `chat_service/mod.rs:328-348` (inside `send_message()`):
   - Line 335-338: `get_or_create_conversation()` → `conversation_id`
   - Line 343-348: `AgentRun::new()` + persist → `agent_run_id`
   - **← INTEGRATION POINT: Both IDs available HERE**

**Problem:** Conversation is created AFTER state history is recorded.

**Solution:** After line 348 in `send_message()`, call `task_repo.update_latest_state_history_metadata()` for TaskExecution/Review contexts only.

## Implementation Tasks

### Task 1: Backend - Add method to update state history metadata (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(task-repo): add update_latest_state_history_metadata method`

**Files:**
- `src-tauri/src/domain/repositories/task_repository.rs` (trait)
- `src-tauri/src/infrastructure/sqlite/task_repository.rs` (impl)

**Changes:**
1. Add repository method: `update_latest_state_history_metadata(task_id, metadata: StateHistoryMetadata)`
2. Struct: `StateHistoryMetadata { conversation_id: String, agent_run_id: String }`
3. SQL: UPDATE `task_state_history` SET `metadata` = ? WHERE task_id = ? ORDER BY created_at DESC LIMIT 1

### Task 2: Backend - Capture conversation_id and agent_run_id after creation (BLOCKING)

**Dependencies:** Task 1
**Atomic Commit:** `feat(chat-service): capture conversation and agent_run IDs in state history`

**Files:**
- `src-tauri/src/application/chat_service/mod.rs` (after line 348)

**Changes:**
1. After agent_run is persisted, call the new repository method
2. Only for TaskExecution and Review context types (these are the ones with state history)

**Integration point:** `send_message()` after line 348:
```rust
// Existing code (lines 335-348):
let conversation = self.get_or_create_conversation(context_type, context_id).await?;
let conversation_id = conversation.id;
// ...
let agent_run = AgentRun::new(conversation_id);
let agent_run_id = agent_run.id.as_str().to_string();
self.agent_run_repo.create(agent_run).await...?;

// NEW: Update state history for task-related contexts
if matches!(context_type, ChatContextType::TaskExecution | ChatContextType::Review) {
    let task_id = TaskId::from_string(context_id.to_string());
    // Best-effort: don't fail send_message if this fails
    let _ = self.task_repo.update_latest_state_history_metadata(
        &task_id,
        &conversation_id.as_str().to_string(),
        &agent_run_id,
    ).await;
}
```

**Note:** Use best-effort (ignore errors) to avoid breaking send_message if metadata update fails.

### Task 3: Backend - Expose metadata in state history API (BLOCKING)

**Dependencies:** Task 2
**Atomic Commit:** `feat(query): expose conversation_id and agent_run_id in state transitions API`

**Files:**
- `src-tauri/src/application/commands/query.rs` (`get_task_state_transitions`)
- Response schema structs

**Changes:**
1. Parse `metadata` JSON to extract `conversation_id` and `agent_run_id`
2. Add to `StateTransitionResponse`:
   - `conversation_id: Option<String>`
   - `agent_run_id: Option<String>`

### Task 4: Frontend - Add metadata fields to state transition types (BLOCKING)

**Dependencies:** Task 3
**Atomic Commit:** `feat(api): add conversationId and agentRunId to state transition types`

**Files:**
- `src/api/tasks.schemas.ts`
- `src/api/tasks.transforms.ts`
- `src/api/tasks.types.ts`

**Changes:**
1. Schema: `conversation_id: z.string().optional()`, `agent_run_id: z.string().optional()`
2. Type: `conversationId?: string`, `agentRunId?: string`
3. Transform: `conversationId: raw.conversation_id`, `agentRunId: raw.agent_run_id`

### Task 5: Frontend - Wire conversation selection to history navigation

**Dependencies:** Task 4
**Atomic Commit:** `feat(ui): wire conversation selection to state history navigation`

**Files:**
- `src/components/tasks/StateTimelineNav.tsx`
- `src/components/tasks/TaskDetailOverlay.tsx`
- `src/hooks/useChatPanelContext.ts`
- `src/components/Chat/IntegratedChatPanel.tsx`

**Changes:**
1. `StateTimelineNav`: Pass both `conversationId` and `agentRunId` when calling `onSelectState`
2. `TaskDetailOverlay`: Track selected historical state metadata, pass to chat panel
3. `useChatPanelContext`: Add `overrideConversationId` and `overrideAgentRunId` props
4. `IntegratedChatPanel`: When `overrideAgentRunId` is set:
   - Fetch the agent_run to get its `started_at` timestamp
   - Scroll to the first message with `created_at >= started_at`
   - Optionally: highlight or filter messages from this agent_run period

**Note:** `ChatMessage` doesn't have `agent_run_id` field, but `AgentRun` has `started_at` timestamp. Use this to scroll to the correct position in the conversation.

## Critical Files

| File | Purpose |
|------|---------|
| `src-tauri/src/domain/repositories/task_repository.rs` | Add `update_latest_state_history_metadata` trait method |
| `src-tauri/src/infrastructure/sqlite/task_repository.rs` | Implement the new method |
| `src-tauri/src/application/chat_service/mod.rs:328-348` | Integration point in `send_message()` |
| `src-tauri/src/application/commands/query.rs` | `get_task_state_transitions` - expose metadata |
| `src/api/tasks.schemas.ts` + `.transforms.ts` + `.types.ts` | Frontend types |
| `src/components/tasks/StateTimelineNav.tsx` | History timeline UI |
| `src/hooks/useChatPanelContext.ts` | Chat context management |
| `src/components/Chat/IntegratedChatPanel.tsx` | Chat panel rendering |

## Verification

### Test Scenario: Multiple Revision Cycles

1. Create task → let it execute → review → request_changes → re_execute → review → request_changes → re_execute → review → approve

### Expected State History
```
ready → executing(conv1, run1) → pending_review → reviewing(conv2, run2) → revision_needed
  → re_executing(conv1, run3) → pending_review → reviewing(conv2, run4) → revision_needed
  → re_executing(conv1, run5) → pending_review → reviewing(conv2, run6) → review_passed → approved
```

### Verification Steps

1. Open task detail overlay with history timeline
2. Click `executing` state → chat shows conv1 messages, scrolled to run1
3. Click first `reviewing` state → chat shows conv2, scrolled to run2
4. Click `re_executing` (1st) → chat shows conv1, scrolled to run3 (different from run1)
5. Click second `reviewing` → chat shows conv2, scrolled to run4 (different from run2)
6. Click `re_executing` (2nd) → chat shows conv1, scrolled to run5
7. Click third `reviewing` → chat shows conv2, scrolled to run6

### Success Criteria
- Each historical state shows the correct conversation
- When same conversation is reused, scrolls to the specific agent_run's messages
- Non-conversation states (ready, pending_review, revision_needed, etc.) don't have conversation metadata

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Graph

```
Task 1 (Backend: repo method)
    ↓
Task 2 (Backend: integration in chat_service)
    ↓
Task 3 (Backend: expose in API)
    ↓
Task 4 (Frontend: types)
    ↓
Task 5 (Frontend: UI wiring)
```

All tasks follow layer boundaries - each compiles independently after the previous completes.
