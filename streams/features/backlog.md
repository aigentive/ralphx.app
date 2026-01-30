# Features Backlog (P0 - Critical Gaps)

> P0 items are phase gaps — bugs where code exists but isn't wired up.
> These BLOCK all PRD work. Fix P0 first, no exceptions.

<!-- All P0 items from Phase 20-22 have been resolved. New P0s from verify stream go here. -->

## From Phase 24 Verification (2026-01-28)

- [x] [Infrastructure] Regex pattern error in fswatch cleanup: pkill pattern uses invalid regex - ralph-tmux.sh:185
- [x] [Infrastructure] Unquoted variable expansion in fswatch arguments - scripts/stream-watch-features.sh:35
- [x] [Infrastructure] Race condition: initial cycle and fswatch startup overlap - scripts/stream-watch-features.sh:24
- [x] [Infrastructure] Orphaned subshells: fswatch pipes not properly managed on stop - ralph-tmux.sh:167
- [x] [Infrastructure] Stream wrappers missing signal trap handlers for clean shutdown - scripts/stream-watch-features.sh:1

## From Phase 24 Re-verification (2026-01-28)

- [x] [Infrastructure] Missing watch file: hygiene stream does not watch streams/features/backlog.md - scripts/stream-watch-hygiene.sh:10

## From Phase 25 Verification (2026-01-29)

- [x] [Backend] Missing migration: v26 for seed_task_id column never added - src-tauri/src/infrastructure/sqlite/migrations/mod.rs

## From Phase 26 Verification (2026-01-29)

- [x] [Backend] Missing production implementation: TaskScheduler trait has no concrete implementation in application layer - src-tauri/src/application/
- [x] [Backend] Service not injected: TaskScheduler missing from AppState builder - src-tauri/src/application/app_state.rs

## From Phase 28 Verification (2026-01-29)

- [x] [Frontend] Dead hook: useIdeationEvents defined but never called in any component - src/hooks/useIdeationEvents.ts:33

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**P0 items:** 10 completed (moved to archive)
**Last maintenance:** 2026-01-30 (archived 1 item)
