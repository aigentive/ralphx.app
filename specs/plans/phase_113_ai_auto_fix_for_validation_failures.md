# Phase 113: AI Auto-Fix for Validation Failures

## Context

Phase 112 adds real-time validation streaming and configurable validation modes (Block/Warn/Off). When mode is `Block` and validation fails, the task transitions to `MergeIncomplete` — a human-waiting state where the user must decide what to do (retry, skip validation, or fix manually).

**Problem:** Many validation failures are simple build errors that an AI agent could fix automatically (type errors, missing imports, lint issues). Requiring human intervention for every validation failure is unnecessary friction.

**Solution:** Add an `AutoFix` validation mode that, when validation fails, transitions to `Merging` instead of `MergeIncomplete`. The merger agent (Opus, full code access) receives the validation failure context and attempts to fix the code. If it succeeds, the merge completes automatically. If it fails, THEN it falls back to `MergeIncomplete` for human intervention.

**Flow comparison:**
```
Block mode (current):
  PendingMerge → merge ok → validation fails → revert → MergeIncomplete (user acts)

AutoFix mode (new):
  PendingMerge → merge ok → validation fails → DON'T revert → Merging (agent fixes) →
    agent succeeds → re-validate → Merged
    agent fails   → revert → MergeIncomplete (user acts)
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Reuse `Merging` state** | Merger agent is already Opus with full code access, already runs validation, auto-completion already handles success/failure. No new state in 24-state machine. No new UI view needed. |
| **Don't revert merge before agent** | Agent needs the merged (but failing) code on the branch to diagnose and fix. Revert only if agent also fails. |
| **Add `AutoFix` to `MergeValidationMode` enum** | Natural 4th option (Off < Warn < AutoFix < Block). Slots into existing settings UI dropdown. No separate boolean needed. |
| **Re-validate in auto-completion** | When merger was spawned for validation recovery (not conflicts), auto-completion must re-run validation. Prevents premature Merged if agent exits without fixing. |
| **One shot** | Agent gets one attempt. If it can't fix in one session, falls back to MergeIncomplete. No retry loops. |

## Plan

### Task 1: Backend — Add AutoFix variant + validation recovery flow (BLOCKING)
**Dependencies:** Phase 112 complete (MergeValidationMode enum + validation log infrastructure)
**Atomic Commit:** `feat(merge): add AutoFix validation mode and recovery flow in side_effects`

**Files:**
- `src-tauri/src/domain/entities/project.rs` — Add `AutoFix` variant to `MergeValidationMode`
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` — Validation recovery flow

