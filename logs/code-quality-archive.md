# Code Quality Archive

> Completed items moved from `logs/code-quality.md` when section count exceeds 10.

---

## Backend (src-tauri/) - P3 Low Impact

### Archived 2026-01-28
- [x] Replace println! debug statements with tracing::debug! in task_transition_service.rs - src-tauri/src/application/task_transition_service.rs
- [x] Remove redundant derive trait tests (Debug, Clone) in health.rs - src-tauri/src/commands/health.rs:37-50

### Archived 2026-01-28 (Deferred Validation)
- ~~Extract task_qa_repo (repetitive CRUD patterns)~~ - STALE: file is 336 LOC, under 500 limit
- ~~Add contextual error messages in artifact type parsing failures~~ - STALE: all locations already have contextual messages
- ~~Extract duplicate parse error handling pattern in workflow/ideation commands~~ - STALE: not a duplicate pattern, each is specific to its context
