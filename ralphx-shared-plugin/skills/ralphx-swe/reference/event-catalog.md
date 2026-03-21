<!-- Source: ralphx-external-mcp/src/tools/events.ts | Last synced: 2026-03-21 -->

# RalphX Event Catalog

All pipeline events emitted by RalphX. Events arrive via webhook (`POST`) or polling (`v1_get_recent_events`). Each event has a discriminated `event_type` field. All events include `project_id` and `timestamp` (ISO 8601 UTC).

States referenced below → see `state-machine.md`.

---

## Task Events

### `task:created`
A new task entered the backlog.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | ID of the new task |
| `title` | string | Human-readable task title |

**Agent reaction:** Index the task. Optionally call `v1_list_tasks` to fetch full details and dependencies.

---

### `task:status_changed`
A task transitioned between pipeline states.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |
| `old_status` | string | Prior state (see state-machine.md) |
| `new_status` | string | Current state |

**Agent reaction:** Branch on `new_status`:
- `Executing` → worker agent is now active; monitor for `task:step_completed` events
- `PendingReview` → review queue updated; expect `review:ready` shortly
- `PendingMerge` → merge queue updated; expect `merge:ready` shortly
- `Merged` → task complete; update downstream dependency tracking
- `Escalated` → human triage required; surface in attention dashboard

---

### `task:step_completed`
An individual execution step within a task finished.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |
| `step_id` | string | Step identifier |
| `step_title` | string | Human-readable step name |

**Agent reaction:** Track execution progress. If monitoring task completion, count completed steps against total steps from `v1_list_tasks`.

---

### `task:execution_started`
A worker agent began executing the task (state: `Executing`).

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |

**Agent reaction:** Note start time for SLA tracking. Await `task:step_completed` events for granular progress.

---

### `task:execution_completed`
The worker agent finished execution; task will transition away from `Executing`.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |

**Agent reaction:** Expect `task:status_changed` with `new_status: PendingReview` (or `Escalated` on failure). If status change does not arrive within 60s, call `v1_get_attention_items` to check for failure.

---

## Review Events

### `review:ready`
Task entered `PendingReview`; reviewer agent is queued.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |

**Agent reaction:** Await `review:approved`, `review:changes_requested`, or `review:escalated`. No action required unless implementing custom review routing.

---

### `review:approved`
Reviewer approved the task; it will advance to `PendingMerge`.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |

**Agent reaction:** Update task tracking. Expect `merge:ready` shortly. Begin any post-approval preparation (e.g., deployment notifications).

---

### `review:changes_requested`
Reviewer rejected the task; it returns to `ReExecuting`.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |

**Agent reaction:** Log re-execution cycle count. If cycle count exceeds threshold, flag for human oversight. Expect `task:execution_started` as worker resumes.

---

### `review:escalated`
Reviewer escalated the task to human attention. Task enters `Escalated` state.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |
| `reason` | string? | Optional escalation reason |

**Agent reaction (CRITICAL):** Surface immediately in attention dashboard. Log `reason` if present. Task is blocked until a human intervenes — do NOT auto-retry or auto-approve. See `v1_get_attention_items`.

---

## Merge Events

### `merge:ready`
Task entered `PendingMerge`; merger agent is queued.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |

**Agent reaction:** Await `merge:completed` or `merge:conflict`. No action required unless implementing custom merge routing.

---

### `merge:completed`
Merger agent successfully merged the task branch. Task enters `Merged` state.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |

**Agent reaction:** Mark task complete in tracking. Unblock any tasks that listed this task as a dependency (`blocked_by`). Trigger downstream deployment or notification workflows.

---

### `merge:conflict`
Merger agent encountered a conflict requiring human resolution.

| Field | Type | Notes |
|-------|------|-------|
| `task_id` | string | — |
| `source_branch` | string | Task branch |
| `target_branch` | string | Merge target (usually `main`) |
| `conflict_files` | string[] | Files with conflicts |
| `strategy` | string | Merge strategy attempted |

**Agent reaction (CRITICAL):** Surface in attention dashboard with `conflict_files` listed. Do NOT auto-resolve — merge conflicts require human judgment. Task is stuck until manually resolved.

---

## Ideation Events

### `ideation:session_created`
A new ideation chat session was created.

| Field | Type | Notes |
|-------|------|-------|
| `session_id` | string | — |

