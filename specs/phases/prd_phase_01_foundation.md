# RalphX - Phase 1: Foundation

## Overview

This phase establishes the foundational infrastructure for RalphX: a Tauri 2.0 application with React + TypeScript frontend, Rust backend, and SQLite database. We'll set up the project structure, configure strict TypeScript, create core domain entities and types, and establish the testing infrastructure.

## Dependencies

- No previous phases required
- Prerequisites: macOS 12+, Xcode CLI tools, Rust toolchain, Node.js 18+, Claude CLI

## Scope

### Included
- Tauri 2.0 project scaffolding with React + TypeScript + Tailwind CSS
- Strict TypeScript configuration (all strict flags enabled)
- SQLite database setup with rusqlite
- Core domain entities: Project, Task (basic), InternalStatus enum
- Newtype pattern for type-safe IDs (TaskId, ProjectId)
- Unified error handling (AppError, AppResult)
- Basic Tauri commands for health check
- Testing infrastructure: Vitest (TS), cargo test (Rust), rstest
- Design system foundation (CSS variables, dark theme tokens)
- Project directory structure following clean architecture

### Excluded
- Repository pattern implementations (Phase 2)
- Full state machine with statig (Phase 3)
- Agentic client abstraction (Phase 4)
- Frontend UI components beyond basic shell (Phases 5-6)
- All agent profiles and plugin system (Phase 7+)

## Detailed Requirements

### 1. Tauri 2.0 Project Setup

From the master plan (lines 20-28):
- **Backend**: Rust (process management, database, file system operations)
- **Frontend**: React + TypeScript + Tailwind CSS
- **Why Tauri**:
  - 10MB bundle vs Electron's 100MB+
  - 30-40MB memory vs Electron's 200-300MB
  - Native macOS integration via WKWebView
  - Excellent CLI process spawning via Shell plugin
  - Built-in sandboxing with scoped file system access

### 2. Directory Structure

From the master plan (lines 1515-1635):
```
ralphx/
├── src-tauri/                  # Rust backend (host)
│   ├── src/
│   │   ├── main.rs             # Entry point only (~50 lines)
│   │   ├── lib.rs              # Re-exports, feature flags
│   │   ├── error.rs            # Unified error types
│   │   ├── commands/           # Tauri commands (thin layer)
│   │   │   └── mod.rs
│   │   ├── domain/             # Core domain (pure Rust, no external deps)
│   │   │   ├── mod.rs
│   │   │   └── entities/       # Domain entities
│   │   │       ├── mod.rs
│   │   │       ├── project.rs
│   │   │       └── task.rs
│   │   └── infrastructure/     # External implementations
│   │       └── mod.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                        # React frontend
│   ├── main.tsx                # Entry point only
│   ├── App.tsx                 # Router setup only
│   ├── types/                  # Shared type definitions
│   │   ├── index.ts            # Re-exports
│   │   ├── task.ts             # Task types + Zod schemas
│   │   ├── project.ts
│   │   └── status.ts           # InternalStatus enum
│   ├── lib/                    # Utilities, no React
│   │   ├── tauri.ts            # Tauri invoke wrappers
│   │   └── validation.ts       # Zod schemas
│   ├── components/
│   │   └── ui/                 # Primitive components (placeholder)
│   └── styles/
│       └── globals.css         # Design system tokens
├── package.json
├── vite.config.ts
├── tailwind.config.js
└── tsconfig.json
```

### 3. Rust Backend Structure

#### Error Handling (from plan lines 5368-5402)
```rust
// src-tauri/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Invalid status transition: {from} → {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Validation error: {0}")]
    Validation(String),
}

// Make errors serializable for Tauri
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

#### Type Safety with Newtypes (from plan lines 5404-5461)
```rust
// src-tauri/src/domain/entities/types.rs

// Prevent mixing up IDs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);

impl TaskId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl ProjectId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

// Status as enum, not strings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalStatus {
    Backlog,
    Ready,
    Blocked,
    Executing,
    ExecutionDone,
    QaRefining,
    QaTesting,
    QaPassed,
    QaFailed,
    PendingReview,
    RevisionNeeded,
    Approved,
    Failed,
    Cancelled,
}

impl InternalStatus {
    /// Returns valid transitions from this status
    pub fn valid_transitions(&self) -> &[InternalStatus] {
        use InternalStatus::*;
        match self {
            Backlog => &[Ready, Cancelled],
            Ready => &[Executing, Blocked, Cancelled],
            Blocked => &[Ready, Cancelled],
            Executing => &[ExecutionDone, Failed, Blocked],
            ExecutionDone => &[QaRefining, PendingReview],
            QaRefining => &[QaTesting],
            QaTesting => &[QaPassed, QaFailed],
            QaPassed => &[PendingReview],
            QaFailed => &[RevisionNeeded],
            PendingReview => &[Approved, RevisionNeeded],
            RevisionNeeded => &[Executing, Cancelled],
            Approved => &[Ready], // Re-open
            Failed => &[Ready],
            Cancelled => &[Ready],
        }
    }

