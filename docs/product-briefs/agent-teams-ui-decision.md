# Agent Teams UI Decision: Split-Pane vs Chat Timeline

**Status:** FINAL RECOMMENDATION (v2 — revised with full briefs)
**Author:** product-analyst (agent-teams-ui-decision team)
**Date:** 2026-02-15
**Inputs:**
- `docs/product-briefs/agent-teams-chat-ui-extension.md` (Approach A — 1561 lines, full brief)
- `docs/product-briefs/agent-teams-split-pane-ui.md` (Approach B — 983 lines, full brief)
- `docs/agent-teams-system-card.md` (agent teams system reference)
- `docs/architecture/agent-chat-system.md` (current chat system)
- `specs/DESIGN.md` (design system)
- Existing RalphX UI codebase analysis (`App.tsx`, `KanbanSplitLayout.tsx`, `ChatPanel.tsx`, `IntegratedChatPanel.tsx`)

---

## 1. Executive Summary

This document compares two competing UI approaches for displaying agent teams in RalphX, scores them across 10 criteria with implementation-level detail from both product briefs, evaluates a hybrid approach, and delivers a final recommendation.

**TL;DR: Recommend Approach C (Hybrid) — chat timeline as default, with dedicated team view as opt-in power mode.** Neither pure approach alone optimally serves all users and scenarios. Approach A wins on weighted score (3.90 vs 3.53) due to lower risk, better scalability, and stronger consistency with existing UI — but Approach B's simultaneous visibility and direct interaction model make it the superior power-user experience for small teams (2-4 agents).

---

## 2. Approach Descriptions

### Approach A: Chat Timeline Extension

Extends the existing `ChatPanel` / `IntegratedChatPanel` with team-aware features. Defined in `agent-teams-chat-ui-extension.md` (9 extension areas, 1561 lines).

| Feature | Implementation |
|---------|---------------|
| **Layout** | Same right-side panel (resizable, 280-600px) used for solo chat |
| **Message display** | Combined chronological timeline with color-coded teammate messages |
| **Filtering** | Tab bar: `[All] [Lead] [coder-1] [coder-2] ...` for per-agent filtering |
| **User messaging** | "Send to" dropdown (`TargetSelector`) in chat input selects target agent |
| **Team status** | `TeamActivityPanel` sidebar showing teammate cards with status, cost, actions |
| **Event routing** | Existing `agent:*` events extended with `teammate_name` + `team_name` fields (backward-compatible) |
| **State management** | New `teamStore.ts` alongside existing `chatStore.ts` (minimal `isTeamActive` addition) |
| **Navigation** | No navigation change — team mode activates within existing chat panel |
| **New files** | 12 frontend (7 components + 3 hooks + 1 store + 1 API) + 2+ backend |
| **Modified files** | 9 frontend + 4+ backend |

**Key design decision:** Team is a _property_ of the existing chat context, not a new view. Context types unchanged — `teamMode` flag on existing contexts.

### Approach B: Split-Pane (tmux-inspired)

Full-screen dedicated view with independent panels per agent. Defined in `agent-teams-split-pane-ui.md` (16 sections, 983 lines).

