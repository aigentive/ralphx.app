# Plan: Build MCP Bridge for Review Issue Tools

## Context

The review issue lifecycle is fully implemented in the backend (Tauri commands, domain entities, repo, DB migration) and frontend (API layer, 5 detail view components, IssueList component). But the **MCP bridge is completely missing** — no HTTP endpoints, no MCP tool definitions, no MCP handlers. This means:

- **Worker agent** during re-execution is told to use `get_task_issues` / `mark_issue_in_progress` / `mark_issue_addressed` (in its frontmatter/system prompt) but these tools silently fail — they don't exist in the MCP layer
- **Reviewer agent** can't check whether issues were actually addressed during re-reviews
- The UI shows issue progress (5 detail views) but the data never updates because agents can't write to it

Additionally, the reviewer is missing `get_step_progress` (exists in MCP, assigned to worker, not to reviewer).

## What Already Exists (reuse, don't rebuild)

| Component | File | Status |
|-----------|------|--------|
| Tauri commands | `src-tauri/src/commands/review_commands.rs:539-717` | ✅ Complete |
| Input/Response types | `src-tauri/src/commands/review_commands_types.rs` | ✅ Complete |
| `ReviewIssueRepository` trait | `src-tauri/src/infrastructure/sqlite/sqlite_review_issue_repo.rs:17-46` | ✅ Complete |
| `ReviewIssue` entity | `src-tauri/src/domain/entities/review_issue.rs` | ✅ Complete |
| `AppState.review_issue_repo` | `src-tauri/src/application/app_state.rs:72` | ✅ Wired |
| Frontend API | `src/api/review-issues.ts` | ✅ Complete |
| Frontend UI (5 views) | `src/components/tasks/detail-views/*.tsx` | ✅ Complete |
| `IssueList` component | `src/components/reviews/IssueList.tsx` | ✅ Complete |

## Changes

### 1. HTTP Layer — Handlers, Types, Routes, Module Registration (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(http_server): add review issue HTTP endpoints for MCP bridge`

**Compilation unit note:** Tasks 1-4 from original plan merged — `issues.rs` imports types from `types.rs`, `mod.rs` registers the module, and `mod.rs` (routes) references handler functions. All four changes must compile together.

**1a. New file `src-tauri/src/http_server/handlers/issues.rs`** — 4 HTTP handler functions following `handlers/steps.rs` pattern:

| Handler | Method | Route | Mirrors Tauri Command |
|---------|--------|-------|-----------------------|
| `get_task_issues_http` | GET | `/api/task_issues/:task_id` | `get_task_issues` (line 539) |
| `get_issue_progress_http` | GET | `/api/issue_progress/:task_id` | `get_issue_progress` (line 568) |
| `mark_issue_in_progress_http` | POST | `/api/mark_issue_in_progress` | `mark_issue_in_progress` (line 657) |
| `mark_issue_addressed_http` | POST | `/api/mark_issue_addressed` | `mark_issue_addressed` (line 690) |

**Pattern**: Each handler accesses `state.app_state.review_issue_repo` (same repo the Tauri commands use). Response types reuse `ReviewIssueResponse` and `IssueProgressResponse` from `review_commands_types.rs`.

**Query param**: `get_task_issues_http` accepts optional `?status=open` query param for filtering.

**1b. Add to `src-tauri/src/http_server/types.rs`** — 2 request types for POST endpoints:
- `MarkIssueInProgressRequest { issue_id: String }`
- `MarkIssueAddressedRequest { issue_id: String, resolution_notes: String, attempt_number: i32 }`

**Note**: Can reuse `ReviewIssueResponse` / `IssueProgressResponse` from `review_commands_types` for responses (they already derive `Serialize`).

**1c. Add 4 routes to `src-tauri/src/http_server/mod.rs`** after the review tools section (line 67):
```
// Review issue tools (worker + reviewer agents)
.route("/api/task_issues/:task_id", get(get_task_issues_http))
.route("/api/issue_progress/:task_id", get(get_issue_progress_http))
.route("/api/mark_issue_in_progress", post(mark_issue_in_progress_http))
.route("/api/mark_issue_addressed", post(mark_issue_addressed_http))
```

**1d. Register module in `src-tauri/src/http_server/handlers/mod.rs`:**
```rust
pub mod issues;
pub use issues::*;
```

### 2. MCP Layer — Tool Definitions, Registration, Dispatch (BLOCKING)
**Dependencies:** Task 1 (HTTP endpoints must exist for dispatch to call)
**Atomic Commit:** `feat(mcp): add review issue tool definitions and dispatch handlers`

**Compilation unit note:** Tasks 5-7 from original plan merged — `issue-tools.ts` is imported by `tools.ts`, and `index.ts` references tool names from `tools.ts`. All three TS files must be consistent.

**2a. New file `ralphx-plugin/ralphx-mcp-server/src/issue-tools.ts`** — 4 tools following `step-tools.ts` pattern:
- `get_task_issues` — task_id (required), status_filter (optional: "open"|"all")
- `get_issue_progress` — task_id (required)
- `mark_issue_in_progress` — issue_id (required)
- `mark_issue_addressed` — issue_id (required), resolution_notes (required), attempt_number (required)

**2b. Update `ralphx-plugin/ralphx-mcp-server/src/tools.ts`:**
- Import `ISSUE_TOOLS` from `issue-tools.ts`
- Add `...ISSUE_TOOLS` to `ALL_TOOLS` array
- Add to `TOOL_ALLOWLIST`:
  - `"ralphx-worker"`: `"get_task_issues"`, `"mark_issue_in_progress"`, `"mark_issue_addressed"`
  - `"ralphx-reviewer"`: `"get_task_issues"`, `"get_issue_progress"`

