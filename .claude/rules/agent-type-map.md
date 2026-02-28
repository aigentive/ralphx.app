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
| `orchestrator-ideation` | ideation | opus | Lead for ideation sessions (proposals, plans) |
| `orchestrator-ideation-readonly` | ideation | sonnet | Read-only ideation fallback |
| `ideation-team-lead` | ideation | opus | Team mode lead for ideation |
| `session-namer` | ideation | haiku | Names ideation sessions |
| `dependency-suggester` | ideation | haiku | Suggests task dependencies |
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
| `ralphx-supervisor` | — | haiku | Supervision tasks |
| `ralphx-qa-prep` | — | sonnet | QA preparation |
| `ralphx-qa-executor` | — | sonnet | QA execution |
| `project-analyzer` | — | sonnet | Project analysis |
| `memory-maintainer` | — | sonnet | Memory management |
| `memory-capture` | — | sonnet | Memory capture |

## Agent Lifecycle Events

All handled by `useAgentEvents` hook (`src/hooks/useAgentEvents.ts`).

| Event | Source | State Transition | Notes |
|---|---|---|---|
| `agent:run_started` | Agent process spawned | `idle` → `generating` | Sets active conversation |
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
