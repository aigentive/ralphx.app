# RalphX - Phase 88: Consolidate Legacy Events to Unified agent: Events

## Overview

The backend dual-emits every event as both `agent:*` (unified, with `context_type`/`context_id` metadata) and legacy `chat:*`/`execution:*` (without metadata). Two frontend hooks (`useChatPanelHandlers.ts`, `useAgentEvents.ts`) already migrated to `agent:*` only, but `useIntegratedChatEvents.ts` still subscribes to **both**, causing every chunk, tool call, and completion to be processed twice — doubling streaming text, duplicating tool call entries, and double-invalidating caches.

This phase removes all legacy event subscriptions from the frontend, then removes legacy emissions from the backend, eliminating dead code and the double-processing bug.

**Reference Plan:**
- `specs/plans/consolidate_legacy_events_to_unified_agent_events.md` - Full analysis with exact line references and compilation unit validation

## Goals

1. Fix double-processing bug: streaming text, tool calls, and completions should fire exactly once
2. Remove all legacy `chat:*`/`execution:*` event subscriptions from frontend hooks
3. Remove all legacy `chat:*`/`execution:*` event emissions from Rust backend
4. Remove dead legacy event constants from both frontend and backend

## Dependencies

### Phase 87 (Real-Time Message Persistence + Streaming Display) - Required

| Dependency | Why Needed |
|------------|------------|
| `agent:chunk` event pipeline | Phase 87 wired streaming text display via `agent:chunk` — this phase removes the duplicate `chat:chunk` path |
| Incremental message persistence | The unified `agent:*` events are the sole event pipeline after this phase |

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Tasks 1 and 2 are independent compilation units (frontend vs backend) — can be executed in parallel

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
2. **Read the ENTIRE implementation plan** at `specs/plans/consolidate_legacy_events_to_unified_agent_events.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Remove legacy event subscriptions from useIntegratedChatEvents and legacy constants from events.ts",
    "plan_section": "Task 1: Frontend — Remove legacy event subscriptions",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "refactor(chat): remove legacy event subscriptions from useIntegratedChatEvents",
    "steps": [
      "Read specs/plans/consolidate_legacy_events_to_unified_agent_events.md section 'Task 1'",
      "In useIntegratedChatEvents.ts, remove 7 legacy bus.subscribe() blocks: chat:tool_call (L47-72), chat:chunk (L111-119), chat:run_completed (L122-143), chat:message_created (L165-170), execution:message_created (L172-177), execution:tool_call (L201-224), execution:run_completed (L227-248)",
      "Keep only the agent:* equivalents: agent:tool_call (L74-98), agent:chunk (L100-109), agent:message_created (L158-163), agent:run_completed (L180-198)",
      "In src/lib/events.ts, remove legacy constants and comment (L20-23): CHAT_CHUNK, CHAT_TOOL_CALL, CHAT_RUN_COMPLETED",
      "Verify no other frontend files import the removed constants (grep for CHAT_CHUNK, CHAT_TOOL_CALL, CHAT_RUN_COMPLETED in src/)",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(chat): remove legacy event subscriptions from useIntegratedChatEvents"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Remove legacy event emissions from chat service and legacy constants from chat_service_types",
    "plan_section": "Task 2: Backend — Remove legacy event emissions",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "refactor(chat): remove legacy event emissions from chat service",
    "steps": [
      "Read specs/plans/consolidate_legacy_events_to_unified_agent_events.md section 'Task 2'",
      "In chat_service_streaming.rs, remove: legacy CHAT_CHUNK emission (L131-138), legacy CHAT_TOOL_CALL emissions in ToolCallStarted (L229-239), ToolCallCompleted (L257-267), and ToolResultReceived (L335-345)",
      "In chat_service_send_background.rs, remove: legacy message_created emission (L174-187), legacy CHAT_RUN_COMPLETED emission post-stream (L320-327), legacy CHAT_RUN_COMPLETED emission post-queue (L565-572), legacy execution:error/chat:error emission (L616-627)",
      "In chat_service/mod.rs, remove: legacy run_started emission (L405-417), legacy message_created emission (L449-462)",
      "In chat_service_types.rs, remove legacy constants and comment (L34-37): CHAT_CHUNK, CHAT_TOOL_CALL, CHAT_RUN_COMPLETED",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: refactor(chat): remove legacy event emissions from chat service"
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
| **Remove frontend first, backend second** | Harmless to emit events nobody listens to, but listening to events that stop being emitted would break UX. Task order ensures safety, though both can run in parallel since the double-emit means both old and new events exist during transition. |
| **Two independent tasks, no cross-dependency** | Frontend and backend are separate compilation units. Removing constants/subscriptions in TS doesn't affect Rust compilation, and vice versa. This enables parallel execution. |
| **Include execution:error/chat:error in scope** | Originally out of scope but discovered during analysis — the backend emits legacy error events that `useEvents.execution.ts` still subscribes to. Backend emission removed here; frontend migration deferred to follow-up. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run lint && npm run typecheck`
- [ ] No unused imports of CHAT_CHUNK, CHAT_TOOL_CALL, CHAT_RUN_COMPLETED
- [ ] No TypeScript errors from removed subscriptions

### Backend - Run `cargo clippy && cargo test`
- [ ] No unused constant warnings for CHAT_CHUNK, CHAT_TOOL_CALL, CHAT_RUN_COMPLETED
- [ ] No compilation errors from removed emissions
- [ ] All existing tests pass

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Grep Verification
- [ ] `grep -r "chat:chunk\|chat:tool_call\|chat:run_completed\|execution:tool_call\|execution:run_completed\|execution:message_created\|execution:run_started\|chat:message_created\|chat:run_started" src/ src-tauri/src/` returns zero matches (except comments/docs)

### Manual Testing
- [ ] Start worker execution → streaming text appears once (not doubled)
- [ ] Tool calls appear once in streaming tooltip
- [ ] Run completion triggers cache invalidation once

### Wiring Verification

**Verify unified agent:* events are the sole active pipeline:**

- [ ] `agent:chunk` → `useIntegratedChatEvents.ts` handles streaming text
- [ ] `agent:tool_call` → `useIntegratedChatEvents.ts` handles tool call accumulation
- [ ] `agent:run_completed` → `useIntegratedChatEvents.ts` handles state cleanup
- [ ] `agent:message_created` → `useIntegratedChatEvents.ts` handles cache invalidation
- [ ] No legacy event strings remain in any `bus.subscribe()` calls

**Common failure modes to check:**
- [ ] No `execution:error` regression in `useEvents.execution.ts` (still subscribes — follow-up needed)
- [ ] No orphaned event constants in either frontend or backend

See `.claude/rules/gap-verification.md` for full verification workflow.
