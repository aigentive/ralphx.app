# Product Brief: Configurable Agent Variants and Dynamic Team Composition

**Status:** DRAFT v3
**Author:** agent-cataloger
**Date:** 2026-02-15
**Scope:** Infrastructure for solo ↔ team mode switching with agent-driven dynamic team composition
**Depends on:** `docs/architecture/ralphx-yaml-config.md`, `docs/architecture/agent-catalog.md`

---

## 1. Executive Summary

This brief proposes two complementary additions to `ralphx.yaml`:

1. **`process_mapping`** — Decouples logical processes (ideation, execution, review, merge) from hardcoded agent names, enabling solo ↔ team mode switching per process.
2. **`team_constraints`** — Defines guardrails (max teammates, allowed tools, model caps) that team leads operate within when dynamically composing their teams.

**Core design principle: Team composition is agent-driven, not config-driven.** When a team lead spawns, it analyzes the task and decides what teammates it needs — custom roles, custom prompts, custom tool sets. Configuration provides constraints and optional presets, not rigid role definitions.

**Two modes:**
- **Dynamic mode** (default): Team lead chooses roles, prompts, and models for teammates based on the task
- **Constrained mode** (opt-in): Team lead limited to predefined agent configs from `agents[]`

**Key principles:**
- Zero breaking changes to existing config
- Current solo-agent flows preserved as default
- Team variants are opt-in, per-task or per-session
- Agents reason about what teammates they need — YAML constrains, not dictates
- Predefined agents serve as templates/presets, not mandatory roles

---

## 2. Current State

### How Process → Agent Mapping Works Today

The mapping is hardcoded in two places in the Rust backend:

#### A. State Machine Entry Handlers (`on_enter_states.rs`)

Direct `spawn()` calls with hardcoded agent type strings:

| State Entry | Agent Type String | Spawn Method |
|-------------|-------------------|--------------|
| `Ready` (qa_enabled) | `"qa-prep"` | `agent_spawner.spawn_background()` |
| `Executing` | — | `chat_service.send_message(TaskExecution, ...)` |
| `QaRefining` | `"qa-refiner"` | `agent_spawner.spawn()` |
| `QaTesting` | `"qa-tester"` | `agent_spawner.spawn()` |
| `Reviewing` | — | `chat_service.send_message(Review, ...)` |
| `ReExecuting` | — | `chat_service.send_message(TaskExecution, ...)` |

#### B. ChatService Agent Resolution (`chat_service_helpers.rs`)

A `resolve_agent()` function maps `ChatContextType` + optional entity status to agent names:

```rust
fn resolve_agent(context_type: &ChatContextType, entity_status: Option<&str>) -> &'static str {
    // Status-aware overrides
    match (context_type, status) {
        (Review, "review_passed") => AGENT_REVIEW_CHAT,
        (Review, "approved") => AGENT_REVIEW_HISTORY,
        (Ideation, "accepted") => AGENT_ORCHESTRATOR_IDEATION_READONLY,
        _ => {}
    }
    // Default rules
    match context_type {
        Ideation => AGENT_ORCHESTRATOR_IDEATION,
        Task => AGENT_CHAT_TASK,
        Project => AGENT_CHAT_PROJECT,
        TaskExecution => AGENT_WORKER,
        Review => AGENT_REVIEWER,
        Merge => AGENT_MERGER,
    }
}
```

#### C. Agent Name Constants (`agent_names.rs`)

All agent names are string constants:

```rust
pub const SHORT_WORKER: &str = "ralphx-worker";
pub const AGENT_WORKER: &str = "ralphx:ralphx-worker";
// ... etc for all 20 agents
```

### Problems with Current Approach

| Problem | Impact |
|---------|--------|
| Adding a new agent variant requires Rust code changes | Slow iteration cycle (rebuild, test, deploy) |
| No way to swap agents per-task or per-session | All tasks use the same worker, same reviewer |
| Process → agent mapping invisible to users | No UI surface for choosing execution strategy |
| Team mode integration requires hardcoding team variants | Each integration needs custom Rust code |
| No ability for agents to compose their own teams | Team structure must be predetermined by humans |

---

## 3. Proposed Design

### 3.1 Design Philosophy: Agent-Driven Team Composition

The fundamental insight: **the agent is best positioned to decide what teammates it needs.** A worker-team lead facing an authentication task might decide it needs a "security-reviewer" and a "test-writer." A worker-team lead facing a UI task might want a "design-qa" and a "accessibility-checker." These roles don't need to be predefined in YAML.

