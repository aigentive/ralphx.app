# Handover: Agent Teams UI Gap Implementation

**Date:** 2026-02-15
**Context:** Two rounds of UI gap implementation + Opus gap verification
**Status:** 7 remaining gaps, testing gap team planned

---

## 1. What Was Done

### Round 1: Ideation Brief Gaps (Team: team-ui-gaps)

Verified `docs/product-briefs/agent-teams-ideation-integration.md` (v5) against the frontend. Found 4 gaps, fixed all 4 with a 4-worker team:

| Task | Worker | Files Created/Modified |
|------|--------|----------------------|
| Session creation team mode selector | session-creation-worker | `src/components/Ideation/StartSessionPanel.tsx` (modified, 332 LOC), `src/components/Ideation/TeamConfigPanel.tsx` (new, 154 LOC) |
| Team findings in PlanDisplay | plan-findings-worker | `src/components/Ideation/TeamFindingsSection.tsx` (new, 152 LOC), `src/components/Ideation/TeamFindingsSection.test.tsx` (new), `src/components/Ideation/PlanDisplay.tsx` (modified +33 LOC) |
| Debate summary UI | debate-ui-worker | `src/components/Ideation/DebateSummary.tsx` (new, 284 LOC), `src/components/Ideation/DebateAdvocateCard.tsx` (new, 189 LOC), `src/components/Ideation/DebateSummary.test.tsx` (new, 189 LOC) |
| API types for team mode | api-types-worker | `src/types/ideation.ts` (modified), `src/api/ideation.schemas.ts` (modified), `src/api/ideation.ts` (modified), `src/hooks/useIdeation.ts` (modified) |

**Post-team fix:** Updated `src/hooks/useIdeation.test.ts` — 2 assertions needed updating for the new 5-param `sessions.create` signature.

### Round 2: All Briefs Gaps (Team: team-ui-gaps-v2)

Audited all 6 approved agent-teams briefs and found 13 additional gaps. Implemented with a 5-worker team:

| Task | Worker | Files Created/Modified |
|------|--------|----------------------|
| Split-pane core (store, grid, header, ViewType) | split-pane-core | `src/stores/splitPaneStore.ts` (new), `src/components/Team/TeamSplitView.tsx` (new), `src/components/Team/TeamSplitHeader.tsx` (new), `src/components/Team/TeamSplitGrid.tsx` (new), `src/components/Team/index.tsx` (new barrel), `src/types/chat.ts` (modified — added "team"), `src/stores/uiStore.ts` (modified — added previousView), `src/App.tsx` (modified — added team view case) |
| Split-pane panes (7 components) | split-pane-panes | `src/components/Team/CoordinatorPane.tsx` (new, 160 LOC), `src/components/Team/TeamOverviewHeader.tsx` (new, 75 LOC), `src/components/Team/TeammatePaneGrid.tsx` (new, 61 LOC), `src/components/Team/TeammatePane.tsx` (new, 97 LOC), `src/components/Team/PaneHeader.tsx` (new, 113 LOC), `src/components/Team/PaneStream.tsx` (new, 84 LOC), `src/components/Team/PaneInput.tsx` (new, 93 LOC) |
| Split-pane hooks + keyboard | split-pane-hooks | `src/hooks/usePaneEvents.ts` (new, 85 LOC), `src/hooks/useTeamKeyboardNav.ts` (new, 120 LOC), `src/hooks/useTeamViewLifecycle.ts` (new, 70 LOC), `src/hooks/usePaneResize.ts` (new, 85 LOC), `src/components/Team/PrefixKeyOverlay.tsx` (new, 40 LOC) |
| Worker team UX (exec mode + process grouping) | worker-team-ux | `src/components/tasks/detail-views/BasicTaskDetail.tsx` (modified — added ExecutionModeSelector), `src/components/execution/TeamProcessGroup.tsx` (new, ~160 LOC), `src/components/execution/RunningProcessPopover.tsx` (modified), `src/api/tasks.ts` (modified — agentVariant param), `src/api/running-processes.types.ts` (modified — teamName, teammates fields) |
| API + polish fixes (5 sub-gaps) | api-polish | `src/api/team.ts` (modified — 3-param sendTeamMessage with target), `src/hooks/useTeamActions.ts` (modified), `src/hooks/useChat.ts` (modified — target param), `src/hooks/useChatActions.ts` (modified), `src/components/Ideation/TeamConfigPanel.tsx` (modified — constrained preset roles), `src/stores/teamStore.ts` (modified — selectHasAnyActiveTeam, selectTotalTeammateCount), `src/components/layout/Navigation.tsx` (modified — team indicator badge), `src/types/events.ts` (modified — 7 payload interfaces), `src/hooks/useTeamEvents.ts` (modified — uses shared types), `src/hooks/useChatEvents.ts` (modified — teammate chunk routing guard) |

