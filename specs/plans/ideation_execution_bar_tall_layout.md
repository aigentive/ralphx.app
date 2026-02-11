# Add Execution Control Bar to Ideation Left Pane (Tall-Page Layout)

## Summary
Bring Ideation in line with Graph/Kanban by adding the `ExecutionControlBar` as a footer under the Ideation left pane, not under chat.

The bar spans from the far left edge through the Plan Browser + middle proposals area, stopping at the split divider before chat. To accommodate this, left-pane content height is reduced (not overlaid), with internal scrolling preserved in sidebar and middle content.

## Locked decisions
- Execution bar visibility in Ideation: always visible when Ideation view is open.
- Bar width scope: sidebar + middle only (exclude right chat panel).
- No-session behavior: Start Session panel remains above the bar (bar still visible).

## Goals
- Match Graph/Kanban execution control placement and behavior consistency.
- Keep current Ideation split-screen interaction model and chat resizing.
- Preserve internal scroll behavior on tall-content pages.

## Non-goals
- No backend/API behavior changes for execution controls.
- No new ideation-specific execution settings.
- No change to Graph/Kanban layout behavior.

## Current state and gap
- `KanbanSplitLayout` and `GraphSplitLayout` expose a left-section `footer` slot where `ExecutionControlBar` is mounted.
- `PlanningView` (`IdeationView`) currently renders sidebar, middle panel, and chat inside one main row, with no dedicated left-footer region.
- Because there is no left footer slot in Ideation, execution controls cannot be placed with the same width/scope as Graph/Kanban.

## Proposed architecture

### Layout model for Ideation
Refactor Ideation into a split model that mirrors the semantic structure of Graph/Kanban while preserving existing visuals:

- Outer container (`ideation-view`): row with left section + resize handle + right chat panel.
- Left section: column layout.
  - Top area (`flex-1 min-h-0`): existing Ideation content row (`PlanBrowser` + main content).
  - Bottom area (`flex-shrink-0`): execution footer region.
- Right section: existing `IntegratedChatPanel` at resizable width.

This creates a deterministic footer area under sidebar+middle only, ending at the split divider before chat.

### Execution bar injection strategy
Use the same injection pattern as other views:

- `App.tsx` builds `<ExecutionControlBar ... />` and passes it into `IdeationView` as `footer`.
- `PlanningView` accepts `footer?: React.ReactNode` and renders it only in the left section footer slot.

## Detailed implementation plan

### 1) Extend Ideation view interface to accept footer
**File:** `src/components/Ideation/PlanningView.tsx`

Changes:
- Add optional prop to `PlanningViewProps`:
  - `footer?: React.ReactNode`
- Thread prop through component signature and rendering.

Rationale:
- Matches existing layout abstraction used by `KanbanSplitLayout`/`GraphSplitLayout`.
- Keeps `PlanningView` reusable and testable without hard-coupling to execution bar internals.

### 2) Restructure Ideation JSX to support a left footer
**File:** `src/components/Ideation/PlanningView.tsx`

Current high-level shape:
- `PlanBrowser` + main content + chat are siblings in one row.

Target shape:
- Left section wrapper (`flex-1 flex flex-col overflow-hidden min-w-0`).
  - Left content row (`flex-1 min-h-0 flex overflow-hidden`):
    - `PlanBrowser`
    - session-based main content (`StartSessionPanel` or session header/content)
  - Footer region (`flex-shrink-0`) renders `{footer}` when provided.
- Existing resize handle.
- Existing right chat section at `chatPanelWidth`.

Critical constraints:
- Maintain `min-h-0` on containers that host scrollable children.
- Keep proposals list container scrollable (`overflow-y-auto`) and independent from page scroll.
- Preserve `PlanBrowser` internal scrolling (`overflow-y-auto`).

### 3) Inject `ExecutionControlBar` into Ideation from App
**File:** `src/App.tsx`

In `currentView === "ideation"` block:
- Pass `footer={...}` to `IdeationView` using the same execution data/actions used by Graph/Kanban:
  - `projectId`
  - `runningCount`, `maxConcurrent`, `queuedCount`
  - `mergingCount`, `hasAttentionMerges`, `mergePipelineData`
  - `isPaused`, `isLoading`
  - `onPauseToggle`, `onStop`
  - `runningProcesses`, `onPauseProcess`, `onStopProcess`
  - `onOpenSettings`
