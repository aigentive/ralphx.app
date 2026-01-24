# RalphX - Phase 2: Data Layer

## Overview

This phase implements the repository pattern architecture for RalphX, providing a clean abstraction over data persistence. We create repository traits in the domain layer and implement them with SQLite (production) and in-memory (testing) backends. This enables full test isolation, future database migration flexibility, and clean architecture principles.

## Dependencies

- Phase 1 (Foundation) must be complete:
  - Rust project structure with domain/entities
  - AppError and AppResult types
  - TaskId, ProjectId newtypes
  - InternalStatus enum
  - Project and Task entity structs
  - SQLite database setup with rusqlite

## Scope

### Included
- Repository trait definitions (TaskRepository, ProjectRepository)
- SQLite repository implementations
- In-memory repository implementations (for testing)
- StateTransition record type for audit logging
- AppState container for dependency injection
- Database migrations system
- Task blockers table and dependency tracking
- Integration with existing entity types

### Excluded
- State machine integration (Phase 3) - repository methods that reference State/TaskEvent will be stubbed
- Full artifact and workflow repositories (Phase 11) - only basic structure if needed
- Agentic client integration (Phase 4)

## Detailed Requirements

### 1. Repository Pattern Architecture

From the master plan (lines 4501-4537):

**Why Repository Pattern?**
- **Testability**: Swap SQLite for in-memory during tests
- **Flexibility**: Migrate to PostgreSQL/cloud storage later
- **Clean Architecture**: Domain logic doesn't know about storage
- **Dependency Inversion**: High-level modules don't depend on low-level

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            APPLICATION LAYER                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │ Tauri Commands  │  │   Use Cases     │  │    App State (DI)           │ │
│  └────────┬────────┘  └────────┬────────┘  └──────────────┬──────────────┘ │
└───────────┼────────────────────┼───────────────────────────┼────────────────┘
            │                    │                           │
            ▼                    ▼                           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DOMAIN LAYER                                   │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │    Entities     │  │    Services     │  │  Repository Traits          │ │
│  │  (Task, etc.)   │  │ (TaskService)   │  │  (trait TaskRepository)     │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┬──────────────┘ │
└────────────────────────────────────────────────────────────┼────────────────┘
                                                             │ implements
                                                             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          INFRASTRUCTURE LAYER                               │
│  ┌───────────────────────────┐  ┌───────────────────────────────────────┐  │
│  │    SQLite Implementation  │  │   In-Memory Implementation (tests)   │  │
│  │  ┌─────────────────────┐  │  │  ┌─────────────────────────────────┐  │  │
│  │  │ SqliteTaskRepo      │  │  │  │ MemoryTaskRepo                  │  │  │
│  │  │ impl TaskRepository │  │  │  │ impl TaskRepository             │  │  │
│  │  └─────────────────────┘  │  │  └─────────────────────────────────┘  │  │
│  └───────────────────────────┘  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2. Repository Trait Definitions

From the master plan (lines 4539-4648):

#### TaskRepository Trait
```rust
// src-tauri/src/domain/repositories/task_repository.rs

use async_trait::async_trait;
use crate::domain::entities::{Task, TaskId, ProjectId, InternalStatus};
use crate::error::AppResult;

/// Repository trait for Task persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ═══════════════════════════════════════════════════════════════════════
    // CRUD Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Create a new task
    async fn create(&self, task: Task) -> AppResult<Task>;

    /// Get task by ID
    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>>;

    /// Get all tasks for a project
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>>;

    /// Update a task
    async fn update(&self, task: &Task) -> AppResult<()>;

    /// Delete a task
    async fn delete(&self, id: &TaskId) -> AppResult<()>;

    // ═══════════════════════════════════════════════════════════════════════
    // Status Operations (Phase 3 will add full state machine integration)
    // ═══════════════════════════════════════════════════════════════════════

    /// Get tasks by status
    async fn get_by_status(&self, project_id: &ProjectId, status: InternalStatus) -> AppResult<Vec<Task>>;

    /// Persist a status change with audit log entry
    async fn persist_status_change(
        &self,
        id: &TaskId,
        from: InternalStatus,
        to: InternalStatus,
        trigger: &str,
    ) -> AppResult<()>;

    /// Get status history for audit
    async fn get_status_history(&self, id: &TaskId) -> AppResult<Vec<StatusTransition>>;

    // ═══════════════════════════════════════════════════════════════════════
    // Query Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Get next task ready for execution (READY status, no blockers)
    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>>;

    /// Get tasks blocking a given task
    async fn get_blockers(&self, id: &TaskId) -> AppResult<Vec<Task>>;

    /// Get tasks blocked by a given task
    async fn get_dependents(&self, id: &TaskId) -> AppResult<Vec<Task>>;

    /// Add a blocker relationship
    async fn add_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()>;

    /// Remove/resolve a blocker relationship
    async fn resolve_blocker(&self, task_id: &TaskId, blocker_id: &TaskId) -> AppResult<()>;
}

/// Status transition record for audit log
#[derive(Debug, Clone)]
pub struct StatusTransition {
    pub from: InternalStatus,
    pub to: InternalStatus,
    pub trigger: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

#### ProjectRepository Trait
```rust
// src-tauri/src/domain/repositories/project_repository.rs

