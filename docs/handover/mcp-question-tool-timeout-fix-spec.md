# MCP Question Tool Timeout Fix

## Problem

The ideation `ask_user_question` MCP tool is not actually unavailable. It is available, gets called, and works for short waits.

The real bug is that the current implementation blocks the MCP tool call longer than the effective runtime ceiling. Around 5 minutes, the tool fails with a transport error, and the agent falls back to plain text:

- Debug log: `Tool 'ask_user_question' failed after 301s: MCP error -32603: fetch failed`
- User-visible chat message: `MCP question tool unavailable. Let me ask directly:`

This leaves the inline question card in a stale state long enough for the user to answer it after the tool has already died, which is misleading and duplicative.

## Confirmed Evidence

Session used for confirmation:

- Ideation session: `c6d29b7b-6bf1-46c0-90b3-40b9c1767e12`
- Conversation: `d29377b2-a0f0-46a1-ab5c-2ffa8086524b`

Observed sequence:

1. `2026-03-17T19:24:09Z`
   The ideation team lead called `ask_user_question`.
   Source: `/var/folders/.../ralphx-claude-debug-55164-8963b530b467466e987ae9c2cdb06052.log`

2. `2026-03-17T19:29:10Z`
   The tool failed after about 301 seconds with `MCP error -32603: fetch failed`.
   Same debug log.

3. `2026-03-17T19:23:43.880272+00:00`
   The assistant message in `src-tauri/ralphx.db` contains:
   `MCP question tool unavailable. Let me ask directly:`

4. `2026-03-17T19:30:14+00:00`
   The original pending question was resolved in `pending_questions`, after the tool had already failed.
   Question: `When verification hits max rounds without converging, what should the terminal status be?`

This proves:

- The tool was allowed and invoked successfully.
- The failure is a long-poll timeout/transport issue, not an allowlist issue.
- The UI question stayed usable after the backing tool call had already died.

## Ruled Out

- Agent allowlist bug: ruled out. The current team-lead allowlist includes `ask_user_question`.
- Readonly prompt mismatch as primary cause: ruled out for the current bug. Current logs show `orchestrator-ideation-readonly` uses `orchestrator-ideation-readonly.md`.

## Root Cause

There are three mismatched timeout contracts:

1. MCP/tool contract says the question blocks for about 5 minutes.
   - `ralphx-plugin/ralphx-mcp-server/src/tools.ts`

2. MCP question handler waits 15 minutes client-side.
   - `ralphx-plugin/ralphx-mcp-server/src/question-handler.ts`

3. Backend question await waits 14 minutes.
   - `src-tauri/src/http_server/handlers/questions.rs`

In practice, the surrounding MCP/runtime stack kills long-running tool waits at about 300 seconds. Because our handler is waiting far longer than that, the call dies with a generic `fetch failed` instead of returning a structured timeout result.

Secondary bug:

- The frontend has no explicit expiry event for timed-out inline questions, so the question card can remain visible after the tool call is already gone.

## Fix Goals

1. The question tool must return a structured timeout result before the hidden runtime ceiling is hit.
2. Timed-out questions must be expired explicitly and removed from the UI.
3. The agent must stop seeing raw tool failures for this path.
4. The timeout contract must be consistent across MCP handler, backend handler, and user-facing descriptions.

## Implementation Plan

### 1. Canonicalize human-wait deadlines in MCP server

Add a small helper module for long-poll human waits:

- deadline just under the observed ceiling
- timeout-like error classification for `fetch failed` near the deadline
- reusable between question/team-plan bridges

Initial target:

- backend timeout: about `285s`
- MCP client timeout: about `290s`

This keeps the backend response ahead of the observed `~301s` runtime failure.

### 2. Fix `ask_user_question` MCP handler

File:

- `ralphx-plugin/ralphx-mcp-server/src/question-handler.ts`

Changes:

- use the canonical deadline helper instead of the current 15-minute wait
- convert timeout-like `fetch failed` cases into structured timeout JSON instead of throwing
- keep successful short waits unchanged

### 3. Expire timed-out questions in backend and emit an event

Files:

- `src-tauri/src/application/question_state.rs`
- `src-tauri/src/http_server/handlers/questions.rs`

Changes:

- add an explicit question-expire path that removes the in-memory waiter but preserves repo history as `expired`
- on timeout in `await_question`, emit `agent:question_expired` with `sessionId` and `requestId`
- stop using generic removal for timeout expiry paths

### 4. Clear stale question UI on expiry

File:

- `src/hooks/useAskUserQuestion.ts`

Changes:

- subscribe to `agent:question_expired`
- clear the active question only when `requestId` matches the currently displayed question

This prevents answering a card whose MCP wait is already gone.

### 5. Align similar team-plan timeout handling

Files:

- `ralphx-plugin/ralphx-mcp-server/src/team-plan-handler.ts`
- `src-tauri/src/http_server/handlers/teams.rs`

Reason:

- the same run showed `request_team_plan` failing after about 300s with the same `fetch failed` transport error
- current code still says 15m/14m, while the agent prompt already documents a 300s timeout

Changes:

- move team-plan wait onto the same canonical near-5-minute deadline
- return structured timeout payloads instead of hard tool errors
- update timeout copy to match the real deadline

## Constraints

- Do not treat this as an MCP allowlist problem.
- Do not keep any wait longer than the effective runtime ceiling.
- Do not leave the inline question card active after a timeout.
- Preserve successful short-wait question flow.
- Preserve repo auditability for expired questions.

## Avoid

- Raising the question timeout above 5 minutes again.
- Swallowing timeout failures without clearing UI state.
- Re-throwing raw `fetch failed` for deadline-driven question timeouts.
- Adding polling workarounds in the frontend.
- Introducing a broad chat-flow refactor unrelated to the timeout path.

## Proof Obligations

- `ask_user_question` answered within the deadline still resumes normally.
- `ask_user_question` not answered in time returns a structured timeout result, not a tool error.
- The agent no longer emits `MCP question tool unavailable` for this timeout path.
- A timed-out question card disappears when the backend expires it.
- A resolved question after normal flow still emits `agent:question_resolved`.
- `request_team_plan` timeout also returns a structured timeout result instead of `fetch failed`.