```
┌─────────────────────────────────────────────────────────┐
│                    YAML Configuration                     │
│  ┌──────────────────┐  ┌─────────────────────────────┐  │
│  │ process_mapping   │  │ team_constraints             │  │
│  │ ideation:         │  │ execution:                   │  │
│  │   default: solo   │  │   max_teammates: 4           │  │
│  │   team: team-lead │  │   allowed_tools: [R,W,E,B]  │  │
│  │ execution:        │  │   model_cap: sonnet          │  │
│  │   default: solo   │  │   mode: dynamic              │  │
│  │   team: team-lead │  │   presets: [coder, tester]   │  │
│  └──────────────────┘  └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                            │
                    Team lead spawns
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Team Lead (at runtime)                       │
│  "For this auth task, I need:"                           │
│    ├── Teammate 1: security-reviewer (custom prompt)     │
│    ├── Teammate 2: test-writer (custom prompt)           │
│    └── Teammate 3: coder (from preset)                   │
│                                                           │
│  Constraints enforced:                                    │
│    ✅ 3 teammates ≤ max_teammates (4)                    │
│    ✅ All tools within allowed_tools                     │
│    ✅ All models ≤ model_cap (sonnet)                    │
└─────────────────────────────────────────────────────────┘
```

### 3.2 YAML Schema: `process_mapping` Section

Maps logical process slots to agents. The `team` variant points to a team lead agent that will dynamically compose its team.

```yaml
process_mapping:
  ideation:
    default: orchestrator-ideation           # solo mode
    readonly: orchestrator-ideation-readonly  # read-only mode
    team: orchestrator-ideation-team         # team lead (composes own team)

  execution:
    default: ralphx-worker                   # solo mode
    team: ralphx-worker-team                 # team lead (composes own team)

  review:
    default: ralphx-reviewer
    chat: ralphx-review-chat
    history: ralphx-review-history

  merge:
    default: ralphx-merger

  qa_prep:
    default: ralphx-qa-prep

  qa_refine:
    default: ralphx-qa-executor

  qa_test:
    default: ralphx-qa-executor

  chat_task:
    default: chat-task

  chat_project:
    default: chat-project
```

### 3.3 YAML Schema: `team_constraints` Section

Defines guardrails that team leads must operate within. This is the safety layer — agents have freedom within these boundaries.

```yaml
team_constraints:
  # Per-process constraints
  ideation:
    max_teammates: 3                         # max simultaneous teammates
    allowed_tools:                            # tools teammates can use
      - Read
      - Grep
      - Glob
      - Bash
      - WebFetch
      - WebSearch
    model_cap: sonnet                        # max model tier for teammates
    mode: dynamic                            # dynamic | constrained
    presets:                                  # optional agent templates
      - researcher                           # references agents[] by name
      - critic

  execution:
    max_teammates: 4
    allowed_tools:
      - Read
      - Write
      - Edit
      - Bash
      - Grep
      - Glob
      - WebFetch
      - WebSearch
    allowed_mcp_tools:                       # MCP tools teammates can access
      - get_task_context
      - get_artifact_content
      - start_step
      - complete_step
    model_cap: sonnet
    mode: dynamic
    presets:
      - ralphx-coder                         # predefined coder template
    timeout_minutes: 30                      # max team runtime

  review:
    max_teammates: 2
    allowed_tools: [Read, Grep, Glob, Bash]
    model_cap: sonnet
    mode: constrained                        # ONLY predefined agents allowed
    presets:
      - ralphx-reviewer

  # Global defaults (applied when process-specific not set)
  _defaults:
    max_teammates: 3
    model_cap: sonnet
    mode: dynamic
    timeout_minutes: 20
```

### 3.4 Dynamic vs. Constrained Mode

| Aspect | Dynamic Mode (default) | Constrained Mode |
|--------|----------------------|------------------|
| **Team composition** | Lead decides roles/prompts per task | Lead picks from `presets` only |
| **Custom prompts** | Lead can write teammate prompts | Prompts come from `agents[]` definitions |
| **Custom tools** | Any tools within `allowed_tools` | Tools from preset agent config only |
| **Custom models** | Any model ≤ `model_cap` | Model from preset agent config only |
| **Use case** | Exploratory/creative work | Security-sensitive, compliance-required |
| **Validation** | Constraint enforcement at spawn | Agent name validation against presets |

**How it works at runtime:**

```
Dynamic mode:
  Lead → "I need a security-reviewer with tools [Read, Grep, Glob]"
  Backend → checks: tools ⊆ allowed_tools? model ≤ model_cap? count ≤ max?
  Backend → spawns teammate with lead-provided prompt + validated tools

Constrained mode:
  Lead → "I need a ralphx-coder"
  Backend → checks: "ralphx-coder" ∈ presets? count ≤ max?
  Backend → spawns teammate with predefined config from agents[]
```

### 3.5 Agent Variant Inheritance (extends)

Team lead variants extend base agent configs:

```yaml
agents:
  # Existing base agent (unchanged)
  - name: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write, Edit, Task] }
    mcp_tools: [start_step, complete_step, ...]
    preapproved_cli_tools: [Read, Write, Edit, Bash, Task, ...]

  # Team lead variant (inherits from base, adds team orchestration)
  - name: ralphx-worker-team
    extends: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker-team.md
    model: opus                              # team leads need stronger reasoning
    # inherits: tools, mcp_tools from ralphx-worker

  # Preset teammate template (optional, used in constrained mode or as suggestion)
  - name: ralphx-coder
    system_prompt_file: ralphx-plugin/agents/coder.md
    model: sonnet
    tools: { extends: base_tools, include: [Write, Edit] }
    mcp_tools: [get_task_context, start_step, complete_step]
```

