# Investigation: Merge Flow Issues (Task d522f82a)

## Executive Summary

The task `d522f82a-7d68-4b4b-822e-d84f7d077a45` ended up in `merge_conflict` status despite having no actual git conflicts. This investigation reveals **3 root causes** and **5 related bugs**.

---

## What Happened (Timeline)

| Time | Event | State |
|------|-------|-------|
| 20:45:14 | Task created | `ready` |
| 20:45:15 | Execution started | `executing` |
| 20:48:33 | Execution completed | `pending_review` |
| 20:48:33 | Review auto-started | `reviewing` |
| 20:49:33 | Review passed | `approved` |
| 20:49:33 | Auto-transition | `pending_merge` |
| 20:49:34 | **Programmatic merge failed** | `merging` (changed_by: `merge_error`) |
| 20:49:35-53 | Merger agent runs, MCP tools fail | `merging` |
| 20:49:53 | **Auto-complete finds branch not merged** | `merge_conflict` |

---

## Root Cause Analysis

### Root Cause 1: Programmatic Merge Cannot Work in Worktree Mode (CONFIRMED)

**Location:** `src-tauri/src/application/git_service.rs:730`

**Root Cause:** `try_rebase_and_merge` tries to `git checkout task_branch` in the main repo, but in worktree mode, the task branch is already checked out in the worktree. Git refuses to checkout a branch that's checked out elsewhere:

```
fatal: 'ralphx/ralphx/task-d522f82a-...' is already checked out at '/Users/lazabogdan/ralphx-worktrees/ralphx/task-...'
```

**Code path that fails:**
```rust
// git_service.rs:730
Self::checkout_branch(repo, task_branch)?;  // FAILS!
```

**Evidence:**
- State history shows `changed_by: merge_error` (not `merge_conflict`)
- Task has `worktree_path` set (worktree was active during merge attempt)
- Working projects (ralphx-demo-2, ralphx-demo-3) may not have had active worktrees during their merges

**Additional finding:** The `ralphx` project is also missing settings that the working projects have:

| Setting | ralphx (failing) | ralphx-demo-2 (working) |
|---------|------------------|-------------------------|
| `base_branch` | **EMPTY** | `main` |
| `worktree_path` | **EMPTY** | `~/ralphx-worktrees/ralphx-demo-2` |
| `worktree_branch` | **EMPTY** | `ralphx/ralphx-demo-2` |

These empty settings may cause issues in the UI (showing "worktree" instead of branch name) but the missing `base_branch` already has a fallback to "main" in the code.

---

### Root Cause 2: Merger Agent Misunderstood Its Task

The merger agent received prompt: `"Resolve merge conflicts for task: {task_id}"`

**What agent did:**
1. Checked git status (clean)
2. Ran `git rebase main` (no-op, already up to date)
3. Called `complete_merge` with commit SHA from **worktree branch** (not main)

**What agent should have done:**
1. Actually merge/push the task branch to main
2. Get the **merge commit SHA from main** (not the task branch)
3. Call `complete_merge` with that SHA

**Bug:** The agent never actually merged the branch - it just checked git status and tried to complete.

---

### Root Cause 3: MCP Tool Error Messages Lost

**Location:** `ralphx-plugin/ralphx-mcp-server/src/tauri-client.ts:47-58`

When the Tauri backend returns an error:
```rust
return Err((StatusCode::BAD_REQUEST, "detailed error message".to_string()));
```

