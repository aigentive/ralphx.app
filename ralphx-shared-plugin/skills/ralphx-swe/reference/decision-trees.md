<!-- Source: docs/external-mcp/autonomous-workflows.md | Last synced: 2026-03-21 -->

# Decision Trees — Common Scenarios

ASCII decision trees for common agent decision points. States → `state-machine.md`. Events → `event-catalog.md`. All tool names are fully qualified `v1_` names.

---

## 1. Review Escalated

Trigger: `review:escalated` webhook or `v1_batch_task_status` returns `escalated`

```
review:escalated received
│
├─ v1_get_task_detail(task_id)
│    ↓
│  escalation_reason present?
│    ├─ YES → v1_create_task_note(task_id, "Escalation: <reason>. Human review required.")
│    └─ NO  → v1_create_task_note(task_id, "Escalation received — no reason provided.")
│
├─ Alert human (dashboard / notification)
│    ↓
│  Human explicitly delegated resolution authority?
│    ├─ YES → v1_resolve_escalation(task_id)
│    │         state: escalated → approved | revision_needed
│    └─ NO  → ❌ DO NOT call v1_approve_review or v1_resolve_escalation
│              Wait for human to act in RalphX UI
```

**Rule:** Never auto-resolve escalations. Surface, annotate, and wait.

---

## 2. Merge Conflict

Trigger: `merge:conflict` webhook or task enters `merge_conflict` state

```
merge:conflict received
│
├─ v1_get_task_detail(task_id)
│    → note conflict_files, source_branch, strategy
│
├─ v1_create_task_note(task_id,
│    "Merge conflict in: <conflict_files>. Strategy: <strategy>. Human resolution required.")
│
├─ Alert human with conflict file list
│    ↓
│  Urge to auto-retry?
│    ├─ YES → ❌ DO NOT call v1_retry_task — resets branch context
│    └─ Correct → Wait for human to resolve conflict in RalphX UI
│                  state: merge_conflict → merged
│                  webhook fires: merge:completed
```

**Rule:** Merge conflicts require human judgment. Agents annotate and surface only.

---

## 3. Task Stuck / Blocked

Trigger: `task:status_changed` → `blocked`, or `v1_get_attention_items` returns blocked items

```
task enters blocked state
│
├─ v1_get_task_detail(task_id)
│    ↓
│  blocked_by dependency present?
│    ├─ YES → v1_batch_task_status([blocking_task_ids])
│    │           ↓
│    │         Blocking task in failed | cancelled | stopped?
│    │           ├─ YES → Alert human: dependency must be fixed or retried first
│    │           │         v1_create_task_note(task_id, "Blocked on <dep_id> which is <status>")
│    │           └─ NO  → Wait; poll blocking task every 60s
│    │                     state resolves when dependency reaches merged
│    │
│    └─ NO (human-input block)
│          → v1_create_task_note(task_id, "Blocked: awaiting human input")
│          → Alert human with context
│               ↓
│            Human removes block via UI
│            state: blocked → ready (auto-scheduled)
│            → v1_resume_scheduling(project_id)  [only if project scheduling was also paused]
```

---

## 4. Failed Task

Trigger: `task:status_changed` → `failed`

```
task enters failed state
│
├─ v1_get_task_detail(task_id)
│    → check failure context and re-execution history
│    ↓
│  Failure count < 3?
│    ├─ YES → Is failure transient (network timeout, capacity error)?
│    │           ├─ YES → v1_retry_task(task_id)
│    │           │         state: failed → ready → executing
│    │           └─ NO  → v1_create_task_note(task_id, "Non-transient failure: <context>")
│    │                     Alert human for diagnosis before retrying
│    │
│    └─ NO (≥ 3 failures)
│          → ❌ DO NOT retry without human review
│          → v1_create_task_note(task_id, "Repeated failures (≥3). Human diagnosis required.")
│          → Alert human with full failure history
```

**Rule:** Retry is only safe for isolated, transient failures. Repeated failure = human escalation.

---

## 5. Ideation Verification Not Converging

Trigger: `v1_get_plan_verification` returns non-`Verified` status beyond expected window

```
v1_trigger_plan_verification(session_id) called
│
├─ Poll v1_get_plan_verification(session_id) every 30s
│    ↓
│  status?
│    ├─ "Verified"   → v1_accept_plan_and_schedule(session_id)
│    │                  webhook fires: ideation:proposals_ready
│    │
│    ├─ "InProgress" → Continue polling (up to 10 min total)
│    │
│    ├─ "Failed"     → v1_get_plan(session_id) — inspect gap list
│    │                  v1_send_ideation_message(session_id, "Verification failed: <gaps>")
│    │                  v1_trigger_plan_verification(session_id)  [retry; max 2 retries]
│    │                       ↓
│    │                    Still "Failed" after 2 retries?
│    │                       → Alert human; provide plan + failure reason
│    │                       → ❌ DO NOT call v1_accept_plan_and_schedule on unverified plan
│    │
│    └─ Polling > 10 min without resolution
│          → v1_get_plan(session_id) — check plan state
│          → Alert human: "Verification not converging after 10 min"
│          → Await human guidance before proceeding
```

---

## 6. Webhook Unhealthy

Trigger: `system:webhook_unhealthy` event (≥10 consecutive delivery failures → webhook deactivated)

```
system:webhook_unhealthy received
│   (webhook_id deactivated; failure_count ≥ 10)
│
├─ Immediately switch to polling fallback
│    → v1_get_recent_events(project_id, cursor: lastSeenCursor, limit: 100)
│    → Process missed events in order; advance cursor after each
│
├─ v1_get_webhook_health(webhook_id)
│    → Confirm deactivated; note failure_count
│    ↓
│  Webhook endpoint reachable?
│    ├─ YES → v1_register_webhook(url: GATEWAY_URL)
│    │         [idempotent — resets failure count, preserves secret]
│    │         webhook reactivated → resume event-driven mode
│    │         stop polling fallback
│    │
│    └─ NO  → Continue polling v1_get_recent_events indefinitely
│              Alert ops: webhook endpoint unreachable
│              Fix endpoint → then v1_register_webhook to reactivate
```

**Rule:** Never drop events during outage. Cursor-based polling guarantees no gaps.

---

## 7. Capacity Exhausted

Trigger: `v1_get_execution_capacity` returns `available: 0`; tasks queue in `ready`

```
v1_get_execution_capacity(project_id)
│   → { used: N, max: N, available: 0 }
│
├─ v1_batch_task_status([all active task ids])
│    → identify slots consumed by: executing | re_executing | reviewing
│                                   merging | qa_refining | qa_testing
│    ↓
│  Any task exceeding SLA threshold (e.g., executing > 2h)?
│    ├─ YES → Is task in paused state?
│    │           ├─ YES → Alert human: paused task holding execution slot
│    │           │         ❌ DO NOT call v1_retry_task (resets to ready — loses context)
│    │           │         Human resumes or cancels via RalphX UI
│    │           └─ NO  → v1_create_task_note(task_id, "SLA exceeded — monitoring")
│    │                     Alert human if still stuck after 2× SLA threshold
│    │
│    └─ NO (capacity legitimately full — normal operations)
│          → Wait; poll v1_get_execution_capacity every 60s
│          → ❌ DO NOT call v1_resume_scheduling — not a capacity unblock mechanism
│               ↓
│            Slot opens (task reaches merged | failed | cancelled | stopped)
│            → Ready tasks auto-start — no agent action needed
```

**Rule:** Capacity is managed by RalphX scheduler. Agents observe; they do not force-start tasks.
