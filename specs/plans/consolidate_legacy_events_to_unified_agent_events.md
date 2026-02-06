# Plan: Consolidate Legacy chat:/execution: Events to Unified agent: Events

## Context

The backend dual-emits every event as both `agent:*` (unified, with `context_type`/`context_id` metadata) and legacy `chat:*`/`execution:*` (without metadata). Two frontend hooks (`useChatPanelHandlers.ts`, `useAgentEvents.ts`) already migrated to `agent:*` only. But `useIntegratedChatEvents.ts` still subscribes to **both**, causing every chunk, tool call, and completion to be processed twice — doubling streaming text, duplicating tool call entries, and double-invalidating caches.

**Goal:** Remove all legacy event subscriptions from the frontend, then remove legacy emissions from the backend, eliminating dead code and the double-processing bug.

## Files to Modify

### Frontend (Task 1)
| File | Change |
|------|--------|
| `src/hooks/useIntegratedChatEvents.ts` | Remove 5 legacy subscriptions: `chat:tool_call` (L53), `chat:chunk` (L113), `chat:run_completed` (L125), `chat:message_created` (L167), `execution:message_created` (L174), `execution:tool_call` (L206), `execution:run_completed` (L230) |
| `src/lib/events.ts` | Remove legacy exports: `CHAT_CHUNK`, `CHAT_TOOL_CALL`, `CHAT_RUN_COMPLETED` (L21-23) |

### Backend (Task 2)
| File | Change |
|------|--------|
| `src-tauri/src/application/chat_service/chat_service_streaming.rs` | Remove legacy `chat:chunk` emission (~L131-138), legacy `chat:tool_call` emissions (~L228-240 ToolCallStarted, ~L267 ToolCallCompleted) |
| `src-tauri/src/application/chat_service/chat_service_send_background.rs` | Remove legacy `chat:run_completed` emissions (~L315-318, ~L565-573), legacy `execution:*/chat:*` message_created emissions (~L170-188, ~L420-424, ~L495-501) |
| `src-tauri/src/application/chat_service/mod.rs` | Remove legacy `execution:run_started`/`chat:run_started` emission (~L407-417), legacy `execution:message_created`/`chat:message_created` emission (~L448-462), legacy run_completed in stop_agent (~L740+) |
| `src-tauri/src/application/chat_service/chat_service_types.rs` | Remove legacy constants: `CHAT_CHUNK`, `CHAT_TOOL_CALL`, `CHAT_RUN_COMPLETED` (L35-37) |

## Task 1: Frontend — Remove legacy event subscriptions (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `refactor(chat): remove legacy event subscriptions from useIntegratedChatEvents`

**Compilation Unit:** ✅ Validated — `CHAT_CHUNK`/`CHAT_TOOL_CALL`/`CHAT_RUN_COMPLETED` exports from `events.ts` have zero other consumers (confirmed via grep). The `useIntegratedChatEvents.ts` legacy subscriptions use hardcoded string literals, not the constants. Both removals compile independently.

1. In `useIntegratedChatEvents.ts`, remove the following `bus.subscribe()` blocks (keep only the `agent:*` equivalents):
   - `chat:tool_call` subscription (L47-72) — already covered by `agent:tool_call` (L74-98)
   - `chat:chunk` subscription (L111-119) — already covered by `agent:chunk` (L100-109)
   - `chat:run_completed` subscription (L122-143) — already covered by `agent:run_completed` (L180-198)
   - `chat:message_created` subscription (L165-170) — already covered by `agent:message_created` (L158-163)
   - `execution:message_created` subscription (L172-177) — already covered by `agent:message_created`
   - `execution:tool_call` subscription (L201-224) — already covered by `agent:tool_call`
   - `execution:run_completed` subscription (L227-248) — already covered by `agent:run_completed`

2. In `src/lib/events.ts`, remove legacy constants and comment (L20-23):
   ```
   - // Legacy events (unified - no longer context-type specific)
   - export const CHAT_CHUNK = "chat:chunk";
   - export const CHAT_TOOL_CALL = "chat:tool_call";
   - export const CHAT_RUN_COMPLETED = "chat:run_completed";
   ```

