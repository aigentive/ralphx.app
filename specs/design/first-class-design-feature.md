# RalphX Design Feature Spec

> Source pattern: `https://gist.github.com/adriandemian/e54a8c49b1941834f5498958279fe3fd`
>
> This adapts the gist's design-system steward idea into a RalphX-native feature. The gist describes source-grounded design-system creation, stewardship, schema output, preview artifacts, UI kits, verification, drift management, and export. RalphX should keep the same source-grounded posture, but use its own Project, chat, artifact, agent, and app-owned storage architecture.

## 1. Product Definition

`Design` is a first-class RalphX workspace for creating, maintaining, previewing, exporting, and reusing design systems derived from selected RalphX Projects.

Primary user job:
- Select one or more existing RalphX Projects as source material.
- Ask a design agent to analyze UI code, styles, assets, copy, and patterns.
- Generate a structured design-system schema stored by RalphX.
- Use that schema to generate aligned screen/component artifacts before any project code is changed.
- Export or import the schema package for reuse across projects.

Non-negotiable ownership rule:
- User project folders are source references only.
- Generated design-system config, schemas, previews, assets, run metadata, and exports live in RalphX-owned storage.
- Export/import is explicit user action, not silent writes into a project checkout.

## 2. First-Layer Scope

First layer includes:
- `Design` navbar entry.
- Project-grouped Design sidebar patterned after `AgentsSidebar`.
- Design-system list under each project.
- Create design-system flow using selected Projects as source references.
- `IntegratedChatPanel` reuse with a new `design` chat context.
- Styleguide artifact pane for a human-readable generated design system with collapsible preview rows.
- Per-row `Looks good` / `Needs work` feedback that routes into the same Design agent conversation.
- One design-system generation agent plus narrowly defined specialist roles for the eventual multi-agent version.
- Schema-backed generation of aligned screen/component artifacts inside RalphX.

First layer does not include:
- Automatic writes into the target project.
- Full Figma integration.
- Full drift scheduler.
- Multi-project brand governance dashboards.
- One-click implementation task creation, except as a later handoff from generated artifacts.

## 3. UX Architecture

### Navigation

Add a `Design` main nav item:
- View id: `design`.
- Icon: `Palette` from `lucide-react`.
- Placement: after `Agents`, before `Ideation`.
- Visibility: always on for the first implementation cut.

Required frontend updates:
- Add `"design"` to `VIEW_TYPE_VALUES`.
- Add nav item to `ALL_NAV_ITEMS`.
- Render `DesignView` from `App.tsx`.
- Keep existing project switching semantics: changing active project focuses the matching project group but does not destroy global design-system records.

### Design View Layout

Mirror the Agents surface:

```text
+-------------------------------------------------------------+
| Navigation                                                  |
+----------------------+----------------------+---------------+
| DesignSidebar         | IntegratedChatPanel  | Styleguide    |
| - Projects            | context: design      | Ready summary |
|   - Design systems    | context_id: ds_id    | Caveats       |
| - Search/filter       |                      | Groups        |
| - New design system   |                      | Preview rows  |
| - Import              |                      | Feedback      |
+----------------------+----------------------+---------------+
```

Implementation shape:
- `frontend/src/components/design/DesignView.tsx`
- `DesignSidebar.tsx`
- `DesignStyleguidePane.tsx`
- `DesignStartComposer.tsx`
- `DesignComposerSurface.tsx`
- `designSystems.ts`
- `useProjectDesignSystems.ts`
- `useDesignSystemEvents.ts`
- `useDesignStyleguideFeedbackBridge.ts`

### Sidebar Behavior

Use the Agents pattern, with design-specific rows:
- Project groups use existing `Project` records.
- Each project lists RalphX-owned `DesignSystem` records linked to that project.
- A design system row shows name, status, version, last updated, source count, and active run indicator.
- `New design system` opens a composer where the user selects source Projects and optional source folders/files.
- `Import` creates a design system from an exported schema package.
- Search filters project names and design-system names.
- Archive hides design systems without deleting stored schema versions.

Selection model:
- Clicking a design system selects it and opens chat.
- If no conversation exists, create one with `contextType = "design"` and `contextId = designSystemId`.
- Styleguide pane opens automatically when the selected design system has a published styleguide or active generation run.

### Chat Behavior

Reuse `IntegratedChatPanel`:
- Add `ContextType = "design"` in TS and Rust.
- Add `CHAT_CONTEXT_REGISTRY.design`.
- Store key prefix: `design`.
- Default placeholder: `Ask Design to analyze, refine, or generate a screen...`
- Agent type label: `design`.
- Supports streaming text and artifact tool widgets.
- Supports queueing.
- Team mode can stay off in the first cut.

Conversation lifecycle:
- Creation flow creates a draft `DesignSystem` first.
- Chat conversation is scoped to the design system, not directly to a project.
- `project_id` is still persisted on messages/runs for filtering and cost attribution, using the primary source project.

### Styleguide Pane

Default surface:
- `Styleguide`: ready summary, actionable caveats, grouped collapsed rows, and per-row preview widgets.
- `Activity`: optional event history for generation, approvals, feedback, and exports.
- `Export`: dialog action from the header, not a dominant tab.

Pane responsibilities:
- Render human design-system sections rather than raw schema.
- Keep raw JSON/YAML hidden unless the user chooses export/developer details.
- Show each generated piece as an inspectable preview widget.
- Let users approve or critique individual pieces.
- Route critiques into the same Design chat with item/source/preview metadata.
- Offer explicit export and later implementation handoff actions.

## 4. Data Ownership And Storage

### Storage Contract

RalphX owns generated design-system state:

```text
Application Support/RalphX/
  design-systems/
    <design_system_id_hash>/
      schema/
      previews/
      assets/
      exports/
      reports/
```

Rules:
- Do not use project names, source folder names, branch names, or user-provided schema names as raw path components.
- Use UUID or hash-derived storage directories.
- Persist source paths as metadata only after project path validation.
- Any export destination chosen by the user must pass path containment/permission checks.

### Database Entities

#### DesignSystem

```ts
type DesignSystem = {
  id: string;
  primaryProjectId: string;
  name: string;
  description?: string;
  status: "draft" | "analyzing" | "schema_ready" | "ready" | "updating" | "failed" | "archived";
  currentSchemaVersionId?: string;
  storageRootRef: string;
  createdAt: string;
  updatedAt: string;
  archivedAt?: string;
};
```

