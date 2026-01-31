# Plan: Visual QA Stream for Playwright Testing

## Overview

Create a dedicated `visual-qa` stream that handles both mock data parity and Playwright visual regression testing. The stream operates in two phases: bootstrap (cover existing components) and maintenance (cover new components from PRDs).

## Role Definition

**Role:** Visual QA Engineer
**Industry equivalent:** SDET (Software Development Engineer in Test) / QA Automation Engineer

This role combines:
- Mock data auditing (ensuring web mode can render features)
- Test writing (Playwright specs with visual regression)
- Baseline management (screenshot generation and updates)

## Architecture

```
streams/
├── visual-qa/
│   ├── backlog.md        # Components needing coverage
│   ├── activity.md       # Work log
│   └── manifest.md       # Coverage tracking (what's covered vs not)

tests/
├── visual/
│   ├── views/            # View-specific specs
│   │   ├── kanban/
│   │   │   └── kanban.spec.ts
│   │   ├── ideation/
│   │   │   └── ideation.spec.ts
│   │   └── activity/
│   │       └── activity.spec.ts
│   ├── modals/           # Modal-specific specs
│   │   └── task-detail.spec.ts
│   ├── states/           # Edge case specs
│   │   └── empty-states.spec.ts
│   └── snapshots/        # Baseline screenshots (auto-generated)
├── pages/                # Page Object Model
│   ├── base.page.ts
│   ├── kanban.page.ts
│   └── ideation.page.ts
├── fixtures/             # Shared setup
│   └── setup.fixtures.ts
└── helpers/              # Utilities
    └── wait.helpers.ts

.claude/rules/
└── stream-visual-qa.md   # Stream rules
```

## Files to Create

### 1. `.claude/rules/stream-visual-qa.md`

Stream rules following existing patterns (features, refactor, polish, verify, hygiene).

Key sections:
- Overview and role
- Bootstrap vs maintenance phases
- Workflow (one component per iteration)
- Mock parity checks
- Test writing patterns
- **Test code quality rules** (embedded, not separate file)
- IDLE detection

### 2. `tests/pages/base.page.ts`

Base page object with shared functionality:

```typescript
import { Page } from "@playwright/test";

export class BasePage {
  constructor(protected page: Page) {}

  async waitForApp() {
    await this.page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });
  }

  async waitForAnimations() {
    await this.page.waitForTimeout(500);
  }

  async navigateTo(path: string) {
    await this.page.goto(path);
    await this.waitForApp();
  }
}
```

### 3. `tests/fixtures/setup.fixtures.ts`

Shared setup patterns:

```typescript
import { Page } from "@playwright/test";

export async function setupApp(page: Page) {
  await page.goto("/");
  await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });
}

export async function setupKanban(page: Page) {
  await setupApp(page);
  // Wait for kanban-specific elements
  await page.waitForSelector('[data-testid^="task-card-"]');
}
```

### 4. `tests/helpers/wait.helpers.ts`

Custom wait utilities:

```typescript
import { Page } from "@playwright/test";

export async function waitForNetworkIdle(page: Page, timeout = 5000) {
  await page.waitForLoadState("networkidle", { timeout });
}

export async function waitForAnimationsComplete(page: Page) {
  await page.waitForTimeout(500);
  // Could add check for CSS animations complete
}
```

### 5. `streams/visual-qa/manifest.md`

Coverage tracking file:

```markdown
# Visual Coverage Manifest

## Bootstrap Status
Phase: IN_PROGRESS | COMPLETE

## Views (6 total)
| View | Mock Ready | Spec File | Baseline | Status |
|------|------------|-----------|----------|--------|
| kanban | ✅ | kanban.spec.ts | ✅ | covered |
| ideation | ❌ | — | — | uncovered |
| activity | ❌ | — | — | uncovered |
| settings | ❌ | — | — | uncovered |
| extensibility | ❌ | — | — | uncovered |
| task_detail | ❌ | — | — | uncovered |

## Modals (6 total)
| Modal | Mock Ready | Spec File | Baseline | Status |
|-------|------------|-----------|----------|--------|
| TaskDetailModal | ❌ | — | — | uncovered |
| ReviewsPanel | ❌ | — | — | uncovered |
| AskUserQuestionModal | ❌ | — | — | uncovered |
| ProjectCreationWizard | ❌ | — | — | uncovered |
| ProposalEditModal | ❌ | — | — | uncovered |
| PermissionDialog | ❌ | — | — | uncovered |

## States & Edge Cases
| State | View/Modal | Spec | Status |
|-------|------------|------|--------|
| empty-kanban | kanban | — | uncovered |
| all-status-columns | kanban | — | uncovered |
| loading-state | various | — | uncovered |
| error-state | various | — | uncovered |
```