**Post-team fix:** Fixed 2 `exactOptionalPropertyTypes` errors in `PaneHeader.tsx`, `TeammatePane.tsx`, `TeammatePaneGrid.tsx` (optional callback props needed `| undefined`).

---

## 2. Remaining Gaps (from Opus Gap Verification)

The Opus auditor (`Explore` agent, model: opus) performed a comprehensive verification. Full results are in the conversation but here are the 7 remaining gaps:

### Gap 1: DebateSummary not wired into PlanDisplay (SMALL FIX)
- **Brief:** Ideation v5 §5.4
- **Issue:** `DebateSummary.tsx` exists with tests but is never imported/rendered in `PlanDisplay.tsx`. Only `TeamFindingsSection` is integrated.
- **Fix:** Import DebateSummary in PlanDisplay, render it when `teamMetadata.teamMode === 'debate'`.

### Gap 2: splitPaneStore simplified (MEDIUM)
- **Brief:** Split-Pane v1 §3-5
- **Issue:** Store has only `focusedPane`, `coordinatorWidth`, `isPrefixKeyActive`, `contextKey`. Brief specifies: `isActive`, `paneOrder` array, per-pane state Record (streaming, minimized/maximized, unreadCount), `initTeam`/`addPane`/`removePane`/`clearTeam`/`focusNext`/`focusPrev`/`minimizePane`/`maximizePane`/`restorePane`/`resetPaneSizes`/`appendPaneChunk`/`clearPaneStream`/`addPaneToolCall`/`addPaneMessage`.
- **Note:** Per-pane streaming is currently handled by teamStore instead. This may be intentional simplification but diverges from brief.

### Gap 3: Keyboard nav missing commands (SMALL)
- **Brief:** Split-Pane v1 §7
- **Issue:** `useTeamKeyboardNav.ts` implements Ctrl+B prefix + arrows/vim/numbers/Escape. Missing: `z` (maximize pane), `-` (minimize), `=` (reset sizes), `x` (stop agent). Also timeout is 3s vs brief's 1.5s.
- **Depends on:** Gap 2 (minimize/maximize state in store).

### Gap 4: No responsive stacking (SMALL)
- **Brief:** Split-Pane v1 §6
- **Issue:** `TeamSplitGrid.tsx` has a 1024px media query but only adjusts coordinator min-width, doesn't stack vertically. Brief says below breakpoint, coordinator should stack above teammates.

### Gap 5: ExecutionModeSelector not in ready state (SMALL FIX)
- **Brief:** Worker v3 §8
- **Issue:** `ExecutionModeSelector` in `BasicTaskDetail.tsx` only renders for `RESTARTABLE_STATUSES` (failed, stopped, cancelled, paused). Not shown for `ready` state where users would first choose between solo and team.

### Gap 6: Multi-track progress + wave gate UI (MEDIUM)
- **Brief:** Worker v3 §2, §4
- **Issue:** `TeamProcessGroup` shows teammate name/step/model but no progress bars or percentage tracking. Wave validation gate visuals don't exist.

