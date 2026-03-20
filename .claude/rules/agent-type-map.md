# Agent Type Map

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## Agent Context Types

Source of truth: `ChatContextType` (Rust: `domain/entities/chat_conversation.rs`) | `ContextType` (TS: `types/chat-conversation.ts`)

| Context Type | Store Key Prefix | Execution Slot | Streaming | Subagents | Diff Views | Queue | Team Mode | Pipeline Stage |
|---|---|---|---|---|---|---|---|---|
| `ideation` | `session:` | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | — |
| `task` | `task:` | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | — |
| `project` | `project:` | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | — |
| `task_execution` | `task_execution:` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Executing → ReExecuting |
| `review` | `review:` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | PendingReview → Reviewing |
| `merge` | `merge:` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | PendingMerge → Merging |

**Execution slot** = counted against `max_concurrent` in `uses_execution_slot()` (`chat_service/mod.rs`).

## Named Agents (ralphx.yaml)

| Agent Name | Context | Model | Role |
|---|---|---|---|
| `orchestrator-ideation` | ideation | opus | Lead for ideation sessions (proposals, plans). MCP: `update_plan_verification`, `get_plan_verification` |
| `orchestrator-ideation-readonly` | ideation | opus | Read-only ideation fallback |
| `ideation-team-lead` | ideation | opus | Team mode lead for ideation. MCP: `update_plan_verification`, `get_plan_verification` |
| `session-namer` | ideation | sonnet | Names ideation sessions |
| `dependency-suggester` | ideation | sonnet | Suggests task dependencies |
| `chat-task` | task | sonnet | Task-level Q&A |
| `chat-project` | project | sonnet | Project-level Q&A |
| `ralphx-worker` | task_execution | sonnet | Code execution, step management |
| `ralphx-coder` | task_execution | sonnet | Alternative worker variant |
| `ralphx-worker-team` | task_execution | sonnet | Team mode worker |
| `ralphx-reviewer` | review | sonnet | Automated code reviewer |
| `ralphx-review-chat` | review | sonnet | Interactive review assistant |
| `ralphx-review-history` | review | sonnet | Historical review context |
| `ralphx-merger` | merge | sonnet | Merge conflict resolution |
| `ralphx-deep-researcher` | — | opus | Deep research for ideation |
| `ralphx-orchestrator` | — | opus | Internal orchestration |
| `ralphx-supervisor` | — | sonnet | Supervision tasks |
| `ralphx-qa-prep` | — | sonnet | QA preparation |
| `ralphx-qa-executor` | — | sonnet | QA execution |
| `project-analyzer` | — | sonnet | Project analysis |
| `memory-maintainer` | — | sonnet | Memory management |
| `memory-capture` | — | sonnet | Memory capture |
| `plan-critic-layer1` | — | opus | Layer 1 completeness critic for plan verification. Returns structured JSON gap analysis only. |
| `plan-critic-layer2` | — | opus | Dual-lens implementation critic (minimal/surgical + defense-in-depth). Read-only. No Write/Edit/Bash. |
| `plan-verifier` | — | opus | Owns adversarial round loop — spawns critics, merges gaps, revises plan. |
| `ideation-specialist-backend` | — | opus | Research Rust/Tauri/SQLite patterns for ideation teams |
| `ideation-specialist-frontend` | — | opus | Research React/TypeScript/Tailwind patterns for ideation teams |
| `ideation-specialist-infra` | — | opus | Research database schema, MCP, config, and git patterns for ideation teams |
| `ideation-specialist-ux` | — | opus | UX/flow verification specialist — ASCII wireframes, user flow diagrams, screen inventory, UX gap analysis. Tools: Read, Grep, Glob, WebFetch, WebSearch. MCP: `get_session_plan`, `get_artifact`, `create_team_artifact`, `get_team_artifacts`, `list_session_proposals`, `get_proposal`, `get_parent_session_context`, `search_memories`, `get_memory`, `get_memories_for_paths`. DisallowedTools: Write, Edit, NotebookEdit, Bash. Spawned by plan-verifier during VERIFY and by ideation-team-lead / orchestrator-ideation during EXPLORE when UI/UX signals detected. |
| `ideation-specialist-code-quality` | — | opus | Pre-round enrichment specialist — reads actual code paths referenced in the plan, identifies targeted quality improvements (complexity reduction, DRY violations, extract opportunities, naming). Runs ONCE before the adversarial loop begins (Step 0.5). Findings injected into plan context so critics see them in every round. Spawned by plan-verifier unconditionally when plan references existing code files. |
| `ideation-specialist-prompt-quality` | — | opus | Per-round prompt quality specialist — token efficiency, information scoping, anti-bloat, tool-prompt alignment. Tools: Read, Grep, Glob, WebFetch, WebSearch. MCP: `get_session_plan`, `get_artifact`, `create_team_artifact`, `get_team_artifacts`, `list_session_proposals`, `get_proposal`, `get_parent_session_context`, `search_memories`, `get_memory`, `get_memories_for_paths`. DisallowedTools: Write, Edit, NotebookEdit, Bash. Spawned by plan-verifier when Affected Files contains `.md` files in `agents/` or `prompts/` directories. |
| `ideation-advocate` | — | opus | Advocate for a specific approach in architectural debates |
| `ideation-critic` | — | opus | Stress-test all approaches with adversarial analysis |

