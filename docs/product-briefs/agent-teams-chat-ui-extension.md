# Product Brief: Chat UI & Backend Extensions for Agent Teams

**Status:** DRAFT v1
**Author:** chat-ui-designer (agent-teams-final-revisions team)
**Date:** 2026-02-15
**Scope:** Extending RalphX's chat system (UI, events, stores, backend) to support agent teams
**Depends on:**
- `docs/architecture/agent-chat-system.md` (current system reference)
- `docs/agent-teams-system-card.md` (agent teams capabilities)
- `docs/product-briefs/agent-teams-ideation-integration.md` (ideation team UI)
- `docs/product-briefs/agent-teams-worker-integration.md` (worker team UI)
- `docs/product-briefs/configurable-agent-variants.md` (process mapping, variant selection)

---

## 1. Executive Summary

This brief documents how RalphX's existing chat system — ChatService, event bus, Zustand stores, React hooks, and Tauri IPC commands — must be extended to support agent teams. The current system is designed for **single-agent conversations**: one agent per context, one stream, one set of events. Agent teams introduce **N concurrent agents** per context, requiring multiplexed events, team-aware state management, multi-target messaging, and new UI surfaces.

**Design principle: Extend, don't replace.** Every extension builds on existing patterns (EventBus subscriptions, `agent:*` event naming, `buildStoreKey()` keying, TanStack Query invalidation). Solo agent flows remain untouched — team features activate only when `teamMode` is set.

**9 extension areas covered:**

| # | Area | Complexity | Key Change |
|---|------|-----------|------------|
| 1 | Multi-agent event system | High | New `team:*` events + `teammate_name` field on existing `agent:*` events |
| 2 | Chat UI for team visibility | High | Combined timeline with agent-color coding + teammate filter tabs |
| 3 | User-to-teammate messaging | Medium | New `send_team_message` IPC + message routing through lead |
| 4 | Team activity panel | Medium | New `TeamActivityPanel` component with live teammate status |
| 5 | ChatContextType extensions | Low | No new context types — team mode is a property on existing contexts |
| 6 | Agent resolution changes | Medium | `resolve_agent()` extended for team lead variants via process_mapping |
| 7 | Streaming from multiple agents | High | Per-teammate stream multiplexing with `teammate_name` discriminator |
| 8 | Team state in Zustand stores | Medium | New `teamStore.ts` + extended `chatStore` with per-teammate running state |
| 9 | Tauri command extensions | Medium | 6 new IPC commands for team lifecycle + messaging |

---

## 2. Current Architecture (Key Subsystems to Extend)

Reference: `docs/architecture/agent-chat-system.md` for full details.

### 2.1 Current Data Flow

```
User message
  → ChatPanel / IntegratedChatPanel
    → Tauri IPC: send_agent_message { contextType, contextId, content }
      → ChatService.send_message()
        → resolve_agent(context_type, entity_status) → single agent name
        → build_command() → SpawnableCommand (claude --agent ralphx:<name>)
        → stream_response() → parse JSON-stream events
          → app_handle.emit("agent:*", payload)
            → EventBus → useAgentEvents / useChatEvents
              → chatStore updates → UI re-render
```

### 2.2 Key Subsystems

| Subsystem | Current Behavior | Team Extension Needed |
|-----------|-----------------|----------------------|
| **ChatService** | One agent per context, sequential send | Concurrent teammate tracking, team lifecycle |
| **Agent Resolution** | `resolve_agent()` → single agent name | Team lead variant selection via `process_mapping` |
| **Event System** | `agent:*` events with `context_type` + `context_id` | Add `teammate_name` to events, new `team:*` events |
| **Streaming Parser** | Single JSON-stream per context | Multiplex N teammate streams concurrently |
| **chatStore** | `isAgentRunning[contextKey]` (boolean per context) | Per-teammate running state within a context |
| **ChatPanel** | Single message timeline with one agent | Multi-agent timeline with color-coded teammate messages |
| **Message Queue** | Sequential queue per context | Queue targets: lead vs specific teammate |
| **Tauri Commands** | 11 commands, all single-agent | 6+ new commands for team management |

### 2.3 Current Event Payload Structure

All events include `context_type` and `context_id` for routing:

```typescript
// Current: agent:chunk event
{ text: string, conversation_id: string, context_type: string, context_id: string }
```

Team extension adds `teammate_name` and `team_name`:

```typescript
// Extended: agent:chunk event (backward-compatible)
{
  text: string,
  conversation_id: string,
  context_type: string,
  context_id: string,
  teammate_name?: string,  // NEW — null for solo agents, set for team members
  team_name?: string,      // NEW — null for solo, set for team contexts
}
```

---

## 3. Extension Area 1: Multi-Agent Event System

### 3.1 Design: Backward-Compatible Event Extension

**Approach:** Extend existing `agent:*` events with optional team fields rather than creating a parallel event namespace. This preserves all existing event consumers while enabling team-aware consumers to filter by `teammate_name`.

#### Extended Event Payloads

Every existing `agent:*` event gains two optional fields:

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `teammate_name` | `string \| null` | `null` | Identifies which teammate emitted the event. `null` = solo agent or team lead. |
| `team_name` | `string \| null` | `null` | Team identifier. `null` = not a team context. |
| `teammate_color` | `string \| null` | `null` | Assigned color for UI differentiation (e.g., `"#3b82f6"`) |

**Backward compatibility:** Existing hooks (`useAgentEvents`, `useChatEvents`) see `teammate_name === null` for solo agents and continue working unchanged. Team-aware consumers filter on `teammate_name !== null`.

#### New Team Lifecycle Events

| Event | Payload | Emitted When |
|-------|---------|-------------|
| `team:created` | `{ context_type, context_id, team_name, lead_name }` | Team lead creates team via TeamCreate |
| `team:teammate_spawned` | `{ context_type, context_id, team_name, teammate_name, role_description, color, model }` | Backend spawns a teammate process |
| `team:teammate_idle` | `{ context_type, context_id, team_name, teammate_name, last_activity? }` | Teammate goes idle (between turns) |
| `team:teammate_shutdown` | `{ context_type, context_id, team_name, teammate_name }` | Teammate process exits |
| `team:message` | `{ context_type, context_id, team_name, from, to, content, timestamp }` | Inter-agent message sent (or user→teammate) |
| `team:disbanded` | `{ context_type, context_id, team_name }` | Team lead calls TeamDelete |
| `team:cost_update` | `{ context_type, context_id, team_name, teammate_name, tokens_used, estimated_cost_usd }` | Periodic per-teammate token usage update |

### 3.2 Event Constants (Frontend)

```typescript
// src/lib/events.ts — NEW team event constants
export const TEAM_CREATED = "team:created";
export const TEAM_TEAMMATE_SPAWNED = "team:teammate_spawned";
export const TEAM_TEAMMATE_IDLE = "team:teammate_idle";
export const TEAM_TEAMMATE_SHUTDOWN = "team:teammate_shutdown";
export const TEAM_MESSAGE = "team:message";
export const TEAM_DISBANDED = "team:disbanded";
export const TEAM_COST_UPDATE = "team:cost_update";
```

### 3.3 Event Constants (Backend)