#### DesignSystemSource

```ts
type DesignSystemSource = {
  id: string;
  designSystemId: string;
  projectId: string;
  role: "primary" | "secondary" | "reference";
  selectedPaths: string[];
  sourceKind: "project_checkout" | "upload" | "url" | "manual_note";
  gitCommit?: string;
  sourceHashes: Record<string, string>;
  lastAnalyzedAt?: string;
};
```

#### DesignSchemaVersion

```ts
type DesignSchemaVersion = {
  id: string;
  designSystemId: string;
  version: string;
  schemaArtifactId: string;
  manifestArtifactId: string;
  styleguideArtifactId: string;
  status: "draft" | "verified" | "superseded" | "failed";
  createdByRunId?: string;
  createdAt: string;
};
```

#### DesignStyleguideItem

```ts
type DesignStyleguideItem = {
  id: string;
  designSystemId: string;
  schemaVersionId: string;
  itemId: string;
  group: "ui_kit" | "type" | "colors" | "spacing" | "components" | "brand";
  label: string;
  summary: string;
  previewArtifactId?: string;
  sourceRefs: Array<{ projectId: string; path: string; line?: number }>;
  confidence: "high" | "medium" | "low";
  approvalStatus: "needs_review" | "approved" | "needs_work";
  feedbackStatus: "none" | "open" | "in_progress" | "resolved";
  updatedAt: string;
};
```

#### DesignStyleguideFeedback

```ts
type DesignStyleguideFeedback = {
  id: string;
  designSystemId: string;
  schemaVersionId: string;
  itemId: string;
  conversationId: string;
  messageId?: string;
  previewArtifactId?: string;
  sourceRefs: Array<{ projectId: string; path: string; line?: number }>;
  feedback: string;
  status: "open" | "in_progress" | "resolved" | "dismissed";
  createdAt: string;
  resolvedAt?: string;
};
```

#### DesignRun

```ts
type DesignRun = {
  id: string;
  designSystemId: string;
  conversationId?: string;
  kind: "create" | "update" | "generate_screen" | "generate_component" | "item_feedback" | "audit" | "export" | "import";
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
  inputSummary: string;
  outputArtifactIds: string[];
  startedAt?: string;
  completedAt?: string;
  error?: string;
};
```

### Artifact Usage

Design should reuse the existing Artifact model where practical, then extend types only when needed:
- `DesignDoc`: human-readable design-system guide.
- `Specification`: generated schema.
- `Findings`: source audit and caveats.
- Add future artifact types only if filtering/rendering becomes ambiguous:
  - `DesignSchema`
  - `DesignPreview`
  - `DesignComponentPattern`
  - `DesignScreenPattern`
  - `DesignExport`

Bucket convention:
- `design-system:<design_system_id>`
- `design-preview:<design_system_id>`
- `design-reports:<design_system_id>`

## 5. Design Schema Contract

The schema is the reusable contract that future agents use when generating screens/components.

Top-level shape:

```json
{
  "schema_version": "1.0",
  "design_system": {
    "id": "uuid",
    "name": "Product Design System",
    "version": "0.1.0",
    "created_at": "2026-04-24T00:00:00Z"
  },
  "sources": [],
  "brand": {},
  "tokens": {},
  "components": [],
  "screen_patterns": [],
  "layout_patterns": [],
  "content_voice": {},
  "assets": [],
  "accessibility": {},
  "usage_rules": [],
  "caveats": [],
  "provenance": {}
}
```

Required token groups:
- Colors: primitives, semantic roles, component roles.
- Typography: font stacks, sizes, weights, line heights, code font.
- Spacing: scale, density, common component padding.
- Radius: base scale and component mappings.
- Shadow/elevation: accepted shadows and forbidden treatments.
- Borders/rings/focus states.
- Motion: duration, easing, allowed transitions.

Component pattern shape:

```json
{
  "id": "button.primary",
  "kind": "component",
  "name": "Primary Button",
  "source_refs": ["frontend/src/components/ui/button.tsx"],
  "slots": ["icon", "label"],
  "variants": ["primary", "secondary", "ghost"],
  "states": ["default", "hover", "focus", "disabled", "loading"],
  "tokens": {
    "background": "color.action.primary",
    "text": "color.action.on_primary",
    "radius": "radius.control"
  },
  "usage": {
    "do": [],
    "avoid": []
  },
  "confidence": "high"
}
```

Screen pattern shape:

```json
{
  "id": "screen.agentic_workspace",
  "kind": "screen",
  "source_refs": ["frontend/src/components/agents/AgentsView.tsx"],
  "layout": "left_sidebar_chat_artifact",
  "regions": ["project_sidebar", "chat", "artifact_pane"],
  "density": "desktop_app_compact",
  "responsive_rules": [],
  "component_refs": [],
  "content_rules": [],
  "confidence": "medium"
}
```

Provenance rule:
- Every meaningful token, asset, component rule, and screen rule needs a source reference and confidence.
- Low-confidence inference is allowed only when labeled as a caveat.

## 6. Backend Architecture

### Domain Layer

Add domain entities and repository traits:
- `DesignSystem`
- `DesignSystemSource`
- `DesignSchemaVersion`
- `DesignStyleguideItem`
- `DesignStyleguideFeedback`
- `DesignRun`
- `DesignAssetRef`
- `DesignExportPackage`

Repository traits:
- `DesignSystemRepository`
- `DesignSystemSourceRepository`
- `DesignSchemaRepository`
- `DesignStyleguideRepository`
- `DesignStyleguideFeedbackRepository`
- `DesignRunRepository`

### Application Services

`DesignSystemService`
- Create draft design systems.
- Rename/archive/list design systems.
- Link source projects.
- Resolve active schema version.

`DesignAnalysisService`
- Build source inventory from selected projects.
- Validate source path containment.
- Hash selected files.
- Produce deterministic source manifest.

`DesignGenerationService`
- Launch design agent runs through ChatService.
- Persist generated schema and previews as artifacts.
- Update `DesignRun` and `DesignSchemaVersion`.

`DesignArtifactService`
- Render the human `DesignStyleguideViewModel` from the machine schema and styleguide items.
- Resolve preview asset references.
- Map design artifacts to existing artifact APIs where possible.

