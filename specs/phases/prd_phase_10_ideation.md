# RalphX - Phase 10: Ideation System

## Overview

Phase 10 implements the Ideation System - a dedicated brainstorming environment where users converse with an Orchestrator to generate Task Proposals before committing them to the Kanban board. This separates ideation from execution, reducing friction and enabling AI-assisted task decomposition with automatic priority calculation.

**Key insight**: Ideation and execution are fundamentally different activities. Mixing them creates friction. RalphX separates them with a two-stage commitment process: Ideas → Proposals → Tasks.

## Dependencies

- **Phase 2 (Data Layer)**: Repository pattern and SQLite infrastructure
- **Phase 4 (Agentic Client)**: For Orchestrator agent communication
- **Phase 5 (Frontend Core)**: Zustand stores, TanStack Query, event system
- **Phase 6 (Kanban UI)**: TaskBoard for receiving applied proposals
- **Phase 7 (Agent System)**: RalphX plugin structure for Orchestrator agent

## Scope

### Included
- Chat interface (contextual side panel, toggle with ⌘+K)
- Ideation View with split layout (conversation + proposals)
- Ideation Sessions (active, archived, converted)
- Task Proposals with priority assessment
- Priority calculation algorithm (5-factor, 0-100 scoring)
- Dependency graph analysis
- Apply proposals to Kanban (Draft, Backlog, Todo columns)
- Orchestrator agent with 11 ideation-specific tools
- Database schema (5 tables: sessions, proposals, dependencies, messages, task_dependencies)

### Excluded
- Deep research loops (Phase 11)
- Custom workflow schemas (Phase 11)
- Artifact system (Phase 11)

---

## Detailed Requirements

### Chat Interface

The chat is implemented as a **contextual side panel** that's always accessible:

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│ RalphX                                                        [⌘K] Toggle Chat     │
├───────────────┬─────────────────────────────────────────────────────┬───────────────┤
│               │                                                     │               │
│  PROJECT NAV  │              MAIN VIEW AREA                         │  CHAT PANEL   │
│               │                                                     │  (Resizable)  │
│  ┌─────────┐  │   ┌─────────────────────────────────────────────┐   │               │
│  │Ideation │  │   │                                             │   │  ┌─────────┐  │
│  └─────────┘  │   │  Current View Content                       │   │  │ Context │  │
│  ┌─────────┐  │   │  (Kanban / Ideation / Settings / etc.)     │   │  │ Aware   │  │
│  │ Kanban  │  │   │                                             │   │  │         │  │
│  └─────────┘  │   │                                             │   │  │ Chat    │  │
│  ┌─────────┐  │   │                                             │   │  │ History │  │
│  │Activity │  │   │                                             │   │  └─────────┘  │
│  └─────────┘  │   │                                             │   │               │
│  ┌─────────┐  │   │                                             │   │  ┌─────────┐  │
│  │Settings │  │   │                                             │   │  │ Input   │  │
│  └─────────┘  │   └─────────────────────────────────────────────┘   │  └─────────┘  │
│               │                                                     │               │
└───────────────┴─────────────────────────────────────────────────────┴───────────────┘
```

**Chat Panel Behaviors:**
- **Toggle**: ⌘+K (or ⌘+J) to show/hide
- **Resizable**: Drag edge to adjust width (min 280px, max 50% of window)
- **Persistent**: Conversation history maintained across view changes
- **Context-aware**: Knows current view and selected items

### Context Awareness

The chat panel adapts based on current context:

| Current Context | Chat Behavior |
|-----------------|---------------|
| **Kanban View** (nothing selected) | General project chat, can suggest tasks |
| **Kanban View** (task selected) | Chat about selected task, can modify it |
| **Ideation View** | Full ideation mode, generates proposals |
| **Task Detail Modal** | Focused on that specific task |
| **Settings** | Can help configure settings |

```typescript
interface ChatContext {
  view: "kanban" | "ideation" | "activity" | "settings" | "task_detail";
  projectId: string;
  selectedTaskId?: string;
  selectedProposalIds?: string[];
  ideationSessionId?: string;
}
```

---

### Ideation View

A dedicated space for brainstorming that produces **Task Proposals** (not real tasks).

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│  Ideation: MyProject                                    [New Session] [Archive]     │
├───────────────────────────────────────────────┬─────────────────────────────────────┤
│  CONVERSATION                                 │  TASK PROPOSALS                      │
│                                               │                                      │
│  ┌───────────────────────────────────────┐   │  ┌────────────────────────────────┐  │
│  │ You: I need user authentication       │   │  │ ☑ 1. Setup auth database       │  │
│  └───────────────────────────────────────┘   │  │    Priority: HIGH (blocks 2)   │  │
│                                               │  │    Category: setup             │  │
│  ┌───────────────────────────────────────┐   │  │    [Edit] [Remove]             │  │
│  │ Orchestrator: I'll help design that.  │   │  └────────────────────────────────┘  │
│  │ Based on the tech stack (React +      │   │                                      │
│  │ Tauri), I suggest these approaches... │   │  ┌────────────────────────────────┐  │
│  └───────────────────────────────────────┘   │  │ ☑ 2. Implement JWT service     │  │
│                                               │  │    Priority: HIGH              │  │
│  ┌───────────────────────────────────────┐   │  │    Depends on: #1              │  │
│  │ You: Use JWT, not sessions            │   │  │    [Edit] [Remove]             │  │
│  └───────────────────────────────────────┘   │  └────────────────────────────────┘  │
│                                               │                                      │
│  ┌───────────────────────────────────────┐   │  ┌────────────────────────────────┐  │
│  │ Orchestrator: Updated. JWT is a good  │   │  │ ☑ 3. Create login UI           │  │
│  │ choice for Tauri. Here are the        │   │  │    Priority: MEDIUM            │  │
│  │ proposed tasks with dependencies...   │   │  │    Depends on: #2              │  │
│  │                                        │   │  │    [Edit] [Remove]             │  │
│  └───────────────────────────────────────┘   │  └────────────────────────────────┘  │
│                                               │                                      │
│                                               │  ─────────────────────────────────   │
│                                               │  Selected: 3 of 4                    │
│                                               │                                      │
│                                               │  [Apply to Draft ▼] [Clear All]     │
├───────────────────────────────────────────────┴─────────────────────────────────────┤
│  [Send message...]                                                     [Attach ▼]  │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

### Ideation Sessions

Each ideation conversation is a **session**:

```typescript
interface IdeationSession {
  id: string;
  projectId: string;
  title: string;                    // Auto-generated or user-defined
  status: "active" | "archived" | "converted";
  messages: ChatMessage[];
  proposals: TaskProposal[];
  createdAt: Date;
  updatedAt: Date;
  archivedAt?: Date;
  convertedAt?: Date;
}
```

**Session Statuses:**
- **active**: Currently being worked on
- **archived**: Completed or paused for later
- **converted**: All proposals applied to Kanban

---

### Task Proposals

Proposals are **draft tasks** that exist only within ideation until applied:

```typescript
interface TaskProposal {
  id: string;
  sessionId: string;