**Agent reaction:** Register session in tracking. Await `ideation:plan_created` or user-driven activity.

---

### `ideation:plan_created`
The orchestrator agent created a plan artifact in the session.

| Field | Type | Notes |
|-------|------|-------|
| `session_id` | string | — |

**Agent reaction:** Await `ideation:verified` before acting on the plan. Plans are not implementation-ready until verified.

---

### `ideation:verified`
The plan passed the adversarial verification loop and is marked `Verified`.

| Field | Type | Notes |
|-------|------|-------|
| `session_id` | string | — |

**Agent reaction:** Plan is implementation-ready. Await `ideation:proposals_ready` for the concrete task list.

---

### `ideation:proposals_ready`
The orchestrator finalized proposals in the session. Tasks are ready to be created.

| Field | Type | Notes |
|-------|------|-------|
| `session_id` | string | — |
| `proposal_count` | number | Number of finalized proposals |

**Agent reaction:** Call `v1_list_tasks` or inspect session proposals to enumerate the new tasks. If auto-propose is enabled, expect `ideation:auto_propose_sent` or `ideation:auto_propose_failed`.

---

### `ideation:auto_propose_sent`
Auto-propose pipeline successfully created tasks from finalized proposals.

| Field | Type | Notes |
|-------|------|-------|
| `session_id` | string | — |

**Agent reaction:** Tasks now exist in the backlog. Expect `task:created` events for each new task. Update dependency tracking.

---

### `ideation:auto_propose_failed`
Auto-propose pipeline failed to create tasks.

| Field | Type | Notes |
|-------|------|-------|
| `session_id` | string | — |
| `error` | string | Failure reason |

**Agent reaction:** Log `error`. Surface to human for manual intervention — proposals exist but tasks were not created. User may need to manually trigger task creation from the session.

---

## System Events

### `system:webhook_unhealthy`
A registered webhook failed ≥10 consecutive delivery attempts and was deactivated.

| Field | Type | Notes |
|-------|------|-------|
| `webhook_id` | string | Deactivated webhook ID |
| `failure_count` | number | Total consecutive failures |

**Agent reaction (CRITICAL):** Switch to polling fallback (`v1_get_recent_events`) immediately. Investigate webhook endpoint availability. Re-register the same URL via `v1_register_webhook` to reactivate (idempotent — resets failure count, preserves secret).

---

### `system:rate_limit_warning`
An API key is approaching its rate limit threshold.

| Field | Type | Notes |
|-------|------|-------|
| `api_key_id` | string | The key approaching the limit |

**Agent reaction:** Reduce request frequency. Implement exponential backoff. If multiple agents share the key, coordinate to distribute load or use separate keys per agent.

---

## Quick Reference

| Event | Category | Key Fields | State Transition |
|-------|----------|-----------|-----------------|
| `task:created` | Task | `task_id`, `title` | → Backlog |
| `task:status_changed` | Task | `old_status`, `new_status` | any → any |
| `task:step_completed` | Task | `step_id`, `step_title` | within Executing |
| `task:execution_started` | Task | `task_id` | → Executing |
| `task:execution_completed` | Task | `task_id` | Executing → PendingReview |
| `review:ready` | Review | `task_id` | → PendingReview |
| `review:approved` | Review | `task_id` | → PendingMerge |
| `review:changes_requested` | Review | `task_id` | → ReExecuting |
| `review:escalated` | Review | `task_id`, `reason?` | → Escalated |
| `merge:ready` | Merge | `task_id` | → PendingMerge |
| `merge:completed` | Merge | `task_id` | → Merged |
| `merge:conflict` | Merge | `conflict_files`, `strategy` | stuck — human needed |
| `ideation:session_created` | Ideation | `session_id` | — |
| `ideation:plan_created` | Ideation | `session_id` | — |
| `ideation:verified` | Ideation | `session_id` | — |
| `ideation:proposals_ready` | Ideation | `session_id`, `proposal_count` | — |
| `ideation:auto_propose_sent` | Ideation | `session_id` | — |
| `ideation:auto_propose_failed` | Ideation | `session_id`, `error` | — |
| `system:webhook_unhealthy` | System | `webhook_id`, `failure_count` | — |
| `system:rate_limit_warning` | System | `api_key_id` | — |

**CRITICAL events requiring immediate human attention:** `review:escalated`, `merge:conflict`, `system:webhook_unhealthy`
