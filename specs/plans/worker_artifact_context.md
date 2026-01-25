# Worker Artifact Context Implementation Plan

## Overview

This plan extends worker execution to dynamically fetch and use artifacts linked to the task being executed. Workers gain context from implementation plans, research documents, and other artifacts before beginning work.

**Problem Statement:**
Currently, workers execute tasks with only the task's title, description, and acceptance criteria. For complex tasks derived from ideation sessions, there may be rich context in:
- Implementation plans (`Specification` artifacts)
- Research documents
- Design documents
- Related artifacts

Without access to this context, workers may miss architectural decisions, coding patterns, or constraints documented in the planning phase.

**Solution:**
Expose MCP tools that allow workers to fetch artifact context dynamically. Update the worker agent prompt to encourage fetching context before implementation.

---

## Dependencies

### Phase 15A (Context-Aware Chat) - Required

| Dependency | Why Needed |
|------------|------------|
| MCP Server with tool scoping | Artifact tools scoped to `worker` via `RALPHX_AGENT_TYPE` |
| `--resume` pattern | Context fetched across multiple conversation turns |
| Tool call visibility in UI | User sees when worker fetches artifacts |

### Phase 15B (Task Execution Chat) - Required

| Dependency | Why Needed |
|------------|------------|
| Worker output persistence | Artifact fetches logged to execution chat history |
| Execution chat in ChatPanel | User can see full context fetch workflow |

### Phase 16 (Ideation Plan Artifacts) - Required

| Dependency | Why Needed |
|------------|------------|
| `Task.source_proposal_id` | Link task to its source proposal |
| `Task.plan_artifact_id` | Direct link to implementation plan |
| `TaskProposal.plan_artifact_id` | Plan reference on proposals |
| `TaskProposal.plan_version_at_creation` | Historical context |

### Artifacts System - Already Complete

The artifacts system provides:
- `ArtifactService` for retrieval
- `ArtifactRelation` for linked artifacts
- Versioning for historical context

---

## Architecture

### Data Flow

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Task Execution Flow                              │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   1. Task Selected for Execution                                       │
│          │                                                             │
│          ▼                                                             │
│   2. Worker Spawned with Task Context                                  │
│      - task.id, title, description, acceptance_criteria                │
│      - task.source_proposal_id (NEW)                                   │
│      - task.plan_artifact_id (NEW)                                     │
│          │                                                             │
│          ▼                                                             │
│   3. Worker Prompt: "Before implementing, fetch relevant context"      │
│          │                                                             │
│          ▼                                                             │
│   4. Worker calls MCP tools:                                           │
│      - get_task_context(task_id) → returns plan + proposal details     │
│      - get_artifact(artifact_id) → returns full artifact content       │
│      - get_related_artifacts(artifact_id) → returns linked artifacts   │
│          │                                                             │
│          ▼                                                             │
│   5. Worker has full context, begins implementation                    │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### MCP Tools for Workers

Add to `TOOL_ALLOWLIST` for `worker`:

| Tool | Parameters | Returns | Description |
|------|------------|---------|-------------|
| `get_task_context` | `task_id` | `TaskContext` | Get task with linked proposal and plan summary |
| `get_artifact` | `artifact_id` | `Artifact` | Fetch full artifact content |
| `get_artifact_version` | `artifact_id`, `version` | `Artifact` | Fetch specific version |
| `get_related_artifacts` | `artifact_id` | `Vec<ArtifactRelation>` | Get linked artifacts |
| `search_project_artifacts` | `project_id`, `query`, `types?` | `Vec<Artifact>` | Search relevant artifacts |

### TaskContext Response Structure

```rust
/// Rich context returned by get_task_context MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// The task being executed
    pub task: Task,

    /// Source proposal if task was created from ideation
    pub source_proposal: Option<TaskProposalSummary>,

    /// Implementation plan artifact (summary, not full content)
    pub plan_artifact: Option<ArtifactSummary>,

    /// Other artifacts related to the plan
    pub related_artifacts: Vec<ArtifactSummary>,

    /// Hints for worker about what context might be useful
    pub context_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProposalSummary {
    pub id: TaskProposalId,
    pub title: String,
    pub description: String,
    pub acceptance_criteria: Vec<String>,
    pub implementation_notes: Option<String>,
    /// Version of plan when proposal was created
    pub plan_version_at_creation: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSummary {
    pub id: ArtifactId,
    pub title: String,
    pub artifact_type: ArtifactType,
    pub current_version: u32,
    /// First ~500 chars of content as preview
    pub content_preview: String,
}
```

### Worker Agent Updates

Update `ralphx-plugin/agents/worker.md`:

```markdown
## Context Fetching (Before Implementation)

Before writing any code, you SHOULD fetch relevant context:

1. **Always call `get_task_context` first** - This returns:
   - Full task details
   - Source proposal with acceptance criteria
   - Implementation plan summary (if exists)
   - Related artifacts

2. **If plan_artifact exists**, call `get_artifact` to read the full plan:
   - Understand architectural decisions
   - Follow coding patterns specified
   - Respect constraints and requirements

3. **For complex tasks**, consider searching for related artifacts:
   - Research documents with background information
   - Design documents with UI/UX decisions
   - Related task implementations for consistency

## Example Workflow

---
1. Receive task assignment
2. Call get_task_context(task_id)
3. If plan_artifact_id present:
   - Call get_artifact(plan_artifact_id)
   - Read and understand the implementation plan
4. If related_artifacts exist:
   - Fetch any that seem relevant
5. Now begin implementation with full context
---
```

---

## Implementation Phases

### Phase 1: Backend - TaskContext Service

1. Create `TaskContextService` in `src-tauri/src/application/`
2. Implement `get_task_context(task_id)` method:
   - Fetch task
   - If `source_proposal_id` present, fetch proposal
   - If `plan_artifact_id` present, fetch artifact summary
   - Fetch related artifacts via `ArtifactRelation`
   - Generate context hints based on artifact types
3. Add HTTP endpoint for MCP proxy

### Phase 2: MCP Tools Implementation

1. Add worker context tools to MCP server:
   - `get_task_context`
   - `get_artifact`
   - `get_artifact_version`
   - `get_related_artifacts`
   - `search_project_artifacts`
2. Update `TOOL_ALLOWLIST` for `worker` agent
3. Wire tools to HTTP proxy endpoints

### Phase 3: Worker Agent Update

1. Update `ralphx-plugin/agents/worker.md`:
   - Add "Context Fetching" section to prompt
   - Document available MCP tools
   - Add example workflow
2. Update worker spawn to include context tool guidance

### Phase 4: Frontend - Context Visibility

1. Show when worker fetches context in execution chat:
   - Tool call indicator for `get_task_context`
   - Artifact preview when `get_artifact` called
2. Add "View Context" button in task detail panel
3. Show linked artifacts in task view

---

## Files to Create/Modify

### Backend (Rust)

| File | Changes |
|------|---------|
| `src-tauri/src/application/task_context_service.rs` | **New file** - TaskContext aggregation |
| `src-tauri/src/application/mod.rs` | Export TaskContextService |
| `src-tauri/src/commands/task_context_commands.rs` | **New file** - Tauri commands for context |
| `src-tauri/src/lib.rs` | Register new commands |

### MCP Server (TypeScript)

| File | Changes |
|------|---------|
| `ralphx-mcp-server/src/tools/worker-context-tools.ts` | **New file** - Context fetch tools |
| `ralphx-mcp-server/src/tool-allowlist.ts` | Add tools to worker allowlist |
| `ralphx-mcp-server/src/http-proxy.ts` | Add context endpoints |

### Frontend (React/TypeScript)

| File | Changes |
|------|---------|
| `src/api/task-context.ts` | **New file** - Context API calls |
| `src/components/Task/TaskContextPanel.tsx` | **New file** - Context display |
| `src/components/Chat/ChatPanel.tsx` | Show artifact previews in tool calls |

### Plugin (Agent Definition)

| File | Changes |
|------|---------|
| `ralphx-plugin/agents/worker.md` | Add context fetching instructions and tool docs |

---

## Tool Specifications

### get_task_context

```typescript
// MCP Tool Definition
{
  name: "get_task_context",
  description: "Fetch rich context for a task including source proposal, implementation plan, and related artifacts. Call this FIRST before implementing any task.",
  parameters: {
    type: "object",
    properties: {
      task_id: {
        type: "string",
        description: "The ID of the task to get context for"
      }
    },
    required: ["task_id"]
  }
}
```

### get_artifact

```typescript
{
  name: "get_artifact",
  description: "Fetch the full content of an artifact by ID. Use after get_task_context reveals a plan_artifact_id.",
  parameters: {
    type: "object",
    properties: {
      artifact_id: {
        type: "string",
        description: "The artifact ID to fetch"
      }
    },
    required: ["artifact_id"]
  }
}
```

### get_related_artifacts

```typescript
{
  name: "get_related_artifacts",
  description: "Get artifacts related to a specific artifact (e.g., research docs related to a plan).",
  parameters: {
    type: "object",
    properties: {
      artifact_id: {
        type: "string",
        description: "The artifact ID to find relations for"
      },
      relation_types: {
        type: "array",
        items: { type: "string" },
        description: "Filter by relation types: 'derived_from', 'references', 'supersedes'"
      }
    },
    required: ["artifact_id"]
  }
}
```

### search_project_artifacts

```typescript
{
  name: "search_project_artifacts",
  description: "Search for artifacts in the project by query and optional type filter.",
  parameters: {
    type: "object",
    properties: {
      project_id: {
        type: "string",
        description: "The project ID to search within"
      },
      query: {
        type: "string",
        description: "Search query (matches title, content)"
      },
      artifact_types: {
        type: "array",
        items: { type: "string" },
        description: "Filter by artifact types: 'specification', 'research', 'design_doc', etc."
      }
    },
    required: ["project_id", "query"]
  }
}
```