### 6. `streams/visual-qa/backlog.md`

Work queue (populated from manifest):

```markdown
# Visual QA Backlog

## Uncovered Components
- [ ] [View] ideation - src/views/IdeationView.tsx
- [ ] [View] activity - src/views/ActivityView.tsx
- [ ] [Modal] TaskDetailModal - src/components/tasks/TaskDetailModal.tsx
...

## Mock Parity Issues
- [ ] [Mock] Add ideation proposals mock data for all tiers
- [ ] [Mock] Add review states (pending, approved, changes_requested)
...
```

### 7. `streams/visual-qa/activity.md`

Standard activity log format.

## Baseline Generation (CRITICAL)

**Problem:** Running `npx playwright test` on a new spec with no snapshot fails:
```
Error: A snapshot doesn't exist at [path], writing actual.
```

**Solution:** For new specs, ALWAYS use `--update-snapshots` first:

```bash
# Step 1: Write the spec file (creates tests/visual/ideation.spec.ts)

# Step 2: Generate baseline (REQUIRED for new specs)
npx playwright test ideation.spec.ts --update-snapshots
# → Creates tests/visual/snapshots/ideation-matches-snapshot-1-chromium-darwin.png

# Step 3: Verify baseline is correct
# - Check file exists: ls tests/visual/snapshots/
# - Check file size is reasonable (not 0 bytes)
# - Optionally open and visually verify

# Step 4: Subsequent runs (CI, regression checks)
npx playwright test ideation.spec.ts
# → Compares against baseline, PASSES
```

