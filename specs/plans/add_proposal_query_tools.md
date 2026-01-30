# Plan: Add Proposal Query Tools to MCP Server

## Summary

Add two MCP tools to enable the orchestrator-ideation agent to query proposals:
- `list_session_proposals` - Lightweight list of proposals in a session
- `get_proposal` - Full details of a single proposal

The backend infrastructure already exists (repository methods `get_by_session` and `get_by_id`). This task is primarily about wiring up the MCP layer.

## Files to Modify

| File | Changes |
|------|---------|
| `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | Add 2 tool definitions + update TOOL_ALLOWLIST |
| `ralphx-plugin/ralphx-mcp-server/src/index.ts` | Add GET dispatch handling for both tools |
| `src-tauri/src/http_server/mod.rs` | Add 2 GET routes |
| `src-tauri/src/http_server/handlers/ideation.rs` | Add 2 handler functions |
| `src-tauri/src/http_server/types.rs` | Add request/response types |

## Implementation Steps

### 1. Add Tool Definitions (`tools.ts`) (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(mcp): add list_session_proposals and get_proposal tool definitions`

Add to `ALL_TOOLS` in the IDEATION TOOLS section (after `update_session_title`):

```typescript
{
  name: "list_session_proposals",
  description:
    "List all task proposals in an ideation session. Returns summary info (id, title, category, priority, dependencies). Use get_proposal for full details including steps and acceptance criteria.",
  inputSchema: {
    type: "object",
    properties: {
      session_id: {
        type: "string",
        description: "The ideation session ID",
      },
    },
    required: ["session_id"],
  },
},
{
  name: "get_proposal",
  description:
    "Get full details of a task proposal including steps and acceptance criteria. Use after list_session_proposals to get complete information for a specific proposal.",
  inputSchema: {
    type: "object",
    properties: {
      proposal_id: {
        type: "string",
        description: "The proposal ID to fetch",
      },
    },
    required: ["proposal_id"],
  },
},
```

Update `TOOL_ALLOWLIST["orchestrator-ideation"]`:
```typescript
"orchestrator-ideation": [
  "create_task_proposal",
  "update_task_proposal",
  "delete_task_proposal",
  "add_proposal_dependency",
  "list_session_proposals",  // NEW
  "get_proposal",            // NEW
  "create_plan_artifact",
  "update_plan_artifact",
  "get_plan_artifact",
  "link_proposals_to_plan",
  "get_session_plan",
],
```

### 2. Add Response Types (`types.rs`) (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(http_server): add proposal query response types`

```rust
/// Lightweight proposal summary for list endpoint
#[derive(Debug, Serialize)]
pub struct ProposalSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub priority: String,
    pub depends_on: Vec<String>,
    pub plan_artifact_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListProposalsResponse {
    pub proposals: Vec<ProposalSummary>,
    pub count: usize,
}

/// Full proposal details for get endpoint
#[derive(Debug, Serialize)]
pub struct ProposalDetailResponse {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub priority: String,
    pub steps: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub depends_on: Vec<String>,
    pub plan_artifact_id: Option<String>,
    pub created_at: String,
}
```

### 3. Add HTTP Handlers (`ideation.rs`)
**Dependencies:** Step 2
**Atomic Commit:** `feat(http_server): add list_session_proposals and get_proposal handlers`

