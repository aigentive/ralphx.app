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
| P0 | Rust regression stabilization | Keep broad Rust runs green before taking on more structural refactors; fix stale or capability-misclassified failures first |
| P1 | Transition handler support layer | Split `merge_validation`, `merge_coordination`, and remaining `side_effects` hot spots while transition-handler context is still fresh |
| P1 | Transition handler follow-up regression | Re-run broad Rust regression after each support-layer split before moving to a different subsystem |
| P2 | Capability test split | Continue moving OS-capability checks out of default broad suites into explicit ignored tests or dedicated capability binaries |
| P2 | Oversized HTTP handlers | After transition-handler stabilization, resume large backend handler refactors like `git.rs` and `teams.rs` |
