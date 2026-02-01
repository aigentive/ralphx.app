# Activity Screen UX Improvement Plan

## Problem Summary

1. **Raw JSON Display**: Tool calls/results show unreadable raw JSON instead of human-friendly formatting
2. **Wrong Scroll Behavior**: History mode auto-scrolls to bottom (oldest) instead of staying at top (newest)
3. **Dense Layout**: Current card design makes it difficult to quickly scan and understand events

## Design Decisions (Confirmed)

| Decision | Choice |
|----------|--------|
| Live mode scroll | Stay at top, new events prepend |
| Event grouping | No grouping, flat list |
| Tool name display | Clean names (strip `mcp__ralphx__` prefix) |

## Implementation

### Task 1: Fix Scroll Behavior (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(activity): disable auto-scroll in history mode`

**File:** `src/components/activity/ActivityView.tsx`

**Changes:**
1. Initialize `autoScroll` based on view mode: `false` for history, `true` for live
2. Remove `messagesEndRef.scrollIntoView()` call for history mode
3. For live mode: Keep new events at top (prepend), maintain scroll position
4. Keep "Scroll to latest" button for manual navigation

```typescript
// Before
const [autoScroll, setAutoScroll] = useState(true);

// After
const [autoScroll, setAutoScroll] = useState(!isHistoricalMode);

// Modify scroll effect
useEffect(() => {
  // Only auto-scroll in live mode when user wants to follow
  if (!isHistoricalMode && autoScroll && messagesEndRef.current) {
    messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
  }
}, [filteredMessages.length, autoScroll, isHistoricalMode]);
```

### Task 2: Semantic Tool Call Rendering (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(activity): add semantic tool call rendering with clean names`

**Files:** `src/components/activity/ActivityMessage.tsx`, `src/components/activity/ActivityView.utils.ts`

**Current:** Shows raw JSON metadata
**Proposed:** Tool name header + formatted key-value arguments

```
┌──────────────────────────────────────────────────────┐
│ ⚡ get_task_steps                          01:20:35  │
│ ───────────────────────────────────────────────────  │
│   task_id  59dfb3d9-e463-45e9-a65d-d9da64b0583f     │
│                                        ▼ Raw JSON   │
└──────────────────────────────────────────────────────┘
```

**Changes:**
1. Add `cleanToolName()` helper to strip `mcp__ralphx__` prefix
2. Extract arguments from metadata, render as key-value pairs
3. Add collapsible "Raw JSON" section for debugging
4. Better visual hierarchy: tool name prominent, args indented

### Task 3: Semantic Tool Result Rendering
**Dependencies:** Task 2 (uses same utilities pattern)
**Atomic Commit:** `feat(activity): add semantic tool result rendering with preview`

**Files:** `src/components/activity/ActivityMessage.tsx`, `src/components/activity/ActivityView.utils.ts`

**Current:** Shows raw JSON with syntax highlighting
**Proposed:** Smart preview + expandable full result

```
┌──────────────────────────────────────────────────────┐
│ ✓ Result                                   01:20:35  │
│ ───────────────────────────────────────────────────  │
│   Task "Escalate test 1" with 3 steps loaded        │
│                                     ▼ Full Response │
└──────────────────────────────────────────────────────┘
```

**Changes:**
1. Add `generateResultPreview()` helper to create human-readable summary
2. Show success indicator (checkmark) by default, error indicator for failures
3. Truncate preview to ~100 chars with meaningful content
4. Expandable section shows full syntax-highlighted JSON

### Task 4: Markdown Support for Text Messages
**Dependencies:** None
**Atomic Commit:** `feat(activity): add markdown rendering for text messages`

**File:** `src/components/activity/ActivityMessage.tsx`

**Current:** Text messages rendered as plain `<p>` with `whitespace-pre-wrap`
**Proposed:** Use ReactMarkdown with existing `markdownComponents` from chat

**Changes:**
1. Reuse `markdownComponents` from `@/components/Chat/MessageItem.markdown` (already imported)
2. Render text messages with ReactMarkdown + remarkGfm (same as thinking blocks)
3. This enables: headers, lists, code blocks, links, tables in agent text output
4. Better typography via the shared markdown components

```typescript
// Before (text case)
case "text":
default: {
  return (
    <p className="text-sm text-[var(--text-primary)] whitespace-pre-wrap break-words mt-1">
      {truncatedContent}
    </p>
  );
}

// After (text case - same as thinking)
case "text":
default: {
  return (
    <div className="text-sm text-[var(--text-primary)] mt-1 prose-sm prose-invert max-w-none">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
        {truncatedContent}
      </ReactMarkdown>
    </div>
  );
}
```

**Note:** `markdownComponents` already includes styled code blocks, tables, lists, links with proper macOS Tahoe styling.

### Task 5: Visual Cleanup
**Dependencies:** Tasks 1-4 (applies polish after core changes)
**Atomic Commit:** `refactor(activity): improve visual hierarchy and reduce badge noise`

**Files:** `src/components/activity/ActivityMessage.tsx`, `src/components/activity/ActivityView.tsx`

**Changes:**
1. Reduce badge noise (fewer inline badges)
2. Better whitespace and padding
3. Timestamp right-aligned, subtle color
4. Consistent card styling across event types

## Files to Modify

| File | Size | Changes |
|------|------|---------|
| `ActivityView.tsx` | 483 LOC | Fix scroll behavior, mode-aware auto-scroll |
| `ActivityMessage.tsx` | 245 LOC | Semantic rendering for tool_call, tool_result |
| `ActivityView.utils.ts` | 224 LOC | Add cleanToolName(), generateResultPreview() |

## Verification

1. **History scroll**: Load Activity → History mode → Verify stays at top, no auto-scroll
2. **Live scroll**: Generate events → Verify new events appear at top without jarring scroll
3. **Tool call display**: Expand tool call → Verify clean name + formatted arguments
4. **Tool result display**: Expand result → Verify preview + expandable full JSON
5. **Visual comparison**: Screenshot before/after for readability improvement

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Graph

```
Task 1 (scroll) ─────────────────────┐
                                      │
Task 2 (tool call) ──→ Task 3 (result)│──→ Task 5 (visual cleanup)
                                      │
Task 4 (markdown) ───────────────────┘
```

**Parallel execution possible:** Tasks 1, 2, and 4 have no dependencies and can run concurrently.
**Sequential requirements:** Task 3 requires Task 2. Task 5 should run last.
