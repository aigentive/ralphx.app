# Ideation Plan Artifacts Implementation Plan

## Overview

This plan extends the ideation system to support **implementation plans as artifacts** before task proposal creation. Users can configure whether plans are required, optional, or created in parallel with proposals.

**Problem Statement:**
Currently, the ideation orchestrator jumps directly from user conversation to task proposals. For complex features, there's a need for an intermediate planning phase where architecture, approach, and implementation details are documented before breaking down into tasks.

**Solution:**
Integrate the existing artifacts system into the ideation flow, allowing the orchestrator to create `Specification` artifacts that serve as implementation plans. Proposals can then reference these plans, and the system can proactively suggest updates when plans change.

---

## Dependencies

### Phase 15 (Context-Aware Chat) - Required

This plan depends on Phase 15 infrastructure:

| Dependency | Why Needed |
|------------|------------|
| MCP Server with tool scoping | Plan tools will be scoped to `orchestrator-ideation` via `RALPHX_AGENT_TYPE` |
| HTTP API proxy pattern | Plan artifact tools follow same proxy pattern as proposal tools |
| `--resume` session management | Plan editing across multiple conversation turns |
| Stream parsing and persistence | Plan creation progress visible in chat |
| Tool call visibility in UI | User sees when orchestrator creates/updates plans |

### Artifacts System - Already Complete

The artifacts system is production-ready:
- `Artifact` entity with `Specification` type
- `prd-library` system bucket (writers: `orchestrator`, `user`)
- `ArtifactService` for CRUD operations
- `ArtifactFlow` engine for event-driven automation
- Frontend components and stores

### Settings System - Partial (needs persistence)

Current state:
- Settings stored in memory (`RwLock<QASettings>`)
- **Not persisted to SQLite** - resets on app restart
- UI components exist (`SettingsView`, `QASettingsPanel`)

For this feature, we need to either:
1. Add SQLite persistence for settings (recommended)
2. Or store plan workflow preference separately

---

## Architecture

### Data Model Changes

#### 1. Add `plan_artifact_id` to TaskProposal

```rust
// src-tauri/src/domain/entities/ideation.rs
pub struct TaskProposal {
    // ... existing fields ...

    /// Optional reference to the implementation plan artifact
    pub plan_artifact_id: Option<ArtifactId>,
}
```

#### 2. Add Settings Field

```rust
// src-tauri/src/domain/qa/config.rs (or new settings module)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeationPlanMode {
    /// Plan must exist before proposals can be created
    Required,
    /// Plan is optional, orchestrator suggests for complex features
    Optional,
    /// Plan and proposals created together, changes suggest sync
    Parallel,
}

impl Default for IdeationPlanMode {
    fn default() -> Self {
        Self::Optional  // Default to optional
    }
}
```

#### 3. Database Migration

```sql
-- Add plan_artifact_id to task_proposals
ALTER TABLE task_proposals ADD COLUMN plan_artifact_id TEXT REFERENCES artifacts(id);

-- Add settings table (if not exists)
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### MCP Tools for Plan Management

Add to `TOOL_ALLOWLIST` for `orchestrator-ideation`:

| Tool | Parameters | Returns | Description |
|------|------------|---------|-------------|
| `create_plan_artifact` | `session_id`, `title`, `content` | `ArtifactId` | Create specification artifact linked to ideation session |
| `update_plan_artifact` | `artifact_id`, `content` | `Artifact` | Update plan content (creates new version) |
| `get_plan_artifact` | `artifact_id` | `Artifact` | Retrieve plan for context |
| `link_proposals_to_plan` | `proposal_ids[]`, `artifact_id` | `void` | Set plan reference on multiple proposals |
| `get_session_plan` | `session_id` | `Option<Artifact>` | Get plan artifact for current session |

### Artifact Flow for Proactive Sync

```rust
// New flow: plan_updated_sync
ArtifactFlow {
    name: "plan_updated_sync",
    trigger: ArtifactFlowTrigger {
        event: ArtifactFlowEvent::ArtifactUpdated,
        filter: Some(ArtifactFlowFilter {
            artifact_types: Some(vec![ArtifactType::Specification]),
            source_bucket: Some("prd-library"),
        }),
    },
    steps: vec![
        // Step 1: Find linked proposals
        ArtifactFlowStep::Custom {
            action: "find_linked_proposals",
        },
        // Step 2: Emit notification event
        ArtifactFlowStep::EmitEvent {
            event_type: "plan:proposals_may_need_update",
            payload: "{ artifact_id, proposal_ids }",
        },
    ],
    is_active: true,
}
```

---

## User Experience

### Workflow Mode: Required

```
User: "I want to add real-time collaboration"
        ↓