`DesignFeedbackService`
- Approve styleguide items.
- Persist item-specific feedback.
- Append feedback messages into the same Design chat conversation.
- Mark item feedback active/resolved as the design agent updates the preview row.

`DesignExportImportService`
- Export schema package.
- Import package into RalphX-owned storage.
- Validate package manifest before import.

### Chat Integration

Backend updates:
- Add `ChatContextType::Design`.
- Resolve design context to primary project working directory for read-only analysis.
- Use `RALPHX_CONTEXT_TYPE=design`.
- Use `RALPHX_CONTEXT_ID=<design_system_id>`.
- Use `RALPHX_PROJECT_ID=<primary_project_id>`.
- Add design route to conversation listing and active-state hydration.

Agent resolution:
- `design` context -> `ralphx-design-steward`.
- Screen/component generation still uses same context; intent is classified by the agent and stored in `DesignRun.kind`.

### API Surface

Tauri commands:
- `list_design_systems(projectId?, includeArchived?)`
- `get_design_system(designSystemId)`
- `create_design_system(input)`
- `update_design_system(input)`
- `archive_design_system(designSystemId)`
- `list_design_sources(designSystemId)`
- `update_design_sources(designSystemId, sources)`
- `get_design_schema(designSystemId, versionId?)`
- `list_design_artifacts(designSystemId, kind?)`
- `export_design_system(designSystemId, options)`
- `import_design_system(packagePath, attachProjectId?)`

HTTP/MCP equivalents can be added only for tools the design agent actually needs.

## 7. Agent Architecture

### First-Layer Agent

`ralphx-design-steward`

Mission:
- Analyze selected source projects.
- Generate and refine a source-grounded design schema.
- Produce previews, component patterns, and screen patterns as RalphX artifacts.
- Answer questions about the design system.
- Generate schema-aligned UI artifacts without changing source projects.

Allowed first-layer tools:
- Read/search selected project source through backend-mediated source tools.
- Read/write design artifacts through design-specific MCP tools.
- Query current design system schema and source manifest.
- Create/update reports and preview records.
- Request user clarification when source authority is missing.

Do not expose low-level bookkeeping:
- No tool should ask the model to replay run IDs, timestamps, storage roots, source hashes, or status transitions.
- Backend owns run state, schema versioning, path validation, export placement, and event emission.

### Specialist Roles

These roles define the eventual multi-agent topology; first cut may run them as internal prompts or sequential phases inside `ralphx-design-steward`.

| Agent | Role | Inputs | Outputs |
|---|---|---|---|
| `ralphx-design-source-analyst` | Inventory project surfaces | selected projects, source filters | source manifest, route/component/style inventory |
| `ralphx-design-token-extractor` | Extract tokens | CSS, Tailwind, theme files, computed samples | token candidates with provenance |
| `ralphx-design-asset-curator` | Catalog assets | public assets, icons, logos, fonts | asset refs, duplicates, missing-assets caveats |
| `ralphx-design-voice-analyst` | Extract writing patterns | copy, docs, locale files | tone, CTA, casing, vocabulary rules |
| `ralphx-design-schema-writer` | Build schema | inventories and findings | versioned design-system schema |
| `ralphx-design-preview-builder` | Create previews | schema, assets | styleguide row preview widgets |
| `ralphx-design-pattern-builder` | Generate screens/components | schema, target project architecture | screen/component artifacts |
| `ralphx-design-verifier` | Verify outputs | schema, preview artifacts | report with objective checks and caveats |
| `ralphx-design-exporter` | Package schema | schema version, assets | export package manifest |

### MCP Tool Set

First-layer design MCP tools:
- `get_design_system`
- `get_design_source_manifest`
- `record_design_source_inventory`
- `upsert_design_schema_draft`
- `publish_design_schema_version`
- `create_design_artifact`
- `update_design_artifact`
- `list_design_artifacts`
- `get_design_styleguide`
- `update_design_styleguide_item`
- `record_design_styleguide_feedback`
- `record_design_verification`
- `request_design_clarification`

Tool surface rule:
- Tool descriptions must mention only Design tools visible to `ralphx-design-steward`.
- Project filesystem access must be mediated by validated source-selection tools, not raw path strings.

## 8. Source Analysis Flow

Creation flow:

1. User opens `Design`.
2. User selects `New design system`.
3. User chooses primary project and optional reference projects.
4. RalphX creates draft `DesignSystem`.
5. RalphX creates/opens `design` chat conversation for the draft.
6. User sends generation prompt.
7. Backend starts `DesignRun(kind = create)`.
8. Agent inventories selected projects.
9. Agent extracts tokens, components, layout patterns, assets, and content voice.
10. Agent writes the machine schema, source audit, and derived human styleguide.
11. Agent generates preview widgets for each styleguide item.
12. Verifier checks schema references, preview renderability, and missing-source caveats.
13. Backend publishes schema version, styleguide view model, and marks design system `ready`.
14. UI opens the `DesignStyleguidePane` on the human styleguide.

State machine:

```text
draft
  -> analyzing
  -> schema_ready
  -> ready
  -> updating
  -> ready

any active state -> failed
ready -> archived
```

Source selection rules:
- Default selection is project root, but UI should let users scope to source folders.
- Generated config never goes into selected source paths.
- Source paths are validated relative to the stored project working directory.
- In worktree mode, analysis uses the selected project checkout unless the user explicitly chooses task/worktree context in a later implementation phase.

## 9. Schema-Aligned Screen And Component Generation

First-layer generation target:
- Generate RalphX artifacts that show the proposed screen/component design.
- Do not write production files.
- Use the design schema as the authority.
- Use the target project architecture as a compatibility reference.

Flow:

1. User selects a design system.
2. User asks: "Generate a settings screen for this app" or "Create a component pattern for pricing cards."
3. Agent loads current schema version.
4. Agent loads target project architecture summary from selected Project.
5. Agent produces:
   - screen/component pattern JSON,
   - visual preview artifact,
   - implementation notes,
   - caveats if source architecture conflicts with the schema.
6. Styleguide pane opens the generated artifact row and preview widget.
7. Later workflow can hand the artifact to Ideation/Tasks, but that is not part of the first cut.

Generation contract:
- Use token names, not hardcoded values, where the target project supports tokens.
- If target project lacks a compatible token system, output an adaptation note.
- Do not invent brand motifs unsupported by schema.
- Flag every schema conflict instead of silently overriding it.

