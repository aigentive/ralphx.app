# Worker Team Execution — System Card

> Reference for worker-team leads coordinating coder teammates for wave-based task execution.
> Read this file at session start (Phase 0) before any team operations.

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

---

## Task Execution Lifecycle

```
Phase 0: RECOVER → read this card, check team state
Phase 1: ANALYZE → get_task_context, get_artifact, get_project_analysis
Phase 2: DECOMPOSE → sub-scopes + file ownership + dependency graph → waves
Phase 3: APPROVE → request_team_plan(process="worker-execution", teammates=[...])
Phase 4: EXECUTE → wave-by-wave: TeamCreate → spawn coders → validate gate → next wave
Phase 5: VALIDATE → final full validation (typecheck + lint + tests)
Phase 6: COMPLETE → mark task done → shutdown coders → TeamDelete → summary
```

---

## Tool Reference

### TeamCreate — Create execution team

```json
{ "team_name": "task-<task_id>", "description": "Execution team for <task title>" }
```

Creates `~/.claude/teams/task-<task_id>/config.json` (membership) and `~/.claude/tasks/task-<task_id>/` (shared task list).

### TaskCreate — Add coder work items

```json
{
  "subject": "Implement auth API types",
  "description": "FILE OWNERSHIP: src/types/auth.ts\nSCOPE: Create TypeScript types...",
  "activeForm": "Implementing auth API types"
}
```

- `subject`: imperative, concise | `description`: include FILE OWNERSHIP + SCOPE | `activeForm`: present continuous for UI spinner

### Task — Spawn a coder (FOREGROUND ONLY)

```json
{
  "subagent_type": "general-purpose",
  "name": "coder-1",
  "team_name": "task-<task_id>",
  "description": "Implement auth types",
  "prompt": "<full self-contained instructions>",
  "model": "sonnet",
  "mode": "bypassPermissions"
}
```

| Parameter | Value | Notes |
|-----------|-------|-------|
| `subagent_type` | `"general-purpose"` | Always — coders need Read/Write/Edit/Bash/Glob/Grep + MCP |
| `name` | `"coder-1"`, `"coder-2"` | Unique per team. Used for SendMessage + task ownership |
| `team_name` | Must match TeamCreate | Joins shared task list |
| `prompt` | Full instructions | Coder has NO access to your conversation — everything goes here |
| `model` | `"haiku"` \| `"sonnet"` \| `"opus"` | Default `sonnet`. `haiku` = simple edits, `opus` = architecture |
| `mode` | `"bypassPermissions"` | Coders implement without permission prompts |

**CRITICAL: Do NOT set `run_in_background: true`** — coders need MCP tool access which requires foreground execution.

### SendMessage — Communicate with coders

| Type | When | Required Fields |
|------|------|-----------------|
| `"message"` | Direct to one coder (default) | `recipient`, `content`, `summary` |
| `"broadcast"` | Critical team-wide issue only | `content`, `summary` |
| `"shutdown_request"` | After COMPLETE | `recipient`, `content` |

### TaskUpdate — Manage task assignments

| Action | Call |
|--------|------|
| Assign to coder | `{ "taskId": "1", "owner": "coder-1", "status": "in_progress" }` |
| Mark complete | `{ "taskId": "1", "status": "completed" }` |
| Set dependencies | `{ "taskId": "3", "addBlockedBy": ["1", "2"] }` |

### TaskList — Check wave progress

`{}` → returns all tasks with id, subject, status, owner, blockedBy.

### TeamDelete — Cleanup after all coders shut down

`{}` — only call after all coders confirm shutdown via `shutdown_response(approve)`.

---

## Wave-Based Execution

### Wave Structure

```
Wave 1: Independent scopes (no cross-dependencies)
    → Spawn coders in parallel (each foreground)
    → All complete → Validation Gate
    → Pass → Wave 2

Wave 2: Dependent scopes (build on Wave 1 outputs)
    → Spawn coders with Wave 1 context
    → Validation Gate
    → Pass → Wave 3 (or COMPLETE)
```

**Wave size:** 1–3 coders max per wave.

### Validation Gate (after every wave)

1. `get_project_analysis(project_id, task_id)` → validation commands
2. Run for modified paths:
   - `src/` modified → `npm run typecheck`, `npm run lint`
   - `src-tauri/` modified → `timeout 10m cargo test --lib --manifest-path src-tauri/Cargo.toml 2>&1 | tail -40`
3. All pass → next wave | Any fail → fix loop

### Fix Loop (max 3 attempts per wave)

```
Validation fails → parse errors → identify failing files
    → TaskCreate (fix task with error context + file ownership)
    → Spawn fix coder with error details
    → Re-validate
    → Pass → continue | Fail → retry (max 3x, then escalate)
```

---

## File Ownership Protocol

| Rule | Detail |
|------|--------|
| Exclusive writes | Each coder owns specific files — no overlap within a wave |
| Read-only shared | Coders can read shared types but NOT modify them |
| New files | Created files belong to the creating coder's scope |
| Cross-wave handoff | Wave N+1 coders inherit ownership of files from Wave N |

