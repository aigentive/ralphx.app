---
name: ralphx-swe
description: >
  RalphX autonomous agent playbook. Teaches external agents pipeline navigation:
  ideation → execution → review → merge. Covers 24-state machine, decision trees
  for edge cases, webhook-first event patterns, failure recovery playbooks.
  Use when connecting to RalphX via External MCP API or managing pipeline tasks.
argument-hint: "[section: quick-start | state-machine | decisions | events | recovery | dos-donts]"
---

# RalphX SWE Skill — Autonomous Pipeline Playbook

> **Skill vs Guide:** Call `v1_get_agent_guide` for tool schemas and sequencing rules. This skill
> teaches **judgment**: when to act vs observe, how to handle edge cases, what events mean for
> decision-making. Zero overlap with the guide's API reference content.

---

## 1. Bootstrap — Start Here Every Session

Three steps before taking any pipeline action:

**Step 1 — Backfill missed events** (catch up on events since last run):
```
v1_get_recent_events(project_id, cursor: lastSeenCursor, limit: 100)
→ Process events in order. Advance cursor after each batch.
→ If this is a fresh start, use cursor: 0 to get recent events.
```

**Step 2 — Check attention items** (find what needs action right now):
```
v1_get_attention_items(project_id)
→ Returns escalated_reviews, failed_tasks, merge_conflicts.
→ Address these before doing anything else.
```

**Step 3 — Load tool reference** (for argument schemas and preconditions):
```
v1_get_agent_guide()  → or v1_get_agent_guide(section: "pipeline") for supervision tools
```

> **Webhook registration is automatic.** `RalphxWebhookRegistrar` handles registration at startup.
> You do NOT need to call `v1_register_webhook` to begin receiving events. If the webhook becomes
> unhealthy (`system:webhook_unhealthy` event), re-register the same URL to reactivate it — see
> Do's and Don'ts table below.

---

## 2. Core Principles

**Observe before act.** Most pipeline states are agent-active (executing, reviewing, merging). Your
role in those states is to watch, not interrupt. Only take action in waiting states
(`review_passed`, `escalated`, `qa_failed`, `merge_conflict`, `merge_incomplete`).

**Webhook-first.** Your webhook is auto-registered — events arrive as HTTP POST without any
setup call. Use `v1_get_recent_events` with a cursor as fallback when the webhook is unhealthy.
Polling task status in a tight loop is the last resort — it wastes rate limit budget.

**Annotate before you intervene.** Before calling any action tool, call `v1_create_task_note` with
your reasoning. This creates an audit trail for humans reviewing your decisions.

**Always pair UUIDs with titles.** Any task ID you surface in a log, message, or annotation must
include the human-readable title. Use `v1_get_task_detail` or `v1_batch_task_status` to resolve
titles. Format: `task-{id} ({Title})`.

**Human merge gate is NON-NEGOTIABLE.** Tasks in `review_passed` require a human to call
`v1_approve_review`. You MUST NOT auto-approve tasks without explicit human delegation. This is
the single hardest rule in this playbook — never violate it.

**Conservative intervention.** When uncertain, annotate and wait. A wrong auto-action (retrying a
non-transient failure, sending a message to a generating agent) causes more damage than a brief
delay. Escalate to human attention when in doubt.

**v1_resolve_escalation requires delegation.** The tool exists, but you MUST NOT call it unless a
human has explicitly granted you escalation resolution authority for this task. Default behavior on
`review:escalated` is: annotate + alert human.

---

## 3. Do's and Don'ts

| Situation | ✅ Do | ❌ Don't |
|-----------|-------|---------|
| Task in `review_passed` | Call `v1_approve_review` or `v1_request_changes` (human decision required) | Auto-approve without human confirmation |
| Task in `escalated` | Annotate + alert human; call `v1_resolve_escalation` ONLY if human explicitly delegated | Auto-resolve escalation |
| `review:escalated` event arrives | `v1_get_task_detail` → `v1_create_task_note` → alert human | Call `v1_approve_review` on escalated task (requires `review_passed` or `escalated` — but still needs human) |
| Task in `merge_conflict` | Annotate with conflict files + alert human | Call `v1_retry_task` (resets branch context) |
| Task in `blocked` | Call `v1_get_task_detail` to inspect blocker; notify human if human-input block | Cancel the blocked task |
| `system:webhook_unhealthy` received | Re-register same URL: `v1_register_webhook(url: SAME_URL)` — reactivates, preserves secret | Register a new URL (creates a duplicate) |
| `v1_accept_plan_and_schedule` fails | Call `v1_resume_scheduling(session_id)` to resume idempotently | Re-call `v1_accept_plan_and_schedule` (may double-create tasks) |
| Agent `agent_status: generating` | Wait 5-10s, then poll again | Send a message (it will be queued and may confuse agent state) |
| Rate limit 429 received | Exponential backoff: 1s → 2s → 4s → 8s with ±200ms jitter | Retry immediately or in a tight loop |
| Task in transient state (`pending_review`, `approved`, `pending_merge`) | Re-poll in 1-2s for settled state | Take any action — these states last milliseconds |
| Task in `paused` | Annotate + alert human; do NOT call `v1_retry_task` | Call `v1_retry_task` (resets to `ready`, loses pre-pause context) |
| Task iterating `reviewing → re_executing` | Log the cycle count; only intervene after ~5 cycles | Interrupt the loop — normal iteration is 2-3 cycles |
| Freshness conflict (`executing → merging`) | Observe; this is automatic rebasing, not a failure | Panic or annotate — it resolves to `ready` or `pending_review` automatically |
| QA fails (`qa_failed`) | Call `v1_get_task_detail` + surface to human; human decides: `v1_request_changes` or `v1_skip_qa` | Auto-retry QA without human review |

