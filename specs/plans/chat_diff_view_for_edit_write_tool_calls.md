# Plan: Chat Diff View for Edit/Write Tool Calls

## Context

Edit/Write tool calls in the chat UI currently render as generic collapsed cards showing raw JSON arguments. The user wants specialized diff-based views inspired by Claude Code's CLI output (shown in screenshots). Key requirements:
- ~3.65 lines preview with gradient blur fade to indicate more content
- Collapsible to show full diff
- Real-time display during streaming (broken out of the aggregated StreamingToolIndicator)
- Persistent diff data that survives branch deletion/merge
- Support for both Local and Worktree git modes

## Architecture Overview

```
┌─ BACKEND ─────────────────────────────────────────────────────────┐
│ chat_service_streaming.rs                                         │
│   ToolCallCompleted(Edit/Write)                                   │
│   → Read old file content (if Write to existing file)             │
│   → Attach as diff_context to content_blocks                      │
│   → Emit agent:tool_call (with diff_context in payload)           │
│   → Resolve path via GitMode (Local=working_dir, Worktree=path)   │
└───────────────────────────────────────────────────────────────────┘
                              ↓ events
┌─ FRONTEND ────────────────────────────────────────────────────────┐
│ DiffToolCallView (NEW)                                            │
│   ← Receives tool call with arguments                             │
│   → Edit: compute diff from old_string → new_string               │
│   → Write: compute diff from diff_context.old_content → content   │
│   → Collapsed: 3.65 lines + gradient mask                         │
│   → Expanded: full diff with line numbers                         │
│                                                                    │
│ ToolCallIndicator → delegates to DiffToolCallView for Edit/Write  │
│ ChatMessageList footer → splits Edit/Write out of streaming list  │
└───────────────────────────────────────────────────────────────────┘
```

## Critical Files

### Backend (modify)
| File | Change |
|------|--------|
| `src-tauri/src/infrastructure/agents/claude/stream_processor.rs` | Add optional `diff_context` field to `ContentBlockItem::ToolUse` and `ToolCall` |
| `src-tauri/src/application/chat_service/chat_service_streaming.rs` | At ToolCallCompleted for Edit/Write: read old file content, attach diff_context |
| `src-tauri/src/application/chat_service/chat_service_types.rs` | Add `diff_context` to `AgentToolCallPayload` |

### Frontend (new)
| File | Purpose |
|------|---------|
| `src/components/Chat/DiffToolCallView.tsx` | Main diff display component (~200 lines) |
| `src/components/Chat/DiffToolCallView.utils.ts` | Diff computation + line parsing (~100 lines) |

### Frontend (modify)
| File | Change |
|------|--------|
| `src/components/Chat/ToolCallIndicator.tsx` | Detect Edit/Write → render DiffToolCallView |
| `src/components/Chat/ChatMessageList.tsx` | Split Edit/Write streaming tool calls from aggregated list in footer |
| `src/components/Chat/StreamingToolIndicator.tsx` | Accept filtered tool calls (Edit/Write removed) |
| `src/hooks/useIntegratedChatEvents.ts` | Pass diff_context from event payload through to streaming state |
| `src/components/Chat/MessageItem.tsx` | Handle diff_context in content blocks for Edit/Write |

## Detailed Design

### Task 1: Backend — Diff Context Capture

**Goal:** When an Edit/Write tool call completes, capture the old file content so we can compute a proper diff later.

**Timing:** `ToolCallCompleted` fires when Claude's arguments are fully parsed but BEFORE the CLI executes the tool. This means the file hasn't been modified yet — we can safely read its current content.

**Git Mode Resolution:**
```
For task_execution context:
  1. Look up task by context_id → get task.worktree_path
  2. Look up project → get project.working_directory + project.git_mode
  3. Worktree mode → use task.worktree_path
  4. Local mode → use project.working_directory
  5. Resolve file_path relative to working dir
```

For non-task contexts (ideation, project chat): use project.working_directory directly (no worktree).

**Changes to stream_processor.rs:**
```rust
// Add optional diff_context to ToolCall and ContentBlockItem::ToolUse
pub struct ToolCall {
    pub id: Option<String>,
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_context: Option<DiffContext>,  // NEW
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffContext {
    pub old_content: Option<String>,  // Previous file content (None if new file)
    pub file_path: String,            // Resolved absolute path for reference
}
```

