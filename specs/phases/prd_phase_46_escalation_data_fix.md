# RalphX - Phase 46: Escalation Data Fix

## Overview

When AI Reviewer escalates a task via `complete_review` MCP tool, the escalation details (feedback, issues) don't appear in the UI. The UI shows "No escalation reason provided" instead of the actual feedback.

This phase fixes a field name mismatch between the MCP tool and backend, and adds support for the issues array that AI reviewers send when escalating tasks.

**Reference Plan:**
- `specs/plans/fix_escalation_data_not_showing_in_ui.md` - Detailed implementation plan with code snippets and data flow analysis

## Goals

1. Fix field name mismatch: MCP sends `feedback`, backend expects `comments`
2. Add support for `issues` array in review completion requests
3. Display escalation issues in the UI with severity badges and file:line references
4. Ensure escalation feedback appears correctly in EscalatedTaskDetail view

## Dependencies

### Phase 45 (Escalated State for Review System) - Required

| Dependency | Why Needed |
|------------|------------|
| Escalated state | This phase fixes data display in the Escalated state UI |
| EscalatedTaskDetail component | Target component for displaying issues |
| ReviewNote model | Stores escalation feedback |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_escalation_data_not_showing_in_ui.md`
2. Understand the data flow from MCP tool → backend → frontend
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_escalation_data_not_showing_in_ui.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add ReviewIssue struct and fix field name mismatch in CompleteReviewRequest",
    "plan_section": "Step 1: Backend - Fix Request Struct",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "fix(http_server): add ReviewIssue struct and fix field name mismatch",
    "steps": [
      "Read specs/plans/fix_escalation_data_not_showing_in_ui.md section 'Step 1'",
      "Add ReviewIssue struct to src-tauri/src/http_server/types.rs",
      "Rename 'comments' to 'feedback' in CompleteReviewRequest",
      "Add 'issues: Option<Vec<ReviewIssue>>' to CompleteReviewRequest",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(http_server): add ReviewIssue struct and fix field name mismatch"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Update handler to use feedback field and store issues in review notes",
    "plan_section": "Step 2: Backend - Update Handler",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "fix(http_server): store issues in review notes",
    "steps": [
      "Read specs/plans/fix_escalation_data_not_showing_in_ui.md section 'Step 2'",
      "Change req.comments to req.feedback in handlers/reviews.rs",
      "Serialize issues to JSON and store in notes field (Option A from plan)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(http_server): store issues in review notes"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Update ReviewNoteResponse to include issues and parse them in get_review_notes",
    "plan_section": "Step 3: Backend - Update Response Types",
    "blocking": [4],
    "blockedBy": [1, 2],
    "atomic_commit": "fix(http_server): include issues in review note response",
    "steps": [
      "Read specs/plans/fix_escalation_data_not_showing_in_ui.md section 'Step 3'",
      "Add 'issues: Option<Vec<ReviewIssue>>' to ReviewNoteResponse in types.rs",
      "Update get_review_notes handler to parse issues from notes and include in response",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(http_server): include issues in review note response"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add ReviewIssue schema and update response types in reviews-api.schemas.ts",
    "plan_section": "Step 4: Frontend - Update Types",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "fix(api): add ReviewIssue schema and update response types",
    "steps": [
      "Read specs/plans/fix_escalation_data_not_showing_in_ui.md section 'Step 4'",
      "Add ReviewIssueSchema to src/lib/tauri/reviews-api.schemas.ts",
      "Update ReviewNoteResponseSchema to include issues array",
      "Export ReviewIssue type",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(api): add ReviewIssue schema and update response types"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add ReviewIssuesList component to EscalatedTaskDetail",
    "plan_section": "Step 5: Frontend - Add Issues Display Component",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(components): add ReviewIssuesList for escalation details",
    "steps": [
      "Read specs/plans/fix_escalation_data_not_showing_in_ui.md section 'Step 5'",
      "Add ReviewIssuesList component to EscalatedTaskDetail.tsx",
      "Integrate with AIEscalationReasonCard or as separate section",
      "Use severity colors from plan (critical=error, major=warning, etc.)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(components): add ReviewIssuesList for escalation details"
    ],
    "passes": true
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
| **Store issues as JSON in notes field** | Avoids database migration, simple to implement, sufficient for display purposes |
| **Parse issues on response** | Keeps storage simple, transforms data at API boundary |
| **Rename to feedback, not comments** | Matches MCP tool field name for consistency |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] CompleteReviewRequest deserializes with feedback and issues fields
- [ ] ReviewNoteResponse includes issues in serialization
- [ ] Handler stores and retrieves issues correctly

### Frontend - Run `npm run test`
- [ ] ReviewIssueSchema validates correctly
- [ ] ReviewNoteResponse type includes issues

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Trigger escalation via AI reviewer with feedback and issues
- [ ] Open escalated task detail view
- [ ] Verify escalation reason shows actual feedback text (not "No escalation reason provided")
- [ ] Verify issues list displays with severity badges and file:line references

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] ReviewIssuesList component is rendered when issues exist
- [ ] Issues data flows from backend → API → component correctly
- [ ] Severity colors display correctly (critical=red, major=orange, etc.)

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
