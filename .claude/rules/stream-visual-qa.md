# Visual QA Stream

**Required Context:** @.claude/rules/code-quality-standards.md | @.claude/rules/git-workflow.md

## Overview

The visual-qa stream handles **visual regression testing and mock data parity auditing**. It ensures UI components render correctly and have proper mock data for browser-based testing.

**Focus:** Write Playwright visual regression tests using Page Object Model, maintain baseline screenshots, and audit mock data completeness.

## Rules

1. **ONE component per iteration, then STOP**
2. **Bootstrap phase first** — cover uncovered components from manifest.md before processing backlog
3. **Mock parity before tests** — ensure component can render in web mode before writing tests
4. **Page Object Model mandatory** — no raw selectors in spec files
5. **Baseline generation required** — always run `--update-snapshots` for new specs
6. **Follow git workflow rules** — see @.claude/rules/git-workflow.md

## Recovery Check (ALWAYS FIRST)

Follow the Recovery Check in `git-workflow.md` with these ownership rules:

```
Ownership sources:
- streams/visual-qa/manifest.md (uncovered items)
- streams/visual-qa/backlog.md (maintenance items)
- Files in tests/visual/, tests/pages/, tests/fixtures/, tests/helpers/

Match if: File path is in test directories OR mentioned in manifest/backlog
```

**If uncommitted files match, they are YOUR WORK. Complete and commit before proceeding.**

## Workflow

### Bootstrap Phase

```
1. Read streams/visual-qa/manifest.md

2. Find first "uncovered" item (Status: uncovered)
   → ALL COVERED? → Set "Bootstrap Status: COMPLETE" → Switch to maintenance

3. For the uncovered component:
   a. Check mock parity:
      - Can web mode render this component?
      - Does mock data exist for required states?
      - If NOT: Create/extend mock data first

   b. Create page object (if not exists):
      - tests/pages/{feature}.page.ts
      - Extend BasePage, add feature-specific selectors

   c. Write Playwright spec:
      - tests/visual/views/{feature}/{feature}.spec.ts (or modals/, states/)
      - Use page object for all selectors
      - Use fixtures for shared setup
      - Include visual regression snapshot test

   d. Generate baseline (CRITICAL):
      - Run: npx playwright test [spec] --update-snapshots
      - Verify: snapshot file exists and is non-empty

   e. Verify test passes:
      - Run: npx playwright test [spec]
      - Should PASS (comparing to baseline)

4. Update manifest.md: Mark as "covered"

5. Log to streams/visual-qa/activity.md

6. Commit: test(visual): add [component] visual regression tests
   - Include: spec file, page object, snapshot file

7. STOP — end iteration
```

### Maintenance Phase

```
1. Read streams/visual-qa/backlog.md

2. Find first unchecked [ ] item
   → NO ITEMS? → Run Discovery Phase

3. Execute same workflow as bootstrap (mock parity → page object → spec → baseline)

4. Mark [x] in backlog.md

5. Update manifest.md if adding new coverage

6. Commit and STOP
```

### Discovery Phase (Self-Backfill)

When bootstrap complete AND backlog empty, discover new components:

```
1. Run Explore agent to find NEW components:
   - Scan src/components/, src/views/, src/modals/
   - Compare against streams/visual-qa/manifest.md
   - Identify components NOT in manifest

2. Found new components?
   → Add to manifest.md with Status: uncovered
   → Add to backlog.md as items
   → Return to Bootstrap Phase (work the newly discovered item)

3. No new components found? → Output IDLE signal → END
```

### Explore Prompt for Component Discovery

```
Scan src/ for React components not in streams/visual-qa/manifest.md:
- Check src/components/**/*.tsx
- Check src/views/**/*.tsx
- Check src/modals/**/*.tsx

For each component file found:
1. Is it in manifest.md? → Skip
2. Not in manifest? → Report as new

Output format:
- [View/Modal/Component] ComponentName - src/path/to/file.tsx

Maximum 10 items.
```

## Test Code Quality Standards

### File Size Limits

| File Type | Max Lines | Refactor At | Action |
|-----------|-----------|-------------|--------|
| Spec file | 200 | 150 | Split by feature area |
| Page Object | 150 | 100 | Extract to sub-page objects |
| Fixtures | 100 | 80 | Split by domain |
| Helpers | 50 | 40 | Extract to utilities |

### Page Object Model (POM)

**All selectors must be in page objects, never in spec files.**