## 10. Export And Import

Export package:

```text
design-system-export.zip
  manifest.json
  schema/design-system.schema.json
  schema/component-patterns.json
  schema/screen-patterns.json
  assets/
  previews/
  reports/
```

Export options:
- Human styleguide + machine schema.
- Machine schema only.
- Styleguide previews only.
- Redacted export that strips source paths and private metadata.

Import flow:
- User chooses package.
- RalphX validates manifest and schema version.
- RalphX stores files in a new app-owned design-system root.
- User may attach imported design system to a project.
- Imported design systems show source status as `imported`, not source-grounded until re-analyzed.

Privacy rule:
- Default export should not include raw source snippets or absolute local source paths.
- Include source provenance as relative paths or redacted labels unless the user chooses full provenance.

## 11. Eventing

Add design-specific events only where chat events are insufficient:
- `design:system_created`
- `design:system_updated`
- `design:schema_published`
- `design:artifact_created`
- `design:styleguide_item_approved`
- `design:styleguide_item_feedback_created`
- `design:run_started`
- `design:run_completed`
- `design:export_completed`
- `design:import_completed`

Most live text/tool streaming remains `agent:*` through the existing chat pipeline.

Frontend query invalidation:
- `design:system_*` invalidates project design-system lists.
- `design:schema_published` invalidates schema and styleguide pane queries.
- `design:artifact_created` invalidates design artifact lists.
- `design:styleguide_item_*` invalidates the styleguide item, appends/updates the Design chat bridge message when needed, and updates row status.
- `agent:*` updates chat state via the existing event hooks once `design` is added to the context registry.

## 12. Implementation Steps

### Phase 0: Spec And Contracts

- Add this spec.
- Add schema examples under `specs/design/schemas/` when implementation begins.
- Add prompt contracts for `ralphx-design-steward`.

### Phase 1: UI Shell

- Add `design` view type and nav item.
- Create `DesignView` using the `AgentsView` split-pane model.
- Create mock-backed sidebar and `DesignStyleguidePane`.
- Add visual tests for sidebar selection, empty state, collapsed groups, expanded previews, and feedback composer.

### Phase 2: Domain And API

- Add domain entities and repository traits.
- Add SQLite migrations.
- Add Tauri command wrappers and TS types.
- Add path-safe storage root helpers.
- Add backend tests proving generated storage is RalphX-owned and project checkouts are not written.

### Phase 3: Chat Context

- Add `ChatContextType::Design`.
- Add TS `ContextType = "design"`.
- Add registry entry and store key parsing.
- Route design context to `ralphx-design-steward`.
- Add tests for send/resume/list/active-state behavior.

### Phase 4: Creation MVP

- Implement design source selection.
- Implement deterministic source inventory.
- Let the design agent produce:
  - source audit,
  - initial schema,
  - derived styleguide view model,
  - preview widgets,
  - caveats,
  - per-item source refs and confidence.
- Persist outputs as artifacts, styleguide items, and schema version.

### Phase 5: Styleguide Feedback MVP

- Render ready summary, caveat banner, collapsed groups, and expanded preview widgets.
- Add `Looks good` approvals.
- Add `Needs work` row feedback composer.
- Append row feedback into the same Design chat conversation with item/source/preview metadata.
- Add verification report as secondary row status/activity, not a default tab.

### Phase 6: Export/Import

- Implement styleguide + machine schema export.
- Keep machine-schema-only export as an advanced option.
- Implement package import.
- Add redacted provenance mode.
- Add round-trip tests.

### Phase 7: Schema-Aligned Generation

- Add screen/component generation intents.
- Generate artifacts only.
- Add later handoff point to Ideation/Tasks after explicit user approval.

## 13. Validation Plan

Backend tests:
- Creating a design system stores generated files under app-owned root.
- Source path selection rejects `..`, absolute child overrides, symlink escapes, and unknown project IDs.
- `design` chat context resolves to primary project for CWD, while storage remains RalphX-owned.
- Schema version publish updates current schema atomically.
- Styleguide view model is derived from schema but hides raw schema by default.
- Styleguide item feedback appends a chat message with item id, preview artifact id, and validated source refs.
- Export/import round trip preserves schema and strips private paths by default.

Frontend tests:
- Navigation shows `Design`.
- Design view groups design systems by project.
- Selecting a design system opens chat and styleguide pane.
- New design-system composer supports primary/reference project selection.
- Styleguide pane renders ready summary, caveat banner, collapsed groups, expanded previews, and feedback actions.
- `Needs work` submits into the same Design chat and highlights the affected row.
- `design` context key participates in event-driven chat updates.

Agent/prompt tests:
- Design prompt does not ask the model to manage storage paths, run IDs, timestamps, or source hashes.
- Design tools listed in prompt match canonical metadata and MCP allowlist.
- Prompt tells the agent to flag uncertainty and keep generation source-grounded.

Manual smoke:
- Create draft from one project.
- Generate styleguide and underlying schema.
- Verify no files appear in the project checkout.
- Open one preview row and send `Needs work` feedback into chat.
- Approve one row with `Looks good`.
- Generate one component artifact from the styleguide.
- Export styleguide + schema package.
- Import package and attach it to another project.

## 14. Open Questions

- Should a design system be attachable to multiple projects as co-equal owners, or one primary project plus references?
- Should source inventory run in the selected project checkout only, or allow explicit task worktree snapshots later?
- Should design schema artifacts use new artifact types immediately, or start with existing `Specification` / `DesignDoc` / `Findings` types?
- Should import create a global unattached design system, or require project attachment at import time?
- How much preview rendering should be static HTML vs React components inside RalphX?
- Should positive approvals append quiet Activity events only, or also optional chat notes?

## 15. Recommended First Cut

Build the first cut as:

```text
Design nav + project-grouped sidebar
+ DesignSystem domain records in RalphX storage
+ design chat context
+ ralphx-design-steward agent
+ human Design Styleguide pane
+ per-item preview widgets
+ Looks good / Needs work feedback bridge
+ styleguide + schema export/import
```

Keep code generation into user projects out of the first cut. The safest first usage layer is styleguide and preview artifact generation: let Design produce aligned screens/components in RalphX, make each piece inspectable and commentable, then add explicit implementation handoff after the design contract is proven.

