# Visual Gap Verification

**Required Context:** @.claude/rules/gap-verification.md | @.claude/rules/visual-verification.md

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## When to Run

**Trigger:** After code gap verification passes (all PRD tasks have `"passes": true`)

**Before:** Updating manifest to complete the phase

## Verification Checks

### Check 1: Component Coverage

For each UI component added/modified in this phase:

1. Is the component listed in the PRD tasks?
2. Did the task trigger per-task visual verification (step 6.5)?
3. Screenshot exists in `screenshots/features/`?

**Output if gap:** List components without visual verification

### Check 2: Mock Parity

For each new UI component:

1. Identify Tauri commands it uses (grep for `invoke(`)
2. Check `src/api-mock/` for matching mock functions
3. Verify web mode renders the component without errors

**Test:** `npm run dev:web` → navigate to view → no undefined/error states

**Output if gap:** List components with missing/broken mock data

### Check 3: Visual Feature Verification (Agent-Browser)

For each **key feature** in the phase PRD:

1. Start dev server (web mode): `npm run dev:web`
2. Navigate to the feature view
3. Use agent-browser to:
   - Open the feature
   - Interact with key flows
   - Take screenshots
   - AI-judge: Does behavior match PRD acceptance criteria?

**Focus areas:**
- Primary user flow works end-to-end
- No visual regressions in surrounding UI
- Design system compliance (accent #ff6b35, SF Pro font)

**Output if gap:** List visual issues with screenshots

### Check 4: Modal Testability Audit (P1, Non-Blocking)

For each modal added in this phase:

1. Can it be triggered in web mode?
2. If NO → Add to P1 backlog (not blocking)

**Note:** Modals without web-mode triggers are P1 technical debt, not phase blockers.

## Workflow

```
1. Code gap verification passes (wiring, API, state, events, types)

2. Run visual gap verification:
   a. Component Coverage check
   b. Mock Parity check
   c. Visual Feature Verification (agent-browser)
   d. Modal Testability audit (P1 logging only)

3. Visual gaps found (Check 1-3)?
   → Document in streams/features/activity.md
   → DO NOT complete phase
   → Fix issues, re-run verification

4. No visual gaps (or only P1 modal issues)?
   → Phase complete, update manifest
```

## Explore Prompt: Component Coverage

```
For Phase [N], find UI components from PRD tasks:
1. Read the phase PRD
2. List components mentioned in task descriptions
3. For each: check if screenshots/features/ has corresponding screenshot
Report components without visual verification.
```

## Explore Prompt: Mock Parity

```
For Phase [N], verify mock parity:
1. Find new/modified components in src/components/, src/views/
2. Grep for invoke() calls to identify Tauri commands used
3. Check src/api-mock/ for matching mock implementations
Report components with missing mock functions.
```

## Agent-Browser Visual Verification Steps

For each key feature:

```
1. Ensure dev server running at http://localhost:5173
2. Navigate to feature view
3. Execute key user flow:
   - Click primary action
   - Verify response/state change
   - Check no console errors
4. Take screenshot: YYYY-MM-DD_HH-MM-SS_phase-N-[feature].png
5. AI-judge against PRD acceptance criteria:
   - Does the feature work as described?
   - Does it match design specs?
   - Any visual regressions?
```

## Visual Gap P0 Format

When visual gaps block completion:

```markdown
### P0 - Visual Gaps (Phase N)

- [ ] [Visual/Coverage] ComponentName - no visual verification screenshot
- [ ] [Visual/Mock] ComponentName - missing mock for `command_name`
- [ ] [Visual/Regression] FeatureName - doesn't match acceptance criteria
```

## P1 Format (Non-Blocking)

For modal testability issues:

```markdown
### P1 - Modal Testability

- [ ] [P1] [Modal] ModalName - no web-mode trigger available
```