```rust
// src-tauri/src/application/chat_service/chat_service_types.rs — NEW
pub mod team_events {
    pub const TEAM_CREATED: &str = "team:created";
    pub const TEAM_TEAMMATE_SPAWNED: &str = "team:teammate_spawned";
    pub const TEAM_TEAMMATE_IDLE: &str = "team:teammate_idle";
    pub const TEAM_TEAMMATE_SHUTDOWN: &str = "team:teammate_shutdown";
    pub const TEAM_MESSAGE: &str = "team:message";
    pub const TEAM_DISBANDED: &str = "team:disbanded";
    pub const TEAM_COST_UPDATE: &str = "team:cost_update";
}
```

### 3.4 Event Flow Diagram

```
Solo Agent (unchanged):
  Backend → agent:run_started { context_type, context_id, teammate_name: null }
         → agent:chunk { ..., teammate_name: null }
         → agent:run_completed { ..., teammate_name: null }

Team Lead:
  Backend → team:created { team_name, lead_name }
         → team:teammate_spawned { teammate_name: "coder-1", color: "#3b82f6" }
         → team:teammate_spawned { teammate_name: "coder-2", color: "#10b981" }

Team Teammate (coder-1):
  Backend → agent:run_started { ..., teammate_name: "coder-1", team_name: "task-abc" }
         → agent:chunk { ..., teammate_name: "coder-1" }
         → agent:tool_call { ..., teammate_name: "coder-1" }
         → agent:run_completed { ..., teammate_name: "coder-1" }

Inter-Agent Message:
  Backend → team:message { from: "coder-1", to: "coder-2", content: "..." }

Team Shutdown:
  Backend → team:teammate_shutdown { teammate_name: "coder-1" }
         → team:teammate_shutdown { teammate_name: "coder-2" }
         → team:disbanded { team_name: "task-abc" }
```

### 3.5 Implementation Notes: API Field Name Drift

> **Note:** The backend event payloads use different field names than specified above. The frontend hook `useTeamEvents.ts` handles the translation. When consuming these events, use the actual backend field names:

| Brief Field Name | Actual Backend Field | Event | Translation in `useTeamEvents.ts` |
|-----------------|---------------------|-------|-----------------------------------|
| `role_description` | `role` | `team:teammate_spawned` | `payload.role` → `roleDescription` |
| `from` | `sender` | `team:message` | `payload.sender` → `from` |
| `to` | `recipient` | `team:message` | `payload.recipient` → `to` |
| `tokens_used` | `input_tokens` + `output_tokens` | `team:cost_update` | Summed: `payload.input_tokens + payload.output_tokens` → `tokens` |
| `estimated_cost_usd` | `estimated_usd` | `team:cost_update` | `payload.estimated_usd` → `costUsd` |

---

## 4. Extension Area 2: Chat UI for Team Visibility

### 4.1 Design Decision: Combined Timeline with Color Coding

**Options evaluated:**

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **A: Combined timeline** | Full chronological context, shows inter-agent flow | May be noisy with many teammates | **Selected** (with filter) |
| **B: Tabbed per-teammate** | Clean per-agent view | Loses cross-agent context, many tabs | Rejected |
| **C: Threaded view** | Groups related messages | Complex UX, doesn't match chat paradigm | Rejected |

**Selected: Combined timeline with agent-color coding + optional per-teammate filter tabs.**

### 4.2 Component Wireframe: Team Chat Timeline

```
┌─────────────────────────────────────────────────────────────────┐
│  Chat Panel (team mode)                                         │
├─────────────────────────────────────────────────────────────────┤
│  Filter: [All ●] [Lead] [🔵 coder-1] [🟢 coder-2] [🟡 coder-3] │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─ YOU ──────────────────────────────────────────────────────┐ │
│  │ Implement the user authentication flow                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌─ Lead ─────────────────────────────────────────────────────┐ │
│  │ Analyzing task... spawning team with 3 coders              │ │
│  │ ├── coder-1: auth middleware + session store               │ │
│  │ ├── coder-2: login/register endpoints                      │ │
│  │ └── coder-3: integration tests                             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌─ 🔵 coder-1 ──────────────────────────────────────────────┐ │
│  │ Starting auth middleware implementation...                  │ │
│  │ ⚙ Read src/middleware/auth.ts                              │ │
│  │ ⚙ Write src/middleware/auth.ts                             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌─ 🟢 coder-2 ──────────────────────────────────────────────┐ │
│  │ Implementing login endpoint...                              │ │
│  │ ⚙ Read src/api/auth.ts                                    │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌─ 💬 coder-1 → coder-2 ────────────────────────────────────┐ │
│  │ "Session type exported from middleware — import from there" │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌─ YOU → coder-3 ───────────────────────────────────────────┐  │
│  │ "Also test the refresh token edge case"                    │  │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  Send to: [Lead ▾]  │ [message input]              [Send ▶] │
└─────────────────────────────────────────────────────────────────┘
```

### 4.3 Message Types in Team Timeline

| Message Source | Visual Treatment | Badge/Prefix |
|---------------|-----------------|--------------|
| User (to lead) | Right-aligned, accent color | "YOU" |
| User (to specific teammate) | Right-aligned, accent color | "YOU → {name}" |
| Team lead | Left-aligned, default | "Lead" |
| Teammate agent output | Left-aligned, agent color border | "🔵 coder-1" (with color dot) |
| Inter-agent message | Left-aligned, dimmed background | "💬 {from} → {to}" |
| System/team event | Centered, muted | "coder-3 joined the team" |

### 4.4 Filter Tabs Behavior

| Tab | Shows | Default |
|-----|-------|---------|
| **All** | Everything in chronological order | Selected by default |
| **Lead** | Only lead messages + user↔lead messages | — |
| **{teammate}** | Only that teammate's output + messages to/from them | — |

Filter tabs are rendered dynamically from the active team's member list. Tab count = 1 (All) + 1 (Lead) + N teammates. Tabs use the teammate's assigned color as indicator dot.

### 4.5 Component Architecture

```
ChatPanel (existing — extended)
  ├── TeamFilterTabs (NEW — only when team active)
  │     └── renders filter chips from teamStore.teammates
  ├── ChatMessages (existing — extended)
  │     ├── UserMessage (existing)
  │     ├── AgentMessage (existing — add teammate color border)
  │     ├── TeammateMessage (NEW — inter-agent message display)
  │     └── TeamSystemEvent (NEW — "coder-3 joined", "Wave 2 validated")
  ├── TeamActivityPanel (NEW — see Section 7)
  └── ChatInput (existing — extended)
        └── TargetSelector (NEW — "Send to: [Lead ▾]" dropdown)
```

---

## 5. Extension Area 3: User-to-Teammate Messaging

### 5.1 Routing Design

Users can message the team lead OR any individual teammate. Phase 1 routes through the lead (consistent with ideation brief Section 7.3).

```
User clicks "Send to: coder-2" → types message → clicks Send
  │
  ▼
Frontend calls send_team_message(contextType, contextId, "coder-2", "message text")
  │
  ▼
Tauri IPC → Backend: TeamMessageService::send_user_message()
  │
  ▼
Backend writes message to lead's stdin pipe (interactive mode):
  "User message for coder-2: message text"
  │
  ▼
Lead receives → forwards to coder-2 via native SendMessage tool
  │
  ▼
coder-2 receives → processes → responds via SendMessage → lead relays
  │
  ▼
Backend captures relay → emits team:message event
  │
  ▼
Frontend: useTeamEvents hook → updates team message timeline
```

### 5.2 "Send To" Target Selector

```
┌─────────────────────────────────────────┐
│  Send to: [Lead         ▾]             │
│           ┌──────────────────┐          │
│           │ Lead (default)    │          │
│           │ 🔵 coder-1       │          │
│           │ 🟢 coder-2       │          │
│           │ 🟡 coder-3       │          │
│           │ All (broadcast)   │          │
│           └──────────────────┘          │
└─────────────────────────────────────────┘
```

