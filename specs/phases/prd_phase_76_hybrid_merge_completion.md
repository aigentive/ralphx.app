# RalphX - Phase 76: Hybrid Merge Completion Detection

## Overview

This phase implements auto-detection of merge completion when the merger agent exits, eliminating the requirement for agents to explicitly call `complete_merge`. When a merge agent finishes (success or failure), the system checks git state to determine if the merge succeeded, making the workflow more robust against agent failures.

The hybrid approach maintains backwards compatibility: agents can still call `complete_merge` explicitly, but the system will auto-detect success if they don't. The `report_conflict` tool remains required for the failure path to provide context.

**Reference Plan:**
- `specs/plans/hybrid_merge_completion_detection.md` - Detailed implementation plan with code examples and architecture decisions

## Goals

1. **Auto-detect merge completion** - Check git state when merge agent exits to determine if merge succeeded
2. **Eliminate agent dependency** - Remove the requirement for agents to call `complete_merge`
3. **Maintain backwards compatibility** - Keep `complete_merge` functional for explicit signaling
4. **Handle failure cases** - Auto-transition to MergeConflict if agent exits with incomplete merge

## Dependencies

### Phase 66 (Per-Task Git Branch Isolation) - Required

| Dependency | Why Needed |
|------------|------------|
| Merge workflow infrastructure | This phase hooks into the existing merger agent and Merging state |
| `complete_merge` HTTP endpoint | Making this endpoint idempotent |
| GitService | Adding detection helpers to existing git operations |

### Phase 75 (Merge Chat Context) - Required

| Dependency | Why Needed |
|------------|------------|
| ChatContextType::Merge | Detection hooks into Merge context type completion |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/hybrid_merge_completion_detection.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

```
Task 1 ──┐
         ├──→ Task 4 ──→ Task 5
Task 2 ──┘
Task 3 (independent)
```

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/hybrid_merge_completion_detection.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add merge state detection helpers to GitService",
    "plan_section": "Task 1: Add merge detection helpers to GitService",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "feat(git): add merge state detection helpers",
    "steps": [
      "Read specs/plans/hybrid_merge_completion_detection.md section 'Task 1'",
      "Add is_rebase_in_progress() - check for .git/rebase-merge and .git/rebase-apply directories",
      "Add has_conflict_markers() - grep for conflict markers in tracked files",
      "Make get_head_sha() public (already exists as private)",
      "Add unit tests for the new detection helpers",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(git): add merge state detection helpers"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Extract shared merge completion logic from side_effects.rs",
    "plan_section": "Task 3: Extract shared merge completion logic",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "refactor(state-machine): extract shared merge completion logic",
    "steps": [
      "Read specs/plans/hybrid_merge_completion_detection.md section 'Task 3'",
      "Extract complete_merge_internal() from attempt_programmatic_merge()",
      "Function should handle: update task with SHA, transition to Merged, cleanup branch/worktree, emit event",
      "Refactor attempt_programmatic_merge() to use the new shared function",
      "Ensure programmatic merge path still works correctly",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: refactor(state-machine): extract shared merge completion logic"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Make complete_merge HTTP handler idempotent with SHA validation",
    "plan_section": "Task 4: Validate + make complete_merge handler idempotent",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(http): make complete_merge idempotent with SHA validation",
    "steps": [
      "Read specs/plans/hybrid_merge_completion_detection.md section 'Task 4'",
      "Add SHA validation: must be 40 hex characters",
      "Return 400 with helpful message if invalid SHA format",
      "Make idempotent: if task already Merged, return success with 'already_merged' status",
      "Add tests for validation and idempotency",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(http): make complete_merge idempotent with SHA validation"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Hook merge auto-completion into agent exit handler",
    "plan_section": "Task 2: Hook into agent completion in background task handler",
    "blocking": [5],
    "blockedBy": [1, 2],
    "atomic_commit": "feat(chat): add merge auto-completion on agent exit",
    "steps": [
      "Read specs/plans/hybrid_merge_completion_detection.md section 'Task 2'",
      "Add attempt_merge_auto_complete() function in chat_service_send_background.rs",
      "Function checks: task still Merging? rebase in progress? conflict markers? then auto-complete or fail",
      "Hook into both Ok and Err branches of process_stream_background result for ChatContextType::Merge",
      "Use GitService helpers from Task 1 and complete_merge_internal from Task 2",
      "Pass required repos/services to the auto-complete function",
      "Add tracing for auto-completion decisions",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(chat): add merge auto-completion on agent exit"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "agent",
    "description": "Update merger agent docs to reflect auto-detection",
    "plan_section": "Task 5: Update merger agent docs",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "docs(plugin): update merger agent for auto-detected completion",
    "steps": [
      "Read specs/plans/hybrid_merge_completion_detection.md section 'Task 5'",
      "Update CRITICAL section: remove 'MUST call complete_merge' requirement",
      "Document that merge completion is auto-detected on agent exit",
      "Keep complete_merge as optional for explicit signaling",
      "Emphasize report_conflict is still required for failure path",
      "Update workflow section to reflect new behavior",
      "Commit: docs(plugin): update merger agent for auto-detected completion"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Event-driven, not polling** | Auto-completion runs exactly once when agent exits, no background polling needed |
| **Idempotent complete_merge** | Backwards compatible - agents can still call it explicitly without errors |
| **Keep report_conflict required** | Agent provides valuable context about why conflicts couldn't be resolved |
| **Check git state, not agent output** | More reliable - git state is the source of truth for merge status |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] is_rebase_in_progress() correctly detects rebase state
- [ ] has_conflict_markers() finds conflict markers in files
- [ ] complete_merge_internal() transitions task and cleans up
- [ ] Auto-completion triggers on Merge agent exit
- [ ] complete_merge HTTP handler validates SHA format
- [ ] complete_merge HTTP handler is idempotent

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Build succeeds (`cargo build --release`)

### Manual Testing
- [ ] **Happy path:** Agent resolves conflicts, exits without calling complete_merge, task auto-transitions to Merged
- [ ] **Agent calls tool:** Agent calls complete_merge explicitly, auto-detection sees already transitioned, no error
- [ ] **Agent fails silently:** Agent exits with conflicts remaining, task auto-transitions to MergeConflict
- [ ] **Agent calls report_conflict:** Agent explicitly reports, auto-detection sees already transitioned, no error
- [ ] **Agent crash:** Process killed mid-merge, auto-detection runs, handles appropriately based on git state

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Merge agent exit triggers auto-completion check
- [ ] GitService detection helpers are called correctly
- [ ] Shared completion logic is used by all three paths (programmatic, auto-detect, HTTP)
- [ ] Events emit correctly for auto-completed merges

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No functions exported but never called
- [ ] Auto-completion actually invoked (not behind condition that's never true)

See `.claude/rules/gap-verification.md` for full verification workflow.
