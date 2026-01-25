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

#### 1. Add Plan Fields to Ideation Entities

```rust
// src-tauri/src/domain/entities/ideation.rs

/// Add to IdeationSession (single plan per session)
pub struct IdeationSession {
    // ... existing fields ...

    /// The implementation plan artifact for this session
    pub plan_artifact_id: Option<ArtifactId>,
}

/// Add to TaskProposal (tracks version at creation for hybrid versioning)
pub struct TaskProposal {
    // ... existing fields ...

    /// Reference to the implementation plan artifact
    pub plan_artifact_id: Option<ArtifactId>,
    /// Plan version when this proposal was created (for historical view)
    pub plan_version_at_creation: Option<u32>,
}
```

#### 2. Add Ideation Settings Module

```rust
// src-tauri/src/domain/ideation/config.rs (NEW FILE - separate from QA)
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

/// Ideation-specific settings (separate from QA settings)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdeationSettings {
    /// How implementation plans are created in ideation flow
    pub plan_mode: IdeationPlanMode,
    /// In Required mode, whether explicit approval is needed before proposals
    pub require_plan_approval: bool,
    /// Whether to show plan suggestions for complex features (in Optional mode)
    pub suggest_plans_for_complex: bool,
    /// Auto-link proposals to session plan when created
    pub auto_link_proposals: bool,
}

impl Default for IdeationSettings {
    fn default() -> Self {
        Self {
            plan_mode: IdeationPlanMode::Optional,
            require_plan_approval: false,  // Plan existence is sufficient by default
            suggest_plans_for_complex: true,
            auto_link_proposals: true,
        }
    }
}
```

#### 3. Database Migration

```sql
-- Add plan_artifact_id to ideation_sessions (single plan per session)
ALTER TABLE ideation_sessions ADD COLUMN plan_artifact_id TEXT REFERENCES artifacts(id);

-- Add plan fields to task_proposals (with version tracking)
ALTER TABLE task_proposals ADD COLUMN plan_artifact_id TEXT REFERENCES artifacts(id);
ALTER TABLE task_proposals ADD COLUMN plan_version_at_creation INTEGER;

-- Create ideation_settings table with single-row pattern
CREATE TABLE IF NOT EXISTS ideation_settings (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),  -- Ensures single row
    plan_mode TEXT NOT NULL DEFAULT 'optional',
    require_plan_approval INTEGER NOT NULL DEFAULT 0,
    suggest_plans_for_complex INTEGER NOT NULL DEFAULT 1,
    auto_link_proposals INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed default settings row
INSERT OR IGNORE INTO ideation_settings (id, updated_at) VALUES (1, datetime('now'));
```

### Methodology Integration

When a methodology is active, it can provide custom artifact types, buckets, and templates:

```rust
// src-tauri/src/domain/entities/methodology.rs (extend existing)
pub struct MethodologyExtension {
    // ... existing fields ...

    /// Custom artifact configuration for ideation plans
    pub plan_artifact_config: Option<MethodologyPlanArtifactConfig>,
    /// Plan templates provided by this methodology
    pub plan_templates: Vec<MethodologyPlanTemplate>,
}

pub struct MethodologyPlanArtifactConfig {
    /// Artifact type to use for plans (existing or custom)
    pub artifact_type: String,  // String to allow methodology-defined types
    /// Bucket to store plans in
    pub bucket_id: String,
}

pub struct MethodologyPlanTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Markdown template with {{placeholders}}
    pub template_content: String,
}
```

**Resolution Logic:**