**Memory tools:** Most agents also have memory read tools (`search_memories`, `get_memory`, `get_memories_for_paths`) — see `ralphx.yaml` for the authoritative `mcp_tools` list per agent.

## Verification Specialist Extensibility Pattern

Adding a new specialist to the plan verification pipeline requires these 7 steps:

| Step | File | Change |
|------|------|--------|
| 1 | `ralphx-plugin/agents/<name>.md` | Create agent prompt with role/scope/refuse boundaries and output format |
| 2 | `ralphx-plugin/ralphx.yaml` | Register agent: model, tools, mcp_tools, disallowedTools |
| 3 | `ralphx-plugin/ralphx-mcp-server/src/agentNames.ts` | Add `export const IDEATION_SPECIALIST_<NAME> = "<name>"` constant |
| 4 | `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | Import constant; add `[IDEATION_SPECIALIST_<NAME>]: [...]` to TOOL_ALLOWLIST |
| 5 | `ralphx-plugin/agents/plan-verifier.md` frontmatter | Add `Task(ralphx:<name>)` to `tools` list |
| 6 | `ralphx-plugin/ralphx.yaml` plan-verifier entry | Add `Task(ralphx:<name>)` to `preapproved_cli_tools` array |
| 7 | `ralphx-plugin/agents/plan-verifier.md` prompt | Add signal → specialist mapping in dynamic role selection section |

**Two specialist dispatch modes:**
| Mode | When | Example |
|------|------|---------|
| **Pre-round enrichment** (Step 0.5) | Runs ONCE before adversarial loop; results injected into plan context | `ideation-specialist-code-quality` — unconditionally when plan references existing code files |
| **Per-round parallel** | Runs alongside critics in each round; selected by signal | `ideation-specialist-ux` — when Affected Files contains `.tsx`/`.ts` in `src/`; `ideation-specialist-prompt-quality` — when Affected Files contains `.md` in `agents/` or `prompts/` |

**Signal mapping rules (per-round specialists):** Scan `## Affected Files` and `## Architecture` sections only (not full plan text). Return: specialist name, trigger signal, signal source. Per-round specialists run in parallel with critics — failure is non-blocking. Specialists create artifacts on the **parent ideation session_id** (not the verification child session_id) so they appear in the Team Artifacts tab.

## Agent Lifecycle Events

All handled by `useAgentEvents` hook (`src/hooks/useAgentEvents.ts`).

| Event | Source | State Transition | Notes |
|---|---|---|---|
| `agent:run_started` | Agent process spawned | `idle` → `generating` | Sets active conversation |
| `agent:conversation_created` | Backend conversation creation | — (query invalidation) | Fires when new ChatConversation row is created. Frontend invalidates conversation list. |
| `agent:message_created` | Agent produces output | — (query invalidation) | Appends to message list |
| `agent:turn_completed` | Agent finishes one turn | `generating` → `waiting_for_input` | Agent alive, awaiting user input |
| `agent:run_completed` | Agent process exits | `*` → `idle` | Flushes queued messages |
| `agent:queue_sent` | Queued message delivered | — (removes from queue) | Matches by content hash |
| `agent:stopped` | User stops agent | `*` → `idle` | User-initiated stop |
| `agent:error` | Agent crash/error | `*` → `idle` | Toast for execution/review/merge |
| `agent:session_recovered` | Session recovery | — (info toast) | Notification only |

## Frontend State: AgentStatus (Tri-State)

Defined in `stores/chatStore.ts`. Keyed by store key (e.g., `"task_execution:task-123"`).

```
AgentStatus = "idle" | "generating" | "waiting_for_input"
```

| Status | Meaning | UI | Queue Behavior |
|---|---|---|---|
| `idle` | No agent process | Hidden / activity icon | Direct send |
| `generating` | Agent producing output | Spinner + "Agent responding..." | Queue message |
| `waiting_for_input` | Agent alive, between turns | Pause icon + "Awaiting input" | Direct send |

**Selectors:**
- `selectAgentStatus(key)` → `AgentStatus` (tri-state)
- `selectIsAgentRunning(key)` → `boolean` (true for both `generating` and `waiting_for_input` — backward compat)

## Store Key Construction

`buildStoreKey(contextType, contextId)` from `lib/chat-context-registry.ts`:

| Context Type | Key Format | Example |
|---|---|---|
| `ideation` | `session:{sessionId}` | `session:abc-123` |
| `task` | `task:{taskId}` | `task:task-456` |
| `project` | `project:{projectId}` | `project:proj-789` |
| `task_execution` | `task_execution:{taskId}` | `task_execution:task-456` |
| `review` | `review:{taskId}` | `review:task-456` |
| `merge` | `merge:{taskId}` | `merge:task-456` |