use async_trait::async_trait;
use crate::domain::entities::{Project, ProjectId};
use crate::error::AppResult;

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, project: Project) -> AppResult<Project>;
    async fn get_by_id(&self, id: &ProjectId) -> AppResult<Option<Project>>;
    async fn get_all(&self) -> AppResult<Vec<Project>>;
    async fn update(&self, project: &Project) -> AppResult<()>;
    async fn delete(&self, id: &ProjectId) -> AppResult<()>;
    async fn get_by_working_directory(&self, path: &str) -> AppResult<Option<Project>>;
}
```

### 3. SQLite Implementation

From the master plan (lines 4651-4796):

```rust
// src-tauri/src/infrastructure/sqlite/sqlite_task_repo.rs

use async_trait::async_trait;
use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::entities::{Task, TaskId, ProjectId, InternalStatus};
use crate::domain::repositories::{TaskRepository, StatusTransition};
use crate::error::{AppError, AppResult};

pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl TaskRepository for SqliteTaskRepository {
    async fn create(&self, task: Task) -> AppResult<Task> {
        let conn = self.conn.lock().await;
        conn.execute(
            r#"INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            params![
                task.id.0,
                task.project_id.0,
                task.category,
                task.title,
                task.description,
                task.priority,
                task.internal_status.to_string(),
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(task)
    }

    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;
        let result = conn.query_row(
            "SELECT * FROM tasks WHERE id = ?",
            params![id.0],
            |row| Task::from_row(row),
        );
        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT * FROM tasks WHERE project_id = ? ORDER BY priority DESC, created_at ASC"
        )?;
        let tasks = stmt
            .query_map(params![project_id.0], |row| Task::from_row(row))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    async fn persist_status_change(
        &self,
        id: &TaskId,
        from: InternalStatus,
        to: InternalStatus,
        trigger: &str,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Use transaction for atomicity
        conn.execute("BEGIN TRANSACTION", [])?;

        // Update task status
        conn.execute(
            "UPDATE tasks SET internal_status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![to.to_string(), id.0],
        )?;

        // Record in audit log
        conn.execute(
            r#"INSERT INTO task_state_history (id, task_id, from_status, to_status, changed_by, created_at)
               VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)"#,
            params![
                uuid::Uuid::new_v4().to_string(),
                id.0,
                from.to_string(),
                to.to_string(),
                trigger,
            ],
        )?;

        conn.execute("COMMIT", [])?;
        Ok(())
    }

    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;
        let result = conn.query_row(
            r#"SELECT * FROM tasks
               WHERE project_id = ?
               AND internal_status = 'ready'
               AND id NOT IN (
                   SELECT task_id FROM task_blockers WHERE resolved = FALSE
               )
               ORDER BY priority DESC, created_at ASC
               LIMIT 1"#,
            params![project_id.0],
            |row| Task::from_row(row),
        );
        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    // ... other methods
}
```

### 4. In-Memory Implementation (for Testing)

From the master plan (lines 4799-4908):

```rust
// src-tauri/src/infrastructure/memory/memory_task_repo.rs

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::{Task, TaskId, ProjectId, InternalStatus};
use crate::domain::repositories::{TaskRepository, StatusTransition};
use crate::error::AppResult;