```typescript
// tests/pages/kanban.page.ts
import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

export class KanbanPage extends BasePage {
  readonly taskCard: (id: string) => Locator;
  readonly column: (status: string) => Locator;

  constructor(page: Page) {
    super(page);
    this.taskCard = (id) => page.locator(`[data-testid="task-card-${id}"]`);
    this.column = (status) => page.locator(`[data-testid="column-${status}"]`);
  }

  async searchTasks(query: string) {
    await this.searchInput.fill(query);
  }
}
```

### Spec File Pattern

```typescript
// tests/visual/views/kanban/kanban.spec.ts
import { test, expect } from "@playwright/test";
import { KanbanPage } from "../../../pages/kanban.page";
import { setupKanban } from "../../../fixtures/setup.fixtures";

test.describe("Kanban Board", () => {
  let kanban: KanbanPage;

  test.beforeEach(async ({ page }) => {
    kanban = new KanbanPage(page);
    await setupKanban(page);
  });

  test("renders board layout", async () => {
    await expect(kanban.column("todo")).toBeVisible();
  });

  test("matches snapshot", async ({ page }) => {
    await kanban.waitForAnimations();
    await expect(page).toHaveScreenshot("kanban-board.png");
  });
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

Before committing:
- [ ] File <= 200 LOC
- [ ] Uses Page Object for selectors (no raw `data-testid` in spec)
- [ ] Shared setup extracted to fixture
- [ ] Test names are descriptive
- [ ] One assertion focus per test
- [ ] Snapshot name matches test purpose

## Baseline Generation (CRITICAL)

**Problem:** Running `npx playwright test` on a new spec with no snapshot fails.

**Solution:** For new specs, ALWAYS use `--update-snapshots` first:

```bash
# Step 1: Write spec file

# Step 2: Generate baseline (REQUIRED for new specs)
npx playwright test [spec] --update-snapshots

# Step 3: Verify baseline exists
ls tests/visual/snapshots/

# Step 4: Subsequent runs (CI, regression)
npx playwright test [spec]
```

## Mock Parity Check

For each component, verify:

1. **Data exists**: Does `src/api-mock/` return data for this component?
2. **Navigation works**: Can we reach this view in web mode?
3. **Renders completely**: No "undefined" or missing content
4. **States available**: Can we trigger different states (empty, loading, error)?

If mock is incomplete:
- Add to `src/api-mock/` or extend existing mock
- This counts as part of the iteration — include in commit

## IDLE Detection

**When:** At the START of an iteration, NOT after completing work.

**Condition:** Bootstrap complete AND backlog empty AND discovery finds no new components

**Flow:**
```
1. Bootstrap incomplete? → Work bootstrap item
2. Backlog has items? → Work backlog item
3. Discovery finds new components? → Add to manifest/backlog → Work item
4. Nothing to do? → IDLE
```

**Action:** Output `<promise>IDLE</promise>`

This signals the fswatch wrapper to take over and wait for file changes.

**NEVER output IDLE after completing work.** Just end the iteration.

## Signal Output Rules

**CRITICAL:** Completion signals must be output as a **standalone final statement**.

- Output the signal as your LAST message content
- Do NOT quote or mention the signal syntax elsewhere in your output
- When discussing signals in logs/activity, refer to them as "the IDLE signal" — never the actual `<promise>` tags

## Activity Log Format

Log entries go in `streams/visual-qa/activity.md`:

```markdown
### YYYY-MM-DD HH:MM:SS - [Component Name] Visual Tests
**What:**
- Created page object: tests/pages/[feature].page.ts
- Created spec: tests/visual/[path]/[feature].spec.ts
- Generated baseline snapshot

**Mock parity:**
- [Status: ready | extended mock data for X]

**Commands:**
- `npx playwright test [spec] --update-snapshots`
- `npx playwright test [spec]`

**Result:** Success/Failed
```

## Playwright Commands Reference

| Scenario | Command |
|----------|---------|
| New spec, first run | `npx playwright test [spec] --update-snapshots` |
| Regression check | `npx playwright test [spec]` |
| UI intentionally changed | `npx playwright test [spec] --update-snapshots` |
| Debug failing test | `npx playwright test [spec] --debug` |

## Reference

- Code quality standards: @.claude/rules/code-quality-standards.md
- Git workflow: @.claude/rules/git-workflow.md
- Commit lock: @.claude/rules/commit-lock.md
