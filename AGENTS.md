> **Maintainer note:** Keep this file compact. Prefer one-line rules, links to source docs, and explicit non-negotiables over prose.

# AGENTS.md

## Project
RalphX — native Mac GUI for autonomous AI development with Rust/Tauri backend and React frontend.

Primary project docs:
- `CLAUDE.md`
- `src-tauri/CLAUDE.md`
- `.claude/rules/*.md`
- `.claude/rules/rust-test-execution.md` for selective Rust test commands, shared SQLite test fixtures, and the no-broad-`fmt` rule

## Codex Rules

| Rule | Detail |
|---|---|
| Read project instructions first | Before substantial work, read the relevant repo docs (`CLAUDE.md` at root, plus subtree docs like `src-tauri/CLAUDE.md`). |
| Preserve user work | Never revert unrelated changes you did not make. If the tree is dirty, isolate your edits and verify diffs before commit. |
| Minimal diffs | Avoid formatter churn and accidental refactors. Keep changes scoped to the task. |
| Rust test filters | `cargo test` accepts one name filter only; for selective Rust test commands use `.claude/rules/rust-test-execution.md`. |
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
