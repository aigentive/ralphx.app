# RalphX - Phase 11: Extensibility

## Overview

Phase 11 implements the Extensibility System - the complete framework for custom workflows, methodology support (BMAD, GSD), artifact management, and deep research loops. This phase transforms RalphX from a fixed-workflow tool into a configurable platform that adapts to different development methodologies.

**Key insight**: A methodology is a combination of Workflow + Agents + Artifacts. When a user activates a methodology, the Kanban columns change to reflect that methodology's workflow while still mapping to internal statuses for consistent side effects.

## Dependencies

- **Phase 1 (Foundation)**: Core entities, database setup, TypeScript types
- **Phase 2 (Data Layer)**: Repository pattern, SQLite infrastructure
- **Phase 5 (Frontend Core)**: Zustand stores, TanStack Query, event system
- **Phase 6 (Kanban UI)**: TaskBoard for dynamic column rendering
- **Phase 7 (Agent System)**: AgentProfile, RalphX plugin structure

## Scope

### Included
- Custom Workflow Schemas with external-to-internal status mapping
- WorkflowRepository and WorkflowService
- Built-in workflows: Default RalphX, Jira-Compatible
- Artifact System with types, buckets, and flows
- ArtifactRepository and ArtifactService
- Artifact flow engine with trigger-based routing
- Deep Research Loops with configurable depth presets
- ResearchProcess entity and ProcessRepository
- Methodology Support (BMAD, GSD)
- MethodologyExtension schema and activation
- Extensibility database migrations
- UI components: WorkflowEditor, ArtifactBrowser, ResearchLauncher, MethodologyBrowser
- Zustand stores: workflowStore, artifactStore

### Excluded
- External sync with Jira/GitHub/Linear/Notion (marked as future)
- Third-party methodology marketplace
- Custom MCP server development UI

---

## Detailed Requirements

### Custom Workflow Schemas

Users can define custom boards that map to internal statuses, enabling Jira-style, GitHub-style, or methodology-specific workflows.

#### WorkflowSchema Interface

```typescript
interface WorkflowSchema {
  id: string;
  name: string;
  description: string;
  columns: WorkflowColumn[];
  externalSync?: ExternalSyncConfig;
  defaults: {
    workerProfile?: string;
    reviewerProfile?: string;
  };
}

interface WorkflowColumn {
  id: string;
  name: string;              // Display: "In QA", "Ready for Dev", "Selected"
  color?: string;
  icon?: string;
  mapsTo: InternalStatus;    // Maps to internal status for side effects
  behavior?: {
    skipReview?: boolean;
    autoAdvance?: boolean;
    agentProfile?: string;   // Override agent for this column
  };
}

// External sync configuration (future implementation)
interface ExternalSyncConfig {
  provider: "jira" | "github" | "linear" | "notion";
  mapping: Record<string, ExternalStatusMapping>;
  sync: {
    direction: "pull" | "push" | "bidirectional";
    webhook?: boolean;
  };
  conflictResolution: "external_wins" | "internal_wins" | "manual";
}
```

#### Built-in Workflows

**Default RalphX:**
```typescript
const defaultWorkflow: WorkflowSchema = {
  id: "ralphx-default",
  name: "RalphX Default",
  columns: [
    { id: "draft", name: "Draft", mapsTo: "backlog" },
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "todo", name: "To Do", mapsTo: "ready" },
    { id: "planned", name: "Planned", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_review", name: "In Review", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
};
```

**Jira-Compatible:**
```typescript
const jiraWorkflow: WorkflowSchema = {
  id: "jira-compat",
  name: "Jira Compatible",
  columns: [
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "selected", name: "Selected for Dev", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_qa", name: "In QA", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
  externalSync: { provider: "jira", direction: "bidirectional" },
};
```

---

### Artifact System

Artifacts are typed documents that flow between processes - outputs from one process become inputs to another.

#### Artifact Types

```typescript
type ArtifactType =
  // Documents
  | "prd" | "research_document" | "design_doc" | "specification"
  // Code
  | "code_change" | "diff" | "test_result"
  // Process
  | "task_spec" | "review_feedback" | "approval" | "findings" | "recommendations"
  // Context
  | "context" | "previous_work" | "research_brief"
  // Logs
  | "activity_log" | "alert" | "intervention";
```

#### Artifact Entity

```typescript
interface Artifact {
  id: string;
  type: ArtifactType;
  name: string;
  content: ArtifactContent;
  metadata: {
    createdAt: Date;
    createdBy: string;  // Agent profile ID or "user"
    taskId?: string;
    processId?: string;
    version: number;
  };
  derivedFrom?: string[];  // Parent artifact IDs
}

type ArtifactContent =
  | { type: "inline"; text: string }
  | { type: "file"; path: string };
```

#### Artifact Buckets

Buckets organize artifacts by purpose with access control:

| Bucket | Accepted Types | Writers | Readers |
|--------|---------------|---------|---------|
| `research-outputs` | research_document, findings, recommendations | deep-researcher, orchestrator | all |
| `work-context` | context, task_spec, previous_work | orchestrator, system | worker, reviewer |
| `code-changes` | code_change, diff, test_result | worker | reviewer |
| `prd-library` | prd, specification, design_doc | orchestrator, user | all |

```typescript
interface ArtifactBucket {
  id: string;
  name: string;
  acceptedTypes: ArtifactType[];
  writers: string[];  // Agent profile IDs or "user" or "system"
  readers: string[];  // Agent profile IDs or "all"
  isSystem: boolean;
}
```