#### 1a. Add `AutoFix` variant

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeValidationMode {
    #[default]
    Block,
    AutoFix,  // NEW — agent tries to fix, then falls back to Block behavior
    Warn,
    Off,
}
```

No migration needed — the column is `TEXT`, existing rows have `'block'` default, `'auto_fix'` is just a new valid string.

#### 1b. Modify `handle_validation_failure` for AutoFix mode

Currently `handle_validation_failure` always:
1. Reverts merge (git reset HEAD~1)
2. Sets metadata with failures
3. Transitions to MergeIncomplete

New behavior when mode is `AutoFix`:
1. **DON'T revert** — keep the merged (failing) code
2. Set metadata with validation failures + `"validation_recovery": true` flag
3. Transition to `Merging` instead of `MergeIncomplete`
4. The merger agent spawns automatically (existing `on_enter(Merging)` behavior)

```rust
async fn handle_validation_failure(
    &self,
    task: &mut Task,
    task_id: &TaskId,
    task_id_str: &str,
    task_repo: &Arc<dyn TaskRepository>,
    failures: &[ValidationFailure],
    log: &[ValidationLogEntry],
    source_branch: &str,
    target_branch: &str,
    merge_path: &Path,
    mode_label: &str,
    validation_mode: &MergeValidationMode,  // NEW param
) {
    if *validation_mode == MergeValidationMode::AutoFix {
        // AutoFix: DON'T revert, transition to Merging for agent recovery
        tracing::info!(
            task_id = task_id_str,
            failure_count = failures.len(),
            "Validation failed (AutoFix mode) — spawning merger agent to attempt fix",
        );

        task.metadata = Some(serde_json::json!({
            "validation_recovery": true,
            "validation_failures": /* failure details */,
            "validation_log": log,
            "source_branch": source_branch,
            "target_branch": target_branch,
        }).to_string());
        task.internal_status = InternalStatus::Merging;
        task.touch();

        let _ = task_repo.update(task).await;
        let _ = task_repo.persist_status_change(
            task_id,
            InternalStatus::PendingMerge,
            InternalStatus::Merging,
            "validation_auto_fix",
        ).await;

        // on_enter(Merging) will spawn the merger agent
        // ... (existing Merging entry triggers agent spawn)
    } else {
        // Block mode: existing behavior — revert and go to MergeIncomplete
        // ... (current code)
    }
}
```

#### 1c. Add `PendingMerge → Merging` transition for validation recovery

Check `status.rs` — `PendingMerge` currently allows `[Merged, Merging]`. `Merging` is already a valid target. No change needed to transition table.

#### 1d. Pass `validation_mode` to handle_validation_failure at all 3 call sites

Update the 3 call sites (lines ~1506, ~1696, ~1884) to pass `&project.merge_validation_mode`.

**Compilation unit note:** The signature change to `handle_validation_failure` and all 3 call sites MUST be in this single task. The `validation_mode` variable already exists at each call site (e.g., line 1495: `let validation_mode = &project.merge_validation_mode;`). Also update `Display` and `FromStr` impls for the new variant.

---

### Task 2: Backend — Update auto-completion to re-validate in recovery mode
**Dependencies:** Task 1
**Atomic Commit:** `feat(merge): re-validate in auto-completion for validation recovery mode`

**File:** `src-tauri/src/application/chat_service/chat_service_send_background.rs` — `attempt_merge_auto_complete()` (line 653)

Currently auto-completion checks git state only (rebase/merge in progress, conflict markers, branch merged). It does NOT re-run validation commands.

When the merger was spawned for validation recovery (`validation_recovery: true` in metadata):
1. After checking git state is clean (no conflicts)
2. **Re-run validation commands** before completing
3. If validation passes → proceed to `complete_merge_internal()` → `Merged`
4. If validation STILL fails → revert (git reset HEAD~1) → `MergeIncomplete`

```rust
// In attempt_merge_auto_complete, after confirming clean git state:
if let Some(metadata_str) = &task.metadata {
    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(metadata_str) {
        if meta.get("validation_recovery").and_then(|v| v.as_bool()).unwrap_or(false) {
            // Re-run validation before completing
            if let Some(validation) = run_validation_commands(&project, &task, repo_path, task_id_str, app_handle) {
                if !validation.all_passed {
                    // Agent didn't fix it — revert and fall back to MergeIncomplete
                    let _ = GitService::reset_hard(repo_path, "HEAD~1");
                    task.metadata = Some(format_validation_error_metadata(...));
                    // Transition to MergeIncomplete
                    return;
                }
            }
        }
    }
}
```

**Note:** `run_validation_commands` is currently a private fn in `side_effects.rs` (line 391). It needs to be accessible from the auto-completion module. Options:
- Extract to a shared module (e.g., `validation.rs` helper)
- Or make it `pub(crate)` and import from side_effects

**Compilation unit note:** Also requires `ValidationResult`, `ValidationFailure`, `ValidationLogEntry`, `format_validation_error_metadata` to be `pub(crate)`. These are all in `side_effects.rs`. The `complete_merge_internal` fn is already `pub` — use same pattern.

---

### Task 3: Backend — Update merger agent context for validation recovery
**Dependencies:** Task 1
**Atomic Commit:** `feat(merge): add validation recovery context for merger agent`

**Files:**
- `src-tauri/src/application/chat_service/chat_service_context.rs` — Different initial message for validation recovery
- `ralphx-plugin/agents/merger.md` — Update system prompt

#### 3a. Different initial message

When spawning the merger agent for validation recovery (detected via `validation_recovery` metadata flag), send a different initial message:

```rust
// In the Merging on_enter or chat_service context builder:
let is_validation_recovery = task.metadata.as_ref()
    .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
    .and_then(|v| v.get("validation_recovery")?.as_bool())
    .unwrap_or(false);

