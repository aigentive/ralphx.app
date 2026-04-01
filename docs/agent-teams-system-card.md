# Agent Teams System Card for RalphX

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

**Status:** Reference document — describes how Claude Code Agent Teams map to RalphX's architecture.
**Last Updated:** 2026-02-15

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture Mapping](#2-architecture-mapping)
3. [Team Lifecycle](#3-team-lifecycle)
4. [Tool Reference](#4-tool-reference)
5. [Communication Patterns](#5-communication-patterns)
6. [Task Coordination](#6-task-coordination)
7. [Permission Model](#7-permission-model)
8. [Context Management](#8-context-management)
9. [Display Modes](#9-display-modes)
10. [Delegate Mode](#10-delegate-mode)
11. [Quality Gates](#11-quality-gates)
12. [Token Usage & Cost Optimization](#12-token-usage--cost-optimization)
13. [Known Limitations](#13-known-limitations)
14. [RalphX-Specific Considerations](#14-ralphx-specific-considerations)
15. [CLI Reference for Team Spawning](#15-cli-reference-for-team-spawning)

---

## 1. Overview

### What Are Agent Teams?

Claude Code Agent Teams coordinate **multiple independent Claude Code instances** working as a team. One session acts as **team lead** (orchestrates, assigns tasks, synthesizes results), while **teammates** work independently in their own context windows, communicating via a shared messaging system.

| Feature | Subagents (Current RalphX) | Agent Teams (Potential) |
|---------|---------------------------|------------------------|
| Context | Own window, results return to caller | Own window, fully independent |
| Communication | Report back to parent only | Teammates message each other directly |
| Coordination | Parent manages all work | Shared task list with self-coordination |
| Token cost | Lower (results summarized) | Higher (each teammate = separate instance) |
| Best for | Focused tasks (coder execution) | Complex work requiring discussion/collaboration |

### How This Maps to RalphX

RalphX already has a sophisticated multi-agent architecture:
- **20 agent types** defined in `ralphx.yaml` (see `docs/architecture/agent-catalog.md`)
- **Background CLI spawning** via `ClaudeCodeClient` (not tmux)
- **MCP-based tool scoping** (three-layer allowlist)
- **24-state task state machine** with auto-transitions
- **Worker → Coder parallel delegation** (up to 3 concurrent coders per wave)

Agent Teams would add a **peer-to-peer communication layer** that RalphX's current subagent-based delegation lacks. Instead of worker → coder being a parent → child relationship with results flowing back, team members could independently coordinate, share findings, and challenge each other's approaches.

### Enabling Agent Teams

```json
// settings.json (or via CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1 env var)
{
  "env": {
    "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
  }
}
```

⚠️ **Experimental feature** — disabled by default. Has known limitations around session resumption, task coordination, and shutdown behavior.

---

## 2. Architecture Mapping

### Agent Teams Components → RalphX Equivalents

| Agent Teams Component | Role | RalphX Equivalent | Notes |
|----------------------|------|-------------------|-------|
| **Team Lead** | Creates team, spawns teammates, coordinates work | `ralphx-worker` or `orchestrator-ideation` | The lead is the main Claude Code session |
| **Teammates** | Separate Claude Code instances working on tasks | `ralphx-coder` instances (currently subagents) | Each teammate has own context window |
| **Task List** | Shared work items teammates claim and complete | RalphX task board (SQLite) + MCP tools | Agent teams use `~/.claude/tasks/{team}/` |
| **Mailbox** | Messaging between agents | Not present in current architecture | New capability agent teams would add |

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Agent Teams Architecture                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────┐                                        │
│  │     Team Lead        │   ← Main Claude Code session           │
│  │  (orchestrator)      │   ← Creates team via TeamCreate        │
│  │                      │   ← Spawns teammates via Task tool     │
│  │  Tools:              │   ← Assigns tasks via TaskCreate       │
│  │  - TeamCreate        │   ← Sends messages via SendMessage     │
│  │  - TeamDelete        │   ← Shuts down via shutdown_request    │
│  │  - SendMessage       │                                        │
│  │  - TaskCreate/Update │                                        │
│  │  - Task (spawn)      │                                        │
│  └──────────┬───────────┘                                        │
│             │                                                    │
│    ┌────────┼────────┐                                           │
│    │        │        │  Message delivery (automatic)              │
│    ▼        ▼        ▼                                           │
│  ┌─────┐ ┌─────┐ ┌─────┐                                       │
│  │ T1  │ │ T2  │ │ T3  │  ← Teammates (own context windows)     │
│  │     │ │     │ │     │  ← Each has CLAUDE.md, MCP, skills      │
│  │     │←→│     │←→│     │  ← Can DM each other directly         │
│  └─────┘ └─────┘ └─────┘                                       │
│                                                                  │
│  ┌──────────────────────────────────────────┐                   │
│  │         Shared Task List                  │                   │
│  │  ~/.claude/tasks/{team-name}/             │                   │
│  │  - Tasks with status, owner, blockedBy    │                   │
│  │  - File-locked claiming (no races)        │                   │
│  └──────────────────────────────────────────┘                   │
│                                                                  │
│  ┌──────────────────────────────────────────┐                   │
│  │         Team Config                       │                   │
│  │  ~/.claude/teams/{team-name}/config.json  │                   │
│  │  - members[]: name, agentId, agentType    │                   │
│  └──────────────────────────────────────────┘                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Current RalphX Spawning Pipeline (for reference)

```
TaskEvent (Schedule/Execute)
    ↓
TransitionHandler → entering_executing()
    ↓
AgenticClientSpawner::spawn_agent()
    ↓ Creates worktree/branch
    ↓ Resolves CWD
    ↓ Builds CLI command from ralphx.yaml config
ClaudeCodeClient::spawn_agent_streaming()
    ↓ claude --plugin-dir ./ralphx-plugin --agent <name> ...
    ↓ --model <model> --tools <cli_tools> --allowedTools <mcp+cli>
    ↓ --append-system-prompt-file <prompt.md>
    ↓ --mcp-config <ralphx-mcp-server> --permission-mode <mode>
    ↓
StreamProcessor → parses stdout JSON → ChatMessage persisted
```

---

## 3. Team Lifecycle

### 3.1 Team Creation

| Step | Tool | What Happens |
|------|------|-------------|
| 1. Create team | `TeamCreate` | Creates `~/.claude/teams/{name}/config.json` + `~/.claude/tasks/{name}/` |
| 2. Create tasks | `TaskCreate` | Adds work items to shared task list |
| 3. Spawn teammates | `Task` (with `team_name`) | Launches separate Claude Code instances |
| 4. Assign tasks | `TaskUpdate` (set `owner`) | Lead assigns or teammates self-claim |

### 3.2 Execution Phase

```
Lead creates team + tasks
    ↓
Lead spawns teammates (Task tool with team_name param)
    ↓
Teammates load project context (CLAUDE.md, MCP, skills)
    ↓
Teammates check TaskList → claim unassigned, unblocked tasks
    ↓
Teammates work independently in own context windows
    ↓
Teammates send messages to lead/each other via SendMessage
    ↓
Teammates mark tasks completed via TaskUpdate
    ↓
Lead synthesizes findings, creates new tasks if needed
```

### 3.3 Shutdown & Cleanup

| Step | Tool | Details |
|------|------|---------|
| Request shutdown | `SendMessage` (type: `shutdown_request`) | Lead asks teammate to stop |
| Approve/reject | `SendMessage` (type: `shutdown_response`) | Teammate approves (exits) or rejects (keeps working) |
| Clean up team | `TeamDelete` | Removes team + task directories. **Fails if teammates still active** |

### 3.4 State Diagram

```
                    ┌──────────────┐
                    │  No Team     │
                    └──────┬───────┘
                           │ TeamCreate
                           ▼
                    ┌──────────────┐
                    │  Team Active │ ← Task tool spawns teammates
                    │              │ ← TaskCreate/Update manages work
                    │              │ ← SendMessage coordinates
                    └──────┬───────┘
                           │ All teammates shutdown_response(approve)
                           ▼
                    ┌──────────────┐
                    │  All Stopped │
                    └──────┬───────┘
                           │ TeamDelete
                           ▼
                    ┌──────────────┐
                    │  Cleaned Up  │
                    └──────────────┘
```

---

## 4. Tool Reference

### 4.1 TeamCreate

Creates a new team with shared task list and config directory.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `team_name` | string | YES | Unique team identifier |
| `description` | string | No | Team purpose |
| `agent_type` | string | No | Type/role of team lead |

**Creates:**
- `~/.claude/teams/{team_name}/config.json` — team metadata + members array
- `~/.claude/tasks/{team_name}/` — shared task list directory

**Config file structure:**
```json
{
  "members": [
    {
      "name": "team-lead",
      "agentId": "team-lead@my-project",
      "agentType": "coordinator"
    }
  ]
}
```

### 4.2 TeamDelete

Removes team and task directories. **Must shut down all teammates first.**

| Behavior | Details |
|----------|---------|
| Prereq | All members must be stopped (no active teammates) |
| Removes | `~/.claude/teams/{team-name}/` + `~/.claude/tasks/{team-name}/` |
| Clears | Team context from current session |
| Failure | Returns error if active members exist |

### 4.3 SendMessage

Inter-agent communication tool. Supports 5 message types:

#### type: "message" (Direct Message)

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | `"message"` | YES | DM to specific teammate |
| `recipient` | string | YES | Teammate **name** (not UUID — e.g., `"coder-1"`) |
| `content` | string | YES | Message text (plain text, not JSON) |
| `summary` | string | YES | 5-10 word preview for UI notification |

```json
// Example: Relay a discovery to a specific coder
{
  "type": "message",
  "recipient": "coder-2",
  "content": "Coder-1 found that UserService needs an async init() call before use. Update your handlers accordingly.",
  "summary": "Relay UserService init requirement"
}
```

#### type: "broadcast"

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | `"broadcast"` | YES | Send to ALL teammates |
| `content` | string | YES | Message text |
| `summary` | string | YES | 5-10 word preview |

⚠️ **Use sparingly** — each broadcast = N separate deliveries (one per teammate). Cost scales linearly with team size. Only for critical team-wide announcements.

```json
// Example: Critical pattern all coders must follow
{
  "type": "broadcast",
  "content": "IMPORTANT: All error handlers must use AppResult<T>, not raw Result. See src/domain/errors.rs for the type.",
  "summary": "Critical error handling pattern update"
}
```

#### type: "shutdown_request"

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | `"shutdown_request"` | YES | Ask teammate to stop |
| `recipient` | string | YES | Teammate name |
| `content` | string | No | Reason for shutdown |

```json
// Example: Gracefully shut down a coder after all tasks complete
{
  "type": "shutdown_request",
  "recipient": "coder-1",
  "content": "All tasks complete, wrapping up the team session"
}
```

The teammate receives a JSON message with `type: "shutdown_request"` and a `requestId`. They MUST respond with `shutdown_response` — simply acknowledging in text is not sufficient.

#### type: "shutdown_response"

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | `"shutdown_response"` | YES | Respond to shutdown request |
| `request_id` | string | YES | Extract `requestId` from incoming shutdown request JSON |
| `approve` | boolean | YES | `true` = exit process, `false` = keep working |
| `content` | string | No (required if reject) | Reason for rejection |

```json
// Example: Approve shutdown
{ "type": "shutdown_response", "request_id": "req-abc-123", "approve": true }

// Example: Reject shutdown (still working)
{
  "type": "shutdown_response",
  "request_id": "req-abc-123",
  "approve": false,
  "content": "Still finishing task #3, need 2 more minutes"
}
```

#### type: "plan_approval_response"

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `type` | `"plan_approval_response"` | YES | Approve/reject teammate's plan |
| `request_id` | string | YES | Extract `requestId` from `plan_approval_request` message |
| `recipient` | string | YES | Teammate name |
| `approve` | boolean | YES | `true` = approve (exits plan mode), `false` = reject with feedback |
| `content` | string | No (required if reject) | Feedback — teammate stays in plan mode and revises |

```json
// Example: Approve a plan
{
  "type": "plan_approval_response",
  "request_id": "req-xyz-789",
  "recipient": "coder-1",
  "approve": true
}

// Example: Reject with feedback
{
  "type": "plan_approval_response",
  "request_id": "req-xyz-789",
  "recipient": "coder-1",
  "approve": false,
  "content": "Missing error handling for the database connection. Add retry logic with exponential backoff."
}
```

### 4.4 Task Management Tools

#### TaskCreate

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `subject` | string | YES | Brief imperative title (e.g., "Fix auth bug") |
| `description` | string | YES | Detailed requirements |
| `activeForm` | string | Recommended | Present continuous form for spinner (e.g., "Fixing auth bug") |
| `metadata` | object | No | Arbitrary key-value metadata |

All tasks created with `status: "pending"` and no owner.

#### TaskUpdate

| Parameter | Type | Description |
|-----------|------|-------------|
| `taskId` | string | YES — task to update |
| `status` | enum | `pending` → `in_progress` → `completed` (or `deleted`) |
| `owner` | string | Agent name claiming the task |
| `subject` | string | Update title |
| `description` | string | Update description |
| `activeForm` | string | Update spinner text |
| `addBlocks` | string[] | Tasks that cannot start until this completes |
| `addBlockedBy` | string[] | Tasks that must complete before this starts |
| `metadata` | object | Merge metadata (set key to `null` to delete) |

#### TaskGet

Returns full task details: subject, description, status, blocks, blockedBy.

#### TaskList

Returns summary of all tasks: id, subject, status, owner, blockedBy.

**Teammate workflow after completing a task:**
1. `TaskUpdate` → mark completed
2. `TaskList` → find available work
3. Look for: `status: "pending"`, no owner, empty `blockedBy`
4. **Prefer lowest ID first** (earlier tasks set up context for later ones)
5. `TaskUpdate` → claim with `owner: "my-name"`

### 4.5 Task Tool (Spawning Teammates)

The `Task` tool spawns teammates when used with `team_name` parameter.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `prompt` | string | YES | Task description + context for teammate |
| `subagent_type` | string | YES | Agent type (e.g., `general-purpose`, `Explore`, `Plan`) |
| `team_name` | string | YES (for teams) | Team to join |
| `name` | string | YES (for teams) | Human-readable teammate name |
| `model` | enum | No | `sonnet`, `opus`, `haiku`, `inherit` |
| `mode` | enum | No | Permission mode: `default`, `plan`, `acceptEdits`, etc. |
| `run_in_background` | boolean | No | Run asynchronously |
| `max_turns` | integer | No | Limit agentic turns |

#### subagent_type → Available Tools Mapping

| `subagent_type` | Tools Available | Best For |
|-----------------|----------------|----------|
| `general-purpose` | **All tools**: Read, Write, Edit, Bash, Glob, Grep, Task, WebFetch, WebSearch | Implementation, full-capability work |
| `Explore` | **Read-only**: Read, Glob, Grep, WebFetch, WebSearch (NO Edit, Write, Bash) | Research, codebase exploration |
| `Plan` | **Read-only**: Read, Glob, Grep, WebFetch, WebSearch (NO Edit, Write, Bash) | Architecture planning, design |
| `Bash` | **Bash only**: Bash | Command execution, git operations |

**Critical:** `Explore` and `Plan` agents **cannot edit files**. Never assign implementation tasks to these types.

#### Key Differences from Regular Subagent Spawning

| Aspect | Regular Subagent (Task tool) | Team Teammate (Task + team_name) |
|--------|------------------------------|----------------------------------|
| Registration | Not registered anywhere | Added to `~/.claude/teams/{name}/config.json` |
| Messaging | Cannot receive messages | Can be messaged by name |
| Task list | No shared task list | Access to `~/.claude/tasks/{team}/` |
| Lifecycle | Runs to completion, returns result | Goes idle between turns, stays alive |
| Notifications | Result returned to caller | Idle notifications sent to lead automatically |
| Peer communication | None | Can DM other teammates directly |

#### Spawn Example

```json
// Lead spawns a coder teammate
{
  "prompt": "Implement the user API endpoint. Files you own: src/api/users.ts, src/api/users.test.ts",
  "subagent_type": "general-purpose",
  "team_name": "task-abc123",
  "name": "coder-1",
  "model": "sonnet",
  "mode": "acceptEdits"
}
```

---

## 5. Communication Patterns

### 5.1 Message Types & When to Use

| Pattern | Tool Call | When to Use |
|---------|----------|-------------|
| **Direct Message** | `SendMessage(type: "message", recipient: "name")` | Default for all communication |
| **Broadcast** | `SendMessage(type: "broadcast")` | Critical team-wide announcements only |
| **Shutdown Request** | `SendMessage(type: "shutdown_request")` | Gracefully end a teammate |
| **Plan Approval** | `SendMessage(type: "plan_approval_response")` | Review teammate's plan |

### 5.2 Automatic Message Delivery

Messages from teammates are **automatically delivered** to recipients. No polling needed.

| Scenario | Behavior |
|----------|----------|
| Recipient is active (mid-turn) | Message queued, delivered when turn ends |
| Recipient is idle | Message delivered immediately, wakes recipient |
| Lead receives teammate message | Appears as new conversation turn |
| UI notification | Brief preview with sender name shown |

### 5.3 Idle Notifications

Teammates go idle after **every turn** — this is normal and expected.

| Key Point | Detail |
|-----------|--------|
| **Idle = waiting for input** | NOT done or unavailable |
| **Can receive messages while idle** | Sending a message wakes them and they resume |
| **Automatic notification** | System notifies lead on teammate idle |
| **Peer DM visibility** | Brief summary of peer-to-peer DMs included in idle notification |
| **Don't react to idle** | Unless you want to assign new work or follow up |

#### Idle Notification Format

The lead receives an automatic system message when a teammate goes idle:

```
[system] Teammate "coder-1" is now idle.
         Last action: Completed task #3 "Implement user API"
         Peer DMs: Sent message to coder-2 about shared types
```

#### When to Act on Idle vs Ignore

| Scenario | Action |
|----------|--------|
| Teammate just sent you a message and went idle | **Ignore idle** — they're waiting for your response. Reply via SendMessage |
| Teammate completed their task and went idle | **Check TaskList** — assign new work or send shutdown_request |
| Teammate idle with no tasks claimed | **Assign work** — send message with instructions or create TaskUpdate with owner |
| Teammate idle after sending peer DM | **Ignore** — normal flow, they're waiting for peer response |
| All teammates idle, all tasks complete | **Shut down team** — send shutdown_requests to all teammates |

### 5.4 Common Communication Workflows (with examples)

#### Nudging an Idle Teammate

```json
// Teammate went idle without completing their task — send a follow-up
{
  "type": "message",
  "recipient": "coder-2",
  "content": "Task #2 is still in_progress. Are you blocked on something? Let me know if you need help.",
  "summary": "Check on coder-2 progress"
}
```

#### Reassigning Work

```json
// Step 1: Update task ownership
TaskUpdate({ "taskId": "3", "owner": "coder-1" })

// Step 2: Notify the new owner
{
  "type": "message",
  "recipient": "coder-1",
  "content": "I've reassigned task #3 (implement auth middleware) to you. Coder-2 was stuck on it. Check TaskGet for full details.",
  "summary": "Reassigned task #3 to coder-1"
}
```

#### Relaying a Discovery Between Coders

```json
// Coder-1 found something that affects coder-2's work
{
  "type": "message",
  "recipient": "coder-2",
  "content": "Coder-1 discovered that the database schema changed — the 'users' table now has a 'role' column. Update your queries to include it.",
  "summary": "Relay schema change from coder-1"
}
```

#### Unblocking a Task

```json
// Step 1: Complete the blocking task
TaskUpdate({ "taskId": "1", "status": "completed" })
// System auto-unblocks task #2 (which had addBlockedBy: ["1"])

// Step 2: Notify the teammate waiting on it
{
  "type": "message",
  "recipient": "coder-2",
  "content": "Task #1 is done — task #2 is now unblocked. You can start working on it. Check TaskList for details.",
  "summary": "Task #2 unblocked, ready to start"
}
```

#### Checking on Progress

```json
// Ask a specific teammate for status (instead of using terminal tools)
{
  "type": "message",
  "recipient": "coder-3",
  "content": "Quick status check — how's task #5 going? Any blockers?",
  "summary": "Status check on task #5"
}
```

#### Broadcasting a Critical Stop

```json
// Blocking issue found — all work must stop
{
  "type": "broadcast",
  "content": "STOP: Found a critical bug in the base types. Do NOT commit anything until I send an all-clear. I'm fixing src/types/base.ts now.",
  "summary": "Critical blocking issue — stop all work"
}
```

### 5.5 Communication Anti-Patterns

| ❌ Don't | ✅ Do Instead |
|----------|--------------|
| Use broadcast for routine updates | Use direct message to specific teammate |
| Send structured JSON status messages | Use TaskUpdate for status, plain text for messages |
| Ignore teammate messages | Read and respond or acknowledge |
| Treat idle as error/completion | Send new message to continue work |
| Use terminal tools to check activity | Send message to teammate directly |
| React to every idle notification | Only act when you have new work to assign |
| Send empty "acknowledged" messages | Only message when you have actionable content |

---

## 6. Task Coordination

### 6.1 Shared Task List

All team members access the same task list stored at `~/.claude/tasks/{team-name}/`.

| Feature | Details |
|---------|---------|
| Location | `~/.claude/tasks/{team-name}/` |
| Access | All teammates can read, claim, create, update |
| Status flow | `pending` → `in_progress` → `completed` |
| Dependencies | `blocks` / `blockedBy` arrays between tasks |
| Claiming | File locking prevents race conditions |
| Auto-unblock | When a task completes, blocked tasks auto-unblock |

### 6.2 Task Claiming Protocol

```
1. Teammate calls TaskList → sees available tasks
2. Finds task: status=pending, no owner, empty blockedBy
3. Calls TaskUpdate(taskId, owner: "my-name", status: "in_progress")
4. File lock ensures only one teammate claims each task
5. Works on task, marks completed
6. Calls TaskList → finds next available task
```

### 6.3 Dependency Management

| Operation | Tool | Example |
|-----------|------|---------|
| Create dependency | `TaskUpdate(taskId: "2", addBlockedBy: ["1"])` | Task 2 waits for task 1 |
| Block downstream | `TaskUpdate(taskId: "1", addBlocks: ["2", "3"])` | Tasks 2,3 wait for task 1 |
| Auto-resolution | System handles | When task 1 completes, tasks 2,3 auto-unblock |

### 6.4 Comparison: Agent Teams Tasks vs RalphX Tasks

| Aspect | Agent Teams TaskList | RalphX Kanban Tasks |
|--------|---------------------|-------------------|
| Storage | `~/.claude/tasks/` (flat files) | SQLite database |
| States | 3 (pending, in_progress, completed) | 24 (backlog→merged) |
| Dependencies | blockedBy/blocks arrays | Task dependency manager + auto-unblock |
| Agent assignment | `owner` field, self-claim | State machine auto-spawns on transition |
| UI | CLI task list, Ctrl+T toggle | Full Kanban board with drag-drop |
| Coordination | Peer-to-peer messaging | Hub-and-spoke via TransitionHandler |

---

## 7. Permission Model

### 7.1 Permission Inheritance

| Rule | Details |
|------|---------|
| **Inheritance** | Teammates start with lead's permission settings |
| **bypassPermissions** | If lead has it, ALL teammates do too (can't override) |
| **Per-teammate override** | Can change individual modes AFTER spawning |
| **Spawn-time restriction** | Cannot set per-teammate modes at spawn time |

### 7.2 Plan Mode for Teammates

Teammates can be required to plan before implementing:

```
Lead spawns teammate with mode: "plan"
    ↓
Teammate works in read-only plan mode
    ↓
Teammate calls ExitPlanMode → sends plan_approval_request to lead
    ↓
Lead reviews plan → SendMessage(type: "plan_approval_response")
    ↓
If approved: teammate exits plan mode, begins implementation
If rejected: teammate stays in plan mode, revises based on feedback
```

### 7.3 RalphX Permission Context

| RalphX Agent | Current Permission | Agent Teams Equivalent |
|-------------|-------------------|----------------------|
| `ralphx-worker` | `acceptEdits` (Write/Edit/Bash pre-approved) | Lead would inherit this |
| `ralphx-coder` | `acceptEdits` | Teammate would inherit from lead |
| `ralphx-reviewer` | `default` (Read-only focus) | Could use `plan` mode |
| `ralphx-merger` | `default` + Read, Edit, Bash preapproved | Custom allowed tools |

### 7.4 RalphX Permission Bridge

RalphX has its own permission system independent of Claude Code's:

```
Agent calls permission_request MCP tool
    ↓
MCP Server: POST /api/permission/request → returns request_id
    ↓
Tauri Backend: create dialog, emit event
    ↓
Frontend: show PermissionDialog to user
    ↓
User Allow/Deny → resolve_permission_request command
    ↓
Backend signals MCP → returns decision to agent
```

This bridge would function the same whether agents are subagents or team members, since MCP tools are available in both modes.

---

## 8. Context Management

### 8.1 What Teammates Receive

| Receives | Details |
|----------|---------|
| ✅ CLAUDE.md | Loaded from working directory (same as regular session) |
| ✅ MCP servers | All configured MCP servers available |
| ✅ Skills | Loaded at session start |
| ✅ Spawn prompt | Task description from lead |
| ✅ Project context | .claude/rules/, settings, etc. |
| ❌ Lead's conversation history | Does NOT carry over |
| ❌ Other teammates' context | Each has own isolated window |

### 8.2 Context Implications for RalphX

| Scenario | Impact | Mitigation |
|----------|--------|-----------|
| Worker spawns coders as teammates | Coders don't see worker's analysis | Include detailed scope in spawn prompt |
| Reviewer needs task context | Must call `get_task_context` MCP tool | MCP tools work in team mode |
| Memory system | Each teammate loads memories independently | `search_memories` MCP tool available |
| Session recovery | Agent teams don't support /resume | RalphX's own session recovery works per-agent |

### 8.3 RalphX MCP Tools in Team Context

RalphX's MCP server (`ralphx-mcp-server`) runs per-agent and would work the same in team mode:

```
Teammate Claude Instance
    ↓ MCP Protocol (JSON-RPC over stdio)
ralphx-mcp-server (Node.js) — reads --agent-type from argv
    ↓ HTTP to :3847
Tauri Backend (Rust) — shared database, same state
    ↓
Business Logic + SQLite
```

**Key consideration:** MCP communication uses stdio (JSON-RPC), which works in both foreground and background. The actual limitation for background teammates (`run_in_background: true`) is **user interaction for permission prompts** — if a tool requires user approval and the teammate is in background, the permission dialog cannot be presented. MCP tools that don't require user approval work fine in background mode.

---

## 9. Display Modes

### 9.1 Available Modes

| Mode | Description | RalphX Relevance |
|------|-------------|-----------------|
| **Auto** (default) | Uses split panes if in tmux/iTerm2, else in-process | Default — falls back to in-process for RalphX |
| **In-process** | All teammates run inside main terminal | Primary mode for RalphX (explicit) |
| **Split panes** (tmux/iTerm2) | Each teammate gets own pane | Not applicable — RalphX spawns background CLI processes |

### 9.2 RalphX Display Considerations

RalphX spawns Claude CLI as **background processes** (not within tmux), so:

| Aspect | Details |
|--------|---------|
| **Display mode** | In-process only (no tmux integration) |
| **Teammate visibility** | Shift+Up/Down to cycle teammates |
| **Direct messaging** | Type to send message to selected teammate |
| **Task list** | Ctrl+T to toggle |
| **Enter teammate session** | Press Enter to view, Escape to interrupt |

### 9.3 Configuring Display Mode

```json
// settings.json
{ "teammateMode": "in-process" }
```

Or per-session:
```bash
claude --teammate-mode in-process
```

---

## 10. Delegate Mode

### 10.1 What It Does

Delegate mode restricts the lead to **coordination-only tools**:
- Spawning teammates
- Sending messages
- Managing tasks
- Shutting down teammates

The lead **cannot** touch code directly — all implementation is delegated.

### 10.2 Activation

Press **Shift+Tab** to cycle into delegate mode after starting a team.

### 10.3 RalphX Mapping

| RalphX Pattern | Agent Teams Equivalent |
|----------------|----------------------|
| `ralphx-worker` orchestrates, `ralphx-coder` implements | Lead in delegate mode + coder teammates |
| `orchestrator-ideation` plans, workers execute | Lead in delegate mode + researcher/worker teammates |
| `ralphx-orchestrator` coordinates complex tasks | Natural fit for delegate mode lead |

### 10.4 When to Use

| ✅ Use Delegate Mode | ❌ Don't Use |
|---------------------|-------------|
| Worker managing multiple coders | Single-agent execution |
| Research with competing hypotheses | Simple, sequential tasks |
| Cross-layer coordination (frontend + backend + tests) | Same-file edits |
| Lead starts implementing instead of waiting | Tasks are truly sequential |

---

## 11. Quality Gates

### 11.1 TeammateIdle Hook

Fires when a teammate is about to go idle. Exit code 2 sends feedback and keeps teammate working.

```json
{
  "hooks": {
    "TeammateIdle": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "./scripts/check-teammate-progress.sh"
          }
        ]
      }
    ]
  }
}
```

**RalphX use case:** Ensure worker checks all steps completed before going idle. Verify coders ran validation commands.

### 11.2 TaskCompleted Hook

Fires when a task is being marked complete. Exit code 2 prevents completion and sends feedback.

```json
{
  "hooks": {
    "TaskCompleted": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "./scripts/validate-task-completion.sh"
          }
        ]
      }
    ]
  }
}
```

**RalphX use case:** Verify all validation commands pass (cargo check, npm run typecheck) before marking task done. Check that all MCP steps are marked completed.

### 11.3 RalphX's Existing Quality Gates (for reference)

RalphX already has quality enforcement:

| Gate | Current Mechanism | Agent Teams Enhancement |
|------|------------------|----------------------|
| Pre-completion validation | Worker/Coder: run all validate commands | TaskCompleted hook as additional check |
| Review requirement | State machine: executing → reviewing (mandatory) | Could add review teammate to team |
| Wave gates | Worker validates each coder wave | TeammateIdle hook to verify wave results |
| Issue tracking | MCP tools: mark_issue_in_progress/addressed | TaskCompleted hook to check all issues resolved |
| Supervisor monitoring | `ralphx-supervisor` detects loops/stalls | TeammateIdle hook as lightweight alternative |

### 11.4 Combining RalphX Hooks with Team Hooks

RalphX's plugin hooks (`ralphx-plugin/hooks/hooks.json`) and team hooks can coexist:

| Hook Source | When Active |
|------------|-------------|
| Plugin hooks | Always (loaded with plugin) |
| Settings hooks | Always (from settings.json) |
| Team hooks | Only during team sessions |
| Agent frontmatter hooks | Only while specific agent active |

All matching hooks run in parallel.

---

## 12. Token Usage & Cost Optimization

### 12.1 Cost Model

| Factor | Impact |
|--------|--------|
| **Each teammate = separate context window** | N teammates ≈ N× token usage |
| **Message delivery** | Each message adds to recipient's context |
| **Broadcasts** | N messages for N teammates |
| **CLAUDE.md loading** | Each teammate loads project context |
| **MCP tool calls** | Each teammate uses tokens for tool IO |

### 12.2 Optimization Strategies

| Strategy | Details |
|----------|---------|
| **Minimize broadcasts** | Use direct messages (DMs) instead |
| **Right-size teams** | 2-4 teammates for most tasks |
| **Use cheaper models** | Haiku for simple tasks, Sonnet for implementation |
| **Scope prompts tightly** | Give teammates focused scope, not broad instructions |
| **Limit max_turns** | Set reasonable turn limits to prevent runaway |
| **Use subagents for focused tasks** | Not everything needs a team |

### 12.3 RalphX-Specific Cost Considerations

| Current Pattern | Token Cost | Agent Teams Cost | Recommendation |
|----------------|-----------|-----------------|----------------|
| Worker → 3 Coders (subagents) | 4 context windows, results summarized back | 4 context windows, peer messaging overhead | Similar cost, teams add coordination overhead |
| Orchestrator-Ideation → 3 Explore (subagents) | 4 windows, explore results summarized | 4 windows + messaging | Subagents more efficient for fire-and-forget research |
| Complex cross-layer feature | Sequential: 1 window | Parallel team: 3-4 windows | Teams worth it for parallelism savings |
| Code review with competing analyses | 1 sequential reviewer | 3 parallel reviewers | Teams worth it if review quality matters |

### 12.4 Model Selection per Role

| Role | Recommended Model | Reasoning |
|------|------------------|-----------|
| Team lead (coordinator) | sonnet | Coordination doesn't need opus |
| Implementation teammate | sonnet | Good balance of capability/cost |
| Architecture/merge teammate | opus | Complex reasoning needed |
| Research teammate | sonnet or haiku | Depends on depth needed |
| Monitoring/validation teammate | haiku | Simple pattern matching |

---

## 13. Known Limitations

### 13.1 Official Limitations (from Claude Code docs)

| Limitation | Impact | Workaround |
|-----------|--------|-----------|
| **No session resumption** | `/resume` and `/rewind` don't restore in-process teammates. Lead may message non-existent teammates | Tell lead to spawn new teammates after resume |
| **Task status can lag** | Teammates sometimes fail to mark tasks completed, blocking dependents | Manually update task status or nudge teammate |
| **Shutdown can be slow** | Teammates finish current request/tool call first | Be patient; set reasonable max_turns |
| **One team per session** | Lead manages one team at a time | Clean up before starting new team |
| **No nested teams** | Teammates cannot spawn their own teams | Only lead manages the team |
| **Fixed lead** | Cannot promote teammate or transfer leadership | Design lead role carefully upfront |
| **Permissions set at spawn** | All teammates start with lead's permission mode | Change individual modes after spawning |
| **Split panes require tmux/iTerm2** | VS Code terminal, Windows Terminal, Ghostty not supported | Use in-process mode |

### 13.2 RalphX-Specific Limitations

| Limitation | Impact | Mitigation |
|-----------|--------|-----------|
| **Background CLI spawning** | RalphX spawns Claude as background processes, not in terminal | In-process mode works; no visual panes |
| **Permission prompts in background** | Background teammates can't show user permission dialogs | Use `--dangerously-skip-permissions` or `acceptEdits` mode for background agents |
| **Existing state machine** | RalphX's 24-state machine manages agent lifecycle independently | Team tasks and RalphX tasks are separate systems |
| **Session recovery** | Agent teams have no session resumption; RalphX's session recovery is per-agent | Teams require re-creation after session loss |
| **Three-layer tool allowlist** | Each teammate needs proper tool scoping | Must configure ralphx.yaml, tools.ts, and frontmatter for each team role |
| **Settings profile propagation** | Settings profiles from ralphx.yaml may not propagate to teammates | Pass settings explicitly in spawn prompt or via --settings flag |
| **Commit lock** | `.commit-lock` system for parallel work | Team members must respect commit lock protocol |

---

## 14. RalphX-Specific Considerations

### 14.1 Integration Points

#### How RalphX Could Leverage Agent Teams

| Integration Point | Current Architecture | Agent Teams Enhancement |
|-------------------|---------------------|----------------------|
| **Worker → Coder delegation** | Subagent Task calls (max 3 parallel) | Team with coder teammates + peer messaging |
| **Ideation exploration** | 3× Task(Explore) subagents | Research team with competing hypotheses |
| **Code review** | Single `ralphx-reviewer` | Multiple reviewers (security, performance, tests) |
| **Cross-layer features** | Sequential execution per task | Frontend + backend + test teammates in parallel |
| **Complex debugging** | Single worker retries | Competing hypothesis investigation |

#### Implementation Considerations

| Aspect | Details |
|--------|---------|
| **CLI spawning** | `ClaudeCodeClient` currently spawns with `--agent` flag. Teams would need `--teammate-mode in-process` and team context |
| **MCP server** | Each teammate needs own MCP server instance (already per-process in current design) |
| **Tool scoping** | `ralphx.yaml` agent configs define tool allowlists per agent type — these translate to teammate capabilities |
| **Settings profiles** | `claude.settings_profiles` (default, z_ai) can be passed per-teammate via `--settings` |
| **Event bus** | RalphX's EventBus emits execution events. Team message delivery is separate (Claude Code internal) |

### 14.2 MCP Tool Availability in Teams

| Context | MCP Tools Available? | Details |
|---------|---------------------|---------|
| Foreground teammate | ✅ Yes | Full MCP access (same as subagent) |
| Background teammate | ✅ Yes (MCP via stdio) | MCP protocol works; limitation is **user permission prompts** cannot be presented |
| Lead in delegate mode | ✅ Yes (if not restricted) | Lead retains MCP access |

**Key consideration:** MCP tools use stdio (JSON-RPC), which works in both foreground and background modes. The limitation for background teammates is that **user-interactive permission dialogs** cannot be shown. For RalphX, where agents use `--dangerously-skip-permissions` or `acceptEdits` mode, this is typically not an issue — MCP tools will work fine in background.

### 14.3 Agent Type → Team Role Mapping (Templates)

The table below shows **template mappings** — predefined agent types that can serve as starting points. See Section 14.3.1 for dynamic role definition.

| RalphX Agent | Potential Team Role | Model | Key MCP Tools |
|-------------|--------------------|----|---------------|
| `ralphx-worker` | **Team Lead** (coordinator) | sonnet | step management, task context, issue tracking |
| `ralphx-coder` | **Teammate** (implementer) | sonnet | step management, task context, artifacts |
| `ralphx-reviewer` | **Teammate** (reviewer) | sonnet | review completion, task context |
| `orchestrator-ideation` | **Team Lead** (ideation) | opus | proposals, plans, analysis |
| `ralphx-deep-researcher` | **Teammate** (researcher) | opus | WebFetch, WebSearch, memories |
| `ralphx-supervisor` | **Teammate** (monitor) | haiku | pattern detection |
| `ralphx-merger` | **Standalone** (not team) | opus | merge target, conflict resolution |

#### 14.3.1 Dynamic Team Roles (Default Behavior)

Team leads **define teammate roles dynamically** based on the specific task rather than being limited to predefined agent configs from YAML. Predefined agents (above) are **templates**, not requirements.

| Aspect | Dynamic (Default) | Constrained (Opt-in) |
|--------|-------------------|---------------------|
| **Role definition** | Lead analyzes task, determines roles | Roles from `ralphx.yaml` agent configs |
| **Model selection** | Lead picks per-teammate (haiku→opus) | Constrained by `allowed_models` in YAML |
| **Team size** | Lead decides (typically 2-5) | Constrained by `max_teammates` in YAML |
| **Tool selection** | Lead decides per-scope | Constrained by `required_mcp_tools` in YAML |
| **Prompt crafting** | Lead writes custom prompts per task | Uses predefined agent system prompts |

**Why dynamic by default:** Tasks vary in structure — a UI task might need 2 frontend + 1 test writer, while an API task needs 1 backend + 1 integration tester. Rigid predefined configs can't anticipate this variety. The lead has full task context and makes better composition decisions.

**Constrained mode (opt-in):** For production guardrails, `ralphx.yaml` can constrain what the lead is allowed to spawn:

```yaml
team_constraints:
  ralphx-worker-team:
    max_teammates: 4
    allowed_models: [sonnet, haiku]           # Cost control: no opus teammates
    allowed_agent_types: [general-purpose]     # Restrict to full-capability agents
    required_mcp_tools: [start_step, complete_step]  # All teammates must have these
```

The configuration provides **boundaries**, not rigid definitions. The lead operates freely within these constraints.

### 14.4 Background Process Spawning Compatibility

RalphX spawns Claude CLI as background processes via Rust's `Command`:

```rust
// Current: ClaudeCodeClient spawns subagent
Command::new("claude")
    .arg("--plugin-dir").arg("./ralphx-plugin")
    .arg("--agent").arg(agent_name)
    .arg("-p").arg(prompt)
    // ... model, tools, allowedTools, mcp-config, etc.
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
```

For agent teams, the lead session **can** use `-p` (print mode). Teammates are spawned internally by the lead via the `Task` tool — they do NOT use `-p` themselves (they run as interactive teammate processes managed by the lead's session).

**Key insight:** The lead can be spawned with `-p "orchestrate this task"` and still create teams + spawn teammates. The `-p` flag affects the lead's interaction mode, not its ability to use team tools. Teammates are spawned WITHOUT `-p` — they are interactive Claude Code instances managed by the team framework.

### 14.5 Recommended Integration Strategy

| Phase | Approach | Scope |
|-------|----------|-------|
| **Phase 1: Research** | Use agent teams manually (human-in-loop) for ideation exploration and debugging | Manual, interactive sessions |
| **Phase 2: Worker Enhancement** | Replace worker → coder subagent pattern with team-based coordination | Execution phase only |
| **Phase 3: Cross-Layer** | Spawn frontend + backend + test teammates for feature development | Full feature lifecycle |
| **Phase 4: Full Integration** | Integrate team management into RalphX's state machine and Tauri backend | Automated team lifecycle |

### 14.6 Configuration Requirements

To enable agent teams in RalphX's spawned agents, **both** environment variables are required:

```yaml
# ralphx.yaml addition
claude:
  env:
    CLAUDECODE: "1"                                    # Required — signals Claude Code runtime
    CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS: "1"         # Required — enables agent teams feature

# settings profile
settings_profiles:
  default:
    env:
      CLAUDECODE: "1"
      CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS: "1"
    teammateMode: "in-process"
```

Both environment variables must be set in the spawning environment:

```rust
// ClaudeCodeClient::spawn_agent_streaming()
cmd.env("CLAUDECODE", "1");
cmd.env("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS", "1");
```

---

## 15. CLI Reference for Team Spawning

> See also: `docs/architecture/claude-spawning-system.md` for RalphX's full spawning pipeline.

### 15.1 Team-Specific CLI Flags

These flags are used internally when the team lead spawns teammates. They are NOT in the public CLI reference but are passed automatically by the Task tool's team spawning logic.

| Flag | Type | Description |
|------|------|-------------|
| `--agent-id` | string | Unique agent identifier, format: `{name}@{team-name}` (e.g., `wave-1@merge-hardening-tests`) |
| `--agent-name` | string | Human-readable name for the teammate (e.g., `wave-1`) |
| `--team-name` | string | Team identifier — matches `TeamCreate` name (e.g., `merge-hardening-tests`) |
| `--agent-color` | string | Terminal color for teammate output (e.g., `blue`, `green`, `yellow`) |
| `--parent-session-id` | UUID | Lead's session ID — links teammate to parent for messaging |
| `--agent-type` | string | Agent specialization type (e.g., `general-purpose`, `Explore`, `Plan`) |

### 15.2 Required Environment Variables

| Variable | Value | Purpose |
|----------|-------|---------|
| `CLAUDECODE` | `1` | Signals Claude Code runtime is active |
| `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` | `1` | Enables agent teams feature |

Both are **required** for team functionality.

### 15.3 Standard CLI Flags Used with Teams

| Flag | Role | Example |
|------|------|---------|
| `--dangerously-skip-permissions` | Bypass permission prompts (when pre-approved) | `--dangerously-skip-permissions` |
| `--model` | Model selection per teammate | `--model sonnet` |
| `--plugin-dir` | Plugin directory (for MCP/skills) | `--plugin-dir ./ralphx-plugin` |
| `--agent` | Agent definition (from plugin) | `--agent worker` |
| `-p` | Print mode (lead only — teammates never use) | `-p "Execute task"` |
| `--mcp-config` | MCP server configuration | `--mcp-config ./mcp.json` |
| `--permission-mode` | Permission level | `--permission-mode acceptEdits` |
| `--tools` | CLI tools to enable | `--tools Read,Write,Edit,Bash` |
| `--allowedTools` | MCP + CLI tools allowlist | `--allowedTools "mcp__ralphx__*"` |

### 15.4 Real Spawn Example

This is the actual command produced when a team lead spawns a teammate:

```bash
env CLAUDECODE=1 \
    CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1 \
  claude \
    --agent-id "wave-1@merge-hardening-tests" \
    --agent-name "wave-1" \
    --team-name "merge-hardening-tests" \
    --agent-color blue \
    --parent-session-id "a1b2c3d4-e5f6-7890-abcd-ef1234567890" \
    --agent-type general-purpose \
    --dangerously-skip-permissions \
    --model sonnet
```

### 15.5 RalphX Combined Spawn Pattern

For RalphX, the full teammate spawn would combine team flags with existing agent flags:

```bash
env CLAUDECODE=1 \
    CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1 \
  claude \
    --plugin-dir ./ralphx-plugin \
    --agent ralphx-coder \
    --agent-id "coder-1@task-abc123" \
    --agent-name "coder-1" \
    --team-name "task-abc123" \
    --agent-color green \
    --parent-session-id "$LEAD_SESSION_ID" \
    --agent-type general-purpose \
    --model sonnet \
    --tools Read,Write,Edit,Bash,Task,Glob,Grep \
    --allowedTools "mcp__ralphx__start_step,mcp__ralphx__complete_step,mcp__ralphx__get_task_context" \
    --mcp-config ./ralphx-mcp-config.json \
    --permission-mode acceptEdits \
    -p "Execute sub-scope: implement user API handlers"
```

**Note:** In practice, the Task tool handles most of this internally when `team_name` is provided. The lead does not manually construct these commands — the team framework builds them from the `Task` tool parameters + `ralphx.yaml` agent config.

---

## Appendix A: Glossary

| Term | Definition |
|------|-----------|
| **Team Lead** | The Claude Code session that creates and manages the team |
| **Teammate** | A separate Claude Code instance spawned as part of a team |
| **Task List** | Shared list of work items at `~/.claude/tasks/{team}/` |
| **Mailbox** | Message delivery system between team members |
| **Idle** | Teammate waiting for input (normal state between turns) |
| **Delegate Mode** | Lead restricted to coordination-only tools |
| **Plan Approval** | Teammate works read-only until lead approves their plan |
| **In-Process** | All teammates run inside lead's terminal |
| **Quality Gate** | Hook that validates work before allowing state transition |

## Appendix B: Tool Quick Reference

| Tool | Purpose | Key Params |
|------|---------|-----------|
| `TeamCreate` | Start new team | `team_name`, `description` |
| `TeamDelete` | Remove team | (none — uses current team) |
| `SendMessage` | Inter-agent communication | `type`, `recipient`, `content`, `summary` |
| `TaskCreate` | Add work item | `subject`, `description`, `activeForm` |
| `TaskUpdate` | Modify task | `taskId`, `status`, `owner`, `addBlockedBy` |
| `TaskGet` | Read task details | `taskId` |
| `TaskList` | List all tasks | (none) |
| `Task` | Spawn teammate | `prompt`, `subagent_type`, `team_name`, `name`, `model` |

## Appendix C: Related Documentation

| Document | Location | Content |
|----------|----------|---------|
| Claude Code Agent Teams | `ai-docs/claude-code/agent-teams.md` | Official-doc stub for agent teams behavior |
| Claude Code Subagents | `ai-docs/claude-code/sub-agents.md` | Official-doc stub for subagent configuration |
| Claude Code Hooks | `ai-docs/claude-code/hooks.md` | Official-doc stub for hook reference |
| Claude Code CLI | `ai-docs/claude-code/cli-reference.md` | Official-doc stub for CLI flags and commands |
| RalphX Spawning System | `docs/architecture/claude-spawning-system.md` | Full CLI spawning pipeline |
| Claude Code Settings | `ai-docs/claude-code/settings.md` | Official-doc stub for configuration scopes |
| Claude Code Plugins | `ai-docs/claude-code/plugins.md` | Official-doc stub for plugin system |
| RalphX Agent Catalog | `docs/architecture/agent-catalog.md` | All 20 RalphX agents |
| RalphX Task State Machine | `.claude/rules/task-state-machine.md` | 24-state lifecycle |
| RalphX Task Execution Agents | `.claude/rules/task-execution-agents.md` | Agent trigger/flow rules |
| RalphX Agent MCP Tools | `.claude/rules/agent-mcp-tools.md` | Three-layer allowlist |
| RalphX Session Recovery | `docs/features/session-recovery.md` | Auto-recovery system |