## 16. V1 Clean Minimal UI

### 16.0 UX Benchmarks

Reference sources:
- Linear, `A calmer interface for a product in motion`: compact controls, reduced icon noise, softer structure, feature-flag comparison, integrated token/color tooling, and agents for source discovery. Source: `https://linear.app/now/behind-the-latest-design-refresh`
- Apple Human Interface Guidelines for macOS: large displays should show more content with fewer nested levels while keeping density comfortable; support resize, keyboard workflows, and personalization. Source: `https://developer.apple.com/design/human-interface-guidelines/designing-for-macos`
- Atlassian Design System: human navigation across foundations, components, and patterns; tokens are a single source of truth, but the design system is presented as guidance and reusable building blocks. Source: `https://atlassian.design/design-system`
- Shopify Polaris: design system as shared language for high-quality admin experiences, with foundations for accessibility, IA, internationalization, content, components, tokens, icons, and tools. Source: `https://polaris-react.shopify.com/foundations`
- Nielsen Norman Group heuristics: visibility of system status, match user language, recognition over recall, consistency, user control, and minimalist design. Source: `https://media.nngroup.com/media/articles/attachments/Heuristic_Summary1_A4_compressed.pdf`

V1 UX principles derived from those sources:
- Do not compete for attention: the styleguide pane is quiet; the preview row gets focus only when expanded.
- Structure should be felt, not seen: use soft grouping, subtle separators, and row rhythm instead of heavy panels/tabs.
- Human language first: `Buttons`, `Primary palette`, `Missing brand fonts`; never lead with schema IDs or token keys.
- Recognition over recall: keep all styleguide groups visible in one scrollable surface; avoid making users hunt across tabs.
- Inspect one piece at a time: collapsed rows give overview, expanded rows show a concrete preview widget.
- Feedback must be contextual: `Needs work` captures the current row, preview artifact, and source refs automatically.
- Show status in place: row-level approval, warnings, active regeneration, and resolved feedback belong on the row.
- Keep expert affordances secondary: raw schema, source hashes, version diffs, and exports live behind `More` or export dialogs.
- Make iteration fast: users should compare, approve, comment, and regenerate a piece without leaving the artifact pane.
- Preserve platform fit: RalphX is a Mac productivity app, so favor compact density, keyboard-friendly actions, resizable panes, and predictable sidebar/chat/detail layout.

Product posture:
- The styleguide is a review workspace, not documentation output.
- The user reviews decisions and rendered examples; RalphX manages schema, provenance, and storage in the background.
- The artifact pane should answer three questions quickly: `What changed?`, `Is this aligned?`, and `What should Design fix next?`
- Every visible control should either help review, fix, export, or understand status.

The first human-facing Design artifact is a `Design Styleguide`, not a schema browser. RalphX still persists a machine-readable schema for automation and export, but the default UI should look like a reviewable design-system checklist with collapsible preview rows.

V1 goal:
- Make the generated design system easy to inspect at a glance.
- Let users open one piece at a time and see a concrete preview widget.
- Let users say `Looks good` or `Needs work` on the exact piece.
- Route item-specific feedback into the same Design agent conversation with source refs and preview refs attached.
- Keep raw schema views out of the normal path.

### 16.1 V1 Information Architecture

```text
Design
  Project sidebar
    Project
      Design system
  Main workspace
    Chat
    Styleguide artifact
      Ready summary
      Caveats
      Styleguide groups
      Export / import actions
```

Default artifact surface:
- `Styleguide` is the primary tab and should be selected by default.
- `Activity` is optional and can show generation/update history.
- `Export` is a secondary action, not a persistent large tab.
- Raw schema is available only from `Export -> Include machine schema` or `More -> View developer schema`.

Suggested artifact header tabs for V1:

```text
[Styleguide] [Activity]                         [Manage sources] [Export] [...]
```

Recommended artifact review toolbar:

```text
Styleguide     v0.3 ready     14 reviewed / 17     Updated 4h ago
-------------------------------------------------------------------
[All] [Needs review 3] [Needs work 1] [Approved 13] [Stale 0]   [/]
```

Toolbar rules:
- Keep it one line on desktop; collapse filters into a menu at narrow widths.
- Counts come from styleguide item state, not from chat messages.
- Search filters rows by human labels, source path, and generated preview names.
- `Manage sources`, `Export`, and developer schema access are secondary actions.

Avoid in V1:
- Top-level `Schema`, `Tokens`, `Components`, `Reports` tabs.
- Raw JSON/YAML as the first thing a user sees.
- Dense schema inspectors.
- Separate pages for every token group.

### 16.1.1 Artifact Experience Model

Each styleguide item should render as four layers, in this order:

1. Review row: human label, short decision summary, status, and row actions.
2. Preview: rendered UI sample that lets the user judge the design directly.
3. Context: optional source refs, confidence, last feedback, and version detail.
4. Agent handoff: `Looks good`, `Needs work`, and `Regenerate from feedback`.

This keeps the artifact human-first while preserving enough detail for expert users to understand why RalphX made a decision.

### 16.1.2 Styleguide Row States

```text
State          Meaning                                      Primary action
-------------  -------------------------------------------  -------------------------
needs_review   Generated or changed, waiting for review      Looks good / Needs work
approved       User accepted this item                       Reopen feedback
needs_work     Feedback exists and has not been resolved      Regenerate from feedback
updating       Design agent is updating this item             View chat
stale          Source refs changed after approval             Review changes
blocked        Missing source, font, asset, or parser input   Resolve caveat
```

Row state rules:
- A row can be generated but still need review; do not use a checkmark as the only state.
- Approval is item-local and resets only when the item or its source refs materially change.
- `stale` should explain what changed in plain language.
- `blocked` should always include the next available action.

### 16.1.3 Focused Item Drawer

The expanded row should be enough for most review. A drawer is useful when a user wants provenance, comments, or a larger preview without leaving the artifact.

```text
+---------------------------------------------------------------+
| Buttons                                                  [x]  |
| Primary, secondary, ghost, icon, loading                      |
| State: Needs review     Source confidence: High               |
|---------------------------------------------------------------|
| Preview                                                       |
| [full-width rendered button matrix]                           |
|                                                               |
| Decision                                                      |
| Use 8px radius, compact vertical padding, brand primary fill. |
|                                                               |
| Sources                                                       |
| frontend/src/components/ui/button.tsx                         |
| frontend/src/styles/theme.css                                 |
|                                                               |
| Last activity                                                 |
| Generated from v0.3 source scan                               |
|                                                               |
|              [Looks good] [Needs work] [Regenerate preview]   |
+---------------------------------------------------------------+
```

