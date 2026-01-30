# RalphX - Phase 41: Streaming Tool Call Deduplication

## Overview

Fix a bug where the same tool call appears multiple times in the streaming tooltip during live agent execution. The root cause is that the backend emits multiple events per tool call lifecycle (started, completed, result), but the frontend ignores the `tool_id` field and generates weak client-side IDs, causing duplicates.

**Reference Plan:**
- `specs/plans/fix_streaming_tool_call_duplication.md` - Detailed root cause analysis and implementation approach

## Goals

1. Eliminate duplicate tool call entries in the streaming tooltip
2. Use backend-provided `tool_id` for reliable deduplication
3. Filter unnecessary result events at the listener level

## Dependencies

### Phase 15 (Context-Aware Chat) - Required

| Dependency | Why Needed |
|------------|------------|
| Chat event system | Tool call events are emitted via the existing Tauri event system |
| `useChatPanelHandlers` hook | Contains the event listener that needs modification |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_streaming_tool_call_duplication.md`
2. Understand the root cause and deduplication strategy
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_streaming_tool_call_duplication.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Add tool_id to event payload and implement deduplication logic",
    "plan_section": "Task 1: Update event listener to use backend tool_id",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "fix(chat): use backend tool_id for streaming deduplication",
    "steps": [
      "Read specs/plans/fix_streaming_tool_call_duplication.md section 'Task 1'",
      "Add `tool_id?: string` to the TypeScript event payload interface in useChatPanelHandlers.ts",
      "Update setStreamingToolCalls to use tool_id as the unique identifier (fall back to timestamp if null)",
      "Implement Map-based deduplication: if tool_id exists in array, update existing entry; else append new",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(chat): use backend tool_id for streaming deduplication"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Filter out result events early in the listener to avoid unnecessary state updates",
    "plan_section": "Task 2: Filter out result events in the listener",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(chat): filter result events early in tool call listener",
    "steps": [
      "Read specs/plans/fix_streaming_tool_call_duplication.md section 'Task 2'",
      "Add early return in the event listener when tool_name starts with 'result:toolu'",
      "Verify the filtering works correctly by reviewing the existing render-level filter in StreamingToolIndicator.tsx",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(chat): filter result events early in tool call listener"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Verify ToolCall type supports all needed fields for lifecycle tracking",
    "plan_section": "Task 3: Update ToolCall type if needed",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "chore(types): ensure ToolCall type supports lifecycle tracking",
    "steps": [
      "Read specs/plans/fix_streaming_tool_call_duplication.md section 'Task 3'",
      "Locate the ToolCall type definition (likely in ToolCallIndicator.tsx or a types file)",
      "Verify the type includes all fields needed: id, tool_name, arguments, result",
      "If any fields are missing, add them with appropriate optional markers",
      "Run npm run lint && npm run typecheck",
      "Commit: chore(types): ensure ToolCall type supports lifecycle tracking"
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
| **Use backend `tool_id` instead of client-side ID** | Backend already assigns unique IDs per tool call; using these ensures deduplication works across all lifecycle events |
| **Update existing entries instead of filtering events** | The lifecycle events (started → completed → result) carry different data; updating preserves the progressive enrichment |
| **Filter result events at listener level** | Results are already filtered at render time; filtering earlier avoids unnecessary React state updates |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] useChatPanelHandlers deduplicates tool calls by tool_id

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Start the app with dev server running
- [ ] Open chat panel and send a message that triggers tool calls
- [ ] Observe the streaming tooltip - each tool call should appear exactly once
- [ ] After run completes, verify final message shows correct tool count
- [ ] Test with multiple concurrent tool calls to verify no duplicates

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Event listener receives `tool_id` from backend payload
- [ ] Deduplication logic correctly identifies existing entries
- [ ] State updates preserve existing data when updating entries
- [ ] Result events are filtered before state update

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
