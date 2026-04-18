# Theme Audit Findings — 2026-04-18

Source: `.artifacts/theme-audit/{dark,light,high-contrast}/*.png` captured by `frontend/tests/visual/theme-audit/theme-audit.spec.ts` at commit `a9ca2a971`.

## Summary

- Views reviewed: 9 (activity, chat, extensibility, graph, ideation, kanban, reviews, settings, task-detail)
- Themes: dark / light / high-contrast
- Critical issues: 5
- High issues: 7
- Medium issues: 6
- Dominant pattern: ~19 components hardcode `bg-[hsl(220_10%_...)]` tailwind classes that never flip; floating overlays (Reviews, ideation sidebar, graph filter stack) also hardcode dark HSLA backgrounds inline.

## Per-View Findings

### Activity
- **Dark:** Clean. Filter chips, search field, tab strip all read correctly. Brand mark orange bleed-over on the "Activity" nav item is the intended active state.
- **Light:** Mostly clean. Loading spinner is still the dark-theme orange tint (reads OK on white). Active "Activity" nav pill is orange pill on white — OK.
- **High-Contrast:** Nav active state correctly flips to yellow (`#FFDD00`). Activity icon tile (top-left) flips to yellow outline — good. Loading spinner remains orange (should be yellow under HC).
- **Recommended fixes:**
  - Route the loading spinner color through `var(--accent-primary)` so HC gets yellow, not orange. Likely in the generic `Spinner` / `LoadingIndicator` component under `frontend/src/components/ui/`.

### Chat
- **Note:** all 3 "chat" captures show the Kanban empty-state page, not the dedicated Chat view. The spec selector for `/chat` currently resolves to the Kanban split-layout fallback (no plan selected). Treat "chat" view as **unverified** — rerun with an explicit route to the Chat panel (see Known Gaps).
- **Dark / Light / HC:** empty-state file icon with yellow sparkles reads on all three. HC correctly tints the icon tile yellow; Light keeps the peach-tinted tile (good).

### Extensibility
- **Dark:** Clean — tab strip with underline on active "Workflows", workflow card reads well, `DEFAULT` chip has a pale background.
- **Light:** Mostly clean. The workflow card background is pale gray on white, workflow bullet dot stays orange — OK. Active tab underline visible. `DEFAULT` chip looks almost invisible — uppercase "DEFAULT" text on a near-white chip with no border.
- **High-Contrast:** Active-tab underline is white (correct). `DEFAULT` chip flips to dark with white border (good). Workflow bullet dot flips to yellow — good.
- **Recommended fixes:**
  - Light-theme `DEFAULT` chip needs a visible border or darker bg. Likely a shared `<Badge variant="muted">` token pulling `--bg-hover` which is too close to `--bg-elevated` in light. Bump contrast in `src/styles/themes/light.css`.

### Graph
- **Dark:** Clean — left-side floating filter stack (Status / Vertical / Standard / Plan + Tier) has dark translucent bg, empty-state info icon and "Select plan" button read well.
- **Light:** CRITICAL — the floating filter stack (Status, Vertical, Standard, Plan + Tier) stays a DARK translucent panel on a white page; labels and chevrons are unreadable. The "No plan selected" icon + text read OK.
- **High-Contrast:** Same panel is dark on black. Shapes blend into the page; borders almost invisible. Chevrons visible, but the group is unreadable as a cohesive control.
- **Recommended fixes:**
  - `frontend/src/components/TaskGraph/controls/GraphControls.tsx` — lines ~193, 221, 279–289, 371, 383, 399, 407, 414–415, 432–433 hardcode `bg-[hsl(220_10%_10%)]` / `bg-[hsl(220_10%_12%)]` / `bg-[hsl(220_10%_15%)]` / `bg-[hsl(220_10%_10%_/_0.9)]` and matching borders. Replace with `bg-[var(--bg-elevated)]` / `bg-[var(--bg-hover)]` / `border-[var(--border-subtle)]` so they flip.
  - Same pattern exists in `TaskGraph/controls/FloatingGraphFilters.tsx`, `GraphLegend.tsx`, `groups/PlanGroupHeader.tsx`, `groups/PlanGroup.tsx`, `groups/TierGroup.tsx`, `groups/TierGroupHeader.tsx`, `groups/PlanGroupSettings.tsx`, `nodes/TaskNode.tsx`, `nodes/TaskNodeCompact.tsx`, `timeline/ExecutionTimeline.tsx` (all found via `rg 'bg-\[hsl\(220' frontend/src/components`).

