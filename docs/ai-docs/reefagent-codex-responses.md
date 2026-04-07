# Reefagent Codex Response Shapes

Status: initial research snapshot on 2026-04-07 from `~/.reefagent/sandbox/logs/codex-events/`.

## Outer JSONL envelope

Reefagent wraps raw Codex stdout/stderr in its own JSONL records.

Observed top-level `kind` values:

- `run_started`
- `stdout_event`
- `run_finished`

Observed wrapper fields:

- `ts`
- `kind`
- `mode`
- `agentId`
- `sessionId`
- `model`
- `providerSessionId` sometimes present
- `reason` sometimes present
- `rawLine`
- parsed `event`
- `stderr`
- `parseError`
- `exitCode`

## Nested Codex event types

Observed nested `event.type` values:

- `thread.started`
- `turn.started`
- `item.started`
- `item.completed`
- `item.updated`
- `turn.completed`
- parent-session collaboration items also appear as `collab_tool_call`

Observed completion payload:

- `turn.completed.usage.input_tokens`
- `turn.completed.usage.cached_input_tokens`
- `turn.completed.usage.output_tokens`

## Observed item shapes

### `agent_message`

Representative shape:

```json
{
  "type": "item.completed",
  "item": {
    "id": "item_0",
    "type": "agent_message",
    "text": "..."
  }
}
```

Notes:

- This is the main assistant-text shape found in the Reefagent corpus.
- Some flows also include assistant-style `message` payloads in Reefagent parser code.

### `mcp_tool_call`

Representative started/completed shape:

```json
{
  "type": "item.started",
  "item": {
    "id": "item_1",
    "type": "mcp_tool_call",
    "server": "reefagent",
    "tool": "respond",
    "arguments": { "response": "..." },
    "result": null,
    "error": null,
    "status": "in_progress"
  }
}
```

```json
{
  "type": "item.completed",
  "item": {
    "id": "item_1",
    "type": "mcp_tool_call",
    "server": "reefagent",
    "tool": "respond",
    "arguments": { "response": "..." },
    "result": {
      "content": [{ "type": "text", "text": "{\"recorded\":true}" }],
      "structured_content": null
    },
    "error": null,
    "status": "completed"
  }
}
```

Failure pattern:

- `result: null`
- `error.message` populated

### `command_execution`

Observed pattern:

- starts in progress with `exit_code: null`
- completes or fails with `aggregated_output`, `exit_code`, and terminal `status`

RalphX implication:

- Codex command/tool execution should normalize into the same UI event family as Claude tool-call rows, but the raw payload is vendor-specific.

### `error`

Observed failure pattern:

- surfaced as an `item.completed` whose `item.type` is `error`
- other failures also appear inside `mcp_tool_call.error` or nonzero `command_execution.exit_code`

RalphX implication:

- do not assume one provider-level stderr string is the only failure channel

### Collaboration / delegation

Observed parent-session collaboration items:

- `collab_tool_call` with `tool: "spawn_agent"`
- `collab_tool_call` with `tool: "wait"`

Observed fields:

- `sender_thread_id`
- `receiver_thread_ids`
- `prompt`
- `agents_states`
- `status`

Observed child-session naming:

- `parentSession:delegate:codex:1`
- `parentSession:delegate:codex:2`

RalphX implication:

- Codex subagent / delegation support is visible in raw events and should be modeled explicitly in the provider-neutral event layer

### Prompt-sidecar metadata

Separate prompt logs such as `codex-stream-effective-*.txt` include:

- provider
- `isResume`
- `providerSessionId`
- `reason`

RalphX implication:

- prompt capture and raw event logs should stay linked but separate
- prompt capture is useful for audit/debug, but it is not a substitute for raw event logging

## What Reefagent normalizes today

Confirmed parser surface from `src/core/codex-jsonl.ts`:

- assistant messages
- tool calls
- tool results
- message suffix de-duplication

It recognizes:

- `item.completed` with `item.type == "agent_message"`
- `item.started` / `item.completed` `mcp_tool_call`
- `item.function_call`, `item.function_call.delta`, `item.function_call_output`
- some assistant `message` payload variants

## What is not present in the sampled corpus

- token-by-token text deltas
- explicit sandbox mode in the raw event line
- explicit approval-policy in the raw event line

Those settings appear in nearby patch / config code, not in the raw event payload itself.

## Representative evidence paths

- `~/.reefagent/sandbox/logs/codex-events/2026-04-06T01-42-48-194Z-stream-telegram-default--1003691416971-7439-4c1ab832.jsonl`
- `~/.reefagent/sandbox/logs/codex-events/2026-04-06T02-34-44-871Z-stream-telegram-default--1003691416971-7439_delegate_codex_1-f5370f32.jsonl`
- `~/.reefagent/sandbox/logs/codex-events/2026-04-06T12-17-19-187Z-stream-telegram-default--1003787836738-101_delegate_codex_2-2544f535.jsonl`
- `~/.reefagent/sandbox/codex-cli-danger-full-access.patch`

## RalphX design implications

- Store raw provider events unchanged.
- Parse them into a normalized event stream for UI and orchestration consumers.
- Keep provider-session ids separate from RalphX run ids.
- Expect Codex failures to arrive through structured item payloads, not only stderr.
- Model child-session lineage explicitly for Codex subagent flows.