**Default target:** Lead (safest — lead can relay or act on the message).

### 5.3 Message Types (User → Agent)

| Action | IPC Command | Backend Behavior |
|--------|-------------|-----------------|
| User → Lead | `send_agent_message` (existing) | Same as solo — message goes directly to lead's conversation |
| User → Teammate | `send_team_message` (new) | Routes through lead's stdin; lead forwards via SendMessage |
| User → All | `send_team_message` with target `"*"` | Routes through lead; lead broadcasts |

### 5.4 API Function

```typescript
// src/api/team.ts — NEW
export async function sendTeamMessage(
  contextType: ContextType,
  contextId: string,
  target: string,       // teammate name, "lead", or "*" (broadcast)
  content: string,
): Promise<void> {
  await invoke("send_team_message", {
    contextType,
    contextId,
    target,
    content,
  });
}
```

---

## 6. Extension Area 4: Team Activity Panel

### 6.1 Component Design

A collapsible sidebar/panel showing live teammate status. Appears in both ideation and execution team contexts.

### 6.2 Wireframe

```
┌─────────────────────────────────────────────────────┐
│  Team Activity                              [3/3 ●] │
│  Total: ~250K tokens | Est. $2.10                   │
├─────────────────────────────────────────────────────┤
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │ 🟢 coder-1 (sonnet)              [Running]  │   │
│  │ Auth middleware + session store               │   │
│  │ ├─ ⚙ Write src/middleware/auth.ts            │   │
│  │ └─ ~85K tokens | $0.51                       │   │
│  │ [💬 Message] [⏹ Stop]                       │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │ 🔵 coder-2 (sonnet)              [Running]  │   │
│  │ Login/register endpoints                      │   │
│  │ ├─ ⚙ Read src/api/auth.ts                   │   │
│  │ └─ ~92K tokens | $0.55                       │   │
│  │ [💬 Message] [⏹ Stop]                       │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌──────────────────────────────────────────────┐   │
│  │ 🟡 coder-3 (haiku)                  [Idle]  │   │
│  │ Integration tests                             │   │
│  │ └─ ~73K tokens | $0.04                       │   │
│  │ [💬 Message]                                 │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ─────────────────────────────────────────────────   │
│  Recent Messages (3)                                 │
│  ├─ coder-1 → coder-2: "Session type is in..."     │
│  ├─ Lead → All: "Use AppResult<T> for handlers"     │
│  └─ YOU → coder-3: "Test refresh token edge case"   │
│                                                      │
│  ┌──────────────────────────────────────────┐        │
│  │ [⏹ Stop All] [🗑 Disband Team]          │        │
│  └──────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────┘
```

### 6.3 Teammate Card States

| State | Badge | Color | Actions Available |
|-------|-------|-------|-------------------|
| Running | `[Running]` | Green dot | Message, Stop |
| Idle | `[Idle]` | Yellow dot | Message |
| Completed | `[Done]` | Gray dot | Message (view history) |
| Failed | `[Failed]` | Red dot | Message (view errors) |
| Shutdown | `[Stopped]` | Gray, dimmed | — |

### 6.4 Component Hierarchy

```
TeamActivityPanel (NEW)
  ├── TeamHeader
  │     ├── team name, member count
  │     └── aggregate cost display
  ├── TeammateCardList
  │     └── TeammateCard (per teammate)
  │           ├── color dot + name + model badge
  │           ├── role description
  │           ├── current activity (last tool call / status)
  │           ├── per-teammate cost
  │           └── action buttons (message, stop)
  ├── TeamMessageFeed (recent inter-agent messages)
  └── TeamActions (stop all, disband)
```

### 6.5 Data Source

`TeamActivityPanel` is driven by the `teamStore` (see Section 11) and refreshed via:
- `team:teammate_spawned` → add to list
- `team:teammate_idle` / `agent:run_started` / `agent:run_completed` → update status (filtered by `teammate_name`)
- `team:message` → append to message feed
- `team:cost_update` → update per-teammate cost
- `team:teammate_shutdown` / `team:disbanded` → remove / clear

---

## 7. Extension Area 5: ChatContextType Extensions

### 7.1 Decision: No New Context Types

**Team mode is a property on existing contexts, not a new context type.**

| Approach | Pros | Cons | Verdict |
|----------|------|------|---------|
| **A: New `team_ideation`, `team_execution` context types** | Clean separation | Doubles context types, breaks all context-aware code | **Rejected** |
| **B: `teamMode` flag on existing contexts** | Minimal change, backward-compatible | Slightly more conditional logic | **Selected** |

### 7.2 ChatContextConfig Extension

```typescript
// src/lib/chat-context-registry.ts — Extended config
export interface ChatContextConfig {
  // ... existing fields ...

  // NEW: Team capability flags
  supportsTeamMode: boolean;     // Can this context have a team?
  teamActivityPanelPosition: "right" | "bottom" | null;  // Where to show team panel
}

// Updated registry entries:
ideation: {
  ...existing,
  supportsTeamMode: true,
  teamActivityPanelPosition: "right",   // Ideation: team panel on right
},
task_execution: {
  ...existing,
  supportsTeamMode: true,
  teamActivityPanelPosition: "bottom",  // Execution: team panel below progress
},
// All other contexts: supportsTeamMode: false
```

### 7.3 Context Key Format (Unchanged)

Team messages use the **same context key** as solo. The store key `task_execution:{taskId}` identifies the context — team state lives in a separate `teamStore`, not in the chat context key.

```
Solo:  buildStoreKey("task_execution", "abc") → "task_execution:abc"
Team:  buildStoreKey("task_execution", "abc") → "task_execution:abc"  // SAME
       teamStore key: "team:task-abc" → { teammates, messages, ... }  // SEPARATE
```

### 7.4 ContextType Values (No Change)

```typescript
// src/types/chat-conversation.ts — UNCHANGED
export const CONTEXT_TYPE_VALUES = [
  "ideation", "task", "project", "task_execution", "review", "merge",
] as const;
```

---

## 8. Extension Area 6: Agent Resolution Changes

### 8.1 Current Resolution Flow

```rust
// chat_service_helpers.rs
fn resolve_agent(context_type, entity_status) → &str {
    // 1. Status-specific overrides (review_passed → review-chat, etc.)
    // 2. Default per context type (Ideation → orchestrator-ideation, etc.)
}
```

### 8.2 Extended Resolution: Process Mapping Lookup

The `configurable-agent-variants.md` brief defines a `process_mapping` section in `ralphx.yaml`. The `resolve_agent()` function is extended to consult this mapping.

```rust
// chat_service_helpers.rs — EXTENDED
pub fn resolve_agent(
    context_type: &ChatContextType,
    entity_status: Option<&str>,
    team_mode: bool,  // NEW parameter
) -> &'static str {
    // 1. Status-specific overrides (unchanged)
    // ... existing match logic ...

    // 2. NEW: If team mode, look up team variant from process_mapping
    if team_mode {
        if let Some(team_agent) = resolve_process_agent(
            context_type_to_process(context_type),
            "team",
        ) {
            return team_agent;
        }
    }

    // 3. Default per context type (unchanged fallback)
    // ... existing match logic ...
}
```

### 8.3 Process → Agent Mapping