Drawer rules:
- It is a focused inspection surface, not a schema viewer.
- It should open from row expansion or `Open full preview`.
- Source paths are visible but muted; absolute paths stay redacted.
- Feedback from the drawer uses the same feedback bridge and chat conversation.

### 16.2 Main Design View

```text
+----------------------------------------------------------------------------------+
| Nav: [Agents] [Design] [Ideation] [Graph] [Kanban]                         [..] |
+---------------------------+-------------------------------+----------------------+
| Design                    | Chat                          | Styleguide           |
| [+] New   [/] Search      |-------------------------------|----------------------|
|                           | RalphX App Design             | Your design system   |
| v RalphX              2   | provider: codex               | is ready             |
|   * App Design    ready   |                               |                      |
|     v0.3  4h ago          | User                          | [ ] Published        |
|   - Tahoe Audit   draft   | Update the primary palette.   |                      |
|                           |                               | Missing brand fonts  |
| v Marketing Site      1   | Design                        | [Upload fonts]       |
|   - Brand Kit     ready   | I will update that section    |                      |
|                           | and keep the source refs.     | UI Kit - Marketing   |
|                           |                               | > Marketing UI kit  v|
|                           | [ Message Design...        ] |                      |
+---------------------------+-------------------------------+----------------------+
```

Behavior:
- Sidebar selects a design system.
- Chat remains the normal `IntegratedChatPanel` with `contextType = design`.
- Right pane is a single styleguide artifact, vertically scrollable.
- Selecting feedback on a styleguide row appends a structured message to the same chat.

### 16.3 Ready Summary

```text
+--------------------------------------------------------------------------+
| Your design system is ready                                              |
|                                                                          |
| New generated artifacts can use this design system by default. You can   |
| update it any time in the chat.                                          |
|--------------------------------------------------------------------------|
| [ ] Published                                                            |
+--------------------------------------------------------------------------+
```

Summary actions:
- `Published` toggles whether new Design generation defaults to this design system for the attached project/team.
- Toggling publish is metadata-only; it does not write files to source projects.
- If multiple published design systems target the same project, RalphX should ask the user to choose one default.

### 16.4 Caveat Banner

```text
+--------------------------------------------------------------------------+
| ! Missing brand fonts                                      [Upload fonts] |
|   RalphX is rendering typography with substitute web fonts.               |
+--------------------------------------------------------------------------+
```

Caveat behavior:
- Show only actionable caveats above the styleguide groups.
- `Upload fonts` stores fonts in RalphX-owned design-system storage.
- Caveat actions should create/update DesignRun state and append a chat note when resolved.

### 16.5 Collapsed Styleguide Groups

```text
UI Kit - Marketing
+--------------------------------------------------------------------------+
| > Marketing UI kit                                                   ✓   |
+--------------------------------------------------------------------------+

Type
+--------------------------------------------------------------------------+
| > Body type                                                          ✓   |
| > Display type                                                       ✓   |
| > Mono labels                                                        ✓   |
+--------------------------------------------------------------------------+

Colors
+--------------------------------------------------------------------------+
| > Neutrals                                                           ✓   |
| > Primary palette                                                    ✓   |
| > Semantic colors                                                    ✓   |
+--------------------------------------------------------------------------+

Spacing
+--------------------------------------------------------------------------+
| > Radii                                                              ✓   |
| > Shadow tiers                                                       ✓   |
| > Spacing + flow                                                     ✓   |
+--------------------------------------------------------------------------+

Components
+--------------------------------------------------------------------------+
| > Buttons                                                            ✓   |
| > Chat bubbles                                                       ✓   |
| > Feature cards                                                      ✓   |
| > Inputs + composer                                                  ✓   |
| > Package tiers                                                      ✓   |
+--------------------------------------------------------------------------+

Brand
+--------------------------------------------------------------------------+
| > Iconography                                                        ✓   |
| > Logo lockups                                                       ✓   |
+--------------------------------------------------------------------------+
```

Group rules:
- Rows stay compact when collapsed.
- Row labels are human terms, not schema keys.
- Checkmarks mean generated and verified enough for preview, not perfect.
- Rows can show warning/error states when source confidence is low.
- The user should never need to understand token taxonomy to review the system.

### 16.6 Expanded Row With Preview And Feedback

```text
Colors
+--------------------------------------------------------------------------+
| v Primary palette                                                        |
|   RalphX orange - primary, hover, soft, ring       [Looks good] [Needs work] |
|--------------------------------------------------------------------------|
|                                                                          |
|  +------------------+ +------------------+ +---------------------------+ |
|  | PRIMARY          | | PRIMARY / HOVER  | | PRIMARY / SOFT            | |
|  |                  | |                  | |                           | |
|  | #FF6B35          | | #FF5419          | | rgba(..., 0.12)           | |
|  +------------------+ +------------------+ +---------------------------+ |
|                                                                          |
|  +--------------------------------------------------------------------+  |
|  | PRIMARY / RING                                                     |  |
|  | rgba(..., 0.24)                                                    |  |
|  +--------------------------------------------------------------------+  |
|                                                                          |
+--------------------------------------------------------------------------+
```

Expanded row behavior:
- One row can be expanded at a time within a group by default.
- The preview widget is the source of truth for human inspection.
- `Looks good` records positive approval for this item.
- `Needs work` opens an inline feedback composer.
- Feedback keeps the current chat conversation and attaches item metadata.

### 16.7 Component Preview Row

```text
Components
+--------------------------------------------------------------------------+
| v Buttons                                                                |
|   Primary, secondary, ghost, icon, loading        [Looks good] [Needs work] |
|--------------------------------------------------------------------------|
|                                                                          |
|     [ Run analysis ]    [ Secondary ]    [ Send to agent ]               |
|                                                                          |
|     [ Small CTA ]       [ Ghost ]        [ Loading ... ]                 |
|                                                                          |
+--------------------------------------------------------------------------+
| > Chat bubbles                                                       ✓   |
| > Feature cards                                                      ✓   |
| > Inputs + composer                                                  ✓   |
+--------------------------------------------------------------------------+
```