```rust
fn get_plan_artifact_config(active_methodology: Option<&MethodologyExtension>) -> PlanArtifactConfig {
    match active_methodology.and_then(|m| m.plan_artifact_config.as_ref()) {
        Some(config) => PlanArtifactConfig {
            artifact_type: config.artifact_type.clone(),
            bucket_id: config.bucket_id.clone(),
        },
        None => PlanArtifactConfig {
            artifact_type: "specification".to_string(),
            bucket_id: "prd-library".to_string(),
        },
    }
}

fn get_plan_templates(active_methodology: Option<&MethodologyExtension>) -> Vec<MethodologyPlanTemplate> {
    active_methodology
        .map(|m| m.plan_templates.clone())
        .unwrap_or_default()  // Empty = no templates (start from scratch)
}
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

#### Settings Panel - New Ideation Section

SettingsView currently has 4 sections: Execution, Model, Review, Supervisor.
Add a 5th section for **Ideation** (with Lightbulb icon):

```
┌─────────────────────────────────────────────────────────────────┐
│  💡 Ideation                                                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Plan Workflow Mode                                              │
│  Control when implementation plans are created                   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ ○ Required - Plan must be created before proposals          ││
│  │ ● Optional - Plan suggested for complex features (Default)  ││
│  │ ○ Parallel - Plan and proposals created together            ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ ☐ Require explicit plan approval (in Required mode)         ││
│  │   User must click "Approve Plan" before creating proposals  ││
│  │   (disabled when not in Required mode)                      ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ ☑ Suggest plans for complex features                        ││
│  │   When in Optional mode, prompt user for complex features   ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ ☑ Auto-link proposals to session plan                       ││
│  │   Automatically set plan reference when creating proposals  ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Data Model & Backend

1. Add `plan_artifact_id` to `TaskProposal` entity
2. Create `src-tauri/src/domain/ideation/config.rs` with `IdeationSettings`, `IdeationPlanMode`
3. Create `IdeationSettingsRepository` trait
4. Implement `SqliteIdeationSettingsRepository`
5. Add database migration for `plan_artifact_id` column and `ideation_settings` table
6. Update `IdeationService` with plan-related methods
7. Add ideation settings to `AppState`

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

1. Create `src/types/ideation-config.ts` with Zod schemas
2. Create `src/components/settings/IdeationSettingsPanel.tsx`
3. Add Ideation section (5th card with Lightbulb icon) to SettingsView
4. Create `src/hooks/useIdeationSettings.ts` for TanStack Query integration
5. Add ideation settings state to ideationStore
6. Wire up settings API calls

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
| `src-tauri/src/domain/ideation/config.rs` | **New file** for `IdeationSettings`, `IdeationPlanMode` |
| `src-tauri/src/domain/ideation/mod.rs` | **New module** exporting ideation config |
| `src-tauri/src/infrastructure/sqlite/migrations.rs` | Add migration for `plan_artifact_id` + `ideation_settings` table |
| `src-tauri/src/application/ideation_service.rs` | Add plan-related methods |
| `src-tauri/src/commands/ideation_commands.rs` | Add plan-related commands + settings commands |
| `src-tauri/src/infrastructure/sqlite/sqlite_ideation_settings_repo.rs` | **New file** for ideation settings persistence |
| `src-tauri/src/domain/repositories/ideation_settings_repository.rs` | **New file** for repository trait |

### MCP Server (TypeScript)

| File | Changes |
|------|---------|
| `ralphx-mcp-server/src/tools/plan-tools.ts` | New file with plan artifact tools |
| `ralphx-mcp-server/src/tool-allowlist.ts` | Add plan tools to orchestrator-ideation |
| `ralphx-mcp-server/src/http-proxy.ts` | Add plan endpoints |

### Frontend (React/TypeScript)

| File | Changes |
|------|---------|
| `src/types/ideation.ts` | Add `planArtifactId` to `TaskProposal`, add `IdeationSettings` |
| `src/types/ideation-config.ts` | **New file** for `IdeationPlanMode`, `IdeationSettings` schemas |
| `src/components/Ideation/IdeationView.tsx` | Add plan display section |
| `src/components/Ideation/PlanEditor.tsx` | **New file** for plan editing |
| `src/components/Ideation/PlanDisplay.tsx` | **New file** for plan display |
| `src/components/settings/SettingsView.tsx` | Add new Ideation section (5th card) |
| `src/components/settings/IdeationSettingsPanel.tsx` | **New file** for ideation settings UI |
| `src/api/ideation.ts` | Add plan-related + settings API calls |
| `src/stores/ideationStore.ts` | Add plan artifact state + ideation settings |
| `src/hooks/useIdeationSettings.ts` | **New file** for ideation settings hook |