#### Artifact Flow Engine

Automate artifact routing between processes:

```typescript
interface ArtifactFlow {
  id: string;
  trigger: {
    event: "artifact_created" | "task_completed" | "process_completed";
    filter?: { artifactTypes?: ArtifactType[]; sourceBucket?: string };
  };
  steps: ArtifactFlowStep[];
}

type ArtifactFlowStep =
  | { type: "copy"; toBucket: string }
  | { type: "spawn_process"; processType: string; agentProfile: string };
```

**Example Flow: Research → Task Decomposition**
```typescript
const researchToDevFlow: ArtifactFlow = {
  id: "research-to-dev",
  trigger: {
    event: "artifact_created",
    filter: { artifactTypes: ["recommendations"], sourceBucket: "research-outputs" },
  },
  steps: [
    { type: "copy", toBucket: "prd-library" },
    { type: "spawn_process", processType: "task_decomposition", agentProfile: "orchestrator" },
  ],
};
```

---

### Deep Research Loops

Support for long-running research agents with configurable depth.

#### ResearchProcess Entity

```typescript
interface ResearchProcess {
  id: string;
  name: string;
  brief: {
    question: string;
    context?: string;
    scope?: string;
    constraints?: string[];
  };
  depth: ResearchDepthPreset | CustomDepth;
  agentProfileId: string;
  output: {
    targetBucket: string;
    artifactTypes: ArtifactType[];
  };
  progress: {
    currentIteration: number;
    status: "pending" | "running" | "paused" | "completed" | "failed";
    lastCheckpoint?: string;  // Artifact ID
  };
}

interface CustomDepth {
  maxIterations: number;
  timeoutHours: number;
  checkpointInterval: number;  // Save progress every N iterations
}
```

#### Research Depth Presets

| Preset | Iterations | Timeout | Use Case |
|--------|------------|---------|----------|
| `quick-scan` | 10 | 30 min | Fast overview |
| `standard` | 50 | 2 hrs | Thorough investigation |
| `deep-dive` | 200 | 8 hrs | Comprehensive analysis |
| `exhaustive` | 500 | 24 hrs | Leave no stone unturned |

```typescript
type ResearchDepthPreset = "quick-scan" | "standard" | "deep-dive" | "exhaustive";

const RESEARCH_PRESETS: Record<ResearchDepthPreset, CustomDepth> = {
  "quick-scan": { maxIterations: 10, timeoutHours: 0.5, checkpointInterval: 5 },
  "standard": { maxIterations: 50, timeoutHours: 2, checkpointInterval: 10 },
  "deep-dive": { maxIterations: 200, timeoutHours: 8, checkpointInterval: 25 },
  "exhaustive": { maxIterations: 500, timeoutHours: 24, checkpointInterval: 50 },
};
```

#### Integration with Orchestrator

Before creating tasks, the Orchestrator spawns deep-researcher if:
- Task requires technology decision
- Domain is unfamiliar
- User explicitly requests research

Research outputs become:
1. Context artifacts for workers
2. Input for PRD refinement
3. Basis for task decomposition

---

### Methodology Support

RalphX can support external development methodologies as extensions.

#### Key Insight
A methodology brings its own Kanban board structure. When a user activates a methodology, the Kanban columns change to reflect that methodology's workflow while still mapping to internal statuses for side effects.

#### MethodologyExtension Schema

```typescript
interface MethodologyExtension {
  id: string;
  name: string;
  description: string;

  // Agent profiles this methodology provides
  agentProfiles: AgentProfile[];

  // Skills bundled with methodology
  skills: string[];  // Paths to skill directories

  // Custom workflow for this methodology
  workflow: WorkflowSchema;

  // Phase/stage definitions
  phases?: {
    id: string;
    name: string;
    order: number;
    agentProfiles: string[];  // Which agents work in this phase
  }[];

  // Document templates
  templates?: {
    type: ArtifactType;
    templatePath: string;
  }[];

  // Hooks for methodology-specific behavior
  hooks?: HooksConfig;
}
```

#### BMAD Method Integration

**BMAD** (Breakthrough Method for Agile AI-Driven Development) uses:
- **8 agents**: Analyst, PM, Architect, UX Designer, Developer, Scrum Master, TEA, Tech Writer
- **4 phases**: Analysis → Planning → Solutioning → Implementation
- **Document-centric**: PRD, Architecture Doc, UX Design, Stories/Epics

```typescript
const bmadWorkflow: WorkflowSchema = {
  id: "bmad-method",
  name: "BMAD Method",
  description: "Breakthrough Method for Agile AI-Driven Development",
  columns: [
    // Phase 1: Analysis
    { id: "brainstorm", name: "Brainstorm", mapsTo: "backlog",
      behavior: { agentProfile: "bmad-analyst" } },
    { id: "research", name: "Research", mapsTo: "executing",
      behavior: { agentProfile: "bmad-analyst" } },

    // Phase 2: Planning
    { id: "prd-draft", name: "PRD Draft", mapsTo: "executing",
      behavior: { agentProfile: "bmad-pm" } },
    { id: "prd-review", name: "PRD Review", mapsTo: "pending_review",
      behavior: { agentProfile: "bmad-pm" } },
    { id: "ux-design", name: "UX Design", mapsTo: "executing",
      behavior: { agentProfile: "bmad-ux" } },

    // Phase 3: Solutioning
    { id: "architecture", name: "Architecture", mapsTo: "executing",
      behavior: { agentProfile: "bmad-architect" } },
    { id: "stories", name: "Stories", mapsTo: "ready",
      behavior: { agentProfile: "bmad-pm" } },

    // Phase 4: Implementation
    { id: "sprint", name: "Sprint", mapsTo: "executing",
      behavior: { agentProfile: "bmad-developer" } },
    { id: "code-review", name: "Code Review", mapsTo: "pending_review",
      behavior: { agentProfile: "bmad-developer" } },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
};
```

