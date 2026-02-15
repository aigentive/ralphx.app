# Product Brief: Split-Pane Team UI (tmux-Inspired)

**Status:** DRAFT v1
**Author:** split-pane-designer (agent-teams-ui-decision team)
**Date:** 2026-02-15
**Scope:** Full-screen split-pane layout for agent team sessions — React/CSS implementation inspired by tmux pane layout
**Depends on:**
- `docs/architecture/agent-chat-system.md` (current chat system)
- `docs/agent-teams-system-card.md` (agent teams capabilities)
- `docs/product-briefs/agent-teams-chat-ui-extension.md` (alternative approach — combined timeline)

---

## 1. Executive Summary

This brief proposes a **dedicated full-screen split-pane view** for agent team sessions, inspired by how tmux visually arranges concurrent processes. Instead of merging all team output into a single timeline (the chat-timeline approach), each agent gets its **own independent panel** with its own chat stream, status, and input.

**Key differentiator from the chat-timeline approach:** The chat-timeline brief (Section 4 of `agent-teams-chat-ui-extension.md`) merges all agent output into one chronological timeline with color-coded filter tabs. This split-pane approach gives each agent a **spatially distinct panel**, providing instant visual awareness of all concurrent agent activity without scrolling or filtering.

**Design metaphor:** tmux with a persistent coordinator pane on the left and stacked teammate panes on the right — but implemented entirely in React with CSS Grid, following the macOS Tahoe design system.

### Why Split Panes?

| Benefit | Chat Timeline | Split Panes |
|---------|--------------|-------------|
| **Concurrent visibility** | Must scroll/filter to see different agents | All agents visible simultaneously |
| **Agent focus** | Filter tab isolates one agent, hides others | Click any pane to focus without losing visibility of others |
| **Spatial awareness** | Agents differentiated only by color dots | Each agent has a dedicated screen region |
| **Direct interaction** | "Send to" dropdown to choose target | Type into the focused pane — target is implicit |
| **Mental model** | "Shared chatroom with colored messages" | "Individual workstations I can see at a glance" |
| **Noise management** | Busy teams create long interleaved timelines | Each pane contains its own agent's output — no interleaving |

### 9 Design Areas Covered

| # | Area | Key Decision |
|---|------|-------------|
| 1 | Layout system | CSS Grid: left column (coordinator, fixed) + right column (auto-grid rows for teammates) |
| 2 | Panel lifecycle | Auto-open on `team:teammate_spawned`, auto-close on `team:teammate_shutdown` |
| 3 | Panel interactions | Click-to-focus, resize handles, minimize/maximize, keyboard nav (Ctrl+B prefix) |
| 4 | Panel content | Each panel = mini ChatPanel with streaming text, tool calls, status badge |
| 5 | Coordinator panel | Always visible, shows team overview header + coordinator chat stream |
| 6 | Responsive behavior | <1200px: tabbed fallback. <768px: single-pane with selector |
| 7 | Navigation | Ctrl+B + arrow keys (tmux-like), Ctrl+B + number for direct pane select |
| 8 | Integration | New view type `"team"` in uiStore, activated when team session starts |
| 9 | Event system | Same `team:*` events, each pane subscribes filtered by `teammate_name` |
| 10 | State management | `splitPaneStore.ts` for layout, reuses `teamStore.ts` for agent state |
| 11 | Technical implementation | `TeamSplitView` → `CoordinatorPane` + `TeammatePaneGrid` → `TeammatePane[]` |

---

## 2. Layout System

### 2.1 Full-Screen Grid Layout

The team view replaces the current view content (kanban, graph, etc.) with a full-screen split-pane layout. The header (56px) remains visible for navigation back to other views.

```
┌──────────────────────────────────────────────────────────────────────┐
│  Header (56px) — [← Back to Kanban]  Team: task-abc  [⏹ Stop All]   │
├──────────────────────────────┬───────────────────────────────────────┤
│                              │  ┌─────────────────────────────────┐  │
│                              │  │ 🟢 coder-1 (sonnet)  [Running] │  │
│   COORDINATOR / LEAD         │  │ Auth middleware + session store  │  │
│                              │  │ ⚙ Write src/middleware/auth.ts  │  │
│   Team overview header       │  │ > streaming output...           │  │
│   ├─ 3 teammates active      │  │                                 │  │
│   ├─ 2 tasks in progress     │  │ [message input]         [Send]  │  │
│   └─ ~$2.10 total cost       │  ├─────────────────────────────────┤  │
│                              │  │                                 │  │
│   ─────────────────────────  │  │ 🔵 coder-2 (sonnet)  [Running] │  │
│                              │  │ Login/register endpoints        │  │
│   Lead chat stream           │  │ ⚙ Read src/api/auth.ts         │  │
│   ┌────────────────────────┐ │  │ > streaming output...           │  │
│   │ YOU: Implement auth    │ │  │                                 │  │
│   │ Lead: Analyzing...     │ │  │ [message input]         [Send]  │  │
│   │ Lead: Spawning team    │ │  ├─────────────────────────────────┤  │
│   │ Lead: coder-1 → auth   │ │  │                                 │  │
│   │ Lead: coder-2 → API    │ │  │ 🟡 coder-3 (haiku)     [Idle]  │  │
│   │ 💬 coder-1→coder-2    │ │  │ Integration tests               │  │
│   └────────────────────────┘ │  │ ✓ Completed test setup          │  │
│                              │  │ Waiting for auth module...       │  │
│   [message input]    [Send]  │  │                                 │  │
│                              │  │ [message input]         [Send]  │  │
│                              │  └─────────────────────────────────┘  │
└──────────────────────────────┴───────────────────────────────────────┘
         40% (resizable)                     60% (auto-distributed)
```

