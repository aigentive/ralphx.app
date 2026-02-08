# Real-Time Merge Validation Streaming + Configurable Validation Mode

## Context

The `pending_merge` detail view shows static fake progress steps ("Fetching latest changes", "Rebasing onto base branch"...) with no visibility into what actually happens during post-merge validation. When validation fails (→ MergeIncomplete), the user only sees a terse error message. They can't see which commands ran, what output they produced, or why they failed. There's also no way to skip validation and proceed with the merge when failures are expected/acceptable.

**Goals:**
1. Stream validation command output in real-time to the MergingTaskDetail view
2. Store full validation log in metadata for historical viewing
3. Make validation behavior configurable (block/warn/off) per project
4. Allow users to "Continue Anyway" when validation fails

## Plan

### Task 1: Backend — Emit validation events + store full log (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(merge): stream validation progress events and store full log`
**Compilation Unit:** Single file — `side_effects.rs`. Changes `run_validation_commands` signature (3→5 params) + all 3 call sites (lines ~1277, ~1448, ~1619) + 8 test call sites (lines ~2190-2278) + `ValidationResult` struct (line 347) + `format_validation_error_metadata` signature (line 524) + `handle_validation_failure` signature (line 1847). All within one file — compiles after this task alone.

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

#### 1a. Add `ValidationLogEntry` struct + expand `ValidationResult`

```rust
#[derive(Debug, Clone, serde::Serialize)]
struct ValidationLogEntry {
    phase: String,       // "setup" or "validate"
    command: String,
    path: String,
    label: String,
    status: String,      // "success" or "failed"
    exit_code: Option<i32>,
    stdout: String,      // truncated to 2000 chars
    stderr: String,      // truncated to 2000 chars
    duration_ms: u64,
}

struct ValidationResult {
    all_passed: bool,
    failures: Vec<ValidationFailure>,
    log: Vec<ValidationLogEntry>,  // ALL commands (success + failure)
}
```

#### 1b. Add `app_handle` + `task_id_str` params to `run_validation_commands`

```rust
fn run_validation_commands(
    project: &Project,
    task: &Task,
    merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> Option<ValidationResult>
```

Update all 3 call sites (lines ~1277, ~1448, ~1619) to pass `task_id_str` and `app_handle`:
```rust
if let Some(validation) = run_validation_commands(
    &project, &task, repo_path, task_id_str,
    self.machine.context.services.app_handle.as_ref(),
) { ... }
```

Update all test call sites (~8 tests) to pass `""` and `None`.

#### 1c. Emit `merge:validation_step` events during command execution

For each setup and validate command, emit two events (before + after):

**Before:**
```rust
if let Some(handle) = app_handle {
    let _ = handle.emit("merge:validation_step", serde_json::json!({
        "task_id": task_id_str,
        "phase": "setup", // or "validate"
        "command": resolved_cmd,
        "path": resolved_path,
        "label": entry.label,
        "status": "running",
    }));
}
```

**After (with timing):**
```rust
let start = std::time::Instant::now();
let result = Command::new("sh").arg("-c").arg(&resolved_cmd)...;
let duration_ms = start.elapsed().as_millis() as u64;

// Build log entry
let log_entry = ValidationLogEntry { phase, command, path, label, status, exit_code, stdout, stderr, duration_ms };
log.push(log_entry.clone());

if let Some(handle) = app_handle {
    let _ = handle.emit("merge:validation_step", serde_json::json!({
        "task_id": task_id_str,
        "phase": log_entry.phase,
        "command": log_entry.command,
        "path": log_entry.path,
        "label": log_entry.label,
        "status": log_entry.status,
        "exit_code": log_entry.exit_code,
        "stdout": log_entry.stdout,
        "stderr": log_entry.stderr,
        "duration_ms": log_entry.duration_ms,
    }));
}
```

#### 1d. Store full validation_log in metadata

**On failure** — expand `format_validation_error_metadata` to include the full log:
```rust
fn format_validation_error_metadata(
    failures: &[ValidationFailure],
    log: &[ValidationLogEntry],
    source_branch: &str,
    target_branch: &str,
) -> String {
    serde_json::json!({
        "error": format!("..."),
        "validation_failures": failure_details,
        "validation_log": log,  // ADD — full log for UI
        "source_branch": source_branch,
        "target_branch": target_branch,
    }).to_string()
}
```

Update `handle_validation_failure` to accept and pass the full log.

**On success** — store validation_log in metadata before calling `complete_merge_internal`:
```rust
if validation.all_passed {
    task.metadata = Some(serde_json::json!({
        "validation_log": validation.log,
        "source_branch": source_branch,
        "target_branch": target_branch,
    }).to_string());
    // ... proceed to complete_merge_internal
}
```

---

