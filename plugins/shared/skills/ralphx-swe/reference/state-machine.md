<!-- Source: .claude/rules/task-state-machine.md | Last synced: 2026-03-21 -->

# RalphX Pipeline State Machine

Full reference for all 24 task states visible via `v1_batch_task_status` and `task:status_changed` events.

> **Guide boundary:** The `v1_get_agent_guide` pipeline section shows a simplified stage flow. This document covers all 24 internal states, auto-transition mechanics, and external-agent behavioral rules.

---

## States by Category

### Idle (3 states)

| State | Meaning | External Action |
|-------|---------|----------------|
| `backlog` | Task exists, not yet scheduled | None — wait for scheduling |
| `ready` | Scheduled, queued for execution | None — system auto-starts when capacity available |
| `blocked` | Waiting on a dependency or human input | Check `v1_get_task_detail` for blocker info; notify human if human-input blocked |

### Active — Agent Running (6 states)

| State | Meaning | External Action |
|-------|---------|----------------|
| `executing` | Worker agent writing code | Observe only |
| `re_executing` | Worker agent revising code after review feedback | Observe only |
| `reviewing` | Reviewer agent evaluating code | Observe only |
| `merging` | Merger agent resolving conflicts | Observe only — do NOT retry merge manually |
| `qa_refining` | QA agent preparing test plan | Observe only |
| `qa_testing` | QA agent running tests | Observe only |

**Rule:** Never interrupt agent-active states. All 6 are gated by `max_concurrent` and counted against execution capacity.

### Transient — Auto-Transition Immediately (5 states)

| State | Auto-Transitions To | Why |
|-------|---------------------|-----|
| `pending_review` | `reviewing` | Reviewer agent spawned immediately when available |
| `qa_passed` | `pending_review` | QA success → hand off to review automatically |
| `revision_needed` | `re_executing` | Review requested changes → re-execution starts immediately |
| `approved` | `pending_merge` | Human approval triggers merge workflow immediately |
| `pending_merge` | `merged` (fast path) or `merging` | Programmatic merge attempted first; conflict → `merging` |

**Rule:** You will rarely observe transient states — they vanish in milliseconds. If `v1_batch_task_status` returns one, check status again shortly for the settled state.

### Waiting — Human Action Required (5 states)

| State | Meaning | Required Action |
|-------|---------|----------------|
| `review_passed` | AI review complete, passed — awaiting human approval | `v1_approve_review` or `v1_request_changes` |
| `escalated` | AI review needs human decision | `v1_resolve_escalation` — only if human explicitly delegated; otherwise alert human |
| `qa_failed` | QA tests failed | Human decides: fix (`v1_request_changes`) or manually skip QA via app UI |
| `merge_incomplete` | Merger agent errored mid-merge | Human investigates; `v1_get_task_detail` for context |
| `merge_conflict` | Merge conflicts could not be resolved | Human must resolve; do NOT retry automatically |

**Rule:** These states require human decisions. Agents annotate (`v1_create_task_note`) and notify. Never auto-resolve `escalated` or `merge_conflict`.

### Suspended (1 state)

| State | Meaning | External Action |
|-------|---------|----------------|
| `paused` | Agent was running; task manually suspended | Observe and wait — no external `v1_resume` tool exists; human resumes via UI |

**Note:** `paused` is NOT terminal. The pre-pause state is preserved internally. Tasks return to the active state they were in when the human resumes.

### Done — Terminal (4 states)

| State | Meaning | Re-enterable? |
|-------|---------|--------------|
| `merged` | Task code is on main branch | YES — can be re-queued via `Retry` (rare) |
| `failed` | Worker agent failed | YES — `v1_retry_task` → `ready` |
| `cancelled` | Task cancelled by user | YES — `v1_retry_task` → `ready` |
| `stopped` | Agent was hard-stopped (not recoverable without manual restart) | YES — `v1_retry_task` → `ready` |

**Rule:** `stopped` ≠ `paused`. Stopped tasks do NOT auto-resume. They require explicit `v1_retry_task` to re-enter the queue from `ready`.

---

## State Transition Reference

