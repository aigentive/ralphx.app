# Agent Teams Orchestration — System Card

> Reference for team leads spawning and coordinating Claude Code Agent Teams.
> Read this file at session start (Phase 0) before any team operations.

## Tool Reference

### TeamCreate — Create a team

Creates the team config and shared task list on disk.

```json
{
  "team_name": "ideation-<session_id>",
  "description": "Research team for <topic>"
}
```

Creates:
- `~/.claude/teams/<team_name>/config.json` — membership registry (auto-populated as teammates join)
- `~/.claude/tasks/<team_name>/` — shared task list directory

**Naming convention:** Always use `ideation-<session_id>` for ideation teams so the team name maps to the session.

---

### TaskCreate — Add work items to the shared task list

```json
{
  "subject": "Research React state management patterns",
  "description": "Investigate existing hooks, stores, and data flow in src/hooks/ and src/stores/. Document patterns for state synchronization between Kanban and detail views.",
  "activeForm": "Researching React state patterns"
}
```

- `subject`: imperative form, concise (e.g., "Research X", "Analyze Y")
- `description`: full context — what to investigate, which files/dirs, expected output
- `activeForm`: present continuous form shown in UI spinner while in_progress

Tasks get auto-incremented IDs (1, 2, 3...). Set dependencies via `TaskUpdate` with `addBlockedBy`.

---

### Task — Spawn a teammate subprocess

This is the critical tool. Each call spawns an independent Claude Code subprocess that joins the team.

```json
{
  "subagent_type": "general-purpose",
  "name": "frontend-researcher",
  "team_name": "ideation-<session_id>",
  "description": "Research frontend patterns",
  "prompt": "<full instructions — see Prompt Authoring below>",
  "model": "sonnet",
  "mode": "bypassPermissions",
  "run_in_background": true
}
```

#### Parameters explained

| Parameter | Required | Values | Purpose |
|-----------|----------|--------|---------|
| `subagent_type` | Yes | `"general-purpose"` (default for teammates) | Determines available tools. `general-purpose` = all tools (Read, Write, Edit, Bash, Glob, Grep, Task tools, SendMessage) |
| `name` | Yes | e.g. `"frontend-researcher"`, `"backend-analyst"` | Human-readable name. Used for `SendMessage` recipient, task ownership, team config. Must be unique within the team |
| `team_name` | Yes | Must match `TeamCreate` name | Joins this team, gets access to shared task list |
| `description` | Yes | 3-5 words | Short summary shown in UI |
| `prompt` | Yes | Full instructions string | Everything the teammate needs — they cannot see your conversation history |
| `model` | No | `"haiku"`, `"sonnet"`, `"opus"` | Model powering the teammate. See Model Selection below |
| `mode` | No | `"default"`, `"bypassPermissions"`, `"plan"` | Permission mode. `bypassPermissions` lets teammate edit/run without asking. `plan` requires plan approval before implementation |
| `run_in_background` | No | `true` / `false` | `true` = async launch (returns immediately, teammate runs in background). Required for parallel spawning |

#### Parallel spawning

To launch multiple teammates concurrently, include multiple `Task` tool calls in a **single message**:

```
Message contains 3 Task calls (all in one response):
  Task { name: "frontend-researcher", ... }
  Task { name: "backend-analyst", ... }
  Task { name: "infra-specialist", ... }
```

They launch simultaneously as independent subprocesses.

#### Model selection guide

| Model | Cost | Use When |
|-------|------|----------|
| `haiku` | Lowest | Simple file searches, data collection, straightforward lookups |
| `sonnet` | Medium | Code analysis, pattern research, most research tasks |
| `opus` | Highest | Architecture decisions, complex synthesis, cross-system analysis |

Default to `sonnet` for research teammates. Use `opus` only for architectural analysis or synthesis tasks.

---

### SendMessage — Communicate with teammates

#### Direct message (most common)

```json
{
  "type": "message",
  "recipient": "frontend-researcher",
  "content": "Backend team found shared types need an `email` field. Check if your frontend components handle this.",
  "summary": "Relay backend finding about email field"
}
```