/// In-memory implementation for testing (no real database)
pub struct MemoryTaskRepository {
    tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    history: Arc<RwLock<Vec<StatusTransition>>>,
    blockers: Arc<RwLock<HashMap<TaskId, Vec<TaskId>>>>,
}

impl MemoryTaskRepository {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            blockers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with pre-populated data (for tests)
    pub fn with_tasks(tasks: Vec<Task>) -> Self {
        let map: HashMap<TaskId, Task> = tasks.into_iter().map(|t| (t.id.clone(), t)).collect();
        Self {
            tasks: Arc::new(RwLock::new(map)),
            history: Arc::new(RwLock::new(Vec::new())),
            blockers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl TaskRepository for MemoryTaskRepository {
    async fn create(&self, task: Task) -> AppResult<Task> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(task)
    }

    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(id).cloned())
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        let mut result: Vec<Task> = tasks
            .values()
            .filter(|t| t.project_id == *project_id)
            .cloned()
            .collect();
        result.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        Ok(result)
    }

    async fn persist_status_change(
        &self,
        id: &TaskId,
        from: InternalStatus,
        to: InternalStatus,
        trigger: &str,
    ) -> AppResult<()> {
        // Update task
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.internal_status = to;
            task.updated_at = chrono::Utc::now();
        }

        // Record history
        let mut history = self.history.write().await;
        history.push(StatusTransition {
            from,
            to,
            trigger: trigger.to_string(),
            timestamp: chrono::Utc::now(),
        });

        Ok(())
    }

    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>> {
        let tasks = self.tasks.read().await;
        let blockers = self.blockers.read().await;

        let mut ready_tasks: Vec<&Task> = tasks
            .values()
            .filter(|t| {
                t.project_id == *project_id
                && t.internal_status == InternalStatus::Ready
                && !blockers.get(&t.id).map(|b| !b.is_empty()).unwrap_or(false)
            })
            .collect();

        ready_tasks.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });

        Ok(ready_tasks.first().cloned().cloned())
    }

    // ... other methods
}
```

### 5. Dependency Injection (App State)

From the master plan (lines 4911-4979):

```rust
// src-tauri/src/application/app_state.rs

use std::sync::Arc;
use crate::domain::repositories::{ProjectRepository, TaskRepository};
use crate::infrastructure::sqlite::{SqliteProjectRepository, SqliteTaskRepository};
use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};

/// Application state container (dependency injection)
pub struct AppState {
    pub project_repo: Arc<dyn ProjectRepository>,
    pub task_repo: Arc<dyn TaskRepository>,
}

impl AppState {
    /// Create production app state with SQLite
    pub fn new_production(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            project_repo: Arc::new(SqliteProjectRepository::new(conn.clone())),
            task_repo: Arc::new(SqliteTaskRepository::new(conn.clone())),
        }
    }

    /// Create test app state with in-memory repositories
    pub fn new_test() -> Self {
        Self {
            project_repo: Arc::new(MemoryProjectRepository::new()),
            task_repo: Arc::new(MemoryTaskRepository::new()),
        }
    }

    /// Create with custom repositories (for advanced testing)
    pub fn with_repos(
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
    ) -> Self {
        Self { project_repo, task_repo }
    }
}
```

### 6. Database Schema Updates

From the master plan (lines 591-740):

```sql
-- Task blockers table (for dependency tracking)
CREATE TABLE task_blockers (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    blocker_id TEXT NOT NULL REFERENCES tasks(id),
    resolved BOOLEAN DEFAULT FALSE,
    resolved_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(task_id, blocker_id)
);

-- Index for efficient blocker queries
CREATE INDEX idx_task_blockers_task_id ON task_blockers(task_id);
CREATE INDEX idx_task_blockers_blocker_id ON task_blockers(blocker_id);
CREATE INDEX idx_task_blockers_unresolved ON task_blockers(task_id) WHERE resolved = FALSE;