### Ideation
- **Dark:** Clean. Left "Plans" sidebar, centered lightbulb hero, mode chips (Solo active with orange outline), orange "Start New Session" CTA, and bottom status bar all read well.
- **Light:** HIGH — the left plans sidebar stays dark gray while main canvas is white. Search input, "No plans yet" empty state, and sidebar body appear to sit on the dark-theme `--bg-secondary` but the main canvas is already `--bg-base` white. The bottom execution status bar (Running: 0/3 ...) also remains dark-theme colored. Orange "New Plan" CTA and orange "Start New Session" CTA correctly stay orange (brand); Solo chip has orange outline on peach fill — OK.
- **High-Contrast:** Sidebar flips to black with subtle borders — good. CTAs correctly flip to solid yellow. Mode chip "Solo" selected state is yellow outline — good. Lightbulb hero tile flips to yellow — good. Bottom status bar correctly goes solid black with readable white labels.
- **Recommended fixes:**
  - Ideation sidebar container (left rail with "Plans / Search sessions / No plans yet") — find the wrapping component under `frontend/src/components/Ideation/` (likely `PlanBrowser.tsx` or its parent layout) and verify the root `<aside>` uses `bg-[var(--bg-secondary)]` rather than a hardcoded gray. The sidebar flips correctly on HC but not Light, implying the Light token `--bg-secondary` may be set but is too close to `--bg-base` — easier to use `--bg-elevated` for the rail.
  - `ExecutionControlBar` (bottom status bar) — remains dark on Light. Confirm it uses `--bg-elevated` / `--bg-base` via theme tokens (not a hardcoded `bg-neutral-900` / `bg-[hsl(220_...)]`). If the component is `frontend/src/components/execution/ExecutionControlBar.tsx`, audit its root container className.

### Kanban
- **Dark:** Clean. Columns (BACKLOG / READY / EXECUTING) with group headers, task cards with feature/Backlog chips, progress bar for Executing task, right-side chat rail all read.
- **Light:** HIGH issues:
  - Executing Task's progress bar retains its dark-theme gradient fill (orange→blue) — this actually reads on white but the dark background stripe of the track is still dark.
  - The send-message button at the bottom-right (paper-plane icon) is a dark square with a faint icon — should flip to light surface or white-on-orange.
  - "Pause" button in the bottom bar has readable text but its hover/pressed background stays dark.
- **High-Contrast:** Mostly clean — cards flip to black-on-white text. BUT:
  - "Ready Task" card retains a blue left border (status accent stripe). Blue on black reads, but HC theme should use yellow or white, not a cold blue.
  - "Executing Task" card's left border is a warm orange (should likely become yellow for HC).
  - The "Feature" chip retains an orange icon on HC — per the rule, accent bleed should be replaced by yellow in HC.
  - Progress bar fill is yellow on HC — good.
- **Recommended fixes:**
  - Progress-bar track color — likely in `TaskCard.tsx` or `ProgressBar.tsx`; ensure track uses `var(--bg-hover)` and fill uses `var(--accent-primary)`.
  - Status-stripe color for card-left-border: map `status → color` to use `var(--status-info)` / `var(--status-success)` tokens that have HC overrides, not a literal `#3b82f6` blue or `#ff6b35` orange.
  - Send-message composer button: likely `Chat/MessageInput.tsx`. Confirm its background uses `--bg-elevated` and the icon uses `--text-primary`.

### Reviews
- **Dark:** Clean. Floating frosted-glass panel with "Reviews" header, All/AI/Human tabs, empty-state disc + "No pending reviews" read well.
- **Light:** CRITICAL — the entire Reviews floating panel renders as a dark translucent rectangle on a white page, making the tab labels, counts, and empty-state all low-contrast ghosts. The floating panel never flipped.
- **High-Contrast:** Panel is black with thin borders; tab labels and counts read OK. Empty-state disc icon is dark gray on black — barely visible; should be white-ish on HC.
- **Recommended fixes:**
  - `frontend/src/App.tsx:1140-1149` — the floating Reviews wrapper hardcodes `background: "hsla(220 10% 10% / 0.92)"`, `border: "1px solid hsla(220 20% 100% / 0.08)"`, and heavy dark shadow. Replace with theme-token-driven inline style (e.g. `background: "var(--bg-elevated-translucent)"` with theme files defining that token) OR move this styling to a CSS class that branches via `[data-theme='light']` / `[data-theme='high-contrast']`.
  - Empty-state icon in `ReviewsPanel` should use `var(--text-muted)` / `var(--text-secondary)` so HC gives visible white disc.