---

## 4. Quick Decision Guide

The 10 most common decision points:

**1. Event arrives: `task:status_changed` → `escalated`**
→ `v1_get_task_detail(task_id)` → `v1_create_task_note(task_id, "Escalated: <reason>")` → Alert human

**2. Event arrives: `review_passed`**
→ Surface in attention dashboard → Wait for human to call `v1_approve_review` or `v1_request_changes`

**3. Event arrives: `merge:conflict`**
→ `v1_get_task_detail(task_id)` for conflict files → `v1_create_task_note(task_id, "Merge conflict in: <files>")` → Alert human

**4. Event arrives: `system:webhook_unhealthy`**
→ Switch to `v1_get_recent_events` polling fallback → Re-register same URL → Resume event-driven mode

**5. `v1_get_attention_items` returns failed tasks**
→ `v1_get_task_detail(task_id)` for failure reason → Transient failure + count < 3 → `v1_retry_task`
→ Non-transient or count ≥ 3 → Annotate + alert human

**6. `v1_get_ideation_status` shows `agent_status: idle` unexpectedly**
→ `v1_send_ideation_message(session_id, "Please continue drafting the plan and finalize proposals.")`

**7. Plan verification not converging after 10 minutes**
→ `v1_get_plan(session_id)` for gap details → Alert human with gaps list → Await human guidance

**8. `v1_get_execution_capacity` returns `available: 0`**
→ Poll every 60s for capacity — do NOT call `v1_resume_scheduling` (not a capacity tool)
→ Capacity opens when a task reaches `merged`, `failed`, `cancelled`, or `stopped`

**9. Task stuck in `blocked` for extended period**
→ `v1_get_task_detail` → check `blocked_by` list → `v1_batch_task_status([blocker_ids])`
→ Blocker `failed`/`stopped` → annotate + consider `v1_retry_task` on blocker; blocker active → wait

**10. Ideation plan ready — should I accept?**
→ Only if: plan exists AND `v1_get_plan_verification` returns `Verified` status
→ Then: `v1_accept_plan_and_schedule(session_id)` → note returned task IDs

---

## 5. Reference Navigation

Four reference files live alongside this skill. Load them when you need depth on a specific topic.
All paths are relative to this skill file's directory (`skills/ralphx-swe/`):

| File | Contents | When to Load |
|------|----------|--------------|
| `reference/state-machine.md` | All 24 pipeline states, transition table, happy paths, behavioral patterns | Confused by a state transition; need precise state semantics |
| `reference/decision-trees.md` | 7 ASCII decision trees for common scenarios | Handling escalation, merge conflict, blocked task, failed task, webhook outage |
| `reference/event-catalog.md` | All 20 event types, fields, and recommended reactions | Unsure how to react to a specific event; need event field names |
| `reference/failure-playbooks.md` | 5 step-by-step recovery procedures with real `v1_` tool calls | Recovering from accept failure, webhook deactivation, blocked task, rate limits, idle agent |

These files are fully self-contained. No external dependencies. No internet required.

---

## 6. Section Dispatch

When this skill is invoked with an argument, load the corresponding content:

If `$ARGUMENTS` contains `state-machine`: Read the file `reference/state-machine.md` in the same
directory as this skill file and present its full content to the user.

If `$ARGUMENTS` contains `decisions`: Read the file `reference/decision-trees.md` in the same
directory as this skill file and present its full content to the user.

If `$ARGUMENTS` contains `events`: Read the file `reference/event-catalog.md` in the same
directory as this skill file and present its full content to the user.

If `$ARGUMENTS` contains `recovery`: Read the file `reference/failure-playbooks.md` in the same
directory as this skill file and present its full content to the user.

If `$ARGUMENTS` contains `dos-donts`: Present Section 3 (Do's and Don'ts) of this skill file in
full, with the complete table of situations and recommended actions.

If `$ARGUMENTS` contains `quick-start`: Present Sections 1 (Bootstrap) and 4 (Quick Decision
Guide) of this skill file in full, as a fast onboarding reference.

If `$ARGUMENTS` is empty or not recognized: Present the full skill content above (Sections 1–5),
then remind the user that `/ralphx-shared-plugin:ralphx-swe [section]` can be used to load a
specific reference file.
