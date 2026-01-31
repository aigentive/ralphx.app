# Plan: Activity Screen UI Improvements

## Context

Phase 48 implemented persistent activity events with:
- **Data stored**: task_id/session_id, internal_status, event_type, role, content, metadata, created_at
- **Filtering**: Type filter, status filter (9 statuses), search
- **Infinite scroll**: Cursor-based pagination with TanStack Query

## Problems to Solve

1. **JSON Display Issue**: Tool results and call arguments show as escaped JSON strings (`\n`, `\"`)
2. **Missing Context Display**: No visual indication of where event originated (task vs session)
3. **Missing Role Display**: Role (agent/system/user) not shown on events
4. **No Role Filter**: Backend supports but no UI
5. **File Size**: ActivityView.tsx is 974 lines (exceeds 400 line limit)

## User Decisions

1. **Thinking blocks**: Render as markdown (headers, code blocks, lists)
2. **Role filter**: Add to UI (agent/system/user)
3. **Refactor**: Extract sub-components as part of this work
4. **Context display**: Show source (task/session) with contextual links

---

## Implementation Plan

### Task 1: Extract Sub-Components (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `refactor(activity): extract sub-components from ActivityView`

**Rationale**: Extracting first makes adding features cleaner.

**New file structure**:
```
src/components/activity/
├── ActivityView.tsx          (~300 lines, main component)
├── ActivityMessage.tsx       (~200 lines, message display with smart rendering)
├── ActivityFilters.tsx       (~150 lines, all filter components)
├── ActivityContext.tsx       (~80 lines, context/source display component)
├── ActivityView.utils.ts     (~100 lines, utilities)
├── ActivityView.types.ts     (~60 lines, types)
└── index.ts                  (re-export)
```

**Files to create**:
- `src/components/activity/ActivityView.types.ts` - UnifiedActivityMessage, ViewMode, filter types
- `src/components/activity/ActivityView.utils.ts` - getMessageColor, getMessageIcon, highlightJSON, formatTimestamp, safeJsonParse
- `src/components/activity/ActivityMessage.tsx` - Message display with smart content rendering
- `src/components/activity/ActivityFilters.tsx` - ViewModeToggle, FilterTabs, StatusFilter, RoleFilter, SearchBar
- `src/components/activity/ActivityContext.tsx` - Source/origin badge with contextual link
- `src/components/activity/index.ts` - Re-export

**Files to modify**:
- `src/components/activity/ActivityView.tsx` - Import from new files, reduce to ~300 lines

---

### Task 5: Safe JSON Parsing (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(activity): add safe JSON parsing utility`

**Problem**: Unguarded `JSON.parse()` can crash on malformed data.

**Solution**: Wrap in try-catch everywhere JSON is parsed:
- `toUnifiedMessage()` for metadata
- Content parsing for tool_result events

```tsx
function safeJsonParse(str: string): { data: unknown; error: boolean } {
  try {
    return { data: JSON.parse(str), error: false };
  } catch {
    return { data: str, error: true };
  }
}
```

**Files to modify**:
- `src/components/activity/ActivityView.utils.ts`
- `src/components/activity/ActivityMessage.tsx`

---

### Task 2: Add Context/Source Display
**Dependencies:** Task 1
**Atomic Commit:** `feat(activity): add context/source display with role badge`

**Problem**: User can't see where an event originated (which task or ideation session).

**Solution**: Add `ActivityContext` component that displays:
- **Icon**: Task icon or Ideation icon
- **Label**: Task title or Session name (if available) or just "Task" / "Session"
- **Link**: Click to navigate to the source
- **Role badge**: Agent / System / User indicator

**Visual design**:
```
┌─────────────────────────────────────────────────────────────┐
│ [🔧] Read                    thinking     Ready    12:45:32 │
│ ├─ Context: Task: "Implement login"  •  Agent                │
│ └─ [expanded content...]                                     │
└─────────────────────────────────────────────────────────────┘
```

**Implementation in ActivityMessage header**:
```tsx
// Below the type/status/timestamp row
<div className="flex items-center gap-2 text-xs text-[var(--text-muted)]">
  <ActivityContext
    taskId={message.taskId}
    sessionId={message.sessionId}
    role={message.role}
  />
</div>
```

