# Ideation + Chat Production-Grade Data-Fetch Refactor Plan

## Summary
This plan upgrades ideation/chat to production-grade by making data loading view-driven, bounded, and event-consistent with low resource usage.

Multiple small, purpose-built calls are better here than hydrating huge payloads and relying on broad cache invalidation.
The plan adopts:
1. Query-first state ownership.
2. Paged chat loading (newest first, load older on demand).
3. Patch-first cache sync with narrow refetch fallback.

## Current Problems Confirmed
1. Overfetch on ideation load: `get_ideation_session_with_data` returns session + all proposals + all session messages (`src-tauri/src/commands/ideation_commands/ideation_commands_session.rs`).
2. Duplicate ownership: App hydrates proposals from `sessionWithData` into Zustand while hooks also fetch/mutate/invalidate (`src/App.tsx`, `src/hooks/useIdeation.ts`, `src/hooks/useProposals.ts`).
3. Chat payloads are unbounded: `get_agent_conversation` returns all conversation messages; no pagination (`src-tauri/src/commands/unified_chat_commands.rs`, `src-tauri/src/application/chat_service/chat_service_repository.rs`).
4. Chat panel overfetches tasks: `IntegratedChatPanel` calls `useTasks(projectId)` (project-wide list) to derive one selected task (`src/components/Chat/IntegratedChatPanel.tsx`).
5. High invalidation fan-out: proposal/dependency/apply flows invalidate broad prefixes (`src/hooks/useEvents.proposal.ts`, `src/hooks/useDependencyGraph.ts`, `src/hooks/useApplyProposals.ts`).
6. Polling duplication in chat: several concurrent 2s intervals can overlap (`src/components/Chat/IntegratedChatPanel.tsx`).

## Target Architecture
1. Server data source of truth: React Query only for sessions/proposals/chat/dependency graph.
2. Zustand scope: UI state only (panel visibility, active selection, ephemeral streaming/queue flags).
3. View contracts:
   - Ideation shell data: session metadata + proposal summary list (no chat messages).
   - Chat panel data: conversations list + paged messages for active conversation only.
   - Dependency graph: fetched only when dependency panel/features are visible.
4. Consistency model:
   - Patch cache from event payloads first.
   - Narrow refetch only when event lacks required fields or sequence is uncertain.
5. Bounded data and memory:
   - Cursor pagination for chat messages.
   - Explicit page size defaults and max limits.
   - Query cache GC/stale tuned per data domain.

## Implementation Plan

### Phase 1: Split Ideation Read APIs by View Needs
1. Add backend read endpoint `get_ideation_session_overview` returning session + proposal summaries + counts; no message array.
2. Keep `get_ideation_session_with_data` temporarily for migration only, mark deprecated, and remove from UI path.
3. Add `list_session_proposals_paginated` with `cursor`/`limit` (future-safe for very large plans; default still fetch all if small).
4. Update frontend ideation API layer in `src/api/ideation.ts` and schemas/transforms to consume overview response.
5. Update ideation hooks in `src/hooks/useIdeation.ts` to use overview query for main screen.

### Phase 2: Add Chat Message Pagination End-to-End
1. Add backend command `get_agent_conversation_page` with input `{ conversation_id, cursor, limit, direction }`.
2. Add repository query for conversation messages with cursor (newest-first fetch path, deterministic ordering).
3. Keep existing `get_agent_conversation` for backward compatibility during migration; stop using it in active views.
4. Add frontend API method `chatApi.getConversationPage(...)` and Zod schemas.
5. Replace `useConversation` in `src/hooks/useChat.ts`/chat panel with `useInfiniteQuery` page model.
6. UI behavior:
   - Initial load = latest page (e.g., 50).
   - Scroll-up fetches older pages.
   - New streaming/event messages patch into page 0 without full refetch.

### Phase 3: Remove Duplicate Data Ownership and Hydration
1. Stop syncing proposals from `sessionData` into `proposalStore` in `src/App.tsx`.
2. Replace proposal/session domain storage in Zustand with selectors from Query cache.
3. Keep only ephemeral UI hints in store (highlight timestamps, scroll focus targets, local panel behavior).
4. Update `PlanningView` props path to consume query-derived proposals directly.
5. Ensure session switching remains flash-free using query keys + placeholder strategy without store duplication.