    pub fn can_transition_to(&self, target: InternalStatus) -> bool {
        self.valid_transitions().contains(&target)
    }
}
```

#### Project Entity (from plan lines 123-135)
```rust
// src-tauri/src/domain/entities/project.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub working_directory: String,
    pub git_mode: GitMode,
    pub worktree_path: Option<String>,
    pub worktree_branch: Option<String>,
    pub base_branch: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitMode {
    Local,
    Worktree,
}
```

#### Task Entity (from plan lines 589-604)
```rust
// src-tauri/src/domain/entities/task.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub category: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: i32,
    pub internal_status: InternalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(project_id: ProjectId, title: String) -> Self {
        Self {
            id: TaskId::new(),
            project_id,
            category: "feature".to_string(),
            title,
            description: None,
            priority: 0,
            internal_status: InternalStatus::Backlog,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }
}
```

### 4. TypeScript Frontend Structure

#### Strict TypeScript Configuration (from plan lines 5616-5630)
```json
// tsconfig.json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "exactOptionalPropertyTypes": true,
    "forceConsistentCasingInFileNames": true,
    "verbatimModuleSyntax": true,
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

#### Type Definitions with Zod (from plan lines 5684-5744)
```typescript
// src/types/status.ts
import { z } from "zod";

export const InternalStatusSchema = z.enum([
  "backlog",
  "ready",
  "blocked",
  "executing",
  "execution_done",
  "qa_refining",
  "qa_testing",
  "qa_passed",
  "qa_failed",
  "pending_review",
  "revision_needed",
  "approved",
  "failed",
  "cancelled",
]);

export type InternalStatus = z.infer<typeof InternalStatusSchema>;

// src/types/task.ts
import { z } from "zod";
import { InternalStatusSchema } from "./status";

export const TaskSchema = z.object({
  id: z.string().uuid(),
  projectId: z.string().uuid(),
  category: z.string(),
  title: z.string().min(1),
  description: z.string().nullable(),
  priority: z.number().int(),
  internalStatus: InternalStatusSchema,
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
  startedAt: z.string().datetime().nullable(),
  completedAt: z.string().datetime().nullable(),
});

export type Task = z.infer<typeof TaskSchema>;

// src/types/project.ts
import { z } from "zod";

export const GitModeSchema = z.enum(["local", "worktree"]);
export type GitMode = z.infer<typeof GitModeSchema>;

export const ProjectSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1),
  workingDirectory: z.string(),
  gitMode: GitModeSchema,
  worktreePath: z.string().nullable(),
  worktreeBranch: z.string().nullable(),
  baseBranch: z.string().nullable(),
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
});

export type Project = z.infer<typeof ProjectSchema>;
```

#### Tauri Invoke Wrappers (from plan lines 5746-5781)
```typescript
// src/lib/tauri.ts
import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// Generic invoke wrapper with runtime validation
async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

// Health check (basic command for testing)
export const api = {
  health: {
    check: () => typedInvoke("health_check", {}, z.object({ status: z.string() })),
  },
};
```

### 5. Design System Foundation

From the master plan (lines 6101-6196):

#### Color Palette (NOT purple/blue - Anti-AI-Slop)
```css
/* src/styles/globals.css */
:root {
  /* Backgrounds - dark grays, NOT pure black */
  --bg-base: #0f0f0f;
  --bg-surface: #1a1a1a;
  --bg-elevated: #242424;
  --bg-hover: #2d2d2d;

  /* Text - off-white, NOT pure white */
  --text-primary: #f0f0f0;
  --text-secondary: #a0a0a0;
  --text-muted: #666666;

  /* Accent - warm, distinctive (NOT purple) */
  --accent-primary: #ff6b35;      /* Warm orange */
  --accent-secondary: #ffa94d;    /* Soft amber */

  /* Status */
  --status-success: #10b981;      /* Emerald */
  --status-warning: #f59e0b;      /* Amber */
  --status-error: #ef4444;        /* Red */
  --status-info: #3b82f6;         /* Blue (sparingly) */

  /* Borders & Dividers */
  --border-subtle: rgba(255, 255, 255, 0.06);
  --border-default: rgba(255, 255, 255, 0.1);

  /* Typography */
  --font-display: 'SF Pro Display', -apple-system, sans-serif;
  --font-body: 'SF Pro Text', -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;

  /* Spacing (8pt Grid System) */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-6: 24px;
  --space-8: 32px;
  --space-12: 48px;
}

body {
  background-color: var(--bg-base);
  color: var(--text-primary);
  font-family: var(--font-body);
}
```

