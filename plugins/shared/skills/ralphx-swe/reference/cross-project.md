<!-- Source: docs/features/cross-project.md | Last synced: 2026-03-24 -->

# Cross-Project Orchestration

RalphX ideation agents can autonomously orchestrate work across multiple projects from a single
master plan. This is a passive capability — no special agent action is required to trigger or
manage it.

---

## What It Is

When an ideation session spans multiple RalphX projects, RalphX automatically:

1. Detects the cross-project scope from the plan
2. Creates child sessions in each target project
3. Distributes task proposals to the appropriate project
4. Tasks then execute in parallel across project boundaries

The orchestration is fully managed by RalphX's internal agents — external agents interact with
each project independently using the standard `v1_*` tools.

---

## What This Means for Your Agent

**Tasks appear in multiple projects.** A single ideation session may produce tasks across two or
more projects. Each project's task list is independent — use `v1_get_attention_items` per project
to check status.

**Events arrive from any project in the plan.** `task:status_changed`, `review:escalated`,
`merge:conflict`, and other events may originate from any project in the cross-project plan.
Handle each event in the context of its own `project_id`.

**Reviews and escalations follow the same rules.** Approval authority remains policy-driven at
`review_passed`, and escalation rules still apply per-task, per-project — cross-project scope does
not change them.

**No special setup required.** Event routing is automatic. Your agent receives events from all
projects it is registered against.

---

## Rule Summary

| Situation | Action |
|-----------|--------|
| Tasks appear in unexpected projects | Expected — cross-project plan distributed work |
| Event arrives from an unfamiliar `project_id` | Handle normally; use `v1_get_task_detail` for context |
| Review or escalation from any project | Same rules apply — surface to human, await decision |
| Orchestration stalls (no tasks created after plan accepted) | Check `v1_get_attention_items` in source project; alert human |