### Phase 4: Patch-First Event Handling (No Broad Invalidations)
1. Replace broad `invalidateQueries` in proposal/dependency hooks with targeted `setQueryData` patches for:
   - proposal created/updated/deleted/reordered.
   - dependency add/remove.
   - chat message/tool/run lifecycle events.
2. Keep fallback targeted invalidation only for ambiguous events.
3. Remove prefix-wide invalidations like `ideationKeys.sessions()`/`ideationKeys.proposals()` when session-specific patch is possible.
4. Add utility patch layer per domain to centralize cache updates and avoid drift.

### Phase 5: Chat Panel Query and Polling Hardening
1. Remove project-wide `useTasks(projectId)` from `IntegratedChatPanel`; use `taskKeys.detail(selectedTaskId)` only when needed.
2. Collapse overlapping polling loops into one orchestrated ticker per active context with stop conditions.
3. Prefer event-driven updates; polling only as recovery fallback with strict TTL window.
4. Ensure execution/review/merge context switching does not trigger extra list fetches unless context actually changed.
5. Apply query options per chat type: short stale for active run, longer stale for idle histories.

### Phase 6: Query Budget + Limits + Indexing
1. Define fetch budgets:
   - Conversations list: max 100 per context view.
   - Messages page size: default 50, max 200.
   - Proposal overview list: default 100 visible, incremental fetch for larger plans.
2. Add/verify DB indexes for new cursor queries on chat messages by `(conversation_id, created_at, id)` optimized for pagination.
3. Add guardrails in command handlers to clamp limits.
4. Add telemetry counters for payload size and query latency (debug logs + optional metrics sink).

### Phase 7: Migration and Cleanup
1. Introduce new APIs behind feature flag `ideation_query_v2`.
2. Migrate ideation and integrated chat components first; keep old endpoints for one release window.
3. Remove deprecated with-data usage from all hooks/components after parity tests pass.
4. Delete dead code paths and broad invalidation remnants.

## Public API / Interface Changes
1. New Tauri commands:
   - `get_ideation_session_overview`
   - `get_agent_conversation_page`
   - `list_session_proposals_paginated` (if adopted immediately)
2. New frontend API types:
   - `IdeationSessionOverviewResponse`
   - `ConversationMessagesPageResponse`
   - `CursorPage<T>`
3. Hook contract updates:
   - `useIdeationSession` becomes overview-oriented.
   - `useConversation` replaced by `useConversationPages` (infinite query model).
4. Event payload compatibility:
   - Prefer including `session_id`/`conversation_id` and minimal changed entity fields for deterministic cache patching.

## Test Cases and Scenarios
1. Unit tests:
   - Cursor pagination correctness (no duplicate/missing messages across pages).
   - Cache patch reducers for create/update/delete/reorder.
   - Query-key scoping and no cross-context bleed.
2. Integration tests:
   - Session switch among many sessions does not load chat history unless chat panel needs it.
   - Large conversation (1k+ msgs) loads first paint fast and fetches older pages only on demand.
   - Proposal edits/deletes update UI instantly without broad refetch storms.
3. Performance tests:
   - Baseline vs refactor on network calls, total bytes, render time, and memory.
   - Ensure reduced queries while preserving live chat correctness under streaming events.
4. Failure-mode tests:
   - Missed event recovery via targeted fallback refetch.
   - Out-of-order events do not corrupt cache state.
   - Backend command errors preserve UI consistency and retries are bounded.

## Acceptance Criteria
1. Ideation opening a session does not fetch full chat history by default.
2. No endpoint in the hot ideation/chat path returns unbounded collections without cursor/limit.
3. Query invalidation count is reduced significantly; most updates are cache patches.
4. Chat remains real-time correct for running agents and stable across context switches.
5. Memory and CPU usage improve measurably with no functional regression.

## Assumptions and Defaults
1. Chosen architecture: Query-first state ownership.
2. Chosen chat policy: paged window loading (newest-first).
3. Chosen sync policy: patch-first cache consistency.
4. Default page size: 50; hard max: 200.
5. Migration keeps legacy commands temporarily for safe rollout.
