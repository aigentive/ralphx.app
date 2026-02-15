# Product Brief: Agent Teams at Orchestrator-Ideation Level

**Status:** DRAFT v2
**Author:** product-researcher
**Date:** 2026-02-15
**Scope:** Integrating Claude Code Agent Teams into RalphX's ideation workflow
**Revision:** Incorporates user feedback — dynamic team roles, direct teammate messaging, team-recommended for complex work

---

## 1. Executive Summary

This brief proposes integrating Claude Code Agent Teams into RalphX's orchestrator-ideation workflow. Agent teams enable multiple independent Claude instances to collaborate — sharing findings, debating approaches, and challenging each other's conclusions — producing higher-quality ideation outcomes than the current single-orchestrator + subagent model.

**Key principles:**
- **ADDITIVE:** Current single-agent flows preserved as default. Teams are opt-in per session.
- **Team-recommended for complex work:** For projects like RalphX, using more compute on complex tasks is the right call. Team mode should be the recommended (not hidden) option for complex features.
- **Dynamic team composition:** The team lead agent decides what roles to spawn based on the task — not limited to predefined YAML configs. Configuration provides *constraints* (max teammates, tool ceilings, model caps), not rigid role definitions.
- **Full user control:** Users can message both the team lead AND individual teammates directly via the RalphX UI.

---

## 2. Current State

### How Ideation Works Today

The orchestrator-ideation agent (Sonnet) runs a 6-phase gated workflow:

| Phase | Name | What Happens |
|-------|------|-------------|
| 0 | RECOVER | Load existing session state (plan, proposals, parent context) |
| 1 | UNDERSTAND | Parse user intent, determine complexity |
| 2 | EXPLORE | Launch up to 3 parallel Explore subagents for codebase research |
| 3 | PLAN | Design implementation approach, create plan artifact via MCP |
| 4 | CONFIRM | Present plan to user, get explicit approval |
| 5 | PROPOSE | Create task proposals linked to plan |
| 6 | FINALIZE | Dependency analysis, critical path, hand off |

### Current Limitations

| Limitation | Impact |
|------------|--------|
| **Subagents report to orchestrator only** | No cross-pollination between Explore agents — findings are siloed |
| **Single planning perspective** | Plan agent evaluates from one viewpoint; no debate or competing approaches |
| **No adversarial challenge** | Proposals are never stress-tested by an independent agent |
| **Sequential synthesis** | Orchestrator must sequentially process all subagent results before planning |
| **Context window pressure** | Orchestrator absorbs all subagent results into one context, risking overflow on complex features |

### Current CLI Spawning

The orchestrator-ideation is spawned via `ClaudeCodeClient` with:
- `--append-system-prompt-file` pointing to `ralphx-plugin/agents/orchestrator-ideation.md`
- `--mcp-config` with MCP tools filtered to: `*_task_proposal`, `*_plan_artifact`
- `--tools "Read,Grep,Glob,Bash,WebFetch,WebSearch,Task"` (no Write/Edit)
- `--model sonnet`
- MCP server passes `--agent-type orchestrator-ideation` for tool filtering

---

## 3. Proposed Integration

### 3.1 Team-Enhanced Ideation Modes

Two new ideation modes, selectable per session:

| Mode | Agents | Best For | Token Cost | Default? |
|------|--------|----------|------------|----------|
| **Solo** (current default) | 1 orchestrator + subagents | Simple features, quick tasks, bug fixes | 1x (baseline) | Default for simple tasks |
| **Research Team** | 1 lead + dynamically-chosen teammates | Complex features, cross-layer work | 3-4x | **Recommended** for complex tasks |
| **Debate Team** | 1 lead + dynamically-chosen perspective teammates | Architectural decisions, high-stakes planning | 4-6x | **Recommended** for architecture decisions |

**Team composition is dynamic:** The lead agent analyzes the task and decides what specialist roles to create. A frontend-heavy feature might get 2 UI specialists + 1 state management specialist. An infrastructure feature might get a DB specialist + a config specialist + a migration specialist. The lead constructs each teammate's role, prompt, and tool set on the fly.

### 3.2 Research Team Mode

**When:** Complex features touching multiple domains (frontend + backend + database).

**Architecture:**
```
User (can message lead or ANY teammate directly)
  │
  ▼
Ideation Lead (orchestrator-ideation-team-lead — Opus)
  │  1. Analyzes task → decides what specialist roles are needed
  │  2. Creates team, spawns teammates with dynamic roles/prompts
  │  3. Synthesizes findings, creates plan artifact + proposals
  │
  ├──▶ [Dynamic Role A] (Sonnet)
  │     Role, prompt, and focus area decided by lead based on task
  │     Example: "React state management specialist" or "API design reviewer"
  │
  ├──▶ [Dynamic Role B] (Sonnet)
  │     Example: "Rust service layer specialist" or "Database schema analyst"
  │
  └──▶ [Dynamic Role C] (Sonnet)
        Example: "Integration point specialist" or "MCP config analyst"
```

**Example: Lead analyzing "Add real-time collaboration" task:**
```
Lead thinks: "This touches WebSocket infra, React state sync, and database events.
I'll spawn:
  1. 'realtime-transport-researcher' — investigate WebSocket vs SSE vs polling options
  2. 'react-state-sync-researcher' — analyze existing store patterns for real-time updates
  3. 'event-system-researcher' — explore DB triggers, event emission, and MCP integration"
```