3. Verify no other frontend files import the removed constants (already confirmed: no other consumers)

4. Run: `npm run lint && npm run typecheck`

## Task 2: Backend — Remove legacy event emissions
**Dependencies:** None (can run in parallel with Task 1 — frontend/backend are independent compilation units)
**Atomic Commit:** `refactor(chat): remove legacy event emissions from chat service`

**Compilation Unit:** ✅ Validated — `events::CHAT_CHUNK`/`CHAT_TOOL_CALL`/`CHAT_RUN_COMPLETED` are only used in `chat_service_streaming.rs` and `chat_service_send_background.rs`. Hardcoded string literals (`"execution:run_started"`, `"chat:run_started"`, etc.) in `mod.rs` and `send_background.rs` have no cross-module dependents. Removing constants + all usages in one task compiles cleanly.

1. In `chat_service_streaming.rs`, remove:
   - Legacy `CHAT_CHUNK` emission after `AGENT_CHUNK` (L131-138)
   - Legacy `CHAT_TOOL_CALL` emission after `AGENT_TOOL_CALL` in `ToolCallStarted` handler (L229-239)
   - Legacy `CHAT_TOOL_CALL` emission after `AGENT_TOOL_CALL` in `ToolCallCompleted` handler (L257-267)
   - Legacy `CHAT_TOOL_CALL` emission after `AGENT_TOOL_CALL` in `ToolResultReceived` handler (L335-345)

2. In `chat_service_send_background.rs`, remove:
   - Legacy `"execution:message_created"`/`"chat:message_created"` emission (L174-187) — after `agent:message_created` for assistant messages
   - Legacy `events::CHAT_RUN_COMPLETED` emission (L320-327) — after `agent:run_completed` post-stream
   - Legacy `events::CHAT_RUN_COMPLETED` emission (L565-572) — after `agent:run_completed` post-queue
   - Legacy `"execution:error"`/`"chat:error"` emission (L616-627) — after `agent:error` in error handler

3. In `chat_service/mod.rs`, remove:
   - Legacy `"execution:run_started"`/`"chat:run_started"` emission (L405-417) — the `// Also emit legacy events` block
   - Legacy `"execution:message_created"`/`"chat:message_created"` emission (L449-462) — the `// Also emit legacy event` block for user messages
   - **Note:** `stop_agent` (L703-751) does NOT have a separate legacy emission — only the unified `agent:run_completed` is emitted. No change needed here.

4. In `chat_service_types.rs`, remove legacy constants and comment (L34-37):
   ```
   - /// Legacy events (unified - no longer context-type specific)
   - pub const CHAT_CHUNK: &str = "chat:chunk";
   - pub const CHAT_TOOL_CALL: &str = "chat:tool_call";
   - pub const CHAT_RUN_COMPLETED: &str = "chat:run_completed";
   ```

5. Run: `cargo clippy --all-targets --all-features -- -D warnings && cargo test`

### Out of Scope (noted for follow-up)
- `execution:error`/`chat:error` emissions in `send_background.rs` (L616-627) — included above
- `useEvents.execution.ts` still subscribes to `execution:error` and `execution:stderr` — these should be migrated to `agent:error` in a follow-up, but are **not** part of the double-processing bug (error events are separate from the chunk/tool_call/run_completed path)
- `execution:stderr` emission — grep for any remaining legacy event strings in verification step

## Verification

1. **Frontend:** `npm run lint && npm run typecheck` — no unused imports or type errors
2. **Backend:** `cargo clippy` — no unused constant warnings, `cargo test` passes
3. **Grep check:** `grep -r "chat:chunk\|chat:tool_call\|chat:run_completed\|execution:tool_call\|execution:run_completed\|execution:message_created\|execution:run_started\|chat:message_created\|chat:run_started" src/ src-tauri/src/` should return zero matches (except comments/docs)
4. **Manual:** Start worker execution → verify streaming text appears once (not doubled), tool calls appear once, completion triggers once

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
- Tasks 1 and 2 are independent compilation units (frontend vs backend) — can be executed in parallel by separate agents
