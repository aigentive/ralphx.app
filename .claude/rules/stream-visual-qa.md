# Visual QA Stream

**Required Context:** @.claude/rules/code-quality-standards.md | @.claude/rules/git-workflow.md

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## NEVER Rules (Cannot Skip)

| Situation | ❌ WRONG | ✅ RIGHT |
|-----------|----------|----------|
| Mock doesn't render | Log "blocked", skip | FIX the mock, then test |
| Dev server needs restart | Wait for user | Restart it yourself (rule #7) |
| Component has no UI trigger | Mark blocked | Create test helper to trigger |
| Test times out | Log and move on | Debug and fix root cause |
| Missing mock data | Document as P1 | Add mock to `src/api-mock/` |
| Discovery finds items | Add to backlog, IDLE | Add to backlog, COMPLETE (triggers new cycle) |

## Rules

1. **ONE component per iteration, then STOP**
2. **Bootstrap first** — cover uncovered items from manifest.md before backlog
3. **Mock parity MUST FIX** — component must render in web mode (see decision tree below)
4. **Page Object Model** — no raw selectors in spec files
5. **Baseline required** — always `--update-snapshots` for new specs
6. **Follow git workflow** — see @.claude/rules/git-workflow.md
7. **Dev server management ALLOWED** — this stream CAN start/restart `npm run dev:web`

## Mock Parity (MUST FIX)

```
Component doesn't render in web mode?
├─ Missing mock data? → Add to src/api-mock/ → Include in commit
├─ Mock exists but broken? → Fix the mock → Include in commit
├─ Needs server restart? → Restart server (allowed, rule #7)
├─ Needs state manipulation? → Create test helper
└─ NEVER log "blocked" and skip — FIX IT
```

## Dev Server Management

**Exception to CLAUDE.md rule #8:** This stream manages the web dev server.

| Scenario | Action |
|----------|--------|
| Not running | `npm run dev:web &` |
| Need reload | `pkill -f "vite.*5173" || true` then `npm run dev:web &` |
| Running, no changes | Leave alone |

**Check:** `curl -s http://localhost:5173 > /dev/null && echo "running" || echo "not running"`

## Recovery Check

Follow `git-workflow.md` Recovery Check. Ownership: `streams/visual-qa/manifest.md`, `streams/visual-qa/backlog.md`, `tests/visual/`, `tests/pages/`, `tests/fixtures/`, `tests/helpers/`

## Workflow

```
1. Read streams/visual-qa/manifest.md
   → Uncovered item? → Work it (step 3)
   → All covered? → Set "Bootstrap Status: COMPLETE"

2. Read streams/visual-qa/backlog.md
   → Item exists? → Work it (step 3)
   → Empty? → Discovery (step 6)

3. For component:
   a. Mock parity — FIX issues (see decision tree above)
   b. Page object — tests/pages/{feature}.page.ts (extend BasePage)
   c. Spec — tests/visual/{views|modals|states}/{feature}/{feature}.spec.ts
   d. Baseline — `npx playwright test [spec] --update-snapshots`
   e. Verify — `npx playwright test [spec]` passes

4. Update manifest.md (mark covered) | backlog.md (mark [x])

5. Commit: test(visual): add [component] visual regression tests
   → STOP

6. Discovery: Explore src/components/, src/views/, src/modals/
   → New components? → Add to manifest (uncovered) + backlog → COMPLETE signal → END
   → None found? → IDLE signal → END
```

## File Patterns

| Type | Pattern | Example |
|------|---------|---------|
| Spec | `{feature}.spec.ts` | `kanban.spec.ts` |
| Spec subset | `{feature}-{subset}.spec.ts` | `kanban-cards.spec.ts` |
| Page object | `{feature}.page.ts` | `kanban.page.ts` |
| Fixture | `{domain}.fixtures.ts` | `tasks.fixtures.ts` |
| Helper | `{purpose}.helpers.ts` | `wait.helpers.ts` |

## File Size Limits

| Type | Max | Refactor At |
|------|-----|-------------|
| Spec | 200 | 150 |
| Page Object | 150 | 100 |
| Fixture | 100 | 80 |
| Helper | 50 | 40 |

## Playwright Commands

| Scenario | Command |
|----------|---------|
| New spec | `npx playwright test [spec] --update-snapshots` |
| Regression | `npx playwright test [spec]` |
| UI changed | `npx playwright test [spec] --update-snapshots` |
| Debug | `npx playwright test [spec] --debug` |

## Signals

| Signal | When | Effect |
|--------|------|--------|
| `COMPLETE` | After doing work (tests, discovery added items) | Exit → fswatch detects file changes → new cycle |
| `IDLE` | Discovery found nothing | Exit → fswatch waits for external changes |
| (none) | After step 5 commit | Continue to next iteration |

**NEVER output IDLE if you added items to backlog** — that's work done, use COMPLETE.

## Signal Output Rules

Output signals as standalone final statement. Never quote `<promise>` tags — refer to "the IDLE signal" in logs.

## Activity Log Format

```markdown
### YYYY-MM-DD HH:MM:SS - [Component Name] Visual Tests
**What:** Created page object + spec + baseline
**Mock parity:** ready | extended mock for X
**Commands:** `npx playwright test [spec] --update-snapshots`
**Result:** Success/Failed
```

## Reference

- Code quality: @.claude/rules/code-quality-standards.md
- Git workflow: @.claude/rules/git-workflow.md
- Commit lock: @.claude/rules/commit-lock.md
