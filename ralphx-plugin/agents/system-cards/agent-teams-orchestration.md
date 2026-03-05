# Agent Teams Orchestration — System Card

> Reference for team leads spawning and coordinating Claude Code Agent Teams.
> Read this file at session start (Phase 0) before any team operations.

## Tool Reference

| Tool | Purpose | Audience | Key Args / Notes |
|------|---------|----------|------------------|
| `TeamCreate` | Create team config + shared task directory | both | `team_name` (use `ideation-<session_id>` for ideation teams), `description` |
| `TaskCreate` | Add work items to team's shared task list | both | `subject` (imperative), `description` (full context), `activeForm` (spinner text) |
| `Task` | Spawn a teammate subprocess | both | `subagent_type: "general-purpose"`, `name` (unique within team), `team_name`, `prompt`, `model`, `mode: "bypassPermissions"`. Ideation: `run_in_background: true`. Execution: foreground only (MCP requires it). |
| `SendMessage` | Communicate with teammates | both | `type: "message"\|"broadcast"\|"shutdown_request"`, `recipient` (teammate name), `content`, `summary`. Broadcast = N API calls — use only for critical team-wide issues. |
| `TaskUpdate` | Assign tasks, set status, add dependencies | both | `taskId`, `owner`, `status`, `addBlockedBy` |
| `TaskList` | Check team progress — all tasks + owners | both | (no args) |
| `TeamDelete` | Cleanup after shutdown | both | (no args) — only after `shutdown_response(approve)` from all teammates |
| `request_team_plan` | **BLOCKING** — request human approval before spawning | both | `process: "ideation"\|"worker-execution"`, `teammates: [{role, model, prompt_summary}]`. Backend records plan but does NOT auto-spawn. Lead waits for approval before calling `Task`. |
| `save_team_session_state` | Persist team state for recovery after interruption | ideation | `session_id`, `state` (JSON: phase, teammates, tasks, artifacts so far) |
| `get_team_session_state` | Restore prior state at Phase 0 RECOVER | ideation | `session_id` — returns saved state or null if fresh session |

**Parallel spawning:** Emit ALL `Task` calls in one response. Multiple calls in one message = simultaneous launch.

**Model guide:** `haiku` — simple lookups | `sonnet` — most tasks (default) | `opus` — architecture/synthesis

---

## Teammate Lifecycle

```
TeamCreate
    |
TaskCreate (x N)
    |
Task (spawn teammates in parallel)
    |
    v
+--[Teammate Process]--+
|                       |
|  1. Starts fresh      |  <-- No access to your conversation history
|  2. Reads prompt      |  <-- Everything it needs must be in the prompt
|  3. Does work         |  <-- Uses Read/Grep/Glob/Bash as needed
|  4. Sends message     |  <-- SendMessage back to you with findings
|  5. Goes IDLE         |  <-- Normal! Not an error. Waiting for input
|  6. Receives message  |  <-- Your SendMessage wakes it up
|  7. Does more work    |
|  8. Goes IDLE again   |
|  ...                  |
|  N. Shutdown request  |  <-- You send shutdown_request
|  N+1. Approves        |  <-- Teammate sends shutdown_response
|  N+2. Process exits   |
+--[End]----------------+
    |
TeamDelete
```

**Key behaviors:**
- Teammates go idle after every turn — this is **normal**, not an error
- Idle teammates can receive messages — sending wakes them up
- Messages from teammates are automatically delivered to you (no polling needed)
- Each teammate has its own independent context window
- Teammates cannot see your conversation or other teammates' conversations (only via SendMessage)

---

## Artifact Workflow (Ideation Teams)

```
Teammates → create_team_artifact(type: "TeamResearch")
                    |
                    ↓
Lead → get_team_artifacts() → synthesize findings
                    |
                    ↓
Lead → create_team_artifact(type: "TeamSummary")
                    |
                    ↓
Lead → create_plan_artifact() — links to TeamSummary
                    |
                    ↓
Plan artifact linked to session — proposals reference it
```

Teammates ALWAYS create a TeamResearch artifact (not just SendMessage). The lead synthesizes into TeamSummary, then creates the plan artifact.

---

## Prompt Authoring for Teammates

The `prompt` parameter is the ONLY context the teammate receives. It must be self-contained.

