---
paths:
  - "agents/orchestrator-ideation/**"
  - "agents/orchestrator-ideation-readonly/**"
  - "agents/ideation-team-lead/**"
  - "src-tauri/src/infrastructure/agents/**"
  - "src-tauri/src/application/chat_service/**"
  - "plugins/app/ralphx-mcp-server/src/tools.ts"
---

# Orchestrator Ideation Workflows

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

**Required Context:** task-execution-agents.md | agent-mcp-tools.md | agent-authoring.md

---

## Overview

The `orchestrator-ideation` agent (active + readonly variant) manages ideation sessions: capturing user requirements, exploring solutions, and generating task proposals + plans. Sessions can be **active** (mutable) or **accepted** (finalized), and accepted sessions spawn **child sessions** for follow-up work.

---

## Phase 0: RECOVER (Always Runs First)

**Entry:** Unconditional on every conversation start (before processing user message)

**Calls:**
1. `get_session_plan(session_id)` → Load existing plan
2. `list_session_proposals(session_id)` → Load existing proposals
3. `get_parent_session_context(session_id)` → Detect child session + inherited context

**Route based on state:**
| State | → Phase |
|-------|---------|
| Has plan + proposals | → **FINALIZE** (present work, ask what to adjust) |
| Has plan, no proposals | → **CONFIRM** (present plan, proceed to proposals) |
| Has parent context | → Load context, summarize, then **UNDERSTAND** |
| Empty | → **UNDERSTAND** (fresh start) |

---

## Sessions: Active vs Accepted

| Aspect | Active Session | Accepted Session |
|--------|---|---|
| **State** | Mutable | Finalized (immutable) |
| **User behavior** | "Follow up", "iterate", "update plan" | Read-only reference |
| **Mutations** | Allowed in-session | Delegated to child sessions |
| **Read-only agent** | N/A | `orchestrator-ideation-readonly` views proposal history |

---

## Follow-up Handling (Natural Language → Action)

Recognize user intent from phrase patterns and route appropriately:

| Phrase | Active Action | Accepted Action |
|--------|---|---|
| "follow up", "continue", "build on this" | Resume current phase | → `create_child_session` |
| "spin off", "separate session" | → `create_child_session` | → `create_child_session` |
| "update/modify plan" | `update_plan_artifact` | → `create_child_session` |
| "add more tasks" | Create proposals in-session | → `create_child_session` |
| "status?", "summary" | Summarize + proposals | Summarize (read-only) |

**Rule:** Never mutate accepted sessions directly. Delegate all mutations to child sessions.

---

## Child Sessions: Spawning & Context Inheritance

**Spawn:** Call `create_child_session(parent_session_id, title, description, initial_prompt, inherit_context=true)`
- `initial_prompt` is **required** for auto-spawn — without it, no agent is triggered on the child session

**Behavior:**
- Backend auto-spawns background `orchestrator-ideation` agent on child (triggered by `initial_prompt`)
- Child loads parent plan + proposals as baseline context
- Child processes user's original request (from `initial_prompt`) through workflow phases (Phase 0 → Phase 1 → ...)
- Child can mutate independently without affecting parent

**Mandatory steps in child session:**
1. Summarize inherited context to user
2. Load parent plan as baseline
3. Acknowledge parent proposals if relevant to user's prompt
4. Build on parent findings (don't re-explore)

---

## MCP Tools

### Always Required
- `get_session_plan` — Load plan (Phase 0)
- `list_session_proposals` — Load proposals (Phase 0)
- `get_parent_session_context` — Detect child sessions (Phase 0)
- `create_child_session` — Delegate mutations to child (accepted sessions)
- `update_plan_artifact` — Modify plan in active session

### Conditional
- `add_proposal` — Create proposals (Phase 3+)
- `update_proposal` — Modify proposals (active sessions only)

### Tool Allowlist Requirement
- `ORCHESTRATOR_IDEATION`: all tools above
- `ORCHESTRATOR_IDEATION_READONLY`: `get_session_plan`, `list_session_proposals`, `create_child_session` (added Feb 2026)

See `agent-mcp-tools.md` for full scope matrix.

---

## Key Files

| Component | Path |
|-----------|------|
| Active ideation agent | `agents/orchestrator-ideation/claude/prompt.md` |
| Readonly ideation agent | `agents/orchestrator-ideation-readonly/claude/prompt.md` |
| MCP tool allowlist | `plugins/app/ralphx-mcp-server/src/tools.ts` |
| ChatService context routing | `src-tauri/src/application/chat_service/chat_service_context.rs` |
| Agent resolution table | `task-execution-agents.md:88-95` |
