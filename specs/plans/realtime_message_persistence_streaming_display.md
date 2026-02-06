# Fix: Real-Time Message Persistence + Streaming Display During Worker Execution

## Context

During worker agent execution (especially after app restart/recovery), the chat panel shows only a typing indicator or tool call chips while the agent runs. The assistant's streaming text is invisible — users see nothing until the entire run completes, at which point ALL messages appear at once. If the app crashes mid-run, the assistant's partial response is lost entirely.

**Root cause (two parts):**

1. **Backend: Assistant message only persisted at stream END** — In `chat_service_send_background.rs:141-151`, the assistant message (text + tool calls + content blocks) is created in `chat_messages` DB table ONLY after `process_stream_background()` returns. During streaming, only Tauri events are emitted. The `ChatMessageRepository` trait has NO `update` method — only `create`. If the app crashes mid-run, all streaming content is lost.

2. **Frontend: No `agent:chunk` subscription** — `useIntegratedChatEvents.ts` subscribes to `agent:tool_call` (accumulated as `streamingToolCalls`) but NOT to `agent:chunk` (streaming text). This was intentionally disabled (`useAgentEvents.ts:56-63`). Tool calls were later re-enabled, text was not.

**Current persistence timing:**

| What | When Persisted | Recovery Safe? |
|------|---------------|----------------|
| User message | Before stream starts (`mod.rs:419-430`) | Yes |
| Assistant text | After stream completes (`send_background.rs:141-151`) | **NO** |
| Tool calls | After stream completes (part of assistant msg) | **NO** |
| Activity events | Real-time (`chat_service_streaming.rs:144-164`) | Yes (but separate table) |

## Approach: Two-Pronged Fix

### Part 1: Backend — Incremental Assistant Message Persistence

Create the assistant message at stream START (empty), then update it periodically during streaming.

### Part 2: Frontend — Streaming Text Display via Events

Subscribe to `agent:chunk` events for real-time text display (no 2s DB polling lag).

---

## Task Dependency Graph

```
Task 1: Trait + Implementations (BLOCKING) ─┐
                                             ├─→ Task 2: Streaming + Send Background + Queue (BLOCKING)
                                             │
                                             └─→ Task 3: Frontend Streaming Display
```

**Compilation Unit Notes:**
- Tasks 1a/1b/1c MUST be one task — adding trait method requires ALL implementors + test mock
- Tasks 1d/1e MUST be one task — changing `process_stream_background` signature requires ALL call sites (send_background.rs:104-112, queue.rs:172-180, and inline queue at send_background.rs:450-458)
- Task 1f is NOT needed — `StreamProcessor` fields are already `pub` (`response_text`, `tool_calls`) per stream_processor.rs:201-202. Use `processor.response_text.clone()` and serialize `processor.tool_calls` directly.
- Tasks 2a/2b/2c/2d MUST be one task — adding prop to hook return type/props requires all consumers to update simultaneously

---

## Part 1: Backend Changes

### Task 1: Add `update_content` to `ChatMessageRepository` trait + all implementations (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(chat): add update_content to ChatMessageRepository for incremental persistence`

#### 1a. Add `update_content` to `ChatMessageRepository` trait

**File:** `src-tauri/src/domain/repositories/chat_message_repository.rs`

```rust
/// Update message content, tool_calls, and content_blocks (for incremental persistence)
async fn update_content(
    &self,
    id: &ChatMessageId,
    content: &str,
    tool_calls: Option<&str>,       // JSON string
    content_blocks: Option<&str>,   // JSON string
) -> AppResult<()>;
```

**NOTE:** Must also add `update_content` to the `MockChatMessageRepository` in the same file's `#[cfg(test)]` module (lines 62-187) to avoid compile error.

#### 1b. Implement in SQLite repo

**File:** `src-tauri/src/infrastructure/sqlite/sqlite_chat_message_repo.rs`

```rust
async fn update_content(
    &self,
    id: &ChatMessageId,
    content: &str,
    tool_calls: Option<&str>,
    content_blocks: Option<&str>,
) -> AppResult<()> {
    let conn = self.conn.lock().await;
    conn.execute(
        "UPDATE chat_messages SET content = ?1, tool_calls = ?2, content_blocks = ?3 WHERE id = ?4",
        rusqlite::params![content, tool_calls, content_blocks, id.as_str()],
    ).map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}
```

#### 1c. Implement in memory repo

**File:** `src-tauri/src/infrastructure/memory/memory_chat_message_repo.rs`

Find message by ID, update fields.

### Task 2: Incremental streaming persistence + create-before-stream pattern (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(chat): incremental assistant message persistence during streaming`