### 2.2 CSS Grid Implementation

```css
.team-split-view {
  display: grid;
  grid-template-columns: var(--coordinator-width, 40%) 1fr;
  grid-template-rows: 1fr;
  height: 100%;
  gap: 0;  /* Separator rendered via border, not gap */
}

.coordinator-pane {
  grid-column: 1;
  grid-row: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-right: 1px solid hsl(220 10% 16%);
}

.teammate-pane-grid {
  grid-column: 2;
  grid-row: 1;
  display: grid;
  grid-template-rows: repeat(var(--teammate-count), 1fr);
  overflow: hidden;
}

.teammate-pane {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-bottom: 1px solid hsl(220 10% 16%);
}

.teammate-pane:last-child {
  border-bottom: none;
}
```

### 2.3 Column Resize

A vertical resize handle between the coordinator pane and teammate grid allows width adjustment.

| Property | Value |
|----------|-------|
| Default split | 40% coordinator / 60% teammates |
| Min coordinator width | 300px |
| Max coordinator width | 60% of viewport |
| Resize handle | 8px hit area, 1px visible line (same pattern as `ResizeHandle.tsx`) |
| Persistence | `localStorage["ralphx-team-split-ratio"]` |

### 2.4 Row Sizing (Teammate Panes)

By default, teammate panes share equal height. Users can drag horizontal resize handles between panes to adjust distribution.

| Scenario | Layout |
|----------|--------|
| 1 teammate | Single pane fills right column |
| 2 teammates | 50/50 vertical split |
| 3 teammates | 33/33/33 vertical split |
| 4+ teammates | Equal split with scrollable overflow (max 4 visible, scroll for more) |
| Teammate minimized | Collapses to 36px header-only bar; remaining panes expand |

---

## 3. Panel Lifecycle

### 3.1 Lifecycle Events → Panel Actions

| Event | Panel Action | Animation |
|-------|-------------|-----------|
| `team:created` | Switch to team view, show coordinator pane (full width initially) | Fade in (200ms) |
| `team:teammate_spawned` | Add teammate pane to right grid, resize existing panes | Slide down from top (250ms) + grid reflow |
| `team:teammate_idle` | Update status badge to yellow "Idle" | Badge color transition (150ms) |
| `team:teammate_shutdown` | Collapse pane to 0 height, remove from grid | Slide up + fade out (200ms), then grid reflow |
| `team:disbanded` | Return to previous view (kanban/graph) | Fade out (200ms) |
| `agent:run_started` (with `teammate_name`) | Update pane status to green "Running" | Badge pulse |
| `agent:run_completed` (with `teammate_name`) | Update pane status, clear streaming | Badge settle |

### 3.2 Auto-View Switching

When a team is created in an ideation or execution context:

```
User triggers team mode (e.g., sends message that spawns team lead)
  │
  ├── Backend emits team:created
  │
  ├── Frontend detects team:created for current context
  │     └── uiStore.setCurrentView("team")
  │     └── teamSplitStore.initTeam(contextKey, teamName)
  │
  ├── Coordinator pane appears (full width, no teammates yet)
  │
  ├── Backend emits team:teammate_spawned (×N)
  │     └── teamSplitStore.addPane(teammate)
  │     └── Grid reflows to accommodate new pane
  │
  └── User sees all panes populated
```

### 3.3 Return to Previous View

When the team is disbanded or the user clicks "Back":

```
team:disbanded event OR user clicks [← Back]
  │
  ├── teamSplitStore.clearTeam(contextKey)
  ├── uiStore.setCurrentView(previousView)  // stored before entering team view
  └── Panes animate out, previous view fades in
```

---

## 4. Panel Interactions

### 4.1 Focus Model

One pane is "focused" at a time. The focused pane has a highlighted border and an active input field.

| Interaction | Behavior |
|-------------|----------|
| **Click pane** | Focus that pane — border highlights, input activates |
| **Type in focused pane** | Message goes to that pane's agent |
| **Ctrl+B → arrow** | Move focus between panes (tmux-like prefix key) |
| **Ctrl+B → number** | Jump to pane N (1=coordinator, 2+=teammates in order) |
| **Escape** | Blur input, keep pane focused |
| **Double-click pane header** | Toggle maximize/restore (pane fills right column or restores) |

### 4.2 Focus Visual Treatment

```css
/* Unfocused pane */
.teammate-pane {
  border-left: 2px solid transparent;
}

/* Focused pane — uses agent's assigned color */
.teammate-pane[data-focused="true"] {
  border-left: 2px solid var(--pane-agent-color);
}

/* Coordinator pane focus */
.coordinator-pane[data-focused="true"] {
  /* Subtle highlight in header area */
  .coordinator-header {
    background: hsl(220 10% 14%);
  }
}
```

### 4.3 Resize Handles