### Task 2: Backend — Configurable validation mode + skip-validation retry (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(merge): add merge_validation_mode setting and skip-validation retry`
**Compilation Unit:** Multi-file backend. `MergeValidationMode` enum + `Project` field + `from_row()` + `new()` + `new_with_worktree()` must be in same commit (project.rs lines 69-150, 181-200). Migration (v20) adds column with DEFAULT so existing rows work. `retry_merge` adds optional param (additive, Tauri handles missing). `side_effects.rs` reads new field (requires Task 1's `validation.log`). Test DB `setup_test_db()` (project.rs line 576) needs new column. All backend — compiles after this task.

#### 2a. Add `merge_validation_mode` to Project entity

**File:** `src-tauri/src/domain/entities/project.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeValidationMode {
    #[default]
    Block,  // Validation failure → MergeIncomplete (user decides)
    Warn,   // Validation failure → proceed to Merged, store warnings
    Off,    // Skip validation entirely
}
```

Add to `Project` struct:
```rust
pub merge_validation_mode: MergeValidationMode,
```

Update `from_row()`, `new()`, serialization.

#### 2b. Migration

**File:** `src-tauri/src/infrastructure/sqlite/migrations/v20_merge_validation_mode.rs` (register in mod.rs, bump SCHEMA_VERSION to 20)

```sql
ALTER TABLE projects ADD COLUMN merge_validation_mode TEXT NOT NULL DEFAULT 'block';
```

#### 2c. Respect mode in `attempt_programmatic_merge`

In the validation check after merge success:
```rust
let mode = &project.merge_validation_mode;
if *mode == MergeValidationMode::Off {
    // Skip validation entirely
} else if let Some(validation) = run_validation_commands(...) {
    if !validation.all_passed {
        if *mode == MergeValidationMode::Warn {
            // Store warnings but proceed to merged
            task.metadata = Some(format_validation_warn_metadata(&validation.log, ...));
            // Continue to complete_merge_internal
        } else {
            // Block mode: revert and go to MergeIncomplete
            self.handle_validation_failure(...).await;
            return;
        }
    }
}
```

#### 2d. Add `skip_validation` param to `retry_merge` command

**File:** `src-tauri/src/commands/git_commands.rs`

```rust
pub async fn retry_merge(
    task_id: String,
    skip_validation: Option<bool>,  // ADD
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
) -> Result<(), String> {
```

When `skip_validation == Some(true)`:
- Set task metadata `{"skip_validation": true}` before transitioning to PendingMerge
- In `attempt_programmatic_merge`, check for this flag and skip validation if present
- Clear the flag after merge attempt (success or failure)

---

### Task 3: Frontend — Live validation display in MergingTaskDetail (BLOCKING)
**Dependencies:** Task 1 (backend emits events consumed by frontend hook)
**Atomic Commit:** `feat(merge): real-time validation progress in MergingTaskDetail`
**Compilation Unit:** Frontend-only. New event schema (events.ts — additive), new hook file (useMergeValidationEvents.ts — new), updated component (MergingTaskDetail.tsx — 332 lines currently, will stay under 500). All TS types are self-contained. Compiles independently of backend changes (events arrive at runtime).

#### 3a. Add event type

**File:** `src/types/events.ts`

```typescript
export const MergeValidationStepEventSchema = z.object({
  task_id: z.string(),
  phase: z.enum(["setup", "validate"]),
  command: z.string(),
  path: z.string(),
  label: z.string(),
  status: z.enum(["running", "success", "failed"]),
  exit_code: z.number().nullable().optional(),
  stdout: z.string().optional(),
  stderr: z.string().optional(),
  duration_ms: z.number().optional(),
});
export type MergeValidationStepEvent = z.infer<typeof MergeValidationStepEventSchema>;
```

#### 3b. Create `useMergeValidationEvents` hook

**File:** `src/hooks/useMergeValidationEvents.ts`

Local hook (not global — subscribes when MergingTaskDetail mounts):

```typescript
export function useMergeValidationEvents(taskId: string) {
  const [steps, setSteps] = useState<MergeValidationStepEvent[]>([]);
  const bus = useEventBus();

  useEffect(() => {
    setSteps([]);
    const unsub = bus.subscribe<unknown>("merge:validation_step", (payload) => {
      const parsed = MergeValidationStepEventSchema.safeParse(payload);
      if (!parsed.success || parsed.data.task_id !== taskId) return;
      const step = parsed.data;
      setSteps(prev => {
        // Update existing step (running→success/failed) or append new
        const idx = prev.findIndex(s => s.command === step.command && s.phase === step.phase);
        if (idx >= 0) {
          const updated = [...prev];
          updated[idx] = step;
          return updated;
        }
        return [...prev, step];
      });
    });
    return unsub;
  }, [bus, taskId]);

  return steps;
}
```

#### 3c. Build `ValidationProgress` component

**File:** Update `src/components/tasks/detail-views/MergingTaskDetail.tsx`

Replace/augment the static `MergeProgressSteps` with a new `ValidationProgress` section that shows when validation steps exist (either from live events or metadata).

**Component structure:**
```
ValidationProgress
  ├── ValidationStepRow  (per command)
  │     ├── Status icon (spinner/check/X)
  │     ├── Phase badge ("Setup" / "Validate")
  │     ├── Command text (monospace)
  │     ├── Duration badge (when complete)
  │     └── Collapsible terminal output (stdout/stderr)
  │         - Auto-expanded: running or failed
  │         - Collapsed: success
  │         - Monospace, dark background, scrollable, max-height
```

**Data sources:**
- **Live:** `useMergeValidationEvents(task.id)` → real-time events while pending_merge
- **Historical:** Parse `task.metadata.validation_log` for past merge attempts (merge_incomplete, merged, or time-travel views)
- **Merge:** Prefer live events; if empty and metadata has validation_log, show metadata

**Styling:** CI/CD pipeline aesthetic with dark terminal blocks:
- Background: `rgba(0,0,0,0.3)`
- Font: monospace, 12px
- Max-height with scroll for long output
- Collapsible via chevron toggle

---

### Task 4: Frontend — Settings UI + "Continue Anyway" button
**Dependencies:** Task 2 (backend `skip_validation` param + `mergeValidationMode` field), Task 3 (reuses `ValidationProgress` component)
**Atomic Commit:** `feat(merge): validation mode settings and skip-validation UI`
**Compilation Unit:** Frontend-only. Adds `mergeValidationMode` to TS `Project` type + schema + transform (project.ts lines 15-75 — must update all three together). Settings UI uses existing `SelectSettingRow`. MergeIncomplete adds optional button (additive). Compiles after this task.

#### 4a. Settings UI

**File:** `src/components/settings/GitSettingsSection.tsx`

Add a new select row to the Git Settings section:

```typescript
const VALIDATION_MODE_OPTIONS = [
  { value: "block", label: "Block on Failure", description: "Validation failure pauses merge — you decide" },
  { value: "warn", label: "Warn on Failure", description: "Merge continues, validation issues logged as warnings" },
  { value: "off", label: "Disabled", description: "Skip post-merge validation entirely" },
];
```

Use existing `SelectSettingRow` component. Update via `update_project_settings` command.

**File:** `src/types/project.ts` — Add `mergeValidationMode` to Project type.

#### 4b. "Retry (Skip Validation)" button in MergeIncompleteTaskDetail

**File:** `src/components/tasks/detail-views/MergeIncompleteTaskDetail.tsx`

When the merge error metadata contains `validation_failures` (validation-caused failure):
- Show the `ValidationProgress` component (reuse from Task 3) with the stored log
- Add a third button: "Retry (Skip Validation)" alongside existing Retry and Mark Resolved
- This button calls `retry_merge` with `{ skipValidation: true }`

```typescript
const handleRetrySkipValidation = useCallback(async () => {
  setIsProcessing(true);
  await invoke("retry_merge", { taskId: task.id, skipValidation: true });
  // ...
}, [task.id]);
```

---

## Files Modified

| # | File | Changes |
|---|------|---------|
| 1 | `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | ValidationLogEntry, emit events, store log, respect mode |
| 2 | `src-tauri/src/domain/entities/project.rs` | MergeValidationMode enum, Project field |
| 3 | `src-tauri/src/infrastructure/sqlite/migrations/v20_merge_validation_mode.rs` | Migration for merge_validation_mode column |
| 3b | `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` | Register v20, bump SCHEMA_VERSION to 20 |
| 4 | `src-tauri/src/commands/git_commands.rs` | skip_validation param on retry_merge |
| 5 | `src/types/events.ts` | MergeValidationStepEvent schema |
| 6 | `src/types/project.ts` | mergeValidationMode field |
| 7 | `src/hooks/useMergeValidationEvents.ts` | New hook |
| 8 | `src/components/tasks/detail-views/MergingTaskDetail.tsx` | ValidationProgress section |
| 9 | `src/components/tasks/detail-views/MergeIncompleteTaskDetail.tsx` | Validation log display + skip button |
| 10 | `src/components/settings/GitSettingsSection.tsx` | Validation mode selector |

## Dependency Graph

```
Task 1 (Backend: events + log)  ─┬─→  Task 2 (Backend: config + skip)  ─┐
                                 │                                        │
                                 └─→  Task 3 (Frontend: live display)  ──┼─→  Task 4 (Frontend: settings + skip UI)
```

**Parallelizable:** Tasks 2 and 3 can execute in parallel (both depend only on Task 1, different layers).

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
2. `cargo test` — all tests pass (update ~8 test call sites for new params)
3. `npm run lint && npm run typecheck` — frontend compiles
4. Manual: Trigger a merge for a task with feature branch target → see live validation streaming in UI
5. Manual: When validation fails, see full output in MergeIncomplete view, click "Retry (Skip Validation)"
6. Manual: Change validation mode in Settings → verify behavior matches mode

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
- Tasks 2 and 3 can execute in parallel since they modify different layers (Rust vs TypeScript)