| Context Type | Solo Agent (default) | Team Lead Agent (team variant) |
|-------------|---------------------|-------------------------------|
| Ideation | `orchestrator-ideation` | `ideation-team-lead` |
| TaskExecution | `ralphx-worker` | `ralphx-worker-team` |
| Review | `ralphx-reviewer` | (no team variant in Phase 1) |
| Merge | `ralphx-merger` | (no team variant) |
| Task | `chat-task` | (no team variant) |
| Project | `chat-project` | (no team variant) |

### 8.4 Resolution Decision Point

Team mode is determined from the entity's metadata before agent resolution:

```rust
// In ChatService::send_message() — EXTENDED
let team_mode = match context_type {
    ChatContextType::Ideation => {
        let session = get_ideation_session(context_id)?;
        session.team_mode.is_some()  // "research" | "debate" → true
    }
    ChatContextType::TaskExecution => {
        let task = get_task(context_id)?;
        task.metadata.get("agent_variant") == Some("team")
    }
    _ => false,
};

let agent_name = resolve_agent(&context_type, entity_status, team_mode);
```

---

## 9. Extension Area 7: Streaming from Multiple Agents

### 9.1 Problem

Currently, `ChatService` streams output from **one** agent process per context. A team has N concurrent processes (lead + teammates) each producing their own JSON-stream output. The streaming parser and event emitter must handle concurrent streams without interleaving.

### 9.2 Design: Per-Teammate Stream Isolation

Each teammate is a separate Claude CLI process with its own stdout pipe. The backend spawns N stream processors, one per process. Each processor tags events with `teammate_name` before emitting.

```
Team Lead Process (stdout) ─→ StreamProcessor("lead")
  → agent:chunk { ..., teammate_name: null }  // Lead = no teammate_name

Coder-1 Process (stdout) ─→ StreamProcessor("coder-1")
  → agent:chunk { ..., teammate_name: "coder-1", teammate_color: "#3b82f6" }

Coder-2 Process (stdout) ─→ StreamProcessor("coder-2")
  → agent:chunk { ..., teammate_name: "coder-2", teammate_color: "#10b981" }
```

### 9.3 Backend: StreamProcessor Extension

```rust
// chat_service_streaming.rs — EXTENDED

pub struct StreamProcessorConfig {
    pub context_type: ChatContextType,
    pub context_id: String,
    pub conversation_id: String,
    // NEW: Team-specific fields
    pub teammate_name: Option<String>,
    pub teammate_color: Option<String>,
    pub team_name: Option<String>,
}

/// Process a line of JSON-stream output from a Claude CLI process.
/// If teammate_name is set, tags all emitted events with it.
pub fn process_stream_line(
    line: &str,
    config: &StreamProcessorConfig,
    app_handle: &AppHandle,
) -> Result<(), StreamError> {
    let mut event = parse_stream_event(line)?;

    // Tag event with team info if this is a teammate stream
    if let Some(ref name) = config.teammate_name {
        event.insert("teammate_name", name.clone());
    }
    if let Some(ref color) = config.teammate_color {
        event.insert("teammate_color", color.clone());
    }
    if let Some(ref team) = config.team_name {
        event.insert("team_name", team.clone());
    }

    emit_event(app_handle, &event)?;
    Ok(())
}
```

### 9.4 Concurrency Model

```
AgenticClientSpawner::spawn_team(...)
  │
  ├── spawn_team_lead() → TeamLeadHandle
  │     └── StreamProcessor (teammate_name: None)
  │
  └── [Lead requests teammate spawns via MCP]
        │
        ├── spawn_teammate("coder-1") → TeammateHandle
        │     └── StreamProcessor (teammate_name: "coder-1")
        │
        ├── spawn_teammate("coder-2") → TeammateHandle
        │     └── StreamProcessor (teammate_name: "coder-2")
        │
        └── spawn_teammate("coder-3") → TeammateHandle
              └── StreamProcessor (teammate_name: "coder-3")
```

Each `TeammateHandle` holds:
- `child_process: tokio::process::Child`
- `stream_task: JoinHandle<()>` (background tokio task processing stdout)
- `stdin_pipe: ChildStdin` (for sending user messages in interactive mode)

### 9.5 Frontend: Event Demultiplexing

Frontend hooks filter events by `teammate_name`:

```typescript
// useChatEvents.ts — EXTENDED
bus.subscribe<AgentChunkPayload>("agent:chunk", (payload) => {
  const { context_type, context_id, teammate_name } = payload;

  // Build context key (same as solo — team state is separate)
  const contextKey = buildStoreKey(context_type as ContextType, context_id);

  if (teammate_name) {
    // Team mode: route to per-teammate streaming state
    teamStore.appendTeammateChunk(teammate_name, payload.text);
  } else {
    // Solo mode or lead: existing behavior
    appendStreamingText(contextKey, payload.text);
  }
});
```

---

## 10. Extension Area 8: Team State in Zustand Stores

### 10.1 New Store: `teamStore.ts`

Separate from `chatStore` — team state is orthogonal to chat context state. This prevents polluting the existing store with team-specific types.

```typescript
// src/stores/teamStore.ts — NEW

interface TeammateState {
  name: string;
  color: string;
  model: string;
  roleDescription: string;
  status: "spawning" | "running" | "idle" | "completed" | "failed" | "shutdown";
  currentActivity: string | null;
  tokensUsed: number;
  estimatedCostUsd: number;
  streamingText: string;   // Accumulates streaming chunks per teammate
}

interface TeamMessage {
  id: string;
  from: string;           // teammate name, "lead", or "user"
  to: string;             // teammate name, "lead", "user", or "*"
  content: string;
  timestamp: string;
}

interface TeamState {
  /** Active team per context key (e.g., "task_execution:abc" → team state) */
  activeTeams: Record<string, {
    teamName: string;
    leadName: string;
    teammates: Record<string, TeammateState>;  // keyed by teammate name
    messages: TeamMessage[];
    totalTokens: number;
    totalEstimatedCostUsd: number;
    createdAt: string;
  }>;
}

interface TeamActions {
  /** Initialize a team for a context */
  createTeam: (contextKey: string, teamName: string, leadName: string) => void;
  /** Add a teammate to the team */
  addTeammate: (contextKey: string, teammate: TeammateState) => void;
  /** Update a teammate's status */
  updateTeammateStatus: (contextKey: string, name: string, status: TeammateState["status"], activity?: string) => void;
  /** Append streaming text for a teammate */
  appendTeammateChunk: (contextKey: string, name: string, text: string) => void;
  /** Clear streaming text for a teammate (on run_completed) */
  clearTeammateStream: (contextKey: string, name: string) => void;
  /** Update teammate cost */
  updateTeammateCost: (contextKey: string, name: string, tokens: number, costUsd: number) => void;
  /** Add a team message */
  addTeamMessage: (contextKey: string, message: TeamMessage) => void;
  /** Remove a teammate (on shutdown) */
  removeTeammate: (contextKey: string, name: string) => void;
  /** Disband the team for a context */
  disbandTeam: (contextKey: string) => void;
  /** Get teammate list for a context */
  getTeammates: (contextKey: string) => TeammateState[];
}
```

### 10.2 chatStore Extensions (Minimal)

The existing `chatStore` only needs one addition — awareness of whether a context has a team active:

```typescript
// src/stores/chatStore.ts — ADDITIONS

interface ChatState {
  // ... existing fields ...

  /** Whether a team is active for a context key (enables team UI) */
  isTeamActive: Record<string, boolean>;  // NEW
}

interface ChatActions {
  // ... existing actions ...

  /** Set team active state for a context */
  setTeamActive: (contextKey: string, isActive: boolean) => void;  // NEW
}
```