| Handle | Position | Direction |
|--------|----------|-----------|
| Vertical (column) | Between coordinator and teammate grid | Left-right |
| Horizontal (row) | Between each teammate pane | Up-down |

Both use the same `ResizeHandle` pattern from the existing codebase (8px hit area, 1px visible, orange on hover).

### 4.4 Pane Actions (Header Buttons)

Each teammate pane header has action buttons:

```
┌─────────────────────────────────────────────────────────┐
│ 🟢 coder-1 (sonnet)  [Running]    [_] [□] [×]  [⏹]   │
│                                     │   │   │    │      │
│                          minimize ──┘   │   │    │      │
│                          maximize ──────┘   │    │      │
│                          close/hide ────────┘    │      │
│                          stop agent ─────────────┘      │
└─────────────────────────────────────────────────────────┘
```

| Button | Action | Keyboard |
|--------|--------|----------|
| Minimize `[_]` | Collapse to 36px header bar | Ctrl+B, - |
| Maximize `[□]` | Expand to fill right column (others minimize) | Ctrl+B, z |
| Close `[×]` | Hide pane (agent still running in background) | — |
| Stop `[⏹]` | Send shutdown_request to agent | — |

---

## 5. Panel Content

### 5.1 Teammate Pane Anatomy

Each teammate pane is a self-contained mini chat panel:

```
┌─ Pane Header ────────────────────────────────────────────┐
│ [color dot] name (model)    role desc    [status] [acts] │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Chat Stream (scrollable)                                │
│  ├─ Agent text output (streaming)                        │
│  ├─ Tool call indicators (file reads, writes, etc.)      │
│  ├─ Subagent task indicators                             │
│  └─ Inter-agent messages (from/to this agent)            │
│                                                          │
├──────────────────────────────────────────────────────────┤
│  [message input]                              [Send ▶]   │
└──────────────────────────────────────────────────────────┘
```

### 5.2 Pane Header Fields

| Field | Source | Example |
|-------|--------|---------|
| Color dot | `team:teammate_spawned.color` | 🟢 |
| Name | `team:teammate_spawned.teammate_name` | coder-1 |
| Model | `team:teammate_spawned.model` | (sonnet) |
| Role description | `team:teammate_spawned.role_description` | Auth middleware |
| Status badge | Derived from agent events | [Running] / [Idle] / [Done] |
| Token count | `team:cost_update` | ~85K tokens |

### 5.3 Chat Stream Content Types

| Content | Visual Treatment | Source Event |
|---------|-----------------|-------------|
| Agent text | Left-aligned, `--text-primary` | `agent:chunk` (filtered by `teammate_name`) |
| Tool call | Compact indicator: `⚙ Write src/file.ts` | `agent:tool_call` (filtered) |
| Subagent task | Spinner + description | `agent:task_started` / `agent:task_completed` |
| Inter-agent message (incoming) | Dimmed bubble: "💬 from coder-2: ..." | `team:message` (where `to` = this agent) |
| Inter-agent message (outgoing) | Dimmed bubble: "→ coder-2: ..." | `team:message` (where `from` = this agent) |
| User message | Right-aligned, accent color | User input from pane |
| System event | Centered, muted | "Agent idle", "Task #3 completed" |

### 5.4 Pane Input Behavior

| State | Input Behavior |
|-------|----------------|
| Pane focused + agent running | Input enabled, placeholder: "Message coder-1..." |
| Pane focused + agent idle | Input enabled, placeholder: "Message coder-1 (idle)..." |
| Pane unfocused | Input dimmed, click to focus |
| Agent shutdown | Input disabled, shows "Agent stopped" |

---

## 6. Coordinator Panel

### 6.1 Special Treatment

The coordinator (team lead) pane is permanently visible on the left. It has two sections:

```
┌─ Coordinator Pane ──────────────────────────────────┐
│                                                      │
│  ┌─ Team Overview Header ─────────────────────────┐ │
│  │  Team: task-abc123                              │ │
│  │  Teammates: 3 active, 0 idle                    │ │
│  │  Tasks: 2 in_progress, 1 completed              │ │
│  │  Cost: ~$2.10 (250K tokens)                     │ │
│  │  [⏹ Stop All]  [🗑 Disband]                    │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─ Lead Chat Stream ─────────────────────────────┐ │
│  │  (standard chat: user messages + lead output)   │ │
│  │  Shows lead's coordination messages,            │ │
│  │  team lifecycle events, and inter-agent         │ │
│  │  message summaries                              │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  [message input]                          [Send ▶]   │
└──────────────────────────────────────────────────────┘
```

### 6.2 Team Overview Header

The header is a compact summary strip (not a full panel). It collapses to a single line when the coordinator chat has many messages.

| Field | Source | Update Frequency |
|-------|--------|-----------------|
| Team name | `team:created.team_name` | Once |
| Active teammates | Count of teammates with status `running` or `idle` | On `team:teammate_*` events |
| Task progress | From shared task list | On `TaskUpdate` events |
| Total cost | Sum of all teammate costs | On `team:cost_update` (debounced 5s) |
| Stop All button | Sends `stop_team` IPC | — |
| Disband button | Sends `stop_team` then confirmation dialog | — |

### 6.3 Lead Chat Stream