| Feature | Implementation |
|---------|---------------|
| **Layout** | Full-screen CSS Grid: `grid-template-columns: var(--coordinator-width, 40%) 1fr`. Coordinator pane LEFT (40%), teammates stacked RIGHT (60%) |
| **Panel lifecycle** | Auto-open on `team:teammate_spawned`, auto-close on `team:teammate_shutdown`. View auto-switches on `team:created` |
| **Interaction** | Click-to-focus model. Focused pane gets highlighted border (agent color) + active input |
| **Panel content** | Each pane = mini ChatPanel with streaming text, tool calls, inter-agent messages, status badge |
| **Coordinator** | Always-visible left pane: team overview header (stats, cost, stop/disband) + lead chat stream |
| **Navigation** | `Ctrl+B` prefix key (tmux-inspired, 1.5s timeout) + arrow keys, number keys, `z`=maximize, `-`=minimize |
| **Responsive** | 4 breakpoints: ≥1440px full grid, 1200-1439px max 2 visible, 768-1199px tabbed fallback, <768px single pane |
| **State management** | New `splitPaneStore.ts` (layout + per-pane streaming) + shared `teamStore.ts` |
| **Max visible** | 6 panes (1 coordinator + 5 teammates). Overflow minimized or scrollable |
| **New files** | 16 frontend (11 components + 1 store + 4 hooks) + 0 new backend (shares A's backend) |
| **Modified files** | 3 frontend (`uiStore.ts`, `App.tsx`, `Navigation.tsx`) |
| **Reused** | `ChatMessageList`, `ChatInput`, `StatusActivityBadge`, `ResizeHandle`, `teamStore.ts`, `team.ts` API |

**Key design decision:** Team mode is a _dedicated experience_ — new `"team"` view type in `uiStore`, separate from normal chat. Auto-switches when team starts, returns to previous view on disband/back.

### Shared Infrastructure (Both Approaches Require)

Both briefs depend on identical backend infrastructure (from the chat-timeline brief Sections 3, 11, 14):

| Component | Purpose |
|-----------|---------|
| `team:*` events (7 new event types) | Team lifecycle + inter-agent messages |
| `teammate_name` field on `agent:*` events | Per-teammate event routing |
| `teamStore.ts` | Team state (teammates, messages, costs) |
| `src/api/team.ts` | Tauri invoke wrappers (6 new commands) |
| `TeamStateTracker` (Rust) | Backend team state management |
| `StreamProcessorConfig` extensions | Per-teammate stream tagging |

---

## 3. Scoring Matrix

Scoring: **1** (poor) to **5** (excellent). Half-points allowed.

### 3.1 User Experience

_Which is more intuitive for monitoring and interacting with multiple agents?_

| Factor | Approach A (Timeline) | Approach B (Split-Pane) |
|--------|----------------------|------------------------|
| Learning curve | Low — extends familiar chat UI | Medium — new paradigm, but auto-switches on team start |
| Mental model | "Chat room with multiple agents" | "Individual workstations I can see at a glance" |
| Context switching | Tabs to filter, but one stream at a time | All streams visible simultaneously (up to 6) |
| Overwhelm risk | Lower (filtered) but may feel noisy with 5+ agents | Higher — 5 panes visible at once, mitigated by minimize/maximize |
| Interaction clarity | Dropdown to select target = indirect | Click-to-focus + type = direct, natural |
| Panel actions | Stop/message via TeammateCard buttons | Per-pane header buttons: minimize, maximize, close, stop |

| Verdict | Score |
|---------|-------|
| **Approach A** | **3.5** — Familiar but indirect interaction model |
| **Approach B** | **4.0** — More immersive with auto-switching; minimize/maximize handles overwhelm |

### 3.2 Information Density

_Which shows more useful information at a glance?_

| Factor | Approach A (Timeline) | Approach B (Split-Pane) |
|--------|----------------------|------------------------|
| Simultaneous visibility | 1 stream (filtered) or all interleaved | Up to 6 streams visible simultaneously |
| Status overview | TeamActivityPanel sidebar shows all statuses | Per-panel badges + coordinator team overview header |
| Tool call visibility | In timeline (may be buried) | Per-panel, always contextual to the agent |
| Inter-agent messages | Inline in timeline (visible chronologically) | Shown in both sender and receiver panes + coordinator summary |
| Cost tracking | Aggregate in TeamActivityPanel | Per-pane token count + coordinator aggregate |
| Streaming indicator | One indicator per context | Per-pane streaming indicator (isolated state) |

| Verdict | Score |
|---------|-------|
| **Approach A** | **3.0** — Compact but filtered; loses simultaneous view |
| **Approach B** | **4.5** — Shows everything at once; spatial layout aids comprehension |

### 3.3 Interaction Model

_Which makes it easier to message specific teammates? To see all activity?_

| Factor | Approach A (Timeline) | Approach B (Split-Pane) |
|--------|----------------------|------------------------|
| Message a specific agent | Select from dropdown → type → send (3 steps) | Click panel → type → send (2 steps) |
| Message the lead | Default target; no dropdown change | Click left panel → type (or Ctrl+B → 1) |
| Broadcast to all | "All" in dropdown | Requires explicit broadcast action |
| See all activity | "All" tab (chronological, interleaved) | Scan all panels (spatial, parallel) |
| See one agent's activity | Click agent tab (clean filtered view) | Focus panel (but others still visible — can maximize with Ctrl+B z) |
| Keyboard navigation | Standard (no special keyboard shortcuts) | Ctrl+B prefix key system: arrows, numbers, z/-, x |

| Verdict | Score |
|---------|-------|
| **Approach A** | **3.5** — Dropdown is clear but adds a step |
| **Approach B** | **4.0** — Click-to-focus is faster; Ctrl+B navigation is powerful for keyboard users |

### 3.4 Implementation Complexity

_Which is harder to build? More risk?_

| Factor | Approach A (Timeline) | Approach B (Split-Pane) |
|--------|----------------------|------------------------|
| New frontend files | 12 (7 components + 3 hooks + 1 store + 1 API) | 16 (11 components + 4 hooks + 1 store) |
| Modified frontend files | 9 (ChatPanel, IntegratedChatPanel, ChatInput, MessageItem, ExecutionTaskDetail, 4 hooks) | 3 (`uiStore.ts`, `App.tsx`, `Navigation.tsx`) |
| New backend files | 2+ (TeamStateTracker, team_commands) | 0 (shares A's backend entirely) |
| Modified backend files | 4+ (streaming, helpers, types, mod) | 0 (identical backend) |
| State management | `teamStore.ts` + minor `chatStore` addition | `splitPaneStore.ts` (complex: layout + focus + per-pane streaming + keyboard state) |
| Existing code reuse | High — extends ChatPanel, IntegratedChatPanel, ChatInput, existing hooks | Medium — reuses ChatMessageList, ChatInput, ResizeHandle, StatusActivityBadge |
| New paradigms | None — all extensions of existing patterns | CSS Grid multi-pane layout, focus system, prefix key navigation, auto-view-switching |
| Risk | Low — additive changes to proven components | Medium — new view type, new layout system, new interaction model |
| Phase count | 4 phases in brief | 4 phases in brief |

| Verdict | Score |
|---------|-------|
| **Approach A** | **4.5** — Extends existing patterns; most changes are additive with low risk |
| **Approach B** | **3.0** — Well-architected (clear component hierarchy, good store separation), but 16 new files + new paradigms = more work and risk |

### 3.5 Performance

_N concurrent streams in one view vs N panels._

| Factor | Approach A (Timeline) | Approach B (Split-Pane) |
|--------|----------------------|------------------------|
| DOM elements | 1 timeline list (react-virtuoso candidate) | Up to 6 panes, each with own scrollable stream |
| Streaming load | All chunks → teamStore → 1 render path | N stores → N render paths (isolated state slices in splitPaneStore) |
| Re-renders | Teammate chunk → teamStore update → potentially timeline re-render | Teammate chunk → only that pane's state slice updates (React isolation) |
| Memory | 1 message list + per-teammate buffer | Per-pane: streaming text (cleared on completion) + last 100 tool calls + 50 messages |
| Event processing | `useTeamEvents` hook routes all events via teammate_name filter | `usePaneEvents` per-pane: filters at subscription level — no global re-renders |
| Resize | N/A (single panel resize) | CSS Grid reflow (GPU-accelerated) + `requestAnimationFrame` for smooth updates |

| Verdict | Score |
|---------|-------|
| **Approach A** | **3.5** — Single render path, but all-in-one can cause jank with many agents |
| **Approach B** | **4.0** — Naturally isolated per pane; per-pane event filtering prevents cascading re-renders |

### 3.6 Consistency with Existing UI

_Which fits better with RalphX's existing patterns?_

| Factor | Approach A (Timeline) | Approach B (Split-Pane) |
|--------|----------------------|------------------------|
| Chat pattern | Matches existing ChatPanel/IntegratedChatPanel exactly | Reuses ChatMessageList + ChatInput inside panes, but pane container is new |
| Layout pattern | KanbanSplitLayout: left content + right chat (matches) | New full-screen multi-pane view; however, uses existing ResizeHandle pattern |
| View switching | No new view type — team activates within existing panel | New `"team"` view type, consistent with existing view switching pattern (kanban, graph, ideation) |
| Panel interaction | Resize handle + toggle (existing) | Click-to-focus + Ctrl+B keyboard nav (new paradigm) |
| Design system | Uses existing card, badge, message components | Uses existing cards + badges; needs new pane chrome + focus ring styling |
| Auto-behavior | Team features appear progressively in existing panel | Auto-switches to team view on team:created — established pattern in RalphX |

| Verdict | Score |
|---------|-------|
| **Approach A** | **4.5** — Natural extension of established patterns; zero new paradigms |
| **Approach B** | **3.0** — Uses existing view switching pattern and reuses key components (ChatMessageList, ChatInput, ResizeHandle, StatusActivityBadge), but introduces new interaction paradigm (focus model, prefix keys) |

### 3.7 Scalability

_What happens with 2 teammates vs 5 vs 10?_

| Team Size | Approach A (Timeline) | Approach B (Split-Pane) |
|-----------|----------------------|------------------------|
| **2 teammates** | Clean: 4 filter tabs, manageable timeline | Perfect: coordinator left + 2 teammates right, generous height |
| **3 teammates** | Good: 5 filter tabs, moderate timeline noise | Good: coordinator left + 3 teammates right (33% each) |
| **5 teammates** | Tab bar gets crowded, timeline busy; filter makes it manageable | Max visible: all 6 panes (1+5). Equal split with scroll for content. Brief says this is the designed limit |
| **6+ teammates** | Tab bar overflow needed, timeline noisy but filter handles it | Brief caps visible at 6 panes. Overflow teammates are minimized to 36px header bar |
| **10 teammates** | Heavy tab bar, fire-hose timeline, but filter tabs + TeamActivityPanel keep it usable | Must minimize most panes — effectively degrades to maximized view of 1-2 panes at a time |

**Key detail from split-pane brief (Section 2.4, Q4):** Max 6 visible panes by design. 4+ teammates get equal split with scrollable overflow (max 4 visible, scroll for more). Minimized teammates collapse to 36px header bars. This is a deliberate design constraint, not a failure.

| Verdict | Score |
|---------|-------|
| **Approach A** | **4.0** — Graceful degradation via filtering; works at any team size |
| **Approach B** | **3.0** — Designed for up to 6 panes with explicit overflow strategy; degrades to minimize/maximize past 5 teammates, which sacrifices the "all visible" value prop |

### 3.8 Responsive / Narrow Screens

_Which degrades better on smaller screens?_

| Screen Width | Approach A (Timeline) | Approach B (Split-Pane) |
|-------------|----------------------|------------------------|
| **≥1440px** | Full experience; chat panel 360px + content | Full split-pane grid: all panes visible simultaneously |
| **1200-1439px** | Chat panel works fine; slightly narrower | Max 2 teammate panes visible; scrollable overflow for rest |
| **768-1199px** | Chat panel can collapse to icon or overlay | **Tabbed fallback**: coordinator tab + one teammate tab; tab bar with color dots |
| **<768px (tablet)** | Chat overlay mode (existing pattern) | **Single pane + selector dropdown**: one pane fills screen at a time |

**Key detail from split-pane brief (Section 7):** Full 4-breakpoint responsive strategy with tabbed and single-pane fallbacks. The brief explicitly addresses narrow screens rather than ignoring them. However, the tabbed fallback at <1200px effectively becomes a version of Approach A (one stream at a time), losing the core "simultaneous visibility" differentiator.

| Verdict | Score |
|---------|-------|
| **Approach A** | **4.5** — Existing responsive patterns handle it natively |
| **Approach B** | **3.0** — Has a complete responsive strategy (not absent as initially assumed), but fallback modes lose the primary value prop. Below 1200px, you're essentially viewing one pane at a time. |

### 3.9 Discoverability

_Which is easier for new users to understand?_

| Factor | Approach A (Timeline) | Approach B (Split-Pane) |
|--------|----------------------|------------------------|
| Entry point | Team features appear progressively in existing chat panel | Auto-switches to team view on `team:created` — user doesn't "discover" it, it just appears |
| Familiarity | "Chat with agents" is universally understood | "Individual workstations" is less familiar, but layout is self-explanatory |
| Affordances | Dropdown, tabs, badges are standard UI | Focus borders, keyboard nav need some learning; but click-to-type is intuitive |
| Onboarding | Minimal — same chat panel, new tabs and sidebar appear | Auto-switch handles entry; prefix key indicator appears when Ctrl+B pressed |
| Return path | Always in context (no view switching) | "← Back" button in team header; team-active indicator in nav when on other views |

| Verdict | Score |
|---------|-------|
| **Approach A** | **4.5** — Familiar patterns; progressive disclosure within existing UI |
| **Approach B** | **3.0** — Auto-view-switching removes the "discovery" problem; click-to-focus is intuitive; but Ctrl+B keyboard nav requires learning. Team-active indicator in header is helpful for context. |

### 3.10 Flexibility / Hybrid Potential

_Can we combine elements of both? Offer both as display modes?_

| Factor | Assessment |
|--------|-----------|
| **Shared backend** | Identical — both use same `team:*` events, `TeamStateTracker`, API layer, stream processors |
| **Shared store** | `teamStore.ts` is shared; split-pane adds `splitPaneStore.ts` for layout only |
| **Shared components** | TeammateCard, TeamCostDisplay, TargetSelector reusable. Split-pane reuses ChatMessageList, ChatInput |
| **Mode switching** | User preference toggle: "Timeline" vs "Split View" |
| **Contextual default** | ≤4 agents → split-pane default (sweet spot); 5+ → timeline default (scalability) |
| **Implementation cost** | Building both = ~1.4x effort (substantial shared infrastructure: backend, teamStore, event system, API) |
| **User confusion** | Two modes = more to learn, but user can pick preferred style |

| Verdict | Score |
|---------|-------|
| **Approach A** | **3.5** — Good standalone; limited by single-stream view |
| **Approach B** | **3.5** — Good for power users; limited by scalability constraints |
| **Hybrid** | **5.0** — Best of both worlds with user choice; ~70% shared infrastructure |

---

## 4. Scoring Summary

| # | Criterion | Weight | A (Timeline) | B (Split-Pane) | A Weighted | B Weighted |
|---|-----------|--------|-------------|---------------|-----------|-----------|
| 1 | User Experience | 15% | 3.5 | 4.0 | 0.525 | 0.600 |
| 2 | Information Density | 10% | 3.0 | 4.5 | 0.300 | 0.450 |
| 3 | Interaction Model | 10% | 3.5 | 4.0 | 0.350 | 0.400 |
| 4 | Implementation Complexity | 15% | 4.5 | 3.0 | 0.675 | 0.450 |
| 5 | Performance | 10% | 3.5 | 4.0 | 0.350 | 0.400 |
| 6 | Consistency | 15% | 4.5 | 3.0 | 0.675 | 0.450 |
| 7 | Scalability | 10% | 4.0 | 3.0 | 0.400 | 0.300 |
| 8 | Responsive | 5% | 4.5 | 3.0 | 0.225 | 0.150 |
| 9 | Discoverability | 5% | 4.5 | 3.0 | 0.225 | 0.150 |
| 10 | Flexibility | 5% | 3.5 | 3.5 | 0.175 | 0.175 |
| | **TOTAL** | **100%** | | | **3.90** | **3.53** |

**Approach A wins on weighted score: 3.90 vs 3.53.**

Approach A dominates on: implementation complexity (+1.5), consistency (+1.5), scalability (+1.0), responsive (+1.5), discoverability (+1.5).
Approach B dominates on: information density (+1.5), UX (+0.5), interaction model (+0.5), performance (+0.5).

**v2 note:** Approach B improved from 3.25 (v1, based on task description) to 3.53 (v2, based on full brief). The split-pane brief's responsive strategy, explicit scalability limits, component reuse, and well-structured architecture reduced the gap. The recommendation remains unchanged.

---

## 5. Edge Case Analysis

### 5.1 Two Teammates (Ideation Debate)

| Approach | Experience |
|----------|-----------|
| **A (Timeline)** | 4 filter tabs (All, Lead, Debater-1, Debater-2). Clean timeline. Works well. |
| **B (Split-Pane)** | Ideal: coordinator left = lead; 2 teammate panes right with generous 50/50 split. Maximum spatial awareness. |
| **Winner** | **B** — split-pane at its absolute best with exactly 2-3 agents |

### 5.2 Five Teammates (Worker Execution)

| Approach | Experience |
|----------|-----------|
| **A (Timeline)** | 7 filter tabs (All + Lead + 5 coders). Timeline busy but filterable. TeamActivityPanel shows all statuses at a glance. |
| **B (Split-Pane)** | 6 panes total (1+5). Brief says max 4 visible teammate panes with equal split; 5th scrollable. Each pane ~150px on 1440px screen. Tight but designed for this. Can minimize inactive to 36px headers. |
| **Winner** | **A** — timeline handles this gracefully with filtering; split-pane is at its designed limit |

### 5.3 Eight+ Teammates (Complex Cross-Layer)

| Approach | Experience |
|----------|-----------|
| **A (Timeline)** | Tab bar needs horizontal scroll. Timeline is fire-hose. Filter is essential. Manageable with TeamActivityPanel for overview. |
| **B (Split-Pane)** | Exceeds designed cap of 6. Most panes minimized to 36px header bars. User maximizes 1-2 at a time — effectively a worse version of tabbed view. |
| **Winner** | **A** — still fully functional; B degrades past its design intent |

### 5.4 Narrow Screen (1280px MacBook Air)

| Approach | Experience |
|----------|-----------|
| **A (Timeline)** | Chat panel squeezes to ~280px min. Kanban shrinks. Still functional. |
| **B (Split-Pane)** | Brief specifies 1200-1439px breakpoint: max 2 teammate panes visible, scrollable overflow. Coordinator min 300px. Reduced but usable. |
| **Winner** | **A** — naturally handles any width; B loses simultaneous visibility below 1440px |

### 5.5 Very Narrow (768-1199px)

| Approach | Experience |
|----------|-----------|
| **A (Timeline)** | Chat panel works normally; can collapse to icon. |
| **B (Split-Pane)** | **Tabbed fallback**: tab bar with agent colors, one pane visible at a time. Essentially becomes Approach A without the combined timeline. |
| **Winner** | **A** — B's tabbed fallback is functional but loses its differentiator |

### 5.6 New User First Encounter

| Approach | Experience |
|----------|-----------|
| **A (Timeline)** | "Oh, there are colored tabs now and messages from different agents. I can filter by agent." Familiar, progressive. |
| **B (Split-Pane)** | Auto-switch on `team:created` means the view appears without user action. Initial "whoa" moment, but layout is self-explanatory: lead on left, coders on right. Prefix key indicator appears contextually. |
| **Winner** | **A** — progressive disclosure is safer for adoption, though B's auto-switch handles the "where do I go?" problem |

### 5.7 Power User Monitoring Multiple Agents

| Approach | Experience |
|----------|-----------|
| **A (Timeline)** | Must constantly switch filter tabs to see what each agent is doing. Can't watch all at once. |
| **B (Split-Pane)** | All agents visible simultaneously. Glance at any panel. See tool calls in real-time per agent. Ctrl+B keyboard nav for rapid focus switching. |
| **Winner** | **B** — this is its strongest and most compelling use case |

---

## 6. Hybrid Approach Analysis

### 6.1 Proposal: Display Mode Toggle

Offer **both** approaches as user-selectable display modes, with a smart default:

```
┌─────────────────────────────────────────────┐
│  Team View: [Timeline ▾] [Split Panes ▾]    │
└─────────────────────────────────────────────┘
```

### 6.2 Implementation Strategy

| Phase | Deliverable | Effort | Value |
|-------|------------|--------|-------|
| **Phase 1** | Chat timeline extension (Approach A) — full implementation | 4-6 weeks | Full team support, works for all scenarios, all screen sizes |
| **Phase 2** | Split-pane view (Approach B) — leveraging shared infrastructure | 3-5 weeks | Power user mode for monitoring 2-5 agents simultaneously |
| **Phase 3** | Display mode toggle + smart defaults | 1 week | User choice |

### 6.3 Smart Defaults

| Context | Default Mode | Reasoning |
|---------|-------------|-----------|
| Ideation (debate, 2 agents) | Split-Pane | Perfect 2-pane layout for debate |
| Ideation (research, 3-5 agents) | Timeline | Multiple researchers = busy panes |
| Execution (2-3 coders) | Split-Pane | Natural "watch them work" experience |
| Execution (4+ coders) | Timeline | Exceeds split-pane sweet spot |
| Any team > 5 teammates | Timeline | Beyond split-pane's designed cap |
| ≤1200px viewport | Timeline (forced) | Split-pane loses value prop below this breakpoint |
| User preference set | User preference | Respect explicit choice |

### 6.4 Shared Infrastructure (Both Modes Use)

| Component | Purpose | Source Brief |
|-----------|---------|-------------|
| `teamStore.ts` | Team state: teammates, messages, costs | Both (shared) |
| `useTeamEvents.ts` | Event routing for team lifecycle (7 event types) | Approach A |
| `src/api/team.ts` | Tauri invoke wrappers (6 new commands) | Approach A |
| `TeammateCard.tsx` | Status display (used in both) | Approach A |
| `TeamCostDisplay.tsx` | Cost tracking | Approach A |
| `TargetSelector.tsx` | "Send to" dropdown (timeline mode) | Approach A |
| Backend: `TeamStateTracker` | Team state tracking service (Rust) | Approach A |
| Backend: team IPC commands | 6 new Tauri commands | Approach A |
| Backend: event extensions | `teammate_name` + `team_name` fields on `agent:*` events | Approach A |
| Backend: `StreamProcessorConfig` | Per-teammate stream tagging | Approach A |
| `ChatMessageList` | Reused in coordinator pane | Existing (no modification) |
| `ChatInput` | Reused in coordinator pane + pane inputs | Existing (no modification) |
| `ResizeHandle` | Column + row resize in split-pane | Existing (no modification) |

**~70% of infrastructure is shared.** Building both modes costs ~1.4x the effort of building one, not 2x. The split-pane brief explicitly notes in Section 17 that it depends on the chat-timeline brief's infrastructure (event system, teamStore, API, backend services).

### 6.5 Hybrid Scoring

| # | Criterion | Hybrid Score | Rationale |
|---|-----------|-------------|-----------|
| 1 | User Experience | 4.5 | Best of both: familiar default + immersive option |
| 2 | Information Density | 4.5 | Split-pane for monitoring; timeline for overview |
| 3 | Interaction Model | 4.0 | User picks preferred interaction style |
| 4 | Implementation Complexity | 3.0 | More total work (~1.4x), but phased and risk-managed |
| 5 | Performance | 4.0 | Each mode optimized for its use case |
| 6 | Consistency | 4.0 | Timeline consistent with app; split-pane is new but opt-in |
| 7 | Scalability | 4.5 | Timeline handles scale; split-pane for small teams |
| 8 | Responsive | 4.0 | Timeline on narrow; split-pane on wide (≥1440px) |
| 9 | Discoverability | 4.0 | Timeline as default; split-pane discovered by power users |
| 10 | Flexibility | 5.0 | Maximum flexibility by definition |
| | **Weighted Total** | **4.10** | vs A=3.90, B=3.53 |

---

## 7. Final Recommendation

### Primary Recommendation: Approach C (Phased Hybrid)

**Build the chat timeline extension first (Phase 1), then add split-pane as an optional power mode (Phase 2).**

### Rationale

1. **Timeline first** because:
   - Lower risk — extends proven patterns with 12 new files, not 16 + new paradigms
   - Works for ALL team sizes (2-10+) with graceful degradation via filtering
   - Works on ALL screen sizes — existing responsive patterns handle it
   - Consistent with RalphX's existing UI language — no new interaction paradigms
   - Ships faster — gets team mode in users' hands sooner
   - Provides all shared infrastructure that split-pane needs (teamStore, events, API, backend)

2. **Split-pane second** because:
   - Serves power users who want simultaneous monitoring — the strongest use case
   - Excellent for the 2-4 agent sweet spot (ideation debate, small execution teams)
   - ~70% of infrastructure is already built in Phase 1; Phase 2 is mostly new frontend components
   - Well-architected brief with clear component hierarchy, phased implementation, and responsive fallbacks
   - CSS Grid layout is modern and performant; isolated state slices prevent rendering bottlenecks
   - Adds "wow factor" — the tmux metaphor resonates with developer users

3. **Not split-pane first** because:
   - Degrades past 5 teammates — designed cap is 6 visible panes; at 8+ it's minimized panes, which is a worse version of tabbed navigation
   - Loses its value prop below 1440px — tabbed fallback at 768-1199px is essentially Approach A without the combined timeline
   - Introduces new interaction paradigm (focus model, Ctrl+B prefix keys) — higher learning curve for initial feature launch
   - Depends on the same backend infrastructure as Approach A — no backend shortcut by going split-pane first
   - 16 new frontend files with new paradigms = more risk for the first release of team mode

### Implementation Phases

```
Phase 1 (Weeks 1-6): Chat Timeline Extension (Approach A)
  ├── teamStore.ts + useTeamEvents.ts (shared infrastructure)
  ├── src/api/team.ts (6 new IPC wrappers)
  ├── TeamActivityPanel + TeammateCard + TeamCostDisplay
  ├── TeamFilterTabs, TargetSelector, TeamMessageBubble, TeamSystemEvent
  ├── ChatPanel team mode + IntegratedChatPanel team mode
  ├── Backend: TeamStateTracker + team_commands.rs
  ├── Backend: StreamProcessorConfig team extensions
  └── Backend: event extensions (teammate_name, team_name on agent:* events)

Phase 2 (Weeks 7-11): Split-Pane View (Approach B)
  ├── splitPaneStore.ts (layout, focus, per-pane streaming state)
  ├── TeamSplitView + TeamSplitHeader + TeamSplitGrid (CSS Grid container)
  ├── CoordinatorPane (reuses ChatMessageList + ChatInput)
  ├── TeammatePaneGrid + TeammatePane + PaneHeader + PaneStream + PaneInput
  ├── usePaneEvents, useTeamKeyboardNav, usePaneResize, useTeamViewLifecycle hooks
  ├── Focus system + Ctrl+B prefix key navigation + PrefixKeyOverlay
  ├── Column + row resize handles (reuses existing ResizeHandle pattern)
  ├── Responsive breakpoints: tabbed fallback (<1200px), single-pane (<768px)
  ├── "team" view type in uiStore + auto-switching on team:created
  └── Team-active indicator in header navigation

Phase 3 (Week 12): Display Mode Toggle
  ├── User preference (timeline / split-pane / auto)
  ├── Smart defaults based on team size + context + viewport width
  └── Settings UI for preference
```

### Success Metrics

| Metric | Target |
|--------|--------|
| Team mode adoption | >50% of complex tasks use team mode within 2 months |
| Display mode preference | Track timeline vs split-pane usage to guide future investment |
| User satisfaction | No increase in support requests vs solo agent mode |
| Performance | <100ms latency for streaming with 5 concurrent agents |
| Split-pane usage | Power users (>5 team sessions) prefer split-pane at >30% |

---

## 8. Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Phase 1 ships and Phase 2 never built | Medium | Phase 1 is fully functional standalone; Phase 2 is a bonus that leverages shared infrastructure |
| Split-pane becomes maintenance burden | Low | Well-architected brief with clean component hierarchy; cap at 6 visible panes |
| Users confused by two display modes | Low | Smart defaults; mode toggle only visible when team active; auto-default by team size |
| Performance with 5+ concurrent streams | Medium | Timeline: debounce streaming. Split-pane: isolated state slices per pane (brief Section 12.4) |
| Scope creep in split-pane layout engine | Medium | Brief defines clear limits: max 6 panes, 4 breakpoints, specific CSS Grid approach |
| Ctrl+B conflicts with browser shortcuts | Low | Brief addresses: Ctrl+B prefix times out after 1.5s; rebindable via keybindings.json |
| Split-pane responsive fallback feels "second-class" | Medium | Tabbed fallback is acceptable since the core value prop requires wide screens by nature |

---

## Appendix A: References

| Document | Path | Lines |
|----------|------|-------|
| Chat Timeline Extension Brief | `docs/product-briefs/agent-teams-chat-ui-extension.md` | 1561 |
| Split-Pane Team UI Brief | `docs/product-briefs/agent-teams-split-pane-ui.md` | 983 |
| Agent Teams System Card | `docs/agent-teams-system-card.md` | 1237 |
| Agent Chat System Architecture | `docs/architecture/agent-chat-system.md` | 409 |
| Ideation Integration Brief | `docs/product-briefs/agent-teams-ideation-integration.md` | — |
| Worker Integration Brief | `docs/product-briefs/agent-teams-worker-integration.md` | — |
| RalphX Design System | `specs/DESIGN.md` | 806 |
| Existing Chat Panel Design | `specs/design/pages/chat-panel.md` | — |

## Appendix B: Scoring Methodology

- **Weights** reflect RalphX's priorities: implementation feasibility (15%), consistency with existing app (15%), and user experience (15%) are highest-weighted. Responsive and discoverability are lower (5% each) because RalphX is a desktop-first Mac app.
- **Scores** are based on implementation-level analysis of both full product briefs, including exact component counts, store designs, responsive breakpoint strategies, and file impact analyses.
- **v2 corrections:** Approach B scores improved in 5 criteria (complexity 2.5→3.0, consistency 2.5→3.0, scalability 2.5→3.0, responsive 2.0→3.0, discoverability 2.5→3.0) after reviewing the split-pane brief's detailed architecture, responsive strategy, and component reuse plan. Weighted total improved from 3.25 to 3.53.
- **Edge cases** (Section 5) verify that scores hold under real-world scenarios with varying team sizes, screen widths, and user experience levels.

## Appendix C: Component Count Comparison

| Metric | Approach A (Timeline) | Approach B (Split-Pane) |
|--------|----------------------|------------------------|
| **New frontend files** | 12 | 16 |
| **New components** | 7 | 11 |
| **New hooks** | 3 | 4 |
| **New stores** | 1 (teamStore) | 1 (splitPaneStore) + shared teamStore |
| **New API files** | 1 (team.ts) | 0 (uses A's team.ts) |
| **Modified frontend files** | 9 | 3 |
| **New backend files** | 2+ | 0 (depends on A's backend) |
| **Modified backend files** | 4+ | 0 |
| **Reused without modification** | 0 (extends existing components) | 7 (ChatMessageList, ChatInput, StatusActivityBadge, ResizeHandle, teamStore, team API, events) |
| **Total new frontend LOC (est.)** | ~2,500 | ~3,500 |
| **Total modified LOC (est.)** | ~800 | ~100 |