**BMAD to RalphX Mapping:**
| BMAD Concept | RalphX Equivalent |
|--------------|-------------------|
| Agent personas | Agent profiles with different skills |
| Workflows (BP, CP, CA, DS) | Skills with step-based execution |
| Documents (PRD, Architecture) | Artifacts in buckets |
| Phase progression | Workflow columns (each phase = column group) |
| Validation checklists | Review hooks |

#### GSD Method Integration

**GSD** (Get Shit Done) uses:
- **11 agents**: project-researcher, phase-researcher, planner, executor, verifier, debugger, etc.
- **Wave-based parallelization**: Plans grouped into waves for parallel execution
- **Checkpoint protocol**: human-verify, decision, human-action types
- **Goal-backward verification**: must-haves derived from phase goals

```typescript
const gsdWorkflow: WorkflowSchema = {
  id: "gsd-method",
  name: "GSD (Get Shit Done)",
  description: "Spec-driven development with wave-based parallelization",
  columns: [
    // Initialize
    { id: "initialize", name: "Initialize", mapsTo: "backlog",
      behavior: { agentProfile: "gsd-project-researcher" } },

    // Discuss (optional)
    { id: "discuss", name: "Discuss", mapsTo: "blocked",
      behavior: { agentProfile: "gsd-orchestrator" } },

    // Plan
    { id: "research", name: "Research", mapsTo: "executing",
      behavior: { agentProfile: "gsd-phase-researcher" } },
    { id: "planning", name: "Planning", mapsTo: "executing",
      behavior: { agentProfile: "gsd-planner" } },
    { id: "plan-check", name: "Plan Check", mapsTo: "pending_review",
      behavior: { agentProfile: "gsd-plan-checker" } },

    // Execute (wave-based)
    { id: "queued", name: "Queued", mapsTo: "ready" },
    { id: "executing", name: "Executing", mapsTo: "executing",
      behavior: { agentProfile: "gsd-executor" } },
    { id: "checkpoint", name: "Checkpoint", mapsTo: "blocked" },

    // Verify
    { id: "verifying", name: "Verifying", mapsTo: "pending_review",
      behavior: { agentProfile: "gsd-verifier" } },
    { id: "debugging", name: "Debugging", mapsTo: "revision_needed",
      behavior: { agentProfile: "gsd-debugger" } },

    // Complete
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
};
```

**GSD-specific task fields:**
```typescript
interface GSDTask extends Task {
  wave?: number;           // Wave 1, 2, 3... for parallel execution
  checkpoint_type?: "auto" | "human-verify" | "decision" | "human-action";
  phase_id?: string;       // "01-setup", "02-core", etc.
  plan_id?: string;        // "01-01", "01-02" within phase
  must_haves?: {
    truths: string[];      // Observable behaviors
    artifacts: string[];   // Required file paths
    key_links: string[];   // Component connections to verify
  };
}
```

**GSD to RalphX Mapping:**
| GSD Concept | RalphX Equivalent |
|-------------|-------------------|
| Phases + Plans | Tasks with `phase_id` and `plan_id` fields |
| Waves | Task `wave` field for parallel execution grouping |
| Checkpoints | Task `checkpoint_type` + `blocked` internal status |
| Must-haves | Task `must_haves` field + verification hooks |
| Model profiles | Agent profile `execution.model` setting |
| STATE.md | Activity log + task state history |

#### Methodology Switching Flow

When user activates a methodology:
1. **Workflow changes** - Kanban columns update to methodology's workflow
2. **Agent profiles load** - Methodology's agents become available
3. **Skills inject** - Methodology's skills available to agents
4. **Artifact templates ready** - Document templates in buckets
5. **Hooks activate** - Methodology-specific lifecycle hooks

```
User selects "BMAD Method" for project
       ↓
Load bmadWorkflow → Update Kanban columns
       ↓
Load BMAD agent profiles (analyst, pm, architect, etc.)
       ↓
Inject BMAD skills into agents
       ↓
Create artifact buckets (prd-drafts, architecture-docs, etc.)
       ↓
Activate BMAD hooks (validation checklists, phase gates)
       ↓
Project now uses BMAD workflow with all side effects intact
```

---

### Extensibility Database Schema

