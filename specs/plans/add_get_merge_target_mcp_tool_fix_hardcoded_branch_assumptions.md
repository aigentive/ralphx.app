# Plan: Add `get_merge_target` MCP Tool & Fix Hardcoded Branch Assumptions

## Context

Tasks belonging to plans with feature branches should merge into the **plan branch**, not `main`. The backend function `resolve_merge_branches()` correctly computes the target, but this logic is only used during the **programmatic merge attempt**. When that fails (conflicts), the merger agent takes over — but it has no way to discover the correct target branch. Additionally, the `complete_merge` handler and auto-detection both hardcode `base_branch` verification, rejecting merges to plan branches.

**Bug evidence:** Task `7307dced` (plan `fb207fe2`) merged to `main` instead of `ralphx/ralphx/plan-fb207fe2`. The task is stuck in `merge_incomplete` because the system doesn't recognize the merge.

## Changes

### 1. Extract `resolve_merge_branches` to shared location (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(state-machine): make resolve_merge_branches pub for cross-module use`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs:244-285`

Currently a private `async fn`. Make it `pub` and re-export from the module so HTTP handlers can call it.

- Make `resolve_merge_branches` → `pub async fn`
- Re-export from `src-tauri/src/domain/state_machine/transition_handler/mod.rs` (already re-exports `complete_merge_internal` at line 15)
- Also re-export from `src-tauri/src/domain/state_machine/mod.rs` (add to `pub use transition_handler::` line 30)
- No logic change — just visibility

### 2. Add backend endpoint + route (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(git): add get_merge_target HTTP endpoint`

**File:** `src-tauri/src/http_server/handlers/git.rs`

New handler following existing patterns (e.g., `get_task_commits`):

```rust
#[derive(Serialize)]
pub struct MergeTargetResponse {
    pub source_branch: String,
    pub target_branch: String,
}

pub async fn get_merge_target(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<MergeTargetResponse>, (StatusCode, String)> {
    // 1. Get task + project
    // 2. Call resolve_merge_branches(task, project, plan_branch_repo)
    // 3. Return { source_branch, target_branch }
}
```