**Inheritance rules:**
- `extends: <agent-name>` — inherit all fields from the named agent
- Explicitly set fields override inherited values
- Missing fields fall through to parent
- Circular extends detected and rejected
- `extends` is optional — agents without it are standalone (backward compatible)

### 3.6 Rust Backend Changes

#### A. New Deserialization Structs

```rust
// In agent_config/mod.rs

#[derive(Debug, Clone, Deserialize, Default)]
struct ProcessSlot {
    default: String,
    #[serde(flatten)]
    variants: HashMap<String, String>,  // variant_name → agent_name
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ProcessMapping {
    #[serde(flatten)]
    slots: HashMap<String, ProcessSlot>,  // key = process name (ideation, execution, etc.)
}

#[derive(Debug, Clone, Deserialize)]
struct TeamConstraints {
    #[serde(default = "default_max_teammates")]
    max_teammates: u8,
    #[serde(default)]
    allowed_tools: Vec<String>,
    #[serde(default)]
    allowed_mcp_tools: Vec<String>,
    #[serde(default = "default_model_cap")]
    model_cap: String,                       // "haiku" | "sonnet" | "opus"
    #[serde(default = "default_mode")]
    mode: TeamMode,                          // "dynamic" | "constrained"
    #[serde(default)]
    presets: Vec<String>,                    // agent names from agents[]
    #[serde(default = "default_timeout")]
    timeout_minutes: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TeamMode {
    Dynamic,
    Constrained,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TeamConstraintsConfig {
    #[serde(rename = "_defaults")]
    defaults: Option<TeamConstraints>,
    #[serde(flatten)]
    processes: HashMap<String, TeamConstraints>,
}

// Updated RalphxConfig:
struct RalphxConfig {
    tool_sets: HashMap<String, Vec<String>>,
    claude: ClaudeRuntimeConfigRaw,
    agents: Vec<AgentConfigRaw>,
    #[serde(default)]
    process_mapping: ProcessMapping,         // NEW
    #[serde(default)]
    team_constraints: TeamConstraintsConfig, // NEW
}
```

#### B. Constraint Enforcement — The Interception Problem

**Critical architectural constraint:** When RalphX spawns a team lead as a Claude CLI process, the lead uses Claude Code's **built-in `Task` tool** to spawn teammates. RalphX cannot intercept or modify `Task` tool behavior — it's a Claude Code internal, not an MCP tool. This means `validate_teammate_spawn()` cannot be called as a middleware on the spawn path.

**Enforcement strategies (ordered by robustness):**

| Strategy | Enforcement | Engineering Cost | Robustness |
|----------|------------|-----------------|------------|
| 1. System prompt | Lead self-enforces constraints from prompt | Low | Weak — no backend guarantee |
| 2. Pre-flight MCP tool | Lead calls `request_team_plan` before spawning | Medium | Good — backend validates before spawn |
| 3. Post-hoc hooks | Monitor spawned teammates, shut down violators | Medium | Reactive — damage may occur first |
| 4. RalphX-managed spawning | Replace `Task` tool with `spawn_teammate` MCP tool | High | Strong — full backend control |

**Recommended approach: Strategy 2 (Phase 1) → Strategy 4 (Phase 2)**

##### Strategy 2: Pre-Flight MCP Validation (Phase 1)

Add an MCP tool `request_team_plan` that the team lead **must** call before spawning any teammates. The lead's system prompt makes this a hard requirement.

```
Team Lead                          RalphX Backend
    │                                   │
    │  1. request_team_plan({           │
    │       process: "execution",       │
    │       teammates: [                │
    │         { role: "security-reviewer", tools: [Read,Grep], model: "sonnet" },
    │         { role: "test-writer", tools: [Read,Write,Bash], model: "sonnet" },
    │         { role: "coder", preset: "ralphx-coder" }
    │       ]                           │
    │     })                            │
    │  ─────────────────────────────►   │
    │                                   │  validate against team_constraints
    │                                   │  ├── count ≤ max_teammates? ✅
    │                                   │  ├── tools ⊆ allowed_tools? ✅
    │                                   │  ├── models ≤ model_cap? ✅
    │                                   │  └── mode check (dynamic/constrained)
    │                                   │
    │  ◄─────────────────────────────   │
    │  { approved: true,                │
    │    plan_id: "tp-abc123",          │
    │    teammates: [                   │
    │      { role: "security-reviewer", │
    │        approved_tools: "Read,Grep",│
    │        approved_model: "sonnet" },│
    │      ...                          │
    │    ] }                            │
    │                                   │
    │  2. Task tool: spawn teammates    │
    │     using approved plan           │
    │  ────────────────────────────►    │
    │  (Claude Code built-in)           │
```

**MCP tool definition:**