### 10.3 Selector Examples

```typescript
// Team-aware selectors
export const selectIsTeamActive = (contextKey: string) =>
  (state: ChatState) => state.isTeamActive[contextKey] ?? false;

// teamStore selectors
export const selectTeammates = (contextKey: string) =>
  (state: TeamState) => {
    const team = state.activeTeams[contextKey];
    return team ? Object.values(team.teammates) : EMPTY_TEAMMATES;
  };

export const selectTeamMessages = (contextKey: string) =>
  (state: TeamState) => state.activeTeams[contextKey]?.messages ?? EMPTY_MESSAGES;

export const selectTeammatByName = (contextKey: string, name: string) =>
  (state: TeamState) => state.activeTeams[contextKey]?.teammates[name] ?? null;
```

### 10.4 Store Key Alignment

Both stores use the same context key format (`buildStoreKey()`):

```
chatStore.isAgentRunning["task_execution:abc"] = true     // lead is running
chatStore.isTeamActive["task_execution:abc"] = true       // team mode
teamStore.activeTeams["task_execution:abc"].teammates = { "coder-1": {...}, ... }
```

---

## 11. Extension Area 9: Tauri Command Extensions

### 11.1 New IPC Commands

| Command | Input | Returns | Description |
|---------|-------|---------|-------------|
| `get_team_status` | `contextType, contextId` | `TeamStatusResponse` | Get current team state (teammates, statuses, costs) |
| `send_team_message` | `contextType, contextId, target, content` | `void` | Send user message to teammate (routed through lead) |
| `stop_teammate` | `contextType, contextId, teammateName` | `bool` | SIGTERM a specific teammate process |
| `stop_team` | `contextType, contextId` | `bool` | Stop all teammates + lead |
| `get_team_messages` | `contextType, contextId, since?: string` | `TeamMessage[]` | Get inter-agent messages (optional since timestamp for polling) |
| `get_teammate_cost` | `contextType, contextId, teammateName` | `TeammateCostResponse` | Get token usage for a specific teammate |

### 11.2 Modified IPC Commands

| Existing Command | Change | Reason |
|-----------------|--------|--------|
| `send_agent_message` | No change needed | Messages to lead use same path as solo |
| `stop_agent` | Extended to detect team mode → calls `stop_team` | Stopping the lead should stop the whole team |
| `is_agent_running` | Extended to check team members | "Running" means lead OR any teammate is active |
| `get_agent_run_status_unified` | Extended to include team info in response | Team-aware status for UI |

### 11.3 Response Types

```rust
// chat_service_types.rs — NEW types

#[derive(Serialize, Clone)]
pub struct TeamStatusResponse {
    pub team_name: String,
    pub context_type: String,
    pub context_id: String,
    pub lead_name: String,
    pub teammates: Vec<TeammateStatusResponse>,
    pub messages: Vec<TeamMessageResponse>,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
pub struct TeammateStatusResponse {
    pub name: String,
    pub color: String,
    pub model: String,
    pub role_description: String,
    pub status: String,             // "running" | "idle" | "completed" | "failed" | "shutdown"
    pub current_activity: Option<String>,
    pub tokens_used: u64,
    pub estimated_cost_usd: f64,
}

#[derive(Serialize, Clone)]
pub struct TeamMessageResponse {
    pub id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Serialize, Clone)]
pub struct TeammateCostResponse {
    pub name: String,
    pub tokens_used: u64,
    pub estimated_cost_usd: f64,
}
```

### 11.4 Frontend API Wrappers

```typescript
// src/api/team.ts — NEW

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { ContextType } from "@/types/chat-conversation";

// Schemas
const TeammateStatusSchema = z.object({
  name: z.string(),
  color: z.string(),
  model: z.string(),
  role_description: z.string(),
  status: z.string(),
  current_activity: z.string().nullable(),
  tokens_used: z.number(),
  estimated_cost_usd: z.number(),
});

const TeamMessageSchema = z.object({
  id: z.string(),
  from: z.string(),
  to: z.string(),
  content: z.string(),
  timestamp: z.string(),
});

const TeamStatusSchema = z.object({
  team_name: z.string(),
  context_type: z.string(),
  context_id: z.string(),
  lead_name: z.string(),
  teammates: z.array(TeammateStatusSchema),
  messages: z.array(TeamMessageSchema),
  total_tokens: z.number(),
  estimated_cost_usd: z.number(),
  created_at: z.string(),
});

// API functions
export async function getTeamStatus(
  contextType: ContextType,
  contextId: string,
): Promise<z.infer<typeof TeamStatusSchema> | null> {
  const result = await invoke("get_team_status", { contextType, contextId });
  return result ? TeamStatusSchema.parse(result) : null;
}

export async function sendTeamMessage(
  contextType: ContextType,
  contextId: string,
  target: string,
  content: string,
): Promise<void> {
  await invoke("send_team_message", { contextType, contextId, target, content });
}

export async function stopTeammate(
  contextType: ContextType,
  contextId: string,
  teammateName: string,
): Promise<boolean> {
  return z.boolean().parse(
    await invoke("stop_teammate", { contextType, contextId, teammateName })
  );
}

export async function stopTeam(
  contextType: ContextType,
  contextId: string,
): Promise<boolean> {
  return z.boolean().parse(
    await invoke("stop_team", { contextType, contextId })
  );
}

export async function getTeamMessages(
  contextType: ContextType,
  contextId: string,
  since?: string,
): Promise<z.infer<typeof TeamMessageSchema>[]> {
  return z.array(TeamMessageSchema).parse(
    await invoke("get_team_messages", { contextType, contextId, since })
  );
}
```

---

## 12. New React Hooks

### 12.1 `useTeamEvents` — Team Lifecycle Event Consumer

