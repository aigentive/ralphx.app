# RalphX - Phase 56: Visual QA Stream for Playwright Testing

## Overview

Create a dedicated `visual-qa` stream that handles both mock data parity auditing and Playwright visual regression testing. The stream operates in two phases: bootstrap (cover existing components) and maintenance (cover new components from PRDs). This adds a sixth autonomous stream to the multi-stream RALPH architecture, enabling continuous visual quality assurance.

**Reference Plan:**
- `specs/plans/visual_qa_stream_for_playwright_testing.md` - Complete implementation details for test infrastructure, stream setup, and quality standards

## Goals

1. Establish Page Object Model (POM) test infrastructure for maintainable Playwright tests
2. Create visual-qa stream with bootstrap and maintenance workflows
3. Integrate with tmux orchestration for parallel stream execution
4. Enable automated visual regression testing with baseline management

## Dependencies

### Phase 55 (Web Target for Browser Testing) - Required

| Dependency | Why Needed |
|------------|------------|
| Web mode with mock backend | Playwright tests run against browser mode, not Tauri |
| Mock data infrastructure | Visual tests need deterministic mock data |
| Existing kanban.spec.ts | Migration target for new directory structure |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/visual_qa_stream_for_playwright_testing.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Test files: `test(visual):` / `feat(tests):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/visual_qa_stream_for_playwright_testing.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Create test directory structure and base page object",
    "plan_section": "Phase 1: Test Infrastructure Setup - Tasks 1, 3",
    "blocking": [2, 3, 4, 5],
    "blockedBy": [],
    "atomic_commit": "feat(tests): add visual test directory structure and base page object",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section 'Phase 1: Test Infrastructure Setup'",
      "Create directory structure: tests/visual/views/kanban, tests/visual/modals, tests/visual/states, tests/pages, tests/pages/modals, tests/fixtures, tests/helpers, streams/visual-qa",
      "Create tests/pages/base.page.ts with BasePage class (waitForApp, waitForAnimations, navigateTo methods)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tests): add visual test directory structure and base page object"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Create setup fixtures and wait helpers",
    "plan_section": "Phase 1: Test Infrastructure Setup - Tasks 4, 5",
    "blocking": [6],
    "blockedBy": [1],
    "atomic_commit": "feat(tests): add setup fixtures and wait helpers",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section 'Files to Create'",
      "Create tests/fixtures/setup.fixtures.ts with setupApp and setupKanban functions",
      "Create tests/helpers/wait.helpers.ts with waitForNetworkIdle and waitForAnimationsComplete",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tests): add setup fixtures and wait helpers"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Move existing kanban spec to new location and update imports",
    "plan_section": "Phase 1: Test Infrastructure Setup - Task 2",
    "blocking": [6],
    "blockedBy": [1],
    "atomic_commit": "refactor(tests): relocate kanban spec to visual/views/kanban",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section 'Phase 1'",
      "Move tests/visual/kanban.spec.ts to tests/visual/views/kanban/kanban.spec.ts",
      "Update any relative import paths if needed",
      "Run npx playwright test to verify test still passes",
      "Commit: refactor(tests): relocate kanban spec to visual/views/kanban"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "documentation",
    "description": "Create stream rules file with full workflow and quality standards",
    "plan_section": "Files to Create - .claude/rules/stream-visual-qa.md",
    "blocking": [7],
    "blockedBy": [1],
    "atomic_commit": "docs(rules): add stream-visual-qa.md with full workflow",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md sections 'Workflow', 'Test Code Quality Standards'",
      "Create .claude/rules/stream-visual-qa.md following existing stream rule patterns (features, refactor, polish, verify, hygiene)",
      "Include: Overview/role, Bootstrap vs maintenance phases, Workflow, Mock parity checks, Test writing patterns, Test code quality rules, IDLE detection",
      "Commit: docs(rules): add stream-visual-qa.md with full workflow"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "agent",
    "description": "Add visual-qa to valid streams in ralph-streams.sh",
    "plan_section": "Stream Infrastructure - ralph-streams.sh",
    "blocking": [7, 8, 9],
    "blockedBy": [1],
    "atomic_commit": "feat(streams): add visual-qa to ralph-streams.sh",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section 'Stream Infrastructure'",
      "Edit ralph-streams.sh line 59: change VALID_STREAMS to include visual-qa",
      "Test: ./ralph-streams.sh visual-qa 1 should recognize stream (will fail on missing PROMPT.md, that's expected)",
      "Commit: feat(streams): add visual-qa to ralph-streams.sh"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Create kanban page object and migrate spec to POM pattern",
    "plan_section": "Test Code Quality Standards - Page Object Model",
    "blocking": [10],
    "blockedBy": [2, 3],
    "atomic_commit": "refactor(tests): migrate kanban spec to page object pattern",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section 'Page Object Model (POM)'",
      "Create tests/pages/kanban.page.ts extending BasePage with kanban-specific selectors and actions",
      "Update tests/visual/views/kanban/kanban.spec.ts to use KanbanPage and setupKanban fixture",
      "Run npx playwright test to verify tests pass",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(tests): migrate kanban spec to page object pattern"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "agent",
    "description": "Create stream watcher script for visual-qa",
    "plan_section": "Stream Infrastructure - scripts/stream-watch-visual-qa.sh",
    "blocking": [10],
    "blockedBy": [4, 5],
    "atomic_commit": "feat(streams): add visual-qa stream watcher script",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section 'Stream Infrastructure'",
      "Create scripts/stream-watch-visual-qa.sh with STREAM=visual-qa, MODEL=sonnet, WATCH_FILES=(manifest.md, backlog.md)",
      "Source stream-watch-common.sh and call start_watch_loop",
      "chmod +x scripts/stream-watch-visual-qa.sh",
      "Commit: feat(streams): add visual-qa stream watcher script"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "agent",
    "description": "Create stream prompt and supporting files (manifest, backlog, activity)",
    "plan_section": "Stream Infrastructure - streams/visual-qa/",
    "blocking": [10],
    "blockedBy": [5],
    "atomic_commit": "feat(streams): add visual-qa stream files",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md sections on PROMPT.md, manifest.md, backlog.md, activity.md",
      "Create streams/visual-qa/PROMPT.md with stream prompt referencing manifest, backlog, and rules",
      "Create streams/visual-qa/manifest.md with coverage tracking tables (views, modals, states)",
      "Create streams/visual-qa/backlog.md with empty uncovered components section",
      "Create streams/visual-qa/activity.md with standard activity log template",
      "Commit: feat(streams): add visual-qa stream files"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Update playwright config for new test structure",
    "plan_section": "Phase 3: Migration & Validation - Task 14",
    "blocking": [10],
    "blockedBy": [5],
    "atomic_commit": "chore(tests): update playwright.config.ts for new structure",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section on Playwright config",
      "Check playwright.config.ts testDir setting",
      "Update testDir if needed to support tests/visual structure",
      "Run npx playwright test to verify all tests still pass",
      "Commit: chore(tests): update playwright.config.ts for new structure"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "agent",
    "description": "Modify ralph-tmux.sh to add visual-qa pane",
    "plan_section": "Phase 5: tmux Integration - Task 24",
    "blocking": [11],
    "blockedBy": [6, 7, 8, 9],
    "atomic_commit": "feat(tmux): add visual-qa pane to ralph-tmux.sh",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section 'Changes to ralph-tmux.sh'",
      "Add pane 6 key binding",
      "Update layout to split right column 5 ways",
      "Add VISUAL-QA pane title",
      "Add stream validation case",
      "Add stream start command",
      "Add pane selection case",
      "Add layout echo line",
      "Update Ctrl+C loop to include pane 6",
      "Add restart_stream case",
      "Update help text",
      "Commit: feat(tmux): add visual-qa pane to ralph-tmux.sh"
    ],
    "passes": false
  },
  {
    "id": 11,
    "category": "documentation",
    "description": "Document test organization in web-testing.md",
    "plan_section": "Phase 3: Migration & Validation - Task 17",
    "blocking": [],
    "blockedBy": [10],
    "atomic_commit": "docs(testing): add test organization section to web-testing.md",
    "steps": [
      "Read specs/plans/visual_qa_stream_for_playwright_testing.md section 'Directory Structure (Modular)'",
      "Add section to docs/web-testing.md documenting the test organization",
      "Include: Page Object Model pattern, fixture usage, naming conventions, quality standards",
      "Commit: docs(testing): add test organization section to web-testing.md"
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
| **Page Object Model for tests** | Centralizes selectors and actions, makes tests readable, single point of change |
| **Separate visual-qa stream** | Dedicated focus on visual quality without mixing with feature development |
| **Bootstrap then maintenance** | Cover existing components first, then maintain coverage for new PRD work |
| **Snapshots committed to git** | Baseline screenshots are source of truth, needed by CI and other developers |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Test Infrastructure
- [ ] `tests/pages/base.page.ts` exists with BasePage class
- [ ] `tests/fixtures/setup.fixtures.ts` exists with setup functions
- [ ] `tests/helpers/wait.helpers.ts` exists with wait utilities
- [ ] Existing kanban test passes with POM pattern

### Stream Infrastructure
- [ ] `./ralph-streams.sh visual-qa 1` recognizes stream and runs
- [ ] `streams/visual-qa/PROMPT.md` exists and is valid
- [ ] `streams/visual-qa/manifest.md` has coverage tracking tables
- [ ] `.claude/rules/stream-visual-qa.md` documents full workflow

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] `npx playwright test` passes

### Manual Testing
- [ ] Start tmux session with `./ralph-tmux.sh start` - 7 panes display
- [ ] Visual-qa stream starts and shows manifest/backlog watching

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Stream watcher sources common functions and calls start_watch_loop
- [ ] PROMPT.md references correct manifest, backlog, and rules files
- [ ] ralph-streams.sh includes visual-qa in VALID_STREAMS

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