**Changes to chat_service_streaming.rs:**
```rust
// In the ToolCallCompleted handler:
StreamEvent::ToolCallCompleted(mut tool_call) => {
    // Check if this is Edit or Write
    let name_lower = tool_call.name.to_lowercase();
    if name_lower == "edit" || name_lower == "write" {
        if let Some(file_path) = extract_file_path(&tool_call.arguments) {
            let resolved_path = resolve_working_path(&working_dir, &file_path);
            let old_content = std::fs::read_to_string(&resolved_path).ok();
            tool_call.diff_context = Some(DiffContext {
                old_content,
                file_path: file_path.clone(),
            });
        }
    }
    // ... existing emit logic, now with diff_context in payload
}
```

**Persistence:** The `diff_context` is serialized as part of `ContentBlockItem::ToolUse` in the `content_blocks` JSON column. No migration needed — it's a new optional JSON field on an existing JSON blob.

### Task 2: Frontend — DiffToolCallView Component

**File:** `src/components/Chat/DiffToolCallView.tsx`

**Props:**
```typescript
interface DiffToolCallViewProps {
  toolCall: ToolCall;
  isStreaming?: boolean;  // Show loading state if args not yet complete
  className?: string;
}
```

**Layout (collapsed):**
```
┌─────────────────────────────────────────────┐
│ ▸ ✏️  edit  src/components/App.tsx  +3 -1   │  ← header (always visible)
│─────────────────────────────────────────────│
│  4  │ - │ const old = "value";              │  ← line 1
│  4  │ + │ const updated = "new";            │  ← line 2
│  5  │ + │ const extra = "added";            │  ← line 3
│  6  │   │ // context line                   │  ← line 3.65 (partial, blurred)
│  ░░░░░░░░░░░ gradient fade ░░░░░░░░░░░░░░░ │  ← blur mask
└─────────────────────────────────────────────┘
```

**Blur/Fade Effect:**
```css
.diff-collapsed::after {
  content: "";
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 24px;  /* covers ~1.2 lines */
  background: linear-gradient(transparent, var(--bg-elevated));
  pointer-events: none;
}
```

Container height for 3.65 lines: `3.65 * 20px (line-height) = 73px`

**Diff Computation (reuse from SimpleDiffView):**
- Extract `computeDiff` and `computeLCS` from `src/components/diff/SimpleDiffView.tsx` into `DiffToolCallView.utils.ts`
- For Edit: `computeDiff(old_string, new_string)` → diff lines
- For Write (new file): all content lines → additions
- For Write (overwrite): `computeDiff(diff_context.old_content, content)` → diff lines

**Rendering (reuse patterns from SimpleDiffView):**
- Same line background colors: `rgba(52, 199, 89, 0.12)` additions, `rgba(255, 69, 58, 0.12)` deletions
- Same line number styling
- Same prefix characters (+/-/space)
- Same monospace font

**Header (reuse ToolCallIndicator patterns):**
- Same wrapper background: `hsl(220 10% 14%)`
- Same chevron, icon, name badge layout
- Add stats badge: `+N -M` in green/red
- File path from arguments

### Task 3: Frontend — ToolCallIndicator Integration (Final Render)

**File:** `src/components/Chat/ToolCallIndicator.tsx`

Detect Edit/Write and delegate:
```typescript
export const ToolCallIndicator = React.memo(function ToolCallIndicator({ toolCall, className }) {
  const isEditOrWrite = ["edit", "write"].includes(toolCall.name.toLowerCase());

  if (isEditOrWrite) {
    return <DiffToolCallView toolCall={toolCall} className={className} />;
  }

  // ... existing generic rendering
});
```

This handles both:
- Historical messages (content blocks with persisted diff_context)
- Streaming tool calls that get promoted to final messages

### Task 4: Frontend — Streaming Integration (Live Render)

**File:** `src/components/Chat/ChatMessageList.tsx`

Modify the Footer component to split streaming tool calls:

```typescript
// In Footer:
const diffToolCalls = streamingToolCalls.filter(
  tc => ["edit", "write"].includes(tc.name.toLowerCase()) && tc.arguments !== null
);
const otherToolCalls = streamingToolCalls.filter(
  tc => !["edit", "write"].includes(tc.name.toLowerCase()) || tc.arguments === null
);

return (
  <div className="px-3 pb-3 w-full" style={contentContainerStyle}>
    {streamingText && <MessageItem ... />}

    {/* Diff views for Edit/Write — shown individually */}
    {diffToolCalls.map(tc => (
      <DiffToolCallView key={tc.id} toolCall={tc} isStreaming className="mb-2" />
    ))}

    {/* Aggregated indicator for remaining tools */}
    {(isSending || isAgentRunning) && (
      otherToolCalls.length > 0 ? (
        <StreamingToolIndicator toolCalls={otherToolCalls} isActive />
      ) : !streamingText && diffToolCalls.length === 0 ? (
        <TypingIndicator />
      ) : null
    )}

    <div ref={messagesEndRef} />
  </div>
);
```