- `recipient`: teammate's `name` (from Task spawn)
- `content`: the full message text
- `summary`: 5-10 word preview shown in UI

#### Broadcast (use sparingly — sends to ALL teammates)

```json
{
  "type": "broadcast",
  "content": "STOP: Breaking change found in base types. Hold all work until resolved.",
  "summary": "Critical blocking issue found"
}
```

**Warning:** Broadcast sends a separate message to every teammate. N teammates = N API calls. Only use for critical team-wide issues.

#### Shutdown request

```json
{
  "type": "shutdown_request",
  "recipient": "frontend-researcher",
  "content": "Research complete, wrapping up session"
}
```

The teammate receives a shutdown request and must respond with `shutdown_response` (approve/reject). Wait for approval before calling `TeamDelete`.

---

### TaskUpdate — Manage tasks

#### Assign a task to a teammate

```json
{
  "taskId": "1",
  "owner": "frontend-researcher",
  "status": "in_progress"
}
```

#### Mark task complete

```json
{
  "taskId": "1",
  "status": "completed"
}
```

#### Set dependencies

```json
{
  "taskId": "3",
  "addBlockedBy": ["1", "2"]
}
```

Task #3 cannot start until #1 and #2 are completed.

---

### TaskList — Check team progress

```json
{}
```

Returns all tasks with: id, subject, status, owner, blockedBy. Use this to monitor progress and find unassigned work.

---

### TeamDelete — Cleanup after shutdown

```json
{}
```

Removes team config and task directories. **Only call after all teammates have confirmed shutdown** via `shutdown_response(approve)`.

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

## Prompt Authoring for Teammates

The `prompt` parameter is the ONLY context the teammate receives. It must be self-contained.

### Required sections in every teammate prompt

```
You are {role-name} on team {team-name}.

## Your Mission
{What to research/analyze — be specific about scope}

## Codebase Context
- Project: RalphX — Native Mac GUI for autonomous AI dev
- Frontend: React/TS in src/ (Zustand, TanStack Query, Tailwind)
- Backend: Rust/Tauri in src-tauri/ (Clean architecture, SQLite)
{Add domain-specific context relevant to this teammate's scope}

## Files to Investigate
{List specific directories and files — don't make them search blindly}

## Expected Output
1. {Specific deliverable — e.g., "List of existing patterns with file locations"}
2. {Specific deliverable — e.g., "Integration constraints affecting other teammates"}
3. {Specific deliverable — e.g., "Recommended approach with trade-offs"}

## When Done
1. Create a research artifact:
   Use the MCP tool `create_team_artifact` with:
   - session_id: "{session_id}"
   - title: "{Your Role} Research Findings"
   - content: "{your findings in markdown}"
   - artifact_type: "TeamResearch"

2. Send a summary message to the team lead:
   Use SendMessage with type: "message", recipient: "team-lead" (or whatever your name is)

3. Mark your task as completed:
   Use TaskUpdate with taskId: "{task_id}", status: "completed"
```

### Prompt tips

- **Be specific about files**: "Read `src/stores/ideationStore.ts`" not "find the ideation store"
- **Set clear boundaries**: "Only investigate frontend hooks — the backend team handles Rust"
- **Include session_id**: Teammates need it for MCP tool calls
- **Include task_id**: So they can mark their task complete
- **Give context about other teammates**: "A backend-analyst is researching the Rust service layer. If you find cross-layer issues, message me."

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

## Complete Example: Research Team for a Feature

