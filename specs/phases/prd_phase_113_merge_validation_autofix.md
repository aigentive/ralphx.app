# RalphX - Phase 113: AI Auto-Fix for Validation Failures

## Overview

Phase 112 adds real-time validation streaming and configurable validation modes (Block/Warn/Off). When mode is `Block` and validation fails, the task transitions to `MergeIncomplete` — requiring human intervention. Many validation failures are simple build errors (type errors, missing imports, lint issues) that an AI agent could fix automatically.

This phase adds an `AutoFix` validation mode: when validation fails, the system transitions to `Merging` instead of `MergeIncomplete`, spawning the merger agent (Opus, full code access) to attempt a fix. If it succeeds, the merge completes automatically. If it fails, THEN it falls back to `MergeIncomplete` for human intervention.

**Reference Plan:**
- `specs/plans/phase_113_ai_auto_fix_for_validation_failures.md` - Full implementation details, code snippets, file analysis notes, compilation unit validation

## Goals

1. Add `AutoFix` variant to `MergeValidationMode` enum as a 4th option (Off < Warn < AutoFix < Block)
2. Route validation failures to merger agent when AutoFix is enabled, with one-shot attempt before human fallback
3. Re-validate in auto-completion to prevent premature `Merged` when agent exits without fixing
4. Provide appropriate context to the merger agent for validation recovery vs conflict resolution

## Dependencies

### Phase 112 (Real-Time Merge Validation Streaming) - Required