**Workflow changes to 6-phase:**

| Phase | Solo (unchanged) | Research Team |
|-------|-----------------|---------------|
| UNDERSTAND | Parse intent | Parse intent + **decide team composition dynamically** |
| EXPLORE | 3 parallel Explore subagents | Dynamic specialist teammates that share findings via messaging |
| PLAN | 1 Plan subagent | Lead synthesizes teammate findings, creates plan with cross-domain awareness |
| Other phases | No change | No change — lead handles CONFIRM, PROPOSE, FINALIZE |

**Key advantages:**
- Teammates can challenge each other's findings (something subagents cannot do)
- Lead tailors team composition to the specific task — no wasted specialists
- User can intervene with any teammate directly via UI

### 3.3 Debate Team Mode

**When:** Architectural decisions where multiple valid approaches exist.

**Architecture:**
```
User (can message lead or ANY advocate directly — e.g., "Advocate B, what about caching?")
  │
  ▼
Ideation Lead (orchestrator-ideation-team-lead — Opus, delegate mode)
  │  1. Identifies competing approaches from task analysis
  │  2. Dynamically creates advocate roles + prompts for each approach
  │  3. Always includes a devil's advocate role
  │  4. Moderates debate, synthesizes winning approach
  │
  ├──▶ [Dynamic Advocate A] (Sonnet)
  │     Lead-generated prompt: "Advocate for [approach]. Research evidence.
  │     Challenge other approaches with concrete data."
  │
  ├──▶ [Dynamic Advocate B] (Sonnet)
  │     Lead-generated prompt: specific to the alternative approach
  │
  └──▶ [Dynamic Devil's Advocate] (Sonnet)
        Lead-generated prompt: "Stress-test ALL proposed approaches.
        Find weaknesses, edge cases, scalability issues."
```

**Example: Lead analyzing "Add state management" task:**
```
Lead thinks: "There are 3 viable approaches: extend Zustand stores, add TanStack Query
cache, or introduce a real-time sync layer. I'll spawn:
  1. 'zustand-advocate' — argue for extending existing Zustand+immer pattern
  2. 'query-cache-advocate' — argue for TanStack Query as source of truth
  3. 'sync-layer-advocate' — argue for new real-time sync abstraction
  4. 'architecture-critic' — stress-test all three for complexity, perf, maintainability"
```

**Workflow:**
1. Lead analyzes task → identifies competing approaches
2. Lead dynamically creates advocate roles with targeted prompts
3. Each advocate researches the codebase from their perspective
4. Advocates share findings and challenge each other via messaging
5. Devil's advocate stress-tests all proposals
6. Lead synthesizes the debate into a plan artifact with justified decision
7. Lead presents plan to user with debate summary
8. User can intervene at any point — message any advocate directly

### 3.4 Agent Variants and Dynamic Composition

**Only ONE new predefined agent is required:** the team lead.

| Agent | Model | Purpose |
|-------|-------|---------|
| `orchestrator-ideation-team-lead` | Opus | Creates team, dynamically decides roles, spawns teammates with custom prompts, synthesizes findings, creates plan/proposals. Has delegate mode active. |

**All other teammates are dynamically created by the lead.** The lead uses the Task tool to spawn each teammate with:
- A custom `prompt` describing the role, focus area, and expected output
- A `subagent_type` (e.g., `general-purpose` for full tools, or a custom type)
- A `model` selection (within the configured ceiling)
- The `team_name` parameter to join the team

**Predefined templates (optional, not mandatory):**

The lead MAY use predefined agent prompt templates from `ralphx-plugin/agents/` as a starting point, but is NOT required to. Templates are presets for common patterns:

| Template | Focus | Use When |
|----------|-------|----------|
| `ideation-specialist-frontend.md` | React/TS/Tailwind research | Lead decides frontend research is needed |
| `ideation-specialist-backend.md` | Rust/Tauri/SQLite research | Lead decides backend research is needed |
| `ideation-specialist-infra.md` | DB schema, MCP, config, git | Lead decides infra research is needed |
| `ideation-advocate.md` | Generic advocacy template | Lead creates debate advocates |
| `ideation-critic.md` | Adversarial stress-testing | Lead creates devil's advocate |

**All ideation teammates are READ-ONLY** (no Write/Edit) — enforced by the constraint configuration, not by predefined agent configs.

---

## 4. Configuration Schema

### 4.1 Design Principle: Constraints, Not Rigid Definitions

**YAML provides guardrails. The lead agent decides composition.**

Two configuration modes:

| Mode | Default? | Who decides team composition? | YAML provides |
|------|----------|------------------------------|---------------|
| **Dynamic** | Yes | Lead agent, based on task analysis | Constraints: max teammates, tool ceiling, model caps, MCP tool ceiling |
| **Constrained** | Opt-in | Lead agent, but limited to predefined agent configs | Rigid role definitions from YAML (for security-sensitive workflows) |

### 4.2 Session-Level Configuration

User selects team mode when creating an ideation session:

```typescript
interface IdeationSessionConfig {
  // Existing fields...
  teamMode: 'solo' | 'research' | 'debate';  // NEW - default: 'solo'
  teamConfig?: {
    maxTeammates: number;       // 2-5, default: 3
    modelCeiling: string;       // max model any teammate can use, default: 'sonnet'
    costBudget?: number;        // Max USD for team session (optional)
    compositionMode: 'dynamic' | 'constrained';  // default: 'dynamic'
  };
}
```

### 4.3 ralphx.yaml — Constraint-Based Configuration

```yaml
agents:
  # Existing orchestrator-ideation unchanged...

  # === TEAM LEAD (the only predefined agent required) ===
  - name: orchestrator-ideation-team-lead
    system_prompt_file: ralphx-plugin/agents/orchestrator-ideation-team-lead.md
    model: opus
    tools:
      extends: base_tools
      include: [Task]
    mcp_tools:
      - create_task_proposal
      - update_task_proposal
      - delete_task_proposal
      - list_session_proposals
      - create_plan_artifact
      - update_plan_artifact
      - get_session_plan
      - get_plan_artifact
      - analyze_session_dependencies
      - create_child_session
      - get_parent_session_context
    # Team tools (TeamCreate, SendMessage, TaskCreate, etc.) are
    # built into Claude Code, not MCP — available when agent teams enabled

  # === TEAMMATE CONSTRAINTS (applied to ALL dynamically-spawned teammates) ===
  team_constraints:
    ideation:
      max_teammates: 5
      model_ceiling: sonnet          # Teammates cannot exceed this model
      tool_ceiling:                  # Maximum tools ANY teammate can access
        allowed: [Read, Grep, Glob, WebFetch, WebSearch, Task]
        denied: [Write, Edit]        # Enforced: all ideation teammates are read-only
      mcp_tool_ceiling:              # Maximum MCP tools ANY teammate can access
        - get_session_plan
        - list_session_proposals
        - get_plan_artifact
      # Lead's requested tools for each teammate are INTERSECTED with these ceilings
      bash_allowed: true             # Some teammates may need Bash (infra research)

  # === OPTIONAL: Predefined templates for constrained mode ===
  team_templates:
    ideation:
      - name: frontend-specialist
        system_prompt_file: ralphx-plugin/agents/ideation-specialist-frontend.md
        model: sonnet
        tools: [Read, Grep, Glob, WebFetch, WebSearch]
        mcp_tools: [get_session_plan, list_session_proposals]

      - name: backend-specialist
        system_prompt_file: ralphx-plugin/agents/ideation-specialist-backend.md
        model: sonnet
        tools: [Read, Grep, Glob, WebFetch, WebSearch]
        mcp_tools: [get_session_plan, list_session_proposals]

      - name: infra-specialist
        system_prompt_file: ralphx-plugin/agents/ideation-specialist-infra.md
        model: sonnet
        tools: [Read, Grep, Glob, Bash, WebFetch, WebSearch]
        mcp_tools: [get_session_plan, list_session_proposals]

      - name: advocate
        system_prompt_file: ralphx-plugin/agents/ideation-advocate.md
        model: sonnet
        tools: [Read, Grep, Glob, WebFetch, WebSearch]

      - name: critic
        system_prompt_file: ralphx-plugin/agents/ideation-critic.md
        model: sonnet
        tools: [Read, Grep, Glob, WebFetch, WebSearch]
```

### 4.4 How Dynamic Mode Works

