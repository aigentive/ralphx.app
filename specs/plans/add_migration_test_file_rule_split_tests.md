# Plan: Add Migration Test File Rule + Split Existing tests.rs

## Problem

Migration tests are all in a single `tests.rs` file (1431 LOC), violating the 500 LOC limit. Each migration (`v1_*.rs` through `v6_*.rs`) has its own file, but all tests are crammed together.

## Changes

### 1. Update Code Quality Standards (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `docs: add migration test file naming convention to code quality standards`

Edit step 4 in the Database table:

**Before:**
```
| 4 | Add tests |
```

**After:**
```
| 4 | Add tests to `vN_description_tests.rs` |
```

**File:** `.claude/rules/code-quality-standards.md` line 46

### 2. Split tests.rs into Per-Migration Test Files
**Dependencies:** Task 1 (convention must be documented first)
**Atomic Commit:** `refactor(migrations): split tests.rs into per-migration test files`

Extract tests from `tests.rs` (1431 LOC) into individual files:

| Current (tests.rs) | Extract To |
|--------------------|------------|
| v1 initial schema tests | `v1_initial_schema_tests.rs` |
| v2 dependency reason tests | `v2_add_dependency_reason_tests.rs` |
| v3 activity events tests | `v3_add_activity_events_tests.rs` |
| v4 blocked reason tests | `v4_add_blocked_reason_tests.rs` |
| v5 review summary tests | `v5_add_review_summary_issues_tests.rs` |
| v6 review issues tests | `v6_review_issues_tests.rs` |
| Shared helpers | Keep in `tests.rs` or `test_helpers.rs` |

**Location:** `src-tauri/src/infrastructure/sqlite/migrations/`

**After split:** Update `mod.rs` to include new test modules.

## Verification

1. Read updated code quality standards - step 4 specifies test file naming
2. `cargo test --package ralphx-lib` - all migration tests pass
3. `wc -l migrations/*.rs` - no single test file >500 LOC

## Commits

1. `docs: add migration test file naming convention to code quality standards`
2. `refactor(migrations): split tests.rs into per-migration test files`

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Compilation Unit Notes

| Task | Unit Type | Rationale |
|------|-----------|-----------|
| 1 | Standalone docs | Pure markdown edit, no code dependencies |
| 2 | Atomic refactor | New test files + mod.rs update must be committed together |

**Task 2 atomicity:** The new `vN_*_tests.rs` files and the `mod.rs` update that declares them must be in the same commit. Creating files without the `mod` declaration would leave dangling modules; updating `mod.rs` without the files would break compilation.