```typescript
// src/hooks/useTeamEvents.ts — NEW

export function useTeamEvents(contextKey: string) {
  const bus = useEventBus();
  const teamStore = useTeamStore();
  const chatStore = useChatStore();

  useEffect(() => {
    const unsubs: Unsubscribe[] = [];

    // Team created
    unsubs.push(bus.subscribe("team:created", (payload) => {
      const key = buildStoreKey(payload.context_type, payload.context_id);
      if (key === contextKey) {
        teamStore.createTeam(key, payload.team_name, payload.lead_name);
        chatStore.setTeamActive(key, true);
      }
    }));

    // Teammate spawned
    unsubs.push(bus.subscribe("team:teammate_spawned", (payload) => {
      const key = buildStoreKey(payload.context_type, payload.context_id);
      if (key === contextKey) {
        teamStore.addTeammate(key, {
          name: payload.teammate_name,
          color: payload.color,
          model: payload.model,
          roleDescription: payload.role_description,
          status: "spawning",
          currentActivity: null,
          tokensUsed: 0,
          estimatedCostUsd: 0,
          streamingText: "",
        });
      }
    }));

    // Teammate running/idle (use existing agent:* events with teammate_name)
    unsubs.push(bus.subscribe("agent:run_started", (payload) => {
      if (payload.teammate_name) {
        const key = buildStoreKey(payload.context_type, payload.context_id);
        if (key === contextKey) {
          teamStore.updateTeammateStatus(key, payload.teammate_name, "running");
        }
      }
    }));

    unsubs.push(bus.subscribe("agent:run_completed", (payload) => {
      if (payload.teammate_name) {
        const key = buildStoreKey(payload.context_type, payload.context_id);
        if (key === contextKey) {
          teamStore.updateTeammateStatus(key, payload.teammate_name, "idle");
          teamStore.clearTeammateStream(key, payload.teammate_name);
        }
      }
    }));

    // Teammate idle
    unsubs.push(bus.subscribe("team:teammate_idle", (payload) => {
      const key = buildStoreKey(payload.context_type, payload.context_id);
      if (key === contextKey) {
        teamStore.updateTeammateStatus(key, payload.teammate_name, "idle", payload.last_activity);
      }
    }));

    // Inter-agent messages
    unsubs.push(bus.subscribe("team:message", (payload) => {
      const key = buildStoreKey(payload.context_type, payload.context_id);
      if (key === contextKey) {
        teamStore.addTeamMessage(key, {
          id: `msg-${Date.now()}`,
          from: payload.from,
          to: payload.to,
          content: payload.content,
          timestamp: payload.timestamp,
        });
      }
    }));

    // Cost updates
    unsubs.push(bus.subscribe("team:cost_update", (payload) => {
      const key = buildStoreKey(payload.context_type, payload.context_id);
      if (key === contextKey) {
        teamStore.updateTeammateCost(
          key, payload.teammate_name,
          payload.tokens_used, payload.estimated_cost_usd,
        );
      }
    }));

    // Teammate shutdown
    unsubs.push(bus.subscribe("team:teammate_shutdown", (payload) => {
      const key = buildStoreKey(payload.context_type, payload.context_id);
      if (key === contextKey) {
        teamStore.updateTeammateStatus(key, payload.teammate_name, "shutdown");
      }
    }));

    // Team disbanded
    unsubs.push(bus.subscribe("team:disbanded", (payload) => {
      const key = buildStoreKey(payload.context_type, payload.context_id);
      if (key === contextKey) {
        teamStore.disbandTeam(key);
        chatStore.setTeamActive(key, false);
      }
    }));

    // Streaming chunks for teammates
    unsubs.push(bus.subscribe("agent:chunk", (payload) => {
      if (payload.teammate_name) {
        const key = buildStoreKey(payload.context_type, payload.context_id);
        if (key === contextKey) {
          teamStore.appendTeammateChunk(key, payload.teammate_name, payload.text);
        }
      }
    }));

    return () => unsubs.forEach(u => u());
  }, [bus, contextKey, teamStore, chatStore]);
}
```

### 12.2 `useTeamStatus` — TanStack Query Hook

```typescript
// src/hooks/useTeamStatus.ts — NEW

export function useTeamStatus(contextType: ContextType, contextId: string) {
  const isTeamActive = useChatStore(selectIsTeamActive(
    buildStoreKey(contextType, contextId)
  ));

  return useQuery({
    queryKey: teamKeys.status(contextType, contextId),
    queryFn: () => getTeamStatus(contextType, contextId),
    enabled: isTeamActive,
    refetchInterval: 5000,  // Poll every 5s for cost updates
  });
}

// Query keys
export const teamKeys = {
  all: ["teams"] as const,
  status: (contextType: ContextType, contextId: string) =>
    [...teamKeys.all, "status", contextType, contextId] as const,
  messages: (contextType: ContextType, contextId: string) =>
    [...teamKeys.all, "messages", contextType, contextId] as const,
};
```

---

## 13. Component Impact Analysis

### 13.1 Modified Components

| Component | Current | Team Extension | Complexity |
|-----------|---------|---------------|------------|
| `ChatPanel.tsx` | Single-agent message timeline | Add `TeamFilterTabs`, color-coded messages, `TargetSelector` in input | Medium |
| `IntegratedChatPanel.tsx` | Single-agent streaming + tool calls | Add team mode detection, conditional `TeamActivityPanel` rendering | Medium |
| `ChatInput` (inside ChatPanel) | Sends to one agent | Add "Send to" dropdown (`TargetSelector`) | Low |
| `MessageItem.tsx` | Renders one agent's messages | Add teammate color border, name badge, inter-agent message style | Low |
| `ExecutionTaskDetail.tsx` | Single-worker progress | Add multi-track per-teammate progress (if team execution) | Medium |

### 13.2 New Components

| Component | Purpose | Parent |
|-----------|---------|--------|
| `TeamActivityPanel.tsx` | Live teammate status panel | IntegratedChatPanel / TaskDetailPanel |
| `TeammateCard.tsx` | Individual teammate status card | TeamActivityPanel |
| `TeamFilterTabs.tsx` | Per-teammate message filter tabs | ChatPanel |
| `TargetSelector.tsx` | "Send to" dropdown for choosing message recipient | ChatInput |
| `TeamMessageBubble.tsx` | Inter-agent message display | ChatMessages |
| `TeamSystemEvent.tsx` | Team lifecycle event display (join/leave/wave) | ChatMessages |
| `TeamCostDisplay.tsx` | Aggregate + per-teammate cost breakdown | TeamActivityPanel |

### 13.3 Modified Hooks

| Hook | Current | Team Extension |
|------|---------|---------------|
| `useAgentEvents.ts` | Filters by `activeConversationId` | Extended: also process events with `teammate_name` for team state |
| `useChatEvents.ts` | Streaming text + tool call accumulation | Extended: route teammate chunks to `teamStore` instead of main stream |
| `useChatActions.ts` | `send`, `queue`, `stop` actions | Extended: add `sendTeamMessage`, `stopTeammate`, `stopTeam` |
| `useChatPanelContext.ts` | Resolves context type + key | Extended: also resolve `isTeamActive` for conditional UI |

### 13.4 New Hooks

| Hook | Purpose |
|------|---------|
| `useTeamEvents.ts` | Consumes `team:*` events, updates `teamStore` |
| `useTeamStatus.ts` | TanStack Query hook for polling team status |
| `useTeamActions.ts` | Team-specific mutation hooks (send message, stop teammate, etc.) |

### 13.5 Modified Stores

| Store | Changes |
|-------|---------|
| `chatStore.ts` | Add `isTeamActive: Record<string, boolean>` + `setTeamActive()` action |

### 13.6 New Stores

| Store | Purpose |
|-------|---------|
| `teamStore.ts` | Per-context team state: teammates, statuses, costs, messages, streaming chunks |

### 13.7 Modified API Layer

| File | Changes |
|------|---------|
| `src/api/chat.ts` | No changes needed (solo messages still go through existing `sendAgentMessage`) |

### 13.8 New API Layer

| File | Purpose |
|------|---------|
| `src/api/team.ts` | New Tauri invoke wrappers for team commands (`getTeamStatus`, `sendTeamMessage`, `stopTeammate`, etc.) |

### 13.9 Modified Types

| File | Changes |
|------|---------|
| `src/types/chat-conversation.ts` | No changes to ContextType values |
| `src/lib/events.ts` | Add 7 new `team:*` event constants |
| `src/lib/chat-context-registry.ts` | Add `supportsTeamMode` and `teamActivityPanelPosition` to `ChatContextConfig` |

---

## 14. Backend Service Architecture

### 14.1 New Service: `TeamStateTracker`

Tracks active teams, their teammates, processes, and costs.