### 6. Database Schema (Initial Tables)

From the master plan (lines 123-135, 589-604):

```sql
-- Projects table
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  working_directory TEXT NOT NULL,
  git_mode TEXT NOT NULL DEFAULT 'local',
  worktree_path TEXT,
  worktree_branch TEXT,
  base_branch TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Tasks table (basic)
CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  category TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  priority INTEGER DEFAULT 0,
  internal_status TEXT NOT NULL DEFAULT 'backlog',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  started_at DATETIME,
  completed_at DATETIME
);

-- Task state history (audit log)
CREATE TABLE task_state_history (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  from_status TEXT,
  to_status TEXT NOT NULL,
  changed_by TEXT NOT NULL,
  reason TEXT,
  metadata JSON,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 7. Testing Infrastructure

From the master plan (lines 2556-3148):

#### Rust Testing
- Use `cargo test` for unit tests
- Use `rstest` for parameterized tests
- Inline tests with `#[cfg(test)]` module

#### TypeScript Testing
- Vitest for unit and component tests
- Tests live next to source files (e.g., `task.test.ts`)
- Mock Tauri API in tests

## Implementation Notes

### TDD is Mandatory
Every task follows the TDD cycle:
1. RED: Write failing tests first
2. GREEN: Write minimal implementation to pass
3. REFACTOR: Clean up while keeping tests green

### Anti-AI-Slop Guardrails
1. NO purple or blue-purple gradients anywhere
2. NO Inter font - use SF Pro or system fonts
3. NO generic icon grids (3 boxes with icons)
4. NO high-saturation colors on dark backgrounds
5. ALWAYS use CSS variables - never hardcode colors
6. ALWAYS follow 8pt grid - no random spacing

### File Size Limits
- Component: max 200 lines
- Hook: max 100 lines
- Service: max 300 lines
- Type definitions: max 200 lines

## Task List

