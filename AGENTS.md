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
| P1 | Autonomous scope drift prevention | Core path landed: proposal scope hints, verifier pressure, review drift detection, revise-first routing, follow-up provenance/dedupe, merge backstop, cross-project lineage; next watch real runs and extract the pure review/scope/follow-up logic out of the root crate without weakening adaptive scope expansion |
| P1 | Build / compile-coupling reduction | Landed slices: shared review scope-drift matching + blocker-fingerprint logic, `complete_review` tool schema/validation, review follow-up prompt / issue-normalization helpers, review decision/policy validation, review outcome/result shaping, review follow-up / scope-metadata shaping, review issue mapping, review history-selection helpers, merge scope-backstop evaluation, child-session matching/construction helpers, execution-status counting/running-status matching, running-process/ideation view shaping, execution status/command response shaping, and task-context artifact preview/hint shaping now live in `ralphx-domain`; verification-child creation now reuses that shared child-session path too; next keep moving high-churn pure backend decision code out of the heavy `src-tauri` crate so every validation run rebuilds less |
| P2 | Transition handler support layer | After concurrency semantics stabilize, resume splitting `merge_validation`, `merge_coordination`, remaining `side_effects` hot spots, the oversized review/freshness corrective-routing path across `src-tauri/src/application/task_transition_service.rs` plus `src-tauri/src/domain/state_machine/transition_handler/on_enter_states/review.rs`, and the growing merge gate stack in `src-tauri/src/domain/state_machine/transition_handler/side_effects/merge_attempt.rs` |
| P2 | Scheduler + watchdog orchestration | `src-tauri/src/application/task_scheduler_service.rs` now carries Ready scheduling, deferred-merge retries, main-merge retries, contention retry logic, and parked-review freshness wakeups; next extract retry families/watchdog querying into smaller support modules once the current semantics settle |
| P2 | Execution command orchestration | Split `src-tauri/src/commands/execution_commands.rs` after the transition-handler support layer pass; priority slices are pause/stop/resume orchestration, queue relaunch helpers, status/event payload shaping, and the oversized embedded test block |
| P2 | Capability test split | Continue moving OS-capability checks out of default broad suites into explicit ignored tests or dedicated capability binaries |
| P3 | Oversized HTTP handlers | After transition-handler stabilization, resume large backend handler refactors like `git.rs` and `teams.rs` |

## Active Reliability Tracker

| Priority | Stream | Next Step |
|---|---|---|
| P0 | Verifier critic resumption protocol | Landed: `plan-verifier` now treats `Task(...)` results with `agentId` as resumable/in-progress and requires full-context rescue prompts; next add deeper regressions and, if needed, runtime-side recovery so critics do not false-escalate while artifacts are still pending |
| P0 | Model-agnostic MCP/tool UX | Landed: high-friction MCP tools now document parent-vs-child session rules plus concrete payload examples; next extend repair-oriented backend/tool-side hints and consider narrower helper/composite tools for weak-model workflows |
| P0 | Startup external-session archival safety | Landed first slice: cold boot now respects the external-session TTL instead of archiving every `created`/`error` session on restart; next add recovery-aware exclusions for verified-without-proposals sessions and external ideation sessions claimed for startup recovery |

## Active Migration Tracker

| Stream | Goal | Status | Notes |
|---|---|---|---|
| Frontend relocation | Move the Vite/React app from repo root into `frontend/` while keeping `src-tauri/` at root | Completed | `package.json`, configs, `public/`, `src/`, and `tests/` now live under `frontend/` |
| Tauri wiring | Repoint Tauri dev/build hooks at `frontend/` and make frontend-local `npm run tauri ...` work | Completed | `frontend/package.json` shells back to repo root for the Tauri CLI; `src-tauri/tauri.conf.json` now points at `../frontend` |
| Repo command surface | Update docs/scripts/CI/release flows from root-frontend assumptions to `frontend/` commands | Completed | README, DEVELOPMENT, getting-started, build/release/CI paths rewired to `frontend/` |
| Tooling path refs | Update Claude/Cursor/rule-manager/path-scoped rule references from `src/**` and `tests/**` to `frontend/src/**` and `frontend/tests/**` where repo-local paths matter | Completed | `.claude/settings.json`, visual testing/api rules, and rule-manager scripts adjusted |
| Root cleanup | Remove stale root artifacts and reduce visible clutter | Completed | `rollback_backup.json` and empty `.config/` removed; local generated dirs now live under `.artifacts/`; `.cursor/` intentionally kept at root |
| Artifact strategy | Move generated local outputs under `.artifacts/` while keeping Playwright visual baselines tracked in-repo | Completed | `logs/`, `reports/`, `screenshots/`, and `backups/` moved under `.artifacts/`; frontend visual baselines stay in `frontend/tests/visual/snapshots/` |
| Asset publishing pipeline | Split ignored source captures from tracked public assets, move asset tooling under `assets/scripts/`, and add reusable compression/publish commands | Completed | `assets/raw/` is now gitignored source, `assets/public/` is the tracked publish set, `assets/scripts/` owns framing/diagram/compression, and `.claude/rules/assets.md` holds the workflow |
| Plugin namespace cleanup | Move the shared plugin under `plugins/shared` now; keep the heavier `ralphx-plugin` move as a later dedicated refactor | Completed | `ralphx-shared-plugin/` now lives at `plugins/shared/`; plugin name/namespace stays `ralphx-shared-plugin` for marketplace and slash-command stability |
| App plugin migration plan | Move the app plugin from `ralphx-plugin/` to `plugins/app/` without breaking runtime plugin discovery, packaging, or docs | Completed | Runtime fallback landed first, then the tree/config/docs/release path move landed with targeted validation |
| Validation | Re-run targeted frontend and Tauri-facing checks after rewiring completes | Completed | `npm --prefix frontend run typecheck`, `npm --prefix frontend run lint`, and `npm --prefix frontend run tauri build -- --help` succeeded |