### Plugin (Agent Definition)

| File | Changes |
|------|---------|
| `ralphx-plugin/agents/orchestrator-ideation.md` | Add plan workflow instructions and tool docs |

---

## Decisions

### 1. Plan Approval Workflow

**Decision:** Configurable with default to NO (plan existence is sufficient)

**Implementation:**
- Add `require_plan_approval: bool` to `IdeationSettings` (default: `false`)
- When `true` in Required mode: show "Approve Plan" button before proposals can be created
- When `false`: plan existence is sufficient; user feedback in conversation is implicit approval

```rust
pub struct IdeationSettings {
    pub plan_mode: IdeationPlanMode,
    pub require_plan_approval: bool,  // NEW - default false
    // ...
}
```

---

### 2. Proactive Sync Behavior

**Decision:** Auto-update with undo capability

**Implementation:**
- When plan is updated, orchestrator automatically revises linked proposals
- Show notification: "Plan updated. 3 proposals revised. [Undo] [View Changes]"
- Undo reverts proposals to pre-update state
- Store previous proposal state before auto-update for undo functionality
- Use `ArtifactFlow` to trigger sync on `artifact_updated` event

---

### 3. Artifact Type - Methodology-Driven

**Decision:** Artifact type mapped based on active methodology

**Implementation:**
- **No methodology active:** Use existing `Specification` type in `prd-library` bucket
- **Methodology active:** Methodology defines its own artifact type and bucket
  - BMAD might use `BmadAnalysisDocument` in `bmad-artifacts` bucket
  - GSD might use `GsdPlanDocument` in `gsd-artifacts` bucket

```rust
// In methodology extension definition
pub struct MethodologyArtifactConfig {
    pub plan_artifact_type: ArtifactType,  // Custom or existing
    pub plan_bucket_id: ArtifactBucketId,
}

// Default (no methodology)
fn default_plan_artifact_config() -> MethodologyArtifactConfig {
    MethodologyArtifactConfig {
        plan_artifact_type: ArtifactType::Specification,
        plan_bucket_id: ArtifactBucketId::from_string("prd-library"),
    }
}
```

**Note:** May require adding custom artifact types to the enum or a more dynamic type system for methodology-defined types.

---

### 4. Plan Versioning

**Decision:** Hybrid approach - show current version but track version at proposal creation

**Implementation:**
- `TaskProposal.plan_artifact_id` points to the artifact (not a specific version)
- Add `TaskProposal.plan_version_at_creation: u32` to track which version existed when proposal was created
- UI shows current plan content by default
- "View as of proposal creation" option shows historical version
- Artifact versioning already exists (`metadata.version` field)

```rust
pub struct TaskProposal {
    pub plan_artifact_id: Option<ArtifactId>,
    pub plan_version_at_creation: Option<u32>,  // NEW
    // ...
}
```

---

### 5. Multiple Plans per Session

**Decision:** Single plan per session

**Implementation:**
- One `plan_artifact_id` per `IdeationSession`
- Simple, clear ownership model
- If user needs multiple plans, create multiple sessions
- Session can be titled to reflect the plan focus

```rust
pub struct IdeationSession {
    pub plan_artifact_id: Option<ArtifactId>,  // Single plan
    // ...
}
```

**Rationale:** Keeps MVP simple. Multiple plans can be added later if needed.

---

### 6. Plan Templates

**Decision:** No templates if no methodology; methodology-driven templates if active

**Implementation:**
- **No methodology active:** Start from scratch (blank plan)
- **Methodology active:** Methodology provides plan templates
  - Templates defined in methodology extension
  - User can select from available templates when creating plan
  - Template pre-populates plan structure

