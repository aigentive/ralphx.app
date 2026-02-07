# RalphX - Phase 91: Chat Diff View for Edit/Write Tool Calls

## Overview

Edit/Write tool calls in the chat UI currently render as generic collapsed cards showing raw JSON arguments. This phase adds specialized diff-based views inspired by Claude Code's CLI output: a ~3.65 line preview with gradient blur fade, collapsible to show the full diff, with real-time display during streaming broken out of the aggregated StreamingToolIndicator. Diff data is persisted in content_blocks JSON (no migration needed) and supports both Local and Worktree git modes.

**Reference Plan:**
- `specs/plans/chat_diff_view_for_edit_write_tool_calls.md` - Full architecture, component design, diff computation, streaming integration, and edge cases

## Goals

1. Render Edit/Write tool calls as inline diff cards with red/green line highlighting
2. Show ~3.65 lines collapsed preview with gradient blur fade, expandable to full diff
3. Break Edit/Write out of aggregated StreamingToolIndicator during streaming for real-time individual display
4. Persist diff context (old file content) in content_blocks JSON for branch-surviving history

## Dependencies

### Phase 87 (Real-Time Message Persistence + Streaming Display) - Required

| Dependency | Why Needed |
|------------|------------|
| `agent:chunk` streaming events | DiffToolCallView renders alongside streaming text in Footer |
| Content blocks persistence | diff_context is stored as optional field in content_blocks JSON |

### Phase 88 (Consolidate Legacy Events) - Required