Orchestrator: "This is a significant feature. Let me create an implementation
              plan first. I'll cover architecture, data flow, and key decisions."
        ↓
[Orchestrator calls create_plan_artifact]
        ↓
UI: Shows plan artifact in right panel (alongside empty proposals section)
        ↓
Orchestrator: "Here's the plan. Key decisions:
              1. WebSocket vs SSE
              2. Conflict resolution strategy
              3. Presence indicators

              Does this approach look right before I break it into tasks?"
        ↓
User: "Yes, go with WebSockets and OT for conflicts"
        ↓
[Orchestrator calls update_plan_artifact with user decisions]
[Orchestrator calls create_task_proposal × N]
[Orchestrator calls link_proposals_to_plan]
        ↓
UI: Shows plan + linked proposals
```

### Workflow Mode: Optional

```
User: "Add a logout button"
        ↓
Orchestrator: "Simple feature - I'll create the task proposal directly."
        ↓
[Orchestrator calls create_task_proposal]
        ↓
(No plan artifact created - feature is simple)

---

User: "Implement authentication system"
        ↓
Orchestrator: "This is a complex feature. Would you like me to create an
              implementation plan first, or should I go straight to tasks?

              [Create Plan First] [Skip to Tasks]"
        ↓
User: [Create Plan First]
        ↓
(Follows Required workflow)
```

### Workflow Mode: Parallel

```
User: "Add dark mode"
        ↓
Orchestrator: Creates plan AND proposals simultaneously
        ↓
UI: Shows both in real-time as they're created
        ↓
Later: User edits plan
        ↓
UI: "Plan updated. 3 linked proposals may need revision. [Review]"
        ↓
User: [Review]
        ↓
Orchestrator: Analyzes changes, suggests proposal updates
```

### UI Changes

#### IdeationView Right Panel

```
┌─────────────────────────────────────────────────────────────────┐
│  Implementation Plan                              [Edit] [Export]│
├─────────────────────────────────────────────────────────────────┤
│  ## Real-time Collaboration Plan                                │
│                                                                  │
│  ### Architecture                                                │
│  - WebSocket server for real-time sync                          │
│  - OT (Operational Transform) for conflict resolution           │
│  ...                                                             │
│                                                                  │
│  [Collapse]                                                      │
├─────────────────────────────────────────────────────────────────┤
│  Proposals (4)                          [Select All] [Apply ▾]  │
├─────────────────────────────────────────────────────────────────┤
│  ☑ WebSocket server setup                    [High] [Feature]   │
│  ☑ OT engine implementation                  [High] [Feature]   │
│  ☐ Presence indicators                       [Med]  [Feature]   │
│  ☐ Connection status UI                      [Low]  [Feature]   │
└─────────────────────────────────────────────────────────────────┘
```

#### Settings Panel Addition

```
┌─────────────────────────────────────────────────────────────────┐
│  Ideation                                                        │
├─────────────────────────────────────────────────────────────────┤
│  Plan Workflow Mode                                              │
│  Control when implementation plans are created                   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ ○ Required - Plan must be created before proposals          ││
│  │ ● Optional - Plan suggested for complex features (Default)  ││
│  │ ○ Parallel - Plan and proposals created together            ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Data Model & Backend

1. Add `plan_artifact_id` to `TaskProposal` entity
2. Add database migration
3. Update `IdeationService` with plan-related methods
4. Add `IdeationPlanMode` to settings
5. Implement settings persistence (SQLite)