```
Phase 0: RECOVER — read this system card, check session state

Phase 2: TEAM COMPOSITION
  → Decided: 2 specialists (frontend-researcher, backend-analyst)
  → Call request_team_plan() for user approval
  → User approves (backend records the plan but does NOT spawn teammates)

Phase 3: EXPLORE (after approval)
  → Lead creates team and spawns teammates natively

  Step 1: TeamCreate
    team_name: "ideation-abc123"
    description: "Research team for user auth feature"

  Step 2: TaskCreate (x2)
    Task #1: "Research frontend auth patterns"
    Task #2: "Research backend auth service layer"

  Step 3: Task (spawn both in ONE message)
    Teammate 1: name="frontend-researcher", model="sonnet", task=#1
    Teammate 2: name="backend-analyst", model="sonnet", task=#2

  Step 4: Monitor
    - Read automatic messages from teammates
    - Relay cross-layer discoveries via SendMessage
    - Wait for both to mark tasks complete

Phase 4: PLAN
  - get_team_artifacts() to collect all TeamResearch
  - Synthesize into master plan
  - create_plan_artifact()

Phase 7: FINALIZE
  - SendMessage shutdown_request to each teammate
  - Wait for shutdown_response(approve) from each
  - TeamDelete
```

---

## Complete Example: Execution Team for a Task

Worker-team leads coordinate coder teammates for wave-based implementation. Unlike ideation teams (research-focused), execution teams write code with exclusive file ownership and validation gates.

```
Phase 0: RECOVER — read this system card, check team session state

Phase 1: ANALYZE
  → get_task_context(task_id) → full task details + plan artifact
  → get_artifact(plan_artifact_id) → extract implementation plan for this task
  → get_project_analysis(project_id, task_id) → validation commands + environment baseline

Phase 2: DECOMPOSE
  → Break task into atomic sub-scopes with file ownership:
     Scope 1: API types (src/types/auth.ts) — Wave 1
     Scope 2: Backend handlers (src-tauri/src/http_server/handlers/auth.rs) — Wave 1
     Scope 3: React hooks (src/hooks/useAuth.ts) — Wave 2 (depends on #1, #2)
     Scope 4: Tests (tests/auth.test.ts) — Wave 3 (depends on all)

Phase 3: APPROVE
  → request_team_plan(process="worker-execution", teammates=[
       { role: "coder-1", model: "sonnet", prompt_summary: "Implement API types" },
       { role: "coder-2", model: "sonnet", prompt_summary: "Implement backend handlers" }
     ])
  → User approves in UI → proceed

Phase 4: EXECUTE (wave-by-wave)

  -- Wave 1 --
  Step 1: TeamCreate
    team_name: "task-42"
    description: "Execution team for user auth"

  Step 2: TaskCreate (x2)
    Task #1: "Implement API types" (file ownership: src/types/auth.ts)
    Task #2: "Implement backend handlers" (file ownership: src-tauri/.../auth.rs)

  Step 3: Spawn coders FOREGROUND (NO run_in_background — MCP requires it)
    Coder 1: name="coder-1", model="sonnet", mode="bypassPermissions"
      → Coder calls start_step(task_id, "Implement auth types")
      → Coder writes code, calls complete_step(task_id, "Implement auth types")
      → Coder sends message to lead: "Auth types done, exported UserResponse"
      → Coder marks task completed
    Coder 2: name="coder-2", model="sonnet", mode="bypassPermissions"
      → Same flow with start_step / complete_step for progress tracking

  Step 4: Wave validation gate
    → get_project_analysis(project_id, task_id) → get validation commands
    → Run: npm run typecheck, cargo test --lib
    → All pass → proceed to Wave 2
    → Any fail → create fix tasks, spawn fix coders, re-validate (max 3 attempts)

  -- Wave 2 --
  Step 5: TaskCreate for dependent scopes (React hooks)
    → Spawn coder-3 with context: "Wave 1 outputs: auth types at src/types/auth.ts"
    → Coder uses start_step / complete_step for progress
    → Wave validation gate (same as Step 4)

  -- Wave 3 --
  Step 6: TaskCreate for tests
    → Spawn coder-4
    → Final wave validation gate

Phase 5: VALIDATE (final)
  → Run ALL validation: typecheck + lint + tests
  → Gate passes → proceed to COMPLETE

Phase 6: COMPLETE
  → Mark task complete via MCP
  → SendMessage shutdown_request to each coder
  → Wait for shutdown_response(approve) from each
  → TeamDelete
  → Provide execution summary (waves, coders, files modified, validation status)
```

**Key differences from ideation teams:**

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