### Gap 7: API field name drift (TRIVIAL — documentation only)
- **Brief:** Chat UI v1 §3
- **Issue:** Backend returns `role`/`sender`/`recipient` instead of brief's `role_description`/`from`/`to`. Handled correctly in `useTeamEvents.ts` with translation logic. Just needs a note in the brief or architecture docs.

---

## 3. Planned Next Steps

### Step A: Fix Remaining 7 Gaps
Create a team to implement the 7 gaps above. Parallelize: gaps 1, 4, 5, 7 are independent small fixes. Gaps 2+3 are linked (store enrichment enables keyboard commands). Gap 6 is independent medium work.

### Step B: Testing Gap Team (CRITICAL — use Opus)
The user specifically requested a dedicated Opus-led team to:
1. **Inventory all critical team agent functionalities** across frontend + backend + MCP
2. **Audit existing test coverage** (196+ team tests from Phase 1A/1B, plus new component tests)
3. **Identify missing integration tests, edge cases, error paths**
4. **Spawn parallel workers to implement missing tests**

This is considered critical infrastructure — use Opus for the gap analysis.

### Step C: Display Mode Toggle (Phase 3)
- **Brief:** UI Decision v2 §6
- **Issue:** User preference toggle between Timeline and Split-Pane display modes with smart defaults not yet built. Lower priority.

### Step D: Project-Level Agent Config Settings
- **Brief:** Configurable Variants v5 §4.4
- **Issue:** No "Project Settings > Agent Configuration" section exists. Lower priority.

---

## 4. Key Patterns & Conventions

### Team Management
- Create team with `TeamCreate`, tasks with `TaskCreate`, set dependencies with `TaskUpdate.addBlockedBy`
- Spawn workers with `Task` tool: `subagent_type: "general-purpose"`, `mode: "bypassPermissions"`, `run_in_background: true`
- Shut down workers with `SendMessage` type `"shutdown_request"` after they complete
- Clean up with `TeamDelete`

### Delegation Rules (from user)
- **Typechecks:** Delegate to a specialized team member — do NOT run `npx tsc --noEmit` yourself. Spawn a worker to run it and fix any issues found.
- **Gap checks:** Use Opus model for all gap verification audits
- **Parallelism:** Maximize parallelism — the user explicitly wants this

### Code Conventions
- Warm orange accent `hsl(14 100% 60%)` — NO purple/blue
- macOS Tahoe glass-morphism styling
- Zustand+immer stores, TanStack Query hooks
- `exactOptionalPropertyTypes: true` — optional callback props need `| undefined` explicitly
- Keep components under 500 LOC (presentational under 200)
- Co-located tests
- Follow import order: React → third-party → internal (@/) → stores → types → components → local

---

## 5. File Inventory (All New Files Created)

```
src/components/Ideation/
├── TeamFindingsSection.tsx          (152 LOC) — Collapsible team research summary table
├── TeamFindingsSection.test.tsx     — 4 tests
├── DebateSummary.tsx                (284 LOC) — Side-by-side + stacked debate comparison
├── DebateAdvocateCard.tsx           (189 LOC) — Collapsible advocate card
├── DebateSummary.test.tsx           (189 LOC) — 15 tests
└── TeamConfigPanel.tsx              (154 LOC) — Team config dropdowns/radios

src/components/Team/
├── index.tsx                        — Barrel exports
├── TeamSplitView.tsx                — Top-level team view
├── TeamSplitHeader.tsx              — Header with back/name/cost/stop
├── TeamSplitGrid.tsx                — CSS Grid layout (coordinator + teammates)
├── CoordinatorPane.tsx              (160 LOC) — Lead pane with messages + input
├── TeamOverviewHeader.tsx           (75 LOC) — Compact stats bar
├── TeammatePaneGrid.tsx             (61 LOC) — Grid of teammate panes
├── TeammatePane.tsx                 (97 LOC) — Per-teammate view
├── PaneHeader.tsx                   (113 LOC) — Teammate pane header
├── PaneStream.tsx                   (84 LOC) — Streaming text display
├── PaneInput.tsx                    (93 LOC) — Compact chat input
└── PrefixKeyOverlay.tsx             (40 LOC) — Keyboard indicator toast

src/components/execution/
└── TeamProcessGroup.tsx             (~160 LOC) — Collapsible team in running processes

src/hooks/
├── usePaneEvents.ts                 (85 LOC) — Per-pane event subscription
├── useTeamKeyboardNav.ts            (120 LOC) — Ctrl+B prefix navigation
├── useTeamViewLifecycle.ts          (70 LOC) — Auto-switch to/from team view
└── usePaneResize.ts                 (85 LOC) — Drag-to-resize coordinator column

src/stores/
└── splitPaneStore.ts                — Split-pane layout state
```