The MCP client tries to parse as JSON (fails because it's plain text), then falls back to:
```
"Tauri API error: Bad Request"
```

**Result:** Agent never saw the actual error:
```
"Commit 5564448... is not on main branch. The merge may not have completed successfully."
```

---

## Bugs Found

### Bug 1: Empty base_branch in Project Settings

**Location:** DB query shows `base_branch: ""`
**Impact:** Code defaults to `"main"` but this should be explicit
**Fix:** Ensure project migration sets `base_branch` properly when switching git modes

### Bug 2: Programmatic Merge Error Path Unclear

**Location:** `side_effects.rs:932-957`
**Impact:** When git operations fail (not conflicts), task enters `merging` with `merge_error` reason, but agent still receives "resolve conflicts" prompt
**Fix:**
1. Log the actual error that caused the failure
2. Consider different prompt for `merge_error` vs `merge_conflict`

### Bug 3: MCP Error Response Format Mismatch

**Location:**
- Backend: `src-tauri/src/http_server/handlers/git.rs` returns `(StatusCode, String)`
- Client: `tauri-client.ts` expects `{ error: string, details?: string }`

**Impact:** Detailed error messages lost, agent can't diagnose issues
**Fix:** Standardize error response format to JSON

### Bug 4: complete_merge Expects Commit On Main (Not Worktree)

**Location:** `git.rs:130-140`
```rust
if !GitService::is_commit_on_branch(&repo_path, &req.commit_sha, base_branch)
```

**Impact:** Agent provides worktree commit SHA, validation fails
**Confusion:** Tool description says "after completing the rebase/merge" but agent interpreted this as "rebase in worktree"
**Fix:** Either:
1. Update tool description to be explicit about what SHA is expected
2. Or have `complete_merge` accept worktree commit and do the final merge itself

### Bug 5: attempt_merge_auto_complete Transitions to MergeConflict for Non-Conflicts

**Location:** `chat_service_send_background.rs:846-876`

When the agent exits without completing, `attempt_merge_auto_complete` checks if commit is on main. If not, it transitions to `merge_conflict` even though there's no actual conflict.

**Impact:** Task stuck in misleading state - says "merge_conflict" but there are no conflicts to resolve
**Fix:** Either:
1. Use a different status like `merge_incomplete`
2. Or have the auto-complete actually perform the merge if worktree is clean

---

## Questions for User

Before proceeding with fixes, I need clarification:

1. **Scope:** Do you want to fix all 5 bugs, or prioritize the most critical ones?

2. **Error Response Format:** Should backend errors be:
   - JSON format: `{ "error": "message", "details": "optional" }`
   - Or structured Axum error responses?

3. **Programmatic Merge Investigation:** Should we add better logging/diagnostics to understand why the initial programmatic merge failed? This might require:
   - Adding detailed error logging in `attempt_programmatic_merge`
   - Possibly checking worktree setup before merge

4. **Merger Agent Behavior:** When merge fails with `merge_error` (not conflicts), should the agent:
   - Try to diagnose and fix the underlying issue?
   - Or escalate to human with diagnostic info?

---

## Implementation Plan

### Task 0 (CRITICAL): Fix Programmatic Merge for Worktree Mode (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(git): delete worktree before programmatic merge to unlock branch`

**Files:**
- `src-tauri/src/application/git_service.rs`
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

**Problem:**
`try_rebase_and_merge` uses `git checkout task_branch` which fails in worktree mode because the branch is locked to the worktree.

**Solution Options:**

**Option A: Delete worktree first, then merge (RECOMMENDED)**
In `attempt_programmatic_merge`, before calling `try_rebase_and_merge`:
1. If task has `worktree_path`, delete the worktree first
2. This unlocks the branch for checkout
3. Proceed with normal merge
4. (Worktree already planned for cleanup after merge)

```rust
// side_effects.rs - before try_rebase_and_merge
if project.git_mode == GitMode::Worktree {
    if let Some(worktree_path) = &task.worktree_path {
        let worktree_path_buf = PathBuf::from(worktree_path);
        if worktree_path_buf.exists() {
            // Delete worktree to unlock the branch
            GitService::delete_worktree(repo_path, &worktree_path_buf)?;
        }
    }
}
```

**Option B: Merge without checkout (advanced)**
Add a new merge method that doesn't require checking out the task branch:
```rust
// Merge by commit SHA instead of branch name
git merge <commit_sha>  // Works without checking out task branch
```

**Option C: Operate in worktree instead of main repo**
Have the programmatic merge happen in the worktree, then fast-forward main.

**Recommendation:** Option A is simplest and aligns with planned cleanup.

---

### Task 1: Add `MergeIncomplete` Status (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(state-machine): add MergeIncomplete status with transitions`

**Files:**
- `src-tauri/src/domain/entities/status.rs`
- `src-tauri/src/domain/state_machine/machine/transitions.rs`
- `src-tauri/src/domain/state_machine/machine/types.rs`
- `src-tauri/src/domain/state_machine/events.rs`

**Changes:**
1. Add `MergeIncomplete` variant to `InternalStatus` enum
2. Add transitions:
   - `Merging` → `MergeIncomplete` (on agent error)
   - `MergeIncomplete` → `Merging` (on retry)
3. Add `MergeAgentError` event to trigger transition
4. Add serde rename for `merge_incomplete`

**Reuse:**
- Follow existing pattern from `MergeConflict` status

---

### Task 2: Fix MCP Error Response Format (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(http): standardize error responses to JSON format`

**Files:**
- `src-tauri/src/http_server/handlers/git.rs`
- `ralphx-plugin/ralphx-mcp-server/src/tauri-client.ts`

**Backend Changes (git.rs):**
Replace all error returns from:
```rust
return Err((StatusCode::BAD_REQUEST, "message".to_string()));
```
To:
```rust
return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
    "error": "message",
    "details": optional_details
}))));
```

**Affected functions:**
- `complete_merge` (lines 79-84, 107-115, 130-140)
- `report_conflict` (lines 260-268)

**Client Changes (tauri-client.ts):**
Update error handling to gracefully handle both plain text and JSON responses:
```typescript
try {
    const errorData = await response.json();
    errorMessage = errorData.error || JSON.stringify(errorData);
} catch {
    // Plain text response
    errorMessage = await response.text() || response.statusText;
}
```

---

### Task 3: Add Diagnostic Logging for Merge Failures
**Dependencies:** None
**Atomic Commit:** `chore(git): add diagnostic logging for merge failures`

**Files:**
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`
- `src-tauri/src/application/git_service.rs`

**Changes:**
1. In `attempt_programmatic_merge` error path (lines 932-957), add detailed logging:
   ```rust
   tracing::error!(
       task_id = task_id_str,
       error = %e,
       worktree_path = ?task.worktree_path,
       task_branch = ?task.task_branch,
       base_branch = ?project.base_branch,
       "Programmatic merge failed - DIAGNOSTIC INFO"
   );
   ```

2. In `GitService::attempt_rebase_and_merge`, log each step:
   - Worktree validation
   - Branch checkout
   - Fetch result
   - Rebase result
   - Merge result

---

### Task 4: Use MergeIncomplete for Non-Conflict Failures
**Dependencies:** Task 1
**Atomic Commit:** `feat(merge): use MergeIncomplete status for non-conflict failures`

**Files:**
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`
- `src-tauri/src/application/chat_service/chat_service_send_background.rs`
- `src-tauri/src/http_server/handlers/git.rs`

**Changes in side_effects.rs:**
In `attempt_programmatic_merge` error path, change:
```rust
task.internal_status = InternalStatus::Merging;
// ... record with "merge_error"
```
To:
```rust
task.internal_status = InternalStatus::MergeIncomplete;
// ... record with "merge_incomplete"
// Use different prompt: "Merge failed for task: {task_id}. Error: {e}. Diagnose and fix."
```

**Changes in chat_service_send_background.rs:**
In `attempt_merge_auto_complete`, when branch not merged to main:
- Change transition from `MergeConflict` to `MergeIncomplete`
- Update reason message

**Changes in git.rs:**
Add new endpoint `retry_merge` or allow `Merging` entry from `MergeIncomplete`:
```rust
// POST /api/git/tasks/{id}/retry-merge
pub async fn retry_merge(...) -> Result<...> {
    // Validate task is MergeIncomplete
    // Spawn merger agent with context
}
```

---

### Task 5: Improve complete_merge Tool Description
**Dependencies:** None
**Atomic Commit:** `docs(mcp): clarify complete_merge tool requirements`

**Files:**
- `ralphx-plugin/ralphx-mcp-server/src/tools.ts`

**Change tool description (line 393-395):**
```typescript
{
    name: "complete_merge",
    description:
      "Signal successful merge completion. IMPORTANT: Call this AFTER you have:" +
      "\n1. Resolved all conflicts (if any)" +
      "\n2. Merged the task branch INTO main (git checkout main && git merge <task-branch>)" +
      "\n3. Obtained the merge commit SHA from main (git rev-parse HEAD on main)" +
      "\n\nThe commit_sha must be the SHA of a commit ON the main branch, not the task branch.",
    // ...
}
```

---

### Task 6: Fix Project Settings and Add Fallback Defaults
**Dependencies:** None
**Atomic Commit:** `fix(project): add fallback defaults for git mode settings and migration`

**Files:**
- `src-tauri/src/domain/entities/project.rs`
- `src-tauri/src/commands/project_commands.rs`
- `src-tauri/src/application/project_service.rs`
- `src-tauri/src/infrastructure/sqlite/sqlite_project_repo.rs`
- `src-tauri/src/infrastructure/sqlite/migrations/` (new migration file)

**Part A: Fix the ralphx project (one-time migration)**
Update DB to set missing values for the ralphx project:
```sql
UPDATE projects
SET base_branch = 'main',
    worktree_parent_directory = '~/ralphx-worktrees'
WHERE id = 'c0738550-18bb-4444-afef-cc75b1de2431'
  AND base_branch IS NULL OR base_branch = '';
```

**Part B: Add fallback defaults in Project entity**
In `project.rs`, add getter methods that provide defaults:
```rust
impl Project {
    pub fn base_branch_or_default(&self) -> &str {
        self.base_branch.as_deref().filter(|s| !s.is_empty()).unwrap_or("main")
    }

    pub fn worktree_parent_or_default(&self) -> &str {
        self.worktree_parent_directory.as_deref().filter(|s| !s.is_empty()).unwrap_or("~/ralphx-worktrees")
    }
}
```

**Part C: Validate on git mode change**
When updating project `git_mode` to `worktree`:
1. If `base_branch` is empty, set to "main"
2. If `worktree_parent_directory` is empty, set to default
3. Log warnings if validation needed

**Part D: Fix all existing projects (migration)**
Add a migration that fixes all projects with git_mode=worktree but missing settings:
```rust
// Migration: v24_fix_worktree_project_settings.rs
UPDATE projects
SET base_branch = 'main'
WHERE git_mode = 'worktree'
  AND (base_branch IS NULL OR base_branch = '');

UPDATE projects
SET worktree_parent_directory = '~/ralphx-worktrees'
WHERE git_mode = 'worktree'
  AND (worktree_parent_directory IS NULL OR worktree_parent_directory = '');
```

---

### Task 7: Add report_incomplete Endpoint
**Dependencies:** Task 1, Task 2
**Atomic Commit:** `feat(mcp): add report_incomplete endpoint and tool for merge failures`

**Files:**
- `src-tauri/src/http_server/handlers/git.rs`
- `ralphx-plugin/ralphx-mcp-server/src/tools.ts`
- `ralphx-plugin/ralphx-mcp-server/src/index.ts`

**Add new endpoint:**
```rust
/// POST /api/git/tasks/{id}/report-incomplete
pub async fn report_incomplete(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
    Json(req): Json<ReportIncompleteRequest>,
) -> Result<Json<MergeOperationResponse>, (StatusCode, String)> {
    // Validate task is in Merging status
    // Transition to MergeIncomplete
    // Store error/diagnostic info
}
```

**Add MCP tool:**
```typescript
{
    name: "report_incomplete",
    description: "Report that merge cannot be completed due to non-conflict errors (e.g., git operation failures, missing configuration). Use this instead of report_conflict when there are no actual merge conflicts.",
    inputSchema: {
        type: "object",
        properties: {
            task_id: { type: "string", description: "Task ID" },
            reason: { type: "string", description: "Detailed explanation of why merge failed" },
            diagnostic_info: { type: "string", description: "Git status, logs, or other diagnostic output" }
        },
        required: ["task_id", "reason"]
    }
}

---

## Task Dependencies

```
Task 0 (CRITICAL: Worktree merge fix) ────→ (required for worktree mode to work AT ALL)

Task 1 (MergeIncomplete status) ──────┬──→ Task 4 (Use MergeIncomplete)
                                      │
Task 2 (MCP error format) ────────────┴──→ Task 7 (report_incomplete endpoint)

Task 3 (Diagnostic logging) ──────────────→ (independent)

Task 5 (Tool description) ────────────────→ (independent)

Task 6 (Project settings) ────────────────→ (includes migration)
```

**Execution order:**
1. **Task 0 (worktree fix)** - CRITICAL: must be first, fixes root cause
2. Task 6 (project settings + migration) - fixes existing bad data
3. Task 1 (status) - required foundation for better error handling
4. Task 2 (MCP errors) - improves debugging
5. Tasks 3, 5 in parallel - independent improvements
6. Task 4 (use MergeIncomplete) - requires Task 1
7. Task 7 (new endpoint) - requires Tasks 1, 2

---

## Verification Plan

### 1. Unit Tests for New Status

```rust
// In machine/tests.rs
#[test]
fn test_merging_to_merge_incomplete_on_error() {
    // Verify Merging → MergeIncomplete transition works
}

#[test]
fn test_merge_incomplete_to_merging_on_retry() {
    // Verify retry path works
}
```

### 2. Test MCP Error Handling

```bash
# Call complete_merge with invalid SHA
curl -X POST http://localhost:3847/api/git/tasks/invalid-task/complete-merge \
  -H "Content-Type: application/json" \
  -d '{"commit_sha": "invalid"}'

# Verify JSON error response with details
```

### 3. Integration Test: Merge Flow

1. Create task in worktree mode project
2. Execute task (creates commit in worktree)
3. Approve task
4. Verify programmatic merge behavior:
   - If successful: task → `merged`
   - If conflict: task → `merging` (agent spawned)
   - If error: task → `merge_incomplete` (with diagnostic info)

### 4. Test Error Propagation

1. Manually trigger `complete_merge` with worktree commit SHA
2. Verify agent sees detailed error: "Commit xxx is not on main branch"
3. Verify agent can call `report_incomplete` with diagnostic info

---

## Critical Files

| File | Purpose | Tasks |
|------|---------|-------|
| `src-tauri/src/application/git_service.rs` | Git operations | 0 |
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | Merge logic | 0, 3, 4 |
| `src-tauri/src/domain/entities/status.rs` | Status enum | 1 |
| `src-tauri/src/domain/state_machine/machine/transitions.rs` | State transitions | 1 |
| `src-tauri/src/domain/state_machine/machine/types.rs` | State types | 1 |
| `src-tauri/src/domain/state_machine/events.rs` | Events | 1 |
| `src-tauri/src/http_server/handlers/git.rs` | Merge endpoints | 2, 4, 7 |
| `ralphx-plugin/ralphx-mcp-server/src/tauri-client.ts` | HTTP client | 2 |
| `src-tauri/src/application/chat_service/chat_service_send_background.rs` | Auto-complete | 4 |
| `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | Tool definitions | 5, 7 |
| `ralphx-plugin/ralphx-mcp-server/src/index.ts` | Tool routing | 7 |
| `src-tauri/src/domain/entities/project.rs` | Project entity | 6 |
| `src-tauri/src/application/project_service.rs` | Project updates | 6 |
| `src-tauri/src/infrastructure/sqlite/migrations/` | DB migrations | 6 |

---

## Estimated Effort

| Task | Complexity | Est. Time |
|------|------------|-----------|
| **0. Worktree merge fix** | **Medium** | **1 hour** |
| 1. MergeIncomplete status | Medium | 1-2 hours |
| 2. MCP error format | Easy | 30 min |
| 3. Diagnostic logging | Easy | 30 min |
| 4. Use MergeIncomplete | Medium | 1 hour |
| 5. Tool description | Easy | 15 min |
| 6. Project settings + migration | Medium | 1 hour |
| 7. report_incomplete endpoint | Medium | 1 hour |

**Total:** ~6-7 hours

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Compilation Unit Validation

**Task 1 is a compilation unit** requiring changes to 4 files:
1. `status.rs` - Add `MergeIncomplete` variant
2. `types.rs` - Add `State::MergeIncomplete` variant
3. `transitions.rs` - Add handler for MergeIncomplete state
4. `events.rs` - Add `MergeAgentError` event (if needed)

All must be in one commit - adding variant to enum without handlers breaks compilation.

**Task 4 depends on Task 1** - uses `InternalStatus::MergeIncomplete` which won't exist until Task 1 is complete.

**Task 7 depends on Tasks 1 and 2** - uses MergeIncomplete status and JSON error format.
