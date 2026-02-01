# RalphX - Phase 60: Review Issues as First-Class Entities

## Overview

Add structured issue tracking to reviews, mirroring the TaskStep pattern. Reviews create issues with severity/category classification, execution addresses them, and subsequent reviews verify them. This transforms flat review notes into a trackable issue lifecycle: open → in_progress → addressed → verified.

**Reference Plan:**
- `specs/plans/review_issues_first_class_entities.md` - Complete implementation plan with database schema, domain entities, repository methods, service layer, Tauri commands, frontend types, and UI components

## Goals

1. Create `review_issues` table with proper lifecycle tracking (open/in_progress/addressed/verified/wontfix)
2. Add MCP tools for reviewers to create structured issues and for workers to track resolution
3. Display issues in UI with severity badges, progress bars, and timeline integration
4. Enable issue verification across review cycles (create → address → verify workflow)

## Dependencies

### Phase 59 (State Time Travel) - Required

| Dependency | Why Needed |
|------------|------------|
| Review notes infrastructure | Issues are created as part of review notes |
| StateHistoryTimeline component | Issues will integrate into this timeline |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/review_issues_first_class_entities.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/review_issues_first_class_entities.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Create review_issues database migration",
    "plan_section": "1.1 Database Migration",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(backend): add review_issues table migration",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '1.1 Database Migration'",
      "Determine next migration version number from existing migrations",
      "Create migration file with review_issues table schema",
      "Include all columns: id, review_note_id, task_id, step_id, no_step_reason, title, description, severity, category, file_path, line_number, code_snippet, status, resolution_notes, addressed_in_attempt, verified_by_review_id, timestamps",
      "Add CHECK constraints for severity and status enums",
      "Create indexes for task_id, status, and review_note_id",
      "Register migration in MIGRATIONS array and bump SCHEMA_VERSION",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add review_issues table migration"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Create ReviewIssue domain entity and enums",
    "plan_section": "1.2 Domain Entity",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(backend): add ReviewIssue domain entity and enums",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '1.2 Domain Entity'",
      "Create src-tauri/src/domain/review_issue.rs",
      "Define ReviewIssueId newtype wrapper",
      "Define ReviewIssue struct with all fields from plan",
      "Define IssueStatus enum (Open, InProgress, Addressed, Verified, WontFix)",
      "Define IssueSeverity enum (Critical, Major, Minor, Suggestion)",
      "Define IssueCategory enum (Bug, Missing, Quality, Design)",
      "Add serde derives for serialization",
      "Export module from domain/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add ReviewIssue domain entity and enums"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Create ReviewIssue repository with CRUD operations",
    "plan_section": "1.3 Repository",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(backend): add ReviewIssue repository with CRUD operations",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '1.3 Repository'",
      "Create src-tauri/src/infrastructure/sqlite/review_issue_repository.rs",
      "Implement create(issue: ReviewIssue) -> Result<ReviewIssue>",
      "Implement bulk_create(issues: Vec<ReviewIssue>) -> Result<Vec<ReviewIssue>>",
      "Implement get_by_id(id: ReviewIssueId) -> Result<Option<ReviewIssue>>",
      "Implement get_by_task_id(task_id: TaskId) -> Result<Vec<ReviewIssue>>",
      "Implement get_open_by_task_id(task_id: TaskId) -> Result<Vec<ReviewIssue>>",
      "Implement update_status(id, status, resolution_notes?) -> Result<ReviewIssue>",
      "Implement get_summary(task_id: TaskId) -> Result<IssueProgressSummary>",
      "Export from infrastructure/sqlite/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add ReviewIssue repository with CRUD operations"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Create ReviewIssue service with business logic",
    "plan_section": "1.4 Service Layer",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(backend): add ReviewIssue service with business logic",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '1.4 Service Layer'",
      "Create src-tauri/src/application/review_issue_service.rs",
      "Implement create_issues_from_review(review_note_id, task_id, issues: Vec<CreateIssueInput>)",
      "Implement mark_issue_in_progress(issue_id)",
      "Implement mark_issue_addressed(issue_id, resolution_notes, attempt_number)",
      "Implement verify_issue(issue_id, review_note_id)",
      "Implement reopen_issue(issue_id, reason)",
      "Implement get_issue_progress(task_id) -> IssueProgressSummary",
      "Add validation: step_id OR no_step_reason must be provided",
      "Export from application/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add ReviewIssue service with business logic"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Add Tauri commands for review issues (review agent tools)",
    "plan_section": "2.1 Review Agent Tools",
    "blocking": [6, 7, 8],
    "blockedBy": [4],
    "atomic_commit": "feat(backend): add Tauri commands for review issues",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '2.1 Review Agent Tools'",
      "Add get_task_issues command (with status_filter param)",
      "Add get_issue_progress command",
      "Add verify_issue command",
      "Add reopen_issue command",
      "Register commands in Tauri invoke handler",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add Tauri commands for review issues"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Add execute agent issue tracking commands",
    "plan_section": "2.2 Execute Agent Tools",
    "blocking": [10],
    "blockedBy": [5],
    "atomic_commit": "feat(backend): add execute agent issue tracking commands",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '2.2 Execute Agent Tools'",
      "Add mark_issue_in_progress command",
      "Add mark_issue_addressed command (with resolution_notes, attempt_number)",
      "Register commands in Tauri invoke handler",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add execute agent issue tracking commands"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "backend",
    "description": "Update CompleteReviewInput with structured issues support",
    "plan_section": "2.3 Update complete_review Tool",
    "blocking": [9],
    "blockedBy": [5],
    "atomic_commit": "feat(backend): update CompleteReviewInput with structured issues",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '2.3 Update complete_review Tool'",
      "Add ReviewIssueInput struct with all fields from plan",
      "Add issues: Vec<ReviewIssueInput> field to CompleteReviewInput",
      "Update complete_review handler to create issues when provided",
      "Add validation: if outcome == NeedsChanges and issues.is_empty() -> Error",
      "Add validation: for each issue, step_id OR no_step_reason must be present",
      "Update all test files constructing CompleteReviewInput",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): update CompleteReviewInput with structured issues"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Add ReviewIssue types and Zod schemas",
    "plan_section": "3.1 Types & Schemas",
    "blocking": [11],
    "blockedBy": [5],
    "atomic_commit": "feat(frontend): add ReviewIssue types and Zod schemas",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '3.1 Types & Schemas'",
      "Create src/types/review-issue.ts",
      "Define ReviewIssueSchema with all fields (snake_case matching Rust)",
      "Define IssueProgressSummarySchema",
      "Define transform functions for camelCase conversion",
      "Export types: ReviewIssue, IssueProgressSummary, IssueStatus, IssueSeverity, IssueCategory",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): add ReviewIssue types and Zod schemas"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "agent",
    "description": "Update reviewer agent for structured issues",
    "plan_section": "4.1 Review Agent Prompt Updates",
    "blocking": [],
    "blockedBy": [7],
    "atomic_commit": "feat(plugin): update reviewer agent for structured issues",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '4.1 Review Agent Prompt Updates'",
      "Update reviewer agent system prompt in ralphx-plugin",
      "Add instructions to use structured issues in complete_review",
      "Document required fields: title, severity, category",
      "Document step_id linking or no_step_reason requirement",
      "Document file_path:line_number for code issues",
      "Commit: feat(plugin): update reviewer agent for structured issues"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "agent",
    "description": "Update worker agent for issue tracking",
    "plan_section": "4.2 Execute Agent Prompt Updates",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "feat(plugin): update worker agent for issue tracking",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '4.2 Execute Agent Prompt Updates'",
      "Update worker agent system prompt in ralphx-plugin",
      "Add instructions to call get_task_issues before starting work",
      "Add instructions to prioritize by severity (critical first)",
      "Add instructions to use mark_issue_in_progress when starting",
      "Add instructions to use mark_issue_addressed with resolution notes",
      "Commit: feat(plugin): update worker agent for issue tracking"
    ],
    "passes": false
  },
  {
    "id": 11,
    "category": "frontend",
    "description": "Add reviewIssuesApi with Tauri invocations",
    "plan_section": "3.2 API Layer",
    "blocking": [12],
    "blockedBy": [8],
    "atomic_commit": "feat(frontend): add reviewIssuesApi with Tauri invocations",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '3.2 API Layer'",
      "Create src/api/review-issues.ts",
      "Add getByTaskId(taskId) using typedInvokeWithTransform",
      "Add getProgress(taskId) using typedInvokeWithTransform",
      "Add updateStatus(issueId, status, notes?) using typedInvokeWithTransform",
      "Export reviewIssuesApi",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): add reviewIssuesApi with Tauri invocations"
    ],
    "passes": false
  },
  {
    "id": 12,
    "category": "frontend",
    "description": "Add IssueList and IssueTimeline UI components",
    "plan_section": "3.3 UI Components",
    "blocking": [13],
    "blockedBy": [11],
    "atomic_commit": "feat(frontend): add IssueList and IssueTimeline components",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '3.3 UI Components'",
      "Create src/components/Reviews/IssueList.tsx",
      "Add IssueCard sub-component with severity color and status badge",
      "Add IssueProgressBar (similar to StepProgressBar)",
      "Add groupBy options: severity | status | step",
      "Create src/components/Reviews/IssueTimeline.tsx for lifecycle display",
      "Add file:line link rendering for code issues",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): add IssueList and IssueTimeline components"
    ],
    "passes": false
  },
  {
    "id": 13,
    "category": "frontend",
    "description": "Integrate issue progress into StateHistoryTimeline",
    "plan_section": "4.3 Progress Tracking",
    "blocking": [],
    "blockedBy": [12],
    "atomic_commit": "feat(frontend): integrate issue progress into StateHistoryTimeline",
    "steps": [
      "Read specs/plans/review_issues_first_class_entities.md section '4.3 Progress Tracking'",
      "Update StateHistoryTimeline to show issues under review entries",
      "Add collapse/expand for issue lists",
      "Show issue diff between reviews (new, resolved, reopened)",
      "Add IssueProgressSummary display with percentResolved",
      "Add severity breakdown (critical/major/minor/suggestion)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): integrate issue progress into StateHistoryTimeline"
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
| **Separate review_issues table** | Enables proper lifecycle tracking, indexing, and querying vs embedded JSON |
| **step_id OR no_step_reason required** | Forces AI to either link issues to steps or explain why not, improving traceability |
| **Severity enum (critical/major/minor/suggestion)** | Enables prioritization in worker execution and progress tracking |
| **Status lifecycle mirrors TaskStep** | Consistent UX and familiar patterns across the codebase |
| **IssueProgressSummary** | Mirrors StepProgressSummary for consistency, enables progress bars and completion metrics |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Migration creates review_issues table with all columns
- [ ] Repository CRUD operations work correctly
- [ ] Service layer validates step_id/no_step_reason requirement
- [ ] complete_review validates issues required for needs_changes outcome

### Frontend - Run `npm run test`
- [ ] ReviewIssue types parse snake_case responses correctly
- [ ] IssueList renders with severity colors and status badges
- [ ] IssueProgressBar shows correct percentages

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Review creates issues → Issues appear in timeline
- [ ] Worker marks issue addressed → Status updates in UI
- [ ] Next review can verify resolved issues
- [ ] Reopened issues return to open status
- [ ] IssueProgressBar shows correct completion percentage

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] IssueList component is rendered in StateHistoryTimeline
- [ ] reviewIssuesApi calls backend commands successfully
- [ ] Issue status changes reflect in UI immediately

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