```rust
// src-tauri/src/application/team_state_tracker.rs — NEW

pub struct TeamStateTracker {
    /// Active teams keyed by "{context_type}:{context_id}"
    teams: Arc<RwLock<HashMap<String, TeamState>>>,
}

pub struct TeamState {
    pub team_name: String,
    pub context_type: ChatContextType,
    pub context_id: String,
    pub lead_name: String,
    pub lead_handle: TeamLeadHandle,
    pub teammates: HashMap<String, TeammateState>,
    pub messages: Vec<TeamMessage>,
    pub created_at: String,
}

pub struct TeammateState {
    pub name: String,
    pub color: String,
    pub model: String,
    pub role_description: String,
    pub status: TeammateStatus,
    pub process_handle: TeammateHandle,
    pub tokens_used: u64,
    pub estimated_cost_usd: f64,
}

pub struct TeammateHandle {
    pub child: tokio::process::Child,
    pub stdin: tokio::process::ChildStdin,
    pub stream_task: JoinHandle<()>,
}

pub enum TeammateStatus {
    Spawning,
    Running,
    Idle,
    Completed,
    Failed,
    Shutdown,
}
```

### 14.2 TeamStateTracker API

| Method | Purpose |
|--------|---------|
| `create_team(context_key, team_name, lead_handle)` | Register new team |
| `add_teammate(context_key, teammate_state)` | Register spawned teammate |
| `update_teammate_status(context_key, name, status)` | Update teammate state |
| `get_team_status(context_key)` → `TeamStatusResponse` | Query current state |
| `send_user_message(context_key, target, content)` | Route user message through lead |
| `stop_teammate(context_key, name)` | SIGTERM a teammate |
| `stop_team(context_key)` | SIGTERM all members |
| `remove_teammate(context_key, name)` | Unregister (after shutdown) |
| `disband_team(context_key)` | Remove team state |
| `get_team_messages(context_key, since)` | Get inter-agent messages |

### 14.3 Integration with ChatService

```rust
// ChatService::send_message() — EXTENDED

// After resolving agent and determining team_mode:
if team_mode {
    // Spawn team lead in INTERACTIVE mode (no -p flag)
    let lead_handle = self.spawn_team_lead(
        &agent_name, &context_type, &context_id, &team_config,
    ).await?;

    // Register team in TeamStateTracker
    self.team_tracker.create_team(
        &context_key, &team_name, lead_handle,
    ).await?;

    // Emit team:created event
    self.emit_team_event("team:created", &TeamCreatedPayload { ... })?;

    // Team lead will request teammate spawns via MCP (request_teammate_spawn)
    // Backend validates and spawns via TeamStateTracker
} else {
    // Existing solo agent flow — unchanged
    self.spawn_single_agent(...).await?;
}
```

### 14.4 MCP Tool: `request_teammate_spawn`

When the team lead requests a teammate via MCP, the backend validates and spawns:

```
Lead calls request_teammate_spawn MCP tool
  │
  ▼
MCP Server: POST /api/team/spawn → Tauri backend
  │
  ▼
TeamStateTracker::spawn_teammate()
  ├── Validate against team_constraints (tools, model, count)
  ├── Build CLI args with team flags (--agent-id, --team-name, --parent-session-id, etc.)
  ├── Spawn Claude CLI process in interactive mode
  ├── Start StreamProcessor for teammate (tags events with teammate_name)
  ├── Register in TeamStateTracker
  └── Emit team:teammate_spawned event
  │
  ▼
Return to lead: { spawned: true, teammate_id: "coder-1" }
```

### 14.5 Inter-Agent Message Capture

Claude Code's native `SendMessage` tool handles inter-agent messaging internally. The backend captures these by parsing the team lead's JSON-stream output:

```
Lead's stdout → StreamProcessor
  │
  ├── type: "result" (regular agent output) → normal event emission
  │
  └── type: "agent:message" (custom) OR tool_call with name "SendMessage"
        → Parse from/to/content
        → Store in TeamStateTracker.messages
        → Emit team:message event → frontend
```

---

## 15. Event Flow Diagrams

### 15.1 Team Startup Sequence

```
User selects "Team" mode → clicks "Start Session"
  │
  ▼
Frontend: sendAgentMessage(contextType, contextId, content)
  │ (same as solo — team mode detected from session/task metadata)
  ▼
Backend: ChatService.send_message()
  │
  ├── resolve_agent(..., team_mode=true) → "ideation-team-lead" / "ralphx-worker-team"
  ├── Build command WITHOUT -p flag (interactive mode)
  ├── Spawn lead process
  │
  ├── Emit: agent:run_started { ..., teammate_name: null, team_name: "task-abc" }
  │         → Frontend: setAgentRunning(contextKey, true)
  │
  ├── Register team: TeamStateTracker.create_team()
  ├── Emit: team:created { team_name, lead_name }
  │         → Frontend: teamStore.createTeam(), chatStore.setTeamActive(true)
  │
  ▼
Lead analyzes task → calls request_teammate_spawn via MCP
  │
  ▼
Backend: TeamStateTracker.spawn_teammate()
  ├── Validate constraints
  ├── Spawn teammate process
  ├── Start StreamProcessor(teammate_name: "coder-1")
  ├── Emit: team:teammate_spawned { teammate_name, color, model, role_description }
  │         → Frontend: teamStore.addTeammate()
  │
  ▼
Teammate starts working → emits agent:* events with teammate_name
  │
  ▼
Frontend: useTeamEvents routes events to teamStore
         → TeamActivityPanel re-renders with live status
         → ChatPanel shows color-coded messages in timeline
```

### 15.2 User-to-Teammate Message Flow

```
User selects "coder-2" in TargetSelector → types message → Send
  │
  ▼
Frontend: sendTeamMessage(contextType, contextId, "coder-2", "Test refresh tokens")
  │
  ▼
Backend: TeamStateTracker.send_user_message()
  ├── Get lead's stdin pipe
  ├── Write: "User message for coder-2: Test refresh tokens\n"
  ├── Store message: { from: "user", to: "coder-2", ... }
  ├── Emit: team:message { from: "user", to: "coder-2", content: "..." }
  │         → Frontend: teamStore.addTeamMessage()
  │         → ChatPanel shows "YOU → coder-2: Test refresh tokens"
  │
  ▼
Lead receives stdin message → calls SendMessage(recipient: "coder-2", ...)
  │
  ▼
coder-2 receives → processes → responds via SendMessage to lead
  │
  ▼
Lead's stream includes response → backend captures
  ├── Store message: { from: "coder-2", to: "user", ... }
  ├── Emit: team:message { from: "coder-2", to: "user", content: "..." }
  │         → Frontend shows response in timeline
```

### 15.3 Team Shutdown Sequence

```
User clicks "Stop All" or lead sends shutdown_request to all
  │
  ├── [User-initiated] Frontend: stopTeam(contextType, contextId)
  │     → Backend: TeamStateTracker.stop_team()
  │       → SIGTERM all teammate processes
  │       → SIGTERM lead process
  │
  └── [Lead-initiated] Lead calls SendMessage(type: shutdown_request) to each
        → Each teammate responds with shutdown_response(approve: true)
        → Teammate processes exit
        → Backend detects exit → emit team:teammate_shutdown
        → Lead calls TeamDelete
        → Backend detects lead exit → emit team:disbanded
  │
  ▼
Frontend: team:teammate_shutdown → teamStore.updateTeammateStatus("shutdown")
         team:disbanded → teamStore.disbandTeam(), chatStore.setTeamActive(false)
         → TeamActivityPanel clears
         → ChatPanel returns to solo mode
```

---

## 16. Migration & Phased Rollout

### Phase 1: Event System + Store Foundation