- Keep battle mode toggle disabled for Ideation (same as Kanban behavior).

Rationale:
- Avoids duplicating execution-state logic.
- Preserves global project-scoped execution semantics.

### 4) Update Ideation component tests
**File:** `src/components/Ideation/PlanningView.test.tsx`

Add/adjust tests:
- Renders footer when `footer` prop is provided.
- Footer remains visible in no-session state (`session=null`).
- Existing core layout assertions still pass:
  - `ideation-view`
  - `plan-browser`
  - `conversation-panel`
  - active session header/proposals as applicable.

Implementation note:
- Pass a simple `<div data-testid="ideation-footer" />` as footer in tests.

### 5) Update App-level tests with Ideation footer wiring
**File:** `src/App.test.tsx`

Because Ideation is mocked in this test file:
- Update Ideation mock to expose whether `footer` was passed (e.g., render `data-testid="ideation-footer-mock"` when prop exists).
- Add assertion when switching to ideation that footer is present.

Rationale:
- Verifies wiring contract at App integration boundary.

### 6) Update visual regression baseline
**File:** `tests/visual/views/ideation/ideation.spec.ts`

- Keep flow unchanged; existing snapshot test captures layout diff.
- Regenerate baseline image because execution bar will now appear at bottom of left section.

## Public interfaces and type changes
- `PlanningViewProps` (`src/components/Ideation/PlanningView.tsx`)
  - New optional prop: `footer?: React.ReactNode`

No other public API/type/schema changes.

## Data flow and behavior
- Execution data source remains unchanged (`useUiStore` + existing App hooks/state).
- Ideation simply receives and renders footer node.
- Execution actions dispatched from Ideation bar call the same handlers as Kanban/Graph.

## Edge cases and failure modes
- **No active session:** StartSession panel remains functional above footer; footer still visible.
- **Very tall content in sidebar/proposals:** Scrolling remains internal and independent.
- **Narrow widths / aggressive resizing:** Chat width constraints remain enforced; footer does not overlap content.
- **Read-only sessions (accepted/archived):** Footer remains visible because controls are project-global.

## Accessibility and UX notes
- `ExecutionControlBar` keeps existing aria attributes/labels.
- Footer placement avoids overlaying content, preserving predictable keyboard/tab order and scroll behavior.
- Visual consistency with Graph/Kanban improves discoverability of execution controls.

## Acceptance criteria
1. In Ideation view, execution bar is visible at the bottom of the left section.
2. Bar starts at left edge and stops at split divider before chat.
3. Sidebar and middle content height is reduced to make room for bar.
4. Sidebar and proposals content stay internally scrollable.
5. No-session state still shows bar below Start Session panel.
6. Existing chat resize behavior in Ideation remains functional.
7. Unit tests and app integration tests cover footer rendering/wiring.
8. Visual snapshot updated for Ideation layout.

## Test matrix

### Component tests
- `PlanningView` with active session + footer:
  - footer visible
  - proposals panel and chat panel visible
- `PlanningView` with no session + footer:
  - start-session panel visible
  - footer visible
- `PlanningView` without footer:
  - backward-compatible rendering

### App integration tests
- Switch to ideation view:
  - ideation renders
  - footer prop is passed/rendered by mock

### Visual regression
- Ideation snapshot includes bottom execution bar on left section.
- No unintended overlap/clipping in left content area.

## Rollout and compatibility
- UI-only change; no data migration.
- Backward compatible: footer prop is optional.
- Safe incremental rollout through existing test suites plus snapshot update.

## Implementation checklist
1. Add `footer` prop to `PlanningViewProps`.
2. Refactor `PlanningView` layout into left-section-with-footer + right chat.
3. Pass `ExecutionControlBar` as `footer` from `App.tsx` for ideation route.
4. Update `PlanningView.test.tsx` for footer behavior.
5. Update `App.test.tsx` ideation mock/assertions for footer wiring.
6. Update ideation visual snapshot baseline.
7. Run targeted tests:
   - `PlanningView` tests
   - `App` tests
   - visual ideation test (or snapshot update workflow)

## Assumptions
- Ideation should share execution-control semantics with Graph/Kanban.
- Execution controls remain project-scoped, not session-scoped.
- Existing footer visual style from `ExecutionControlBar` is acceptable in Ideation without variant styling.
