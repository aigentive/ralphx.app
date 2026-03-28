> **Maintainer note:** Keep this file compact. Prefer one-line rules, links to source docs, and explicit non-negotiables over prose.

# AGENTS.md

## Project
RalphX — native Mac GUI for autonomous AI development with Rust/Tauri backend and React frontend.

Primary project docs:
- `CLAUDE.md`
- `src-tauri/CLAUDE.md`
- `.claude/rules/*.md`
- `.claude/rules/agent-mcp-tools.md` for multi-layer agent MCP/tool alignment across prompt frontmatter, `ralphx.yaml`, and MCP allowlists
- `.claude/rules/rust-test-execution.md` for selective Rust test commands, the standard Rust test stack, shared SQLite fixtures/builders, and the no-broad-`fmt` rule

## Codex Rules

| Rule | Detail |
|---|---|
| Read project instructions first | Before substantial work, read the relevant repo docs (`CLAUDE.md` at root, plus subtree docs like `src-tauri/CLAUDE.md`). |
| Preserve user work | Never revert unrelated changes you did not make. If the tree is dirty, isolate your edits and verify diffs before commit. |
| Minimal diffs | Avoid formatter churn and accidental refactors. Keep changes scoped to the task. |
| Agent tool alignment | MCP/tool changes are multi-layer: keep prompt frontmatter, `ralphx.yaml`, and MCP allowlists in sync. Source: `.claude/rules/agent-mcp-tools.md`. |
| Handler module split | Oversized Rust HTTP handlers belong in directory-backed modules (`foo/mod.rs` + endpoint-family files), not single multi-thousand-line `foo.rs` files. |
| Mechanical extraction only (NON-NEGOTIABLE) | Large code moves/splits must use real move/extract operations (`mv`, `sed`, `awk`, scripted extraction). Do not hand-copy or retype large existing bodies into new files. Source: `.claude/rules/code-quality-standards.md`. |
| Apply-patch is fix-up only (NON-NEGOTIABLE) | After a mechanical move, `apply_patch` is only for the small follow-up layer: imports, visibility, re-exports, module wiring, tests. It is not a substitute for moving existing code. |
| Mechanical split recovery | If a large extraction drifts into patch-copying or visibility churn, restore the module to `HEAD`, clean any parked WIP out of the repo tree, and redo the split mechanically; do not keep iterating on a half-split tree. |
| Rustfmt scope safety | Never run `rustfmt` on Rust module roots like `mod.rs` unless the user explicitly wants recursive formatting; it can rewrite child modules and create unrelated churn. |
| Cargo during refactors | Never overlap Cargo jobs while validating a large extraction; one targeted run at a time or you recreate build-lock noise and lose signal. |
| Rust test runner split | Use `cargo test` for selective filters and doctests; use `cargo nextest run` for broad Rust lib runs. CI runs both. Details: `.claude/rules/rust-test-execution.md`. |
| Rust toolchain source of truth | `rust-toolchain.toml` is authoritative; use a `rustup`-managed toolchain so the repo pin actually applies. |
| Rust std API stability (NON-NEGOTIABLE) | Do not use unstable std APIs in production Rust (example: `is_multiple_of`). Use stable equivalents such as `%` with a zero guard where needed. Source of truth: `.claude/rules/rust-stable-apis.md`. |
| Worktree safety (NON-NEGOTIABLE) | In Worktree mode, task/review flows must never silently fall back to the user’s main repo checkout. Prefer hard failure and repair/self-heal paths. |
| Verify before commit | Review `git diff` for every file you touched against `HEAD` and confirm only intended hunks remain before committing. |

## Backend

When working in `src-tauri/`, also follow:
- `src-tauri/CLAUDE.md`
- `.claude/rules/rust-stable-apis.md`
- `.claude/rules/rust-test-execution.md`
- `.claude/rules/task-git-branching.md`
- `.claude/rules/code-quality-standards.md`
- `.claude/rules/agent-mcp-tools.md`

## Optimization Tracking

