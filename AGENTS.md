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
| P0 | Global orchestration semantics | Restart barrier plus runtime ideation queue semantics landed; next finish explicit stop-vs-pause UX/resume behavior and extend the same halt semantics across the remaining slot-consuming contexts |
| P0 | Concurrency admission control | M1 landed: ideation/verification now honor a global ideation admission gate before spawn; next extend admission control across the remaining slot-consuming contexts |
| P0 | Pipeline allocation | Global ideation cap + borrow toggle landed in `ExecutionState`; next add persisted per-project ideation allocation and runnable-execution-aware borrowing |
| P1 | Queue + recovery alignment | Landed: paused ideation relaunches on resume and stop clears queued slot-consuming work; next align any remaining queued task/review/merge relaunch paths and document the final UX contract |
| P1 | Settings surface | Store live allocation settings in DB/UI; keep `ralphx.yaml` as defaults/advanced guardrails only |
| P2 | Transition handler support layer | After concurrency semantics stabilize, resume splitting `merge_validation`, `merge_coordination`, and remaining `side_effects` hot spots |
| P2 | Capability test split | Continue moving OS-capability checks out of default broad suites into explicit ignored tests or dedicated capability binaries |
| P3 | Oversized HTTP handlers | After transition-handler stabilization, resume large backend handler refactors like `git.rs` and `teams.rs` |

## Current TDD Rollout

| Milestone | Tests First | Implementation Files |
|---|---|---|
| M1 Admission gate | Done: global ideation admission gate in shared chat service, verification-child counts-as-ideation tests, execution-not-starved tests, borrow-policy tests; next extend to per-project allocation + non-ideation slot consumers | `src-tauri/src/application/chat_service/mod.rs`, `src-tauri/src/commands/execution_commands.rs`, targeted tests in `src-tauri/tests/` |
| M2 Global pause/stop semantics | Landed: persisted `ExecutionHaltMode` in `app_state`, startup task recovery suppression, startup ideation recovery suppression, command-side halt-mode persistence; next wire runtime queue/launch behavior and stop-vs-resume UX to the same barrier | `src-tauri/src/commands/execution_commands.rs`, `src-tauri/src/application/startup_jobs.rs`, app-state repos/migrations/tests |
| M3 Queue + startup recovery | Landed for ideation: paused ideation sends queue instead of spawning, queued continuations stay pending under the halt barrier, resume relaunches paused ideation, and stop clears queued slot-consuming work; next confirm whether any queued task/review/merge restart paths need the same treatment | `src-tauri/src/application/chat_service/mod.rs`, `src-tauri/src/application/chat_service/chat_service_queue.rs`, `src-tauri/src/application/chat_service/chat_service_send_background.rs`, `src-tauri/src/commands/execution_commands.rs`, ideation runtime/external handlers |
| M4 DB/settings backend | Add failing repo/command tests for `global_ideation_max`, `project_ideation_max`, `allow_ideation_borrow_idle_execution`, YAML-seeded defaults | execution settings repos/commands, migrations, `src-tauri/ralphx.yaml`, API contracts |
| M5 Settings UI | Add failing UI tests for global/project ideation allocation controls and stop-vs-pause UX around resume availability | `src/components/settings/SettingsView.tsx`, `src/api/execution.ts`, related schemas/transforms/tests |

## Allocation Rules

| Rule | Detail |
|---|---|
| Shared hard caps stay | Keep existing `global_max_concurrent` and per-project `max_concurrent_tasks` |
| New pipeline cap | Add `global_ideation_max` + `project_ideation_max`; verification child sessions count as ideation |
| Derived reserve | Execution reserve is derived (`max - ideation_max`), not separately edited |
| Borrowing | Ideation may borrow only idle execution capacity and only when no runnable execution work is waiting |
| Global pause | Stops active work and blocks new starts everywhere, but preserves resumability |
| Global stop | Stops active work and blocks new starts everywhere, and stopped work must not auto-resume on startup or via execution-bar resume |