---

## Worker Prompt Update

Full addition to `ralphx-plugin/agents/worker.md`:

```markdown
---
name: worker
description: Executes implementation tasks with full artifact context access
tools:
  - Read
  - Write
  - Edit
  - Glob
  - Grep
  - Bash
disallowed_tools: []
---

# Worker Agent

You are a skilled software developer executing implementation tasks.

## Context Fetching (IMPORTANT - Do This First)

Before writing any code, you MUST fetch relevant context to understand the full picture:

### Step 1: Get Task Context
Always start by calling `get_task_context` with the task ID:

get_task_context(task_id: "...")

This returns:
- **task**: Full task details (title, description, acceptance criteria)
- **source_proposal**: The original proposal with implementation notes
- **plan_artifact**: Summary of the implementation plan (if exists)
- **related_artifacts**: Other relevant documents
- **context_hints**: Suggestions for what else to fetch

### Step 2: Read Implementation Plan
If `plan_artifact` is present in the response, fetch the full plan:

get_artifact(artifact_id: "<plan_artifact.id>")

Read the plan carefully for:
- Architectural decisions and rationale
- Coding patterns to follow
- Constraints and requirements
- Dependencies on other tasks

### Step 3: Fetch Related Artifacts (Optional)
For complex tasks, related artifacts may provide valuable context:
- Research documents with background information
- Design documents with UI/UX decisions
- Previously completed related tasks

get_related_artifacts(artifact_id: "<plan_artifact.id>")

### Step 4: Begin Implementation
Now that you have full context, proceed with implementation following:
1. The acceptance criteria from the task/proposal
2. The architectural decisions from the plan
3. Any patterns or constraints documented

## Available MCP Tools

| Tool | When to Use |
|------|------------|
| `get_task_context` | ALWAYS first - get task + linked artifacts |
| `get_artifact` | Read full artifact content |
| `get_artifact_version` | Read specific historical version |
| `get_related_artifacts` | Find linked documents |
| `search_project_artifacts` | Search for relevant context |

## Example Workflow

---
User assigns task: "Implement WebSocket server"

1. get_task_context("task-123")
   → Returns task, proposal, plan_artifact_id: "artifact-456"

2. get_artifact("artifact-456")
   → Returns implementation plan:
     "Use tokio-tungstenite, implement reconnection logic,
      follow existing event patterns in src/events/"

3. Now implement following the plan's guidance
---

---

[Rest of existing worker.md content]
```

---

## Verification Checklist

### Backend
- [ ] `TaskContextService` created and tested
- [ ] `get_task_context` returns complete context
- [ ] Artifact summaries include content preview
- [ ] Related artifacts fetched via `ArtifactRelation`
- [ ] HTTP endpoints work for MCP proxy

### MCP Tools
- [ ] `get_task_context` tool registered for worker
- [ ] `get_artifact` tool registered for worker
- [ ] `get_related_artifacts` tool registered for worker
- [ ] `search_project_artifacts` tool registered for worker
- [ ] All tools in TOOL_ALLOWLIST for `worker`

### Worker Agent
- [ ] Context fetching instructions in prompt
- [ ] Tool documentation added
- [ ] Example workflow included
- [ ] Worker actually calls `get_task_context` first in practice

### Frontend
- [ ] Context fetch tool calls visible in execution chat
- [ ] Artifact previews shown when artifacts fetched
- [ ] "View Context" option in task detail panel
- [ ] Linked artifacts visible in task view

### Integration
- [ ] End-to-end: Task from ideation → worker fetches plan → implements correctly
- [ ] Historical version access works (plan_version_at_creation)
- [ ] Search finds relevant artifacts

---

## Decisions

### 1. Automatic vs Manual Context Fetch

**Decision:** Worker always calls tools manually (Option A)

**Rationale:**
- Workers have agency to decide what context is relevant
- Keep initial prompt lean
- Worker fetches what it needs based on task complexity
- Avoids bloating context with potentially irrelevant artifacts

### 2. Context Size Limits

**Decision:** Summary with 500-char preview; full content requires separate call

**Implementation:**
- `TaskContext.plan_artifact` returns `ArtifactSummary` with 500-char `content_preview`
- Full artifact content requires explicit `get_artifact(artifact_id)` call
- Prevents context bloat for tasks with large implementation plans
- Worker decides if full plan content is needed

### 3. Caching Strategy

**Decision:** No caching for now

**Rationale:**
- Keep implementation simple for MVP
- Artifact fetches are infrequent (typically once at start of execution)
- Can add caching later if performance becomes an issue
- Avoids cache invalidation complexity

---

## Related Documents

- `specs/plans/ideation_plan_artifacts.md` - Phase 16, creates the artifact links
- `specs/plans/task_execution_chat.md` - Phase 15B, worker persistence
- `specs/plans/context_aware_chat_implementation.md` - Phase 15A infrastructure
- `src-tauri/src/domain/entities/artifact.rs` - Artifact types and relations
