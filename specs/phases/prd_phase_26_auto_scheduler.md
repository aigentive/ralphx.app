# RalphX - Phase 26: Auto-Scheduler for Ready Tasks

## Overview

This phase implements automatic task scheduling when tasks enter the Ready status. Currently, tasks must be manually started even when execution slots are available. This phase adds proactive scheduling that automatically picks up Ready tasks when capacity allows, creating a true queue-based execution system.

The auto-scheduler triggers in four scenarios: when a task enters Ready, when a running task completes (freeing a slot), on app startup, and when execution is unpaused or max_concurrent is increased.

**Reference Plan:**
- `specs/plans/auto_scheduler_ready_tasks.md` - Detailed implementation plan with code snippets and file locations

## Goals

1. Automatically start Ready tasks when execution slots are available
2. Chain task execution so completing one task triggers the next
3. Resume Ready task scheduling on app startup
4. Trigger scheduling when execution is unpaused or capacity increases

## Dependencies

### Phase 21 (Execution Control & Task Resumption) - Required

| Dependency | Why Needed |
|------------|------------|
| ExecutionState service | Provides `can_start_task()` capacity check |
| StartupJobRunner | Existing infrastructure for startup task resumption |
| TransitionHandler | Core state machine handler where scheduling logic lives |

### Phase 3 (State Machine) - Required

| Dependency | Why Needed |
|------------|------------|
| State machine architecture | `on_enter`/`on_exit` hooks for triggering scheduling |
| statig integration | Event-driven state transitions |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/auto_scheduler_ready_tasks.md`
2. Understand the trigger points and their interactions
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run `cargo clippy --all-targets --all-features -- -D warnings`
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/auto_scheduler_ready_tasks.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Add try_schedule_ready_tasks() method to TransitionHandler",
    "plan_section": "1. Add try_schedule_ready_tasks() to TransitionHandler",
    "steps": [
      "Read specs/plans/auto_scheduler_ready_tasks.md section '1. Add try_schedule_ready_tasks()'",
      "Add method that checks can_start_task(), fetches oldest Ready task across all projects",
      "Use spawn to avoid blocking current transition (circular dependency prevention)",
      "Trigger StartExecution event on the Ready task via state machine",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(scheduler): add try_schedule_ready_tasks method"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Call scheduler from on_exit() when execution slot frees",
    "plan_section": "2. Call from on_exit() when slot frees",
    "steps": [
      "Read specs/plans/auto_scheduler_ready_tasks.md section '2. Call from on_exit()'",
      "In on_exit() for Executing/QaRefining/etc states, after decrement_running()",
      "Call try_schedule_ready_tasks() to pick up next Ready task",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(scheduler): trigger scheduling when execution slot frees"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Call scheduler from on_enter(Ready) when task becomes Ready",
    "plan_section": "3. Call from on_enter(Ready)",
    "steps": [
      "Read specs/plans/auto_scheduler_ready_tasks.md section '3. Call from on_enter(Ready)'",
      "Add entry action for Ready state in side_effects.rs",
      "Call try_schedule_ready_tasks() to immediately execute if slot available",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(scheduler): auto-start Ready tasks when slots available"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Extend StartupJobRunner to schedule Ready tasks",
    "plan_section": "4. Add to startup resumption",
    "steps": [
      "Read specs/plans/auto_scheduler_ready_tasks.md section '4. Add to startup resumption'",
      "After existing agent-active task resumption in startup_jobs.rs",
      "Call try_schedule_ready_tasks() to pick up queued Ready tasks",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(scheduler): schedule Ready tasks on app startup"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Trigger scheduling on unpause and max_concurrent increase",
    "plan_section": "5. Handle unpause/max_concurrent changes",
    "steps": [
      "Read specs/plans/auto_scheduler_ready_tasks.md section '5. Handle unpause/max_concurrent changes'",
      "In resume_execution command, call try_schedule_ready_tasks() after resuming",
      "In set_max_concurrent command, call try_schedule_ready_tasks() when capacity increases",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(scheduler): trigger scheduling on unpause and capacity increase"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add cross-project Ready task query",
    "plan_section": "Challenges - Cross-project scheduling",
    "steps": [
      "Add repository method to fetch oldest Ready task across all projects",
      "Order by created_at ascending to ensure FIFO scheduling",
      "Handle case where no Ready tasks exist (return None)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(scheduler): add cross-project Ready task query"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add functional tests for scheduler logic",
    "plan_section": "Verification",
    "steps": [
      "Test: try_schedule_ready_tasks returns early when can_start_task is false",
      "Test: cross-project query returns oldest Ready task by created_at",
      "Test: cross-project query returns None when no Ready tasks exist",
      "Run cargo test",
      "Commit: test(scheduler): add functional tests for scheduler"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Spawn async task for scheduling** | Avoids circular dependency - TransitionHandler transitioning another task while handling a transition |
| **Cross-project scheduling** | Ready tasks from any project can fill available slots, maximizing throughput |
| **FIFO ordering by created_at** | Fair scheduling - oldest Ready task gets executed first |
| **Atomic check-and-schedule** | Prevents race conditions when multiple tasks complete simultaneously |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `try_schedule_ready_tasks` correctly checks capacity before scheduling
- [ ] Scheduling triggers on exit from execution states
- [ ] Scheduling triggers on entry to Ready state
- [ ] Startup schedules existing Ready tasks
- [ ] Unpause triggers scheduling
- [ ] Capacity increase triggers scheduling

### Build Verification
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `cargo build --release` succeeds

### Manual Testing
- [ ] Create task in Draft, drag to Ready -> auto-starts if slots available
- [ ] Set max_concurrent=1, queue 3 tasks -> executes sequentially
- [ ] Pause, queue tasks, unpause -> picks up queued tasks
- [ ] Restart app with Ready tasks -> auto-starts on startup

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `try_schedule_ready_tasks` is called from all trigger points (on_exit, on_enter, startup, unpause, capacity change)
- [ ] Cross-project query returns correct oldest Ready task
- [ ] StartExecution event properly transitions task to Executing

**Common failure modes to check:**
- [ ] No spawned tasks that never complete
- [ ] No deadlocks from concurrent scheduling attempts
- [ ] No tasks stuck in Ready when slots are available

See `.claude/rules/gap-verification.md` for full verification workflow.