```typescript
// In ralphx-mcp-server/src/tools.ts
{
  name: "request_team_plan",
  description: "Submit a team composition plan for validation. MUST be called before spawning teammates.",
  inputSchema: {
    type: "object",
    properties: {
      process: { type: "string", description: "Process type (execution, ideation, review)" },
      teammates: {
        type: "array",
        items: {
          type: "object",
          properties: {
            role: { type: "string", description: "Teammate role name" },
            tools: { type: "array", items: { type: "string" }, description: "Requested CLI tools" },
            mcp_tools: { type: "array", items: { type: "string" }, description: "Requested MCP tools" },
            model: { type: "string", description: "Requested model (haiku|sonnet|opus)" },
            preset: { type: "string", description: "Preset agent name (alternative to custom config)" },
            prompt_summary: { type: "string", description: "Brief description of teammate's instructions" }
          },
          required: ["role"]
        }
      }
    },
    required: ["process", "teammates"]
  }
}
```

**Rust validation handler:**

```rust
pub fn validate_team_plan(
    process: &str,
    teammates: &[TeammateRequest],
) -> Result<ApprovedTeamPlan, TeamConstraintError> {
    let constraints = get_team_constraints(process);

    // 1. Check teammate count
    if teammates.len() > constraints.max_teammates as usize {
        return Err(TeamConstraintError::MaxTeammatesExceeded {
            max: constraints.max_teammates,
            requested: teammates.len(),
        });
    }

    // 2. Per-teammate validation
    let mut approved = Vec::new();
    for req in teammates {
        match constraints.mode {
            TeamMode::Constrained => {
                let preset = req.preset.as_ref().ok_or(
                    TeamConstraintError::PresetRequired { role: req.role.clone() }
                )?;
                if !constraints.presets.contains(preset) {
                    return Err(TeamConstraintError::AgentNotInPresets {
                        agent: preset.clone(),
                        allowed: constraints.presets.clone(),
                    });
                }
                approved.push(ApprovedTeammate::from_preset(preset));
            }
            TeamMode::Dynamic => {
                // Validate tools
                for tool in &req.tools {
                    if !constraints.allowed_tools.contains(tool) {
                        return Err(TeamConstraintError::ToolNotAllowed {
                            tool: tool.clone(),
                            role: req.role.clone(),
                        });
                    }
                }
                // Validate MCP tools
                for tool in &req.mcp_tools {
                    if !constraints.allowed_mcp_tools.contains(tool) {
                        return Err(TeamConstraintError::McpToolNotAllowed {
                            tool: tool.clone(),
                            role: req.role.clone(),
                        });
                    }
                }
                // Validate model
                if !model_within_cap(&req.model, &constraints.model_cap) {
                    return Err(TeamConstraintError::ModelExceedsCap {
                        requested: req.model.clone(),
                        cap: constraints.model_cap.clone(),
                    });
                }
                approved.push(ApprovedTeammate::from_dynamic(req));
            }
        }
    }

    Ok(ApprovedTeamPlan {
        plan_id: generate_plan_id(),
        process: process.to_string(),
        teammates: approved,
    })
}
```

**Limitation:** This is "honor system + backend logging." The lead's prompt says to call `request_team_plan` first, but Claude Code's Task tool doesn't check for a plan_id. A prompt-injected or malfunctioning lead could skip validation. Mitigation: log all team spawns, alert on unvalidated teammates.

##### Strategy 4: RalphX-Managed Spawning (Phase 2 — Endgame)

Replace Claude Code's built-in `Task` tool with a RalphX-managed `spawn_teammate` MCP tool for team leads. The lead's `--tools` flag would **exclude** `Task`, and spawning goes through RalphX's backend.

```
Team Lead                          RalphX Backend
    │                                   │
    │  spawn_teammate({                 │
    │    role: "security-reviewer",     │
    │    prompt: "Review auth code...", │
    │    tools: [Read, Grep, Glob],    │
    │    model: "sonnet"               │
    │  })                               │
    │  ─────────────────────────────►   │
    │                                   │  validate_team_plan() ← same logic
    │                                   │  │
    │                                   │  ▼ if approved:
    │                                   │  ClaudeCodeClient::spawn_teammate()
    │                                   │  ├── --agent-name "security-reviewer"
    │                                   │  ├── --team-name <team-id>
    │                                   │  ├── --parent-session-id <lead-session>
    │                                   │  ├── --tools "Read,Grep,Glob"
    │                                   │  ├── --model sonnet
    │                                   │  ├── --append-system-prompt <lead-provided>
    │                                   │  └── --mcp-config <dynamic-mcp-config>
    │                                   │
    │  ◄─────────────────────────────   │
    │  { spawned: true,                 │
    │    teammate_id: "sec-rev-1",      │
    │    session_id: "..." }            │
```

This gives RalphX full control:
- **Validation** happens in Rust before any Claude CLI process starts
- **Tool allowlists** are enforced via CLI flags (`--tools`, `--allowedTools`)
- **MCP tool filtering** uses a per-teammate dynamic MCP config (see Section 3.8)
- **Model caps** enforced via `--model` flag
- **Teammate tracking** — RalphX knows every teammate spawned, can shut them down