**File:** `src/hooks/useIntegratedChatEvents.ts`

Enhance the `agent:tool_call` handler to pass through `diff_context`:
```typescript
bus.subscribe("agent:tool_call", (payload) => {
  // ... existing logic
  setStreamingToolCalls((prev) => [
    ...prev,
    {
      id: `streaming-agent-${Date.now()}-${prev.length}`,
      name: tool_name,
      arguments: args,
      result,
      diffContext: payload.diff_context,  // NEW: pass through
    },
  ]);
});
```

### Task 5: Frontend — Diff Utilities

**File:** `src/components/Chat/DiffToolCallView.utils.ts`

Extract from `SimpleDiffView.tsx`:
- `computeDiff(oldContent, newContent): DiffLine[]`
- `computeLCS(oldLines, newLines): Match[]`
- Line type helpers: `getLineBackground`, `getLineNumColor`, `getLinePrefix`, `getPrefixColor`

Add new helpers:
- `extractEditDiff(toolCall): { lines: DiffLine[], filePath: string, additions: number, deletions: number }`
- `extractWriteDiff(toolCall): { lines: DiffLine[], filePath: string, additions: number, deletions: number }`
- `isDiffToolCall(name: string): boolean`

### Task 6: Type Updates

**File:** `src/components/Chat/ToolCallIndicator.tsx` — Add `diffContext` to ToolCall interface:
```typescript
export interface ToolCall {
  id: string;
  name: string;
  arguments: unknown;
  result?: unknown;
  error?: string;
  diffContext?: {           // NEW
    oldContent?: string;
    filePath: string;
  };
}
```

**File:** `src/api/chat.ts` — Update `parseToolCalls` and `parseContentBlocks` to extract diff_context

## Implementation Order (Enhanced with Dependencies)

### Task 1: Backend — DiffContext types + ToolCall/ContentBlockItem field (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(stream-processor): add DiffContext struct and diff_context field to ToolCall`

**Files:** `src-tauri/src/infrastructure/agents/claude/stream_processor.rs`

- Add `DiffContext` struct (with `Serialize`, `Deserialize`)
- Add `diff_context: Option<DiffContext>` to `ToolCall` struct
- Add `diff_context: Option<serde_json::Value>` to `ContentBlockItem::ToolUse` variant
- **CRITICAL:** Update ALL construction sites for `ToolCall` (lines 306-311, 370-375) to include `diff_context: None`
- **CRITICAL:** Update ALL construction sites for `ContentBlockItem::ToolUse` (lines 316-321, 379-383) to include `diff_context: None`
- Update tests that construct `ToolCall` (line 762-766)

**Compilation check:** `cargo check` — all construction sites updated in same file.

### Task 2: Backend — Payload update + Streaming capture (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(chat-streaming): capture old file content for Edit/Write diff context`

**Files:**
- `src-tauri/src/application/chat_service/chat_service_types.rs` — Add `diff_context` field to `AgentToolCallPayload`
- `src-tauri/src/application/chat_service/chat_service_streaming.rs` — At `ToolCallCompleted`, read old file content for Edit/Write, attach to payload emission

**Why merged:** `chat_service_streaming.rs` sets `diff_context` on `AgentToolCallPayload`. If the field doesn't exist on the struct yet, it won't compile. Both files must be in the same task.

**Compilation check:** `cargo check` after both files updated.

### Task 3: Frontend — Type updates (ToolCall, ContentBlockItem, parseToolCalls, parseContentBlocks) (BLOCKING)
**Dependencies:** None (additive, optional fields)
**Atomic Commit:** `feat(chat-types): add diffContext field to ToolCall and ContentBlockItem types`

**Files:**
- `src/components/Chat/ToolCallIndicator.tsx` — Add optional `diffContext?` to `ToolCall` interface
- `src/components/Chat/MessageItem.tsx` — Add optional `diffContext?` to `ContentBlockItem` interface
- `src/api/chat.ts` — Update `parseToolCalls` to extract `diff_context` → `diffContext`, update `parseContentBlocks` to preserve `diff_context`

**NOTE:** Plan originally omitted `ContentBlockItem` update in MessageItem.tsx — content blocks carry `diff_context` in persisted JSON too.

**Compilation check:** `npm run typecheck` — all fields are optional additions, won't break existing code.

### Task 4: Frontend — Diff utilities (BLOCKING)
**Dependencies:** Task 3 (needs `ToolCall` type with `diffContext`)
**Atomic Commit:** `feat(chat-diff): extract diff computation utilities from SimpleDiffView`