| Task | Files | Estimate |
|------|-------|----------|
| Add `team:*` event constants to frontend + backend | `events.ts`, `chat_service_types.rs` | Small |
| Add `teammate_name`/`team_name` fields to existing event payloads | `chat_service_streaming.rs`, event payload structs | Small |
| Create `teamStore.ts` | New file | Medium |
| Add `isTeamActive` to `chatStore.ts` | Existing file | Small |
| Create `useTeamEvents.ts` hook | New file | Medium |
| Create `src/api/team.ts` API wrappers | New file | Small |

### Phase 2: Backend Team Management

| Task | Files | Estimate |
|------|-------|----------|
| Create `TeamStateTracker` service | New Rust module | Large |
| Extend `ChatService.send_message()` for team mode | `mod.rs`, `chat_service_context.rs` | Medium |
| Extend `resolve_agent()` for team variants | `chat_service_helpers.rs` | Small |
| Implement `StreamProcessorConfig` team extensions | `chat_service_streaming.rs` | Medium |
| Register new Tauri IPC commands | `unified_chat_commands.rs` or new `team_commands.rs` | Medium |
| MCP tool: `request_teammate_spawn` | MCP server + Tauri HTTP handler | Medium |

### Phase 3: UI Components

| Task | Files | Estimate |
|------|-------|----------|
| `TeamActivityPanel` + `TeammateCard` | New components | Medium |
| `TeamFilterTabs` for ChatPanel | New component | Small |
| `TargetSelector` (Send to dropdown) | New component | Small |
| `TeamMessageBubble` + `TeamSystemEvent` | New components | Small |
| Extend `ChatPanel` for team mode | Existing + new children | Medium |
| Extend `ExecutionTaskDetail` for multi-track | Existing | Medium |
| `TeamCostDisplay` | New component | Small |

### Phase 4: Polish & Integration

| Task | Files | Estimate |
|------|-------|----------|
| Per-teammate cost tracking integration | Backend + frontend | Medium |
| Team resume in RECOVER phase | Backend + MCP tools | Large |
| Debate mode side-by-side UI | New component | Medium |
| Narrow viewport responsive layout | CSS/component | Small |

---

## 17. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Event storm from N teammates** | High | Rate-limit `team:cost_update` to every 5s. Batch `agent:chunk` events in frontend (debounce). |
| **Store bloat from team messages** | Medium | Cap `teamStore.messages` per context (e.g., last 200). Older messages available via `get_team_messages` API. |
| **Concurrent stream parsing race conditions** | High | Each teammate's `StreamProcessor` runs in its own tokio task with independent state. No shared mutable parser state. |
| **Interactive mode stdin injection** | Medium | Well-defined message format. Lead's system prompt specifies how to parse user-forwarded messages. |
| **UI complexity with many teammates** | Medium | Default max 5 teammates. Filter tabs allow focusing on one. Collapsible `TeamActivityPanel`. |
| **Backward compatibility break** | Low | All `teammate_name` fields are optional/nullable. Solo flows see `null` and behave identically to today. |
| **Memory usage per teammate** | Medium | Each teammate = 1 process + 1 stream task + 1 store entry. Cap at 5-8 teammates via `team_constraints`. |

---

## 18. Open Questions

| # | Question | Options | Recommendation |
|---|----------|---------|----------------|
| 1 | Should the `TeamActivityPanel` replace the plan browser in ideation or sit alongside it? | Replace / Alongside / Tabbed | **Alongside** — plan browser shows plan, team panel shows live activity. Both are needed simultaneously. |
| 2 | Should inter-agent messages be persisted to the DB or only held in memory? | DB (survives restart) / Memory (simpler) | **DB** — needed for team resume in RECOVER phase. Use existing `chat_messages` table with a `team_message` flag or a dedicated `team_messages` table. |
| 3 | How should token cost estimation work for teammates? | Parse from stream / Track via task_completed events / MCP polling | **Parse from `agent:task_completed` events** which include `total_tokens`. Supplement with periodic `team:cost_update` events from backend polling. |
| 4 | Should teammate streaming text show in the main chat timeline or only in TeammateCard? | Timeline only / Card only / Both | **Both** — Timeline shows final messages; TeammateCard shows live streaming with collapse. Avoids overwhelming timeline with partial text. |

---

## 19. Dependencies

| Dependency | Status | Brief |
|------------|--------|-------|
| Agent Teams system card | Complete | `docs/agent-teams-system-card.md` |
| Ideation integration brief | Complete (v5) | `docs/product-briefs/agent-teams-ideation-integration.md` |
| Worker integration brief | Complete (v3) | `docs/product-briefs/agent-teams-worker-integration.md` |
| Configurable agent variants | Complete (v5) | `docs/product-briefs/configurable-agent-variants.md` |
| Interactive mode support in `ClaudeCodeClient` | Required | Currently only supports `-p` mode. Team agents need stdin pipe for messaging. |
| Claude Code Agent Teams feature | Experimental | Enabled via `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` |

---

## Appendix A: File Index

### New Files

| File | Purpose |
|------|---------|
| `src/stores/teamStore.ts` | Team state management (Zustand + immer) |
| `src/api/team.ts` | Tauri invoke wrappers for team commands |
| `src/hooks/useTeamEvents.ts` | Team lifecycle event consumer |
| `src/hooks/useTeamStatus.ts` | TanStack Query hook for team status polling |
| `src/hooks/useTeamActions.ts` | Team mutation hooks (send message, stop, etc.) |
| `src/components/Chat/TeamActivityPanel.tsx` | Live teammate status panel |
| `src/components/Chat/TeammateCard.tsx` | Individual teammate card |
| `src/components/Chat/TeamFilterTabs.tsx` | Per-teammate message filter |
| `src/components/Chat/TargetSelector.tsx` | "Send to" dropdown |
| `src/components/Chat/TeamMessageBubble.tsx` | Inter-agent message display |
| `src/components/Chat/TeamSystemEvent.tsx` | Team lifecycle event display |
| `src/components/Chat/TeamCostDisplay.tsx` | Cost breakdown display |
| `src-tauri/src/application/team_state_tracker.rs` | Team state tracking service |
| `src-tauri/src/commands/team_commands.rs` | Tauri IPC command handlers for teams |

### Modified Files

| File | Changes |
|------|---------|
| `src/lib/events.ts` | Add 7 `team:*` event constants |
| `src/lib/chat-context-registry.ts` | Add `supportsTeamMode`, `teamActivityPanelPosition` |
| `src/stores/chatStore.ts` | Add `isTeamActive` state + `setTeamActive` action |
| `src/hooks/useAgentEvents.ts` | Handle `teammate_name` in existing events |
| `src/hooks/useChatEvents.ts` | Route teammate chunks to teamStore |
| `src/components/Chat/ChatPanel.tsx` | Add TeamFilterTabs, TargetSelector, team message styles |
| `src/components/Chat/IntegratedChatPanel.tsx` | Add team mode detection, TeamActivityPanel |
| `src/components/Chat/MessageItem.tsx` | Add teammate color border, name badge |
| `src/components/tasks/detail-views/ExecutionTaskDetail.tsx` | Multi-track teammate progress |
| `src-tauri/src/application/chat_service/chat_service_types.rs` | Add team event constants + payload structs |
| `src-tauri/src/application/chat_service/chat_service_streaming.rs` | Add `StreamProcessorConfig` with team fields |
| `src-tauri/src/application/chat_service/chat_service_helpers.rs` | Extend `resolve_agent()` with `team_mode` param |
| `src-tauri/src/application/chat_service/mod.rs` | Team mode detection in `send_message()` |
| `src-tauri/src/commands/unified_chat_commands.rs` | Extend `stop_agent`, `is_agent_running` for team awareness |