### Settings
- **Dark:** HIGH — the selected sidebar item ("Execution") renders as a bright white/near-white pill with dark text. This looks inverted — everywhere else the selected state is orange-tinted. All other category items read properly.
- **Light:** Clean. Selected "Execution" pill is orange on white, correct. Input fields (Max Concurrent Tasks, Project Ideation Cap) have subtle borders and readable values. Close (X) button has a subtle peach outline — fine.
- **High-Contrast:** Selected item flips to solid yellow with black text — good. Input fields have white borders on black — readable. Close (X) button has yellow outline — good. Execution icon tile is dark with yellow lightning icon — good.
- **Recommended fixes:**
  - Dark-theme selected-nav-item bg is wrong. Likely the Settings sidebar item uses a component whose `data-state=active` style resolves to `--color-white` / `--bg-base` inversion. Check `frontend/src/components/settings/SettingsSidebar.tsx` (or equivalent) — expected: `data-[state=active]:bg-[var(--accent-primary)]/15 data-[state=active]:text-[var(--accent-primary)]` OR a subtle elevated bg, not solid white.

### Task Detail (Kanban split view)
- **Dark:** Clean. Task title, feature/Backlog chips, description, empty "No steps defined yet", and right-side Task chat with "Ask about this task..." composer all read.
- **Light:** MEDIUM — "No steps defined yet" placeholder is light gray on near-white, borderline unreadable. "Ask about this task..." placeholder reads a bit faintly. Send button in composer is a dark peach tile with orange icon — low contrast vs white bg.
- **High-Contrast:** Clean — all text reads white-on-black. "No steps defined yet" placeholder correctly appears as dim gray, still legible. Send composer button flips to white icon on black — good.
- **Recommended fixes:**
  - "No steps defined yet" — should use `var(--text-muted)` not a fixed gray. Likely in the component that renders the empty steps list (`TaskDetailPanel` or `StepList` subcomponent).
  - Send button background: same fix as Kanban composer above.

## Cross-View Patterns

1. **Hardcoded `bg-[hsl(220_10%_...)]`** — 19 files use this pattern (confirmed via `rg 'bg-\[hsl\(220' frontend/src/components`). All of them will stay dark regardless of theme. Affected files:
   - `TaskGraph/controls/GraphControls.tsx`
   - `TaskGraph/controls/FloatingGraphFilters.tsx`
   - `TaskGraph/controls/GraphLegend.tsx`
   - `TaskGraph/groups/PlanGroupHeader.tsx`
   - `TaskGraph/groups/PlanGroup.tsx`
   - `TaskGraph/groups/TierGroup.tsx`
   - `TaskGraph/groups/TierGroupHeader.tsx`
   - `TaskGraph/groups/PlanGroupSettings.tsx`
   - `TaskGraph/nodes/TaskNode.tsx`
   - `TaskGraph/nodes/TaskNodeCompact.tsx`
   - `TaskGraph/timeline/ExecutionTimeline.tsx`
   - `tasks/TaskFormFields.tsx`
   - `tasks/TaskFormFields.constants.ts`
   - `tasks/TaskCreationForm.tsx`
   - `tasks/TaskCreationOverlay.tsx`
   - `tasks/TaskDetailOverlay.tsx`
   - `Chat/ConversationStatsPopover.tsx`
   - `ui/ResizeHandle.tsx`
   - `Team/TeammatePane.test.tsx`

2. **Hardcoded inline `hsla(220 ...)` in App.tsx** — the Reviews floating wrapper at `App.tsx:1140-1149`. Same class of problem.

3. **Dark-shadow bleed** — `boxShadow: "0 4px 16px hsla(220 20% 0% / 0.4)..."` on Reviews panel reads as a heavy pool of black on Light/HC. Needs theme-aware shadow token.

4. **Accent bleed in HC** — orange accents still leak in HC for:
   - Feature chip icon on Kanban cards
   - Executing card left status-stripe (orange)
   - Ready card left status-stripe (blue)
   - Activity loading spinner
   These should map to `var(--accent-primary)` (yellow on HC) or status-token overrides.