```rust
/// TeammateSpawnRequest — what the team lead asks to spawn via MCP.
struct TeammateSpawnRequest {
    role: String,                    // human-readable role name
    prompt: Option<String>,          // lead-provided system prompt (dynamic mode)
    preset: Option<String>,          // preset agent name (constrained mode)
    tools: Vec<String>,              // requested CLI tools
    mcp_tools: Vec<String>,          // requested MCP tools
    model: String,                   // requested model tier
}
```

#### C. Agent Resolution via Process Mapping

Replace hardcoded `resolve_agent()` with config-driven lookup:

```rust
pub fn resolve_process_agent(
    process: &str,
    variant: &str,
) -> Option<String> {
    let config = load_config();
    let slot = config.process_mapping.slots.get(process)?;

    // Try requested variant first, fall back to "default"
    let agent_name = slot.variants.get(variant)
        .or_else(|| Some(&slot.default))
        .cloned()?;

    // Validate agent exists in config
    get_agent_config(&agent_name).map(|_| agent_name)
}
```

#### D. Variant Selection at Spawn Time

| Priority | Source | Example |
|----------|--------|---------|
| 1 | Per-task metadata | `task.metadata.agent_variant = "team"` |
| 2 | Per-session setting | Session-level team mode toggle |
| 3 | Per-project setting | Project default in DB |
| 4 | Process mapping default | `process_mapping.execution.default` |

```rust
// In on_enter_states.rs — updated State::Executing handler
State::Executing => {
    let variant = self.resolve_variant("execution", &task);
    let agent_name = resolve_process_agent("execution", &variant)
        .unwrap_or(AGENT_WORKER);  // fallback to constant
    // ... spawn with resolved agent_name
}
```

#### E. Agent Config Inheritance Resolution

```rust
fn resolve_agent_extends(
    raw: &AgentConfigRaw,
    all_agents: &[AgentConfigRaw],
    stack: &mut Vec<String>,
) -> AgentConfigRaw {
    if let Some(ref parent_name) = raw.extends {
        if stack.contains(parent_name) {
            warn!("Circular agent extends: {:?}", stack);
            return raw.clone();
        }
        stack.push(parent_name.clone());
        let parent = all_agents.iter().find(|a| a.name == *parent_name);
        if let Some(parent) = parent {
            let resolved_parent = resolve_agent_extends(parent, all_agents, stack);
            return merge_agent_configs(&resolved_parent, raw);
        }
    }
    raw.clone()
}

fn merge_agent_configs(parent: &AgentConfigRaw, child: &AgentConfigRaw) -> AgentConfigRaw {
    AgentConfigRaw {
        name: child.name.clone(),
        system_prompt_file: child.system_prompt_file.clone()
            .or(parent.system_prompt_file.clone()),
        model: child.model.clone().or(parent.model.clone()),
        tools: if child.tools != Default::default() { child.tools.clone() }
               else { parent.tools.clone() },
        mcp_tools: if child.mcp_tools.is_empty() { parent.mcp_tools.clone() }
                   else { child.mcp_tools.clone() },
        preapproved_cli_tools: if child.preapproved_cli_tools.is_empty() {
            parent.preapproved_cli_tools.clone()
        } else {
            child.preapproved_cli_tools.clone()
        },
        settings_profile: child.settings_profile.clone()
            .or(parent.settings_profile.clone()),
        extends: None,
    }
}
```

### 3.7 How Team Leads Spawn Teammates

The team lead workflow follows two phases depending on enforcement strategy:

#### Phase 1 Flow (Pre-flight MCP Validation)

```
1. Lead analyzes task → determines needed roles/skills
2. Lead calls request_team_plan() MCP tool with proposed team composition
3. RalphX backend validates against team_constraints → returns approved plan
4. Lead spawns teammates using Claude Code's built-in Task tool
5. Lead coordinates via SendMessage, TaskCreate, TaskUpdate
```

The lead's system prompt **requires** calling `request_team_plan` before any `Task` spawn:

```markdown
## MANDATORY: Team Composition Approval

Before spawning ANY teammate, you MUST call `request_team_plan` with your proposed team.
DO NOT use the Task tool to spawn teammates until you receive an approved plan.
Include each teammate's role, tools, model, and a brief prompt summary.
```

#### Phase 2 Flow (RalphX-Managed Spawning)

```
1. Lead analyzes task → determines needed roles/skills
2. Lead calls spawn_teammate() MCP tool for each teammate
3. RalphX backend validates → spawns Claude CLI process with correct flags
4. Lead coordinates via SendMessage, TaskCreate, TaskUpdate
```

In Phase 2, the `Task` tool is **removed** from the team lead's tool allowlist:

```yaml
# Team lead tool config — no Task tool, uses spawn_teammate MCP instead
- name: ralphx-worker-team
  extends: ralphx-worker
  tools: { extends: base_tools, include: [Write, Edit] }  # no Task
  mcp_tools: [spawn_teammate, shutdown_teammate, get_team_status, ...]
```

### 3.8 Dynamic MCP Tool Filtering for Teammates

**Problem:** `tools.ts` has hardcoded per-agent `TOOL_ALLOWLIST` entries. Dynamic teammates with custom roles won't have entries in that allowlist.

