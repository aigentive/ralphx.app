# RalphX - Phase 74: Review Dialog Improvements

## Overview

Fix data discrepancies and visual issues in the ReviewDetailModal component. The current implementation has several issues: commits display in reverse chronological order (newest first) instead of chronological (oldest first), the Changes tab only shows files from Write/Edit tool calls instead of the full git diff, duplicate task titles appear in both header and sidebar, AI review summaries render as plain text instead of markdown, review history shows full notes instead of summary field, and the dialog is smaller than optimal.

**Reference Plan:**
- `specs/plans/review_dialog_improvements.md` - Detailed implementation plan with code snippets and file locations

## Goals

1. Fix commit order to display chronologically (oldest first)
2. Show all changed files in Changes tab using git diff instead of activity events
3. Remove duplicate task title from sidebar
4. Render AI review summary as markdown
5. Use summary field in review history for cleaner display
6. Increase dialog size to near full screen (95vw x 95vh)

## Dependencies

### Phase 71 (Per-Commit File Changes in Review Dialog) - Required

| Dependency | Why Needed |
|------------|------------|
| Review dialog infrastructure | Modal, tabs, and diff viewer already implemented |
| Commit history display | History tab and commit list already in place |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/review_dialog_improvements.md`
2. Understand the specific code changes for each task
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
4. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- All tasks have `"blockedBy": []` - they are independent and can be done in any order
- Execute tasks in ID order for consistency

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/review_dialog_improvements.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Fix commit order to display chronologically (oldest first)",
    "plan_section": "1. Commit Sorting - Chronological Order",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(diff): reverse commit order to chronological",
    "steps": [
      "Read specs/plans/review_dialog_improvements.md section '1. Commit Sorting - Chronological Order'",
      "Edit src/hooks/useGitDiff.ts line 112: add .reverse() after map",
      "Also update line 183 in the refresh callback to add .reverse()",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(diff): reverse commit order to chronological"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Use git diff for file changes instead of activity events",
    "plan_section": "2. Changes Tab - Show Full Git Diff",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(diff): use git diff for file changes instead of activity events",
    "steps": [
      "Read specs/plans/review_dialog_improvements.md section '2. Changes Tab - Show Full Git Diff'",
      "Edit src-tauri/src/application/diff_service.rs get_task_file_changes method",
      "Replace activity event filtering with direct git diff --name-status",
      "Keep line count fetching for each file using existing helper",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(diff): use git diff for file changes instead of activity events"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Remove duplicate task title from sidebar",
    "plan_section": "3. Remove Duplicate Task Title",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(reviews): remove duplicate task title from sidebar",
    "steps": [
      "Read specs/plans/review_dialog_improvements.md section '3. Remove Duplicate Task Title'",
      "Edit src/components/reviews/ReviewDetailModal.tsx TaskContextSection",
      "Remove the h3 element with task title (keep title only in modal header)",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(reviews): remove duplicate task title from sidebar"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Render AI review summary as markdown",
    "plan_section": "4. AI Review Summary - Render as Markdown",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(reviews): render AI review summary as markdown",
    "steps": [
      "Read specs/plans/review_dialog_improvements.md section '4. AI Review Summary - Render as Markdown'",
      "Add imports for ReactMarkdown and remarkGfm (already in dependencies)",
      "Replace <p> with <div> containing ReactMarkdown in AIReviewSummary",
      "Add prose classes for proper markdown styling",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(reviews): render AI review summary as markdown"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Use summary field in review history for cleaner display",
    "plan_section": "5. Review History - Use Summary Field",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(reviews): use summary field in review history",
    "steps": [
      "Read specs/plans/review_dialog_improvements.md section '5. Review History - Use Summary Field'",
      "Verify ReviewEntry type has summary field (check if type needs update)",
      "Edit ReviewHistoryTimeline to use entry.summary instead of entry.notes",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(reviews): use summary field in review history"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Increase dialog size to near full screen",
    "plan_section": "6. Dialog Size - Near Full Screen",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(reviews): increase dialog size to near full screen",
    "steps": [
      "Read specs/plans/review_dialog_improvements.md section '6. Dialog Size - Near Full Screen'",
      "Edit src/components/reviews/ReviewDetailModal.tsx dialog content classes",
      "Change w-[90vw] h-[85vh] to w-[95vw] h-[95vh]",
      "Change sidebar w-[300px] to w-[400px]",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(reviews): increase dialog size to near full screen"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Use git diff directly instead of activity events** | Activity events only capture Write/Edit tool calls, missing files changed by other means (shell commands, git operations). Git diff shows the true state. |
| **Reverse commits in frontend hook** | Backend returns newest-first (git default). Reversing in hook keeps backend simple and matches what users expect (reading top-to-bottom = chronological order). |
| **Use summary field for review history** | The `notes` field contains full markdown review. The `summary` field is designed for brief excerpts suitable for timeline display. |
| **ReactMarkdown for AI summary** | AI reviews use markdown formatting. Plain text display loses headings, lists, code blocks. ReactMarkdown already used elsewhere in codebase. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run lint && npm run typecheck`
- [ ] No TypeScript errors
- [ ] No ESLint warnings

### Backend - Run `cargo clippy && cargo test`
- [ ] No clippy warnings
- [ ] Tests pass

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Manual Testing
- [ ] Open task in `review_passed` state
- [ ] Click "Review Code" → dialog opens
- [ ] Commits display oldest first (chronological)
- [ ] Changes tab shows all modified files (matching commits total)
- [ ] Task title appears only in header, not in sidebar
- [ ] AI review summary renders markdown correctly (headings, lists, code)
- [ ] Review history shows clean summary excerpts
- [ ] Dialog fills most of screen (95vw x 95vh) with 400px sidebar

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Commit reversal: useGitDiff hook → commits state → CommitList component
- [ ] Git diff: diff_service.rs → Tauri command → diffApi → useGitDiff → ChangesTab
- [ ] Markdown rendering: ReactMarkdown imported and used in AIReviewSummary

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called

See `.claude/rules/gap-verification.md` for full verification workflow.