```rust
// In methodology extension
pub struct MethodologyPlanTemplate {
    pub name: String,
    pub description: String,
    pub template_content: String,  // Markdown template with placeholders
}

pub struct MethodologyExtension {
    pub plan_templates: Vec<MethodologyPlanTemplate>,
    // ...
}
```

**Default (no methodology):** Empty array, no template selection UI shown.

---

### 7. Settings Persistence

**Decision:** SQLite persistence for ideation settings

**Implementation:**
- Create `ideation_settings` table in SQLite
- `IdeationSettingsRepository` trait with `SqliteIdeationSettingsRepository` implementation
- Settings persist across app restarts
- Becomes the pattern for future settings modules

```sql
CREATE TABLE ideation_settings (
    id INTEGER PRIMARY KEY DEFAULT 1,
    plan_mode TEXT NOT NULL DEFAULT 'optional',
    require_plan_approval INTEGER NOT NULL DEFAULT 0,
    suggest_plans_for_complex INTEGER NOT NULL DEFAULT 1,
    auto_link_proposals INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL
);

-- Ensure single row
INSERT OR IGNORE INTO ideation_settings (id) VALUES (1);
```

---

## Verification Checklist

### Backend - Data Model
- [ ] `plan_artifact_id` column exists in `ideation_sessions` table
- [ ] `plan_artifact_id` column exists in `task_proposals` table
- [ ] `plan_version_at_creation` column exists in `task_proposals` table
- [ ] `ideation_settings` table created with single-row pattern
- [ ] All 4 settings fields persist correctly

### Backend - Settings
- [ ] `IdeationSettingsRepository` trait implemented
- [ ] `SqliteIdeationSettingsRepository` works correctly
- [ ] Settings changes emit Tauri event for frontend sync
- [ ] Default values applied on first load

### Backend - Methodology Integration
- [ ] Active methodology's artifact config used when present
- [ ] Falls back to `Specification` type and `prd-library` bucket when no methodology
- [ ] Methodology plan templates accessible via API

### MCP Server
- [ ] Plan tools in TOOL_ALLOWLIST for orchestrator-ideation
- [ ] HTTP endpoints proxied correctly
- [ ] Tool responses include artifact IDs
- [ ] Artifact type resolution considers active methodology

### Frontend - Settings
- [ ] Ideation section appears in SettingsView (5th card, Lightbulb icon)
- [ ] Plan mode selector works (Required/Optional/Parallel)
- [ ] "Require explicit approval" toggle disabled when not in Required mode
- [ ] All settings persist across app restart

### Frontend - Plan Display
- [ ] Plan display shows in IdeationView when plan exists
- [ ] Plan editor supports markdown with preview
- [ ] "Approve Plan" button shows when `require_plan_approval` is true
- [ ] Plan-proposal linkage visible in proposal cards
- [ ] "View as of proposal creation" shows historical version

### Frontend - Templates
- [ ] Template selector shows when methodology provides templates
- [ ] No template UI when no methodology active
- [ ] Selected template pre-populates plan content

### Agent Behavior
- [ ] Orchestrator respects plan mode setting
- [ ] Plan created before proposals in Required mode
- [ ] Waits for approval when `require_plan_approval` is true
- [ ] User prompted for plan in Optional mode (complex features only)
- [ ] Plan and proposals created together in Parallel mode

### Proactive Sync (Auto-Update)
- [ ] Artifact flow triggers on plan update
- [ ] Orchestrator auto-revises linked proposals
- [ ] Notification shows: "Plan updated. N proposals revised. [Undo] [View Changes]"
- [ ] Undo reverts proposals to pre-update state
- [ ] Previous proposal state stored for undo functionality

---

## Related Documents

- `specs/plans/context_aware_chat_implementation.md` - Phase 15A infrastructure
- `specs/plans/task_execution_chat.md` - Phase 15B worker execution
- `specs/plans/workflow_management_ui.md` - Methodology-workflow relationship
- `src-tauri/src/domain/entities/artifact.rs` - Artifact types and system buckets
