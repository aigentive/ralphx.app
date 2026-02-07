# RalphX - Phase 92: Deduplicate Tool Calls in IntegratedChatPanel

## Overview

Fix duplicate tool call display in the streaming indicator for `IntegratedChatPanel` (Kanban/Ideation chat). The backend emits 2-3 `agent:tool_call` events per tool call (Started + Completed + Result), and `useIntegratedChatEvents.ts` blindly appends every event without deduplication — causing each tool call to appear **twice** in `StreamingToolIndicator`.

The sibling hook `useChatPanelHandlers.ts` was already fixed in Phase 41 with the correct upsert-by-`tool_id` pattern. This is a port of that proven fix to the remaining hook.

**Reference Plan:**
- `specs/plans/fix_duplicate_tool_call_display_in_streaming_indicator.md` - Root cause analysis, before/after code, Phase 91 compatibility notes

## Goals

1. Eliminate duplicate tool call entries in IntegratedChatPanel's streaming indicator
2. Port the proven upsert-by-`tool_id` dedup pattern from `useChatPanelHandlers.ts`
3. Filter unnecessary `result:toolu*` events at the listener level

## Dependencies

### Phase 41 (Streaming Tool Call Deduplication) - Reference

| Dependency | Why Needed |
|------------|------------|
| Dedup pattern in `useChatPanelHandlers.ts` | Proven reference implementation to port |

### Phase 91 (Chat Diff View) - Forward Compatible

| Dependency | Why Needed |
|------------|------------|
| `diff_context` field on `AgentToolCallPayload` | This fix is additive; Phase 91 Task 6 can extend the upsert later |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_duplicate_tool_call_display_in_streaming_indicator.md`
2. Review the reference implementation in `src/hooks/useChatPanelHandlers.ts:226-266`
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
4. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_duplicate_tool_call_display_in_streaming_indicator.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Port tool_id dedup and result-event filtering to useIntegratedChatEvents",
    "plan_section": "Fix: 1 file, ~20 lines changed",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(chat-streaming): deduplicate tool call events in integrated chat panel",
    "steps": [
      "Read specs/plans/fix_duplicate_tool_call_display_in_streaming_indicator.md",
      "Read reference implementation in src/hooks/useChatPanelHandlers.ts:226-266",
      "In src/hooks/useIntegratedChatEvents.ts, add tool_id?: string to the agent:tool_call event payload type (line 47-52)",
      "Destructure tool_id from payload alongside existing fields (line 53)",
      "Add early return when tool_name.startsWith('result:toolu') before the conversation_id check",
      "Replace the append-only setStreamingToolCalls with upsert-by-tool_id: find existing by id, update in-place if found, append if new",
      "Use tool_id ?? `streaming-agent-${Date.now()}` as the id (matching the plan's fallback pattern)",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(chat-streaming): deduplicate tool call events in integrated chat panel"
    ],
    "passes": true
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
| **Port proven pattern from useChatPanelHandlers** | Phase 41 already validated this approach; identical bug, identical fix |
| **Single task (not split)** | One file, ~20 lines, single compilation unit — splitting would add overhead with no benefit |
| **Keep StreamingToolIndicator's render-level filter** | Defense-in-depth; harmless safety net, no reason to remove |
| **Forward-compatible with Phase 91** | Upsert can be extended to merge `diffContext` field later without conflict |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Manual Testing
- [ ] Trigger agent execution in IntegratedChatPanel (Kanban or Ideation chat)
- [ ] Watch the streaming tool indicator — each tool call should appear exactly once
- [ ] Tool call shows immediately on start (generic text), then updates with real args on completion
- [ ] Verify ChatPanel (non-Kanban chat) still works correctly (untouched code)

### Wiring Verification

**Verify the event payload change is correctly wired:**

- [ ] `tool_id` is destructured from the `agent:tool_call` event payload
- [ ] `result:toolu*` events are filtered before state update
- [ ] Existing tool calls are updated in-place (not duplicated)
- [ ] New tool calls are appended with `tool_id` as their `id`
- [ ] Fallback ID (`streaming-agent-${Date.now()}`) handles missing `tool_id`

See `.claude/rules/gap-verification.md` for full verification workflow.
