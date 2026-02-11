# Fix Conversation Stop Behavior End-to-End (Integrated Chat)

## Summary
The Stop action in conversation chat is not reliably stopping the active run because the stop path in `IntegratedChatPanel` is context-inconsistent and UI activity state is over-derived from workflow status instead of live run state.

Identified issues:
1. `useIntegratedChatHandlers` routes Stop to the wrong context in review/merge flows.
   - `src/hooks/useIntegratedChatHandlers.ts:213` builds stop context without `review` or `merge`.
   - In review/merge, it falls back to `task`, so `stop_agent` targets the wrong registry key.
2. Execution-mode stop bypasses unified chat stop semantics.
   - `src/hooks/useIntegratedChatHandlers.ts:225` calls `recoverTaskExecution(...)` instead of `stopAgent("task_execution", ...)`.
   - This couples stop-click to reconciliation policy and can leave UX ambiguous.
3. UI keeps showing "agent active" in execution mode even when no live run is active.
   - `src/components/Chat/IntegratedChatPanel.tsx:533` uses `isExecutionMode` as an activity signal.
   - `src/components/Chat/IntegratedChatPanel.tsx:694` passes `isExecutionMode || isAgentRunning` into `ChatInput`, so Stop affordance can persist independently of real run state.

## Implementation Plan

### 1. Normalize stop target context (single source of truth)
1. Add/extend a shared context resolver in `useIntegratedChatHandlers` to return exact `ContextType` + `contextId` for all modes:
   - `task_execution` when `isExecutionMode`
   - `review` when `isReviewMode`
   - `merge` when `isMergeMode`
   - `ideation` when `ideationSessionId`
   - `task` when `selectedTaskId` (non-agent task chat)
   - `project` fallback
2. Update hook props in `useIntegratedChatHandlers` to include `isMergeMode`.
3. Use this resolver for `handleStopAgent` (and optionally all queue/delete/edit context operations to prevent future drift).

### 2. Make stop-click always perform immediate run cancellation first
1. In `handleStopAgent`, always call unified `stopAgent(ctxType, ctxId)` first.
2. For execution mode only, run `recoverTaskExecution(selectedTaskId)` after successful stop attempt (or regardless, but only after cancellation path is attempted), so task status reconciliation remains supported.
3. Keep stop non-throwing to preserve UX, but instrument failures (existing logger/toast pattern) so silent failures are reduced.

### 3. Decouple "agent active" UI from execution-status UI
1. In `IntegratedChatPanel`, change:
   - `isAgentActive` to depend on live run signals only (`isSending || isAgentRunning`) for history-disabled mode.
   - `ChatInput.isAgentRunning` prop to use live run state only, not `isExecutionMode`.
2. Keep execution-mode-specific placeholder text if desired, but do not use it as "run active" truth.
3. Result: Stop button and "Agent responding..." only appear when a run is actually active.

### 4. Keep event-driven shutdown behavior intact and robust
1. Confirm existing listeners in `useAgentEvents` continue clearing store state on:
   - `agent:run_completed`
   - `agent:error`
2. Optionally add explicit `agent:stopped` listener in `useAgentEvents` to set `setAgentRunning(contextKey, false)` immediately (defensive; backend already emits `run_completed` on stop, but this improves resilience if completion emission regresses).

### 5. Add focused tests for regressions
1. `useIntegratedChatHandlers` tests (new file if absent):
   - Stops with `review` context in review mode.
   - Stops with `merge` context in merge mode.
   - In execution mode, calls `stopAgent("task_execution", taskId)` and then `recoverTaskExecution(taskId)` in order.
2. `IntegratedChatPanel` behavior tests:
   - Stop button visibility follows live `isAgentRunning`, not `isExecutionMode`.
   - Status badge does not show "Agent responding..." when execution status is active but no live run.
3. Optional event test:
   - `agent:stopped` (if listener added) clears running state.

### 6. Validation scenarios (manual + automated)
1. Ideation chat: start run -> click Stop -> stream halts, badge clears, no further chunks.
2. Review chat: start review run -> click Stop -> run cancels (previously failed due `task` context mismatch).
3. Merge chat: same as review.
4. Execution chat: start worker run -> click Stop -> immediate cancellation + recovery transition behavior preserved.
5. Negative case: Stop with no running agent returns gracefully and does not show false active state.

## Public API / Interface Changes
1. `useIntegratedChatHandlers` input contract:
   - Add `isMergeMode: boolean`.
2. Internal behavior contract:
   - `handleStopAgent` guarantees unified stop attempt against the exact active chat context before any recovery/reconciliation work.

No backend API signature changes are required.

## Assumptions and Defaults
1. Expected UX: Stop in conversation chat means "cancel the currently active agent run for this conversation context now."
2. Execution recovery is still desired, but it is secondary to immediate cancellation.
3. `agent:run_completed` remains emitted on stop by backend (`src-tauri/src/application/chat_service/mod.rs:734`), with optional `agent:stopped` handling as extra safety.
4. No migration or data model changes are needed; this is handler/state logic only.