**Note:** Import `resolve_merge_branches` from `crate::domain::state_machine::transition_handler::resolve_merge_branches` (or `crate::domain::state_machine::resolve_merge_branches` after re-export). Also needs `use crate::domain::repositories::PlanBranchRepository;` if not already imported (it's not — current git.rs imports are at lines 6-18).

**File:** `src-tauri/src/http_server/mod.rs` — add route after line 91:
```rust
.route("/api/git/tasks/:id/merge-target", get(get_merge_target))
```

### 3. Add MCP tool definition + handler + allowlist + description update
**Dependencies:** Task 2
**Atomic Commit:** `feat(mcp): add get_merge_target tool and update merge tool descriptions`

**Compilation unit note:** Original plan split this across Tasks 3, 4, 5, and 9. Merged because all modify the same two TS files (`tools.ts` and `index.ts`) and the tool must be defined, handled, allowlisted, and described consistently.

**File:** `ralphx-plugin/ralphx-mcp-server/src/tools.ts`

Add tool in MERGE TOOLS section (after `report_incomplete` at line 465):

```typescript
{
  name: "get_merge_target",
  description: "Get the resolved merge target branches for a task. " +
    "Returns source_branch (task's branch) and target_branch (where to merge INTO). " +
    "IMPORTANT: Always call this BEFORE merging to know the correct target. " +
    "The target may be a plan feature branch instead of main.",
  inputSchema: {
    type: "object",
    properties: {
      task_id: { type: "string", description: "The task ID" },
    },
    required: ["task_id"],
  },
}
```

Add `"get_merge_target"` to `ralphx-merger` allowlist (line 657-664).

Update `complete_merge` description (line 393-415): Remove hardcoded "main branch" references. Replace with:
```
"Signal successful merge completion. Call get_merge_target first to determine the correct target branch."
```

**File:** `ralphx-plugin/ralphx-mcp-server/src/index.ts`

Add GET handler in the tool dispatch (after `report_incomplete` handler around line 291):

```typescript
} else if (name === "get_merge_target") {
  const { task_id } = args as { task_id: string };
  result = await callTauriGet(`git/tasks/${task_id}/merge-target`);
}
```

Add `"get_merge_target"` to `taskScopedTools` array (line 94-110).

Also add `mcp__ralphx__report_incomplete` to merger.md `allowedTools` frontmatter (currently missing — `report_incomplete` is in the TOOL_ALLOWLIST but not in the agent's frontmatter `allowedTools` list).

**File:** `src-tauri/src/infrastructure/agents/claude/agent_config.rs:200-209`

Add `"get_merge_target"` and `"report_incomplete"` to the `ralphx-merger` `allowed_mcp_tools` array. This is the **Rust-side single source of truth** for what MCP tools are passed via `--allowedTools` when spawning the agent. Without this, the agent can never call the tool regardless of MCP server config.

```rust
// BEFORE (line 203-207):
allowed_mcp_tools: &[
    "complete_merge",
    "report_conflict",
    "get_task_context",
],

// AFTER:
allowed_mcp_tools: &[
    "complete_merge",
    "report_conflict",
    "report_incomplete",
    "get_merge_target",
    "get_task_context",
],
```

Also update the test at line 345 (`test_get_allowed_mcp_tools_merger_agent`) to expect the new tools.

### 4. Update merger agent prompt
**Dependencies:** Task 3
**Atomic Commit:** `docs(merger): update workflow to use get_merge_target`

**File:** `ralphx-plugin/agents/merger.md`

Update Step 1 to call `get_merge_target` first:

```markdown
### Step 1: Get Merge Target and Task Context

Start by getting the correct merge target:
\```
get_merge_target(task_id: "...")
\```

This returns:
- **source_branch**: The branch with task changes (usually task branch)
- **target_branch**: Where to merge INTO (may be a plan feature branch, NOT always main)

Then get full task context:
\```
get_task_context(task_id: "...")
\```
```

Update Step 5 to use `target_branch` instead of hardcoded "main":

```markdown
### Step 5: Complete the Merge

Merge INTO the **target_branch** from Step 1 (NOT always main):
1. `git checkout <target_branch>`
2. `git merge <source_branch>`  (or complete rebase)
3. Exit — system auto-detects
```

Update the `complete_merge` tool description to remove "ON the main branch" language.

### 5. Fix `complete_merge` handler verification
**Dependencies:** Task 1
**Atomic Commit:** `fix(git): use resolved merge target in complete_merge handler`

**File:** `src-tauri/src/http_server/handlers/git.rs:137-155`

Replace hardcoded `base_branch` with resolved target:

```rust
// BEFORE (line 138):
let base_branch = project.base_branch.as_deref().unwrap_or("main");
// ...
if !GitService::is_commit_on_branch(&repo_path, &req.commit_sha, base_branch)

// AFTER:
let (_, target_branch) = resolve_merge_branches(&task, &project, &state.app_state.plan_branch_repo).await;
// ...
if !GitService::is_commit_on_branch(&repo_path, &req.commit_sha, &target_branch)
```

**Note:** Also update error messages in the `Err` arm (lines 144-154) to reference `target_branch` instead of `base_branch`.

### 6. Fix `attempt_merge_auto_complete` verification
**Dependencies:** Task 1
**Atomic Commit:** `fix(merge): use resolved merge target in auto-complete verification`

**File:** `src-tauri/src/application/chat_service/chat_service_send_background.rs:826-844`

Replace hardcoded `base_branch` with resolved target:

```rust
// BEFORE (line 828-831):
let base_branch = project.base_branch.as_deref().unwrap_or("main");
match GitService::is_commit_on_branch(&main_repo_path, &task_branch_head, base_branch)

// AFTER:
let (_, target_branch) = resolve_merge_branches(&task, &project, plan_branch_repo).await;
match GitService::is_commit_on_branch(&main_repo_path, &task_branch_head, &target_branch)
```

**Note:** Requires adding `use crate::domain::state_machine::resolve_merge_branches;` to imports. The `plan_branch_repo` parameter is already available in the function signature (line 644). Also update the log messages (lines 838-840, 844) to reference `target_branch` instead of `base_branch`.

## Task Dependency Graph

```
Task 1 (pub + re-export) ──┬──→ Task 2 (backend endpoint) ──→ Task 3 (MCP tool+handler+allowlist) ──→ Task 4 (merger prompt)
                           ├──→ Task 5 (fix complete_merge handler)
                           └──→ Task 6 (fix auto-complete verification)
```

Tasks 2, 5, 6 can run in parallel after Task 1.
Tasks 3, 4 are sequential (MCP tool must exist before updating merger prompt).

## Compilation Unit Notes

**Merged tasks:** Original Tasks 3 (tool def), 4 (handler), 5 (allowlist), and 9 (description update) were merged into a single Task 3 because they all modify the same two TypeScript files (`tools.ts` and `index.ts`). A partial state (tool defined but not handled, or handled but not allowlisted) would cause runtime errors.

**Safe splits:** Tasks 5 and 6 (Rust handler fixes) modify different files (`git.rs` vs `chat_service_send_background.rs`) and each is a complete compilation unit — both import `resolve_merge_branches` independently.

## Files Modified

| File | Task | Change |
|------|------|--------|
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | 1 | Make `resolve_merge_branches` pub |
| `src-tauri/src/domain/state_machine/transition_handler/mod.rs` | 1 | Re-export `resolve_merge_branches` |
| `src-tauri/src/domain/state_machine/mod.rs` | 1 | Re-export `resolve_merge_branches` |
| `src-tauri/src/http_server/handlers/git.rs` | 2, 5 | Add `get_merge_target` handler (T2) + fix `complete_merge` verification (T5) |
| `src-tauri/src/http_server/mod.rs` | 2 | Add route |
| `src-tauri/src/application/chat_service/chat_service_send_background.rs` | 6 | Fix auto-complete verification |
| `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | 3 | Add tool def + update allowlist + fix description |
| `ralphx-plugin/ralphx-mcp-server/src/index.ts` | 3 | Add tool handler + task scope |
| `src-tauri/src/infrastructure/agents/claude/agent_config.rs` | 3 | Add `get_merge_target` + `report_incomplete` to merger MCP allowlist |
| `ralphx-plugin/agents/merger.md` | 4 | Update workflow to use `get_merge_target` |

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings` — backend compiles
2. `cargo test` — existing tests pass
3. `cd ralphx-plugin/ralphx-mcp-server && npm run build` — MCP server compiles
4. Manual test: create a plan with feature branch, execute a task, trigger merge conflict → verify merger agent calls `get_merge_target` and merges to plan branch
5. Check `curl http://127.0.0.1:3847/api/git/tasks/<task-id>/merge-target` returns correct branches

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
