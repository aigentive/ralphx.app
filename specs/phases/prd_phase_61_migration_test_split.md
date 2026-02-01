# RalphX - Phase 61: Migration Test File Split

## Overview

Migration tests are currently consolidated in a single `tests.rs` file (1431 LOC), significantly exceeding the 500 LOC limit. Each migration already has its own implementation file (`v1_*.rs` through `v6_*.rs`), but all tests are crammed together. This phase establishes the convention and splits the test file into per-migration test files.

**Reference Plan:**
- `specs/plans/add_migration_test_file_rule_split_tests.md` - Detailed extraction plan with test categorization

## Goals

1. Document the per-migration test file naming convention in code quality standards
2. Split `tests.rs` (1431 LOC) into ~6 individual test files, each under 500 LOC
3. Maintain test functionality and pass all existing tests

## Dependencies

### Phase 60 (Review Issues as First-Class Entities) - Not Required

This phase is independent refactoring work that doesn't depend on Phase 60 features.

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/add_migration_test_file_rule_split_tests.md`
2. Understand the test categorization and extraction targets
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
4. Commit with descriptive message

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
2. **Read the ENTIRE implementation plan** at `specs/plans/add_migration_test_file_rule_split_tests.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "documentation",
    "description": "Update code quality standards with migration test file naming convention",
    "plan_section": "1. Update Code Quality Standards",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "docs: add migration test file naming convention to code quality standards",
    "steps": [
      "Read specs/plans/add_migration_test_file_rule_split_tests.md section '1. Update Code Quality Standards'",
      "Edit .claude/rules/code-quality-standards.md line 46",
      "Change '| 4 | Add tests |' to '| 4 | Add tests to `vN_description_tests.rs` |'",
      "Verify the change by reading the file",
      "Commit: docs: add migration test file naming convention to code quality standards"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Split tests.rs into per-migration test files",
    "plan_section": "2. Split tests.rs into Per-Migration Test Files",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "refactor(migrations): split tests.rs into per-migration test files",
    "steps": [
      "Read specs/plans/add_migration_test_file_rule_split_tests.md section '2. Split tests.rs'",
      "Read current tests.rs to identify test groupings",
      "Create v1_initial_schema_tests.rs with v1 tests (core migration system + core tables + relationships + state tracking + review + ideation + chat + artifacts + settings + cascade delete)",
      "Create v2_add_dependency_reason_tests.rs with v2 tests",
      "Create v3_add_activity_events_tests.rs with v3 tests",
      "Create v4_add_blocked_reason_tests.rs with v4 tests",
      "Create v5_add_review_summary_issues_tests.rs (if v5 tests exist)",
      "Create v6_review_issues_tests.rs with v6 tests",
      "Keep shared helper function tests in tests.rs (or test_helpers.rs)",
      "Update mod.rs to include new test modules under #[cfg(test)]",
      "Run cargo test --package ralphx-lib to verify all tests pass",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Verify no single file >500 LOC with wc -l",
      "Commit: refactor(migrations): split tests.rs into per-migration test files"
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
| **Per-migration test files** | Mirrors the existing per-migration implementation file pattern (v1_*.rs, v2_*.rs, etc.) |
| **Shared helpers in tests.rs** | Core migration system tests and helper function tests don't belong to a specific migration |
| **Atomic commit for split** | New test files + mod.rs update must be committed together to avoid broken compilation |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All migration tests pass (`cargo test --package ralphx-lib`)
- [ ] No test regressions

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### File Size Verification
- [ ] `wc -l migrations/*.rs` shows no file >500 LOC
- [ ] Each test file is focused on its migration version

### Documentation Verification
- [ ] `.claude/rules/code-quality-standards.md` step 4 mentions test file naming

### Wiring Verification

**For each new test file, verify:**

- [ ] Test module is declared in mod.rs under `#[cfg(test)]`
- [ ] Tests use `super::*` to access migration helpers
- [ ] Tests compile and run independently

**Common failure modes to check:**
- [ ] No orphaned test modules (declared but file missing)
- [ ] No missing module declarations (file exists but not declared)
- [ ] No duplicate test function names across modules

See `.claude/rules/gap-verification.md` for full verification workflow.
