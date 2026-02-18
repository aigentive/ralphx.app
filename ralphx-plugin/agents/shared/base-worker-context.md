# Base Worker Context

> Shared base for worker.md, coder.md, reviewer.md. Include via `<!-- @shared/base-worker-context.md -->`.

## Project Context

RalphX: React/TS frontend + Rust/Tauri backend + SQLite. MCP: `Claude Agent → ralphx-mcp-server (TS) → HTTP :3847 → Tauri`.

## Universal Constraints

- Modify only files directly related to the task
- TDD mandatory: tests first, then implementation
- Tauri invoke uses camelCase (`contextId`, NOT `context_id`)
- No fragile string comparisons — use enum variants or error codes
- USE TransitionHandler for status changes — NEVER direct DB update
- Lint before commit: `src-tauri/` → `cargo clippy`, `src/` → `npm run lint`

## Environment Setup (call before writing code)

```
get_project_analysis(project_id: RALPHX_PROJECT_ID, task_id: ...)
```
→ Run `worktree_setup` commands → Run `validate` commands to confirm clean baseline.
If `status: "analyzing"` — wait `retry_after_secs` and retry.

## Step Tracking Protocol

| Action | Call |
|--------|------|
| Before each step | `start_step(step_id)` |
| After success | `complete_step(step_id, note?)` |
| Not needed | `skip_step(step_id, reason)` |
| Failed | `fail_step(step_id, error)` |
| Missing steps | `add_step(task_id, title)` |

## Pre-Completion Validation (MANDATORY)

1. Run ALL `validate` commands for every path you modified
2. Validation fails on YOUR changes → fix before completing
3. Validation fails on pre-existing code → note but do not block

## Re-Execution (when `RALPHX_TASK_STATE=re_executing`)

1. `get_review_notes(task_id)` — read all prior feedback
2. `get_task_issues(task_id, status_filter: "open")` — get structured issues
3. Fix critical issues first, then major → minor → suggestions
4. `mark_issue_in_progress(issue_id)` → fix → `mark_issue_addressed(issue_id, notes, attempt_number)`

## Quality Checklist

- [ ] Tests pass (`npm run test:run` or `timeout 10m cargo test --lib`)
- [ ] TypeScript strict (`npm run typecheck`)
- [ ] Linting passes
- [ ] All open issues addressed
- [ ] Changes committed