```sql
-- Workflows
CREATE TABLE workflows (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  schema_json TEXT NOT NULL,  -- Full WorkflowSchema as JSON
  is_default BOOLEAN DEFAULT FALSE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Artifacts
CREATE TABLE artifacts (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  name TEXT NOT NULL,
  content_type TEXT NOT NULL,  -- "inline" | "file"
  content_text TEXT,
  content_path TEXT,
  bucket_id TEXT REFERENCES artifact_buckets(id),
  task_id TEXT REFERENCES tasks(id),
  process_id TEXT REFERENCES processes(id),
  created_by TEXT NOT NULL,
  version INTEGER DEFAULT 1,
  previous_version_id TEXT,
  metadata_json TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Artifact Buckets
CREATE TABLE artifact_buckets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  config_json TEXT NOT NULL,
  is_system BOOLEAN DEFAULT FALSE
);

-- Artifact Relations (derivedFrom, relatedTo)
CREATE TABLE artifact_relations (
  id TEXT PRIMARY KEY,
  from_artifact_id TEXT NOT NULL REFERENCES artifacts(id),
  to_artifact_id TEXT NOT NULL REFERENCES artifacts(id),
  relation_type TEXT NOT NULL  -- "derived_from" | "related_to"
);

-- Artifact Flows
CREATE TABLE artifact_flows (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  trigger_json TEXT NOT NULL,
  steps_json TEXT NOT NULL,
  is_active BOOLEAN DEFAULT TRUE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Processes (Research loops, etc.)
CREATE TABLE processes (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,  -- "research" | "development" | "review"
  name TEXT NOT NULL,
  config_json TEXT NOT NULL,
  status TEXT NOT NULL,
  current_iteration INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  started_at DATETIME,
  completed_at DATETIME
);

-- Task extensions for methodology support
ALTER TABLE tasks ADD COLUMN external_status TEXT;
ALTER TABLE tasks ADD COLUMN wave INTEGER;  -- For parallel execution grouping
ALTER TABLE tasks ADD COLUMN checkpoint_type TEXT;  -- "auto" | "human-verify" | "decision" | "human-action"
ALTER TABLE tasks ADD COLUMN phase_id TEXT;
ALTER TABLE tasks ADD COLUMN plan_id TEXT;
ALTER TABLE tasks ADD COLUMN must_haves_json TEXT;

-- Task dependencies (explicit, separate from blockers)
CREATE TABLE task_dependencies (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  depends_on_task_id TEXT NOT NULL REFERENCES tasks(id),
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Methodology extensions
CREATE TABLE methodology_extensions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  config_json TEXT NOT NULL,
  is_active BOOLEAN DEFAULT FALSE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_artifacts_bucket ON artifacts(bucket_id);
CREATE INDEX idx_artifacts_type ON artifacts(type);
CREATE INDEX idx_tasks_wave ON tasks(wave);
CREATE INDEX idx_processes_status ON processes(status);
CREATE INDEX idx_artifact_relations_from ON artifact_relations(from_artifact_id);
CREATE INDEX idx_artifact_relations_to ON artifact_relations(to_artifact_id);
```

---

### UI Components

#### Component Directory Structure

```
src/components/
├── workflows/
│   ├── WorkflowEditor.tsx      # Create/edit custom workflows
│   └── WorkflowSelector.tsx    # Choose methodology or workflow
├── artifacts/
│   ├── ArtifactBrowser.tsx     # Browse artifacts by bucket/type
│   ├── ArtifactCard.tsx        # Individual artifact display
│   └── ArtifactFlow.tsx        # Visualize artifact routing
├── research/
│   ├── ResearchLauncher.tsx    # Start research process
│   ├── ResearchProgress.tsx    # Monitor active research
│   └── ResearchResults.tsx     # View research outputs
└── methodologies/
    ├── MethodologyBrowser.tsx  # Browse/install methodologies
    └── MethodologyConfig.tsx   # Configure methodology settings
```

#### Zustand Stores

```typescript
// workflowStore.ts
interface WorkflowStore {
  workflows: WorkflowSchema[];
  activeWorkflowId: string | null;
  setActiveWorkflow: (id: string) => void;
  addWorkflow: (workflow: WorkflowSchema) => void;
  updateWorkflow: (id: string, updates: Partial<WorkflowSchema>) => void;
  deleteWorkflow: (id: string) => void;
}

// artifactStore.ts
interface ArtifactStore {
  artifacts: Map<string, Artifact>;
  buckets: ArtifactBucket[];
  selectedBucketId: string | null;
  selectedArtifactId: string | null;
  setSelectedBucket: (id: string) => void;
  setSelectedArtifact: (id: string) => void;
  addArtifact: (artifact: Artifact) => void;
}

// methodologyStore.ts
interface MethodologyStore {
  methodologies: MethodologyExtension[];
  activeMethodologyId: string | null;
  activateMethodology: (id: string) => Promise<void>;
  deactivateMethodology: () => Promise<void>;
}
```

---

### Extension Points Summary

| Extension Point | Description | Implementation |
|-----------------|-------------|----------------|
| **Custom Workflows** | Define board layouts with custom columns | `WorkflowSchema` JSON in database |
| **Status Mappings** | Map external statuses to internal ones | `WorkflowColumn.mapsTo` field |
| **External Sync** | Bidirectional sync with Jira/GitHub/etc | `ExternalSyncConfig` + provider adapters (future) |
| **Artifact Types** | Define new document categories | Type enum extension |
| **Artifact Buckets** | Create storage/routing buckets | `ArtifactBucket` config |
| **Artifact Flows** | Automate artifact routing | `ArtifactFlow` trigger rules |
| **Research Presets** | Custom research depth configs | `ResearchDepthPreset` |
| **Methodologies** | BMAD, GSD, custom methods | `MethodologyExtension` packages |

---

### Key Architecture Principles

1. **Leverage Claude Code's native system** - Use plugins, skills, agents, hooks directly instead of reinventing
2. **Internal status = side effects** - 9 internal statuses with documented, predictable behavior
3. **External status = UI flexibility** - Custom workflows map to internal statuses
4. **Agents = Claude Code components** - Composed of agent definitions, skills, hooks
5. **Artifacts = typed I/O** - Documents flow between processes through typed buckets
6. **Methodologies = configuration** - BMAD, GSD, etc. are configuration packages, not code changes

