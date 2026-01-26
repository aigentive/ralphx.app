# RalphX - Phase 16: Ideation Plan Artifacts

## Overview

This phase extends the ideation system to support **implementation plans as artifacts** before task proposal creation. Users can configure workflow modes (Required, Optional, Parallel), and the orchestrator creates `Specification` artifacts that serve as implementation plans linked to proposals.

**Reference Plan:**
- `specs/plans/ideation_plan_artifacts.md` - Complete implementation plan with data model, MCP tools, methodology integration, and UI mockups

## Goals

1. Add plan artifact fields to `IdeationSession` and `TaskProposal` entities
2. Create `IdeationSettings` module with SQLite persistence (separate from QA)
3. Implement MCP tools for plan artifact management (`create_plan_artifact`, `update_plan_artifact`, etc.)
4. Update orchestrator-ideation agent for plan workflow awareness
5. Implement proactive sync via ArtifactFlow (auto-update proposals when plan changes)
6. Add Ideation settings section to SettingsView (5th card)
7. Display plan artifacts in IdeationView right panel
8. Add traceability fields to Task entity for worker context access

## Dependencies

- **Phase 15A must be complete** (context-aware chat with MCP infrastructure)
- Phase 15B provides additional context but is not strictly required
- Reuses Phase 15A infrastructure:
  - MCP Server with tool scoping (`RALPHX_AGENT_TYPE`)
  - HTTP API proxy pattern
  - `--resume` for plan editing across conversation turns
  - Tool call visibility in UI

### Existing Systems Used

