# RalphX - Phase 87: Real-Time Message Persistence + Streaming Display

## Overview

During worker agent execution, the chat panel shows only a typing indicator or tool call chips. The assistant's streaming text is invisible — users see nothing until the run completes, at which point ALL messages appear at once. If the app crashes mid-run, the assistant's partial response is lost entirely.

This phase fixes two root causes: (1) Backend persists assistant messages only at stream END — no crash recovery. (2) Frontend doesn't subscribe to `agent:chunk` events — no real-time text display.

**Reference Plan:**
- `specs/plans/realtime_message_persistence_streaming_display.md` - Detailed implementation plan with compilation unit analysis, code snippets, and edge case handling

## Goals

1. **Crash-safe assistant messages** — Create assistant DB record at stream START, flush incrementally every 2s during streaming, finalize on completion
2. **Real-time streaming text** — Subscribe frontend to `agent:chunk` events for live assistant text display (replaces typing indicator)
3. **Zero data loss on recovery** — Partial assistant text survives app crashes (up to 2s stale)

## Dependencies

### Phase 15b (Task Execution Chat) - Required

| Dependency | Why Needed |
|------------|------------|
| `ChatMessageRepository` trait | Adding `update_content` method to existing trait |
| `process_stream_background` | Modifying signature to accept repo + message ID |
| `useIntegratedChatEvents` | Adding `agent:chunk` subscription to existing hook |

---

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/realtime_message_persistence_streaming_display.md`
2. Understand the task dependency graph and compilation unit notes
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

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
2. **Read the ENTIRE implementation plan** at `specs/plans/realtime_message_persistence_streaming_display.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add update_content method to ChatMessageRepository trait + SQLite, memory, and test mock implementations",
    "plan_section": "Task 1: Add update_content to ChatMessageRepository trait + all implementations",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(chat): add update_content to ChatMessageRepository for incremental persistence",
    "steps": [
      "Read specs/plans/realtime_message_persistence_streaming_display.md sections 1a, 1b, 1c",
      "Add `update_content(&self, id, content, tool_calls, content_blocks) -> AppResult<()>` to ChatMessageRepository trait in chat_message_repository.rs",
      "Add update_content to MockChatMessageRepository in same file's #[cfg(test)] module (lines 62-187) — return Ok(())",
      "Implement in SqliteChatMessageRepository: UPDATE chat_messages SET content, tool_calls, content_blocks WHERE id",
      "Implement in MemoryChatMessageRepository: find by ID in HashMap, update content/tool_calls/content_blocks fields",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(chat): add update_content to ChatMessageRepository for incremental persistence"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add incremental persistence to streaming: create assistant message before stream, debounced flush during, final update after",
    "plan_section": "Task 2: Incremental streaming persistence + create-before-stream pattern",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(chat): incremental assistant message persistence during streaming",
    "steps": [
      "Read specs/plans/realtime_message_persistence_streaming_display.md sections 1d, 1e",
      "Modify process_stream_background signature in chat_service_streaming.rs to accept Optional chat_message_repo + assistant_message_id",
      "Add debounced flush (2s interval) inside streaming loop using processor.response_text (pub field) and serde_json::to_string(&processor.tool_calls)",
      "In chat_service_send_background.rs: create empty assistant message BEFORE process_stream_background call (line ~104)",
      "Pass chat_message_repo + assistant_msg_id to process_stream_background",
      "After streaming: replace chat_message_repo.create with chat_message_repo.update_content for final content",
      "Update ALL 3 call sites of process_stream_background: send_background.rs:104 (primary), send_background.rs:~450 (inline queue), chat_service_queue.rs:~172 (standalone queue)",
      "Apply same create-before-stream + update-after pattern to queue processing call sites",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(chat): incremental assistant message persistence during streaming"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Display streaming assistant text via agent:chunk events in chat panel",
    "plan_section": "Task 3: Frontend streaming text display via events",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(chat): display streaming assistant text via agent:chunk events",
    "steps": [
      "Read specs/plans/realtime_message_persistence_streaming_display.md sections 2a, 2b, 2c, 2d",
      "In useChatPanelContext.ts: add useState<string>('') for streamingText + setStreamingText, clear on context change, return both",
      "In useIntegratedChatEvents.ts: add setStreamingText to props, subscribe to 'agent:chunk' + 'chat:chunk' events with conversation_id filtering, clear on all completion/cleanup handlers",
      "In ChatMessageList.tsx: add streamingText prop, render as MessageItem before tool indicator, show TypingIndicator only when no streaming text and no tool calls",
      "In IntegratedChatPanel.tsx: destructure streamingText + setStreamingText from useChatPanelContext, pass setStreamingText to useIntegratedChatEvents, pass streamingText to ChatMessageList",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): display streaming assistant text via agent:chunk events"
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
| **Create empty assistant msg at stream START** | Guarantees DB row exists for incremental updates; crash recovery always has partial data |
| **Debounced 2s flush (not per-chunk)** | Per-chunk would hammer DB; 2s balances recovery granularity vs performance |
| **StreamProcessor pub fields (no accessors)** | `response_text` and `tool_calls` are already public — no wrapper methods needed |
| **Frontend independent of backend persistence** | `agent:chunk` events were already emitted; frontend task doesn't require backend changes |
| **update_content replaces create after stream** | Avoids duplicate messages; single row is created then updated in place |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `update_content` correctly updates content, tool_calls, content_blocks in SQLite
- [ ] `update_content` correctly updates fields in memory repo
- [ ] No regressions in existing chat message tests

### Frontend - Run `npm run test`
- [ ] No TypeScript compilation errors from new props
- [ ] Streaming text state resets on context change

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Start worker execution → verify assistant text streams in real-time (not just typing indicator)
- [ ] On completion → streaming text replaced by persisted message (no duplicates)
- [ ] Start execution → quit app mid-stream → reopen → partial assistant text visible in chat
- [ ] DB check: `SELECT content FROM chat_messages WHERE role='assistant' ORDER BY created_at DESC LIMIT 1` shows partial text
- [ ] Queue processing (--resume) → same streaming behavior for follow-up messages

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `update_content` called from streaming loop (debounced flush)
- [ ] `update_content` called after stream completion (final update)
- [ ] `agent:chunk` events subscribed in `useIntegratedChatEvents`
- [ ] `streamingText` flows: `useChatPanelContext` → `IntegratedChatPanel` → `ChatMessageList`
- [ ] `setStreamingText("")` called on all completion/error/cleanup paths

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