1. Lead receives task from user
2. Lead analyzes task in UNDERSTAND phase — determines what roles are needed
3. Lead spawns each teammate via Task tool, constructing:
   - **prompt**: Custom role description, focus area, expected output format
   - **model**: Within the `model_ceiling` from YAML
   - **tools**: Within the `tool_ceiling` from YAML (backend intersects lead's request with ceiling)
4. Backend validates each spawn against `team_constraints` before allowing it
5. If a spawn request exceeds any constraint, it's rejected with an error message to the lead

### 4.5 MCP Tool Scoping for Dynamic Teammates

**Challenge:** Currently `tools.ts` has per-agent hardcoded MCP allowlists. Dynamic teammates don't have pre-assigned agent types in the allowlist.

**Solution — Two options:**

| Option | Approach | Trade-off |
|--------|----------|-----------|
| **A: "team-member" allowlist** | Add a broad `ideation-team-member` entry to `tools.ts` matching the `mcp_tool_ceiling` | Simple. Slightly less granular per-teammate. |
| **B: Dynamic env var** | Pass `RALPHX_MCP_ALLOWED_TOOLS=get_session_plan,list_session_proposals` as env var, MCP server reads it | More granular. Requires MCP server change. |

**Recommendation:** Option A for Phase 1 (simplest), migrate to Option B if granular per-teammate MCP scoping is needed.

### 4.6 CLI Spawn Pattern for Teammates

Each ideation teammate is spawned by the RalphX backend after the lead requests it via MCP. The backend validates against YAML constraints, then spawns an **interactive** session (no `-p` flag — teammates must stay alive for messaging):

```bash
# Example: Lead requested a "realtime-transport-researcher" teammate
# Backend validates against team_constraints, then spawns:

env \
  CLAUDECODE=1 \
  CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1 \
  RALPHX_PROJECT_ID={project_id} \
  RALPHX_SESSION_ID={ideation_session_id} \
  RALPHX_AGENT_TYPE=ideation-team-member \
claude \
  --output-format stream-json \
  --agent-id realtime-transport-researcher\@ideation-{session_id} \
  --agent-name realtime-transport-researcher \
  --team-name ideation-{session_id} \
  --agent-color {auto_assigned_color} \
  --parent-session-id {lead_session_uuid} \
  --agent-type ideation-team-member \
  --model sonnet \
  --tools "{intersect(lead_requested_tools, tool_ceiling)}" \
  --allowedTools "mcp__ralphx__get_session_plan,mcp__ralphx__list_session_proposals" \
  --mcp-config {mcp_config_path} \
  --strict-mcp-config \
  --append-system-prompt "{lead_generated_role_prompt + base_teammate_instructions}" \
  --dangerously-skip-permissions \
  --plugin-dir ./ralphx-plugin \
  --setting-sources user,project \
  --disable-slash-commands
```

**CRITICAL: No `-p` flag.** Teammates are interactive sessions that receive messages via Claude Code's native SendMessage tool. The `-p` flag would make them execute once and exit, breaking team communication. The lead-generated role prompt is passed via `--append-system-prompt` instead.

**Key difference from current RalphX spawning:** Current agents use `-p` (print mode). Team agents use interactive mode — the process stays alive, receives messages as input, and terminates only on shutdown_request. This requires `ClaudeCodeClient` to support a second spawning path (see Section 8).

**Constraint enforcement flow:**
```
Lead requests: spawn("realtime-transport-researcher", tools=[Read,Grep,Glob,Bash], model=sonnet)
  │
  ▼
Backend reads team_constraints.ideation from YAML
  │
  ▼
Validates: tools ∩ tool_ceiling = [Read,Grep,Glob,Bash] ∩ [Read,Grep,Glob,WebFetch,WebSearch,Task,Bash] = ✅
Validates: model ≤ model_ceiling (sonnet ≤ sonnet) = ✅
Validates: teammate_count < max_teammates = ✅
  │
  ▼
Spawn approved → build CLI args → spawn process
```

---

## 5. UI/UX Changes

### 5.1 Session Creation

**Wireframe — New Session Dialog (modified):**
```
┌─────────────────────────────────────────────────────┐
│  New Ideation Session                                │
├─────────────────────────────────────────────────────┤
│                                                      │
│  What would you like to build?                       │
│  ┌──────────────────────────────────────────────┐   │
│  │ [Text input area]                             │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  Ideation Mode:                                      │
│  ┌──────────┐ ┌────────────┐ ┌────────────┐        │
│  │   Solo   │ │ ★ Research │ │ ★ Debate   │        │
│  │          │ │    Team    │ │    Team    │        │
│  └──────────┘ └────────────┘ └────────────┘        │
│                                                      │
│  ★ = Recommended for complex features                │
│                                                      │
│  [When Research or Debate selected:]                 │
│  ┌──────────────────────────────────────────────┐   │
│  │ Max teammates: 3 ▾  Model ceiling: Sonnet ▾  │   │
│  │ Budget: $5.00 (optional)                      │   │
│  │ Composition: ● Dynamic  ○ Constrained         │   │
│  └──────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────┐   │
│  │ ⓘ The lead agent will decide what specialist  │   │
│  │   roles to create based on your task.         │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  [Start Session]                                     │
└─────────────────────────────────────────────────────┘
```

### 5.2 Team Activity Panel

When team mode is active, the ideation view gets a new panel showing teammate activity. Teammate names are dynamic (assigned by the lead, not predefined):

**Wireframe — Team Activity (right panel, replaces or augments plan browser):**
```
┌─────────────────────────────────────────────────────┐
│  Team Activity                              [3/3 ●] │
├─────────────────────────────────────────────────────┤
│                                                      │
│  🟢 realtime-transport-researcher [Exploring...]     │
│  ├─ Read src/hooks/useWebSocket.ts                   │
│  ├─ Grep "EventSource" in src/                       │
│  └─ Finding: "No existing WebSocket infra, SSE..."   │
│  [💬 Message]                                        │
│                                                      │
│  🔵 react-state-sync-researcher  [Exploring...]      │
│  ├─ Read src/stores/taskStore.ts                     │
│  ├─ Read src/hooks/useTanStackQuery.ts               │
│  └─ Finding: "Zustand stores use immer, could..."    │
│  [💬 Message]                                        │
│                                                      │
│  🟡 event-system-researcher      [Idle]              │
│  └─ Completed: "DB has no trigger system, MCP..."    │
│  [💬 Message]                                        │
│                                                      │
│  ─────────────────────────────────────────────────   │
│  Team Messages (4)                                   │
│  ├─ transport → state-sync: "WebSocket needs..."     │
│  ├─ state-sync → transport: "Zustand can handle..."  │
│  ├─ event-sys → lead: "MCP event bridge needed..."   │
│  └─ YOU → transport: "What about HTTP/2 SSE?"        │
│                                                      │
│  ┌──────────────────────────────────────────┐        │
│  │ Message: [input] Send to: [dropdown ▾]  │        │
│  └──────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────┘
```

**User-to-teammate messaging:** Each teammate card has a "Message" button. The bottom input area allows sending a message to any teammate or the lead. Messages appear in the Team Messages feed alongside inter-agent messages.

### 5.3 Cost Indicator

Team sessions show a cost indicator in the session header:

```
┌────────────────────────────────────────────┐
│  Session: "Add real-time collaboration"    │
│  Mode: Research Team (3 specialists)       │
│  Tokens: ~450K  |  Est. Cost: $3.20       │
└────────────────────────────────────────────┘
```

### 5.4 Plan Presentation (Enhanced)

When the lead presents the plan after team research, the plan display includes a "Team Findings" section:

```
## Implementation Plan

### Team Research Summary
| Specialist | Key Finding |
|------------|-------------|
| Frontend | Existing ChatPanel uses unified hooks; new component needs... |
| Backend | AgenticClient trait already supports team spawning pattern... |
| Infra | Database schema needs new team_sessions table... |

### Architecture
[Plan content — same format as today]

### Debate Summary (Debate mode only)
| Approach | Advocate | Strengths | Weaknesses |
|----------|----------|-----------|------------|
| WebSockets | Advocate A | Real-time, bidirectional | Complexity, state mgmt |
| SSE | Advocate B | Simple, HTTP-based | One-directional |
| Winner: WebSockets — justification: ...
```

---

## 6. API Surface Changes

### 6.1 Backend (Rust)

**New/Modified Tauri Commands:**

| Command | Type | Parameters | Returns |
|---------|------|------------|---------|
| `create_ideation_session` | Modified | Add `team_mode: Option<String>`, `team_config: Option<TeamConfig>` | Existing response + team info |
| `get_ideation_team_status` | New | `session_id: String` | `TeamStatusResponse` |
| `send_team_message` | New | `session_id: String, target: String, content: String` | `MessageResult` |
| `request_teammate_spawn` | New | `session_id: String, spawn_request: TeammateSpawnRequest` | `SpawnResult` (validated against constraints) |

**New Types:**

```rust
pub struct TeamConfig {
    pub max_teammates: u32,           // 2-5
    pub model_ceiling: String,        // "sonnet" | "haiku" | "opus"
    pub cost_budget_usd: Option<f64>,
    pub composition_mode: CompositionMode,  // Dynamic | Constrained
}

pub enum CompositionMode {
    Dynamic,      // Lead decides roles, YAML provides constraints
    Constrained,  // Lead limited to predefined templates from YAML
}

pub struct TeammateSpawnRequest {
    pub name: String,              // Lead-chosen role name (e.g., "security-reviewer")
    pub prompt: String,            // Lead-generated prompt for this teammate
    pub model: String,             // Must be ≤ model_ceiling
    pub requested_tools: Vec<String>,  // Intersected with tool_ceiling
    pub requested_mcp_tools: Vec<String>,  // Intersected with mcp_tool_ceiling
    pub template: Option<String>,  // Optional predefined template to use
}

pub struct TeamStatusResponse {
    pub team_name: String,
    pub teammates: Vec<TeammateStatus>,
    pub messages: Vec<TeamMessage>,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub composition_mode: CompositionMode,
}

pub struct TeammateStatus {
    pub name: String,              // Dynamic role name assigned by lead
    pub role_description: String,  // Brief description of what this teammate does
    pub status: String,            // "exploring", "idle", "completed"
    pub current_activity: Option<String>,
    pub color: String,
    pub model: String,
}

pub struct TeamMessage {
    pub from: String,              // Teammate name or "user"
    pub to: String,                // Teammate name, "lead", or "user"
    pub content: String,
    pub timestamp: String,
}
```

### 6.2 Frontend API

**New API functions in `src/api/ideation.ts`:**

```typescript
export async function getIdeationTeamStatus(sessionId: string): Promise<TeamStatusResponse> { ... }
export async function sendTeamMessage(sessionId: string, target: string, content: string): Promise<void> { ... }
export async function requestTeammateSpawn(sessionId: string, request: TeammateSpawnRequest): Promise<SpawnResult> { ... }
```

**Modified:**

```typescript
export async function createSession(input: CreateSessionInput): Promise<CreateSessionResponse> {
  // Add teamMode, teamConfig (with compositionMode) to input
}
```

**New hooks:**

```typescript
// Poll team status for live activity panel
export function useIdeationTeamStatus(sessionId: string) { ... }
// Send message to teammate or lead
export function useSendTeamMessage(sessionId: string) { ... }
```

### 6.3 MCP Server

**Minimal changes needed:**

| Change | Reason |
|--------|--------|
| Add `ideation-team-member` to `TOOL_ALLOWLIST` in `tools.ts` | Dynamic teammates need an agent type for MCP tool filtering |
| Allowlist: `[get_session_plan, list_session_proposals, get_plan_artifact]` | Matches `mcp_tool_ceiling` from YAML |

No structural MCP server changes — the existing tool filtering system works. The new `ideation-team-member` agent type acts as the ceiling for all dynamically-spawned ideation teammates.

---

## 7. Backend Architecture

### 7.1 Team Lifecycle for Ideation (Agent-Led Model)

```
User creates session (teamMode: "research", compositionMode: "dynamic")
  │
  ▼
Backend creates ideation session (existing flow)
  │
  ▼
Backend spawns team lead (orchestrator-ideation-team-lead) in INTERACTIVE mode
  │  Lead has team CLI flags: --team-name ideation-{session_id}
  │
  ▼
Lead uses native TeamCreate to create team (config + task list dirs)
  │
  ▼
Lead enters UNDERSTAND phase → analyzes task → decides team composition dynamically
  │  e.g., "I need a transport researcher, a state sync researcher, and an event system researcher"
  │
  ▼
Lead requests teammate spawns via MCP tool (request_teammate_spawn)
  │  Each request includes: name, prompt, model, tools
  │
  ▼
Backend validates each spawn against team_constraints from YAML
  │  ├─ tools ∩ tool_ceiling
  │  ├─ model ≤ model_ceiling
  │  └─ count < max_teammates
  │
  ▼
Backend spawns each teammate in INTERACTIVE mode with team CLI flags (Section 4.6)
  │  Teammates join team via --team-name, --parent-session-id
  │
  ▼
Teammates explore codebase, share findings via native SendMessage
Lead coordinates via native SendMessage (Claude Code's internal messaging)
User can message any teammate via UI (routed through lead — see Section 7.3)
  │
  ▼
Lead synthesizes findings → creates plan artifact via MCP
  │
  ▼
Lead presents plan to user via chat → CONFIRM phase
  │
  ▼
Lead creates proposals → FINALIZE phase
  │
  ▼
Lead sends shutdown_request to all teammates via native SendMessage
  │
  ▼
Lead calls TeamDelete to clean up team config + task list directories
Backend detects lead exit → cleans up any remaining teammate processes
```

### 7.2 Integration with Existing Spawner

The `AgenticClientSpawner` needs to be extended:

```rust
// Current: single agent per session
pub async fn spawn_ideation_agent(session_id: &str, ...) -> AgentResult<AgentHandle> { ... }

// New: team-aware spawning
pub async fn spawn_ideation_team_lead(
    session_id: &str,
    team_config: &TeamConfig,
    ...
) -> AgentResult<TeamLeadHandle> {
    // 1. Create team config + task list directories
    // 2. Spawn team lead with team CLI flags
    // 3. Return TeamLeadHandle (lead manages teammate requests)
}

// New: spawn a teammate (called when lead requests via MCP)
pub async fn spawn_ideation_teammate(
    session_id: &str,
    spawn_request: &TeammateSpawnRequest,
    team_constraints: &TeamConstraints,
) -> AgentResult<TeammateHandle> {
    // 1. Validate spawn_request against team_constraints
    // 2. Intersect requested tools with tool_ceiling
    // 3. Build CLI args with team flags
    // 4. Spawn teammate process
    // 5. Track in TeamStateTracker
}
```

### 7.3 User-to-Teammate Message Routing

Claude Code agent teams use an **internal runtime messaging system** (SendMessage tool), NOT filesystem-based mailboxes. User messages must be routed through the team lead or via a dedicated MCP tool:

**Option A: Route through lead (simpler, recommended for Phase 1):**
```
User clicks "Message" on teammate card
  │
  ▼
Frontend calls send_team_message(session_id, "transport-researcher", "What about HTTP/2 SSE?")
  │
  ▼
Backend writes message to lead's stdin pipe (interactive mode)
  │  Format: "User message for transport-researcher: What about HTTP/2 SSE?"
  │
  ▼
Lead receives message → forwards to teammate via native SendMessage tool
  │
  ▼
Teammate receives message → responds via SendMessage → lead relays to backend
  │
  ▼
Backend emits event → UI shows response in Team Messages feed
```

**Option B: MCP relay tool (more direct, Phase 2+):**
```
User clicks "Message" on teammate card
  │
  ▼
Frontend calls send_team_message(session_id, "transport-researcher", "What about HTTP/2 SSE?")
  │
  ▼
Backend stores message in pending queue
  │
  ▼
Teammate calls poll_user_messages MCP tool (on each turn) → receives pending messages
  │
  ▼
Teammate responds via relay_response MCP tool → Backend emits event → UI shows response
```

**Recommendation:** Option A for Phase 1. The lead is already coordinating all communication. Adding stdin message injection to `ClaudeCodeClient`'s interactive mode is required anyway (Section 8). Option B adds MCP tools but removes the lead as intermediary for faster communication.

### 7.4 Execution State Integration

Team sessions count against `max_concurrent_tasks` as follows:
- **Solo session**: 1 slot (same as today)
- **Research team**: 1 slot for lead + 1 slot per teammate (total: 2-6, depending on dynamic composition)
- **Debate team**: 1 slot for lead + 1 slot per advocate (total: 3-6)

**Important:** The exact slot count is dynamic because the lead decides how many teammates to spawn. The backend enforces `max_teammates` from YAML constraints, and total slots are capped by `max_concurrent_tasks` in execution settings.

---

## 8. Critical Architecture Constraint: Interactive vs Print Mode

### The Problem

RalphX currently spawns ALL agents in **print mode** (`-p` flag) for headless execution. Agent teams require **interactive sessions** — teammates must stay alive to receive messages via SendMessage, and the lead must be interactive to use TeamCreate/SendMessage/TaskCreate.

Print mode (`-p`) makes Claude execute a single prompt and exit. Teammates spawned with `-p` cannot participate in team communication.

### Resolution Options

| Option | Approach | Trade-offs |
|--------|----------|------------|
| **A: Interactive Agents** | Spawn lead + teammates WITHOUT `-p`, communicate via stdin pipe | Requires `ClaudeCodeClient` refactoring to support interactive mode. Lead uses native TeamCreate/SendMessage. Cleanest integration with Claude Code agent teams. |
| **B: SDK Interface** | Wait for Agent SDK to expose team management programmatically | Depends on Anthropic roadmap. Cleanest long-term but timeline unknown. |
| **C: Backend-as-Lead** | No Claude lead agent. RalphX backend manages team lifecycle directly. | Most control but requires reimplementing team messaging, loses native agent team features. |

### Recommended: Option A (Agent-Led, Interactive Mode)

The team lead and all teammates run as **interactive Claude Code sessions** (no `-p` flag). This is how agent teams are designed to work:

1. Backend spawns team lead in interactive mode with team CLI flags
2. Lead uses native TeamCreate, SendMessage, TaskCreate tools
3. Lead requests teammate spawns via MCP tool (`request_teammate_spawn`)
4. Backend validates against YAML constraints, then spawns teammate in interactive mode
5. Teammates join the team and communicate via Claude Code's native messaging system
6. Backend monitors processes and streams output for the UI

**Why not Option C (Backend-as-Lead)?**
The v1 draft recommended Option C, but technical review revealed:
- Claude Code messaging is internal to the runtime — NOT filesystem-based mailboxes. The backend can't inject messages into teammate inboxes via filesystem writes.
- Reimplementing team messaging in Rust would be fragile and duplicate Claude Code's native capabilities.
- The agent-led model (Option A) leverages all native agent teams features: messaging, task lists, idle notifications, shutdown protocol.

**Impact on `ClaudeCodeClient`:**
Currently `ClaudeCodeClient` only supports print mode (spawn → read stdout → process exits). For team agents, it needs a new interactive spawning mode:
- Spawn process without `-p`
- Hold stdin pipe open for sending user messages
- Stream stdout for UI updates
- Process stays alive until shutdown_request

This is a significant but well-scoped refactor — `ClaudeCodeClient` already handles process lifecycle, just needs a second spawning path.

### MCP Tools: Work in All Modes

MCP servers are stdio-based, per-process. They work regardless of whether the agent is foreground or background. The only limitation for background agents is that they cannot prompt the user for permission — but with `--dangerously-skip-permissions`, this is not a concern for RalphX-spawned teammates.

---

## 9. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Token cost explosion** | High | Budget cap per session. Cost indicator in UI. Team recommended for complex work only. |
| **Interactive mode refactor** | High | `ClaudeCodeClient` needs interactive spawning mode (no `-p`). Well-scoped: second spawn path, stdin pipe, process stays alive. See Section 8. |
| **User message routing latency** | Medium | Phase 1 routes through lead (adds a hop). Acceptable for ideation. Phase 2 adds direct MCP relay. |
| **Dynamic role quality** | Medium | Lead may create poorly-scoped roles. Mitigation: good lead system prompt with role-creation examples. Constraint validation catches tool/model issues. |
| **MCP tool scoping for dynamic roles** | Medium | "team-member" allowlist in tools.ts as ceiling (Section 4.5). Validated against YAML constraints. |
| **Teammate context divergence** | Medium | Teammates share findings via messaging. Lead synthesizes into unified plan. User can intervene directly. |
| **Team coordination overhead** | Medium | Lead manages coordination. User can redirect individual teammates if needed. |
| **Session recovery complexity** | Medium | Teams don't support `--resume`. If teammate crashes, re-spawn it. Backend tracks team state. |
| **Increased latency** | Low | Research phase takes longer (3-5 min vs 1-2 min), but plan quality improves. User notified via progress UI. |
| **File conflicts** | None | All ideation agents are read-only. No file writes in ideation workflow. |

---

## 10. Cost/Benefit Analysis

### Costs

| Factor | Solo Session | Research Team (3) | Debate Team (4) |
|--------|-------------|-------------------|-----------------|
| Token usage | ~100K | ~350-400K | ~500-600K |
| Estimated cost | ~$0.80 | ~$2.50-3.50 | ~$4.00-5.00 |
| Latency (EXPLORE phase) | 1-2 min | 3-5 min | 4-6 min |
| Context windows | 1 | 4 | 5 |

### Benefits

| Benefit | Solo | Research Team | Debate Team |
|---------|------|--------------|-------------|
| Cross-domain awareness | Low (sequential) | High (parallel + messaging) | Medium |
| Plan quality | Good | Better (multi-perspective) | Best (adversarial) |
| Missed edge cases | Common | Fewer | Fewest |
| User touchpoints | 2-3 | 2-3 (same!) | 2-3 (same!) |
| Architecture decision confidence | Medium | High | Very High |

### When to Use Each

| Scenario | Recommended Mode | Why |
|----------|-----------------|-----|
| Simple feature (< 3 tasks) | Solo | Low complexity doesn't justify team overhead |
| Quick bug fix ideation | Solo | Fast turnaround more valuable than depth |
| Feature touching 2+ layers | **Research Team ★** | Cross-domain awareness prevents integration failures |
| Complex project feature | **Research Team ★** | "It would be a mistake not to use more compute" |
| New subsystem or architecture change | **Debate Team ★** | Adversarial challenge catches blind spots early |
| Performance-critical decisions | **Debate Team ★** | Multiple approaches need rigorous comparison |
| Security-sensitive design | **Debate Team ★** (constrained mode) | Limit teammates to predefined security-aware roles |

**★ = Recommended.** For complex projects like RalphX, team mode should be the bias — solo is the fallback for simple tasks, not the default for everything.

---

## 11. Phased Rollout

**Note:** Ideation and worker integration ship in the SAME phase (per user direction). No sequential phasing between the two.

### Phase 1: Foundation + Dynamic Teams (MVP)
- **Backend:** Team lifecycle management, constraint validation engine, teammate spawning
- **Backend:** Team config + task list directory management (Hybrid model)
- **Backend:** User-to-teammate message routing
- **Config:** `team_constraints` in ralphx.yaml, `compositionMode` (dynamic/constrained)
- **Agent:** `orchestrator-ideation-team-lead` prompt + system card
- **MCP:** `ideation-team-member` allowlist in tools.ts, `request_teammate_spawn` tool
- **UI:** Team mode selector in session creation (Research + Debate)
- **UI:** Team activity panel with dynamic role names
- **UI:** User-to-teammate direct messaging
- **Scope:** Research Team with dynamic composition. Lead decides roles.

### Phase 2: Debate Mode + Full Lifecycle
- Debate Team mode with dynamically-created advocate roles
- UI: Competing plans side-by-side presentation
- Teammates persist through PLAN phase for feedback
- Cost tracking and budget enforcement
- Template library for common ideation patterns (optional presets)

### Phase 3: Advanced Control
- Per-project team mode defaults (e.g., "always use Research Team for this project")
- User-defined constraint overrides per session
- Team session recovery (re-spawn teammates on crash)
- Analytics: team vs solo quality comparison dashboard

---

## 12. Open Questions for Product Review

**Answered (incorporated into brief):**
- ~~User-to-teammate messaging~~ → YES, users can message both lead and individual teammates (Section 5.2, 7.3)
- ~~Cost tolerance~~ → Both tiered. Team recommended for complex, solo for simple (Section 3.1, 10)
- ~~Phase scope~~ → Both ideation and worker in same phase (Section 11)
- ~~Flow preservation~~ → Confirmed opt-in, current flows untouched (Section 1)
- ~~Dynamic vs predefined roles~~ → Dynamic default, constrained opt-in (Section 4.1)

**Remaining open questions:**

1. **Team findings persistence:** Currently plan artifacts persist but teammate research doesn't. Should we store teammate findings as additional artifacts? Or are they ephemeral (only the synthesized plan matters)?

2. **Cost presentation:** Should we show per-teammate token usage breakdown, or just aggregate? (Per-teammate helps users understand which roles are most valuable.)

3. **Lead model selection:** Is Opus the right default for the team lead? It's more expensive but better at coordination. Alternative: Sonnet lead for research teams, Opus only for debate teams.

4. **Integration with Active Plan:** When a team-ideated plan is accepted, should it be tagged as "team-ideated" for downstream visibility? This could help correlate plan quality with team usage.

5. **Dynamic role guardrails:** In dynamic mode, should there be a minimum prompt length or quality check for lead-generated teammate prompts? Or trust the lead fully?

6. **Constrained mode UX:** When constrained mode is selected, should the user see and approve the predefined roles before the lead spawns them?

---

## 13. Success Metrics

| Metric | Target |
|--------|--------|
| Plan quality (user approval rate without modification) | +20% vs solo |
| Edge cases caught pre-implementation | +40% vs solo |
| Average plan rejections per session | -30% vs solo |
| User satisfaction with proposals | Measure via feedback |
| Token cost per session (Research Team) | < 4x solo |
| Token cost per session (Debate Team) | < 6x solo |
| Adoption rate (% sessions using team mode) | 15-25% after 30 days |

---

## 14. Dependencies

| Dependency | Status | Owner |
|------------|--------|-------|
| Claude Code Agent Teams feature | Experimental (enabled via env var) | Anthropic |
| Agent Teams system card | ✅ Complete (#4) | system-card-writer |
| ralphx.yaml config system | ✅ Documented (#3) | agent-cataloger |
| CLI spawning system | ✅ Documented (#1) | systems-researcher |
| RalphX agent catalog | ✅ Documented (#2) | agent-cataloger |
| Worker integration brief | ✅ Complete (#6) | system-card-writer |
| Agent variant/YAML mapping | In progress (#7) | agent-cataloger |

---

## 15. Non-Goals

- Modifying the current solo orchestrator-ideation agent
- Requiring teams for any ideation session (always opt-in)
- Implementing nested teams (teams of teams)
- Adding Write/Edit tools to any ideation teammate (maintain read-only principle for ideation)
- Changing the 6-phase gated workflow structure
- Automated team mode selection (user always chooses the mode)
- Rigid YAML-driven role definitions as the default (dynamic is default; constrained is opt-in)
- Building a custom team protocol (we use Claude Code Agent Teams as-is)