| Priority | Area | Next Step |
|---|---|---|
| P0 | Global orchestration semantics | Landed: persisted halt mode, explicit stopped UI state, stopped-aware control bar copy, `resume_execution` rejection after stop, queued-message pressure surfaced in execution status/control-bar telemetry, and inline execution-bar warning treatment for held agent messages; next observe whether thresholding or stronger escalation is needed |
| P0 | Concurrency admission control | Landed: project-aware slot admission now applies across chat spawns, scheduler selection, low-level spawner dispatch, startup recovery, merge-retry scheduling, manual task resume, paused ideation queue resume fairness across projects, paused task/review/merge queue fairness across projects, reconciliation auto-resume/retry paths, mixed ideation-vs-review/merge contention tests, and combined mixed-load resume regressions across both queue families; next keep broadening contention coverage where real queue pressure revealed gaps |
| P0 | Pipeline allocation | Ideation spawn/resume now respect global cap, project ideation cap, project total cap, borrowing only when execution is not waiting, cross-project paused-queue fairness during resume, and execution-side queued work now gets first claim on shared capacity before ideation relaunch; task/review/merge admission, manual task resume, reconciliation retries, and spawner dispatch all respect project totals across runtime + recovery paths; next validate under heavier mixed load and decide whether queued-message pressure needs stronger inline UX than the current telemetry |
| P1 | Queue + recovery alignment | Landed: pause queues all slot-consuming sends, resume relaunches paused ideation + active task/review/merge queued work in execution-first order, stop clears queued slot-consuming work, and execution status now reports queued agent-message pressure separately from Ready-task queue depth; next validate whether more real-time queue event emission is worth the extra plumbing |
| P1 | Settings surface | YAML-seeded defaults plus UI/API controls now cover global/project ideation caps + borrow toggle; next keep validating those controls against per-project admission and borrowing behavior |
| P2 | Transition handler support layer | After concurrency semantics stabilize, resume splitting `merge_validation`, `merge_coordination`, remaining `side_effects` hot spots, and the oversized review/freshness corrective-routing path across `src-tauri/src/application/task_transition_service.rs` plus `src-tauri/src/domain/state_machine/transition_handler/on_enter_states/review.rs` |
| P2 | Scheduler + watchdog orchestration | `src-tauri/src/application/task_scheduler_service.rs` now carries Ready scheduling, deferred-merge retries, main-merge retries, contention retry logic, and parked-review freshness wakeups; next extract retry families/watchdog querying into smaller support modules once the current semantics settle |
| P2 | Execution command orchestration | Split `src-tauri/src/commands/execution_commands.rs` after the transition-handler support layer pass; priority slices are pause/stop/resume orchestration, queue relaunch helpers, status/event payload shaping, and the oversized embedded test block |
| P2 | Capability test split | Continue moving OS-capability checks out of default broad suites into explicit ignored tests or dedicated capability binaries |
| P3 | Oversized HTTP handlers | After transition-handler stabilization, resume large backend handler refactors like `git.rs` and `teams.rs` |

## Current TDD Rollout

| Milestone | Tests First | Implementation Files |
|---|---|---|
| M1 Admission gate | Landed: global/project ideation admission, verification-child counts-as-ideation tests, project-aware chat-service admission for task/review/merge, scheduler-side project-capacity skipping, startup recovery project-capacity checks, and merge-retry scheduler wiring; next broader regression on queue pressure / recovery-heavy flows | `src-tauri/src/application/chat_service/mod.rs`, `src-tauri/src/application/task_scheduler_service.rs`, `src-tauri/src/application/startup_jobs.rs`, `src-tauri/src/commands/execution_commands.rs`, targeted tests in `src-tauri/tests/` |
| M2 Global pause/stop semantics | Landed: persisted `ExecutionHaltMode` in `app_state`, startup task recovery suppression, startup ideation recovery suppression, command-side halt-mode persistence, stopped-aware status payloads, and explicit stopped UI/control-bar semantics | `src-tauri/src/commands/execution_commands.rs`, `src-tauri/src/application/startup_jobs.rs`, app-state repos/migrations/tests, execution UI/hooks |
| M3 Queue + startup recovery | Landed for slot-consuming contexts: paused sends queue instead of spawning, ideation continuations stay pending under the halt barrier, resume relaunches paused ideation + active task/review/merge queued work, and stop clears queued slot-consuming work | `src-tauri/src/application/chat_service/mod.rs`, `src-tauri/src/application/chat_service/chat_service_queue.rs`, `src-tauri/src/application/chat_service/chat_service_send_background.rs`, `src-tauri/src/commands/execution_commands.rs`, `src-tauri/tests/chat_service_pause_flows.rs` |
| M4 DB/settings backend | Landed: repo/domain/command/migration coverage plus YAML-seeded pristine-row bootstrap for `global_ideation_max`, `project_ideation_max`, `allow_ideation_borrow_idle_execution`; next move to UI/API controls | execution settings repos/commands, migrations, `src-tauri/ralphx.yaml`, API contracts |
| M5 Settings UI | Landed: global/project ideation allocation controls now flow through frontend schemas, API transforms, mocks, `App.tsx`, and `SettingsView`; next validate them against per-project admission, borrowing, and paused-queue fairness under heavier mixed load | `src/components/settings/SettingsView.tsx`, `src/api/execution.ts`, related schemas/transforms/tests |

## Allocation Rules

| Rule | Detail |
|---|---|
| Shared hard caps stay | Keep existing `global_max_concurrent` and per-project `max_concurrent_tasks` |
| New pipeline cap | Add `global_ideation_max` + `project_ideation_max`; verification child sessions count as ideation |
| Derived reserve | Execution reserve is derived (`max - ideation_max`), not separately edited |
| Borrowing | Ideation may borrow only idle execution capacity and only when no runnable execution work is waiting |
| Global pause | Stops active work and blocks new starts everywhere, but preserves resumability |
| Global stop | Stops active work and blocks new starts everywhere, and stopped work must not auto-resume on startup or via execution-bar resume |