  // Core fields
  title: string;
  description: string;
  category: "setup" | "feature" | "integration" | "styling" | "testing" | "documentation";

  // Steps (like PRD tasks)
  steps?: string[];
  acceptanceCriteria?: string[];

  // Priority assessment (auto-calculated)
  suggestedPriority: Priority;
  priorityScore: number;           // 0-100 for sorting
  priorityReason: string;          // Human-readable explanation

  // Dependencies (references other proposals in same session)
  dependsOn: string[];             // Proposal IDs this depends on
  blocks: string[];                // Proposal IDs this would unblock

  // Complexity estimate
  estimatedComplexity: "trivial" | "simple" | "moderate" | "complex" | "very_complex";

  // User modifications
  userPriority?: Priority;         // Override if user disagrees
  userModified: boolean;           // True if user edited any field

  // Status
  status: "pending" | "accepted" | "rejected" | "modified";
  selected: boolean;               // Checkbox state in UI

  // Link to created task (after apply)
  createdTaskId?: string;

  createdAt: Date;
  updatedAt: Date;
}

type Priority = "critical" | "high" | "medium" | "low";
```

---

### Apply Proposals to Kanban

When user clicks "Apply", selected proposals become real tasks:

```typescript
interface ApplyProposalsOptions {
  proposalIds: string[];
  targetColumn: "draft" | "backlog" | "todo";
  preserveDependencies: boolean;    // Create task_dependencies records
  assignWave?: number;              // For parallel execution grouping
}

interface ApplyProposalsResult {
  createdTasks: Task[];
  dependenciesCreated: number;
  warnings?: string[];              // e.g., "Circular dependency detected"
}
```

**Apply Options:**
- **Apply to Draft**: Tasks go to Draft column (needs more refinement)
- **Apply to Backlog**: Tasks go to Backlog (confirmed, not scheduled)
- **Apply to Todo**: Tasks go to Todo (ready to be planned)
- **Preserve Dependencies**: Creates task_dependency records from proposal_dependencies

---

### Priority Assessment System

The Orchestrator calculates priority using 5 factors:

```typescript
interface PriorityAssessment {
  proposalId: string;

  // Final results
  suggestedPriority: Priority;
  priorityScore: number;           // 0-100
  priorityReason: string;