**ActivityContext component**:
```tsx
function ActivityContext({ taskId, sessionId, role }: Props) {
  // Fetch task/session name if needed (or use passed prop)
  const label = taskId ? `Task: ${taskId.slice(0,8)}...` : `Session: ${sessionId?.slice(0,8)}...`;
  const icon = taskId ? <CheckSquare /> : <MessageSquare />;
  const roleLabel = role === 'agent' ? 'Agent' : role === 'system' ? 'System' : 'User';

  return (
    <div className="flex items-center gap-2">
      <span className="flex items-center gap-1">
        {icon}
        <button onClick={() => navigate(taskId ? `/tasks/${taskId}` : `/ideation/${sessionId}`)}>
          {label}
        </button>
      </span>
      <span className="px-1.5 py-0.5 rounded bg-[var(--bg-base)]">{roleLabel}</span>
    </div>
  );
}
```

**Files to modify**:
- `src/components/activity/ActivityContext.tsx` (create)
- `src/components/activity/ActivityMessage.tsx` (integrate)
- `src/components/activity/ActivityView.types.ts` (add role to UnifiedActivityMessage)

---

### Task 3: Add Role Filter UI
**Dependencies:** Task 1
**Atomic Commit:** `feat(activity): add role filter UI`

**Problem**: Backend supports role filtering but no UI control.

**Solution**: Add `RoleFilter` component to filter bar:
- Dropdown or pill selector: All / Agent / System / User
- Multi-select like type filter
- Wire to `historicalFilter.roles` state

**Files to modify**:
- `src/components/activity/ActivityFilters.tsx` - Add RoleFilter
- `src/components/activity/ActivityView.tsx` - Wire role filter state

---

### Task 4: Smart Content Rendering
**Dependencies:** Task 1, Task 5
**Atomic Commit:** `feat(activity): add smart content rendering for tool results and thinking blocks`

**Problem**: Tool results show as escaped JSON strings.

**Solution**: Type-specific rendering in `ActivityMessage`:

1. **`tool_result` events**:
   - Parse content as JSON
   - Display with syntax highlighting in collapsible block
   - Fallback to plain text if parse fails

2. **`tool_call` events**:
   - Show tool name as header badge (already done)
   - Render arguments from metadata as formatted JSON
   - Don't show raw `"Read ({...})"` string as content

3. **`thinking` events**:
   - Render as markdown using existing `react-markdown` + `remark-gfm`
   - Reuse `markdownComponents` from `src/components/Chat/MessageItem.markdown.tsx`
   - Support headers, code blocks, lists, bold/italic, tables

4. **`text` / `error` events**:
   - Keep as plain text with whitespace preserved

**No new dependencies needed** - `react-markdown` and `remark-gfm` already installed.

**Files to modify**:
- `src/components/activity/ActivityMessage.tsx`
- `src/components/activity/ActivityView.utils.ts` (add safeJsonParse)

---

## Critical Files

| File | Action |
|------|--------|
| `src/components/activity/ActivityView.tsx` | Refactor: extract, reduce to ~300 lines |
| `src/components/activity/ActivityMessage.tsx` | Create: smart rendering + context display |
| `src/components/activity/ActivityFilters.tsx` | Create: all filters including role |
| `src/components/activity/ActivityContext.tsx` | Create: source/origin badge with link |
| `src/components/activity/ActivityView.utils.ts` | Create: utilities |
| `src/components/activity/ActivityView.types.ts` | Create: types |
| `src/components/Chat/MessageItem.markdown.tsx` | Reuse: existing markdown components |

---

## Execution Order

1. **Task 1**: Extract sub-components (foundation for clean feature work)
2. **Task 5**: Safe JSON parsing (prevents crashes)
3. **Task 2**: Context/source display with role badge
4. **Task 3**: Role filter UI
5. **Task 4**: Smart content rendering (main feature)

---

## Verification

1. Run app: `npm run tauri dev`
2. Navigate to Activity screen with existing data
3. Verify:
   - **Context display**: Each event shows source (Task/Session) with clickable link
   - **Role display**: Agent/System/User badge on each event
   - **Role filter**: Can filter by role
   - **Tool results**: Show as formatted JSON (not escaped strings)
   - **Tool calls**: Show tool name + formatted arguments
   - **Thinking blocks**: Render as markdown
   - **Status filter**: Still works
   - **Infinite scroll**: Still works
   - **No crashes**: Malformed JSON doesn't break UI
4. Run linters: `npm run lint && npm run typecheck`

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