The lead's chat stream shows:
- User ↔ Lead messages (same as existing ChatPanel)
- Team lifecycle events: "Spawned coder-1", "coder-2 completed task #3"
- Inter-agent message summaries (lead sees all peer DMs as summaries)
- Lead's own coordination output

This reuses the existing `ChatMessageList` component with team event interleaving.

---

## 7. Responsive Behavior

### 7.1 Breakpoint Strategy

| Breakpoint | Layout | Behavior |
|------------|--------|----------|
| ≥1440px | Full split-pane grid | All panes visible simultaneously |
| 1200–1439px | Reduced grid | Max 2 teammate panes visible; scrollable overflow |
| 768–1199px | **Tabbed fallback** | Coordinator tab + one teammate tab visible; tab bar to switch |
| <768px | **Single pane + selector** | Dropdown to select which pane to view; one pane fills screen |

### 7.2 Tabbed Fallback (768–1199px)

```
┌──────────────────────────────────────────────────────┐
│  [Lead] [🟢 coder-1] [🔵 coder-2] [🟡 coder-3]     │
├──────────────────────────────────────────────────────┤
│                                                      │
│  (Full-height content of selected tab)               │
│                                                      │
│  Same pane content as split view, but one at a time  │
│                                                      │
├──────────────────────────────────────────────────────┤
│  [message input]                          [Send ▶]   │
└──────────────────────────────────────────────────────┘
```

Tab bar uses teammate colors as indicator dots. Unread badge (small dot) appears on tabs with new messages.

### 7.3 Single Pane (<768px)

```
┌──────────────────────────────────────────┐
│  Viewing: [coder-1 ▾]     [⏹ Stop All]  │
├──────────────────────────────────────────┤
│                                          │
│  (Full content of selected pane)         │
│                                          │
├──────────────────────────────────────────┤
│  [message input]              [Send ▶]   │
└──────────────────────────────────────────┘
```

---

## 8. Keyboard Navigation

### 8.1 tmux-Inspired Prefix Key

Keyboard navigation uses **Ctrl+B** as a prefix key (same as tmux default), followed by a navigation key.

| Shortcut | Action |
|----------|--------|
| `Ctrl+B` → `↑` | Focus pane above |
| `Ctrl+B` → `↓` | Focus pane below |
| `Ctrl+B` → `←` | Focus coordinator (left) |
| `Ctrl+B` → `→` | Focus teammate grid (right, topmost or last focused) |
| `Ctrl+B` → `1` | Focus coordinator |
| `Ctrl+B` → `2` | Focus teammate 1 |
| `Ctrl+B` → `3` | Focus teammate 2 |
| `Ctrl+B` → `N` | Focus teammate N-1 |
| `Ctrl+B` → `z` | Toggle maximize focused pane |
| `Ctrl+B` → `-` | Minimize focused pane |
| `Ctrl+B` → `=` | Reset all pane sizes to equal |
| `Ctrl+B` → `x` | Stop focused agent (with confirmation) |

### 8.2 Implementation

```typescript
// useTeamKeyboardNav.ts — prefix key handler
function useTeamKeyboardNav(isTeamView: boolean) {
  const [prefixActive, setPrefixActive] = useState(false);
  const prefixTimeoutRef = useRef<NodeJS.Timeout>();

  useEffect(() => {
    if (!isTeamView) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+B activates prefix mode (1.5s timeout)
      if (e.ctrlKey && e.key === "b") {
        e.preventDefault();
        setPrefixActive(true);
        clearTimeout(prefixTimeoutRef.current);
        prefixTimeoutRef.current = setTimeout(() => setPrefixActive(false), 1500);
        return;
      }

      if (!prefixActive) return;

      // Consume the next key as a navigation command
      setPrefixActive(false);
      switch (e.key) {
        case "ArrowUp": focusPaneAbove(); break;
        case "ArrowDown": focusPaneBelow(); break;
        case "ArrowLeft": focusCoordinator(); break;
        case "ArrowRight": focusTeammateGrid(); break;
        case "z": toggleMaximize(); break;
        case "-": minimizeFocused(); break;
        case "=": resetPaneSizes(); break;
        // ... number keys for direct pane selection
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isTeamView, prefixActive]);
}
```

### 8.3 Prefix Key Indicator

When Ctrl+B is pressed, a small indicator appears in the header:

```
┌──────────────────────────────────────────────────────┐
│  ← Back   Team: task-abc   [⌨ Ctrl+B active]  [⏹]  │
└──────────────────────────────────────────────────────┘
```

Disappears after 1.5s or after the next keypress.

---

## 9. Integration with Existing UI

### 9.1 New View Type: `"team"`

The team split view is a **new view type** in `uiStore`, alongside existing views (ideation, kanban, graph, etc.).

```typescript
// src/stores/uiStore.ts — EXTENDED
export type ViewType = "ideation" | "graph" | "kanban" | "extensibility"
                     | "activity" | "settings" | "team";  // NEW

interface UiState {
  currentView: ViewType;
  previousView: ViewType | null;  // NEW — for "Back" navigation from team view
  // ... existing fields
}
```

### 9.2 View Switching