| Required Section | Content | Why |
|-----------------|---------|-----|
| Role identity | `"You are {role} on team {team-name}"` | No implicit context |
| Mission | Specific scope + hard boundaries | Prevents overlap with other teammates |
| Codebase context | Project overview + relevant dirs/files | No shared history |
| Files to investigate | Specific paths (not "find X") | Saves search rounds |
| Expected output | Numbered deliverables | Defines done |
| When done | `create_team_artifact` → `SendMessage` to lead → `TaskUpdate` complete | Audit trail + progress |

**Tips:**
- Include `session_id` — required for MCP tool calls
- Include `task_id` — so teammates can mark their task complete
- Name other teammates and their scope — prevents duplicate work
- Scope boundaries: "Only investigate frontend hooks — backend team handles Rust"
- Use `mode: "bypassPermissions"` — teammates should not prompt for permissions

---

## RalphX MCP Tools for Teammates

Teammates that need to interact with RalphX (create artifacts, read plans, etc.) need the RalphX MCP tools. These are available when the teammate is spawned with `subagent_type: "general-purpose"` and the RalphX plugin is configured.

| MCP Tool | Purpose | When to Include |
|----------|---------|-----------------|
| `get_session_plan` | Read the master plan artifact | Always for research teammates |
| `list_session_proposals` | See existing proposals | When working on refinement |
| `create_team_artifact` | Store research findings | Always — primary output method |
| `get_team_artifacts` | Read other teammates' findings | For synthesis or cross-referencing |

Include these in the teammate's prompt instructions so they know which MCP tools to call.

---

## Complete Example: Research Team

```
Phase 0: RECOVER
  → get_team_session_state(session_id) — check for prior interrupted state
  → Read this system card

Phase 1: COMPOSE
  → Analyze task → decide roles (e.g., frontend-researcher, backend-analyst)

Phase 2: APPROVE
  → request_team_plan(process="ideation", teammates=[...]) — BLOCKS until user approves
  → save_team_session_state(session_id, {phase: "approved", teammates: [...]})

Phase 3: EXPLORE (after approval)
  → TeamCreate(team_name="ideation-<session_id>")
  → TaskCreate x2 (Task #1: frontend research, Task #2: backend research)
  → Task x2 in ONE message — parallel spawn, run_in_background: true
  → Monitor: read messages, relay cross-layer findings via SendMessage

Phase 4: PLAN
  → get_team_artifacts() — collect all TeamResearch artifacts
  → create_team_artifact(type="TeamSummary") — synthesize findings
  → create_plan_artifact() — links to TeamSummary

Phase 5: FINALIZE
  → SendMessage shutdown_request to each teammate
  → Wait for shutdown_response(approve) from each
  → TeamDelete
```

---

## Complete Example: Execution Team

```
Phase 0: RECOVER
  → get_team_session_state(task_id) — check for prior interrupted state

Phase 1: ANALYZE
  → get_task_context(task_id) + get_artifact(plan_artifact_id)
  → get_project_analysis(project_id, task_id) — validation baseline

Phase 2: DECOMPOSE
  → Break into file-ownership scopes with wave ordering:
     Wave 1: types + backend handler (independent, no deps)
     Wave 2: React hooks (depends on Wave 1 outputs)
     Wave 3: Tests (depends on all)

Phase 3: APPROVE
  → request_team_plan(process="worker-execution", teammates=[...]) — BLOCKS until approval

Phase 4: EXECUTE (wave-by-wave — foreground only, MCP required)
  Wave 1:
    → TeamCreate + TaskCreate x2
    → Task x2 in ONE message (no run_in_background)
    → Each coder: start_step → implement → complete_step → SendMessage to lead
    → Wave gate: npm run typecheck + cargo test --lib (all must pass before Wave 2)
  Wave 2+: repeat with dependent scopes, passing Wave 1 outputs as context in prompt

Phase 5: VALIDATE + COMPLETE
  → Run ALL validation: typecheck + lint + tests
  → execution_complete(task_id)
  → SendMessage shutdown_request to each coder → wait → TeamDelete
```

| Aspect | Ideation Team | Execution Team |
|--------|---------------|----------------|
| Purpose | Research + discover | Implement code |
| Output | Team artifacts (markdown) | Code files + step progress |
| File access | Read-only | Exclusive write ownership |
| Progress tracking | Artifacts + messages | `start_step` / `complete_step` MCP tools |
| Validation | N/A | Gate between every wave (`get_project_analysis`) |
| Background mode | `run_in_background: true` | **Foreground only** (MCP access required) |
| Plan approval | `request_team_plan()` | `request_team_plan()` (same) |
| Teammate model | Typically `sonnet` | Dynamic: `haiku` (simple), `sonnet` (complex), `opus` (architecture) |