**Compilation unit:** This task changes `process_stream_background` signature — must update ALL 3 call sites in same task:
1. `chat_service_send_background.rs:104-112` (primary send)
2. `chat_service_send_background.rs:450-458` (inline queue processing)
3. `chat_service_queue.rs:172-180` (standalone queue processing)

#### 1d. Modify streaming to persist incrementally

**File:** `src-tauri/src/application/chat_service/chat_service_streaming.rs`

Change `process_stream_background` signature to accept `chat_message_repo` + `assistant_message_id`:

```rust
pub async fn process_stream_background<R: Runtime>(
    // ... existing params ...
    chat_message_repo: Option<Arc<dyn ChatMessageRepository>>,  // NEW
    assistant_message_id: Option<String>,                        // NEW
) -> Result<(String, Vec<ToolCall>, Vec<ContentBlockItem>, Option<String>), String>
```

Inside the streaming loop, add debounced flush (every 2 seconds):

```rust
let mut last_flush = std::time::Instant::now();
const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

// After accumulating text/tool_calls:
if last_flush.elapsed() >= FLUSH_INTERVAL {
    if let (Some(ref repo), Some(ref msg_id)) = (&chat_message_repo, &assistant_message_id) {
        let current_text = processor.get_accumulated_text();
        let current_tools = processor.get_completed_tool_calls_json();
        let _ = repo.update_content(
            &ChatMessageId::from_string(msg_id.clone()),
            &current_text,
            current_tools.as_deref(),
            None,  // content_blocks only on final update
        ).await;
    }
    last_flush = std::time::Instant::now();
}
```

**NOTE on StreamProcessor access:** `response_text` and `tool_calls` are **public fields** (stream_processor.rs:201-202). No accessor methods needed — use `processor.response_text.clone()` and `serde_json::to_string(&processor.tool_calls).ok()` directly.

#### 1e. Create assistant message BEFORE streaming, update after

**File:** `src-tauri/src/application/chat_service/chat_service_send_background.rs`

**Before streaming:**
```rust
// Create empty assistant message BEFORE streaming starts
let assistant_msg = chat_service_context::create_assistant_message(
    context_type, &context_id, "", conversation_id, &[], &[],
);
let assistant_msg_id = assistant_msg.id.as_str().to_string();
let _ = chat_message_repo.create(assistant_msg).await;

// Emit message_created so frontend shows it
// (empty content - frontend will fill from streaming events)
if let Some(ref handle) = app_handle {
    let _ = handle.emit("agent:message_created", AgentMessageCreatedPayload {
        message_id: assistant_msg_id.clone(),
        conversation_id: conversation_id.as_str().to_string(),
        context_type: context_type.to_string(),
        context_id: context_id.clone(),
        role: get_assistant_role(&context_type).to_string(),
        content: String::new(),
    });
}
```

**Pass to streaming:**
```rust
let result = process_stream_background(
    child, context_type, &context_id, &conversation_id,
    app_handle.clone(),
    Some(Arc::clone(&activity_event_repo)),
    Some(Arc::clone(&task_repo)),
    Some(Arc::clone(&chat_message_repo)),      // NEW
    Some(assistant_msg_id.clone()),              // NEW
).await;
```

**After streaming (replace create with update):**
```rust
// CHANGED: Update existing message instead of creating new one
if !response_text.is_empty() || !tool_calls.is_empty() {
    let tool_calls_json = serde_json::to_string(&tool_calls).ok();
    let content_blocks_json = serde_json::to_string(&content_blocks).ok();
    let _ = chat_message_repo.update_content(
        &ChatMessageId::from_string(assistant_msg_id.clone()),
        &response_text,
        tool_calls_json.as_deref(),
        content_blocks_json.as_deref(),
    ).await;

    // Emit message_created with full content (triggers frontend cache refresh)
    // ... existing emit code ...
}
```

**Same pattern for queue processing** in `chat_service_send_background.rs:~460` and `chat_service_queue.rs:~185`.

**NOTE:** Task 1f from original plan is NOT needed — `StreamProcessor.response_text` and `StreamProcessor.tool_calls` are already `pub` fields (verified at `stream_processor.rs:201-202`). Access directly as `processor.response_text.clone()` and `serde_json::to_string(&processor.tool_calls).ok()`.

---

## Part 2: Frontend Changes

### Task 3: Frontend streaming text display via events
**Dependencies:** None (backend events already emitted; frontend can accumulate even before persistence changes)
**Atomic Commit:** `feat(chat): display streaming assistant text via agent:chunk events`

**Compilation unit:** All 4 files must change together — adding `streamingText` state to context hook changes the return type, which is consumed by `IntegratedChatPanel`, which passes it to `useIntegratedChatEvents` and `ChatMessageList`.

#### 2a. `useChatPanelContext.ts` — Add streaming text state

**File:** `src/hooks/useChatPanelContext.ts`