| Trigger | Action |
|---------|--------|
| `team:created` event (current context) | `uiStore.previousView = currentView; uiStore.currentView = "team"` |
| User clicks "← Back" in team header | `uiStore.currentView = previousView` |
| `team:disbanded` event | `uiStore.currentView = previousView` |
| User navigates via header nav (⌘1-6) | Normal view switch; team view remains available via team indicator |

### 9.3 Team View in App.tsx

```tsx
// App.tsx — view renderer (extended)
function MainContent() {
  const currentView = useUiStore((s) => s.currentView);

  switch (currentView) {
    case "ideation": return <IdeationView />;
    case "kanban": return <KanbanSplitLayout />;
    case "graph": return <GraphSplitLayout />;
    case "team": return <TeamSplitView />;  // NEW
    // ... other views
  }
}
```

### 9.4 Team Indicator in Header

When a team is active but the user navigates to another view, a persistent indicator appears in the header:

```
┌──────────────────────────────────────────────────────────────┐
│  [Ideation] [Graph] [Kanban]    🤖 Team Active (3) [View →] │
└──────────────────────────────────────────────────────────────┘
```

Clicking "View →" switches back to the team view.

### 9.5 Relationship to Kanban/Task Detail

The team view is **independent** of the Kanban board. However:
- The team's source context (e.g., a task in execution) is shown in the coordinator header
- Clicking a task reference in any pane opens a tooltip/popover with task details (not a full navigation)
- The team view does NOT show TaskDetailOverlay — pane content replaces it

---

## 10. Event System

### 10.1 Event Routing: Per-Pane Filtering

Each pane subscribes to the same `agent:*` and `team:*` events but filters by `teammate_name`. This reuses the event architecture from the chat-timeline brief (Section 3 of `agent-teams-chat-ui-extension.md`).

```
EventBus
  │
  ├── team:created → TeamSplitView (creates coordinator pane)
  ├── team:teammate_spawned → TeamSplitView (adds teammate pane)
  │
  ├── agent:chunk { teammate_name: "coder-1" }
  │     └── TeammatePane("coder-1") processes
  │     └── TeammatePane("coder-2") ignores
  │     └── CoordinatorPane ignores
  │
  ├── agent:chunk { teammate_name: null }
  │     └── CoordinatorPane processes (lead output)
  │     └── TeammatePane(*) ignores
  │
  ├── team:message { from: "coder-1", to: "coder-2" }
  │     └── TeammatePane("coder-1") shows as outgoing
  │     └── TeammatePane("coder-2") shows as incoming
  │     └── CoordinatorPane shows as summary
  │
  ├── team:teammate_idle { teammate_name: "coder-3" }
  │     └── TeammatePane("coder-3") updates status badge
  │
  └── team:disbanded
        └── TeamSplitView exits → returns to previous view
```

### 10.2 Per-Pane Event Hook

```typescript
// usePaneEvents.ts — scoped event consumer for a single pane
function usePaneEvents(
  contextKey: string,
  teammateName: string | null,  // null = coordinator
) {
  const bus = useEventBus();

  useEffect(() => {
    const unsubs: Unsubscribe[] = [];

    // Streaming chunks — filtered by teammate_name
    unsubs.push(bus.subscribe("agent:chunk", (payload) => {
      const key = buildStoreKey(payload.context_type, payload.context_id);
      if (key !== contextKey) return;
      if (payload.teammate_name !== teammateName) return;
      // Append to this pane's streaming buffer
      splitPaneStore.appendPaneChunk(teammateName ?? "lead", payload.text);
    }));

    // Tool calls — same filtering
    unsubs.push(bus.subscribe("agent:tool_call", (payload) => {
      if (buildStoreKey(payload.context_type, payload.context_id) !== contextKey) return;
      if (payload.teammate_name !== teammateName) return;
      splitPaneStore.addPaneToolCall(teammateName ?? "lead", payload);
    }));

    // Inter-agent messages — show in both sender and receiver panes
    unsubs.push(bus.subscribe("team:message", (payload) => {
      if (buildStoreKey(payload.context_type, payload.context_id) !== contextKey) return;
      if (payload.from === teammateName || payload.to === teammateName || teammateName === null) {
        splitPaneStore.addPaneMessage(teammateName ?? "lead", payload);
      }
    }));

    return () => unsubs.forEach(u => u());
  }, [bus, contextKey, teammateName]);
}
```

### 10.3 Coordinator Event Aggregation

The coordinator pane additionally subscribes to **all** team events (unfiltered) to build the team overview:

| Event | Coordinator Action |
|-------|--------------------|
| `team:teammate_spawned` | Update teammate count, add to overview list |
| `team:teammate_idle` | Update teammate status in overview |
| `team:teammate_shutdown` | Decrement active count |
| `team:message` | Show in lead's chat as summary: "💬 coder-1 → coder-2" |
| `team:cost_update` | Update aggregate cost display |
| All `agent:*` with `teammate_name: null` | Normal lead chat stream processing |

---

## 11. State Management

### 11.1 New Store: `splitPaneStore.ts`

Manages layout state for the split-pane view. Separate from `teamStore.ts` (which manages team agent state) and `chatStore.ts` (which manages chat conversations).