| Dependency | Why Needed |
|------------|------------|
| Unified `agent:tool_call` event | Single event path for tool call streaming with diff_context payload |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/chat_diff_view_for_edit_write_tool_calls.md`
2. Understand the architecture and component structure
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
- Backend track (Tasks 1-2) and Frontend track (Tasks 3-6) can run in parallel

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/chat_diff_view_for_edit_write_tool_calls.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add DiffContext struct and diff_context field to ToolCall and ContentBlockItem::ToolUse in stream_processor.rs",
    "plan_section": "Task 1: Backend — Diff Context Capture (stream_processor.rs changes)",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(stream-processor): add DiffContext struct and diff_context field to ToolCall",
    "steps": [
      "Read specs/plans/chat_diff_view_for_edit_write_tool_calls.md section 'Task 1: Backend — Diff Context Capture'",
      "Add DiffContext struct with Serialize, Deserialize derives and fields: old_content (Option<String>), file_path (String)",
      "Add diff_context: Option<DiffContext> with #[serde(skip_serializing_if = \"Option::is_none\")] to ToolCall struct",
      "Add diff_context: Option<serde_json::Value> with #[serde(skip_serializing_if = \"Option::is_none\")] to ContentBlockItem::ToolUse variant",
      "CRITICAL: Update ALL ToolCall construction sites (lines ~306-311, ~370-375) to include diff_context: None",
      "CRITICAL: Update ALL ContentBlockItem::ToolUse construction sites (lines ~316-321, ~379-383) to include diff_context: None",
      "Update test_tool_call_serialization test (line ~762) to include diff_context: None",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(stream-processor): add DiffContext struct and diff_context field to ToolCall"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add diff_context to AgentToolCallPayload and capture old file content at ToolCallCompleted for Edit/Write",
    "plan_section": "Task 1: Backend — Diff Context Capture (chat_service_streaming.rs and chat_service_types.rs changes)",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(chat-streaming): capture old file content for Edit/Write diff context",
    "steps": [
      "Read specs/plans/chat_diff_view_for_edit_write_tool_calls.md section 'Task 1: Backend — Diff Context Capture'",
      "In chat_service_types.rs: Add diff_context: Option<serde_json::Value> with #[serde(skip_serializing_if = \"Option::is_none\")] to AgentToolCallPayload",
      "In chat_service_streaming.rs ToolCallCompleted handler: Check if tool_call.name is Edit or Write (case-insensitive)",
      "If Edit/Write: Extract file_path from tool_call.arguments, read old file content via std::fs::read_to_string",
      "Set tool_call.diff_context = Some(DiffContext { old_content, file_path })",
      "Update AgentToolCallPayload emission to include diff_context: serde_json::to_value(&tool_call.diff_context).ok().flatten()",
      "Also update the ContentBlockItem pushed to content_blocks to carry diff_context",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(chat-streaming): capture old file content for Edit/Write diff context"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add diffContext field to ToolCall and ContentBlockItem types, update parseToolCalls and parseContentBlocks",
    "plan_section": "Task 6: Type Updates",
    "blocking": [4, 5, 6],
    "blockedBy": [],
    "atomic_commit": "feat(chat-types): add diffContext field to ToolCall and ContentBlockItem types",
    "steps": [
      "Read specs/plans/chat_diff_view_for_edit_write_tool_calls.md section 'Task 6: Type Updates'",
      "In src/components/Chat/ToolCallIndicator.tsx: Add optional diffContext?: { oldContent?: string; filePath: string } to ToolCall interface",
      "In src/components/Chat/MessageItem.tsx: Add optional diffContext?: { oldContent?: string; filePath: string } to ContentBlockItem interface",
      "In src/api/chat.ts parseToolCalls(): Map tc.diff_context to diffContext with snake_case→camelCase transform",
      "In src/api/chat.ts parseContentBlocks(): Preserve diff_context field on tool_use blocks, transform to camelCase",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat-types): add diffContext field to ToolCall and ContentBlockItem types"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Extract diff computation utilities from SimpleDiffView into DiffToolCallView.utils.ts",
    "plan_section": "Task 5: Frontend — Diff Utilities",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(chat-diff): extract diff computation utilities from SimpleDiffView",
    "steps": [
      "Read specs/plans/chat_diff_view_for_edit_write_tool_calls.md section 'Task 5: Frontend — Diff Utilities'",
      "Create src/components/Chat/DiffToolCallView.utils.ts",
      "Extract from SimpleDiffView.tsx: computeDiff, computeLCS, DiffLine and Match interfaces",
      "Extract line helpers: getLineBackground, getLineNumColor, getLinePrefix, getPrefixColor",
      "Add new helpers: extractEditDiff(toolCall), extractWriteDiff(toolCall), isDiffToolCall(name)",
      "extractEditDiff: compute diff from old_string→new_string in arguments",
      "extractWriteDiff: if diffContext.oldContent exists, compute diff; else all lines are additions",
      "isDiffToolCall: returns true for 'edit' or 'write' (case-insensitive)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat-diff): extract diff computation utilities from SimpleDiffView"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Create DiffToolCallView component with collapsed preview, gradient blur, and expand/collapse",
    "plan_section": "Task 2: Frontend — DiffToolCallView Component",
    "blocking": [6],
    "blockedBy": [3, 4],
    "atomic_commit": "feat(chat-diff): add DiffToolCallView component with collapse/expand and gradient blur",
    "steps": [
      "Read specs/plans/chat_diff_view_for_edit_write_tool_calls.md section 'Task 2: Frontend — DiffToolCallView Component'",
      "Create src/components/Chat/DiffToolCallView.tsx (~200 lines)",
      "Props: toolCall: ToolCall, isStreaming?: boolean, className?: string",
      "Header: chevron + tool icon + name badge + file path + stats badge (+N -M in green/red)",
      "Collapsed state: show ~3.65 lines (73px height at 20px line-height) with gradient blur overlay",
      "Gradient blur: absolute positioned ::after pseudo-element with linear-gradient(transparent, var(--bg-elevated))",
      "Expanded state: full diff with line numbers, red/green backgrounds from utils",
      "For Edit: use extractEditDiff from utils",
      "For Write: use extractWriteDiff from utils",
      "If <4 lines: show fully, no blur needed. If error or no file_path: fall back to null (let parent render generic)",
      "Reuse ToolCallIndicator visual patterns: hsl(220 10% 14%) background, same chevron/icon/badge layout",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat-diff): add DiffToolCallView component with collapse/expand and gradient blur"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Wire DiffToolCallView into ToolCallIndicator, split streaming tool calls in ChatMessageList footer, pass diff_context in events",
    "plan_section": "Task 3: Frontend — ToolCallIndicator Integration + Task 4: Frontend — Streaming Integration",
    "blocking": [],
    "blockedBy": [5],
    "atomic_commit": "feat(chat-diff): wire DiffToolCallView into ToolCallIndicator and streaming footer",
    "steps": [
      "Read specs/plans/chat_diff_view_for_edit_write_tool_calls.md sections 'Task 3' and 'Task 4'",
      "In ToolCallIndicator.tsx: At top of component, detect Edit/Write via isDiffToolCall helper, if true return <DiffToolCallView>",
      "If DiffToolCallView returns null (error/no file_path), fall through to existing generic rendering",
      "In ChatMessageList.tsx Footer: Split streamingToolCalls into diffToolCalls (edit/write with args) and otherToolCalls",
      "Render diffToolCalls as individual <DiffToolCallView> cards above the StreamingToolIndicator",
      "Pass otherToolCalls to <StreamingToolIndicator> instead of all streamingToolCalls",
      "In useIntegratedChatEvents.ts: Update agent:tool_call subscription type to include diff_context",
      "Map payload.diff_context to diffContext (snake→camel) when constructing streaming tool call objects",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat-diff): wire DiffToolCallView into ToolCallIndicator and streaming footer"
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
| **Diff computed on frontend, not backend** | Edit tool calls already have old_string/new_string in arguments — no backend work needed. Only Write needs backend help (old file content). Keeps backend minimal. |
| **diff_context as optional JSON field in content_blocks** | No migration needed. Optional field on existing JSON blob means old data is unaffected. New data gets diff_context automatically. |
| **Reuse SimpleDiffView diff algorithm** | Proven LCS-based diff already exists. Extract rather than rewrite. |
| **Break Edit/Write out of StreamingToolIndicator** | Each file operation deserves its own visual card with diff preview. Other tools stay aggregated since they're informational. |
| **3.65 lines collapsed height** | Matches Claude Code CLI visual density. Shows enough context to understand the change without overwhelming. |
| **Gradient blur fade** | Smooth visual hint that more content exists below. Better than a hard cutoff. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] DiffContext serialization/deserialization roundtrip
- [ ] ToolCall with diff_context: None serializes without diff_context key (skip_serializing_if)
- [ ] ToolCall with diff_context: Some(...) serializes correctly

### Frontend - Run `npm run test`
- [ ] DiffToolCallView renders Edit tool call with diff lines
- [ ] DiffToolCallView renders Write tool call (new file) with all-green lines
- [ ] DiffToolCallView collapsed state shows ~3.65 lines with blur
- [ ] DiffToolCallView expand/collapse toggle works
- [ ] extractEditDiff produces correct additions/deletions counts
- [ ] extractWriteDiff handles new file (no old content) case
- [ ] isDiffToolCall returns true for "edit", "Edit", "write", "Write"

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Edit tool calls show diff with red/green lines in ideation chat
- [ ] Write tool calls show content (all green for new files)
- [ ] Collapsed view shows ~3.65 lines with gradient blur fade
- [ ] Clicking header expands to full diff with line numbers
- [ ] Header shows file path + additions/deletions count
- [ ] During streaming: Edit/Write appear as separate diff cards above aggregated tool indicator
- [ ] Other streaming tools (bash, grep, read) stay in aggregated StreamingToolIndicator
- [ ] Historical messages with persisted diff_context render correctly on page load

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] ToolCallIndicator detects Edit/Write and delegates to DiffToolCallView
- [ ] DiffToolCallView is imported AND rendered (not behind disabled flag)
- [ ] ChatMessageList Footer splits diff tool calls from aggregated list
- [ ] useIntegratedChatEvents passes diff_context through to streaming state
- [ ] parseToolCalls extracts diff_context from backend response
- [ ] parseContentBlocks preserves diff_context on tool_use blocks

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