**Example:**
```
Coder 1 owns: src/api/users.ts, src/api/users.test.ts
Coder 2 owns: src-tauri/src/http_server/handlers/users.rs
Both read:    src/types/user.ts (neither can modify)
```

---

## Step Tools for Progress Tracking

Coders use MCP step tools to report progress visible in the RalphX UI:

| Tool | When | Effect |
|------|------|--------|
| `start_step(task_id, step_name)` | Beginning implementation of a scope | Shows "in progress" in task detail |
| `complete_step(task_id, step_name)` | Scope implementation done | Shows "complete" in task detail |

Coders call these in sequence: `start_step` → implement → `complete_step` for each logical unit.

---

## MCP Tools for Coders

### Allowed (include in coder prompts)

| Tool | Purpose |
|------|---------|
| `get_task_context(task_id)` | Full task details + plan artifact + context hints |
| `get_artifact(artifact_id)` | Read plan artifacts for implementation details |
| `get_project_analysis(project_id, task_id)` | Environment info + validation commands |
| `start_step(task_id, step_name)` | Mark step in progress |
| `complete_step(task_id, step_name)` | Mark step done |

### Forbidden (ideation-only — never include in coder prompts)

| Tool | Why Forbidden |
|------|---------------|
| `get_session_plan` | Ideation sessions only |
| `list_session_proposals` | Ideation sessions only |
| `create_team_artifact` | Research output, not execution |
| `create_task_proposal` | Proposals are ideation concern |

---

## Team Lifecycle

```
TeamCreate("task-<id>")
    ↓
TaskCreate (per coder in wave)
    ↓
Task (spawn coders — FOREGROUND, no run_in_background)
    ↓
Monitor: read auto-delivered messages, relay discoveries
    ↓
Coders complete → Validation Gate
    ↓
[Next wave or COMPLETE]
    ↓
SendMessage(shutdown_request) to each coder
    ↓
Wait for shutdown_response(approve) from each
    ↓
TeamDelete
```

**Key behaviors:**
- Coders go idle after every turn — normal, not an error
- Idle coders can receive messages — sending wakes them up
- Messages from coders are automatically delivered (no polling)
- Each coder has its own independent context window
- Coders cannot see your conversation or other coders' conversations

---

## Resume / Recovery

### State Persistence

Call `save_team_session_state(...)` after each wave with: current phase, team composition, wave progress, completed waves.

### Recovery Routes

| Interrupted Phase | Resume Strategy |
|-------------------|-----------------|
| ANALYZE / DECOMPOSE | Restart from that phase with cached context |
| APPROVE | Re-submit plan for approval |
| EXECUTE | Check TaskList for completed waves → skip them → re-spawn for incomplete waves |
| VALIDATE | Re-run validation → proceed or create fix tasks |
| COMPLETE | Re-send shutdown requests, TeamDelete |

### Resume Coder Prompt Addition

When resuming, inject into coder prompt:
```
RESUME CONTEXT: Resuming wave N. Prior waves completed successfully.
Wave N-1 outputs: [list files created/modified by previous waves]
```

---

## Coder Prompt Template

```
You are {coder-name} on team task-{task_id}.

## Your Mission
{Specific scope and boundaries}

## Exclusive File Ownership (you can write to these files)
- {file1}
- {file2}

## Read-Only Dependencies (DO NOT modify)
- {shared type files}

## Codebase Context
- Project: RalphX — Native Mac GUI for autonomous AI dev
- Frontend: React/TS in src/ (Zustand, TanStack Query, Tailwind)
- Backend: Rust/Tauri in src-tauri/ (Clean architecture, SQLite)
{Domain-specific context}

## Implementation Instructions
{Extracted from plan artifact}

## MCP Tools Available
- get_task_context({task_id}) — full task context
- get_artifact({artifact_id}) — read plan artifacts
- get_project_analysis({project_id}, {task_id}) — validation commands
- start_step({task_id}, "{step_name}") — mark step in progress
- complete_step({task_id}, "{step_name}") — mark step done

## Tools NOT Available (do NOT use)
- get_session_plan, list_session_proposals, create_team_artifact, create_task_proposal

## Constraints
- Do NOT modify files outside your ownership list
- Run validation commands before completing
- Report progress via start_step / complete_step

## When Done
1. complete_step({task_id}, "{step_name}") for each step
2. SendMessage(type="message", recipient="{lead-name}", summary="Scope complete", content="<changes + cross-scope issues>")
3. TaskUpdate(taskId="{task_id}", status="completed")
```

---

## Communication Patterns

| Pattern | When | Action |
|---------|------|--------|
| Relay discovery | Coder finds cross-scope issue | SendMessage → affected coder with context |
| Nudge idle | Coder idle, no progress | SendMessage → "Status check — any blockers?" |
| Broadcast critical | Blocking issue, all coders affected | SendMessage(broadcast) → "STOP: [issue]" |
| Dynamic re-scope | Coder finishes early | Assign remaining work from another coder's scope |
| Shutdown | After COMPLETE | SendMessage(shutdown_request) → each coder |
