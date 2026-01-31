# RalphX - Phase 49: Fix Escalation Reason Not Displaying

## Overview

When an AI reviewer escalates a task via `complete_review`, the escalation reason/feedback is sent and stored, but the `EscalatedTaskDetail` UI component shows "No escalation reason provided" instead of displaying the actual feedback.

The root cause is that two response paths exist for review notes - the HTTP endpoint parses embedded issues from notes while the Tauri command returns raw notes without parsing. This phase fixes the Tauri command to match the HTTP endpoint behavior.

**Reference Plan:**
- `specs/plans/fix_escalation_reason_not_displaying.md` - Detailed implementation plan with code examples

## Goals

1. Add `ReviewIssue` type and `issues` field to `ReviewNoteResponse` struct
2. Extract `parse_issues_from_notes` helper to a shared location for reuse
3. Update `get_task_state_history` Tauri command to parse issues from notes

## Dependencies

### Phase 46 (Escalation Data Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| Escalated state implementation | This fix builds on the escalated state infrastructure |
| Issues array in HTTP handler | We're replicating this behavior to the Tauri command |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_escalation_reason_not_displaying.md`
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

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_escalation_reason_not_displaying.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add ReviewIssue type and issues field to ReviewNoteResponse",
    "plan_section": "Task 1: Add issues field to Tauri ReviewNoteResponse",
    "blocking": [3],
    "blockedBy": [],
    "atomic_commit": "feat(reviews): add ReviewIssue type and issues field to ReviewNoteResponse",
    "steps": [
      "Read specs/plans/fix_escalation_reason_not_displaying.md section 'Task 1'",
      "Add ReviewIssue struct to review_commands_types.rs with fields: severity, file, line, description",
      "Add issues: Option<Vec<ReviewIssue>> field to ReviewNoteResponse struct",
      "Add #[serde(skip_serializing_if = \"Option::is_none\")] attribute to issues field",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(reviews): add ReviewIssue type and issues field to ReviewNoteResponse"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Extract parse_issues_from_notes helper to shared module",
    "plan_section": "Task 2: Extract parsing helper to shared location",
    "blocking": [3],
    "blockedBy": [],
    "atomic_commit": "refactor(reviews): extract parse_issues_from_notes to shared module",
    "steps": [
      "Read specs/plans/fix_escalation_reason_not_displaying.md section 'Task 2'",
      "Read src-tauri/src/http_server/handlers/reviews.rs to find the parse_issues_from_notes function",
      "Create src-tauri/src/commands/review_helpers.rs with the extracted parsing logic",
      "Add pub mod review_helpers to src-tauri/src/commands/mod.rs",
      "Update reviews.rs to import and use the shared helper",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: refactor(reviews): extract parse_issues_from_notes to shared module"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Update get_task_state_history to parse issues from notes",
    "plan_section": "Task 3: Update get_task_state_history command",
    "blocking": [],
    "blockedBy": [1, 2],
    "atomic_commit": "feat(reviews): parse issues in get_task_state_history response",
    "steps": [
      "Read specs/plans/fix_escalation_reason_not_displaying.md section 'Task 3'",
      "Update get_task_state_history in review_commands.rs to use parse_issues_from_notes",
      "Modify the From<ReviewNote> conversion to parse issues and clean notes",
      "Ensure the response includes both parsed issues and cleaned notes text",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(reviews): parse issues in get_task_state_history response"
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
| **Parse issues in backend (not frontend)** | Keeps parsing logic in Rust, avoids duplication across languages |
| **Extract to shared module** | Reuses existing HTTP handler logic, single source of truth |
| **Optional issues field** | Backward compatible - notes without embedded JSON still work |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `parse_issues_from_notes` correctly extracts issues JSON
- [ ] `parse_issues_from_notes` returns clean notes without JSON prefix
- [ ] `get_task_state_history` returns both issues and cleaned notes

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Build succeeds (`cargo build --release`)

### Manual Testing
- [ ] Create a task and run to review stage
- [ ] Have AI reviewer escalate with feedback and issues
- [ ] Verify `EscalatedTaskDetail` shows escalation reason text (not JSON)
- [ ] Verify issues list with severity badges displays correctly

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `get_task_state_history` Tauri command returns issues field
- [ ] Frontend receives and displays issues correctly
- [ ] Notes field contains clean text without embedded JSON

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