let prompt = if is_validation_recovery {
    format!(
        "Fix validation failures for task: {}. The merge succeeded but post-merge \
         validation commands failed. The failing code is on the target branch. \
         Read the validation failures from task context, fix the code, run validation \
         to confirm, then commit your fixes.",
        task_id
    )
} else {
    format!("Resolve merge conflicts for task: {}", task_id)
};
```

#### 3b. Update merger agent system prompt

Add a section to `ralphx-plugin/agents/merger.md` explaining the validation recovery workflow:

```markdown
## Validation Recovery Mode

Sometimes you are spawned not because of git conflicts, but because post-merge validation
failed (build errors, lint failures, type errors). In this case:

1. The merge already succeeded — the code is on the target branch
2. There are NO conflict markers to resolve
3. Your job is to fix the build/validation errors

**How to detect:** Your initial message will say "Fix validation failures" instead of
"Resolve merge conflicts". The task metadata will contain `validation_recovery: true`
and `validation_failures` with error details.

**Workflow:**
1. Call `get_task_context(task_id)` — read validation failures from metadata
2. Call `get_project_analysis(project_id)` — get validation commands
3. Read the failing code and error output
4. Fix the code (edit files, add imports, fix types, etc.)
5. Run validation commands to confirm fixes work
6. If fixed: commit your changes and exit (auto-completion handles the rest)
7. If cannot fix: call `report_incomplete()` with explanation
```

---

### Task 4: Frontend — Show validation recovery context in MergingTaskDetail
**Dependencies:** Task 1
**Atomic Commit:** `feat(ui): show validation recovery context in MergingTaskDetail`

**File:** `src/components/tasks/detail-views/MergingTaskDetail.tsx` (331 LOC — under limit)

When the task is in `Merging` state with `validation_recovery: true` in metadata:
- Show "Fixing validation errors..." instead of "Resolving merge conflicts..."
- Display the validation failures that triggered the recovery (from metadata)
- Show the ValidationProgress component (from Phase 112) with the stored log

This is a small UI enhancement — detect the metadata flag and adjust the messaging and displayed context.

---

### Task 5: Frontend — Add AutoFix option to settings UI
**Dependencies:** Phase 112 Task 4 (settings UI with validation mode dropdown exists)
**Atomic Commit:** `feat(ui): add AutoFix option to validation mode settings`

**File:** `src/components/settings/GitSettingsSection.tsx`

Add "Auto-fix" to `VALIDATION_MODE_OPTIONS`:

```typescript
const VALIDATION_MODE_OPTIONS = [
  { value: "block", label: "Block on Failure", description: "Validation failure pauses merge — you decide" },
  { value: "auto_fix", label: "Auto-fix", description: "AI agent attempts to fix validation errors before asking you" },
  { value: "warn", label: "Warn on Failure", description: "Merge continues, validation issues logged as warnings" },
  { value: "off", label: "Disabled", description: "Skip post-merge validation entirely" },
];
```

**File:** `src/types/project.ts` — Add `auto_fix` to the Zod enum for `mergeValidationMode`.

---

## Files Modified

| # | File | Changes |
|---|------|---------|
| 1 | `src-tauri/src/domain/entities/project.rs` | Add `AutoFix` variant to `MergeValidationMode` |
| 2 | `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | AutoFix branch in handle_validation_failure, pass mode to all call sites |
| 3 | `src-tauri/src/application/chat_service/chat_service_send_background.rs` | Re-validate in auto-completion for recovery mode |
| 4 | `src-tauri/src/application/chat_service/chat_service_context.rs` | Different initial message for validation recovery |
| 5 | `ralphx-plugin/agents/merger.md` | Add validation recovery workflow section |
| 6 | `src/components/tasks/detail-views/MergingTaskDetail.tsx` | Detect recovery mode, show appropriate messaging |
| 7 | `src/components/settings/GitSettingsSection.tsx` | Add Auto-fix option to dropdown |
| 8 | `src/types/project.ts` | Add `auto_fix` to validation mode enum |

