# Implement Hybrid Merge Completion Detection

**Approach:** Auto-detect merge success, keep `report_conflict` for agent failure reporting

## Background

> Why are we relying on an agent to call `complete_merge` through MCP when we could detect this programmatically? We know the base branch, the merge location, and the origin branch. The programmatic merge detects success automatically — why not the agent merge?

## Current Architecture

### Programmatic Merge (Fast Path)
```
PendingMerge → attempt_programmatic_merge()
             → GitService::try_rebase_and_merge()
             → MergeAttemptResult::Success { commit_sha }  ← WE detect it
             → Direct DB update: task.status = Merged
```

### Agent Merge (Conflict Path)
```
Merging → Agent spawned
        → Agent resolves conflicts
        → Agent runs: git add . && git rebase --continue
        → Agent runs: git rev-parse HEAD
        → Agent CALLS: complete_merge(task_id, commit_sha)  ← AGENT tells us
        → HTTP handler updates status to Merged
```

## You Are Correct

After the agent runs `git rebase --continue`, the merge is **already complete from git's perspective**:
- The commit SHA exists
- The branch is rebased onto base
- `.git/rebase-merge` directory is gone
- No conflict markers remain (if resolution was correct)

We **could** detect this the same way we detect programmatic merge success.

## Why We Currently Don't (Historical Reasons)

| Reason | Validity |
|--------|----------|
| Agent might not be "done" yet | Weak — rebase --continue means done |
| Agent needs to verify code compiles | Valid — but we could verify ourselves |
| Clear contract for agent | Design choice, not technical requirement |
| Agent can provide failure context | Only applies to `report_conflict` |

## Solution: Event-Driven Detection (No Polling)

We already have agent completion detection — the background tokio task awaits the CLI process. When the agent exits, we hook into that completion to check git state:

```
Agent exits → Background task completes → Check task status
                                           ├─ Already transitioned? → Done (agent called tool)
                                           └─ Still Merging? → Check git state → Auto-complete/fail
```

### Detection Points

| Signal | How to Detect |
|--------|---------------|
| Rebase complete | `.git/rebase-merge` dir gone |
| No conflicts remain | `grep -r "<<<<<<< HEAD" .` returns nothing |
| Merge commit exists | `git rev-parse HEAD` |

## Approach: Hybrid

1. **Success path:** When agent exits, check git state
   - If rebase complete + no conflicts → auto-transition to Merged
   - Runs once on agent exit, not polling

2. **Failure path:** Keep `report_conflict` for agent to explain why
   - Agent can explicitly signal "I can't resolve this"
   - Provides context for human intervention

3. **Make `complete_merge` idempotent** (backwards compatible)
   - Agent can still call it, but we don't require it
   - If already merged, return success

## Implementation

### Task 1: Add merge detection helpers to GitService (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(git): add merge state detection helpers`

**File:** `src-tauri/src/application/git_service.rs`

Add:
```rust
/// Check if a rebase is in progress
pub fn is_rebase_in_progress(worktree: &Path) -> bool {
    worktree.join(".git/rebase-merge").exists()
        || worktree.join(".git/rebase-apply").exists()
}

/// Check for conflict markers in tracked files
pub fn has_conflict_markers(worktree: &Path) -> Result<bool> {
    // grep -r "<<<<<<< " . (in worktree)
}

/// Get current HEAD commit SHA
pub fn get_head_sha(worktree: &Path) -> Result<String> {
    // git rev-parse HEAD
}
```

### Task 2: Hook into agent completion in background task handler

**Dependencies:** Task 1, Task 3
**Atomic Commit:** `feat(chat): add merge auto-completion on agent exit`

**File:** `src-tauri/src/application/chat_service/chat_service_send_background.rs`

After the agent process completes (success or failure), check if merge context needs auto-completion:

```rust
// In spawn_send_message_background(), after process_stream_background() returns:
match result {
    Ok((response_text, tool_calls, content_blocks, claude_session_id)) => {
        // Existing success handling...

        // NEW: Check if Merge agent completed without transitioning
        if context_type == ChatContextType::Merge {
            attempt_merge_auto_complete(&context_id, &app_handle).await;
        }
    }
    Err(e) => {
        // Existing error handling...

        // NEW: Even on agent error, check if merge succeeded in git
        if context_type == ChatContextType::Merge {
            attempt_merge_auto_complete(&context_id, &app_handle).await;
        }
    }
}
```