```typescript
// src/stores/splitPaneStore.ts — NEW

interface PaneState {
  id: string;                    // "lead" or teammate name
  isMinimized: boolean;
  isFocused: boolean;
  customHeight?: number;         // null = auto (equal distribution)
  streamingText: string;         // Accumulates agent:chunk text
  streamingToolCalls: ToolCall[];
  streamingTasks: Map<string, StreamingTask>;
  messages: PaneMessage[];       // Chat messages shown in this pane
  unreadCount: number;           // Messages received while unfocused
}

interface PaneMessage {
  id: string;
  type: "user" | "agent" | "inter_agent" | "system";
  from?: string;
  to?: string;
  content: string;
  timestamp: string;
  toolCalls?: ToolCall[];
}

interface SplitPaneState {
  // Layout
  isActive: boolean;
  contextKey: string | null;       // Which context this team view is for
  coordinatorWidth: number;        // Percentage (default: 40)
  panes: Record<string, PaneState>; // Keyed by pane ID ("lead", "coder-1", etc.)
  paneOrder: string[];             // Ordered teammate IDs (display order)
  focusedPaneId: string;           // Currently focused pane

  // Keyboard
  prefixKeyActive: boolean;
}

interface SplitPaneActions {
  // Lifecycle
  initTeam: (contextKey: string, leadName: string) => void;
  addPane: (teammateName: string) => void;
  removePane: (teammateName: string) => void;
  clearTeam: () => void;

  // Focus
  setFocusedPane: (paneId: string) => void;
  focusNext: () => void;
  focusPrev: () => void;

  // Layout
  setCoordinatorWidth: (percent: number) => void;
  minimizePane: (paneId: string) => void;
  maximizePane: (paneId: string) => void;
  restorePane: (paneId: string) => void;
  resetPaneSizes: () => void;

  // Content (streaming state per pane)
  appendPaneChunk: (paneId: string, text: string) => void;
  clearPaneStream: (paneId: string) => void;
  addPaneToolCall: (paneId: string, toolCall: ToolCall) => void;
  addPaneMessage: (paneId: string, message: PaneMessage) => void;

  // Keyboard
  setPrefixKeyActive: (active: boolean) => void;
}
```

### 11.2 Relationship to Other Stores

```
splitPaneStore — Layout & per-pane streaming state (NEW)
  ├── panes["lead"].streamingText
  ├── panes["coder-1"].streamingText
  └── panes["coder-2"].streamingText

teamStore — Team agent state (from chat-timeline brief, reused)
  ├── activeTeams[contextKey].teammates
  ├── activeTeams[contextKey].messages
  └── activeTeams[contextKey].totalEstimatedCostUsd

chatStore — Conversation state (existing, minimal extension)
  └── isTeamActive[contextKey]: boolean
```

**Why separate stores?**
- `splitPaneStore` is UI-layout state (pane sizes, focus, streaming buffers). It's destroyed when leaving team view.
- `teamStore` is team-agent state (teammate metadata, costs, inter-agent messages). It persists as long as the team exists.
- `chatStore` manages conversation-level state (which conversation is active, running flags). It's reused for the coordinator pane's chat.

### 11.3 Store Key Alignment

All three stores use the same `contextKey` pattern from `buildStoreKey()`:

```
splitPaneStore.contextKey = "task_execution:abc"
teamStore.activeTeams["task_execution:abc"] = { ... }
chatStore.isTeamActive["task_execution:abc"] = true
```

---

## 12. Technical Implementation

### 12.1 Component Hierarchy

```
TeamSplitView (NEW — top-level view component)
  ├── TeamSplitHeader (NEW — "← Back", team name, prefix key indicator, stop all)
  ├── TeamSplitGrid (NEW — CSS Grid container)
  │     ├── CoordinatorPane (NEW)
  │     │     ├── TeamOverviewHeader (NEW — compact team stats)
  │     │     ├── ChatMessageList (EXISTING — reused for lead chat)
  │     │     └── ChatInput (EXISTING — reused, sends to lead)
  │     │
  │     ├── ColumnResizeHandle (EXISTING pattern — vertical between columns)
  │     │
  │     └── TeammatePaneGrid (NEW — right column grid)
  │           ├── TeammatePane (NEW — one per active teammate)
  │           │     ├── PaneHeader (NEW — name, model, status, actions)
  │           │     ├── PaneStream (NEW — streaming text + tool calls)
  │           │     ├── PaneMessages (NEW — inter-agent messages for this agent)
  │           │     └── PaneInput (NEW — sends message to this teammate)
  │           │
  │           └── RowResizeHandle (EXISTING pattern — horizontal between rows)
  │
  └── PrefixKeyOverlay (NEW — visual indicator when Ctrl+B is active)
```

### 12.2 Component Details

| Component | Purpose | Reuses |
|-----------|---------|--------|
| `TeamSplitView` | View container, event wiring, keyboard nav | — |
| `TeamSplitHeader` | Navigation bar with back button, team info, global actions | Header pattern from App.tsx |
| `TeamSplitGrid` | CSS Grid layout manager | — |
| `CoordinatorPane` | Lead chat + team overview | `ChatMessageList`, `ChatInput` |
| `TeamOverviewHeader` | Compact stats: teammates, tasks, cost | — |
| `TeammatePaneGrid` | Auto-sizing grid for N teammate panes | — |
| `TeammatePane` | Self-contained mini chat for one agent | `ChatInput` (simplified) |
| `PaneHeader` | Teammate identity + status + action buttons | `StatusActivityBadge` pattern |
| `PaneStream` | Streaming text display with tool call indicators | Streaming logic from `useChatEvents` |
| `PaneMessages` | Inter-agent message bubbles for this agent | — |
| `PaneInput` | Message input for this teammate | `ChatInput` (minimal variant) |
| `PrefixKeyOverlay` | "Ctrl+B active" toast | — |

