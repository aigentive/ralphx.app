> **Maintainer note:** Keep this file compact. Prefer one-line rules, links to source docs, and explicit non-negotiables over prose.

# AGENTS.md

## Project
RalphX — native Mac GUI for autonomous AI development with Rust/Tauri backend and React frontend.

Primary project docs:
- `CLAUDE.md`
- `src-tauri/CLAUDE.md`
- `.claude/rules/*.md`
- `.claude/rules/rust-test-execution.md` for selective Rust test commands, the standard Rust test stack, shared SQLite fixtures/builders, and the no-broad-`fmt` rule

## Codex Rules

| Rule | Detail |
|---|---|
| Read project instructions first | Before substantial work, read the relevant repo docs (`CLAUDE.md` at root, plus subtree docs like `src-tauri/CLAUDE.md`). |
| Preserve user work | Never revert unrelated changes you did not make. If the tree is dirty, isolate your edits and verify diffs before commit. |
| Minimal diffs | Avoid formatter churn and accidental refactors. Keep changes scoped to the task. |
| Handler module split | Oversized Rust HTTP handlers belong in directory-backed modules (`foo/mod.rs` + endpoint-family files), not single multi-thousand-line `foo.rs` files. |
| Extraction-first refactors | For large Rust module splits, programmatically move existing code into child files first, then make the smallest follow-up patches for visibility/imports/tests; don't hand-rewrite big functions. |
| Rustfmt scope safety | Never run `rustfmt` on Rust module roots like `mod.rs` unless the user explicitly wants recursive formatting; it can rewrite child modules and create unrelated churn. |
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

## Optimization Tracking

| Priority | Area | Next Step |
|---|---|---|
| P0 | Global orchestration semantics | Execution status/UI + `resume_execution` now preserve the stopped barrier instead of treating it like pause; next finish the remaining stop-vs-pause UX rules and confirm whether any non-ideation queued restart paths still need special handling |
| P0 | Concurrency admission control | M1 landed: ideation/verification now honor a global ideation admission gate before spawn; next extend admission control across the remaining slot-consuming contexts |
| P0 | Pipeline allocation | Ideation spawn/resume now respect global cap, project ideation cap, project total cap, and borrowing only when execution is not waiting; next extend pipeline-aware admission beyond ideation and verify stop-vs-pause UX still matches the new allocation model |
| P1 | Queue + recovery alignment | Landed: pause queues all slot-consuming sends, resume relaunches paused ideation + active task-side queued work, and stop clears queued slot-consuming work; next document the final UX contract and verify whether review/merge need any extra explicit resume coverage beyond the shared task-side relaunch path |
| P1 | Settings surface | YAML-seeded defaults plus UI/API controls now cover global/project ideation caps + borrow toggle; next validate final stop-vs-pause UX wording and then return to per-project ideation enforcement/borrowing behavior |
| P2 | Transition handler support layer | After concurrency semantics stabilize, resume splitting `merge_validation`, `merge_coordination`, and remaining `side_effects` hot spots |
| P2 | Capability test split | Continue moving OS-capability checks out of default broad suites into explicit ignored tests or dedicated capability binaries |
| P3 | Oversized HTTP handlers | After transition-handler stabilization, resume large backend handler refactors like `git.rs` and `teams.rs` |

## Current TDD Rollout

| Milestone | Tests First | Implementation Files |
|---|---|---|
| M1 Admission gate | Done: global ideation admission gate in shared chat service, verification-child counts-as-ideation tests, execution-not-starved tests, borrow-policy tests; next extend to per-project allocation + non-ideation slot consumers | `src-tauri/src/application/chat_service/mod.rs`, `src-tauri/src/commands/execution_commands.rs`, targeted tests in `src-tauri/tests/` |
| M2 Global pause/stop semantics | Landed: persisted `ExecutionHaltMode` in `app_state`, startup task recovery suppression, startup ideation recovery suppression, command-side halt-mode persistence; next wire runtime queue/launch behavior and stop-vs-resume UX to the same barrier | `src-tauri/src/commands/execution_commands.rs`, `src-tauri/src/application/startup_jobs.rs`, app-state repos/migrations/tests |
| M3 Queue + startup recovery | Landed for slot-consuming contexts: paused sends queue instead of spawning, ideation continuations stay pending under the halt barrier, resume relaunches paused ideation, and stop clears queued slot-consuming work; next confirm whether any queued task/review/merge restart paths need extra resume handling beyond normal task restoration | `src-tauri/src/application/chat_service/mod.rs`, `src-tauri/src/application/chat_service/chat_service_queue.rs`, `src-tauri/src/application/chat_service/chat_service_send_background.rs`, `src-tauri/src/commands/execution_commands.rs`, `src-tauri/tests/chat_service_pause_flows.rs` |
| M4 DB/settings backend | Landed: repo/domain/command/migration coverage plus YAML-seeded pristine-row bootstrap for `global_ideation_max`, `project_ideation_max`, `allow_ideation_borrow_idle_execution`; next move to UI/API controls | execution settings repos/commands, migrations, `src-tauri/ralphx.yaml`, API contracts |
| M5 Settings UI | Landed: global/project ideation allocation controls now flow through frontend schemas, API transforms, mocks, `App.tsx`, and `SettingsView`; next add any stop-vs-pause UX around resume availability once the backend contract is final | `src/components/settings/SettingsView.tsx`, `src/api/execution.ts`, related schemas/transforms/tests |

## Allocation Rules

| Rule | Detail |
|---|---|
| Shared hard caps stay | Keep existing `global_max_concurrent` and per-project `max_concurrent_tasks` |
| New pipeline cap | Add `global_ideation_max` + `project_ideation_max`; verification child sessions count as ideation |
| Derived reserve | Execution reserve is derived (`max - ideation_max`), not separately edited |
| Borrowing | Ideation may borrow only idle execution capacity and only when no runnable execution work is waiting |
| Global pause | Stops active work and blocks new starts everywhere, but preserves resumability |
| Global stop | Stops active work and blocks new starts everywhere, and stopped work must not auto-resume on startup or via execution-bar resume |