Preview requirements:
- Render realistic controls using extracted styles.
- Include states users actually care about: default, hover/focus proxy, disabled/loading where useful.
- Use generated preview widgets, not raw code snippets.
- Keep component comments local to the row.

### 16.8 UI Kit Preview Row

```text
UI Kit - Marketing
+--------------------------------------------------------------------------+
| v Marketing UI kit                                                       |
|   Landing, package cards, contact CTA             [Looks good] [Needs work] |
|--------------------------------------------------------------------------|
|                                                                          |
|  +----------------------------+  +----------------------------+           |
|  | Hero / desktop             |  | Package tiers              |           |
|  | [mini rendered preview]    |  | [mini rendered preview]    |           |
|  +----------------------------+  +----------------------------+           |
|                                                                          |
|  [Open full preview] [Regenerate from feedback]                          |
+--------------------------------------------------------------------------+
```

UI kit behavior:
- Preview cards are inspectable summaries; full preview opens in a focused panel/modal.
- `Regenerate from feedback` is enabled after the user has left unresolved feedback on this row.
- Full preview is still read-only and stored by RalphX.

### 16.9 Needs Work Feedback Flow

Inline composer:

```text
+--------------------------------------------------------------------------+
| v Buttons                                                                |
|   Primary, secondary, ghost, icon, loading        [Looks good] [Needs work] |
|--------------------------------------------------------------------------|
| Feedback                                                                 |
| [ The primary button is too pill-shaped. Match the app's 8px radius.    ] |
|                                                                          |
| Attachments                                                              |
| - item: components.buttons                                               |
| - preview: design-preview/buttons.v3                                     |
| - source refs: frontend/src/components/ui/button.tsx                     |
|                                                                          |
|                                      [Cancel] [Send feedback to Design]  |
+--------------------------------------------------------------------------+
```

Chat result:

```text
Design styleguide feedback
Item: Components / Buttons
Preview: design-preview/buttons.v3
Source refs: frontend/src/components/ui/button.tsx

The primary button is too pill-shaped. Match the app's 8px radius.
```

Feedback contract:
- The feedback is appended to the same Design conversation as a normal user-visible message.
- Metadata carries the exact styleguide item id, preview artifact id, source refs, and optional snapshot/version id.
- The Design agent responds in that same conversation and can patch only the affected styleguide item/artifacts.
- The artifact pane highlights the row while a feedback run is active.

### 16.10 Looks Good Flow

```text
+--------------------------------------------------------------------------+
| v Primary palette                                                        |
|   RalphX orange - primary, hover, soft, ring       [Looks good] [Needs work] |
+--------------------------------------------------------------------------+
```

After click:

```text
+--------------------------------------------------------------------------+
| v Primary palette                                                        |
|   Approved by user  2026-04-24                         [Reopen feedback] |
+--------------------------------------------------------------------------+
```

Approval behavior:
- Records `styleguide_item.approval_status = approved`.
- Does not need to append noisy chat by default.
- Optional event can be logged in `Activity`.
- If a later source update changes the item, approval resets to `needs_review`.

### 16.11 Feedback Hook Architecture

The feedback bridge should reuse the existing project-agent bridge pattern represented by `useProjectAgentBridgeEvents`:
- UI action emits or directly persists a design feedback event.
- Backend appends a bridge message into the active Design conversation.
- React Query invalidates the conversation and styleguide item queries.
- The user sees feedback in the same chat thread, not a separate comments panel.

New hook shape:

```ts
useDesignStyleguideFeedbackBridge({
  designSystemId,
  conversationId,
  activeSchemaVersionId,
});
```

Feedback event payload:

```ts
type DesignStyleguideFeedbackEvent = {
  eventType: "design:styleguide_feedback";
  eventKey: string;
  designSystemId: string;
  conversationId: string;
  schemaVersionId: string;
  itemId: string;
  itemGroup: string;
  itemLabel: string;
  previewArtifactId?: string;
  sourceRefs: Array<{ projectId: string; path: string; line?: number }>;
  feedback: string;
  createdAt: string;
};
```

Bridge message metadata:

```json
{
  "kind": "design_styleguide_feedback",
  "designSystemId": "...",
  "schemaVersionId": "...",
  "itemId": "components.buttons",
  "previewArtifactId": "design-preview-buttons-v3",
  "sourceRefs": [
    { "projectId": "...", "path": "frontend/src/components/ui/button.tsx" }
  ]
}
```

Backend responsibilities:
- Validate the design system and item id.
- Validate source refs against selected project source manifests.
- Create a `DesignRun(kind = item_feedback)` or append to the active run if one exists.
- Append the bridge message through the same chat-message path used for project-agent bridge messages.
- Emit `design:styleguide_item_feedback_created` and `agent:message_created`.

Frontend responsibilities:
- Keep the row expanded after feedback submission.
- Show active/queued status at row level.
- Invalidate `designStyleguideKeys.item(designSystemId, itemId)` and the conversation query.
- Scroll the chat to the appended bridge message when appropriate.

### 16.12 Minimal Activity View

```text
+--------------------------------------------------------------------------+
| Activity                                                                 |
+--------------------------------------------------------------------------+
| 10:42  Generated initial styleguide                                      |
| 10:45  User approved Primary palette                                     |
| 10:47  Feedback added on Buttons                                         |
| 10:49  Buttons regenerated from feedback                                 |
| 10:51  Exported styleguide package                                        |
+--------------------------------------------------------------------------+
```

Activity rules:
- Keep it secondary.
- Show human-readable events only.
- Link activity rows back to styleguide items when possible.

### 16.13 Export Dialog, Not Primary Tab

```text
+---------------------------------------------------------------+
| Export design system                                     [x]  |
+---------------------------------------------------------------+
| Contents                                                      |
| (o) Human styleguide + machine schema                         |
| ( ) Machine schema only                                       |
| ( ) Styleguide previews only                                  |
|                                                               |
| Privacy                                                       |
| [x] Redact absolute paths                                     |
| [x] Exclude raw source snippets                               |
| [ ] Include full provenance                                   |
|                                                               |
| Format                                                        |
| (o) Zip package                                               |
| ( ) JSON only                                                 |
|                                                               |
|                                      [Cancel] [Export]        |
+---------------------------------------------------------------+
```