### 12.3 New Hooks

| Hook | Purpose |
|------|---------|
| `usePaneEvents` | Per-pane event subscription filtered by `teammate_name` |
| `useTeamKeyboardNav` | Ctrl+B prefix key + navigation commands |
| `usePaneResize` | Row/column resize drag handling |
| `useTeamViewLifecycle` | Auto-switch to/from team view on `team:created`/`team:disbanded` |

### 12.4 Performance Considerations

| Concern | Mitigation |
|---------|-----------|
| **N concurrent streaming panes** | Each pane has its own streaming buffer in `splitPaneStore`. React only re-renders the pane that receives a chunk (isolated state slices). |
| **Event storm from N agents** | Events are filtered at the hook level (`usePaneEvents`) — each pane only processes events matching its `teammate_name`. No global re-renders. |
| **DOM complexity with many panes** | Max 8 panes visible (1 coordinator + 7 teammates). Overflow teammates are in a scrollable list or minimized. |
| **Streaming text accumulation** | Streaming buffers are cleared on `agent:run_completed`. Long-running sessions use chunked appending (same as existing `useChatEvents`). |
| **Resize performance** | CSS Grid reflow is GPU-accelerated. Resize uses `requestAnimationFrame` for smooth updates. |
| **Memory per pane** | Each pane stores: streaming text (cleared on completion) + last 100 tool calls + last 50 messages. Older items available via API query. |

### 12.5 File Impact Analysis

#### New Files

| File | Purpose |
|------|---------|
| `src/components/Team/TeamSplitView.tsx` | Top-level team view component |
| `src/components/Team/TeamSplitHeader.tsx` | Team view header bar |
| `src/components/Team/TeamSplitGrid.tsx` | CSS Grid layout manager |
| `src/components/Team/CoordinatorPane.tsx` | Lead pane (overview + chat) |
| `src/components/Team/TeamOverviewHeader.tsx` | Compact team stats |
| `src/components/Team/TeammatePaneGrid.tsx` | Right column grid container |
| `src/components/Team/TeammatePane.tsx` | Single teammate pane |
| `src/components/Team/PaneHeader.tsx` | Teammate pane header |
| `src/components/Team/PaneStream.tsx` | Streaming text display |
| `src/components/Team/PaneInput.tsx` | Simplified chat input for panes |
| `src/components/Team/PrefixKeyOverlay.tsx` | Keyboard prefix indicator |
| `src/stores/splitPaneStore.ts` | Layout + per-pane streaming state |
| `src/hooks/usePaneEvents.ts` | Per-pane event filtering |
| `src/hooks/useTeamKeyboardNav.ts` | tmux-like keyboard navigation |
| `src/hooks/usePaneResize.ts` | Row/column resize handling |
| `src/hooks/useTeamViewLifecycle.ts` | Auto view switching on team events |

#### Modified Files

| File | Change |
|------|--------|
| `src/stores/uiStore.ts` | Add `"team"` to `ViewType`, add `previousView` field |
| `src/App.tsx` | Add `case "team": return <TeamSplitView />` to view switch |
| `src/components/layout/Navigation.tsx` | Add team-active indicator in header |

#### Reused (No Modification)

| File | Reused By |
|------|-----------|
| `src/components/Chat/ChatMessageList.tsx` | CoordinatorPane |
| `src/components/Chat/ChatInput.tsx` | CoordinatorPane, PaneInput |
| `src/components/Chat/StatusActivityBadge.tsx` | PaneHeader |
| `src/components/ui/ResizeHandle.tsx` | Column + Row resize handles |
| `src/stores/teamStore.ts` | Team agent state (shared with chat-timeline approach) |
| `src/api/team.ts` | Team API wrappers (shared) |
| `src/lib/events.ts` | Event constants (shared, needs team:* additions from chat-timeline brief) |

---

## 13. Phased Implementation

### Phase 1: Static Layout + View Switching

| Task | Files | Estimate |
|------|-------|----------|
| Add `"team"` view type to uiStore | `uiStore.ts` | Small |
| Create `TeamSplitView` shell with CSS Grid | New component | Medium |
| Create `CoordinatorPane` with existing `ChatMessageList` | New component | Medium |
| Create `TeammatePane` with static content | New component | Medium |
| Add view switching in App.tsx | `App.tsx` | Small |
| `splitPaneStore.ts` with layout state | New store | Medium |

### Phase 2: Event Wiring + Streaming

| Task | Files | Estimate |
|------|-------|----------|
| `usePaneEvents` hook with per-pane filtering | New hook | Medium |
| `useTeamViewLifecycle` for auto view switching | New hook | Small |
| Per-pane streaming text accumulation | `splitPaneStore`, `PaneStream` | Medium |
| Tool call indicators in panes | `PaneStream` | Small |
| Inter-agent message display | `PaneMessages` in `TeammatePane` | Medium |