```json
[
  {
    "category": "setup",
    "description": "Set up agent-browser for visual verification",
    "steps": [
      "Install agent-browser globally: `npm install -g agent-browser`",
      "Create `.claude/skills/agent-browser/` directory",
      "Copy the EXACT SKILL.md content from specs/plan.md lines 3444-3502 to `.claude/skills/agent-browser/SKILL.md`",
      "Create `screenshots/` directory with `.gitkeep`",
      "Verify agent-browser works: `agent-browser --version`"
    ],
    "passes": true
  },
  {
    "category": "setup",
    "description": "Update Claude Code settings for agent-browser permissions",
    "steps": [
      "Read current `.claude/settings.json`",
      "Add the EXACT permissions from specs/plan.md lines 3508-3527 (agent-browser bash permissions)",
      "Merge with existing permissions, do not replace",
      "Verify JSON is valid"
    ],
    "passes": true
  },
  {
    "category": "setup",
    "description": "Update PROMPT.md with visual verification workflow",
    "steps": [
      "Read specs/plan.md lines 3541-3589 for the EXACT Visual Verification section content",
      "Read specs/plan.md lines 3709-3719 for the task type verification table",
      "Add the Visual Verification section to PROMPT.md after the Implementation Workflow section",
      "Include the table showing which task types require visual verification"
    ],
    "passes": true
  },
  {
    "category": "setup",
    "description": "Initialize Tauri 2.0 project with React + TypeScript",
    "steps": [
      "Run `npm create tauri-app@latest ralphx -- --template react-ts`",
      "Verify project builds with `npm run tauri dev`",
      "Add Tailwind CSS: `npm install -D tailwindcss postcss autoprefixer`",
      "Configure Tailwind with dark mode and custom content paths",
      "Verify Tailwind works by adding a test class"
    ],
    "passes": false
  },
  {
    "category": "setup",
    "description": "Configure strict TypeScript settings",
    "steps": [
      "Write test in src/lib/validation.test.ts that expects strict type checking",
      "Update tsconfig.json with all strict flags from plan",
      "Add path aliases (@/*) for cleaner imports",
      "Verify tests pass and type checking is strict"
    ],
    "passes": false
  },
  {
    "category": "setup",
    "description": "Set up Vitest testing infrastructure",
    "steps": [
      "Install Vitest: `npm install -D vitest @testing-library/react @testing-library/jest-dom`",
      "Create vitest.config.ts with proper setup",
      "Create src/test/setup.ts for test utilities",
      "Write a sample test in src/lib/validation.test.ts",
      "Add test scripts to package.json",
      "Verify `npm run test` works"
    ],
    "passes": false
  },
  {
    "category": "setup",
    "description": "Create Rust project directory structure",
    "steps": [
      "Create domain/ module with mod.rs",
      "Create domain/entities/ with mod.rs",
      "Create commands/ module with mod.rs",
      "Create infrastructure/ module with mod.rs",
      "Create error.rs with AppError and AppResult",
      "Update lib.rs to export all modules",
      "Verify `cargo build` succeeds"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement Rust error handling (AppError, AppResult)",
    "steps": [
      "Write tests in src-tauri/src/error.rs for error serialization",
      "Add thiserror dependency to Cargo.toml",
      "Implement AppError enum with variants: Database, TaskNotFound, ProjectNotFound, InvalidTransition, Validation",
      "Implement Serialize for Tauri compatibility",
      "Define AppResult<T> type alias",
      "Verify tests pass with `cargo test`"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement newtype IDs (TaskId, ProjectId)",
    "steps": [
      "Write tests for TaskId and ProjectId in domain/entities/types.rs",
      "Add uuid dependency to Cargo.toml",
      "Implement TaskId with new() method generating UUID",
      "Implement ProjectId with new() method generating UUID",
      "Add derive macros: Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement InternalStatus enum with transition validation",
    "steps": [
      "Write tests for valid_transitions() and can_transition_to() methods",
      "Test all 14 status values parse correctly",
      "Test invalid transitions are rejected",
      "Implement InternalStatus enum with all 14 variants",
      "Implement valid_transitions() returning allowed next states",
      "Implement can_transition_to() using valid_transitions()",
      "Add serde rename_all = snake_case",
      "Verify all tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement Project entity struct",
    "steps": [
      "Write tests for Project creation and serialization",
      "Add chrono dependency for DateTime",
      "Implement Project struct with all fields",
      "Implement GitMode enum (Local, Worktree)",
      "Add derive macros for serialization",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement Task entity struct",
    "steps": [
      "Write tests for Task creation and default values",
      "Implement Task struct with all fields",
      "Implement Task::new() constructor with sensible defaults",
      "Verify internal_status defaults to Backlog",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Set up SQLite database with rusqlite",
    "steps": [
      "Add rusqlite dependency with bundled feature",
      "Create infrastructure/sqlite/ module",
      "Implement connection.rs with database path handling",
      "Create migrations.rs with schema creation SQL",
      "Write tests for database initialization",
      "Verify tables are created correctly"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement basic Tauri health_check command",
    "steps": [
      "Write test that invokes health_check command",
      "Create commands/health.rs with health_check function",
      "Return { status: \"ok\" } response",
      "Register command in main.rs",
      "Verify command works via frontend"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create TypeScript type definitions with Zod schemas",
    "steps": [
      "Install Zod: `npm install zod`",
      "Write tests for InternalStatusSchema validation in src/types/status.test.ts",
      "Write tests for TaskSchema validation in src/types/task.test.ts",
      "Write tests for ProjectSchema validation in src/types/project.test.ts",
      "Implement InternalStatusSchema with all 14 variants",
      "Implement TaskSchema with all fields",
      "Implement ProjectSchema with all fields",
      "Export types from src/types/index.ts",
      "Verify all tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement Tauri invoke wrapper with type safety",
    "steps": [
      "Write tests for typedInvoke function with mocked invoke",
      "Implement typedInvoke with Zod validation",
      "Create api.health.check() wrapper",
      "Verify wrapper returns typed response",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create design system foundation (CSS variables)",
    "steps": [
      "Create src/styles/globals.css with CSS variables",
      "Add all color tokens (bg, text, accent, status)",
      "Add typography tokens (font-display, font-body, font-mono)",
      "Add spacing tokens (8pt grid)",
      "Import globals.css in main.tsx",
      "Apply dark theme to body",
      "Verify app renders with dark background"
    ],
    "passes": false
  },
  {
    "category": "setup",
    "description": "Configure Tailwind with design system tokens",
    "steps": [
      "Update tailwind.config.js to extend with CSS variable colors",
      "Add custom spacing scale matching 8pt grid",
      "Add font family configuration",
      "Disable default colors to enforce design system",
      "Write test component using Tailwind classes",
      "Verify Tailwind compiles correctly"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create basic App shell with dark theme",
    "steps": [
      "Write component test for App rendering",
      "Update App.tsx with minimal shell",
      "Apply dark theme background",
      "Add placeholder text confirming app works",
      "Verify visual appearance with `npm run tauri dev`"
    ],
    "passes": false
  }
]
```
