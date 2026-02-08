# RalphX - Phase 112: Real-Time Merge Validation Streaming

## Overview

The `pending_merge` detail view shows static fake progress steps with no visibility into post-merge validation. When validation fails (→ MergeIncomplete), users only see a terse error message — no visibility into which commands ran, their output, or why they failed. There's also no way to skip validation when failures are expected/acceptable.

This phase adds real-time validation streaming to the merge UI, stores full validation logs in metadata, makes validation behavior configurable (block/warn/off), and lets users retry merges while skipping validation.

**Reference Plan:**
- `specs/plans/real_time_merge_validation_streaming.md` - Full implementation details, code snippets, and compilation unit analysis

## Goals

1. Stream validation command output in real-time to the MergingTaskDetail view
2. Store full validation log in task metadata for historical viewing
3. Make validation behavior configurable (block/warn/off) per project
4. Allow users to "Continue Anyway" when validation fails

## Dependencies

### Phase 111 (Fix Remaining plan_branch_repo Gaps) - Required

| Dependency | Why Needed |
|------------|------------|
| Merge workflow infrastructure | This phase builds on the existing programmatic merge + validation pipeline in `side_effects.rs` |
| MergeIncompleteTaskDetail view | Phase 99/108 established this view; this phase adds validation log display and skip-validation button |
| GitSettingsSection | Phase 79 established per-project git settings; this phase adds validation mode selector |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/real_time_merge_validation_streaming.md`
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
- **Tasks 2 and 3 are parallelizable** (both depend only on Task 1, different layers)

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/real_time_merge_validation_streaming.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Emit merge:validation_step events during validation + store full validation log in task metadata",
    "plan_section": "Task 1: Backend — Emit validation events + store full log",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "feat(merge): stream validation progress events and store full log",
    "steps": [
      "Read specs/plans/real_time_merge_validation_streaming.md section 'Task 1'",
      "Add ValidationLogEntry struct (phase, command, path, label, status, exit_code, stdout, stderr, duration_ms)",
      "Add log: Vec<ValidationLogEntry> field to ValidationResult struct (line 347)",
      "Add task_id_str: &str and app_handle: Option<&tauri::AppHandle> params to run_validation_commands (line 364)",
      "Update all 3 call sites (~lines 1277, 1448, 1619) to pass task_id_str and self.machine.context.services.app_handle.as_ref()",
      "Update all 8 test call sites (~lines 2190-2278) to pass '' and None",
      "For each setup and validate command: emit 'running' event before execution, time the command, build log entry, emit completed event after",
      "Truncate stdout/stderr to 2000 chars in log entries",
      "Add log: &[ValidationLogEntry] param to format_validation_error_metadata (line 524) and include validation_log in JSON output",
      "Update handle_validation_failure (line 1847) to accept and pass the full log",
      "On validation success: store validation_log in task.metadata before complete_merge_internal",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): stream validation progress events and store full log"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add MergeValidationMode enum (block/warn/off) to Project, migration v20, respect mode in merge, add skip_validation to retry_merge",
    "plan_section": "Task 2: Backend — Configurable validation mode + skip-validation retry",
    "blocking": [4],
    "blockedBy": [1],
    "atomic_commit": "feat(merge): add merge_validation_mode setting and skip-validation retry",
    "steps": [
      "Read specs/plans/real_time_merge_validation_streaming.md section 'Task 2'",
      "Add MergeValidationMode enum (Block/Warn/Off with Default=Block, serde rename_all snake_case) to project.rs",
      "Add merge_validation_mode: MergeValidationMode field to Project struct",
      "Update Project::new() and Project::new_with_worktree() to include merge_validation_mode: MergeValidationMode::default()",
      "Update Project::from_row() to read merge_validation_mode column with unwrap_or default",
      "Update setup_test_db() in project.rs tests to include merge_validation_mode column",
      "Create v20_merge_validation_mode.rs migration using add_column_if_not_exists helper",
      "Register v20 in mod.rs MIGRATIONS array and bump SCHEMA_VERSION to 20",
      "In attempt_programmatic_merge (side_effects.rs): check project.merge_validation_mode before/instead of calling run_validation_commands",
      "Add format_validation_warn_metadata() for Warn mode (store log but proceed to merged)",
      "Add skip_validation: Option<bool> param to retry_merge command (git_commands.rs line 269)",
      "When skip_validation is true: set task metadata flag, check in attempt_programmatic_merge, clear after attempt",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): add merge_validation_mode setting and skip-validation retry"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add MergeValidationStepEvent schema, useMergeValidationEvents hook, and ValidationProgress component in MergingTaskDetail",
    "plan_section": "Task 3: Frontend — Live validation display in MergingTaskDetail",
    "blocking": [4],
    "blockedBy": [1],
    "atomic_commit": "feat(merge): real-time validation progress in MergingTaskDetail",
    "steps": [
      "Read specs/plans/real_time_merge_validation_streaming.md section 'Task 3'",
      "Add MergeValidationStepEventSchema and type to src/types/events.ts",
      "Create src/hooks/useMergeValidationEvents.ts with local event subscription hook",
      "In MergingTaskDetail.tsx: import and call useMergeValidationEvents(task.id)",
      "Build ValidationProgress component with ValidationStepRow sub-component",
      "Implement data source merging: prefer live events, fall back to task.metadata.validation_log for historical views",
      "Style with CI/CD pipeline aesthetic: dark terminal blocks, monospace font, collapsible output sections",
      "Auto-expand running/failed steps, collapse successful ones",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(merge): real-time validation progress in MergingTaskDetail"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add validation mode selector to GitSettingsSection + skip-validation button in MergeIncompleteTaskDetail",
    "plan_section": "Task 4: Frontend — Settings UI + 'Continue Anyway' button",
    "blocking": [],
    "blockedBy": [2, 3],
    "atomic_commit": "feat(merge): validation mode settings and skip-validation UI",
    "steps": [
      "Read specs/plans/real_time_merge_validation_streaming.md section 'Task 4'",
      "Add merge_validation_mode to ProjectResponseSchema (snake_case, z.enum), Project interface (mergeValidationMode), and transformProject() in src/types/project.ts",
      "Add VALIDATION_MODE_OPTIONS array and SelectSettingRow for validation mode in GitSettingsSection.tsx",
      "Wire setting change to api.projects.update with mergeValidationMode field",
      "In MergeIncompleteTaskDetail.tsx: detect validation_failures in metadata",
      "When validation-caused failure: show ValidationProgress component (import from MergingTaskDetail) with stored validation_log",
      "Add 'Retry (Skip Validation)' button that calls retry_merge with { skipValidation: true }",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(merge): validation mode settings and skip-validation UI"
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
| **Tauri events for real-time streaming** | Events are fire-and-forget, non-blocking, and naturally fit the existing EventBus architecture. No new API endpoints needed. |
| **Store full log in task.metadata** | Reuses existing metadata JSON field — no new DB columns. Historical viewing works via the same field used for error context. |
| **MergeValidationMode as Project field** | Per-project setting follows existing GitMode/baseBranch pattern. Migration with DEFAULT preserves backward compatibility. |
| **skip_validation via task metadata flag** | Temporary flag cleared after use — avoids adding a new column or side channel. Checked in the same `attempt_programmatic_merge` flow. |
| **ValidationProgress in MergingTaskDetail** | Keeps the component self-contained. Reused in MergeIncompleteTaskDetail via import (not extraction to shared file). |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All 8 updated test call sites for `run_validation_commands` pass with new params
- [ ] `MergeValidationMode` serializes/deserializes correctly (block/warn/off)
- [ ] `Project::from_row()` handles missing `merge_validation_mode` column gracefully
- [ ] Migration v20 runs idempotently

### Frontend - Run `npm run typecheck`
- [ ] `MergeValidationStepEventSchema` validates correctly
- [ ] `useMergeValidationEvents` hook compiles with correct types
- [ ] `Project` type includes `mergeValidationMode` field
- [ ] `transformProject` maps `merge_validation_mode` → `mergeValidationMode`

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Trigger a merge for a task with feature branch target → see live validation steps streaming in real-time
- [ ] When validation fails → MergeIncomplete view shows full validation log with stdout/stderr
- [ ] Click "Retry (Skip Validation)" → merge completes without running validation
- [ ] Change validation mode in Settings → verify block/warn/off behavior matches
- [ ] Historical view (time travel) shows validation log from task metadata

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `merge:validation_step` events emitted by backend are received by `useMergeValidationEvents` hook
- [ ] `ValidationProgress` component renders in both MergingTaskDetail and MergeIncompleteTaskDetail
- [ ] Settings UI `SelectSettingRow` persists `mergeValidationMode` via `api.projects.update`
- [ ] `retry_merge` command accepts `skipValidation` param from frontend invoke

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