### Modified Files
```
src/types/chat.ts                    — Added "team" to VIEW_TYPE_VALUES
src/types/ideation.ts                — Added TeamMode, CompositionMode, TeamConfig + Zod schemas
src/types/events.ts                  — Added 7 team event payload interfaces
src/stores/uiStore.ts                — Added previousView + setPreviousView
src/stores/teamStore.ts              — Added selectHasAnyActiveTeam, selectTotalTeammateCount
src/api/ideation.ts                  — Extended createSession with teamMode + teamConfig
src/api/ideation.schemas.ts          — Added team response schemas
src/api/team.ts                      — 3-param sendTeamMessage with target
src/api/tasks.ts                     — agentVariant param on move()
src/api/running-processes.types.ts   — teamName + teammates fields
src/hooks/useIdeation.ts             — teamMode + teamConfig in mutation
src/hooks/useIdeation.test.ts        — Fixed assertions for 5-param create
src/hooks/useTeamActions.ts          — target param on sendTeamMessage
src/hooks/useChat.ts                 — target param passthrough
src/hooks/useChatActions.ts          — target param passthrough
src/hooks/useTeamEvents.ts           — Uses shared payload types
src/hooks/useChatEvents.ts           — Teammate chunk routing guard
src/components/Ideation/StartSessionPanel.tsx  — Team mode selector + config panel
src/components/Ideation/PlanDisplay.tsx        — TeamMetadata prop + team badge + findings
src/components/Ideation/TeamConfigPanel.tsx     — Constrained preset roles display
src/components/layout/Navigation.tsx            — Team indicator badge
src/components/tasks/detail-views/BasicTaskDetail.tsx — ExecutionModeSelector
src/components/execution/RunningProcessPopover.tsx    — TeamProcessGroup rendering
src/App.tsx                          — Team view case
```

---

## 6. Reference Documents

| Document | Path | Purpose |
|----------|------|---------|
| Ideation integration brief | `docs/product-briefs/agent-teams-ideation-integration.md` | v5, all questions resolved |
| Worker integration brief | `docs/product-briefs/agent-teams-worker-integration.md` | v3 |
| Configurable variants brief | `docs/product-briefs/configurable-agent-variants.md` | v5 |
| Chat UI extension brief | `docs/product-briefs/agent-teams-chat-ui-extension.md` | v1 |
| Split-pane UI brief | `docs/product-briefs/agent-teams-split-pane-ui.md` | v1 |
| UI decision brief | `docs/product-briefs/agent-teams-ui-decision.md` | v2, phased hybrid |
| System card | `docs/agent-teams-system-card.md` | 1,237 lines, comprehensive reference |

---

## 7. Quick Resume Command

To continue where we left off:
```
Continue the agent teams UI gap implementation. Read docs/handover/agent-teams-ui-gaps-handover.md for full context.

Next steps:
1. Fix the 7 remaining gaps (Section 2)
2. Launch Opus testing gap team (Section 3, Step B)
```
