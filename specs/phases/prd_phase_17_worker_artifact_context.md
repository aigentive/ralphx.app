# RalphX - Phase 17: Worker Artifact Context

## Overview

This phase extends worker execution to dynamically fetch and use artifacts linked to the task being executed. Workers gain context from implementation plans, research documents, and other artifacts before beginning work.

**Reference Plan:**
- `specs/plans/worker_artifact_context.md` - Complete implementation plan with data flow, MCP tools, and worker agent updates

## Goals

1. Create `TaskContextService` for aggregating task context (proposal, plan, related artifacts)
2. Implement MCP tools for workers (`get_task_context`, `get_artifact`, `get_related_artifacts`, `search_project_artifacts`)
3. Add HTTP endpoints for MCP proxy
4. Update worker agent prompt with context fetching instructions
5. Show context fetch operations in execution chat UI
6. Add "View Context" option in task detail panel

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

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/worker_artifact_context.md`
2. Understand the data flow, MCP tools, and TaskContext structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write tests where appropriate
4. Run `npm run lint && npm run typecheck` and `cargo test`
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/worker_artifact_context.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Create TaskContext and summary types",
    "plan_section": "Architecture - TaskContext Response Structure",
    "steps": [
      "Read specs/plans/worker_artifact_context.md section 'TaskContext Response Structure'",
      "Create src-tauri/src/domain/entities/task_context.rs:",
      "  - TaskContext struct (task, source_proposal, plan_artifact, related_artifacts, context_hints)",
      "  - TaskProposalSummary struct (id, title, description, acceptance_criteria, implementation_notes, plan_version_at_creation)",
      "  - ArtifactSummary struct (id, title, artifact_type, current_version, content_preview)",
      "Update mod.rs to export new module",
      "Write unit tests for struct creation",
      "Run cargo test",
      "Commit: feat(entities): add TaskContext and summary types"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create TaskContextService",
    "plan_section": "Implementation Phases - Phase 1: Backend",
    "steps": [
      "Read specs/plans/worker_artifact_context.md section 'Phase 1: Backend - TaskContext Service'",
      "Create src-tauri/src/application/task_context_service.rs:",
      "  - Inject TaskRepository, TaskProposalRepository, ArtifactRepository",
      "  - Implement get_task_context(task_id) method:",
      "    1. Fetch task by ID",
      "    2. If source_proposal_id present, fetch proposal and create TaskProposalSummary",
      "    3. If plan_artifact_id present, fetch artifact and create ArtifactSummary (500-char preview)",
      "    4. Fetch related artifacts via ArtifactRelation",
      "    5. Generate context_hints based on what's available",
      "    6. Return TaskContext",
      "Update application/mod.rs to export",
      "Add to AppState",
      "Write unit tests",
      "Run cargo test",
      "Commit: feat(application): add TaskContextService for aggregating task context"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add HTTP endpoints for worker context tools",
    "plan_section": "Files to Create/Modify - Backend",
    "steps": [
      "Add HTTP endpoints to src-tauri/src/http_server.rs:",
      "  - GET /api/task_context/:task_id -> TaskContext",
      "  - GET /api/artifact/:artifact_id -> Artifact (full content)",
      "  - GET /api/artifact/:artifact_id/version/:version -> Artifact (specific version)",
      "  - GET /api/artifact/:artifact_id/related -> Vec<ArtifactRelation>",
      "  - POST /api/artifacts/search (project_id, query, artifact_types?) -> Vec<ArtifactSummary>",
      "Each endpoint uses TaskContextService or ArtifactService",
      "Test endpoints with curl",
      "Run cargo test",
      "Commit: feat(backend): add HTTP endpoints for worker context tools"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add Tauri commands for task context",
    "plan_section": "Files to Create/Modify - Backend",
    "steps": [
      "Create src-tauri/src/commands/task_context_commands.rs:",
      "  - get_task_context(task_id) -> TaskContext",
      "  - get_artifact_full(artifact_id) -> Artifact",
      "  - get_artifact_version(artifact_id, version) -> Artifact",
      "  - get_related_artifacts(artifact_id) -> Vec<ArtifactRelation>",
      "  - search_artifacts(project_id, query, artifact_types?) -> Vec<ArtifactSummary>",
      "Register commands in lib.rs invoke_handler",
      "Write unit tests",
      "Run cargo test",
      "Commit: feat(commands): add task context commands"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Add worker context tools to MCP server",
    "plan_section": "Tool Specifications",
    "steps": [
      "Reference the mcp-builder skill (/mcp-builder) for MCP tool best practices",
      "Read specs/plans/worker_artifact_context.md section 'Tool Specifications'",
      "Create ralphx-mcp-server/src/tools/worker-context-tools.ts:",
      "  - get_task_context tool (task_id -> TaskContext)",
      "  - get_artifact tool (artifact_id -> Artifact)",
      "  - get_artifact_version tool (artifact_id, version -> Artifact)",
      "  - get_related_artifacts tool (artifact_id, relation_types? -> Vec<ArtifactRelation>)",
      "  - search_project_artifacts tool (project_id, query, artifact_types? -> Vec<ArtifactSummary>)",
      "Update ralphx-mcp-server/src/index.ts to register tools",
      "Update ralphx-mcp-server/src/http-proxy.ts with new endpoints",
      "Build: npm run build",
      "Commit: feat(mcp): add worker context tools"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Update TOOL_ALLOWLIST for worker agent",
    "plan_section": "MCP Tools for Workers",
    "steps": [
      "Reference the mcp-builder skill (/mcp-builder) for MCP configuration best practices if needed",
      "Update ralphx-mcp-server/src/tool-allowlist.ts:",
      "  - Add to worker allowlist: get_task_context, get_artifact, get_artifact_version, get_related_artifacts, search_project_artifacts",
      "  - Worker now has 5 MCP tools (previously 0)",
      "Test manually: set RALPHX_AGENT_TYPE=worker and verify tools returned",
      "Build: npm run build",
      "Commit: feat(mcp): add context tools to worker allowlist"
    ],
    "passes": true
  },
  {
    "category": "plugin",
    "description": "Update worker agent with context fetching instructions",
    "plan_section": "Worker Prompt Update",
    "steps": [
      "Read specs/plans/worker_artifact_context.md section 'Worker Prompt Update'",
      "Update ralphx-plugin/agents/worker.md:",
      "  - Add 'Context Fetching (IMPORTANT - Do This First)' section",
      "  - Document Step 1: Get Task Context (always call get_task_context first)",
      "  - Document Step 2: Read Implementation Plan (if plan_artifact exists)",
      "  - Document Step 3: Fetch Related Artifacts (optional for complex tasks)",
      "  - Document Step 4: Begin Implementation",
      "  - Add 'Available MCP Tools' table",
      "  - Add example workflow",
      "Test agent invocation manually",
      "Commit: feat(plugin): add context fetching instructions to worker agent"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create task context types and API",
    "plan_section": "Files to Create/Modify - Frontend",
    "steps": [
      "Create src/types/task-context.ts:",
      "  - TaskContext interface",
      "  - TaskProposalSummary interface",
      "  - ArtifactSummary interface",
      "  - Zod schemas for validation",
      "Create src/api/task-context.ts:",
      "  - getTaskContext(taskId)",
      "  - getArtifactFull(artifactId)",
      "  - getArtifactVersion(artifactId, version)",
      "  - getRelatedArtifacts(artifactId)",
      "  - searchArtifacts(projectId, query, artifactTypes?)",
      "Run npm run typecheck",
      "Commit: feat(types): add task context types and API"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create TaskContextPanel component",
    "plan_section": "Files to Create/Modify - Frontend",
    "steps": [
      "Create src/components/Task/TaskContextPanel.tsx:",
      "  - Display linked proposal summary (if exists)",
      "  - Display plan artifact preview with 'View Full' button",
      "  - List related artifacts with type icons",
      "  - Show context hints",
      "  - Loading and empty states",
      "Use shadcn Card and Collapsible components",
      "Create TaskContextPanel.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(task): add TaskContextPanel component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add 'View Context' button to TaskDetailPanel",
    "plan_section": "Implementation Phases - Phase 4: Frontend",
    "steps": [
      "Update src/components/Task/TaskDetailPanel.tsx:",
      "  - Add 'View Context' button (visible when task has source_proposal_id or plan_artifact_id)",
      "  - On click: show TaskContextPanel in slide-over or modal",
      "  - Fetch context via getTaskContext API",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task): add View Context button to TaskDetailPanel"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Show artifact previews in execution chat tool calls",
    "plan_section": "Implementation Phases - Phase 4: Frontend",
    "steps": [
      "Update src/components/Chat/ToolCallIndicator.tsx:",
      "  - Detect get_task_context and get_artifact tool calls",
      "  - For get_task_context: show summary of what was returned",
      "  - For get_artifact: show artifact title and content preview",
      "  - Collapsible detail view for full response",
      "Update src/components/Chat/ChatPanel.tsx if needed",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): show artifact previews in tool call indicators"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Show linked artifacts in task view",
    "plan_section": "Verification Checklist - Frontend",
    "steps": [
      "Update task card or detail view:",
      "  - Show small indicator when task has plan_artifact_id",
      "  - Tooltip: 'Has implementation plan'",
      "  - Show indicator when task has source_proposal_id",
      "  - Tooltip: 'Created from proposal'",
      "Use Lucide icons (FileText for plan, Lightbulb for proposal)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task): show linked artifact indicators in task view"
    ],
    "passes": true
  },
  {
    "category": "documentation",
    "description": "Update CLAUDE.md files for Phase 17",
    "steps": [
      "Update src/CLAUDE.md with:",
      "  - TaskContextPanel component",
      "  - Task context types and API",
      "Update src-tauri/CLAUDE.md with:",
      "  - TaskContextService",
      "  - TaskContext entities",
      "  - HTTP endpoints for context",
      "Update logs/activity.md with Phase 17 completion summary",
      "Commit: docs: update documentation for worker artifact context"
    ],
    "passes": true
  }
]
```