| Dependency | Why Needed |
|------------|------------|
| `MergeValidationMode` enum (Block/Warn/Off) | AutoFix is a new variant added to this enum |
| `handle_validation_failure()` fn | AutoFix branch added to this function |
| `run_validation_commands()` fn | Re-used for auto-completion re-validation |
| `ValidationProgress` component | Displayed in MergingTaskDetail for recovery context |
| Settings UI validation mode dropdown | AutoFix option added to existing dropdown |
| Frontend `mergeValidationMode` Zod type | `auto_fix` value added to existing enum |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/phase_113_ai_auto_fix_for_validation_failures.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/phase_113_ai_auto_fix_for_validation_failures.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add AutoFix variant to MergeValidationMode and implement validation recovery flow in handle_validation_failure",
    "plan_section": "Task 1: Backend — Add AutoFix variant + validation recovery flow",
    "blocking": [2, 3, 4],
    "blockedBy": [],
    "atomic_commit": "feat(merge): add AutoFix validation mode and recovery flow in side_effects",
    "steps": [
      "Read specs/plans/phase_113_ai_auto_fix_for_validation_failures.md section 'Task 1'",
      "Add AutoFix variant to MergeValidationMode enum in project.rs",
      "Update Display impl to handle AutoFix (format as 'auto_fix')",
      "Update FromStr impl to parse 'auto_fix' string",
      "Add tests for AutoFix serialization, deserialization, Display, FromStr",
      "Add validation_mode parameter to handle_validation_failure signature in side_effects.rs",
      "Implement AutoFix branch: skip revert, set validation_recovery metadata, transition to Merging",
      "Keep existing Block behavior in else branch",
      "Update all 3 call sites (~lines 1506, 1696, 1884) to pass validation_mode",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): add AutoFix validation mode and recovery flow in side_effects"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Update merge auto-completion to re-run validation when in validation recovery mode before completing merge",
    "plan_section": "Task 2: Backend — Update auto-completion to re-validate in recovery mode",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(merge): re-validate in auto-completion for validation recovery mode",
    "steps": [
      "Read specs/plans/phase_113_ai_auto_fix_for_validation_failures.md section 'Task 2'",
      "Make run_validation_commands and related types (ValidationResult, ValidationFailure, ValidationLogEntry, format_validation_error_metadata) pub(crate) in side_effects.rs",
      "In attempt_merge_auto_complete (chat_service_send_background.rs), after git state checks pass, detect validation_recovery flag in task metadata",
      "If validation_recovery is true: re-run validation commands before calling complete_merge_internal",
      "If re-validation fails: revert with git reset HEAD~1, transition to MergeIncomplete with error metadata",
      "If re-validation passes: proceed with complete_merge_internal as normal",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): re-validate in auto-completion for validation recovery mode"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "agent",
    "description": "Update merger agent context to send validation-recovery-specific initial message and update agent system prompt",
    "plan_section": "Task 3: Backend — Update merger agent context for validation recovery",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(merge): add validation recovery context for merger agent",
    "steps": [
      "Read specs/plans/phase_113_ai_auto_fix_for_validation_failures.md section 'Task 3'",
      "In chat_service_context.rs, modify the Merge context message builder to check task metadata for validation_recovery flag",
      "If validation_recovery: use 'Fix validation failures' prompt instead of 'Resolve merge conflicts' prompt",
      "Add Validation Recovery Mode section to ralphx-plugin/agents/merger.md with detection instructions and workflow",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): add validation recovery context for merger agent"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Show validation recovery context in MergingTaskDetail — different messaging and failure details when in recovery mode",
    "plan_section": "Task 4: Frontend — Show validation recovery context in MergingTaskDetail",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(ui): show validation recovery context in MergingTaskDetail",
    "steps": [
      "Read specs/plans/phase_113_ai_auto_fix_for_validation_failures.md section 'Task 4'",
      "In MergingTaskDetail.tsx, parse task.metadata for validation_recovery flag",
      "If validation_recovery: show 'Fixing validation errors...' instead of 'Resolving merge conflicts...'",
      "Display validation failures from metadata (validation_failures array)",
      "Show ValidationProgress component (from Phase 112) with stored validation log",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): show validation recovery context in MergingTaskDetail"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add AutoFix option to validation mode settings dropdown and update frontend Zod type",
    "plan_section": "Task 5: Frontend — Add AutoFix option to settings UI",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(ui): add AutoFix option to validation mode settings",
    "steps": [
      "Read specs/plans/phase_113_ai_auto_fix_for_validation_failures.md section 'Task 5'",
      "Add 'auto_fix' to the Zod enum for mergeValidationMode in src/types/project.ts",
      "Add Auto-fix option to VALIDATION_MODE_OPTIONS array in GitSettingsSection.tsx (between Block and Warn)",
      "Set label to 'Auto-fix' and description to 'AI agent attempts to fix validation errors before asking you'",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): add AutoFix option to validation mode settings"
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
| **Reuse `Merging` state** | Merger agent is already Opus with full code access, already runs validation, auto-completion already handles success/failure. No new state in 24-state machine. No new UI view needed. |
| **Don't revert merge before agent** | Agent needs the merged (but failing) code on the branch to diagnose and fix. Revert only if agent also fails. |
| **Add `AutoFix` to `MergeValidationMode` enum** | Natural 4th option (Off < Warn < AutoFix < Block). Slots into existing settings UI dropdown. No separate boolean needed. |
| **Re-validate in auto-completion** | When merger was spawned for validation recovery (not conflicts), auto-completion must re-run validation. Prevents premature Merged if agent exits without fixing. |
| **One shot** | Agent gets one attempt. If it can't fix in one session, falls back to MergeIncomplete. No retry loops. |
| **`validation_recovery` metadata flag** | Clean signal between side_effects (sets flag) → auto-completion (checks flag) → chat context (reads flag) → UI (reads flag). Single metadata field gates all recovery behavior. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] AutoFix variant serializes to `"auto_fix"` and deserializes back
- [ ] `FromStr` parses `"auto_fix"` correctly
- [ ] `Display` formats AutoFix as `"auto_fix"`

### Frontend - Run `npm run typecheck`
- [ ] Zod schema accepts `"auto_fix"` as valid validation mode
- [ ] Settings dropdown renders 4 options (Block, Auto-fix, Warn, Disabled)

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Manual Testing
- [ ] Set validation mode to "Auto-fix", trigger merge with intentional build errors → task enters Merging (not MergeIncomplete)
- [ ] MergingTaskDetail shows "Fixing validation errors..." with failure details
- [ ] Merger agent fixes code, commits, exits → auto-completion re-runs validation → passes → Merged
- [ ] Same setup with unfixable errors → agent calls report_incomplete → MergeIncomplete
- [ ] Verify Block mode still works as before (revert → MergeIncomplete immediately)
- [ ] Verify Warn mode still works as before (proceed with warnings)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] AutoFix variant in Rust enum → serialized to DB → deserialized in side_effects → routes to Merging
- [ ] `validation_recovery` metadata flag set by side_effects → read by auto-completion, chat context, and UI
- [ ] Settings dropdown value `auto_fix` → saved to project → read during merge attempt
- [ ] Merger agent receives correct initial message based on metadata flag

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] `run_validation_commands` accessibility (must be `pub(crate)`)

See `.claude/rules/gap-verification.md` for full verification workflow.