## Dependency Graph

```
Task 1 (AutoFix variant + recovery flow) ─┬─→ Task 2 (auto-completion re-validation)
                                           ├─→ Task 3 (merger agent context)
                                           └─→ Task 4 (MergingTaskDetail UI)
                                                       Task 5 (settings dropdown) — depends on Phase 112 Task 4
```

Tasks 2, 3, 4 are parallelizable (all depend only on Task 1, different files).

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
2. `cargo test` — all tests pass
3. `npm run lint && npm run typecheck` — frontend compiles
4. Manual: Set validation mode to "Auto-fix", trigger a merge with a task whose code has intentional build errors
   - Observe: validation fails → task enters Merging (not MergeIncomplete)
   - Observe: MergingTaskDetail shows "Fixing validation errors..." with failure details
   - Observe: merger agent fixes code, commits, exits
   - Observe: auto-completion re-runs validation → passes → Merged
5. Manual: Same setup but with unfixable errors → agent calls report_incomplete → MergeIncomplete
6. Manual: Verify Block mode still works as before (revert → MergeIncomplete immediately)

## File Analysis Notes (from source reading)

| File | Current State | Plan Impact |
|------|---------------|-------------|
| `project.rs` (263 LOC + 540 LOC tests) | `MergeValidationMode` has 3 variants: Block, Warn, Off. `Display`, `FromStr`, serde impls all use match. | Add `AutoFix` variant — must update ALL 3 match impls + tests. Additive, compiles alone. |
| `side_effects.rs` (~2164 LOC + tests) | `handle_validation_failure` at line 2118, 10 params, no `validation_mode` param. 3 call sites: lines 1506, 1696, 1884. Each call site already has `let validation_mode = &project.merge_validation_mode;` in scope. | Signature change + all 3 call sites must be in same task (Task 1). ✅ Plan is correct. |
| `chat_service_send_background.rs` | `attempt_merge_auto_complete` at line 653. Checks git state only (rebase, conflicts, branch merged). `run_validation_commands` is **private** in side_effects.rs (line 391). | Task 2 needs `pub(crate)` visibility for validation fns + types. Independent from Task 1's signature change. |
| `chat_service_context.rs` | Merge context at line 197 uses hardcoded "resolving merge conflicts" message. No metadata inspection. | Task 3 adds metadata-based message branching. Independent file, clean compilation unit. |
| `merger.md` (224 LOC) | No validation recovery section. Agent prompt assumes conflict resolution only. | Task 3 adds new section. Under plugin file size limit (100 LOC limit is for agents — this is a system prompt, not code). |
| `MergingTaskDetail.tsx` (331 LOC) | No metadata parsing for validation_recovery. | Task 4 adds conditional messaging. Under 500 LOC limit. |
| `GitSettingsSection.tsx` | **No validation mode options yet** — Phase 112 hasn't added frontend settings. | Task 5 depends on Phase 112 adding the dropdown first. Plan correctly notes this dependency. |
| `project.ts` (types) | **No `mergeValidationMode` field yet** — Phase 112 hasn't added frontend types. | Task 5 depends on Phase 112 adding the Zod schema first. |

## Compilation Unit Validation

| Check | Result |
|-------|--------|
| Task 1: Signature change to `handle_validation_failure` + all 3 call sites? | ✅ All in same task |
| Task 1: New `AutoFix` enum variant + all match impls (`Display`, `FromStr`, serde)? | ✅ All in same file (`project.rs`) |
| Task 2: Needs `pub(crate)` for `run_validation_commands` + related types? | ⚠️ Plan mentions extraction but should be explicit about visibility changes |
| Task 3: `chat_service_context.rs` change independent of other tasks? | ✅ Reads metadata (set by Task 1), different file |
| Task 4: Frontend reads metadata, no backend deps beyond Task 1? | ✅ Clean separation |
| Task 5: Depends on Phase 112 frontend types existing? | ✅ Correctly noted as dependency |

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