This architecture enables:
- Adding new methodologies without code changes
- Custom workflows that still trigger correct side effects
- Agent specialization through skill composition
- Research → Planning → Execution artifact flow
- Third-party plugin ecosystem via Claude Code marketplace

---

## Implementation Notes

### File Size Limits
- UI components: 100 lines max (WorkflowEditor, ArtifactBrowser, etc.)
- Store files: 150 lines max
- Service files: 200 lines max
- Repository implementations: 250 lines max

### TDD Requirements
All tasks require tests written before implementation:
- Unit tests for domain entities and services
- Repository tests with in-memory implementations
- Component tests with React Testing Library
- Integration tests for methodology switching

### Anti-AI-Slop Guardrails
- No purple gradients or generic AI aesthetics
- Warm orange accent (#ff6b35) as primary color
- Workflows visualized with clear column boundaries
- Artifacts displayed as cards with type badges
- Research progress shown with iteration counts, not vague "loading"

---

## Task List

```json
[
  {
    "category": "setup",
    "description": "Create extensibility database migrations",
    "steps": [
      "Write unit tests for migration SQL syntax validation",
      "Create migration file: 011_extensibility_workflows.sql with workflows table",
      "Create migration file: 012_extensibility_artifacts.sql with artifacts, artifact_buckets, artifact_relations tables",
      "Create migration file: 013_extensibility_artifact_flows.sql with artifact_flows table",
      "Create migration file: 014_extensibility_processes.sql with processes table",
      "Create migration file: 015_extensibility_task_extensions.sql with ALTER TABLE for task fields",
      "Create migration file: 016_extensibility_task_dependencies.sql with task_dependencies table",
      "Create migration file: 017_extensibility_methodologies.sql with methodology_extensions table",
      "Create migration file: 018_extensibility_indexes.sql with all indexes",
      "Run cargo test to verify migrations apply correctly"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement WorkflowSchema and WorkflowColumn Rust types",
    "steps": [
      "Write unit tests for WorkflowSchema serialization/deserialization",
      "Create src-tauri/src/domain/entities/workflow.rs",
      "Implement WorkflowSchema struct with serde derives",
      "Implement WorkflowColumn struct with behavior field",
      "Implement ExternalSyncConfig struct (placeholder for future)",
      "Add From/Into traits for JSON conversion",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement WorkflowRepository trait",
    "steps": [
      "Write unit tests for repository methods using mock",
      "Create src-tauri/src/domain/repositories/workflow_repo.rs",
      "Define WorkflowRepository trait with async methods",
      "Methods: create, get_by_id, get_all, get_default, update, delete, set_default",
      "Export from domain/repositories/mod.rs",
      "Run cargo test to verify trait compiles"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteWorkflowRepository",
    "steps": [
      "Write integration tests using test database",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_workflow_repo.rs",
      "Implement all WorkflowRepository methods",
      "Handle JSON serialization of schema_json column",
      "Export from infrastructure/sqlite/mod.rs",
      "Run cargo test to verify CRUD operations"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement MemoryWorkflowRepository",
    "steps": [
      "Write unit tests for in-memory operations",
      "Create src-tauri/src/infrastructure/memory/memory_workflow_repo.rs",
      "Implement all WorkflowRepository methods with HashMap storage",
      "Export from infrastructure/memory/mod.rs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Seed built-in workflows (Default, Jira)",
    "steps": [
      "Write unit tests for workflow seeding",
      "Create built-in workflow definitions in domain/entities/workflow.rs",
      "Add seed_builtin_workflows function to SqliteWorkflowRepository",
      "Call seeding on database initialization",
      "Run cargo test to verify workflows are seeded"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Artifact and ArtifactBucket Rust types",
    "steps": [
      "Write unit tests for Artifact serialization",
      "Create src-tauri/src/domain/entities/artifact.rs",
      "Implement ArtifactType enum with all 15 types",
      "Implement Artifact struct with content variants",
      "Implement ArtifactBucket struct",
      "Implement ArtifactRelation struct",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ArtifactRepository trait",
    "steps": [
      "Write unit tests for repository methods using mock",
      "Create src-tauri/src/domain/repositories/artifact_repo.rs",
      "Define ArtifactRepository trait with async methods",
      "Methods: create, get_by_id, get_by_bucket, get_by_type, get_by_task, update, delete",
      "Methods: get_derived_from, get_related, add_relation",
      "Export from domain/repositories/mod.rs",
      "Run cargo test to verify trait compiles"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ArtifactBucketRepository trait",
    "steps": [
      "Write unit tests for bucket repository methods",
      "Create src-tauri/src/domain/repositories/artifact_bucket_repo.rs",
      "Define ArtifactBucketRepository trait",
      "Methods: create, get_by_id, get_all, get_system_buckets, update, delete",
      "Export from domain/repositories/mod.rs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteArtifactRepository",
    "steps": [
      "Write integration tests using test database",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_artifact_repo.rs",
      "Implement all ArtifactRepository methods",
      "Handle content_type (inline vs file) correctly",
      "Handle artifact_relations table operations",
      "Export from infrastructure/sqlite/mod.rs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteArtifactBucketRepository",
    "steps": [
      "Write integration tests using test database",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_artifact_bucket_repo.rs",
      "Implement all ArtifactBucketRepository methods",
      "Handle config_json serialization",
      "Export from infrastructure/sqlite/mod.rs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Seed built-in artifact buckets",
    "steps": [
      "Write unit tests for bucket seeding",
      "Define 4 system buckets: research-outputs, work-context, code-changes, prd-library",
      "Add seed_system_buckets function to SqliteArtifactBucketRepository",
      "Call seeding on database initialization",
      "Run cargo test to verify buckets are seeded"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ArtifactFlow and ArtifactFlowEngine Rust types",
    "steps": [
      "Write unit tests for ArtifactFlow serialization",
      "Create src-tauri/src/domain/entities/artifact_flow.rs",
      "Implement ArtifactFlow struct with trigger and steps",
      "Implement ArtifactFlowStep enum (copy, spawn_process)",
      "Implement ArtifactFlowEngine with evaluate_triggers method",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ArtifactFlowRepository trait and SQLite implementation",
    "steps": [
      "Write unit tests for flow repository methods",
      "Create src-tauri/src/domain/repositories/artifact_flow_repo.rs",
      "Define ArtifactFlowRepository trait: create, get_by_id, get_active, update, delete",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_artifact_flow_repo.rs",
      "Implement SQLite repository",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ResearchProcess and ResearchDepthPreset Rust types",
    "steps": [
      "Write unit tests for ResearchProcess serialization",
      "Create src-tauri/src/domain/entities/research.rs",
      "Implement ResearchDepthPreset enum with 4 presets",
      "Implement CustomDepth struct",
      "Implement ResearchProcess struct with progress tracking",
      "Implement RESEARCH_PRESETS constant with default values",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ProcessRepository trait and SQLite implementation",
    "steps": [
      "Write unit tests for process repository methods",
      "Create src-tauri/src/domain/repositories/process_repo.rs",
      "Define ProcessRepository trait: create, get_by_id, get_by_status, update_progress, complete, fail",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_process_repo.rs",
      "Implement SQLite repository",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement MethodologyExtension Rust types",
    "steps": [
      "Write unit tests for MethodologyExtension serialization",
      "Create src-tauri/src/domain/entities/methodology.rs",
      "Implement MethodologyExtension struct",
      "Implement MethodologyPhase struct for phase definitions",
      "Implement MethodologyTemplate struct for document templates",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement MethodologyRepository trait and SQLite implementation",
    "steps": [
      "Write unit tests for methodology repository methods",
      "Create src-tauri/src/domain/repositories/methodology_repo.rs",
      "Define MethodologyRepository trait: create, get_by_id, get_all, get_active, activate, deactivate, delete",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_methodology_repo.rs",
      "Implement SQLite repository",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Seed built-in methodologies (BMAD, GSD)",
    "steps": [
      "Write unit tests for methodology seeding",
      "Create BMAD methodology definition with workflow and phases",
      "Create GSD methodology definition with workflow and phases",
      "Add seed_builtin_methodologies function",
      "Call seeding on database initialization",
      "Run cargo test to verify methodologies are seeded"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement WorkflowService",
    "steps": [
      "Write unit tests for workflow service methods",
      "Create src-tauri/src/domain/services/workflow_service.rs",
      "Implement WorkflowService with repository dependency",
      "Methods: get_active_workflow, apply_workflow, validate_column_mappings",
      "Handle dynamic Kanban column generation from WorkflowSchema",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ArtifactService",
    "steps": [
      "Write unit tests for artifact service methods",
      "Create src-tauri/src/domain/services/artifact_service.rs",
      "Implement ArtifactService with repository dependencies",
      "Methods: create_artifact, get_artifacts_for_task, copy_to_bucket, version_artifact",
      "Handle content storage (inline vs file)",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ArtifactFlowService",
    "steps": [
      "Write unit tests for flow service methods",
      "Create src-tauri/src/domain/services/artifact_flow_service.rs",
      "Implement ArtifactFlowService with flow engine",
      "Methods: on_artifact_created, on_task_completed, evaluate_flows, execute_steps",
      "Integrate with event system for automatic triggering",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ResearchService",
    "steps": [
      "Write unit tests for research service methods",
      "Create src-tauri/src/domain/services/research_service.rs",
      "Implement ResearchService with process repository",
      "Methods: start_research, pause_research, resume_research, checkpoint, complete",
      "Handle preset-to-config conversion",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement MethodologyService",
    "steps": [
      "Write unit tests for methodology service methods",
      "Create src-tauri/src/domain/services/methodology_service.rs",
      "Implement MethodologyService with all repository dependencies",
      "Methods: activate_methodology, deactivate_methodology, get_active",
      "Handle workflow switching, agent profile loading, skill injection",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Update AppState with extensibility repositories",
    "steps": [
      "Update AppState struct with workflow_repo, artifact_repo, artifact_bucket_repo",
      "Add artifact_flow_repo, process_repo, methodology_repo to AppState",
      "Update app initialization to create all repositories",
      "Run cargo test to verify AppState initialization"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for workflows",
    "steps": [
      "Write integration tests for Tauri commands",
      "Create commands: get_workflows, get_workflow, create_workflow, update_workflow, delete_workflow",
      "Create command: set_default_workflow",
      "Create command: get_active_workflow_columns (returns columns for current workflow)",
      "Register commands in main.rs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for artifacts",
    "steps": [
      "Write integration tests for Tauri commands",
      "Create commands: get_artifacts, get_artifact, create_artifact, update_artifact, delete_artifact",
      "Create commands: get_artifacts_by_bucket, get_artifacts_by_task",
      "Create commands: get_buckets, create_bucket",
      "Create commands: add_artifact_relation, get_artifact_relations",
      "Register commands in main.rs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for research processes",
    "steps": [
      "Write integration tests for Tauri commands",
      "Create commands: start_research, pause_research, resume_research, stop_research",
      "Create commands: get_research_processes, get_research_process",
      "Create command: get_research_presets (returns available depth presets)",
      "Register commands in main.rs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for methodologies",
    "steps": [
      "Write integration tests for Tauri commands",
      "Create commands: get_methodologies, get_active_methodology",
      "Create commands: activate_methodology, deactivate_methodology",
      "Register commands in main.rs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TypeScript types for workflows with Zod schemas",
    "steps": [
      "Write unit tests for Zod schema parsing",
      "Create src/types/workflow.ts",
      "Define WorkflowSchema and WorkflowColumn types",
      "Define WorkflowBehavior and ExternalSyncConfig types",
      "Create Zod schemas for runtime validation",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TypeScript types for artifacts with Zod schemas",
    "steps": [
      "Write unit tests for Zod schema parsing",
      "Create src/types/artifact.ts",
      "Define ArtifactType union and Artifact type",
      "Define ArtifactBucket and ArtifactRelation types",
      "Define ArtifactFlow and ArtifactFlowStep types",
      "Create Zod schemas for runtime validation",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TypeScript types for research with Zod schemas",
    "steps": [
      "Write unit tests for Zod schema parsing",
      "Create src/types/research.ts",
      "Define ResearchProcess and ResearchDepthPreset types",
      "Define CustomDepth and ResearchProgress types",
      "Create Zod schemas for runtime validation",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TypeScript types for methodologies with Zod schemas",
    "steps": [
      "Write unit tests for Zod schema parsing",
      "Create src/types/methodology.ts",
      "Define MethodologyExtension type with phases, templates, hooks",
      "Create Zod schemas for runtime validation",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrappers for workflows",
    "steps": [
      "Write unit tests for API wrapper functions",
      "Create src/lib/api/workflows.ts",
      "Implement: getWorkflows, getWorkflow, createWorkflow, updateWorkflow, deleteWorkflow",
      "Implement: setDefaultWorkflow, getActiveWorkflowColumns",
      "Add Zod validation on responses",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrappers for artifacts",
    "steps": [
      "Write unit tests for API wrapper functions",
      "Create src/lib/api/artifacts.ts",
      "Implement: getArtifacts, getArtifact, createArtifact, updateArtifact, deleteArtifact",
      "Implement: getArtifactsByBucket, getArtifactsByTask",
      "Implement: getBuckets, createBucket",
      "Implement: addArtifactRelation, getArtifactRelations",
      "Add Zod validation on responses",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrappers for research",
    "steps": [
      "Write unit tests for API wrapper functions",
      "Create src/lib/api/research.ts",
      "Implement: startResearch, pauseResearch, resumeResearch, stopResearch",
      "Implement: getResearchProcesses, getResearchProcess, getResearchPresets",
      "Add Zod validation on responses",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrappers for methodologies",
    "steps": [
      "Write unit tests for API wrapper functions",
      "Create src/lib/api/methodologies.ts",
      "Implement: getMethodologies, getActiveMethodology",
      "Implement: activateMethodology, deactivateMethodology",
      "Add Zod validation on responses",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement workflowStore with Zustand",
    "steps": [
      "Write unit tests for store actions",
      "Create src/stores/workflowStore.ts",
      "Define WorkflowStore interface with state and actions",
      "Implement: setActiveWorkflow, addWorkflow, updateWorkflow, deleteWorkflow",
      "Integrate with TanStack Query for data fetching",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement artifactStore with Zustand",
    "steps": [
      "Write unit tests for store actions",
      "Create src/stores/artifactStore.ts",
      "Define ArtifactStore interface with buckets and artifacts",
      "Implement: setSelectedBucket, setSelectedArtifact, addArtifact",
      "Integrate with TanStack Query for data fetching",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement methodologyStore with Zustand",
    "steps": [
      "Write unit tests for store actions",
      "Create src/stores/methodologyStore.ts",
      "Define MethodologyStore interface",
      "Implement: activateMethodology, deactivateMethodology",
      "Handle workflow and agent profile switching",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useWorkflows hook with TanStack Query",
    "steps": [
      "Write unit tests for hook behavior",
      "Create src/hooks/useWorkflows.ts",
      "Implement useWorkflows query hook",
      "Implement useWorkflow(id) query hook",
      "Implement useWorkflowMutations for CRUD operations",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useArtifacts hooks with TanStack Query",
    "steps": [
      "Write unit tests for hook behavior",
      "Create src/hooks/useArtifacts.ts",
      "Implement useArtifacts query hook with bucket filter",
      "Implement useArtifact(id) query hook",
      "Implement useBuckets query hook",
      "Implement useArtifactMutations for CRUD operations",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useResearch hooks with TanStack Query",
    "steps": [
      "Write unit tests for hook behavior",
      "Create src/hooks/useResearch.ts",
      "Implement useResearchProcesses query hook",
      "Implement useResearchProcess(id) query hook",
      "Implement useResearchPresets query hook",
      "Implement useResearchMutations for start/pause/resume/stop",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useMethodologies hook with TanStack Query",
    "steps": [
      "Write unit tests for hook behavior",
      "Create src/hooks/useMethodologies.ts",
      "Implement useMethodologies query hook",
      "Implement useActiveMethodology query hook",
      "Implement useMethodologyMutations for activate/deactivate",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create WorkflowSelector component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/workflows/WorkflowSelector.tsx",
      "Implement dropdown with workflow list",
      "Show current workflow as selected",
      "Handle workflow selection change",
      "Apply anti-AI-slop styling (warm orange accent)",
      "Keep under 100 lines",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create WorkflowEditor component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/workflows/WorkflowEditor.tsx",
      "Implement form for creating/editing WorkflowSchema",
      "Allow adding/removing columns with drag-drop reorder",
      "Column config: name, color, icon, mapsTo (internal status dropdown)",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines (split into sub-components if needed)",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ArtifactCard component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/artifacts/ArtifactCard.tsx",
      "Display artifact name, type badge, created timestamp",
      "Show version number if > 1",
      "Handle click for selection",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ArtifactBrowser component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/artifacts/ArtifactBrowser.tsx",
      "Implement bucket sidebar with artifact list",
      "Filter by bucket selection",
      "Filter by artifact type (optional)",
      "Display ArtifactCards in main area",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines (split into sub-components if needed)",
      "Run npm test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ArtifactFlow component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/artifacts/ArtifactFlow.tsx",
      "Visualize artifact flow triggers and steps",
      "Show connections between buckets/processes",
      "Simple diagram (no fancy visualization library needed)",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines",
      "Run npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create ResearchLauncher component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/research/ResearchLauncher.tsx",
      "Implement form for starting research process",
      "Fields: question, context, scope, constraints (optional)",
      "Depth preset selector (quick-scan, standard, deep-dive, exhaustive)",
      "Custom depth option with iteration/timeout inputs",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines",
      "Run npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create ResearchProgress component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/research/ResearchProgress.tsx",
      "Display research process name, status, iteration count",
      "Show progress bar (currentIteration / maxIterations)",
      "Pause/Resume/Stop buttons",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines",
      "Run npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create ResearchResults component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/research/ResearchResults.tsx",
      "Display artifacts produced by research process",
      "Link to artifact browser for each output",
      "Show summary of findings/recommendations",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines",
      "Run npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create MethodologyBrowser component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/methodologies/MethodologyBrowser.tsx",
      "Display list of available methodologies",
      "Show active methodology badge",
      "Methodology card: name, description, phase count, agent count",
      "Activate/Deactivate buttons",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines",
      "Run npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create MethodologyConfig component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create src/components/methodologies/MethodologyConfig.tsx",
      "Display active methodology details",
      "Show workflow columns with color chips",
      "Show phase progression diagram",
      "List agent profiles with roles",
      "Apply anti-AI-slop styling",
      "Keep under 100 lines",
      "Run npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integrate WorkflowSelector with TaskBoard header",
    "steps": [
      "Write integration tests for workflow switching",
      "Add WorkflowSelector to TaskBoard header area",
      "When workflow changes, re-render columns from new WorkflowSchema",
      "Preserve task data, only column mapping changes",
      "Run npm test and npm run typecheck to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Create ExtensibilityView for settings/configuration",
    "steps": [
      "Write component tests for ExtensibilityView",
      "Create src/components/ExtensibilityView.tsx",
      "Tab layout: Workflows | Artifacts | Research | Methodologies",
      "Each tab renders respective browser/editor components",
      "Add navigation to App layout",
      "Run npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integrate methodology activation with app state",
    "steps": [
      "Write integration tests for methodology activation flow",
      "When methodology activates: update workflowStore with methodology workflow",
      "Reload Kanban columns from new workflow",
      "Update available agent profiles in agent store",
      "Show toast notification on successful activation",
      "Run npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: Workflow CRUD and column rendering",
    "steps": [
      "Create integration test file for workflow operations",
      "Test: Create custom workflow with 5 columns",
      "Test: Set as default workflow",
      "Test: Verify TaskBoard renders correct columns",
      "Test: Delete workflow and verify fallback to default",
      "Run cargo test and npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: Artifact creation and bucket routing",
    "steps": [
      "Create integration test file for artifact operations",
      "Test: Create artifact in research-outputs bucket",
      "Test: Copy artifact to prd-library bucket",
      "Test: Create artifact relation (derived_from)",
      "Test: Query artifacts by bucket and type",
      "Run cargo test and npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: Research process lifecycle",
    "steps": [
      "Create integration test file for research processes",
      "Test: Start research with quick-scan preset",
      "Test: Pause and resume research",
      "Test: Checkpoint saves progress",
      "Test: Complete research creates output artifacts",
      "Run cargo test and npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: Methodology activation and deactivation",
    "steps": [
      "Create integration test file for methodology switching",
      "Test: Activate BMAD methodology",
      "Test: Verify workflow columns match BMAD definition",
      "Test: Verify agent profiles loaded",
      "Test: Deactivate methodology returns to default",
      "Run cargo test and npm test to verify"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: GSD-specific task fields (wave, checkpoint)",
    "steps": [
      "Create integration test file for GSD features",
      "Test: Activate GSD methodology",
      "Test: Create task with wave=1 and checkpoint_type=human-verify",
      "Test: Query tasks by wave for parallel execution",
      "Test: Checkpoint transitions task to blocked status",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Visual verification of extensibility UI components",
    "steps": [
      "Start application with npm run tauri dev",
      "Navigate to ExtensibilityView",
      "Capture screenshot of Workflows tab",
      "Capture screenshot of Artifacts tab with sample artifacts",
      "Capture screenshot of Research tab with progress",
      "Capture screenshot of Methodologies tab with BMAD active",
      "Verify anti-AI-slop styling (warm orange, no purple gradients)",
      "Save screenshots to screenshots/phase_11_*.png"
    ],
    "passes": false
  }
]
```
