# Plan: Chat Tool-Call Parity + Path/Content Normalization + Streaming Order Fix

## Summary
Use both DB fixtures (`ea29067d-12f5-4b2b-9070-25da96461325`, `52e775db-23bb-4af4-a19f-2edbce416e49`) as acceptance references, and deliver one batch that:
1. finishes widget parity for uncovered tool calls (`Skill`),
2. normalizes file/search/read rendering (repo-relative paths, cleaned read output, fixed no-match behavior),
3. renders user messages as markdown,
4. removes streaming duplicate/out-of-order behavior during live runs.

Observed tool-use coverage in those conversations:
- Covered by specialized renderers: `Read`, `Grep`, `Glob`, `Bash`, `Task`, `mcp__ralphx__*`.
- Not covered: `Skill` (currently generic fallback).
- Main rendering defects come from current parsing/normalization, not missing DB data.

## Implementation Details

### 1) Add shared normalization/parsing utilities
Files:
- `src/components/Chat/tool-widgets/shared.constants.ts`
- `src/components/Chat/tool-widgets/shared.constants.test.ts`

Changes:
1. Add `normalizeDisplayPath(path: string): string`
- Converts `\\` to `/`.
- Removes leading absolute prefix to show repo-relative where possible, by anchoring from first known project segment:
  - `src`, `src-tauri`, `tests`, `specs`, `scripts`, `docs`, `mockups`, `assets`, `public`.
- Removes leading `.../` or `/.../` artifacts.
- Falls back to original basename-tail when no anchor exists.

2. Fix `shortenPath(...)`
- Never produce `/.../`.
- Work on normalized path only.

3. Add search parser:
- `parseSearchResult(result: unknown): { paths: string[]; isEmpty: boolean; note?: string }`
- Handles:
  - result metadata lines (`Found N files`, pagination lines),
  - explicit no-result lines (`No matches found`, `No files found`, `No files matched`),
  - content lines with `path:line:match` shape (extract path portion),
  - dedupe + normalize path outputs.

4. Add read-output parser:
- `parseReadOutput(result: unknown, offset?: number): { lines: string[]; inferredStartLine: number; error?: string }`
- Removes tool-added line prefixes like `   500→`.
- Preserves actual code indentation after the arrow.
- Extracts `<tool_use_error>...</tool_use_error>` into clean error text.

### 2) Update Read/Grep/Glob widgets to use new parser layer
Files:
- `src/components/Chat/tool-widgets/ReadWidget.tsx`
- `src/components/Chat/tool-widgets/GrepWidget.tsx`
- `src/components/Chat/tool-widgets/GlobWidget.tsx`
- `src/components/Chat/tool-widgets/GrepWidget.test.tsx`
- `src/components/Chat/tool-widgets/GlobWidget.test.tsx`
- `src/components/Chat/tool-widgets/ReadWidget.test.tsx` (new)

Changes:
1. `ReadWidget`
- Header path: normalize to repo-relative, then shorten.
- Body: render parsed lines without duplicated line-number prefixes from tool output.
- Start line: prefer explicit `offset`; else inferred from stripped prefix.
- Error rendering: show cleaned error text (no XML wrapper tags).

2. `GrepWidget` + `GlobWidget`
- Replace `parseToolResultAsLines` usage with `parseSearchResult`.
- File count badge uses parsed `paths.length` only.
- No-match behavior:
  - keep card header/badge,
  - move low-emphasis note outside card body (muted, small text) to avoid “no matches” appearing as a file row.
- Normalize listed paths to repo-relative.

### 3) Add Skill widget for parity
Files:
- `src/components/Chat/tool-widgets/SkillWidget.tsx` (new)
- `src/components/Chat/tool-widgets/registry.ts`
- `src/components/Chat/tool-widgets/index.ts` (if needed by exports)
- `src/components/Chat/tool-widgets/SkillWidget.test.tsx` (new)