**Solution:** Pass allowed MCP tools to the MCP server via environment variable in the per-teammate dynamic MCP config.

```json
{
  "mcpServers": {
    "ralphx": {
      "command": "node",
      "args": ["<mcp-server-path>/dist/index.js"],
      "env": {
        "RALPHX_AGENT_TYPE": "dynamic-teammate",
        "RALPHX_MCP_URL": "http://localhost:3847",
        "RALPHX_ALLOWED_MCP_TOOLS": "get_task_context,get_artifact_content,start_step,complete_step"
      }
    }
  }
}
```

**MCP server changes in `tools.ts`:**

```typescript
function getToolAllowlist(agentType: string): string[] | null {
  // Check env var first — used for dynamic teammates
  const envAllowed = process.env.RALPHX_ALLOWED_MCP_TOOLS;
  if (envAllowed) {
    return envAllowed.split(',').map(t => t.trim());
  }

  // Fall back to hardcoded per-agent allowlist
  return TOOL_ALLOWLIST[agentType] ?? null;
}
```

This way:
- **Predefined agents** continue using the hardcoded `TOOL_ALLOWLIST` (no change)
- **Dynamic teammates** get their MCP tools from the env var, set by the Rust backend when spawning the teammate's Claude CLI process
- **Agent prompt frontmatter** (Layer 3) still applies — the teammate's system prompt can further restrict MCP tools

### 3.10 Environment Variable Overrides

```bash
# Override process variant selection
RALPHX_PROCESS_VARIANT_EXECUTION=team

# Override team mode
RALPHX_TEAM_MODE_EXECUTION=constrained

# Override max teammates
RALPHX_TEAM_MAX_EXECUTION=6

# Override model cap
RALPHX_TEAM_MODEL_CAP_EXECUTION=opus

# Per-agent settings (existing mechanism, still works)
RALPHX_CLAUDE_SETTINGS_PROFILE_RALPHX_WORKER_TEAM=z_ai
```

### 3.11 Runtime Switching

#### Per-Task Switching

```
┌─────────────────────────────┐
│  Start Task: "Add auth"     │
│                             │
│  Execution mode:            │
│  ○ Solo (default)           │
│  ● Team (agent decides      │
│    team composition)         │
│                             │
│  [Start]                    │
└─────────────────────────────┘
```

Sets `task.metadata.agent_variant = "team"` → state machine spawns team lead instead of solo worker.

#### Per-Session Switching (Ideation)

```
┌─────────────────────────────┐
│  New Ideation Session       │
│                             │
│  Mode:                      │
│  ○ Solo (single orchestrator)│
│  ● Team (multi-perspective  │
│    ideation)                 │
│                             │
│  [Start]                    │
└─────────────────────────────┘
```

---

## 4. UI Surface

### 4.1 Task Execution Mode Selector

**Location:** Task detail panel → "Start Execution" action
**When visible:** Only when `process_mapping.execution` has a `team` variant
**Default:** "Solo" (maps to `default` variant)

### 4.2 Ideation Session Mode Selector

**Location:** Ideation sidebar → "New Session" button
**When visible:** Only when `process_mapping.ideation` has a `team` variant
**Default:** "Solo"

### 4.3 Team Activity Monitor

**Location:** Task detail panel (when team mode active)
**Shows:** Live teammate list, their roles, current status, message log
**Purpose:** Visibility into dynamic team composition

### 4.4 Project-Level Default Setting

**Location:** Project Settings → "Agent Configuration"
**Options:** Set default variant for each process, configure team constraints overrides

### 4.5 System Tray / Status Bar Indicator

- Solo mode: single-agent icon
- Team mode: multi-agent icon with teammate count and role labels

---

## 5. Migration Path

### Phase 0: Foundation

1. Add `process_mapping` section to `ralphx.yaml` — if absent, fall back to hardcoded constants
2. Add `team_constraints` section — if absent, use sensible defaults
3. Add `extends` field to `AgentConfigRaw` — if absent, standalone (current behavior)
4. Update `resolve_agent()` to check process mapping first, fall back to constants
5. All existing configs continue to work unchanged

**Fallback chain:**
```
process_mapping lookup
    → found? Use mapped agent
    → not found? Fall back to hardcoded constant (current behavior)

team_constraints lookup
    → found? Use process-specific constraints
    → not found? Use _defaults
    → no _defaults? Use hardcoded sensible defaults
```

### Phase 1: Pre-Flight MCP Validation + Team Leads

1. Implement `request_team_plan` MCP tool (pre-flight validation)
2. Implement `validate_team_plan()` in Rust backend
3. Create team lead system prompts (`worker-team.md`, `orchestrator-ideation-team.md`)
4. Prompts instruct leads: analyze task → `request_team_plan` → spawn via `Task` → coordinate
5. Add `team` variants to `process_mapping`
6. Add `team_constraints` with sensible defaults
7. Dynamic MCP tool filtering via `RALPHX_ALLOWED_MCP_TOOLS` env var in `tools.ts`
8. Add UI mode selector for execution and ideation
9. Add team activity monitor
10. **Enforcement: prompt-enforced + backend logging/alerting on unvalidated spawns**