  // Factor breakdown
  factors: {
    dependencyFactor: {
      score: number;               // 0-30 points
      blocksCount: number;
      reason: string;              // "Blocks 3 other tasks"
    };
    criticalPathFactor: {
      score: number;               // 0-25 points
      isOnCriticalPath: boolean;
      pathLength: number;
      reason: string;
    };
    businessValueFactor: {
      score: number;               // 0-20 points
      keywords: string[];          // ["MVP", "core", "essential"]
      reason: string;
    };
    complexityFactor: {
      score: number;               // 0-15 points (inverse - simpler = higher)
      complexity: string;
      reason: string;              // "Quick win - simple task"
    };
    userHintFactor: {
      score: number;               // 0-10 points
      hints: string[];             // ["urgent", "blocker", "ASAP"]
      reason: string;
    };
  };
}
```

**Priority Scoring Breakdown:**

| Factor | Max Points | Description |
|--------|------------|-------------|
| **Dependency** | 30 | Tasks that unblock others get higher priority |
| **Critical Path** | 25 | Tasks on the longest path to completion |
| **Business Value** | 20 | Keywords from conversation indicating importance |
| **Complexity** | 15 | Simpler tasks scored higher (quick wins first) |
| **User Hints** | 10 | Explicit urgency signals from user |
| **Total** | 100 | |

**Score to Priority Mapping:**

| Score Range | Priority |
|-------------|----------|
| 80-100 | Critical |
| 60-79 | High |
| 40-59 | Medium |
| 0-39 | Low |

**Priority Keywords Detection:**

```typescript
const PRIORITY_KEYWORDS = {
  critical: ["critical", "blocker", "blocking", "urgent", "ASAP", "emergency", "must have"],
  high: ["important", "priority", "essential", "core", "MVP", "key", "crucial"],
  low: ["nice to have", "optional", "future", "later", "eventually", "if time"],
};
```

---

### Dependency Analysis

```typescript
interface DependencyGraph {
  nodes: {
    proposalId: string;
    title: string;
    inDegree: number;              // Number of dependencies
    outDegree: number;             // Number of tasks this blocks
  }[];
  edges: {
    from: string;                  // Depends on
    to: string;                    // Is dependency of
  }[];
  criticalPath: string[];          // Ordered list of proposal IDs
  hasCycles: boolean;
  cycles?: string[][];             // If cycles detected, list them
}
```

---

### Orchestrator Tools for Ideation (11 tools)

**Session Management:**
1. `create_ideation_session` - Start a new ideation session
2. `get_ideation_session` - Get current session with all proposals

**Proposal CRUD:**
3. `create_task_proposal` - Create a new task proposal
4. `update_task_proposal` - Update an existing proposal
5. `delete_task_proposal` - Remove a proposal from session

**Priority & Dependency Analysis:**
6. `assess_priority` - Calculate priority for a single proposal
7. `assess_all_priorities` - Recalculate all priorities in session
8. `analyze_dependencies` - Build dependency graph
9. `suggest_dependencies` - AI suggests likely dependencies

**Apply to Kanban:**
10. `apply_proposals_to_kanban` - Convert proposals to real tasks

**Context Retrieval:**
11. `get_project_context` - Get project info, tech stack, existing tasks
12. `get_existing_tasks` - Get existing tasks for context

---

### Orchestrator Agent Definition

Located at `.claude/agents/orchestrator-ideation.md`:

```markdown
---
name: orchestrator-ideation
description: Facilitates ideation sessions and generates task proposals
tools:
  - create_ideation_session
  - get_ideation_session
  - create_task_proposal
  - update_task_proposal
  - delete_task_proposal
  - assess_priority
  - assess_all_priorities
  - analyze_dependencies
  - suggest_dependencies
  - apply_proposals_to_kanban
  - get_project_context
  - get_existing_tasks
model: sonnet
---

You are the Ideation Orchestrator for RalphX...
```

**Agent Workflow Phases:**
1. **Understand**: Ask clarifying questions, get context
2. **Decompose**: Break features into atomic tasks
3. **Organize**: Identify dependencies, calculate priorities
4. **Present**: Show proposals, explain reasoning, allow modifications

---

### Database Schema

**5 Tables:**

```sql
-- IDEATION SESSIONS
CREATE TABLE ideation_sessions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title TEXT,
  status TEXT NOT NULL DEFAULT 'active',  -- 'active' | 'archived' | 'converted'
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  archived_at DATETIME,
  converted_at DATETIME
);

