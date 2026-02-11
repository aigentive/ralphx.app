# Execution Popover Active-Plan Filter Switches

## Summary
Add an `Active plan` switch to each execution popover card (Running, Queued, Merge) so users can filter popover content to the currently selected active plan. Keep execution bar counts project-wide exactly as they are today.

## Detail 1: UX and Behavior
- Add one independent switch in each popover header:
  - Running popover switch
  - Queued popover switch
  - Merge popover switch
- Switches are independent (not synchronized).
- Switch affects only popover content, not execution bar counters.
- If no active plan is selected for the project:
  - switch is disabled
  - effective filter is OFF
- Default behavior:
  - if active plan exists and user never chose a value for that popover: default ON
  - if user chose before: use saved value

## Detail 2: API and Data Contract Changes
Backend should provide plan association directly for running/merge rows.

Rust command response changes:
- `get_running_processes` (`src-tauri/src/commands/execution_commands.rs`)
  - add `ideation_session_id: Option<String>` to `RunningProcess`
- `get_merge_pipeline` (`src-tauri/src/commands/merge_pipeline_commands.rs`)
  - add `ideation_session_id: Option<String>` to `MergePipelineTask`

Frontend schema/type/transform updates:
- `src/api/running-processes.schemas.ts`
- `src/api/running-processes.types.ts`
- `src/api/running-processes.transforms.ts`
- `src/api/merge-pipeline.schemas.ts`
- `src/api/merge-pipeline.types.ts`
- `src/api/merge-pipeline.transforms.ts`

Compatibility:
- Treat new field as optional in frontend schema so rollout is backward-compatible.

## Detail 3: Frontend State and Filtering Rules
Preference persistence:
- Persist per project and per popover key (`running`, `queued`, `merge`) in UI state (or a dedicated tiny store).
- Suggested persisted shape:
  - `executionPlanFilterPrefsByProject[projectId][popoverKey] = boolean | undefined`

Effective filter formula:
- `effectiveFilter = activePlanId ? (savedPref ?? true) : false`

Filtering logic:
- Running: include process when `process.ideationSessionId === activePlanId`.
- Queued: include queued task when `task.ideationSessionId === activePlanId`.
- Merge: filter each section (`active`, `waiting`, `needsAttention`) by `task.ideationSessionId === activePlanId`.

Execution bar invariants:
- Keep `runningCount`, `queuedCount`, `mergingCount` project-wide and unchanged by switches.

## Detail 4: Testing and Acceptance
Automated tests:
- Backend command tests: verify `ideation_session_id` is included in running/merge response payloads.
- Frontend API tests: verify parsing and transform for new optional field.
- Popover UI tests:
  - switch visible in each popover header
  - switch disabled when no active plan
  - default ON when active plan exists and no saved pref
  - persistence per project/per popover
  - content filtering correctness for running/queued/merge
- Execution bar tests:
  - counts remain project-wide regardless of switch states

Manual acceptance scenarios:
1. Active plan selected, untouched switches: popovers show only active-plan items.
2. Toggle OFF only merge switch: merge popover shows all merge items; others remain filtered.
3. Clear active plan: switches disabled and all popovers show project-wide items.
4. Reload app: per-project switch preferences restore.