**2c. Update `ralphx-plugin/ralphx-mcp-server/src/index.ts`** — handler dispatch:
- `get_task_issues` → `callTauriGet(`task_issues/${task_id}?status=${status_filter}`)`
- `get_issue_progress` → `callTauriGet(`issue_progress/${task_id}`)`
- `mark_issue_in_progress` → `callTauri("mark_issue_in_progress", { issue_id })`
- `mark_issue_addressed` → `callTauri("mark_issue_addressed", { issue_id, resolution_notes, attempt_number })`

Add `get_task_issues`, `mark_issue_in_progress`, `mark_issue_addressed` to `taskScopedTools` array (they have `task_id`/`issue_id` params).

### 3. Agent Config Layer 1 (Rust) — `src-tauri/src/infrastructure/agents/claude/agent_config.rs`
**Dependencies:** None (additive — just adds strings to arrays)
**Atomic Commit:** `feat(agents): add review issue tools to worker and reviewer allowlists`

**Worker** (line ~127): Add to `allowed_mcp_tools`:
- `"get_task_issues"`
- `"mark_issue_in_progress"`
- `"mark_issue_addressed"`

**Reviewer** (line ~147): Add to `allowed_mcp_tools`:
- `"get_task_issues"`
- `"get_step_progress"` (already exists in MCP, just not assigned to reviewer)
- `"get_issue_progress"`

Update corresponding tests.

### 4. Agent Config Layer 3 + Reviewer Prompt — `ralphx-plugin/agents/reviewer.md`
**Dependencies:** None (additive — markdown only)
**Atomic Commit:** `feat(agents): add issue tools to reviewer frontmatter and re-review workflow`

**Compilation unit note:** Tasks 9-10 from original plan merged — both modify the same file (`reviewer.md`), frontmatter and body section respectively.

**Worker** (`ralphx-plugin/agents/worker.md`): Tools already listed ✅ (no change needed)

**Reviewer** (`ralphx-plugin/agents/reviewer.md`): Add to `tools:` list:
- `mcp__ralphx__get_task_issues`
- `mcp__ralphx__get_step_progress`
- `mcp__ralphx__get_issue_progress`

**Reviewer system prompt update** — Add a "Re-Review Workflow" section telling it to:
1. Call `get_task_issues(task_id)` to see structured issues from prior review
2. Check which issues have status `addressed` vs still `open`
3. Use `get_step_progress(task_id)` to see what the worker did at each step
4. Cross-reference addressed issues against actual code changes

### 5. Remove "Files Under Review" placeholder — `src/components/tasks/detail-views/ReviewingTaskDetail.tsx`
**Dependencies:** None (independent cleanup)
**Atomic Commit:** `fix(ui): remove unimplemented Files Under Review placeholder`

Delete the placeholder section at lines 255-261:
```tsx
{/* Files Under Review - placeholder */}
<section data-testid="files-under-review-empty">
  <SectionTitle muted>Files Under Review</SectionTitle>
  <p className="text-[12px] text-white/35 italic">
    File list will appear once review gathers context
  </p>
</section>
```

This is an unimplemented feature stub with no backend support. Remove it to avoid misleading the user.

## Files Modified (Summary)

| Task | File | Action |
|------|------|--------|
| 1 | `src-tauri/src/http_server/handlers/issues.rs` | **CREATE** — 4 HTTP handlers |
| 1 | `src-tauri/src/http_server/handlers/mod.rs` | Add `pub mod issues; pub use issues::*;` |
| 1 | `src-tauri/src/http_server/mod.rs` | Add 4 routes |
| 1 | `src-tauri/src/http_server/types.rs` | Add 2 request types |
| 2 | `ralphx-plugin/ralphx-mcp-server/src/issue-tools.ts` | **CREATE** — 4 MCP tool definitions |
| 2 | `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | Import issue tools, update `ALL_TOOLS` + `TOOL_ALLOWLIST` |
| 2 | `ralphx-plugin/ralphx-mcp-server/src/index.ts` | Add 4 handler cases + taskScopedTools |
| 3 | `src-tauri/src/infrastructure/agents/claude/agent_config.rs` | Update worker + reviewer `allowed_mcp_tools` + tests |
| 4 | `ralphx-plugin/agents/reviewer.md` | Add tools to frontmatter + re-review workflow section |
| 5 | `src/components/tasks/detail-views/ReviewingTaskDetail.tsx` | Remove "Files Under Review" placeholder (lines 255-261) |

## Task Dependency Graph

```
Task 1 (HTTP Layer)  ──→  Task 2 (MCP Layer)
Task 3 (Agent Config Rust)  [independent]
Task 4 (Reviewer frontmatter + prompt)  [independent]
Task 5 (Remove UI placeholder)  [independent]
```

Tasks 3, 4, 5 are independent of each other and of the 1→2 chain.
Tasks 1→2 must be sequential (MCP dispatch calls HTTP endpoints).
Optimal: Run Task 1, then Task 2; Tasks 3, 4, 5 can be done in any order.

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings` — Rust compiles
2. `cargo test` — agent config tests pass
3. `npm run typecheck` in `ralphx-mcp-server/` — MCP server types check
4. Manual test: `curl http://localhost:3847/api/task_issues/{task_id}` returns issues
5. Manual test: `curl -X POST http://localhost:3847/api/mark_issue_in_progress -d '{"issue_id":"..."}' ` updates status
6. End-to-end: Run a task through review → needs_changes → re-execution and verify:
   - Worker agent can fetch issues via MCP
   - Worker agent can mark issues in_progress/addressed via MCP
   - UI reflects issue status changes in real-time
   - Re-review: Reviewer can see issue resolution status

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