---

## Key Architecture Decisions

From the implementation plan:

| Decision | Rationale |
|----------|-----------|
| **Manual context fetch (not auto-inject)** | Workers have agency to decide what context is relevant; keeps initial prompt lean |
| **500-char preview in TaskContext** | Prevents context bloat; full content requires explicit `get_artifact` call |
| **No caching for MVP** | Keep implementation simple; artifact fetches are infrequent; can add later |
| **5 MCP tools for workers** | `get_task_context`, `get_artifact`, `get_artifact_version`, `get_related_artifacts`, `search_project_artifacts` |
| **Worker calls get_task_context first** | Prompt instructs worker to always fetch context before implementing |

---

## Verification Checklist

After completing all tasks:

### Backend
- [ ] `TaskContextService` created and tested
- [ ] `get_task_context` returns complete context (task, proposal, plan, related)
- [ ] `ArtifactSummary` includes 500-char content preview
- [ ] Related artifacts fetched via `ArtifactRelation`
- [ ] HTTP endpoints work for MCP proxy
- [ ] All Tauri commands registered and working

### MCP Tools
- [ ] `get_task_context` tool registered and working
- [ ] `get_artifact` tool registered and working
- [ ] `get_artifact_version` tool registered and working
- [ ] `get_related_artifacts` tool registered and working
- [ ] `search_project_artifacts` tool registered and working
- [ ] All 5 tools in TOOL_ALLOWLIST for `worker`
- [ ] Tools return proper responses via HTTP proxy

### Worker Agent
- [ ] Context fetching instructions added to worker.md
- [ ] All 5 MCP tools documented in worker prompt
- [ ] Example workflow included
- [ ] Worker actually calls `get_task_context` first in practice

### Frontend
- [ ] TaskContextPanel component created
- [ ] "View Context" button visible for tasks with artifacts
- [ ] Context fetch tool calls visible in execution chat
- [ ] Artifact previews shown when `get_artifact` called
- [ ] Linked artifact indicators shown on task cards

### Integration (Manual Verification)
- [ ] Historical version access works (`plan_version_at_creation`)
- [ ] Search finds relevant artifacts by query