```tsx
const [streamingText, setStreamingText] = useState<string>("");

// In context change effect (line ~136), add:
setStreamingText("");

// Return alongside existing values:
return { ..., streamingText, setStreamingText };
```

#### 2b. `useIntegratedChatEvents.ts` — Subscribe to chunk events

**File:** `src/hooks/useIntegratedChatEvents.ts`

Add `setStreamingText` to props. Subscribe to `agent:chunk` + `chat:chunk`:

```tsx
// Streaming text chunks
bus.subscribe<{ text: string; conversation_id: string }>(
  "agent:chunk", (payload) => {
    if (payload.conversation_id === activeConversationIdRef.current) {
      setStreamingText((prev) => prev + payload.text);
    }
  }
);

// Legacy chunk event
bus.subscribe<{ text: string; conversation_id: string }>(
  "chat:chunk", (payload) => {
    if (payload.conversation_id === activeConversationIdRef.current) {
      setStreamingText((prev) => prev + payload.text);
    }
  }
);

// In ALL completion handlers, add: setStreamingText("");
// In cleanup, add: setStreamingText("");
```

#### 2c. `ChatMessageList.tsx` — Render streaming text

**File:** `src/components/Chat/ChatMessageList.tsx`

Add `streamingText` prop. In Footer, render as assistant bubble before tool indicator:

```tsx
{streamingText && (
  <MessageItem
    key="streaming-assistant"
    role="assistant"
    content={streamingText}
    createdAt={new Date().toISOString()}
    toolCalls={null}
    contentBlocks={null}
  />
)}
{/* Only show TypingIndicator when no streaming text */}
{(isSending || isAgentRunning) && !streamingText && streamingToolCalls.length === 0 && (
  <TypingIndicator />
)}
```

#### 2d. `IntegratedChatPanel.tsx` — Wire it up

**File:** `src/components/Chat/IntegratedChatPanel.tsx`

Thread `streamingText` + `setStreamingText` from `useChatPanelContext` to `useIntegratedChatEvents` and `ChatMessageList`.

---

## Files to Modify Summary

| Task | File | Layer | Change |
|------|------|-------|--------|
| 1 | `src-tauri/src/domain/repositories/chat_message_repository.rs` | Backend | Add `update_content` to trait + test mock |
| 1 | `src-tauri/src/infrastructure/sqlite/sqlite_chat_message_repo.rs` | Backend | Implement SQL UPDATE |
| 1 | `src-tauri/src/infrastructure/memory/memory_chat_message_repo.rs` | Backend | Implement in-memory update |
| 2 | `src-tauri/src/application/chat_service/chat_service_streaming.rs` | Backend | Accept repo+msg_id, debounced flush |
| 2 | `src-tauri/src/application/chat_service/chat_service_send_background.rs` | Backend | Create msg before stream, update after (both primary + inline queue) |
| 2 | `src-tauri/src/application/chat_service/chat_service_queue.rs` | Backend | Same create-before-stream pattern |
| 3 | `src/hooks/useChatPanelContext.ts` | Frontend | Add `streamingText` state |
| 3 | `src/hooks/useIntegratedChatEvents.ts` | Frontend | Subscribe to `agent:chunk` |
| 3 | `src/components/Chat/ChatMessageList.tsx` | Frontend | Render streaming text |
| 3 | `src/components/Chat/IntegratedChatPanel.tsx` | Frontend | Thread streaming text |

**Removed:** `StreamProcessor` accessor methods — fields are already `pub` (stream_processor.rs:201-202)

## Edge Cases

| Case | Handling |
|------|----------|
| App crash mid-stream | Last flush (up to 2s old) is in DB. On recovery, partial assistant message visible. |
| Run completes normally | Final update replaces partial content with complete text + tool_calls + content_blocks |
| Empty assistant response | Empty message created at start; if response stays empty, delete it (or leave empty) |
| Context switch mid-stream | Frontend clears `streamingText`; backend flush continues independently |
| Queue processing (--resume) | Same pattern: create empty assistant msg, flush during stream, update on complete |

## Verification

1. **Manual test — streaming display:**
   - Start worker agent execution → open task detail chat
   - Verify: assistant text streams in real-time, tool calls show as indicators
   - Verify: on completion, streaming text replaced by persisted message

2. **Manual test — crash recovery:**
   - Start worker execution → quit app mid-stream
   - Reopen app → check DB: `SELECT content FROM chat_messages WHERE role='assistant' ORDER BY created_at DESC LIMIT 1`
   - Verify: partial assistant text present in DB
   - Open task detail → verify partial message visible + recovery run starts

3. **Lint:**
   - Backend: `cargo clippy --all-targets --all-features -- -D warnings && cargo test`
   - Frontend: `npm run lint && npm run typecheck && npm run test:run`

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