The auto-complete function:
```rust
async fn attempt_merge_auto_complete(task_id: &str, app_handle: &AppHandle) {
    // 1. Get task - if not in Merging state, agent already handled it
    let task = task_repo.get(task_id).await?;
    if task.internal_status != InternalStatus::Merging {
        return; // Agent called complete_merge or report_conflict
    }

    // 2. Check git state
    let worktree = Path::new(&task.worktree_path);

    if GitService::is_rebase_in_progress(worktree) {
        // Rebase incomplete - agent failed without reporting
        transition_to_merge_conflict(task_id, "Agent exited with incomplete rebase").await;
        return;
    }

    if GitService::has_conflict_markers(worktree)? {
        // Conflicts remain - agent failed without reporting
        transition_to_merge_conflict(task_id, "Agent exited with unresolved conflicts").await;
        return;
    }

    // 3. Merge succeeded! Auto-complete
    let commit_sha = GitService::get_head_sha(worktree)?;
    complete_merge_internal(task_id, &commit_sha, ...).await;
}
```

**No polling.** This runs exactly once, when the agent process exits.

### Task 3: Extract shared merge completion logic (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `refactor(state-machine): extract shared merge completion logic`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

Extract the merge completion code (currently in `attempt_programmatic_merge`) into a shared function:

```rust
async fn complete_merge_internal(
    task_id: &TaskId,
    commit_sha: &str,
    task_repo: &TaskRepo,
    transition_service: &TransitionService,
    event_emitter: &EventEmitter,
) -> Result<()> {
    // 1. Update task with merge commit SHA
    // 2. Transition to Merged
    // 3. Cleanup worktree + branch
    // 4. Emit task:merged event
}
```

Use this from:
- Programmatic merge success path
- Merge completion watcher
- `complete_merge` HTTP handler (for backwards compat)

### Task 4: Validate + make `complete_merge` handler idempotent

**Dependencies:** None
**Atomic Commit:** `feat(http): make complete_merge idempotent with SHA validation`

**File:** `src-tauri/src/http_server/handlers/git.rs`

```rust
// Validate full SHA (40 hex chars)
if req.commit_sha.len() != 40 || !req.commit_sha.chars().all(|c| c.is_ascii_hexdigit()) {
    return Err((
        StatusCode::BAD_REQUEST,
        "commit_sha must be a full 40-character SHA (use `git rev-parse HEAD`)".to_string()
    ));
}

// Idempotent: if already merged, return success
if task.internal_status == InternalStatus::Merged {
    return Ok(Json(json!({ "status": "already_merged" })));
}
```

### Task 5: Update merger agent docs

**Dependencies:** Task 2
**Atomic Commit:** `docs(plugin): update merger agent for auto-detected completion`

**File:** `ralphx-plugin/agents/merger.md`

Change from "MUST call complete_merge" to:
- Merge completion is auto-detected
- `complete_merge` is optional (for explicit signaling)
- `report_conflict` is still required for failure path

## Files Modified

| File | Change |
|------|--------|
| `src-tauri/src/application/git_service.rs` | Add detection helpers |
| `src-tauri/src/application/chat_service/chat_service_send_background.rs` | Hook into agent completion, call auto-complete |
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | Extract shared completion logic |
| `src-tauri/src/http_server/handlers/git.rs` | Make `complete_merge` idempotent |
| `ralphx-plugin/agents/merger.md` | Update docs (complete_merge optional) |

## Verification

1. **Happy path:** Agent resolves conflicts → exits without calling `complete_merge` → auto-detection completes merge
2. **Agent calls tool:** Agent calls `complete_merge` → auto-detection sees task already transitioned → no-op
3. **Agent fails silently:** Agent exits with conflicts remaining → auto-detection transitions to MergeConflict
4. **Agent calls report_conflict:** Auto-detection sees task already transitioned → no-op
5. **Agent crash:** Process killed → auto-detection runs on error path → checks git state

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Execution Order

```
Task 1 ──┐
         ├──→ Task 2 ──→ Task 5
Task 3 ──┘
Task 4 (independent)
```

- **Tasks 1, 3, 4** can be executed in parallel (no dependencies)
- **Task 2** requires Tasks 1 and 3 (uses GitService helpers + shared completion logic)
- **Task 5** requires Task 2 (documents the new behavior)