### Phase 2: RalphX-Managed Spawning (Full Control)

1. Implement `spawn_teammate` MCP tool — RalphX backend spawns teammates directly
2. Implement `shutdown_teammate`, `get_team_status` MCP tools
3. Remove `Task` tool from team lead allowlists; all spawning via MCP
4. Backend generates per-teammate dynamic MCP configs with `RALPHX_ALLOWED_MCP_TOOLS`
5. Backend tracks all teammates — can shut down violators, enforce timeout
6. **Enforcement: full backend control, no honor system**

### Phase 3: Expanded Coverage + User Customization

1. Team mode for review process (multi-reviewer teams)
2. Build library of useful preset agents (security-reviewer, test-writer, etc.)
3. Per-project constraint customization via UI
4. Custom preset creation via UI
5. Constraint templates (e.g., "strict security" preset)
6. Team composition analytics (which roles are most spawned, success rates)

---

## 6. YAML Schema Changes Summary

### New Top-Level Sections

| Section | Purpose |
|---------|---------|
| `process_mapping` | Maps process slots to agent variants (solo, team, readonly) |
| `team_constraints` | Guardrails for dynamic team composition per process |

### New Fields

| Section | Field | Type | Default | Description |
|---------|-------|------|---------|-------------|
| `process_mapping.<slot>` | `default` | `String` | required | Default (solo) agent |
| `process_mapping.<slot>` | `team` | `String` | — | Team lead agent |
| `process_mapping.<slot>` | `<variant>` | `String` | — | Other named variants |
| `team_constraints.<process>` | `max_teammates` | `u8` | `3` | Max simultaneous teammates |
| `team_constraints.<process>` | `allowed_tools` | `Vec<String>` | all base tools | Tools teammates can use |
| `team_constraints.<process>` | `allowed_mcp_tools` | `Vec<String>` | `[]` | MCP tools teammates can access |
| `team_constraints.<process>` | `model_cap` | `String` | `"sonnet"` | Max model tier for teammates |
| `team_constraints.<process>` | `mode` | `"dynamic"\|"constrained"` | `"dynamic"` | Dynamic or preset-only |
| `team_constraints.<process>` | `presets` | `Vec<String>` | `[]` | Agent templates from `agents[]` |
| `team_constraints.<process>` | `timeout_minutes` | `u32` | `20` | Max team runtime |
| `team_constraints._defaults` | (all above) | — | — | Defaults for unspecified processes |
| `agents[]` | `extends` | `String?` | `null` | Parent agent for config inheritance |

### New Rust Structs

| Struct | Purpose |
|--------|---------|
| `ProcessSlot` | Single process slot with default + variants |
| `ProcessMapping` | HashMap-based collection of all process slots |
| `TeamConstraints` | Guardrails for a single process |
| `TeamConstraintsConfig` | All process constraints + defaults |
| `TeamMode` | Enum: Dynamic \| Constrained |
| `TeammateSpawnRequest` | What a team lead asks to spawn via MCP |
| `ApprovedTeamPlan` | Backend-approved team composition plan |

### New MCP Tools

| Tool | Phase | Purpose |
|------|-------|---------|
| `request_team_plan` | 1 | Pre-flight team composition validation |
| `spawn_teammate` | 2 | RalphX-managed teammate spawning |
| `shutdown_teammate` | 2 | Graceful teammate shutdown |
| `get_team_status` | 2 | Query current team composition and status |

### New Public APIs (Rust)

| Function | Purpose |
|----------|---------|
| `resolve_process_agent(process, variant)` | Config-driven agent resolution |
| `get_team_constraints(process)` | Get constraints for a process (merged with defaults) |
| `validate_team_plan(process, teammates)` | Validate entire team composition against constraints |
| `resolve_agent_extends(raw, all)` | Agent config inheritance resolution |

### MCP Server Changes (`tools.ts`)

| Change | Purpose |
|--------|---------|
| `getToolAllowlist()` checks `RALPHX_ALLOWED_MCP_TOOLS` env var | Dynamic MCP tool filtering for custom teammates |
| New `request_team_plan` tool handler | Routes to Rust backend for constraint validation |

---

## 7. Example: Full Config