| From | Can Transition To |
|------|------------------|
| `backlog` | `ready`, `cancelled` |
| `ready` | `executing`, `blocked`, `cancelled` |
| `blocked` | `ready`, `cancelled` |
| `executing` | `qa_refining`, `pending_review`, `failed`, `blocked`, `merging`, `stopped`, `paused` |
| `qa_refining` | `qa_testing`, `stopped`, `paused` |
| `qa_testing` | `qa_passed`, `qa_failed`, `stopped`, `paused` |
| `qa_passed` | `pending_review` (auto) |
| `qa_failed` | `revision_needed` |
| `pending_review` | `reviewing` (auto) |
| `reviewing` | `review_passed`, `revision_needed`, `escalated`, `merging`, `stopped`, `paused` |
| `review_passed` | `approved`, `revision_needed` |
| `escalated` | `approved`, `revision_needed` |
| `revision_needed` | `re_executing` (auto), `cancelled` |
| `re_executing` | `pending_review`, `failed`, `blocked`, `merging`, `stopped`, `paused` |
| `approved` | `pending_merge` (auto) |
| `pending_merge` | `merged`, `merging` |
| `merging` | `merged`, `merge_conflict`, `merge_incomplete`, `stopped`, `paused` |
| `merge_incomplete` | `pending_merge`, `merged` |
| `merge_conflict` | `merged` |
| `merged` | `ready` |
| `failed` | `ready` |
| `cancelled` | `ready` |
| `stopped` | `ready` |
| `paused` | back to pre-pause state (`executing`, `re_executing`, `qa_refining`, `qa_testing`, `reviewing`, `merging`) |

---

## Key Behavioral Patterns

### Happy Path — Without QA

```
backlog → ready → executing → pending_review → (auto) reviewing
→ review_passed → [human: v1_approve_review] → approved → (auto) pending_merge
→ merged  (fast path: programmatic merge)
       or merging → merged  (merger agent needed)
```

### Happy Path — With QA

```
backlog → ready → executing → qa_refining → qa_testing
→ qa_passed → (auto) pending_review → (auto) reviewing
→ review_passed → [human: v1_approve_review] → approved → (auto) pending_merge → merged
```

### Revision Loop (Normal — Can Repeat 2-3×)

```
reviewing → revision_needed → (auto) re_executing → pending_review
→ (auto) reviewing → ... repeat until review_passed or failed/cancelled
```

When you observe this loop, it is expected behavior — the reviewer is iterating with the worker. Only intervene if the loop repeats more than ~5 times or if `failed` is reached.

### Branch Freshness Conflict (Auto-Routed)

Tasks can unexpectedly enter `merging` from `executing`, `re_executing`, or `reviewing` when the main branch has been updated since the task branch was created. This is automatic:

```
executing → (branch stale) → merging → merged → ready  (re-queued for execution)
reviewing → (branch stale) → merging → merged → pending_review  (re-queued for review)
```

**External view:** You will see `task:status_changed` events `executing → merging → merged → ready/pending_review`. This is NOT a failure — the system is rebasing and continuing. The task restarts from the appropriate stage.

**Cap:** After 3 consecutive freshness conflicts, the task transitions to `failed`.

---

## Pause vs Stop vs Cancel — External Perspective

| Aspect | `paused` | `stopped` | `cancelled` |
|--------|----------|-----------|-------------|
| Terminal? | NO | YES | YES |
| Agent killed? | YES | YES | YES |
| Auto-resumable? | NO (human via UI only) | NO | NO |
| Restart via `v1_retry_task`? | N/A (resumes to pre-pause state) | YES → `ready` | YES → `ready` |
| Work preserved? | YES (branch + commits intact) | YES (branch intact) | YES (branch intact) |
| When used | Temporary suspension (scope pause, provider error) | Hard stop — manual control required | Task no longer needed |

**Key rule:** If you observe `paused`, annotate and notify the human. Do NOT attempt to restart via `v1_retry_task` — that resets to `ready` (loses pre-pause context). Human uses the app UI to resume.

---

## What External Agents Can Observe

`v1_batch_task_status` returns the current `status` string for up to 50 tasks. All 24 states are valid return values. Events arrive passively via automated infrastructure — listen for `task:status_changed` events for real-time transitions.

**Human gate (NON-NEGOTIABLE):** `review_passed` and `escalated` require a human to call `v1_approve_review`, `v1_request_changes`, or `v1_resolve_escalation`. Autonomous agents MUST NOT auto-approve without explicit human delegation.