5. **Status bar (bottom) doesn't flip on Light** — visible in Ideation and partially in Kanban. `ExecutionControlBar` is suspect.

6. **"Muted" text tokens too close to bg on Light** — "No steps defined yet" and the `DEFAULT` chip are both near-invisible. Light theme's `--text-muted` / `--bg-hover` may need more separation.

## Priority Fix List (sorted by severity)

1. **CRITICAL — Reviews panel** (all themes except Dark): hardcoded dark glass panel in `App.tsx:1140-1149`. Thousands of pixels of the Light view are an opaque dark rectangle. Fix: route bg/border/shadow through theme tokens or CSS class.
2. **CRITICAL — Graph floating filter stack** (Light + HC): `TaskGraph/controls/GraphControls.tsx` hardcoded `hsl(220 10% ...)` tokens. Replace with `var(--bg-elevated)` / `var(--bg-hover)` / `var(--border-subtle)`.
3. **CRITICAL — Ideation left sidebar** (Light): sidebar does not flip, stays dark gray on white canvas. Needs `bg-[var(--bg-elevated)]` or similar on the `<aside>` root.
4. **CRITICAL — Execution control bar** (Light): bottom status bar across Kanban/Ideation remains dark. Likely component `execution/ExecutionControlBar.tsx` — audit root container for theme tokens.
5. **CRITICAL — Dark theme Settings selected item**: renders as bright white instead of accent-tinted. Audit `SettingsSidebar` active-state classes.
6. **HIGH — Kanban status-stripe colors in HC**: Ready (blue) and Executing (orange) card left borders should switch to yellow or a neutral HC-safe token.
7. **HIGH — Send / composer buttons** (Light): Kanban + Task Detail composer buttons are dark peach tiles with orange icon on white canvas. Flip to `--bg-elevated` + `--text-primary` icon OR `--accent-primary` solid bg with white icon.
8. **HIGH — Feature chip icon stays orange in HC**: chip icons should resolve to `var(--accent-primary)` (yellow) on HC.
9. **HIGH — Progress bar track** (Light): Executing Task progress bar track retains dark color. Track uses wrong token.
10. **HIGH — Status-stripe / progress gradient tokens**: all hardcoded color gradients (orange→blue) should be theme-aware.
11. **HIGH — TaskGraph nodes/groups hardcoded colors** (10+ files): every TaskGraph subcomponent uses `bg-[hsl(220_...)]`; if any graph data is present these will all look wrong on Light/HC.
12. **HIGH — Activity loading spinner in HC**: should be yellow, not orange.
13. **MEDIUM — Light theme "DEFAULT" chip invisibility**: add border or bump bg contrast in `light.css` or chip token.
14. **MEDIUM — "No steps defined yet" text in Light**: uses too-light gray on near-white; tighten `--text-muted` or switch to `--text-secondary`.
15. **MEDIUM — "No pending reviews" empty-state disc in HC**: dark gray disc on black barely visible.
16. **MEDIUM — Shadow tokens**: heavy black shadows in floating overlays look wrong on Light / wash out on HC. Add theme-aware `--shadow-elevated` token.
17. **MEDIUM — TaskCreation* + ConversationStatsPopover + ResizeHandle**: same `hsl(220_...)` hardcoding audit.
18. **MEDIUM — Solo/Research/Debate chips backgrounds**: mode chips look OK but worth verifying active/inactive backgrounds use theme tokens (not hardcoded peach fill).

## Known Gaps

- **Chat view was not captured correctly** — all 3 "chat" screenshots actually show the Kanban empty-state page. Rerun `theme-audit.spec.ts` with an explicit route or state that forces the Chat panel to render with content.
- **Insights view was not captured** (nav button gated by taskCount ≥ 10 in mock data).
- **Reviews panel fixtures are empty** — only empty-state was captured. Status badges (green success, red error, yellow warning) in real review cards were not observed across themes. Schedule a follow-up with seeded review data.
- **Kanban columns with many task states not captured** — only Backlog / Ready / Executing columns with 1 card each. Review, Merge-conflict, Failed, Approved states not seen.
- **Modal/overlay variants** — Accept modal, confirm dialogs, proposal edit modal, team artifact drawer — none captured; these are high-risk for dark-on-light scrims.
- **Task detail specialized views** (ExecutionTaskDetail, ReviewingTaskDetail, MergeConflictTaskDetail, etc.) — only BasicTaskDetail captured for a backlog task.