Export rule:
- Export is a dialog from the styleguide header.
- The normal review UI should not expose raw schema unless the user asks for export/developer details.

### 16.14 Import Dialog

```text
+---------------------------------------------------------------+
| Import design system                                     [x]  |
+---------------------------------------------------------------+
| [ Drop package or choose file...                            ] |
|                                                               |
| Validation                                                    |
| [ok] manifest found                                           |
| [ok] schema supported                                         |
| [warn] no preview widgets included                            |
|                                                               |
| Attach to project                                             |
| [ RalphX                                             v ]       |
|                                                               |
| (o) Import as reference                                       |
| ( ) Import and re-analyze against attached project            |
|                                                               |
|                                      [Cancel] [Import]        |
+---------------------------------------------------------------+
```

### 16.15 Manage Sources Drawer

```text
+---------------------------------------------------------------+
| Manage sources                                           [x]  |
+---------------------------------------------------------------+
| Primary project                                               |
| [ RalphX                                             v ]       |
|                                                               |
| Source scope                                                  |
| [x] UI files   [x] Styles   [x] Assets   [x] Copy   [ ] Tests |
|                                                               |
| Included paths                                                |
| frontend/src                                                  |
| frontend/src/styles                                           |
| public                                                        |
|                                                               |
| Reference projects                                            |
| [x] Marketing Site                                            |
| [ ] Docs Site                                                 |
|                                                               |
|                          [Cancel] [Save] [Re-analyze]         |
+---------------------------------------------------------------+
```

### 16.15.1 Artifact State Screens

Empty state:

```text
+---------------------------------------------------------------+
| Design                                                        |
|---------------------------------------------------------------|
| No design system selected                                     |
|                                                               |
| Create a design system from a project or import an existing   |
| styleguide package.                                           |
|                                                               |
|                         [New design system] [Import]          |
+---------------------------------------------------------------+
```

Generating state:

```text
+---------------------------------------------------------------+
| Styleguide                                      Generating...  |
|---------------------------------------------------------------|
| Analyzing selected sources                                     |
| [#####-------------------------------]  UI patterns            |
|                                                               |
| Latest note                                                    |
| Found reusable button, composer, and card patterns.            |
|                                                               |
| Chat remains available while the styleguide is generated.      |
+---------------------------------------------------------------+
```

Item updating state:

```text
+---------------------------------------------------------------+
| v Buttons                                             Updating |
|   Applying feedback from chat                                  |
|---------------------------------------------------------------|
| The preview stays visible until the updated version is ready.  |
|                                      [View chat] [Cancel run]  |
+---------------------------------------------------------------+
```

Stale item state:

```text
+---------------------------------------------------------------+
| > Primary palette                                      Stale   |
|   Source colors changed after approval                         |
|                                      [Review changes]          |
+---------------------------------------------------------------+
```

Compare view:

```text
+---------------------------------------------------------------+
| Primary palette - review changes                         [x]  |
|---------------------------------------------------------------|
| Previous approved                 New generated               |
| +-----------------------------+   +-------------------------+ |
| | #1F39D2 primary             |   | #2348E8 primary         | |
| | rgba(..., 0.16) ring        |   | rgba(..., 0.18) ring    | |
| +-----------------------------+   +-------------------------+ |
|                                                               |
| Source change                                                  |
| frontend/src/styles/theme.css updated token primary.           |
|                                                               |
|                         [Keep previous] [Accept new] [Comment] |
+---------------------------------------------------------------+
```

State rules:
- Keep the old preview visible while a row is regenerating.
- Compare views open only for stale or regenerated rows, not as a permanent tab.
- `Cancel run` cancels the item feedback run only when backend cancellation is available.
- Empty and generating states should preserve the chat/artifact layout so the interface does not jump.

### 16.16 V1 Screen Inventory

| Screen | Required In V1 | Notes |
|---|---:|---|
| Design main view | Yes | Project sidebar + chat + styleguide pane |
| Empty state | Yes | Create/import entrypoints only |
| New design system composer | Yes | Select project/source scope |
| Design Styleguide artifact | Yes | Default artifact view |
| Review toolbar and filters | Yes | All, needs review, needs work, approved, stale |
| Collapsed styleguide groups | Yes | Human labels and status marks |
| Expanded preview row | Yes | Per-piece preview widget |
| Focused item drawer | Yes | Larger preview, sources, activity, row actions |
| Compare stale/regenerated item | Nice-to-have | Required only once stale-state support lands |
| `Looks good` action | Yes | Records item approval |
| `Needs work` action | Yes | Opens feedback composer and appends chat message |
| Feedback bridge hook | Yes | Same conversation, item metadata attached |
| Item updating state | Yes | Shows active feedback/regeneration run |
| Missing asset/font caveats | Yes | Actionable banner above groups |
| Export dialog | Yes | Secondary action; schema hidden by default |
| Import dialog | Yes | Validate then store in RalphX-owned root |
| Activity view | Nice-to-have | Human event history only |
| Raw schema viewer | Later/debug | Not visible in normal V1 flow |
| Separate schema/token/component tabs | No | Replaced by single Styleguide surface |

### 16.17 V1 Implementation Delta

Update the earlier implementation phases with this simplified target:

1. Replace `DesignArtifactPane` tab model with a `DesignStyleguidePane` default surface.
2. Persist machine schema, but render a derived `DesignStyleguideViewModel` for the UI.
3. Add styleguide item entities or metadata:
   - `item_id`
   - `group`
   - `label`
   - `summary`
   - `preview_artifact_id`
   - `source_refs`
   - `confidence`
   - `approval_status`
   - `feedback_status`
   - `review_state`
   - `source_status`
   - `last_feedback_message_id`
   - `last_feedback_run_id`
4. Add feedback commands:
   - `approve_design_styleguide_item`
   - `create_design_styleguide_feedback`
   - `resolve_design_styleguide_feedback`
5. Add bridge/event plumbing:
   - `design:styleguide_item_feedback_created`
   - `design:styleguide_item_approved`
   - append feedback into the same `design` chat conversation.
6. Add focused tests:
   - feedback on a styleguide item appends a chat message with item/source metadata,
   - approving a row changes only item state,
   - row filters reflect review state counts,
   - item update keeps the previous preview visible while regeneration is active,
   - source refs are validated against the selected source manifest,
   - raw schema is not the default artifact view.