### Phase 2: MCP Tools & HTTP API

1. Create plan artifact tools in MCP server
2. Add HTTP endpoints to Tauri backend
3. Update `TOOL_ALLOWLIST` for orchestrator-ideation
4. Wire up tool calls to `ArtifactService`

### Phase 3: Orchestrator Agent Update

1. Update `orchestrator-ideation.md` agent definition
2. Add plan workflow instructions based on mode
3. Add plan creation/update tool documentation
4. Update example interactions

### Phase 4: Frontend - Plan Display

1. Add plan artifact section to IdeationView right panel
2. Implement plan editor (markdown with preview)
3. Add plan collapse/expand functionality
4. Show plan-proposal linkage indicator

### Phase 5: Frontend - Settings

1. Add `IdeationPlanMode` selector to SettingsView
2. Wire up to settings store and backend
3. Add settings persistence hook

### Phase 6: Proactive Sync (Artifact Flow)

1. Create `plan_updated_sync` artifact flow
2. Implement `find_linked_proposals` custom action
3. Add `plan:proposals_may_need_update` event handling
4. Create notification UI for stale proposals
5. Implement "Review Changes" workflow

### Phase 7: Export & Import

1. Add "Export Plan" button (downloads as markdown)
2. Add "Import Plan" option (upload existing spec)
3. Handle plan versioning on import

---

## Files to Create/Modify

### Backend (Rust)

| File | Changes |
|------|---------|
| `src-tauri/src/domain/entities/ideation.rs` | Add `plan_artifact_id` to `TaskProposal` |
| `src-tauri/src/domain/entities/settings.rs` | New file for `IdeationPlanMode` |
| `src-tauri/src/infrastructure/sqlite/migrations.rs` | Add migration for `plan_artifact_id` |
| `src-tauri/src/application/ideation_service.rs` | Add plan-related methods |
| `src-tauri/src/commands/ideation_commands.rs` | Add plan-related commands |
| `src-tauri/src/commands/settings_commands.rs` | Add/update settings commands |
| `src-tauri/src/infrastructure/sqlite/sqlite_settings_repo.rs` | New file for settings persistence |

### MCP Server (TypeScript)

| File | Changes |
|------|---------|
| `ralphx-mcp-server/src/tools/plan-tools.ts` | New file with plan artifact tools |
| `ralphx-mcp-server/src/tool-allowlist.ts` | Add plan tools to orchestrator-ideation |
| `ralphx-mcp-server/src/http-proxy.ts` | Add plan endpoints |

### Frontend (React/TypeScript)

| File | Changes |
|------|---------|
| `src/types/ideation.ts` | Add `planArtifactId` to `TaskProposal` |
| `src/types/settings.ts` | Add `IdeationPlanMode` type |
| `src/components/Ideation/IdeationView.tsx` | Add plan display section |
| `src/components/Ideation/PlanEditor.tsx` | New file for plan editing |
| `src/components/Ideation/PlanDisplay.tsx` | New file for plan display |
| `src/components/settings/SettingsView.tsx` | Add ideation plan mode setting |
| `src/api/ideation.ts` | Add plan-related API calls |
| `src/stores/ideationStore.ts` | Add plan artifact state |

### Plugin (Agent Definition)

| File | Changes |
|------|---------|
| `ralphx-plugin/agents/orchestrator-ideation.md` | Add plan workflow instructions and tool docs |

---

## Open Questions

### 1. Plan Approval Workflow

**Question:** In "Required" mode, should the plan require explicit user approval before proposals can be created?

**Options:**
- A) Yes - User must click "Approve Plan" button
- B) No - Plan existence is sufficient; user feedback in conversation is implicit approval
- C) Configurable - Add separate `require_plan_approval` setting

**Considerations:**
- Option A adds friction but ensures deliberate decisions
- Option B is faster but user might miss reviewing the plan
- Option C is flexible but adds settings complexity

---

### 2. Proactive Sync Behavior

**Question:** When a plan is updated, should the system auto-update proposals or just notify?