Changes:
- New specialized widget for `Skill` tool calls:
  - Header: skill icon + `skill` argument (`ralphx:rule-manager` style) + status badge.
  - Body: result text preview (collapsed/expandable via `WidgetCard`).
  - Error state badge/content mirrors existing widget style.
- Register `"skill": SkillWidget` in widget registry.

### 4) Render user messages with markdown
Files:
- `src/components/Chat/TextBubble.tsx`
- `src/components/Chat/MessageItem.test.tsx` or `src/components/Chat/TextBubble.test.tsx` (new)

Changes:
- Remove plain `<p>` path for user messages.
- Use same `ReactMarkdown` + `remarkGfm` + `markdownComponents` pipeline for both user and assistant.
- Preserve existing bubble styling and copy behavior.

### 5) Fix streaming duplicate/out-of-order display
Files:
- `src/hooks/useAgentEvents.ts`
- `src/hooks/useIntegratedChatEvents.ts`
- `src/hooks/useChat.test.ts` or dedicated hook tests (new as needed)

Changes:
1. `useAgentEvents` (`agent:message_created`)
- Keep optimistic append only for `role === "user"`.
- For assistant message-created events, do not append synthetic message; only invalidate/refetch conversation.

2. `useIntegratedChatEvents` (`agent:message_created`)
- Read `role` when present.
- If event is assistant message for active conversation/context:
  - clear `streamingText`, `streamingToolCalls`, `streamingTasks` immediately,
  - invalidate conversation query.
- Keep `agent:run_completed` cleanup as safety fallback.

Result:
- No temporary duplicate assistant message + streaming footer.
- Stable ordering while streaming and after completion.

## Public APIs / Interfaces / Types
1. Internal tool-widget utility API additions:
- `normalizeDisplayPath(...)`
- `parseSearchResult(...)`
- `parseReadOutput(...)`

2. Event payload usage update:
- `agent:message_created` now consumed with optional `role` in `useIntegratedChatEvents` (backend contract already provides it; frontend begins using it).

No backend schema/database migrations required.

## Test Cases and Scenarios

### Widget parsing/format tests
1. Read parser strips `N→` prefixes and preserves code indentation.
2. Read parser infers start line from prefixed output when offset absent.
3. Read parser extracts text from `<tool_use_error>...`.
4. Search parser ignores metadata lines (`Found N files`, pagination).
5. Search parser correctly treats `No matches/files found` as empty (0 files).
6. Search parser extracts file path from `path:line:match` entries.
7. Path normalization converts absolute workspace paths to repo-relative.

### Component behavior tests
1. `ReadWidget` shows normalized relative header path and clean code lines.
2. `GrepWidget`/`GlobWidget` badges match real file count; no-match note rendered muted outside card body.
3. `SkillWidget` renders `skill` arg + result/error.
4. `TextBubble` renders markdown for user message (e.g., headings, list, code span).

### Streaming behavior tests
1. Assistant `agent:message_created` does not add optimistic duplicate message.
2. Assistant `agent:message_created` clears live streaming footer state.
3. User `agent:message_created` still appends immediately for responsiveness.
4. Conversation invalidation still occurs on message-created and run-completed.

### Manual verification using your two conversation fixtures
1. Open `ea29067d...`:
- `Skill` call renders specialized card.
- `Read` cards have clean content (no extra prefixed line numbers/arrows).
- Paths appear repo-relative, not `/.../`.
- `Grep/Glob` no longer show “No matches found” as file count/body mismatch.
2. Open `52e775db...`:
- same path/markdown/widget behavior consistency.
3. Start a live run:
- no duplicate assistant entries during stream,
- message order remains stable before and after completion.

## Assumptions and Defaults
1. Repo-relative display is derived via path-segment anchoring in frontend (no backend path injection needed).
2. Existing visual style from current widget system (`WidgetCard`, `WidgetHeader`, `Badge`) remains unchanged.
3. `Task` tool remains handled by `TaskToolCallCard` (no redesign in this pass).
4. Streaming correctness is solved in frontend event handling without changing backend event emission order.