### Phase 3: Interactions + Polish

| Task | Files | Estimate |
|------|-------|----------|
| Click-to-focus + focus visual treatment | `TeamSplitGrid`, `TeammatePane` | Small |
| `PaneInput` (message input per pane) | New component | Medium |
| Keyboard navigation (`useTeamKeyboardNav`) | New hook | Medium |
| Column + row resize handles | `usePaneResize` | Medium |
| Minimize/maximize pane actions | `PaneHeader`, `splitPaneStore` | Small |
| Team overview header in coordinator pane | `TeamOverviewHeader` | Small |

### Phase 4: Responsive + Integration

| Task | Files | Estimate |
|------|-------|----------|
| Tabbed fallback for <1200px | `TeamSplitView` responsive branch | Medium |
| Single-pane mode for <768px | `TeamSplitView` responsive branch | Small |
| Team-active indicator in header nav | `Navigation.tsx` | Small |
| Panel lifecycle animations (spawn/shutdown) | CSS + `TeammatePane` | Small |
| Prefix key visual indicator | `PrefixKeyOverlay` | Small |

---

## 14. Comparison with Chat Timeline Approach

| Dimension | Chat Timeline (other brief) | Split Panes (this brief) |
|-----------|---------------------------|-------------------------|
| **Core metaphor** | Shared chatroom | Individual workstations |
| **Agent visibility** | One at a time (via filter) or interleaved (all) | All simultaneously (up to 8) |
| **Interaction model** | "Send to" dropdown | Type in focused pane |
| **Complexity** | Extends existing ChatPanel | New view + new components |
| **Component count** | 7 new components | 16 new components |
| **Store additions** | 1 new store (teamStore) | 2 new stores (teamStore + splitPaneStore) |
| **Reuse of existing code** | High (extends ChatPanel, ChatMessageList) | Medium (reuses ChatInput, ChatMessageList, StatusActivityBadge) |
| **Learning curve for users** | Low (familiar chat UI + tabs) | Medium (new layout paradigm, keyboard shortcuts) |
| **Spatial awareness** | Low (must filter or scroll) | High (all agents visible at once) |
| **Screen efficiency** | Good for narrow screens | Needs wide screen (≥1200px for best experience) |
| **Maximum useful teammates** | Unlimited (timeline scales) | 4-7 before panes get too small |
| **Implementation effort** | Medium (extends existing) | Large (new view, new layout system) |
| **Backend changes** | Same event system, same IPC commands | Same (identical backend requirements) |
| **Responsive design** | Natural (single column) | Needs fallback layouts for small screens |
| **Integration risk** | Low (additive to existing panels) | Medium (new view type, routing changes) |

---

## 15. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|-----------|
| **Panes too small with many teammates** | Medium | Cap visible panes at 4-7; overflow is minimized or tabbed. Default to maximize single teammate when >5. |
| **Keyboard shortcut conflicts** | Low | Ctrl+B prefix avoids conflicts. Prefix times out after 1.5s. User can rebind via keybindings.json. |
| **Performance with N concurrent streams** | Medium | Isolated pane state slices prevent cascading re-renders. Each `usePaneEvents` filters at subscription level. |
| **Complexity vs chat timeline** | Medium | More components to build and maintain. Mitigated by clean component hierarchy and store separation. |
| **Responsive fallback UX gap** | Medium | Tabbed fallback on medium screens loses the core "simultaneous visibility" value prop. Acceptable trade-off. |
| **View state loss on navigation** | Low | `splitPaneStore` persists while team is active. Navigating away and back restores pane layout. |
| **Coordinator pane too busy** | Low | Team overview header is collapsible. Lead chat uses existing ChatMessageList with proven scroll handling. |

---

## 16. Open Questions

| # | Question | Options | Recommendation |
|---|----------|---------|----------------|
| 1 | Should the team view be a route (`/team/:id`) or a view state? | Route / View state | **View state** — consistent with existing pattern (kanban, graph are not routes) |
| 2 | Should we support "picture-in-picture" for a teammate pane? | Yes (detachable pane) / No | **No (Phase 1)** — adds significant complexity. Can be added later. |
| 3 | Should the coordinator pane show a minimap of all teammate activity? | Yes (visual overview) / No (text stats only) | **Text stats only** — minimap is complex and the overview header provides sufficient awareness. |
| 4 | Maximum visible teammates before overflow? | 4 / 6 / 8 | **6** (1 coordinator + 5 teammates visible). More than 5 teammate panes becomes unusably small. |
| 5 | Should unfocused panes dim their content? | Yes (dim to 70% opacity) / No | **Yes** — subtle dim (90% opacity) on unfocused panes helps identify the active pane without being distracting. |

---

## 17. Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| `team:*` event system (from chat-timeline brief Section 3) | Required | Both approaches share the same backend event infrastructure |
| `teamStore.ts` (from chat-timeline brief Section 10) | Required | Reused for team agent state (teammates, costs, messages) |
| `src/api/team.ts` (from chat-timeline brief Section 11) | Required | Reused for IPC wrappers |
| Backend: `TeamStateTracker` service | Required | Same backend for both approaches |
| Backend: `StreamProcessorConfig` team extensions | Required | Per-teammate stream tagging |
| `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` | Required | Feature flag for agent teams |