**Files:**
- `src/components/Chat/DiffToolCallView.utils.ts` (NEW) — Extract `computeDiff`, `computeLCS`, line helpers from `SimpleDiffView.tsx`; add `extractEditDiff`, `extractWriteDiff`, `isDiffToolCall`

**Compilation check:** `npm run typecheck` — new file, additive.

### Task 5: Frontend — DiffToolCallView component (BLOCKING)
**Dependencies:** Task 3, Task 4
**Atomic Commit:** `feat(chat-diff): add DiffToolCallView component with collapse/expand and gradient blur`

**Files:**
- `src/components/Chat/DiffToolCallView.tsx` (NEW) — Core diff display component (~200 lines)

**Compilation check:** `npm run typecheck` — new file, imports from Task 3 and Task 4.

### Task 6: Frontend — ToolCallIndicator integration + Streaming integration
**Dependencies:** Task 5
**Atomic Commit:** `feat(chat-diff): wire DiffToolCallView into ToolCallIndicator and streaming footer`

**Files:**
- `src/components/Chat/ToolCallIndicator.tsx` — Detect Edit/Write, delegate to `DiffToolCallView`
- `src/components/Chat/ChatMessageList.tsx` — Split Edit/Write from streaming tool calls in Footer
- `src/hooks/useIntegratedChatEvents.ts` — Pass `diff_context` from event payload through to streaming state

**Why merged:** Original Tasks 3, 4, and 9 (ToolCallIndicator integration, streaming split, event handler) all modify different files but form a single "wiring" unit. They can be done atomically since each file modification is independent, but they're logically connected — streaming integration needs `DiffToolCallView` imported in `ChatMessageList`, and the event handler needs the `diffContext` field on `ToolCall`.

**Compilation check:** `npm run typecheck && npm run lint`

### Dependency Graph

```
Task 1 (Backend: types)
    ↓
Task 2 (Backend: streaming capture)

Task 3 (Frontend: types) ←── can run parallel with Tasks 1-2
    ↓
Task 4 (Frontend: utilities)
    ↓
Task 5 (Frontend: DiffToolCallView)
    ↓
Task 6 (Frontend: wiring)
```

Backend (Tasks 1→2) and Frontend (Tasks 3→4→5→6) are independent tracks that can be developed in parallel.

## Compilation Unit Notes

| Issue | Resolution |
|-------|------------|
| `ToolCall` struct gains new field | All 4 construction sites in `stream_processor.rs` updated in Task 1 |
| `AgentToolCallPayload` gains new field | Struct definition + emission site in same Task 2 |
| `ContentBlockItem` TS interface gains field | Optional field — additive, won't break existing code |
| `ToolCall` TS interface gains field | Optional field — additive, won't break existing code |
| `parseToolCalls` extracts new field | Maps `diff_context` → `diffContext` — existing callers unaffected (field is optional) |

## Edge Cases

| Case | Handling |
|------|----------|
| Edit with 1-2 line diff | Show fully (no blur needed) |
| Write to new file (no old content) | All lines shown as additions |
| Write to existing file | Old content from diff_context → proper diff |
| Very large Write (1000+ lines) | Collapsed to 3.65 lines, "Show all" button |
| Tool call error | Fall back to existing ToolCallIndicator error view |
| No file_path in arguments | Fall back to existing generic view |
| Non-task context (ideation chat) | Use project.working_directory for old content |
| Worktree mode | Use task.worktree_path for file reads |
| Local mode | Use project.working_directory for file reads |
| File doesn't exist yet (new file) | diff_context.old_content = None → all additions |
| Binary file | Skip diff, show generic view |

## Verification

### Manual Testing
1. Start dev server: `npm run dev:web`
2. Open ideation chat → trigger agent that uses Edit/Write
3. Verify:
   - Edit tool calls show diff with red/green lines
   - Write tool calls show content (all green for new files)
   - Collapsed view shows ~3.65 lines with gradient blur
   - Clicking expands to full diff
   - Header shows file path + additions/deletions count
   - During streaming: Edit/Write appear as separate diff cards, other tools stay aggregated
4. Test git modes:
   - Local mode project → verify old content is read from working directory
   - Worktree mode task → verify old content is read from worktree path

### Automated Testing
- `DiffToolCallView.test.tsx` — Unit tests for rendering, collapse/expand, blur effect
- `DiffToolCallView.utils.test.ts` — Diff computation edge cases
- Backend: Rust tests for DiffContext capture and path resolution

### Linting
- `npm run lint && npm run typecheck` for frontend changes
- `cargo clippy --all-targets --all-features -- -D warnings && cargo test` for backend changes

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