-- Index for status queries
CREATE INDEX idx_tasks_project_status ON tasks(project_id, internal_status);
CREATE INDEX idx_tasks_priority ON tasks(project_id, priority DESC, created_at ASC);
```

### 7. Task Entity Extensions

The Task entity needs `from_row` for SQLite deserialization:

```rust
impl Task {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: TaskId(row.get("id")?),
            project_id: ProjectId(row.get("project_id")?),
            category: row.get("category")?,
            title: row.get("title")?,
            description: row.get("description")?,
            priority: row.get("priority")?,
            internal_status: row.get::<_, String>("internal_status")?
                .parse()
                .unwrap_or(InternalStatus::Backlog),
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            started_at: row.get("started_at")?,
            completed_at: row.get("completed_at")?,
        })
    }
}
```

## Implementation Notes

### TDD is Mandatory
Every task follows the TDD cycle:
1. RED: Write failing tests first
2. GREEN: Write minimal implementation to pass
3. REFACTOR: Clean up while keeping tests green

### Async Trait Pattern
Use `async_trait` crate for async methods in traits. This adds some overhead but enables clean async repository interfaces.

### Connection Management
- SQLite connections are wrapped in `Arc<Mutex<Connection>>`
- Tokio mutex is used for async compatibility
- Connection pooling not needed for SQLite (single writer)

### Transaction Safety
- Multi-step operations (like status changes with audit logs) use explicit transactions
- In-memory implementation doesn't need transactions but maintains consistency

### Testing Strategy
- Unit tests use `MemoryTaskRepository` exclusively
- Integration tests can use SQLite with in-memory database (`:memory:`)
- No mocking libraries needed - trait objects provide natural mocking

## Task List

```json
[
  {
    "category": "setup",
    "description": "Add async-trait and tokio dependencies to Cargo.toml",
    "steps": [
      "Add async-trait = \"0.1\" to Cargo.toml dependencies",
      "Add tokio = { version = \"1\", features = [\"sync\", \"rt-multi-thread\"] } to dependencies",
      "Verify cargo build succeeds"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create domain/repositories module structure",
    "steps": [
      "Create src-tauri/src/domain/repositories/ directory",
      "Create mod.rs with pub mod declarations for task_repository, project_repository",
      "Create status_transition.rs with StatusTransition struct",
      "Update domain/mod.rs to export repositories module",
      "Verify cargo build succeeds"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TaskRepository trait definition",
    "steps": [
      "Write tests for StatusTransition serialization/construction",
      "Create task_repository.rs with TaskRepository trait",
      "Define all CRUD method signatures (create, get_by_id, get_by_project, update, delete)",
      "Define status operations (get_by_status, persist_status_change, get_status_history)",
      "Define query operations (get_next_executable, get_blockers, get_dependents, add_blocker, resolve_blocker)",
      "Add StatusTransition struct with Debug, Clone derives",
      "Verify cargo build succeeds"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ProjectRepository trait definition",
    "steps": [
      "Write tests for ProjectRepository trait object usage",
      "Create project_repository.rs with ProjectRepository trait",
      "Define CRUD methods (create, get_by_id, get_all, update, delete)",
      "Define get_by_working_directory method",
      "Verify cargo build succeeds"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Add InternalStatus string conversion methods",
    "steps": [
      "Write tests for InternalStatus to/from string conversion",
      "Implement Display trait for InternalStatus (to_string)",
      "Implement FromStr trait for InternalStatus (parse)",
      "Ensure all 14 status variants round-trip correctly",
      "Verify all tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Task::from_row for SQLite deserialization",
    "steps": [
      "Write tests for Task::from_row with mock row data",
      "Implement from_row method on Task entity",
      "Handle DateTime parsing from SQLite strings",
      "Handle Optional fields (description, started_at, completed_at)",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Project::from_row for SQLite deserialization",
    "steps": [
      "Write tests for Project::from_row with mock row data",
      "Implement from_row method on Project entity",
      "Handle GitMode parsing from string",
      "Handle Optional fields (worktree_path, worktree_branch, base_branch)",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create infrastructure/memory module for in-memory repositories",
    "steps": [
      "Create src-tauri/src/infrastructure/memory/ directory",
      "Create mod.rs with pub mod declarations",
      "Update infrastructure/mod.rs to export memory module",
      "Verify cargo build succeeds"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement MemoryTaskRepository",
    "steps": [
      "Write comprehensive tests for all TaskRepository methods",
      "Test create returns the task with ID",
      "Test get_by_id returns None for missing task",
      "Test get_by_project filters correctly and sorts by priority/created_at",
      "Test update modifies existing task",
      "Test delete removes task",
      "Test persist_status_change updates task and records history",
      "Test get_status_history returns recorded transitions",
      "Test get_next_executable respects blockers and sorting",
      "Test add_blocker and resolve_blocker work correctly",
      "Implement MemoryTaskRepository struct with RwLock<HashMap>",
      "Implement all trait methods",
      "Verify all tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement MemoryProjectRepository",
    "steps": [
      "Write tests for all ProjectRepository methods",
      "Test create, get_by_id, get_all, update, delete",
      "Test get_by_working_directory returns correct project",
      "Implement MemoryProjectRepository struct",
      "Implement all trait methods",
      "Verify all tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Add task_blockers table to database migrations",
    "steps": [
      "Write test that verifies task_blockers table is created",
      "Add CREATE TABLE task_blockers SQL to migrations",
      "Add indexes for efficient blocker queries",
      "Add index on tasks for status queries",
      "Run migrations and verify schema",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteTaskRepository CRUD operations",
    "steps": [
      "Write integration tests using in-memory SQLite database",
      "Test create inserts task and returns it",
      "Test get_by_id retrieves task correctly",
      "Test get_by_project returns sorted tasks",
      "Test update modifies task fields",
      "Test delete removes task from database",
      "Implement SqliteTaskRepository struct",
      "Implement CRUD methods with proper SQL",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteTaskRepository status operations",
    "steps": [
      "Write tests for persist_status_change with transaction safety",
      "Test that status change and history are atomic",
      "Test get_status_history returns transitions in order",
      "Test get_by_status filters correctly",
      "Implement persist_status_change with BEGIN/COMMIT",
      "Implement get_status_history query",
      "Implement get_by_status query",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteTaskRepository blocker operations",
    "steps": [
      "Write tests for add_blocker and resolve_blocker",
      "Test get_blockers returns blocking tasks",
      "Test get_dependents returns dependent tasks",
      "Test get_next_executable excludes blocked tasks",
      "Implement add_blocker INSERT",
      "Implement resolve_blocker UPDATE",
      "Implement get_blockers JOIN query",
      "Implement get_dependents JOIN query",
      "Implement get_next_executable with blocker subquery",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteProjectRepository",
    "steps": [
      "Write integration tests for all ProjectRepository methods",
      "Test CRUD operations",
      "Test get_by_working_directory",
      "Implement SqliteProjectRepository struct",
      "Implement all trait methods",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create application/app_state.rs with AppState",
    "steps": [
      "Create src-tauri/src/application/ directory and mod.rs",
      "Write tests for AppState::new_test() and with_repos()",
      "Implement AppState struct with repository trait objects",
      "Implement new_production() constructor",
      "Implement new_test() constructor with memory repos",
      "Implement with_repos() for custom injection",
      "Update lib.rs to export application module",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Integrate AppState with Tauri managed state",
    "steps": [
      "Write test that retrieves AppState from Tauri state",
      "Update main.rs to create AppState and add to managed state",
      "Update health_check command to accept State<AppState>",
      "Verify app builds and health_check works",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for task CRUD",
    "steps": [
      "Write tests for list_tasks, get_task, create_task, update_task, delete_task commands",
      "Create commands/task_commands.rs",
      "Implement list_tasks command using task_repo.get_by_project()",
      "Implement get_task command using task_repo.get_by_id()",
      "Implement create_task command",
      "Implement update_task command",
      "Implement delete_task command",
      "Register commands in main.rs",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create Tauri commands for project CRUD",
    "steps": [
      "Write tests for list_projects, get_project, create_project, update_project, delete_project commands",
      "Create commands/project_commands.rs",
      "Implement all CRUD commands using project_repo",
      "Register commands in main.rs",
      "Verify tests pass"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Create integration test demonstrating repository swapping",
    "steps": [
      "Write test that uses MemoryTaskRepository for business logic",
      "Verify same logic works with SQLite (in-memory database)",
      "Document the pattern for future developers",
      "Verify tests pass"
    ],
    "passes": false
  }
]
```