## App Plugin Migration Tracker

| Stream | Goal | Status | Notes |
|---|---|---|---|
| Target naming | Finalize `ralphx-plugin -> plugins/app` as the repo path while keeping agent/plugin semantics clear | Completed | `plugins/app` won over `plugins/core`; the plugin contains agents, hooks, internal MCP, and external MCP, so `app` matches the actual role better |
| Runtime path resolution | Update backend/plugin-dir discovery from `./ralphx-plugin` to `./plugins/app` without breaking worktree/dev/prod fallback logic | Completed | Claude runtime now prefers `plugins/app`, keeps `ralphx-plugin` as a fallback, routes teammate spawns through the shared resolver, and updates default plugin-dir surfaces before the tree move |
| Config surface | Rewrite plugin paths in `ralphx.yaml` and path-scoped Claude rules/docs | Completed | Mechanical path rewrites landed; plugin ids and agent names stayed unchanged |
| Packaging + release | Update release/build scripts and production app-data plugin install path to the new location | Completed | Release provisioning now copies from `plugins/app` into `~/Library/Application Support/com.ralphx.app/plugins/app` |
| Docs + rules | Rewrite architectural/docs references after runtime/config changes are stable | Completed | Docs/rules now point at `plugins/app`; only explicit legacy fallback explanations still mention `ralphx-plugin` |
| Validation | Run targeted runtime/config checks after the move and only then commit | Completed | `cargo test --manifest-path src-tauri/Cargo.toml test_resolve_plugin_dir_ --lib`, `cargo test --manifest-path src-tauri/Cargo.toml test_plugin_repo_root_supports_nested_plugins_app_layout --lib`, `cargo test --manifest-path src-tauri/Cargo.toml test_teammate_spawn_config_new_defaults --lib`, `cargo test --manifest-path src-tauri/Cargo.toml -p ralphx-domain test_agent_config_default`, `cargo test --manifest-path src-tauri/Cargo.toml test_all_system_prompt_files_exist --lib`, and `bash -n scripts/build-release.sh scripts/count-loc.sh plugins/app/skills/rule-manager/scripts/rule-suggest-paths.sh` passed |

## Cross-Session Tracker Notes

| Tracker | Current Guardrails |
|---|---|
| Autonomous scope drift prevention | Accepted ideation sessions stay read-only; unrelated blocking failures should prefer spawning follow-up ideation sessions with first-class provenance; review should send back to revise before escalating when drift looks fixable; any future scope guard must allow necessary plan correction and adjacent files, not only exact initial file lists |
| Build / compile-coupling reduction | Favor extracting pure high-churn decision logic out of the root `src-tauri` crate over cosmetic handler/file-size cleanup; prefer mechanical moves first and small fix-up edits second; the goal is faster incremental validation, not just smaller files |

## Current TDD Rollout

| Milestone | Tests First | Implementation Files |
|---|---|---|
| M1 Admission gate | Landed: global/project ideation admission, verification-child counts-as-ideation tests, project-aware chat-service admission for task/review/merge, scheduler-side project-capacity skipping, startup recovery project-capacity checks, and merge-retry scheduler wiring; next broader regression on queue pressure / recovery-heavy flows | `src-tauri/src/application/chat_service/mod.rs`, `src-tauri/src/application/task_scheduler_service.rs`, `src-tauri/src/application/startup_jobs.rs`, `src-tauri/src/commands/execution_commands.rs`, targeted tests in `src-tauri/tests/` |
| M2 Global pause/stop semantics | Landed: persisted `ExecutionHaltMode` in `app_state`, startup task recovery suppression, startup ideation recovery suppression, command-side halt-mode persistence, stopped-aware status payloads, and explicit stopped UI/control-bar semantics | `src-tauri/src/commands/execution_commands.rs`, `src-tauri/src/application/startup_jobs.rs`, app-state repos/migrations/tests, execution UI/hooks |
| M3 Queue + startup recovery | Landed for slot-consuming contexts: paused sends queue instead of spawning, ideation continuations stay pending under the halt barrier, resume relaunches paused ideation + active task/review/merge queued work, and stop clears queued slot-consuming work | `src-tauri/src/application/chat_service/mod.rs`, `src-tauri/src/application/chat_service/chat_service_queue.rs`, `src-tauri/src/application/chat_service/chat_service_send_background.rs`, `src-tauri/src/commands/execution_commands.rs`, `src-tauri/tests/chat_service_pause_flows.rs` |
| M4 DB/settings backend | Landed: repo/domain/command/migration coverage plus YAML-seeded pristine-row bootstrap for `global_ideation_max`, `project_ideation_max`, `allow_ideation_borrow_idle_execution`; next move to UI/API controls | execution settings repos/commands, migrations, `ralphx.yaml`, API contracts |
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