```rust
pub async fn list_session_proposals(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<ListProposalsResponse>, StatusCode> {
    let session_id = IdeationSessionId::from_string(session_id);

    // Get all proposals for session
    let proposals = state.app_state.task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to list proposals: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get all dependencies for the session
    let all_deps = state.app_state.proposal_dependency_repo
        .get_all_for_session(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to get dependencies: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Build dependency map: proposal_id -> [depends_on_ids]
    let mut dep_map: HashMap<String, Vec<String>> = HashMap::new();
    for (from, to) in all_deps {
        dep_map.entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    let count = proposals.len();
    let summaries: Vec<ProposalSummary> = proposals.into_iter()
        .map(|p| {
            let id_str = p.id.to_string();
            ProposalSummary {
                id: id_str.clone(),
                title: p.title,
                category: p.category.to_string(),
                priority: p.effective_priority().to_string(),
                depends_on: dep_map.remove(&id_str).unwrap_or_default(),
                plan_artifact_id: p.plan_artifact_id.map(|id| id.to_string()),
            }
        })
        .collect();

    Ok(Json(ListProposalsResponse { proposals: summaries, count }))
}

pub async fn get_proposal(
    State(state): State<HttpServerState>,
    Path(proposal_id): Path<String>,
) -> Result<Json<ProposalDetailResponse>, StatusCode> {
    let proposal_id = TaskProposalId::from_string(proposal_id.clone());

    let proposal = state.app_state.task_proposal_repo
        .get_by_id(&proposal_id)
        .await
        .map_err(|e| {
            error!("Failed to get proposal: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get dependencies for this proposal
    let deps = state.app_state.proposal_dependency_repo
        .get_dependencies(&proposal_id)
        .await
        .map_err(|e| {
            error!("Failed to get dependencies: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Parse steps and acceptance_criteria from JSON strings
    let steps: Vec<String> = proposal.steps
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let acceptance_criteria: Vec<String> = proposal.acceptance_criteria
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    Ok(Json(ProposalDetailResponse {
        id: proposal.id.to_string(),
        session_id: proposal.session_id.to_string(),
        title: proposal.title,
        description: proposal.description,
        category: proposal.category.to_string(),
        priority: proposal.effective_priority().to_string(),
        steps,
        acceptance_criteria,
        depends_on: deps.iter().map(|d| d.to_string()).collect(),
        plan_artifact_id: proposal.plan_artifact_id.map(|id| id.to_string()),
        created_at: proposal.created_at.to_rfc3339(),
    }))
}
```

### 4. Add Routes (`mod.rs`)
**Dependencies:** Step 3
**Atomic Commit:** `feat(http_server): register proposal query routes`

Add after existing ideation routes:
```rust
// Proposal query tools (orchestrator-ideation agent)
.route("/api/list_session_proposals/:session_id", get(list_session_proposals))
.route("/api/proposal/:proposal_id", get(get_proposal))
```

### 5. Add MCP Dispatch (`index.ts`)
**Dependencies:** Step 1, Step 4
**Atomic Commit:** `feat(mcp): add GET dispatch for proposal query tools`

Add in the special GET handling section:
```typescript
} else if (name === "list_session_proposals") {
  const { session_id } = args as { session_id: string };
  result = await callTauriGet(`list_session_proposals/${session_id}`);
} else if (name === "get_proposal") {
  const { proposal_id } = args as { proposal_id: string };
  result = await callTauriGet(`proposal/${proposal_id}`);
}
```

## Response Format Examples

### list_session_proposals response:
```json
{
  "proposals": [
    {
      "id": "uuid-1",
      "title": "Create ThemeProvider context",
      "category": "feature",
      "priority": "high",
      "depends_on": [],
      "plan_artifact_id": "uuid-plan"
    },
    {
      "id": "uuid-2",
      "title": "Add dark mode toggle",
      "category": "feature",
      "priority": "medium",
      "depends_on": ["uuid-1"],
      "plan_artifact_id": "uuid-plan"
    }
  ],
  "count": 2
}
```

### get_proposal response:
```json
{
  "id": "uuid-1",
  "session_id": "session-uuid",
  "title": "Create ThemeProvider context",
  "description": "Create a React context for theme management...",
  "category": "feature",
  "priority": "high",
  "steps": [
    "Create ThemeContext.tsx",
    "Add theme provider wrapper",
    "Export useTheme hook"
  ],
  "acceptance_criteria": [
    "Theme context provides current theme",
    "useTheme hook works in child components"
  ],
  "depends_on": [],
  "plan_artifact_id": "uuid-plan",
  "created_at": "2026-01-30T10:00:00Z"
}
```

## Verification

1. **Build backend**: `cd src-tauri && cargo build`
2. **Build MCP server**: `cd ralphx-plugin/ralphx-mcp-server && npm run build`
3. **Test tools available**: Start app, verify tools appear in debug agent type
4. **Test list endpoint**: Call `list_session_proposals` with a valid session ID
5. **Test get endpoint**: Call `get_proposal` with a proposal ID from the list

## Commit Lock Workflow (Parallel Agent Coordination)

Reference: `.claude/rules/commit-lock.md`

### Before Committing
```bash
# 1. Establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Check/acquire lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Read lock content, wait 3s, retry up to 30s
  # If stale (same content >30s), delete and proceed
fi

# 3. Create lock
echo "<stream-name> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 4. Stage and commit
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

### After Committing
```bash
# ALWAYS release lock (success or failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Lock Rules
1. Acquire lock BEFORE `git add`
2. Release lock AFTER commit (success OR failure)
3. Stale = same content + >30 sec old
4. Never force-delete active lock from another agent