**Options:**
- A) Notify only - "Plan updated. Review linked proposals?"
- B) Suggest updates - Show diff of what would change, user confirms
- C) Auto-update - Orchestrator automatically revises proposals (with undo)

**Considerations:**
- Option A is safest but requires manual work
- Option B balances automation with user control
- Option C is most automated but may surprise users

---

### 3. Artifact Type

**Question:** Should we use the existing `Specification` type or add a new `ImplementationPlan` type?

**Options:**
- A) Use `Specification` - Already exists, semantic fit
- B) Add `ImplementationPlan` - Clearer distinction, better querying
- C) Add `IdeationPlan` - Specific to ideation context

**Considerations:**
- Option A avoids schema changes
- Option B/C allow filtering plans specifically
- Existing `prd-library` bucket accepts `Specification`

---

### 4. Plan Versioning

**Question:** How should plan versions be handled when proposals exist?

**Options:**
- A) Proposals link to latest version only - `plan_artifact_id` always points to current
- B) Proposals link to specific version - Preserves historical context
- C) Hybrid - Show current version but allow viewing version at proposal creation time

**Considerations:**
- Option A is simpler but loses history
- Option B is more accurate but complex
- Option C provides best UX but most implementation work

---

### 5. Multiple Plans per Session

**Question:** Can an ideation session have multiple plans (e.g., one per major feature)?

**Options:**
- A) Single plan per session - Simple, clear ownership
- B) Multiple plans allowed - Each proposal links to one plan
- C) Hierarchical - One main plan with sub-plans

**Considerations:**
- Option A is simpler for MVP
- Option B supports diverse sessions
- Option C matches complex project structures

---

### 6. Plan Templates

**Question:** Should we provide plan templates for common scenarios?

**Options:**
- A) No templates - Start from scratch each time
- B) Basic templates - "Feature Plan", "Refactor Plan", "Integration Plan"
- C) Methodology-driven templates - BMAD/GSD provide their own templates

**Considerations:**
- Option A is simplest
- Option B speeds up common cases
- Option C integrates with extensibility system

---

### 7. Settings Persistence Priority

**Question:** Should we implement full settings persistence now or defer?

**Options:**
- A) Implement now - All settings to SQLite
- B) Partial - Just `IdeationPlanMode` to SQLite
- C) Defer - Store in localStorage via frontend (temporary)

**Considerations:**
- Option A is most complete but more work
- Option B is focused on this feature
- Option C is fastest but technical debt

---

## Verification Checklist

### Backend
- [ ] `plan_artifact_id` column exists in `task_proposals` table
- [ ] `IdeationPlanMode` setting persisted to database
- [ ] Plan artifact tools create artifacts in `prd-library` bucket
- [ ] `link_proposals_to_plan` updates multiple proposals atomically
- [ ] Settings changes emit Tauri event for frontend sync

### MCP Server
- [ ] Plan tools in TOOL_ALLOWLIST for orchestrator-ideation
- [ ] HTTP endpoints proxied correctly
- [ ] Tool responses include artifact IDs

### Frontend
- [ ] Plan display shows in IdeationView when plan exists
- [ ] Plan editor supports markdown with preview
- [ ] Settings UI shows plan mode selector
- [ ] Plan-proposal linkage visible in proposal cards

### Agent
- [ ] Orchestrator respects plan mode setting
- [ ] Plan created before proposals in Required mode
- [ ] User prompted for plan in Optional mode (complex features)
- [ ] Plan and proposals created together in Parallel mode

### Proactive Sync
- [ ] Artifact flow triggers on plan update
- [ ] Notification shown for stale proposals
- [ ] Review workflow allows accepting/rejecting suggested changes

---

## Related Documents

- `specs/plans/context_aware_chat_implementation.md` - Phase 15A infrastructure
- `specs/plans/task_execution_chat.md` - Phase 15B worker execution
- `specs/plans/workflow_management_ui.md` - Methodology-workflow relationship
- `src-tauri/src/domain/entities/artifact.rs` - Artifact types and system buckets