**Key behaviors:**
- `--update-snapshots` GENERATES new snapshots (doesn't fail)
- Without flag + no existing snapshot = FAIL
- Without flag + existing snapshot = COMPARE (expected behavior)

**Stream must:** Always run with `--update-snapshots` when creating new specs.

## Workflow

### Bootstrap Phase

```
1. Read streams/visual-qa/manifest.md

2. Find first "uncovered" item
   → ALL COVERED? → Set "Bootstrap Status: COMPLETE" → Switch to maintenance mode

3. For the uncovered component:
   a. Check mock parity:
      - Can web mode render this component?
      - Does mock data exist for required states?
      - If NOT: Create/extend mock data first

   b. Write Playwright spec:
      - Create page object first (tests/pages/{feature}.page.ts)
      - Create spec using page object pattern (see "Spec File Pattern" below)
      - Use fixtures for shared setup
      - Include visual regression snapshot test

   c. Generate baseline (CRITICAL):
      - Run: npx playwright test [spec] --update-snapshots
      - This CREATES the snapshot file (not a comparison)
      - Verify: ls tests/visual/snapshots/ shows new PNG
      - Verify: file size > 0 (screenshot captured correctly)

   d. Verify test passes without flag:
      - Run: npx playwright test [spec]
      - Should PASS (comparing to just-created baseline)

4. Update manifest.md: Mark as "covered"

5. Log to activity.md

6. Commit: test(visual): add [component] visual regression tests
   - Include BOTH: spec file AND snapshot file

7. STOP — end iteration
```

### Maintenance Phase

```
1. Read streams/visual-qa/backlog.md

2. Find first unchecked [ ] item
   → NO ITEMS? → Output IDLE signal → END

3. Execute same workflow as bootstrap (mock parity → spec → baseline)

4. Mark [x] in backlog.md

5. Update manifest.md

6. Commit and STOP
```

### How New Components Enter Backlog

Two mechanisms:
1. **Verify stream**: After PRD completion, could detect new components without visual tests
2. **Hygiene stream**: Could scan for new component files and add to visual-qa backlog

Or simpler: manual addition when PRD adds new UI.

## Test Code Quality Standards

### File Size Limits

| File Type | Max Lines | Refactor At | Action |
|-----------|-----------|-------------|--------|
| Spec file | 200 | 150 | Split by feature/component |
| Page Object | 150 | 100 | Extract to sub-pages |
| Fixtures | 100 | 80 | Split by domain |
| Helpers | 50 | 40 | Extract to utilities |

### Directory Structure (Modular)

```
tests/
├── visual/
│   ├── views/                    # One dir per view
│   │   ├── kanban/
│   │   │   ├── kanban.spec.ts           # Core layout tests (≤200 LOC)
│   │   │   ├── kanban-columns.spec.ts   # Column-specific tests
│   │   │   ├── kanban-cards.spec.ts     # Card-specific tests
│   │   │   └── kanban-interactions.spec.ts
│   │   ├── ideation/
│   │   │   ├── ideation.spec.ts
│   │   │   └── ideation-proposals.spec.ts
│   │   └── activity/
│   │       └── activity.spec.ts
│   ├── modals/                   # Modal-specific tests
│   │   ├── task-detail.spec.ts
│   │   ├── reviews-panel.spec.ts
│   │   └── permission-dialog.spec.ts
│   ├── states/                   # Edge case/state tests
│   │   ├── empty-states.spec.ts
│   │   ├── loading-states.spec.ts
│   │   └── error-states.spec.ts
│   └── snapshots/                # Auto-generated baselines
│
├── pages/                        # Page Object Model
│   ├── base.page.ts              # Shared navigation, waits
│   ├── kanban.page.ts            # Kanban-specific selectors/actions
│   ├── ideation.page.ts
│   └── modals/
│       ├── task-detail.page.ts
│       └── reviews.page.ts
│
├── fixtures/                     # Shared test data
│   ├── tasks.fixtures.ts         # Task-related test data
│   ├── projects.fixtures.ts
│   └── setup.fixtures.ts         # Common beforeEach setup
│
└── helpers/                      # Utility functions
    ├── wait.helpers.ts           # Custom wait conditions
    ├── screenshot.helpers.ts     # Screenshot utilities
    └── navigation.helpers.ts     # Route navigation
```

### Page Object Model (POM)

**Why:** Centralizes selectors and actions, makes tests readable, single point of change.

```typescript
// tests/pages/kanban.page.ts (≤150 LOC)
import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

export class KanbanPage extends BasePage {
  // Selectors (grouped by feature)
  readonly taskCard: (id: string) => Locator;
  readonly column: (status: string) => Locator;
  readonly searchInput: Locator;

  constructor(page: Page) {
    super(page);
    this.taskCard = (id) => page.locator(`[data-testid="task-card-${id}"]`);
    this.column = (status) => page.locator(`[data-testid="column-${status}"]`);
    this.searchInput = page.locator('[data-testid="task-search"]');
  }

  // Actions (keep simple, ≤10 lines each)
  async searchTasks(query: string) {
    await this.searchInput.fill(query);
    await this.page.keyboard.press("Enter");
  }

  async dragTaskToColumn(taskId: string, targetStatus: string) {
    await this.taskCard(taskId).dragTo(this.column(targetStatus));
  }
}
```

### Spec File Pattern

```typescript
// tests/visual/views/kanban/kanban.spec.ts (≤200 LOC)
import { test, expect } from "@playwright/test";
import { KanbanPage } from "../../../pages/kanban.page";
import { setupKanban } from "../../../fixtures/setup.fixtures";

test.describe("Kanban Board", () => {
  let kanban: KanbanPage;

  test.beforeEach(async ({ page }) => {
    kanban = new KanbanPage(page);
    await setupKanban(page);  // Shared setup
  });

  test("renders board layout", async () => {
    await expect(kanban.column("todo")).toBeVisible();
    await expect(kanban.column("in_progress")).toBeVisible();
    await expect(kanban.column("done")).toBeVisible();
  });

  test("matches snapshot", async ({ page }) => {
    await kanban.waitForAnimations();
    await expect(page).toHaveScreenshot("kanban-board.png");
  });

  // Keep test count ≤10 per file
  // If more needed → split to kanban-columns.spec.ts, etc.
});
```

### Split Triggers

| Condition | Action |
|-----------|--------|
| Spec file > 150 LOC | Split by feature area |
| > 10 tests in one file | Split by test type (visual/interaction/state) |
| Page object > 100 LOC | Extract to sub-page objects |
| Same selector in 3+ files | Move to page object |
| Same setup in 3+ files | Extract to fixture |

### Naming Conventions

| Type | Pattern | Example |
|------|---------|---------|
| Spec file | `{feature}.spec.ts` | `kanban.spec.ts` |
| Spec subset | `{feature}-{subset}.spec.ts` | `kanban-cards.spec.ts` |
| Page object | `{feature}.page.ts` | `kanban.page.ts` |
| Fixture | `{domain}.fixtures.ts` | `tasks.fixtures.ts` |
| Helper | `{purpose}.helpers.ts` | `wait.helpers.ts` |

### Code Quality Checklist (Per Spec)

Before committing a new spec:
- [ ] File ≤ 200 LOC
- [ ] Uses Page Object for selectors (no raw `data-testid` in spec)
- [ ] Shared setup extracted to fixture
- [ ] Test names are descriptive (not "test 1", "test 2")
- [ ] One assertion focus per test (avoid mega-tests)
- [ ] Snapshot name matches test purpose

## Playwright Commands Reference

| Scenario | Command | Result |
|----------|---------|--------|
| **New spec, first run** | `npx playwright test [spec] --update-snapshots` | Creates baseline PNG |
| **Regression check** | `npx playwright test [spec]` | Compares to baseline, PASS/FAIL |
| **UI intentionally changed** | `npx playwright test [spec] --update-snapshots` | Updates baseline PNG |
| **Debug failing test** | `npx playwright test [spec] --debug` | Step-through mode |
| **Interactive UI** | `npx playwright test --ui` | Visual test runner |
| **View report** | `npx playwright show-report` | Opens HTML report |

**Snapshot file location:** `tests/visual/snapshots/[test-name]-[browser]-[platform].png`
Example: `ideation-matches-snapshot-1-chromium-darwin.png`

## Mock Parity Check Process

For each component, verify:

1. **Data exists**: Does `src/api-mock/` return data for this component?
2. **Navigation works**: Can we reach this view in web mode?
3. **Renders completely**: No "undefined" or missing content
4. **States available**: Can we trigger different states (empty, loading, error)?

If mock is incomplete:
- Add to `src/api-mock/` or extend existing mock
- Update `src/api-mock/store.ts` if needed
- Add test data profile if useful

## Integration with Existing Streams

| Stream | Interaction |
|--------|-------------|
| **features** | After PRD completion, visual-qa covers new UI |
| **verify** | Could detect "component without visual test" as gap |
| **hygiene** | Could refill visual-qa backlog by scanning for uncovered components |
| **refactor** | If component structure changes, visual-qa updates tests |

## Stream Infrastructure (ACTUAL IMPLEMENTATION)

To add visual-qa as a proper stream, these files need modification:

### 1. `ralph-streams.sh` (line 59)

Add "visual-qa" to valid streams:

```bash
# Current:
VALID_STREAMS="features|refactor|polish|verify|hygiene"

# Change to:
VALID_STREAMS="features|refactor|polish|verify|hygiene|visual-qa"
```

### 2. `scripts/stream-watch-visual-qa.sh` (NEW FILE)

```bash
#!/bin/bash
# stream-watch-visual-qa.sh - fswatch wrapper for visual-qa stream
#
# Runs the visual-qa stream once on startup, then watches for file changes
# and re-runs when manifest.md or backlog.md are modified.

# Stream configuration
STREAM="visual-qa"
MODEL="${RALPH_MODEL:-sonnet}"  # Use sonnet for test writing (faster, cheaper)
WATCH_FILES=("streams/visual-qa/manifest.md" "streams/visual-qa/backlog.md")

# Source common functions
source "$(dirname "$0")/stream-watch-common.sh"

# Start the watch loop (does not return)
start_watch_loop
```

### 3. `streams/visual-qa/PROMPT.md` (NEW FILE)

```markdown
@streams/visual-qa/manifest.md @streams/visual-qa/backlog.md @.claude/rules/stream-visual-qa.md

# Visual QA Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

Follow recovery check pattern from stream rules.

---

Execute ONE component, then STOP.

## Priority
1. **Uncovered items in manifest.md** (Bootstrap phase)
2. **Backlog items** (Maintenance phase)

## Quick Workflow
```
Bootstrap? → Pick first uncovered → Page object → Spec → Baseline → Mark covered → Commit → STOP
Maintenance? → Pick backlog item → Same flow → Mark [x] → Commit → STOP
All covered? → Output IDLE signal
```

## Git Commit Rules (CRITICAL - parallel streams)

**NEVER use `git add .` or `git add -A`** — other streams have uncommitted changes!

**Follow the atomic commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All operations (check + acquire + commit + release) in ONE Bash command
- Use stream name `visual-qa`
- Commit prefix: `test(visual):`

## All components covered and backlog empty?
Output: `<promise>IDLE</promise>`

---

Full workflow in: `.claude/rules/stream-visual-qa.md`
```

### 4. `ralph-tmux.sh` (MULTIPLE LOCATIONS)

**Line ~65-70 - Add validation case:**
```bash
case "$only_stream" in
    features|refactor|polish|verify|hygiene|visual-qa) ;;
    *)
```

**Line ~104-114 - Update layout comment and create 7th pane:**
The layout will need adjustment. Options:
- Add 6th right-column pane (squeeze existing)
- Add a second column on far right
- Use tabs instead of splits

**Line ~149-163 - Add stream start:**
```bash
if [ -z "$only_stream" ] || [ "$only_stream" = "visual-qa" ]; then
    tmux send-keys -t "$SESSION_NAME:0.6" "./scripts/stream-watch-visual-qa.sh" C-m
fi
```

**Line ~166-173 - Add case for pane selection:**
```bash
visual-qa) tmux select-pane -t "$SESSION_NAME:0.6" ;;
```

**Line ~263-293 - Add restart_stream case:**
```bash
visual-qa)
    pane="6"
    script="./scripts/stream-watch-visual-qa.sh"
    ;;
```

**Line ~419 - Update help text:**
```bash
echo "Streams: features, refactor, polish, verify, hygiene, visual-qa"
```

### Tmux Layout (7 panes)

Add visual-qa as pane 6 in the right column:

```
┌─────────────────────────────────────────────────────────────────┐
│ [0] STATUS (5% height)                                          │
├─────────────────────────────────┬───────────────────────────────┤
│                                 │ [2] REFACTOR                  │
│                                 ├───────────────────────────────┤
│                                 │ [3] POLISH                    │
│ [1] FEATURES (opus)             ├───────────────────────────────┤
│ 60% width                       │ [4] VERIFY                    │
│                                 ├───────────────────────────────┤
│                                 │ [5] HYGIENE                   │
│                                 ├───────────────────────────────┤
│                                 │ [6] VISUAL-QA                 │
└─────────────────────────────────┴───────────────────────────────┘
```

**Changes to `ralph-tmux.sh`:**

1. **Line ~104-114** - Update layout comment to show 7 panes

2. **Line ~126-132** - Split right column into 5 parts instead of 4:
```bash
# Split right column into 5 equal parts for streams
# Pane 2 = REFACTOR (top), Pane 3 = bottom 80%
tmux split-window -t "$SESSION_NAME:0.2" -v -p 80

# Pane 3 = POLISH, Pane 4 = bottom 75%
tmux split-window -t "$SESSION_NAME:0.3" -v -p 75

# Pane 4 = VERIFY, Pane 5 = bottom 66%
tmux split-window -t "$SESSION_NAME:0.4" -v -p 66

# Pane 5 = HYGIENE, Pane 6 = VISUAL-QA (bottom 50%)
tmux split-window -t "$SESSION_NAME:0.5" -v -p 50
```

3. **Line ~135-140** - Add pane title:
```bash
tmux select-pane -t "$SESSION_NAME:0.6" -T "VISUAL-QA"
```

4. **Line ~96** - Add key binding:
```bash
tmux bind-key -T prefix 6 select-pane -t 6 \; resize-pane -Z
```

5. **Line ~149-163** - Add stream start:
```bash
if [ -z "$only_stream" ] || [ "$only_stream" = "visual-qa" ]; then
    tmux send-keys -t "$SESSION_NAME:0.6" "./scripts/stream-watch-visual-qa.sh" C-m
fi
```

6. **Line ~166-173** - Add case:
```bash
visual-qa) tmux select-pane -t "$SESSION_NAME:0.6" ;;
```

7. **Line ~183-187** - Add to layout echo:
```bash
echo "  [6] VISUAL-QA - Playwright tests (sonnet)$([ -n "$only_stream" ] && [ "$only_stream" != "visual-qa" ] && echo " [not started]")"
```

8. **Line ~221** - Update loop range for Ctrl+C:
```bash
for pane in 0 1 2 3 4 5 6; do
```

9. **Line ~263-293** - Add restart_stream case:
```bash
visual-qa)
    pane="6"
    script="./scripts/stream-watch-visual-qa.sh"
    ;;
```

10. **Line ~65-70** - Add validation:
```bash
features|refactor|polish|verify|hygiene|visual-qa) ;;
```

11. **Line ~419** - Update help:
```bash
echo "Streams: features, refactor, polish, verify, hygiene, visual-qa"
```

## Critical Files

### Stream Infrastructure (to modify/create)
| File | Purpose |
|------|---------|
| `ralph-streams.sh` | Add "visual-qa" to VALID_STREAMS (line 59) |
| `scripts/stream-watch-visual-qa.sh` | fswatch wrapper (to create) |
| `streams/visual-qa/PROMPT.md` | Stream prompt (to create) |
| `.claude/rules/stream-visual-qa.md` | Stream rules (to create) |
| `streams/visual-qa/manifest.md` | Coverage tracking (to create) |
| `streams/visual-qa/backlog.md` | Work queue (to create) |
| `streams/visual-qa/activity.md` | Activity log (to create) |

### Test Infrastructure (to create)
| File | Purpose |
|------|---------|
| `tests/visual/views/**/*.spec.ts` | View test files |
| `tests/visual/modals/*.spec.ts` | Modal test files |
| `tests/pages/base.page.ts` | Base page object |
| `tests/pages/*.page.ts` | Feature page objects |
| `tests/fixtures/setup.fixtures.ts` | Shared setup |
| `tests/helpers/wait.helpers.ts` | Wait utilities |

### Existing Files (reference/may modify)
| File | Purpose |
|------|---------|
| `src/api-mock/` | Mock implementations (may need extension) |
| `playwright.config.ts` | May need testDir update |
| `docs/web-testing.md` | Update with test organization |
| `ralph-tmux.sh` | Optional - add visual-qa pane |

## Verification

After implementation:

1. Run `npm run dev:web` - verify web mode starts
2. Navigate to `http://localhost:5173` - verify app renders with mock data
3. Run `npx playwright test` - verify existing tests pass
4. Create one new spec following the pattern - verify baseline generation works
5. Verify manifest tracking works correctly

## Implementation Order

### Phase 1: Test Infrastructure Setup

#### Task 1: Create directory structure (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `chore(tests): create visual test directory structure`

```
mkdir -p tests/visual/views/kanban tests/visual/modals tests/visual/states
mkdir -p tests/pages tests/pages/modals
mkdir -p tests/fixtures tests/helpers
mkdir -p streams/visual-qa
```

#### Task 2: Move existing kanban.spec.ts to new location
**Dependencies:** Task 1
**Atomic Commit:** `refactor(tests): relocate kanban spec to visual/views/kanban`

`tests/visual/kanban.spec.ts` → `tests/visual/views/kanban/kanban.spec.ts`

#### Task 3: Create base page object (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(tests): add base page object with shared wait methods`

`tests/pages/base.page.ts` (shared wait methods, navigation)

#### Task 4: Create setup fixture
**Dependencies:** Task 1
**Atomic Commit:** `feat(tests): add setup fixtures for common beforeEach`

`tests/fixtures/setup.fixtures.ts` (common beforeEach)

#### Task 5: Create wait helpers
**Dependencies:** Task 1
**Atomic Commit:** `feat(tests): add wait helper utilities`

`tests/helpers/wait.helpers.ts`

### Phase 2: Stream Infrastructure Setup

#### Task 6: Add visual-qa to valid streams (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(streams): add visual-qa to ralph-streams.sh`

Add "visual-qa" to `ralph-streams.sh` VALID_STREAMS (line 59)

#### Task 7: Create stream watcher script (BLOCKING)
**Dependencies:** Task 6
**Atomic Commit:** `feat(streams): add visual-qa stream watcher script`

Create `scripts/stream-watch-visual-qa.sh`:
- STREAM="visual-qa"
- MODEL=sonnet
- WATCH_FILES=(manifest.md, backlog.md)
- chmod +x scripts/stream-watch-visual-qa.sh

#### Task 8: Create stream prompt file
**Dependencies:** Task 6
**Atomic Commit:** `feat(streams): add visual-qa PROMPT.md`

Create `streams/visual-qa/PROMPT.md` with stream prompt

#### Task 9: Create coverage manifest
**Dependencies:** Task 6
**Atomic Commit:** `feat(streams): add visual-qa manifest.md for coverage tracking`

Create `streams/visual-qa/manifest.md` with current coverage state

#### Task 10: Create backlog file
**Dependencies:** Task 6
**Atomic Commit:** `feat(streams): add visual-qa backlog.md`

Create `streams/visual-qa/backlog.md` (empty initially)

#### Task 11: Create activity log
**Dependencies:** Task 6
**Atomic Commit:** `feat(streams): add visual-qa activity.md`

Create `streams/visual-qa/activity.md` (empty template)

#### Task 12: Create stream rules file (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `docs(rules): add stream-visual-qa.md with full workflow`

Create `.claude/rules/stream-visual-qa.md` with full workflow + quality rules

### Phase 3: Migration & Validation

#### Task 13: Update kanban spec to use page object pattern
**Dependencies:** Task 2, Task 3
**Atomic Commit:** `refactor(tests): migrate kanban spec to page object pattern`

Update existing kanban.spec.ts to use page object pattern

#### Task 14: Update playwright config testDir
**Dependencies:** Task 2
**Atomic Commit:** `chore(tests): update playwright.config.ts testDir for new structure`

Update playwright.config.ts testDir if needed for new structure

#### Task 15: Verify tests pass
**Dependencies:** Task 13, Task 14
**Atomic Commit:** None (verification only)

Verify tests still pass: `npx playwright test`

#### Task 16: Test stream manually
**Dependencies:** Task 7, Task 8, Task 9, Task 10, Task 11, Task 12
**Atomic Commit:** None (verification only)

Test stream manually: `./ralph-streams.sh visual-qa 1`

#### Task 17: Document test organization
**Dependencies:** Task 15
**Atomic Commit:** `docs(testing): add test organization section to web-testing.md`

Document in `docs/web-testing.md` (add section on test organization)

### Phase 4: First New Spec (Validation)

#### Task 18: Pick uncovered view
**Dependencies:** Task 9
**Atomic Commit:** None (planning only)

Pick one uncovered view (e.g., ideation)

#### Task 19: Ensure mock data exists
**Dependencies:** Task 18
**Atomic Commit:** `feat(mocks): add ideation view mock data` (if needed)

Ensure mock data exists for ideation view

#### Task 20: Create ideation page object
**Dependencies:** Task 3, Task 18
**Atomic Commit:** `feat(tests): add ideation page object`

Create page object: `tests/pages/ideation.page.ts`

#### Task 21: Create ideation spec
**Dependencies:** Task 4, Task 5, Task 20
**Atomic Commit:** `test(visual): add ideation view visual regression spec`

Create spec: `tests/visual/views/ideation/ideation.spec.ts`

#### Task 22: Generate baseline snapshot
**Dependencies:** Task 21
**Atomic Commit:** `test(visual): add ideation baseline snapshot`

Generate baseline: `npx playwright test ideation --update-snapshots`

#### Task 23: Update manifest coverage
**Dependencies:** Task 22
**Atomic Commit:** `chore(streams): mark ideation as covered in manifest`

Verify quality rules followed, update manifest.md

### Phase 5: tmux Integration

#### Task 24: Modify ralph-tmux.sh for visual-qa pane
**Dependencies:** Task 6
**Atomic Commit:** `feat(tmux): add visual-qa pane to ralph-tmux.sh`

Modify `ralph-tmux.sh` to add visual-qa pane:
- Add pane 6 key binding (line ~96)
- Update layout to split right column 5 ways (lines ~126-132)
- Add VISUAL-QA pane title (line ~135-140)
- Add stream validation case (line ~65-70)
- Add stream start command (line ~149-163)
- Add pane selection case (line ~166-173)
- Add layout echo line (line ~183-187)
- Update Ctrl+C loop to include pane 6 (line ~221)
- Add restart_stream case (line ~263-293)
- Update help text (line ~419)

#### Task 25: Verify tmux session
**Dependencies:** Task 24
**Atomic Commit:** None (verification only)

Test full tmux session: `./ralph-tmux.sh start`
- Verify 7 panes display correctly
- Verify visual-qa stream starts

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