-- TASK PROPOSALS
CREATE TABLE task_proposals (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  description TEXT,
  category TEXT NOT NULL,
  steps TEXT,                              -- JSON array
  acceptance_criteria TEXT,                -- JSON array
  suggested_priority TEXT NOT NULL,
  priority_score INTEGER NOT NULL DEFAULT 50,
  priority_reason TEXT,
  priority_factors TEXT,                   -- JSON
  estimated_complexity TEXT DEFAULT 'moderate',
  user_priority TEXT,
  user_modified BOOLEAN DEFAULT FALSE,
  status TEXT NOT NULL DEFAULT 'pending',
  selected BOOLEAN DEFAULT TRUE,
  created_task_id TEXT REFERENCES tasks(id),
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- PROPOSAL DEPENDENCIES
CREATE TABLE proposal_dependencies (
  id TEXT PRIMARY KEY,
  proposal_id TEXT NOT NULL REFERENCES task_proposals(id) ON DELETE CASCADE,
  depends_on_proposal_id TEXT NOT NULL REFERENCES task_proposals(id) ON DELETE CASCADE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(proposal_id, depends_on_proposal_id),
  CHECK(proposal_id != depends_on_proposal_id)
);

-- CHAT MESSAGES
CREATE TABLE chat_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT REFERENCES ideation_sessions(id) ON DELETE CASCADE,
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
  role TEXT NOT NULL,                      -- 'user' | 'orchestrator' | 'system'
  content TEXT NOT NULL,
  metadata TEXT,                           -- JSON
  parent_message_id TEXT REFERENCES chat_messages(id),
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- TASK DEPENDENCIES (for applied tasks)
CREATE TABLE IF NOT EXISTS task_dependencies (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  depends_on_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(task_id, depends_on_task_id),
  CHECK(task_id != depends_on_task_id)
);
```

---

### UI Components

**ProposalCard:**
- Checkbox, title, priority badge, category
- Dependency info (depends on count, blocks count)
- Edit and Remove actions
- Visual states: Default, Selected (orange border), High priority (warm color), Modified indicator

**ProposalList:**
- Drag to reorder
- Multi-select with Shift+click
- Bulk actions (select all, deselect all)
- Dependency visualization (lines between cards)
- Priority-based auto-sort option

**ApplyModal:**
- List of selected proposals
- Dependency graph preview
- Target column selector (Draft, Backlog, Todo)
- "Preserve dependencies" checkbox
- Warnings for circular deps and missing deps

**ChatPanel:**
- Toggle with ⌘+K
- Resizable (min 280px, max 50% width)
- Message history
- Context indicator (current view)
- Input with send button

---

### Ideation → Kanban Transition Flow

1. User opens Ideation view → New IdeationSession created (status: active)
2. User converses with Orchestrator → ChatMessages stored
3. Orchestrator creates TaskProposals → Priorities calculated, dependencies inferred
4. User reviews proposals in side panel → Can edit, remove, reorder
5. User selects proposals via checkboxes
6. User clicks "Apply to [Column]" → System validates no circular deps
7. For each selected proposal: Create Task, copy fields, set dependencies
8. Update session status to 'converted' if all applied
9. Tasks appear in target column with blockers, normal Kanban workflow continues

---

## Implementation Notes

### Key Architecture Principles

1. **Ideation ≠ Execution** - Separate brainstorming from task management
2. **Proposals before Tasks** - Two-stage commitment reduces friction
3. **Automatic Priority** - System suggests, user confirms
4. **Context-Aware Chat** - Chat adapts to current view and selection
5. **Dependency-First Planning** - Priority derived from dependency graph

### File Size Limits

- Components: 150 lines max
- Hooks: 100 lines max
- Stores: 150 lines max
- Repository implementations: 200 lines max

### Anti-AI-Slop Guardrails

- No purple gradients
- No Inter font
- Warm orange accent (#ff6b35) for selected states
- Soft amber secondary (#ffa94d)
- Dark surfaces with subtle borders

### TDD Requirements

All tasks require:
1. Write tests FIRST
2. Implement to make tests pass
3. Run linting/type checks

---

## Task List

```json
[
  {
    "category": "setup",
    "description": "Create ideation database migrations",
    "steps": [
      "Write migration test that verifies schema creation",
      "Create migration file for ideation_sessions table with indexes",
      "Create migration file for task_proposals table with indexes",
      "Create migration file for proposal_dependencies table with constraints",
      "Create migration file for chat_messages table with indexes",
      "Create migration file for task_dependencies table (if not exists)",
      "Run migrations and verify with sqlite3 .schema",
      "Test foreign key constraints work correctly"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement IdeationSession Rust domain entity",
    "steps": [
      "Write unit tests for IdeationSession struct serialization",
      "Create IdeationSession struct with all fields (id, project_id, title, status, timestamps)",
      "Implement IdeationSessionStatus enum (Active, Archived, Converted)",
      "Add FromStr and Display traits for SQLite compatibility",
      "Implement builder pattern for IdeationSession creation",
      "Export from domain/entities module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TaskProposal Rust domain entity",
    "steps": [
      "Write unit tests for TaskProposal struct and Priority enum",
      "Create Priority enum (Critical, High, Medium, Low) with FromStr/Display",
      "Create Complexity enum (Trivial, Simple, Moderate, Complex, VeryComplex)",
      "Create ProposalStatus enum (Pending, Accepted, Rejected, Modified)",
      "Create TaskProposal struct with all fields",
      "Create TaskCategory enum with all variants",
      "Implement JSON serialization for steps and acceptance_criteria arrays",
      "Export from domain/entities module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement PriorityAssessment domain types",
    "steps": [
      "Write unit tests for PriorityAssessment and factor structs",
      "Create DependencyFactor struct (score, blocksCount, reason)",
      "Create CriticalPathFactor struct (score, isOnCriticalPath, pathLength, reason)",
      "Create BusinessValueFactor struct (score, keywords, reason)",
      "Create ComplexityFactor struct (score, complexity, reason)",
      "Create UserHintFactor struct (score, hints, reason)",
      "Create PriorityFactors container struct",
      "Create PriorityAssessment struct with all fields",
      "Export from domain/entities module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ChatMessage and DependencyGraph domain types",
    "steps": [
      "Write unit tests for ChatMessage and MessageRole enum",
      "Create MessageRole enum (User, Orchestrator, System)",
      "Create ChatMessage struct with all fields",
      "Create DependencyGraphNode struct (proposalId, title, inDegree, outDegree)",
      "Create DependencyGraphEdge struct (from, to)",
      "Create DependencyGraph struct with cycle detection fields",
      "Export from domain/entities module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement IdeationSessionRepository trait",
    "steps": [
      "Write unit tests with mock repository for all methods",
      "Define IdeationSessionRepository trait with async methods:",
      "  - create(session: IdeationSession) -> Result<IdeationSession>",
      "  - get_by_id(id: &str) -> Result<Option<IdeationSession>>",
      "  - get_by_project(project_id: &str) -> Result<Vec<IdeationSession>>",
      "  - update_status(id: &str, status: IdeationSessionStatus) -> Result<()>",
      "  - delete(id: &str) -> Result<()>",
      "Export from domain/repositories module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TaskProposalRepository trait",
    "steps": [
      "Write unit tests with mock repository for all methods",
      "Define TaskProposalRepository trait with async methods:",
      "  - create(proposal: TaskProposal) -> Result<TaskProposal>",
      "  - get_by_id(id: &str) -> Result<Option<TaskProposal>>",
      "  - get_by_session(session_id: &str) -> Result<Vec<TaskProposal>>",
      "  - update(proposal: TaskProposal) -> Result<TaskProposal>",
      "  - update_priority(id: &str, assessment: PriorityAssessment) -> Result<()>",
      "  - update_selection(id: &str, selected: bool) -> Result<()>",
      "  - set_created_task_id(id: &str, task_id: &str) -> Result<()>",
      "  - delete(id: &str) -> Result<()>",
      "  - reorder(session_id: &str, proposal_ids: Vec<String>) -> Result<()>",
      "Export from domain/repositories module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ProposalDependencyRepository trait",
    "steps": [
      "Write unit tests with mock repository for all methods",
      "Define ProposalDependencyRepository trait with async methods:",
      "  - add_dependency(proposal_id: &str, depends_on_id: &str) -> Result<()>",
      "  - remove_dependency(proposal_id: &str, depends_on_id: &str) -> Result<()>",
      "  - get_dependencies(proposal_id: &str) -> Result<Vec<String>>",
      "  - get_dependents(proposal_id: &str) -> Result<Vec<String>>",
      "  - get_all_for_session(session_id: &str) -> Result<Vec<(String, String)>>",
      "Export from domain/repositories module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ChatMessageRepository trait",
    "steps": [
      "Write unit tests with mock repository for all methods",
      "Define ChatMessageRepository trait with async methods:",
      "  - create(message: ChatMessage) -> Result<ChatMessage>",
      "  - get_by_session(session_id: &str) -> Result<Vec<ChatMessage>>",
      "  - get_by_project(project_id: &str) -> Result<Vec<ChatMessage>>",
      "  - get_by_task(task_id: &str) -> Result<Vec<ChatMessage>>",
      "  - delete_by_session(session_id: &str) -> Result<()>",
      "Export from domain/repositories module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TaskDependencyRepository trait",
    "steps": [
      "Write unit tests with mock repository for all methods",
      "Define TaskDependencyRepository trait with async methods:",
      "  - add_dependency(task_id: &str, depends_on_task_id: &str) -> Result<()>",
      "  - remove_dependency(task_id: &str, depends_on_task_id: &str) -> Result<()>",
      "  - get_blockers(task_id: &str) -> Result<Vec<String>>",
      "  - get_blocked_by(task_id: &str) -> Result<Vec<String>>",
      "  - has_circular_dependency(task_id: &str, potential_dep: &str) -> Result<bool>",
      "Export from domain/repositories module"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteIdeationSessionRepository",
    "steps": [
      "Write integration tests against real SQLite database",
      "Implement create() with INSERT and returning full entity",
      "Implement get_by_id() with SELECT and from_row conversion",
      "Implement get_by_project() ordered by updated_at DESC",
      "Implement update_status() with timestamp updates",
      "Implement delete() with CASCADE to proposals and messages",
      "Run tests with test database"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteTaskProposalRepository",
    "steps": [
      "Write integration tests against real SQLite database",
      "Implement create() with INSERT and JSON serialization for arrays",
      "Implement get_by_id() with from_row conversion",
      "Implement get_by_session() ordered by sort_order",
      "Implement update() preserving timestamps",
      "Implement update_priority() with assessment fields",
      "Implement update_selection() for checkbox state",
      "Implement set_created_task_id() for apply flow",
      "Implement delete() with CASCADE to dependencies",
      "Implement reorder() with UPDATE sort_order"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteProposalDependencyRepository",
    "steps": [
      "Write integration tests against real SQLite database",
      "Implement add_dependency() with UNIQUE constraint handling",
      "Implement remove_dependency()",
      "Implement get_dependencies() - proposals this depends on",
      "Implement get_dependents() - proposals that depend on this",
      "Implement get_all_for_session() joining through proposals table",
      "Verify CHECK constraint prevents self-dependencies"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteChatMessageRepository",
    "steps": [
      "Write integration tests against real SQLite database",
      "Implement create() with INSERT",
      "Implement get_by_session() ordered by created_at",
      "Implement get_by_project() ordered by created_at",
      "Implement get_by_task() ordered by created_at",
      "Implement delete_by_session() for cleanup"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteTaskDependencyRepository",
    "steps": [
      "Write integration tests against real SQLite database",
      "Implement add_dependency() with UNIQUE constraint handling",
      "Implement remove_dependency()",
      "Implement get_blockers() - tasks this depends on",
      "Implement get_blocked_by() - tasks that depend on this",
      "Implement has_circular_dependency() with recursive CTE or DFS",
      "Verify CHECK constraint prevents self-dependencies"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement PriorityService for priority calculation",
    "steps": [
      "Write unit tests for each priority factor calculation",
      "Create PriorityService struct",
      "Implement calculate_dependency_factor() - 0-30 points based on blocks count",
      "Implement calculate_critical_path_factor() - 0-25 points using graph analysis",
      "Implement calculate_business_value_factor() - 0-20 points using keyword detection",
      "Implement calculate_complexity_factor() - 0-15 points (simpler = higher)",
      "Implement calculate_user_hint_factor() - 0-10 points from conversation",
      "Implement assess_priority() combining all factors",
      "Implement assess_all_priorities() for batch processing",
      "Implement score_to_priority() mapping (80-100=Critical, etc.)"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement DependencyService for graph analysis",
    "steps": [
      "Write unit tests for dependency graph building and cycle detection",
      "Create DependencyService struct",
      "Implement build_graph() creating DependencyGraph from proposals",
      "Implement detect_cycles() using Tarjan's or DFS algorithm",
      "Implement find_critical_path() using topological sort + longest path",
      "Implement suggest_dependencies() - AI-based inference (stub for now)",
      "Implement validate_no_cycles() for apply validation"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement IdeationService for orchestrating ideation flow",
    "steps": [
      "Write unit tests for IdeationService methods",
      "Create IdeationService struct with repository dependencies",
      "Implement create_session() with auto-title generation",
      "Implement get_session_with_proposals() joining data",
      "Implement archive_session() with timestamp",
      "Implement create_proposal() with initial priority assessment",
      "Implement update_proposal() preserving user modifications",
      "Implement delete_proposal() updating graph",
      "Implement add_message() for chat history",
      "Implement get_session_messages()"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ApplyService for converting proposals to tasks",
    "steps": [
      "Write unit tests for apply workflow",
      "Create ApplyService struct with repository dependencies",
      "Implement validate_selection() - check no circular deps in selection",
      "Implement apply_proposals() main method:",
      "  - Validate selection",
      "  - Create Task for each proposal (copy fields, map status)",
      "  - Create task_dependencies from proposal_dependencies",
      "  - Update proposal.created_task_id",
      "  - Update proposal.status to 'accepted'",
      "  - Check if session should be 'converted'",
      "Implement map_column_to_status() (draft→Backlog, backlog→Backlog, todo→Ready)",
      "Return ApplyProposalsResult with created tasks and warnings"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Update AppState with ideation repositories",
    "steps": [
      "Add IdeationSessionRepository to AppState",
      "Add TaskProposalRepository to AppState",
      "Add ProposalDependencyRepository to AppState",
      "Add ChatMessageRepository to AppState",
      "Add TaskDependencyRepository to AppState",
      "Update Tauri state initialization",
      "Write test verifying all repos accessible"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for ideation sessions",
    "steps": [
      "Write integration tests for each command",
      "Create create_ideation_session command",
      "Create get_ideation_session command (with proposals and messages)",
      "Create list_ideation_sessions command (by project)",
      "Create archive_ideation_session command",
      "Create delete_ideation_session command",
      "Register commands in Tauri builder"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for task proposals",
    "steps": [
      "Write integration tests for each command",
      "Create create_task_proposal command",
      "Create update_task_proposal command",
      "Create delete_task_proposal command",
      "Create toggle_proposal_selection command",
      "Create reorder_proposals command",
      "Create assess_proposal_priority command",
      "Create assess_all_priorities command",
      "Register commands in Tauri builder"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for dependencies and apply",
    "steps": [
      "Write integration tests for each command",
      "Create add_proposal_dependency command",
      "Create remove_proposal_dependency command",
      "Create analyze_dependencies command (returns DependencyGraph)",
      "Create apply_proposals_to_kanban command",
      "Create get_task_blockers command",
      "Register commands in Tauri builder"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for chat messages",
    "steps": [
      "Write integration tests for each command",
      "Create send_chat_message command",
      "Create get_session_messages command",
      "Create get_project_messages command",
      "Create get_task_messages command",
      "Register commands in Tauri builder"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TypeScript types for ideation system",
    "steps": [
      "Write unit tests for Zod schema validation",
      "Create IdeationSession type and Zod schema",
      "Create IdeationSessionStatus type ('active' | 'archived' | 'converted')",
      "Create TaskProposal type and Zod schema",
      "Create Priority type ('critical' | 'high' | 'medium' | 'low')",
      "Create Complexity type ('trivial' | ... | 'very_complex')",
      "Create ProposalStatus type ('pending' | 'accepted' | 'rejected' | 'modified')",
      "Create TaskCategory type (6 variants)",
      "Create PriorityAssessment type with all factor types",
      "Create DependencyGraph type",
      "Create ChatMessage type with MessageRole",
      "Create ApplyProposalsOptions and ApplyProposalsResult types",
      "Export from src/types/ideation.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TypeScript types for chat context",
    "steps": [
      "Write unit tests for Zod schema validation",
      "Create ChatContext type with view discriminator",
      "Create ViewType type ('kanban' | 'ideation' | 'activity' | 'settings' | 'task_detail')",
      "Export from src/types/chat.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrappers for ideation",
    "steps": [
      "Write unit tests for API wrapper functions",
      "Create ideationApi.ts with type-safe invoke wrappers:",
      "  - createIdeationSession(projectId, title?)",
      "  - getIdeationSession(sessionId)",
      "  - listIdeationSessions(projectId)",
      "  - archiveIdeationSession(sessionId)",
      "  - deleteIdeationSession(sessionId)",
      "Export from src/api/ideation.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrappers for proposals",
    "steps": [
      "Write unit tests for API wrapper functions",
      "Create proposalApi.ts with type-safe invoke wrappers:",
      "  - createTaskProposal(sessionId, data)",
      "  - updateTaskProposal(proposalId, changes)",
      "  - deleteTaskProposal(proposalId)",
      "  - toggleProposalSelection(proposalId, selected)",
      "  - reorderProposals(sessionId, proposalIds)",
      "  - assessProposalPriority(proposalId)",
      "  - assessAllPriorities(sessionId)",
      "  - addProposalDependency(proposalId, dependsOnId)",
      "  - removeProposalDependency(proposalId, dependsOnId)",
      "  - analyzeDependencies(sessionId)",
      "  - applyProposalsToKanban(options)",
      "Export from src/api/proposal.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrappers for chat",
    "steps": [
      "Write unit tests for API wrapper functions",
      "Create chatApi.ts with type-safe invoke wrappers:",
      "  - sendChatMessage(context, content)",
      "  - getSessionMessages(sessionId)",
      "  - getProjectMessages(projectId)",
      "  - getTaskMessages(taskId)",
      "Export from src/api/chat.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ideationStore with Zustand",
    "steps": [
      "Write unit tests for store actions",
      "Create ideationStore with state:",
      "  - sessions: Map<string, IdeationSession>",
      "  - activeSessionId: string | null",
      "  - isLoading: boolean",
      "  - error: string | null",
      "Implement actions:",
      "  - setActiveSession(sessionId)",
      "  - addSession(session)",
      "  - updateSession(sessionId, changes)",
      "  - removeSession(sessionId)",
      "  - clearError()",
      "Export from src/stores/ideationStore.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create proposalStore with Zustand",
    "steps": [
      "Write unit tests for store actions",
      "Create proposalStore with state:",
      "  - proposals: Map<string, TaskProposal>",
      "  - selectedProposalIds: Set<string>",
      "  - isLoading: boolean",
      "  - error: string | null",
      "Implement actions:",
      "  - setProposals(sessionId, proposals)",
      "  - addProposal(proposal)",
      "  - updateProposal(proposalId, changes)",
      "  - removeProposal(proposalId)",
      "  - toggleSelection(proposalId)",
      "  - selectAll(sessionId)",
      "  - deselectAll()",
      "  - reorder(proposalIds)",
      "Export from src/stores/proposalStore.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create chatStore with Zustand",
    "steps": [
      "Write unit tests for store actions",
      "Create chatStore with state:",
      "  - messages: Map<string, ChatMessage[]> (keyed by context)",
      "  - context: ChatContext",
      "  - isOpen: boolean",
      "  - width: number (panel width)",
      "  - isLoading: boolean",
      "Implement actions:",
      "  - setContext(context)",
      "  - togglePanel()",
      "  - setWidth(width)",
      "  - addMessage(contextKey, message)",
      "  - setMessages(contextKey, messages)",
      "  - clearMessages(contextKey)",
      "Export from src/stores/chatStore.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create useIdeationSession hook",
    "steps": [
      "Write unit tests for hook with mocked API",
      "Create useIdeationSession(sessionId) hook using TanStack Query:",
      "  - Fetch session with proposals and messages",
      "  - Return { session, proposals, messages, isLoading, error }",
      "Create useIdeationSessions(projectId) hook:",
      "  - Fetch all sessions for project",
      "  - Return { sessions, isLoading, error }",
      "Export from src/hooks/useIdeation.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create useProposals hook",
    "steps": [
      "Write unit tests for hook with mocked API",
      "Create useProposals(sessionId) hook:",
      "  - Fetch proposals for session",
      "  - Return { proposals, isLoading, error }",
      "Create useProposalMutation hook:",
      "  - createProposal mutation",
      "  - updateProposal mutation",
      "  - deleteProposal mutation",
      "  - toggleSelection mutation",
      "  - reorder mutation",
      "Export from src/hooks/useProposals.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create usePriorityAssessment hook",
    "steps": [
      "Write unit tests for hook with mocked API",
      "Create usePriorityAssessment() hook:",
      "  - assessPriority(proposalId) mutation",
      "  - assessAllPriorities(sessionId) mutation",
      "  - Return assessment results and loading state",
      "Export from src/hooks/usePriorityAssessment.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create useDependencyGraph hook",
    "steps": [
      "Write unit tests for hook with mocked API",
      "Create useDependencyGraph(sessionId) hook:",
      "  - Fetch dependency graph",
      "  - Return { graph, hasCycles, criticalPath, isLoading }",
      "Create useDependencyMutation hook:",
      "  - addDependency mutation",
      "  - removeDependency mutation",
      "Export from src/hooks/useDependencyGraph.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create useApplyProposals hook",
    "steps": [
      "Write unit tests for hook with mocked API",
      "Create useApplyProposals() hook:",
      "  - apply(options: ApplyProposalsOptions) mutation",
      "  - Return { apply, result, isLoading, error }",
      "  - Invalidate task queries on success",
      "Export from src/hooks/useApplyProposals.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create useChat hook",
    "steps": [
      "Write unit tests for hook with mocked API",
      "Create useChat(context: ChatContext) hook:",
      "  - Fetch messages for context",
      "  - sendMessage(content) mutation",
      "  - Return { messages, sendMessage, isLoading }",
      "Export from src/hooks/useChat.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ChatPanel component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create ChatPanel component with:",
      "  - Header with context indicator and close button",
      "  - Message list with virtual scrolling",
      "  - Auto-scroll to bottom on new messages",
      "  - Input field with send button",
      "  - ⌘+K keyboard shortcut to toggle",
      "  - Resizable width (min 280px, max 50%)",
      "  - Loading state while messages fetch",
      "Apply design system (dark surface, subtle border)",
      "Export from src/components/Chat/ChatPanel.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ChatMessage component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create ChatMessage component with:",
      "  - Role indicator (user vs orchestrator styling)",
      "  - Markdown rendering for content",
      "  - Timestamp display",
      "  - User messages aligned right, orchestrator left",
      "Apply design system (warm colors for user, neutral for orchestrator)",
      "Export from src/components/Chat/ChatMessage.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ChatInput component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create ChatInput component with:",
      "  - Textarea with auto-resize",
      "  - Send button",
      "  - Enter to send, Shift+Enter for newline",
      "  - Disabled state while sending",
      "  - Attach button (placeholder for future)",
      "Export from src/components/Chat/ChatInput.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ProposalCard component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create ProposalCard component with:",
      "  - Checkbox for selection",
      "  - Title and description preview",
      "  - Priority badge (Critical=red, High=orange, Medium=yellow, Low=gray)",
      "  - Category badge",
      "  - Dependency info (depends on X, blocks Y)",
      "  - Edit and Remove action buttons",
      "  - Selected state (orange border)",
      "  - Modified indicator",
      "Apply anti-AI-slop design (no purple, warm orange accent)",
      "Export from src/components/Ideation/ProposalCard.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ProposalList component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create ProposalList component with:",
      "  - List of ProposalCard components",
      "  - Drag-to-reorder with @dnd-kit",
      "  - Multi-select with Shift+click",
      "  - Select all / Deselect all buttons",
      "  - Sort by priority button",
      "  - Clear all button",
      "  - Empty state when no proposals",
      "Export from src/components/Ideation/ProposalList.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ProposalEditModal component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create ProposalEditModal component with:",
      "  - Title input",
      "  - Description textarea",
      "  - Category selector",
      "  - Steps editor (add/remove/reorder)",
      "  - Acceptance criteria editor",
      "  - Priority override selector",
      "  - Complexity selector",
      "  - Save and Cancel buttons",
      "Export from src/components/Ideation/ProposalEditModal.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create ApplyModal component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create ApplyModal component with:",
      "  - List of selected proposals summary",
      "  - Dependency graph preview (simple visualization)",
      "  - Target column selector (Draft, Backlog, Todo)",
      "  - Preserve dependencies checkbox",
      "  - Warnings display (circular deps, missing deps)",
      "  - Apply and Cancel buttons",
      "  - Loading state during apply",
      "Export from src/components/Ideation/ApplyModal.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create PriorityBadge component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create PriorityBadge component with:",
      "  - Critical: Red background (#ef4444)",
      "  - High: Orange background (#ff6b35)",
      "  - Medium: Amber background (#ffa94d)",
      "  - Low: Gray background (#6b7280)",
      "  - Compact and full size variants",
      "Export from src/components/Ideation/PriorityBadge.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create IdeationView component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create IdeationView component with:",
      "  - Split layout: Conversation (left) + Proposals (right)",
      "  - Header with session title, New Session, Archive buttons",
      "  - Conversation panel with message history",
      "  - Proposals panel with ProposalList",
      "  - Apply to [Column] dropdown at bottom",
      "  - Message input at bottom",
      "  - Responsive layout (stack on mobile)",
      "Export from src/components/Ideation/IdeationView.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create SessionSelector component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create SessionSelector component with:",
      "  - Dropdown listing sessions for project",
      "  - Session status indicators",
      "  - New session button",
      "  - Archive action per session",
      "Export from src/components/Ideation/SessionSelector.tsx"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create DependencyVisualization component",
    "steps": [
      "Write component tests with React Testing Library",
      "Create simple dependency visualization with:",
      "  - Lines connecting dependent proposals (SVG or CSS)",
      "  - Critical path highlighting",
      "  - Cycle warning indicators",
      "  - Compact mode for ApplyModal",
      "Export from src/components/Ideation/DependencyVisualization.tsx"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integrate ChatPanel with App layout",
    "steps": [
      "Write integration test for chat toggle",
      "Add ChatPanel to App layout as resizable side panel",
      "Implement ⌘+K global shortcut to toggle",
      "Connect chatStore for open/close state",
      "Persist panel width in localStorage",
      "Test chat opens/closes correctly across views"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integrate IdeationView with navigation",
    "steps": [
      "Write integration test for view switching",
      "Add Ideation link to project navigation",
      "Wire up IdeationView to router",
      "Ensure chat context updates when entering Ideation view",
      "Test session persistence when navigating away and back"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Connect Orchestrator agent to chat",
    "steps": [
      "Write integration test for agent communication",
      "Create OrchestratorService that invokes claude CLI",
      "Implement message streaming from agent to UI",
      "Handle tool calls from agent (create_task_proposal, etc.)",
      "Update proposals store when agent creates proposals",
      "Test full conversation flow with mock agent"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create orchestrator-ideation agent definition",
    "steps": [
      "Create .claude/agents/orchestrator-ideation.md with:",
      "  - name: orchestrator-ideation",
      "  - description: Facilitates ideation sessions and generates task proposals",
      "  - tools: all 11 ideation tools listed",
      "  - model: sonnet",
      "  - Full system prompt with workflow phases",
      "  - Example interaction",
      "  - Guidelines for conversational style"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create ideation skills for orchestrator",
    "steps": [
      "Create .claude/skills/task-decomposition.md:",
      "  - Guide for breaking features into atomic tasks",
      "Create .claude/skills/priority-assessment.md:",
      "  - Guide for calculating and explaining priority",
      "Create .claude/skills/dependency-analysis.md:",
      "  - Guide for identifying task dependencies",
      "Reference skills in orchestrator agent definition"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: Create ideation session flow",
    "steps": [
      "Write test that creates new session from UI",
      "Verify session appears in session list",
      "Verify chat context is updated",
      "Verify empty proposals list shown"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: Full ideation to Kanban flow",
    "steps": [
      "Write test with mock orchestrator agent",
      "Create session → Send message → Receive proposals",
      "Edit proposal priority",
      "Select proposals and apply to Backlog",
      "Verify tasks created in Kanban",
      "Verify dependencies preserved",
      "Verify session status updated"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: Priority calculation",
    "steps": [
      "Write test for priority service",
      "Create proposals with known dependencies",
      "Trigger assess_all_priorities",
      "Verify scores match expected factor calculations",
      "Verify priority mappings correct (80+ = Critical, etc.)"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Integration test: Circular dependency detection",
    "steps": [
      "Write test for dependency validation",
      "Create proposals A → B → C → A (circular)",
      "Attempt to apply",
      "Verify warning returned about cycle",
      "Verify apply blocked or warnings shown"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Visual verification of ideation UI",
    "steps": [
      "Start the app with npm run tauri dev",
      "Navigate to Ideation view",
      "Create a session and add mock proposals manually",
      "Take screenshots of:",
      "  - Empty ideation view",
      "  - View with proposals (selected and unselected)",
      "  - Apply modal with dependency graph",
      "  - Chat panel open alongside view",
      "Verify design matches spec (warm orange, no purple, dark surfaces)"
    ],
    "passes": false
  }
]
```