```yaml
tool_sets:
  base_tools: [Read, Grep, Glob, Bash, WebFetch, WebSearch, Skill]

process_mapping:
  ideation:
    default: orchestrator-ideation
    readonly: orchestrator-ideation-readonly
    team: orchestrator-ideation-team
  execution:
    default: ralphx-worker
    team: ralphx-worker-team
  review:
    default: ralphx-reviewer
    chat: ralphx-review-chat
    history: ralphx-review-history
  merge:
    default: ralphx-merger
  chat_task:
    default: chat-task
  chat_project:
    default: chat-project

team_constraints:
  _defaults:
    max_teammates: 3
    model_cap: sonnet
    mode: dynamic
    timeout_minutes: 20

  ideation:
    max_teammates: 3
    allowed_tools: [Read, Grep, Glob, Bash, WebFetch, WebSearch]
    model_cap: sonnet
    mode: dynamic
    presets: [researcher, critic]

  execution:
    max_teammates: 4
    allowed_tools: [Read, Write, Edit, Bash, Grep, Glob, WebFetch, WebSearch]
    allowed_mcp_tools: [get_task_context, get_artifact_content, start_step, complete_step]
    model_cap: sonnet
    mode: dynamic
    presets: [ralphx-coder]
    timeout_minutes: 30

  review:
    max_teammates: 2
    allowed_tools: [Read, Grep, Glob, Bash]
    model_cap: sonnet
    mode: constrained                        # only predefined reviewers
    presets: [ralphx-reviewer]

claude:
  mcp_server_name: ralphx
  permission_mode: default
  settings_profile: default

agents:
  # --- Existing agents (unchanged) ---
  - name: orchestrator-ideation
    system_prompt_file: ralphx-plugin/agents/orchestrator-ideation.md
    model: opus
    tools: { extends: base_tools, include: [Task] }
    mcp_tools: [create_task_proposal, update_task_proposal, ...]

  - name: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write, Edit, Task] }
    mcp_tools: [start_step, complete_step, ...]

  # --- Team lead variants ---
  - name: orchestrator-ideation-team
    extends: orchestrator-ideation
    system_prompt_file: ralphx-plugin/agents/orchestrator-ideation-team.md
    # inherits: model, tools, mcp_tools

  - name: ralphx-worker-team
    extends: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker-team.md
    model: opus
    # inherits: tools, mcp_tools

  # --- Preset teammate templates (used as suggestions or in constrained mode) ---
  - name: ralphx-coder
    system_prompt_file: ralphx-plugin/agents/coder.md
    model: sonnet
    tools: { extends: base_tools, include: [Write, Edit] }
    mcp_tools: [get_task_context, start_step, complete_step]
```

---

## 8. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Dynamic teams spawn too many agents → cost explosion | Medium | High | `max_teammates` hard cap, `timeout_minutes`, `model_cap` enforced in backend |
| Team leads generate poor custom prompts | Medium | Medium | Provide high-quality preset templates as examples in the lead's system prompt |
| Phase 1 enforcement bypass (lead skips `request_team_plan`) | Medium | Medium | Backend logs all team spawns; alert on unvalidated teammates; Phase 2 eliminates this gap entirely |
| Phase 2 engineering complexity | — | Medium | Phase 1 provides working system while Phase 2 is built; incremental rollout |
| Constraint validation bypass via prompt injection | Low | High | Phase 1: partial (prompt-based). Phase 2: full (Rust backend controls spawn) |
| Config complexity for users | Medium | Low | Both sections are optional; defaults match current solo behavior |
| Circular extends | Low | High | Cycle detection with stack tracking |
| Dynamic mode produces unpredictable results | Medium | Medium | Start with constrained mode for critical processes (review, merge), dynamic for exploratory (ideation, execution) |
| MCP tool filtering for dynamic teammates | Low | Medium | `RALPHX_ALLOWED_MCP_TOOLS` env var in per-teammate MCP config; `tools.ts` checks env var before hardcoded allowlist |
| Agent name conflicts between dynamic names and presets | Low | Low | Dynamic teammates use `<team-name>/<role>` namespace, presets use `agents[]` namespace |

---

## 9. Success Criteria

**Phase 0:**
- [ ] `process_mapping` section is optional — existing configs work unchanged
- [ ] `team_constraints` section is optional — sensible defaults applied
- [ ] `extends` field resolves agent inheritance correctly (including nested)
- [ ] `resolve_process_agent()` returns correct agent for all process/variant combinations
- [ ] Fallback to hardcoded constants when process mapping is absent
- [ ] All 20 existing agents continue to work without modification
- [ ] Circular extends detected and rejected with warning

**Phase 1:**
- [ ] `request_team_plan` MCP tool validates team composition before spawn
- [ ] `validate_team_plan()` enforces all constraints (tools, MCP tools, model, count, mode)
- [ ] Dynamic mode: team leads can spawn custom-prompted teammates (after approval)
- [ ] Constrained mode: team leads limited to preset agents only
- [ ] `RALPHX_ALLOWED_MCP_TOOLS` env var filters MCP tools for dynamic teammates
- [ ] `tools.ts` reads env var for dynamic teammate allowlists
- [ ] Per-task variant selection via metadata
- [ ] Per-session variant selection for ideation
- [ ] Environment variable overrides for constraints and variants
- [ ] Backend logs all team spawns; alerts on unvalidated teammates
- [ ] Cost controls: max_teammates, timeout_minutes, model_cap all enforced

**Phase 2:**
- [ ] `spawn_teammate` MCP tool gives RalphX full control over teammate spawning
- [ ] `Task` tool removed from team lead allowlists
- [ ] Per-teammate dynamic MCP config generated with correct `RALPHX_ALLOWED_MCP_TOOLS`
- [ ] Backend can shut down violating or timed-out teammates
- [ ] No honor-system gaps — all constraints enforced in Rust before spawn