- **Artifacts System** - Already complete with `Specification` type and `prd-library` bucket
- **Methodology System** - For custom artifact types and templates
- **ArtifactFlow Engine** - For proactive sync automation

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/ideation_plan_artifacts.md`
2. Understand the data model, methodology integration, and proactive sync architecture
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
2. **Read the ENTIRE implementation plan** at `specs/plans/ideation_plan_artifacts.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "database",
    "description": "Create database migration for plan artifact fields and ideation settings",
    "plan_section": "Architecture - Database Migration",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'Database Migration'",
      "Add migration to src-tauri/src/infrastructure/sqlite/migrations.rs:",
      "  - ALTER ideation_sessions ADD plan_artifact_id TEXT",
      "  - ALTER task_proposals ADD plan_artifact_id TEXT",
      "  - ALTER task_proposals ADD plan_version_at_creation INTEGER",
      "  - CREATE TABLE ideation_settings (single-row pattern)",
      "  - INSERT OR IGNORE default settings row",
      "Run cargo test to verify migration applies",
      "Commit: feat(db): add plan artifact fields and ideation settings schema"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create IdeationSettings entity and repository",
    "plan_section": "Architecture - Add Ideation Settings Module",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'Add Ideation Settings Module'",
      "Create src-tauri/src/domain/ideation/ directory",
      "Create src-tauri/src/domain/ideation/config.rs:",
      "  - IdeationPlanMode enum (Required, Optional, Parallel)",
      "  - IdeationSettings struct with all fields",
      "  - Default implementation (Optional mode, no approval required)",
      "Create src-tauri/src/domain/ideation/mod.rs to export",
      "Create src-tauri/src/domain/repositories/ideation_settings_repository.rs trait",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_ideation_settings_repo.rs",
      "Update mod.rs files to export new modules",
      "Add to AppState",
      "Write unit tests",
      "Run cargo test",
      "Commit: feat(domain): add IdeationSettings entity and repository"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add plan_artifact_id fields to IdeationSession and TaskProposal entities",
    "plan_section": "Architecture - Data Model Changes",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'Add Plan Fields to Ideation Entities'",
      "Update src-tauri/src/domain/entities/ideation.rs:",
      "  - Add plan_artifact_id: Option<ArtifactId> to IdeationSession",
      "  - Add plan_artifact_id: Option<ArtifactId> to TaskProposal",
      "  - Add plan_version_at_creation: Option<u32> to TaskProposal",
      "Update SQLite ideation repository implementations to handle new fields",
      "Write unit tests for field persistence",
      "Run cargo test",
      "Commit: feat(entities): add plan artifact fields to ideation entities"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add traceability fields to Task entity for worker context",
    "plan_section": "Architecture - Add Traceability Fields to Task Entity",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'Add Traceability Fields to Task Entity'",
      "Add migration: ALTER tasks ADD source_proposal_id TEXT",
      "Add migration: ALTER tasks ADD plan_artifact_id TEXT",
      "Update src-tauri/src/domain/entities/task.rs:",
      "  - Add source_proposal_id: Option<TaskProposalId>",
      "  - Add plan_artifact_id: Option<ArtifactId>",
      "Update SQLite task repository to handle new fields",
      "Update src-tauri/src/application/apply_service.rs:",
      "  - Copy source_proposal_id and plan_artifact_id when creating task from proposal",
      "Write unit tests",
      "Run cargo test",
      "Commit: feat(entities): add traceability fields to Task for worker context"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add Tauri commands for ideation settings",
    "plan_section": "Files to Create/Modify - Backend",
    "steps": [
      "Update src-tauri/src/commands/ideation_commands.rs with commands:",
      "  - get_ideation_settings() -> IdeationSettings",
      "  - update_ideation_settings(settings: IdeationSettings) -> IdeationSettings",
      "Register commands in lib.rs invoke_handler",
      "Write unit tests for commands",
      "Run cargo test",
      "Commit: feat(commands): add ideation settings commands"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add plan artifact HTTP endpoints for MCP proxy",
    "plan_section": "MCP Tools for Plan Management",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'MCP Tools for Plan Management'",
      "Add HTTP endpoints to src-tauri/src/http_server.rs:",
      "  - POST /api/create_plan_artifact (session_id, title, content)",
      "  - POST /api/update_plan_artifact (artifact_id, content)",
      "  - GET /api/get_plan_artifact/:artifact_id",
      "  - POST /api/link_proposals_to_plan (proposal_ids, artifact_id)",
      "  - GET /api/get_session_plan/:session_id",
      "Each endpoint uses ArtifactService and IdeationService",
      "Test endpoints with curl",
      "Run cargo test",
      "Commit: feat(backend): add plan artifact HTTP endpoints for MCP proxy"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Add plan artifact tools to MCP server",
    "plan_section": "MCP Tools for Plan Management",
    "steps": [
      "Reference the mcp-builder skill (/mcp-builder) for MCP tool best practices",
      "Read specs/plans/ideation_plan_artifacts.md section 'MCP Tools for Plan Management'",
      "Create ralphx-mcp-server/src/plan-tools.ts with tool definitions:",
      "  - create_plan_artifact (session_id, title, content)",
      "  - update_plan_artifact (artifact_id, content)",
      "  - get_plan_artifact (artifact_id)",
      "  - link_proposals_to_plan (proposal_ids, artifact_id)",
      "  - get_session_plan (session_id)",
      "Update ralphx-mcp-server/src/index.ts to register tools",
      "Update TOOL_ALLOWLIST: add plan tools to orchestrator-ideation",
      "Build and verify: npm run build",
      "Commit: feat(mcp): add plan artifact tools for orchestrator-ideation"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Implement generic methodology integration infrastructure for plan artifacts",
    "plan_section": "Methodology Integration",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'Methodology Integration'",
      "Update src-tauri/src/domain/entities/methodology.rs:",
      "  - Add MethodologyPlanArtifactConfig struct (artifact_type, bucket_id)",
      "  - Add MethodologyPlanTemplate struct (id, name, description, template_content)",
      "  - Add plan_artifact_config: Option and plan_templates: Vec fields to MethodologyExtension",
      "Create get_plan_artifact_config() helper in IdeationService:",
      "  - Returns default config (Specification type, prd-library bucket)",
      "  - Infrastructure ready for future methodology configs",
      "Note: No specific methodology configs implemented yet - just the base infrastructure",
      "Run cargo test",
      "Commit: feat(methodology): add generic plan artifact config infrastructure"
    ],
    "passes": true
  },
  {
    "category": "plugin",
    "description": "Update orchestrator-ideation agent for plan workflow",
    "plan_section": "Files to Create/Modify - Plugin",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'User Experience'",
      "Update ralphx-plugin/agents/orchestrator-ideation.md:",
      "  - Add plan workflow instructions section",
      "  - Document plan mode behavior (Required/Optional/Parallel)",
      "  - Add plan creation/update tool documentation",
      "  - Add example interactions for each mode",
      "  - Explain when to suggest plans (complex features in Optional mode)",
      "Test agent invocation manually",
      "Commit: feat(plugin): update orchestrator-ideation for plan workflow"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create ideation settings types and API",
    "plan_section": "Files to Create/Modify - Frontend",
    "steps": [
      "Create src/types/ideation-config.ts:",
      "  - IdeationPlanMode type ('required' | 'optional' | 'parallel')",
      "  - IdeationSettings interface",
      "  - Zod schemas for validation",
      "Update src/api/ideation.ts with functions:",
      "  - getIdeationSettings()",
      "  - updateIdeationSettings(settings)",
      "Run npm run typecheck",
      "Commit: feat(types): add ideation settings types and API"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Update ideation types for plan artifact fields",
    "plan_section": "Files to Create/Modify - Frontend",
    "steps": [
      "Update src/types/ideation.ts:",
      "  - Add planArtifactId to IdeationSession schema",
      "  - Add planArtifactId and planVersionAtCreation to TaskProposal schema",
      "Update src/types/task.ts:",
      "  - Add sourceProposalId: string | null",
      "  - Add planArtifactId: string | null",
      "Run npm run typecheck",
      "Commit: feat(types): add plan artifact fields to ideation and task types"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create IdeationSettingsPanel component",
    "plan_section": "UI Changes - Settings Panel",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'Settings Panel - New Ideation Section'",
      "Create src/components/settings/IdeationSettingsPanel.tsx:",
      "  - Plan Workflow Mode radio group (Required/Optional/Parallel)",
      "  - 'Require explicit approval' checkbox (disabled when not Required)",
      "  - 'Suggest plans for complex features' checkbox",
      "  - 'Auto-link proposals to session plan' checkbox",
      "Use shadcn RadioGroup and Checkbox components",
      "Create useIdeationSettings hook for TanStack Query integration",
      "Create IdeationSettingsPanel.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(settings): add IdeationSettingsPanel component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add Ideation section to SettingsView",
    "plan_section": "UI Changes - Settings Panel",
    "steps": [
      "Update src/components/settings/SettingsView.tsx:",
      "  - Import IdeationSettingsPanel",
      "  - Add 5th Card with Lightbulb icon for Ideation section",
      "  - Position appropriately (after existing sections)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(settings): add Ideation section to SettingsView"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create PlanDisplay component for IdeationView",
    "plan_section": "UI Changes - IdeationView Right Panel",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'IdeationView Right Panel'",
      "Create src/components/Ideation/PlanDisplay.tsx:",
      "  - Show plan artifact title and content (markdown rendered)",
      "  - Collapse/expand functionality",
      "  - Edit and Export buttons in header",
      "  - 'Approve Plan' button when require_plan_approval is true",
      "  - Plan-proposal linkage indicator",
      "Use shadcn Card and Collapsible components",
      "Create PlanDisplay.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(ideation): add PlanDisplay component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create PlanEditor component",
    "plan_section": "UI Changes - IdeationView Right Panel",
    "steps": [
      "Create src/components/Ideation/PlanEditor.tsx:",
      "  - Markdown editor with preview toggle",
      "  - Save and Cancel buttons",
      "  - Auto-save indicator (optional)",
      "  - Calls update_plan_artifact on save",
      "Create PlanEditor.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(ideation): add PlanEditor component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Integrate plan display in IdeationView",
    "plan_section": "UI Changes - IdeationView Right Panel",
    "steps": [
      "Update src/components/Ideation/IdeationView.tsx:",
      "  - Check if session has plan_artifact_id",
      "  - If plan exists: show PlanDisplay above proposals section",
      "  - If no plan and mode is Required: show 'Waiting for plan...' message",
      "  - Connect to planArtifact state in ideationStore",
      "Update src/stores/ideationStore.ts:",
      "  - Add planArtifact state",
      "  - Add fetchPlanArtifact action",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): integrate plan display in IdeationView",
      "STOP: Output <promise>COMPLETE</promise> - Next task (proactive sync ArtifactFlow) is high-complexity event-driven, consider switching to Opus"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Implement proactive sync ArtifactFlow",
    "plan_section": "Artifact Flow for Proactive Sync",
    "steps": [
      "Read specs/plans/ideation_plan_artifacts.md section 'Artifact Flow for Proactive Sync'",
      "Create plan_updated_sync flow:",
      "  - Trigger: artifact_updated event on Specification type",
      "  - Step 1: find_linked_proposals (query proposals with matching plan_artifact_id)",
      "  - Step 2: emit plan:proposals_may_need_update event with artifact_id, proposal_ids",
      "Register flow in ArtifactFlow engine",
      "Write tests for flow trigger and execution",
      "Run cargo test",
      "Commit: feat(artifact-flow): add plan_updated_sync for proactive proposal updates",
      "STOP: Output <promise>COMPLETE</promise> - High-complexity task complete, can switch back to Sonnet"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Handle proactive sync notification in UI",
    "plan_section": "Decisions - Proactive Sync Behavior",
    "steps": [
      "Subscribe to Tauri event plan:proposals_may_need_update in IdeationView",
      "Show notification: 'Plan updated. N proposals may need revision. [Review]'",
      "On Review click: highlight affected proposals",
      "Implement undo functionality:",
      "  - Store previous proposal state before auto-update",
      "  - Show [Undo] button in notification",
      "  - Undo reverts proposals to pre-update state",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): handle proactive sync notifications"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add plan template selection infrastructure",
    "plan_section": "Decisions - Plan Templates",
    "steps": [
      "Create src/components/Ideation/PlanTemplateSelector.tsx:",
      "  - Fetch templates from active methodology via API (returns empty array if no methodology)",
      "  - Show template picker dropdown only when templates array is non-empty",
      "  - On select: pre-populate plan content with template",
      "  - Component hidden when no templates available (blank plan by default)",
      "Update PlanEditor to conditionally show template selector for new plans",
      "Note: Currently will always be hidden since no methodologies define templates yet",
      "Create PlanTemplateSelector.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(ideation): add plan template selection infrastructure"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add 'View as of proposal creation' historical view",
    "plan_section": "Decisions - Plan Versioning",
    "steps": [
      "Update proposal card in IdeationView:",
      "  - Show 'View plan as of creation' link when plan_version_at_creation differs from current",
      "  - Clicking fetches artifact at specific version",
      "  - Display in modal or slide-over panel",
      "Update src/api/artifact.ts:",
      "  - Add getArtifactVersion(artifactId, version) function",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add historical plan version view for proposals"
    ],
    "passes": true
  },
  {
    "category": "integration",
    "description": "Add plan export functionality",
    "plan_section": "Implementation Phases - Phase 7: Export & Import",
    "steps": [
      "Add 'Export' button to PlanDisplay header",
      "On click: download plan as markdown file",
      "Filename: {session_title}_plan.md or plan_{artifact_id}.md",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add plan export functionality"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Add plan import functionality",
    "plan_section": "Implementation Phases - Phase 7: Export & Import",
    "steps": [
      "Add 'Import' button to IdeationView (visible when no plan exists)",
      "On click: open file picker for .md files",
      "Read file content and create plan artifact via create_plan_artifact",
      "Link imported plan to current session",
      "Handle versioning: imported plan starts at version 1",
      "Show success notification with plan title",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add plan import functionality"
    ],
    "passes": false
  },
  {
    "category": "documentation",
    "description": "Update CLAUDE.md files for Phase 16",
    "steps": [
      "Update src/CLAUDE.md with:",
      "  - IdeationSettingsPanel component",
      "  - PlanDisplay and PlanEditor components",
      "  - Plan artifact state in ideationStore",
      "Update src-tauri/CLAUDE.md with:",
      "  - IdeationSettings entity and repository",
      "  - Plan artifact HTTP endpoints",
      "  - Proactive sync ArtifactFlow",
      "Update logs/activity.md with Phase 16 completion summary",
      "Commit: docs: update documentation for ideation plan artifacts"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

From the implementation plan:

| Decision | Rationale |
|----------|-----------|
| **Single plan per session** | Simple ownership model; multiple plans = multiple sessions |
| **Plan mode configurable** | User preferences; default to Optional (plan suggested for complex features) |
| **No explicit approval by default** | Plan existence is sufficient; conversation feedback is implicit approval |
| **Hybrid versioning** | Track version at proposal creation for historical view, show current by default |
| **Methodology-driven artifact types (generic infra)** | Base infrastructure only; no methodology = Specification; future methodologies can define custom type/bucket |
| **No templates without methodology** | Start from scratch by default; template infrastructure ready for future methodologies |
| **SQLite persistence for settings** | Settings persist across restarts; single-row pattern |
| **Task traceability fields** | `source_proposal_id` and `plan_artifact_id` enable worker context access |
| **Auto-update with undo** | Proactive sync updates proposals automatically; undo reverts to pre-update state |

---

## Verification Checklist

After completing all tasks:

### Backend - Data Model
- [ ] `plan_artifact_id` column exists in `ideation_sessions` table
- [ ] `plan_artifact_id` column exists in `task_proposals` table
- [ ] `plan_version_at_creation` column exists in `task_proposals` table
- [ ] `source_proposal_id` column exists in `tasks` table
- [ ] `plan_artifact_id` column exists in `tasks` table
- [ ] `ideation_settings` table created with single-row pattern
- [ ] All 4 settings fields persist correctly

### Backend - Settings
- [ ] `IdeationSettingsRepository` trait implemented
- [ ] `SqliteIdeationSettingsRepository` works correctly
- [ ] Default values applied on first load
- [ ] Settings changes persist across app restart

### Backend - Methodology Integration (Generic Infrastructure)
- [ ] `MethodologyPlanArtifactConfig` and `MethodologyPlanTemplate` structs defined
- [ ] `get_plan_artifact_config()` method returns default when no methodology active
- [ ] Falls back to `Specification` type and `prd-library` bucket
- [ ] Infrastructure ready for methodologies to define custom configs (not implemented yet)

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
- [ ] Export downloads plan as markdown file
- [ ] Import creates plan artifact from uploaded markdown file

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

### Proactive Sync
- [ ] Artifact flow triggers on plan update
- [ ] Notification shows affected proposal count
- [ ] Undo reverts proposals to pre-update state
- [ ] Previous proposal state stored for undo

### Task Traceability
- [ ] ApplyService copies `source_proposal_id` to task
- [ ] ApplyService copies `plan_artifact_id` to task
- [ ] Task can trace back to original proposal and plan
