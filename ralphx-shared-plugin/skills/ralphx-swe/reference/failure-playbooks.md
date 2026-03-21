# Failure Playbooks

Step-by-step recovery for the 5 most common failure modes. Each playbook uses real `v1_` tool calls with valid parameters.

---

## Playbook 1 — `accept_plan_and_schedule` Fails Mid-Saga

**Symptom:** `v1_accept_plan_and_schedule` returns an error or partial result; some tasks may not have been created.

**Root cause:** The saga applies proposals and schedules tasks in sequence. A network hiccup or backend timeout mid-flight leaves the session in a partial state.

**Recovery:**

```typescript
// 1. Do NOT retry v1_accept_plan_and_schedule — it may double-create tasks.
//    Use the idempotent resume tool instead.

// 2. Resume from last successful step
const result = await mcp.call('v1_resume_scheduling', {
  session_id: 'sess-abc123',   // the ideation session that failed
});

// 3. Verify tasks were created
if (result.success) {
  console.log(`Scheduled: ${result.task_ids.join(', ')}`);
} else {
  // Backend unreachable — wait and retry v1_resume_scheduling
  console.error(result.message);
}

// 4. Confirm tasks entered backlog
const { events } = await mcp.call('v1_get_recent_events', {
  project_id: 'proj-xyz',
  cursor: lastSeenCursor,
  limit: 20,
});
// Look for ideation:proposals_ready or task:created events
```

**Key:** `v1_resume_scheduling` is safe to call multiple times — `apply_proposals` is idempotent on the backend.

---

## Playbook 2 — Webhook Unhealthy (`system:webhook_unhealthy`)

**Symptom:** Receive `system:webhook_unhealthy` event, or events stop arriving and `v1_get_webhook_health` shows `active: false`.

**Root cause:** 10 consecutive delivery failures deactivate the webhook automatically.

**Recovery:**

```typescript
// 1. Confirm webhook is deactivated
const health = await mcp.call('v1_get_webhook_health');
const broken = health.webhooks.filter(w => !w.active);
// broken[0] → { id: 'wh-ghi789', failure_count: 10, active: false }

// 2. Re-register the SAME URL — idempotent, reactivates without new secret
const reg = await mcp.call('v1_register_webhook', {
  url: 'http://127.0.0.1:18789/hooks/ralphx',  // same URL as before
  // event_types omitted → receive all events
});
// Returns existing webhook_id, resets failure_count to 0, active: true
// Secret is NOT regenerated — keep using original secret for HMAC verification

// 3. Backfill missed events using cursor
const { events, next_cursor } = await mcp.call('v1_get_recent_events', {
  project_id: 'proj-xyz',
  cursor: lastSeenCursor,   // cursor stored before outage
  limit: 100,
});
for (const event of events) {
  await dispatcher.dispatch(event);
  lastSeenCursor = event.id;
}
```

**Key:** Re-registering the same URL never creates a duplicate — it returns the existing `webhook_id` with a reset failure count.

---

## Playbook 3 — Task Stuck in Blocked State

**Symptom:** `task:status_changed` event shows `new_status: "Blocked"`, or `v1_get_task_detail` returns `status: "Blocked"`.

**Root cause:** Task has unresolved dependency tasks that haven't been merged yet.

**Recovery:**

```typescript
// 1. Inspect the blocked task
const detail = await mcp.call('v1_get_task_detail', {
  task_id: 'task-abc123',
});
// Check detail.blocked_by — list of dependency task IDs

// 2. Check status of each blocker
const { tasks } = await mcp.call('v1_batch_task_status', {
  task_ids: detail.blocked_by,   // up to 50 IDs
});

for (const blocker of tasks) {
  if (blocker.status === 'Failed' || blocker.status === 'Stopped') {
    // Dependency failed — annotate and retry
    await mcp.call('v1_create_task_note', {
      task_id: blocker.task_id,
      note: `Retrying: was blocking task-abc123. Status was ${blocker.status}.`,
    });
    await mcp.call('v1_retry_task', { task_id: blocker.task_id });
  }
  // If blocker.status === 'Executing' or 'PendingReview' → wait, it will unblock naturally
  // If blocker.status === 'Merged' → task should auto-unblock; wait for next event cycle
}

// 3. No action needed if all blockers are progressing — task auto-unblocks when
//    dependencies merge. Monitor via task:status_changed events.
```

**Key:** Never force-unblock by cancelling dependencies. Only retry genuinely failed blockers.

---

## Playbook 4 — Rate Limit (HTTP 429)

**Symptom:** MCP tool call returns `{ "error": "backend_error", "status": 429 }`.

**Root cause:** Token bucket exhausted — default limit is 10 req/s per API key.

**Recovery:**

```typescript
async function callWithBackoff<T>(
  tool: string,
  args: Record<string, unknown>,
  maxRetries = 4,
): Promise<T> {
  let delay = 1000;  // start at 1s
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    const result = await mcp.call(tool, args);
    if (typeof result === 'object' && result !== null &&
        'error' in result && result.error === 'backend_error' &&
        'status' in result && result.status === 429) {
      if (attempt === maxRetries) throw new Error(`${tool} rate-limited after ${maxRetries} retries`);
      await sleep(delay + Math.random() * 200);  // jitter prevents thundering herd
      delay = Math.min(delay * 2, 30_000);       // cap at 30s
      continue;
    }
    return result as T;
  }
  throw new Error('unreachable');
}

// Usage — same as normal tool call
const detail = await callWithBackoff('v1_get_task_detail', { task_id: 'task-abc123' });
```

**Retry schedule:** 1s → 2s → 4s → 8s (with ±200ms jitter). Cap at 30s for sustained outages.

**Prevention:** Batch reads with `v1_batch_task_status` (up to 50 tasks per call) instead of looping `v1_get_task_detail`.

---

## Playbook 5 — Ideation Agent Idle / Not Responding

**Symptom:** No `ideation:plan_created` or `ideation:auto_propose_sent` event after several minutes; `v1_get_ideation_status` shows agent state is not `generating`.

**Root cause:** The orchestrator agent may be waiting for input, or the session stalled after the initial prompt.

**Recovery:**

```typescript
// 1. Check session state
const status = await mcp.call('v1_get_ideation_status', {
  session_id: 'sess-abc123',
});
// status.agent_status: 'idle' | 'generating' | 'waiting_for_input'
// status.proposal_count: number of proposals drafted so far

if (status.agent_status === 'waiting_for_input' || status.agent_status === 'idle') {
  // 2. Prompt the agent to continue
  await mcp.call('v1_send_ideation_message', {
    session_id: 'sess-abc123',
    message: 'Please continue drafting the implementation plan and finalize the proposals.',
  });
}

// 3. If proposals exist but plan hasn't been created, trigger verification
if (status.proposal_count > 0) {
  const plan = await mcp.call('v1_get_plan', { session_id: 'sess-abc123' });
  if (plan.status !== 'verified') {
    await mcp.call('v1_trigger_plan_verification', { session_id: 'sess-abc123' });
  }
}

// 4. Poll for response (webhooks preferred — fall back to polling)
// Wait for ideation:plan_created or ideation:verified event
// If no response after 5 minutes, send another v1_send_ideation_message nudge
```

**Key:** `v1_send_ideation_message` is safe to call on any agent state — it queues the message if the agent is between turns.
