# RalphX - Phase 3: State Machine

## Overview

This phase implements the core state machine engine using the **statig** crate - a type-safe, hierarchical state machine library for Rust. The state machine is the heart of RalphX, governing all task lifecycle transitions with compile-time validation, async actions for agent spawning, and lifecycle hooks for side effects.

## Dependencies

- Phase 1 (Foundation) must be complete:
  - Rust project structure with domain/entities
  - AppError and AppResult types
  - TaskId, ProjectId newtypes
  - InternalStatus enum with valid_transitions() method
  - Project and Task entity structs

- Phase 2 (Data Layer) must be complete:
  - TaskRepository trait with persist_status_change() method
  - StatusTransition record type
  - SQLite and in-memory repository implementations
  - task_state_history table for audit logging

## Scope

### Included
- statig crate integration with async feature
- TaskEvent enum (all transition triggers)
- TaskContext for shared state machine context
- TaskStateMachine with statig macros
- Hierarchical superstates (Execution, QA, Review)
- State-local data for QaFailed and Failed states
- Entry/exit actions for each state
- Transition guards (conditions for allowing transitions)
- SQLite rehydration pattern (load state from DB, validate with statig)
- State serialization for persistence
- task_state_data table for state-local data
- Comprehensive test suite

### Excluded
- Agent spawning implementations (Phase 4) - use mock/stub services
- Event emission to frontend (Phase 5) - log events only
- Notification service (Phase 9)
- Full review workflow logic (Phase 9)

## Detailed Requirements

### 1. State Machine Design Philosophy

From the master plan (lines 6278-6285):

The state machine is the **core engine** of RalphX. Every status has:
1. **Granular states** - Each distinct operation has its own status (no compound states)
2. **Explicit transitions** - Only defined transitions are allowed
3. **Lifecycle hooks** - `on_enter`, `on_exit`, and transition callbacks
4. **Guards** - Conditions that must be true for a transition to occur
5. **Side effects** - Actions triggered by transitions (spawn agents, emit events, etc.)

### 2. The 14 Internal Statuses

From the master plan (lines 6287-6329):

| Status | Category | Description |
|--------|----------|-------------|
| `Backlog` | idle | Not ready for work, parked |
| `Ready` | idle | Ready to be picked up |
| `Blocked` | idle | Waiting on dependencies or human input |
| `QaPrepping` | qa_prep | QA Prep agent generating acceptance criteria (background) |
| `Executing` | execution | Worker agent actively running |
| `ExecutionDone` | execution | Worker finished, awaiting QA or review |
| `QaRefining` | qa_test | QA agent refining plan based on actual implementation |
| `QaTesting` | qa_test | Browser tests executing |
| `QaPassed` | qa_test | All QA tests passed |
| `QaFailed` | qa_test | QA tests failed, needs attention |
| `PendingReview` | review | Awaiting AI reviewer |
| `RevisionNeeded` | review | Review found issues, needs rework |
| `Approved` | terminal | Complete and verified |
| `Failed` | terminal | Requires manual intervention |
| `Cancelled` | terminal | Intentionally abandoned |

Note: The existing InternalStatus enum from Phase 1 has 14 variants. QaPrepping is tracked separately as a background process flag, not a primary status.

### 3. TaskEvent Enum

From the master plan (lines 6943-6966):

```rust
#[derive(Debug, Clone)]
pub enum TaskEvent {
    // User actions
    Schedule,           // User moves task to Ready
    Cancel,             // User cancels task
    ForceApprove,       // Human override
    Retry,              // Retry from failed/cancelled
    SkipQa,             // Human skips QA failure

    // Agent signals
    ExecutionComplete,  // Worker finished
    ExecutionFailed { error: String },
    NeedsHumanInput { reason: String },
    QaRefinementComplete,
    QaTestsComplete { passed: bool },
    ReviewComplete { approved: bool, feedback: Option<String> },

    // System signals
    BlockersResolved,
    BlockerDetected { blocker_id: String },
}
```

### 4. TaskContext

From the master plan (lines 6970-6996):

```rust
#[derive(Debug)]
pub struct TaskContext {
    pub task_id: String,
    pub project_id: String,
    pub qa_enabled: bool,
    pub qa_prep_complete: bool,
    pub blockers: Vec<Blocker>,
    pub review_feedback: Option<String>,
    pub error: Option<String>,
    pub services: TaskServices,
}

#[derive(Debug)]
pub struct Blocker {
    pub id: String,
    pub resolved: bool,
}

// Services injected into the state machine
pub struct TaskServices {
    pub agent_spawner: Box<dyn AgentSpawner>,
    pub event_emitter: Box<dyn EventEmitter>,
    pub notifier: Box<dyn Notifier>,
}
```

### 5. Hierarchical State Structure

From the master plan (lines 7654-7706):

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                     TaskStateMachine                                     │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                         │
│  ┌─────────┐         ┌───────┐         ┌─────────┐                                     │
│  │ BACKLOG │ ──────► │ READY │ ──────► │ BLOCKED │                                     │
│  └─────────┘         └───┬───┘         └────┬────┘                                     │
│                          │                  │                                           │
│                          │ auto             │ blockers_resolved                         │
│                          ▼                  │                                           │
│  ┌──────────────────────────────────────────┼────────────────────────────────────────┐ │
│  │ <<superstate>> EXECUTION                 │                                        │ │
│  │ ┌───────────┐         ┌────────────────┐ │                                        │ │
│  │ │ EXECUTING │ ──────► │ EXECUTION_DONE │◄┘                                        │ │
│  │ └───────────┘         └───────┬────────┘                                          │ │
│  └───────────────────────────────┼───────────────────────────────────────────────────┘ │
│                                  │                                                      │
│                    ┌─────────────┴─────────────┐                                       │
│                    │ [qa_enabled]               │ [!qa_enabled]                         │
│                    ▼                            │                                       │
│  ┌─────────────────────────────────────────────┼────────────────────────────────────┐ │
│  │ <<superstate>> QA                           │                                    │ │
│  │ ┌─────────────┐     ┌────────────┐          │                                    │ │
│  │ │ QA_REFINING │ ──► │ QA_TESTING │          │                                    │ │
│  │ └─────────────┘     └─────┬──────┘          │                                    │ │
│  │                     ┌─────┴─────┐           │                                    │ │
│  │                     ▼           ▼           │                                    │ │
│  │              ┌───────────┐ ┌───────────┐    │                                    │ │
│  │              │ QA_PASSED │ │ QA_FAILED │    │                                    │ │
│  │              └─────┬─────┘ └─────┬─────┘    │                                    │ │
│  └────────────────────┼─────────────┼─────────────────────────────────────────────────┘ │
│                       │             │ retry     │                                       │
│                       ▼             ▼           ▼                                       │
│  ┌────────────────────────────────────────────────────────────────────────────────────┐ │
│  │ <<superstate>> REVIEW                                                              │ │
│  │ ┌────────────────┐         ┌─────────────────┐                                    │ │
│  │ │ PENDING_REVIEW │ ──────► │ REVISION_NEEDED │ ─────► (back to EXECUTING)         │ │
│  │ └───────┬────────┘         └─────────────────┘                                    │ │
│  └─────────┼──────────────────────────────────────────────────────────────────────────┘ │
│            │ approved                                                                   │
│            ▼                                                                            │
│  ┌──────────────────────────────────────────────────────────────────────────────────┐  │
│  │ <<terminal>>                                                                      │  │
│  │ ┌──────────┐     ┌────────┐     ┌───────────┐                                    │  │
│  │ │ APPROVED │     │ FAILED │     │ CANCELLED │  ◄── (from any non-terminal state) │  │
│  │ └──────────┘     └────────┘     └───────────┘                                    │  │
│  └──────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                         │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### 6. All State Transitions

From the master plan (lines 6587-6790):

| From | To | Trigger | Guard/Condition |
|------|-----|---------|-----------------|
| Backlog | Ready | user | - |
| Backlog | Cancelled | user | - |
| Ready | Executing | automatic | no unresolved blockers |
| Ready | Blocked | system | has unresolved blockers |
| Blocked | Ready | system | all blockers resolved |
| Blocked | Cancelled | user | - |
| Executing | ExecutionDone | agent | - |
| Executing | Failed | agent | has unrecoverable error |
| Executing | Blocked | agent | needs human input |
| ExecutionDone | QaRefining | automatic | qa_enabled |
| ExecutionDone | PendingReview | automatic | !qa_enabled |
| QaRefining | QaTesting | agent | - |
| QaRefining | Failed | agent | qa prep failed |
| QaTesting | QaPassed | agent | all tests passed |
| QaTesting | QaFailed | agent | tests failed |
| QaPassed | PendingReview | automatic | - |
| QaFailed | RevisionNeeded | system | - |
| QaFailed | PendingReview | user | skip QA (human override) |
| PendingReview | Approved | agent | - |
| PendingReview | RevisionNeeded | agent | - |
| PendingReview | Approved | user | human override |
| RevisionNeeded | Executing | automatic | - |
| Failed | Ready | user | retry (clear error) |
| Cancelled | Ready | user | reopen |
| Approved | Ready | user | re-run task |

### 7. State Entry/Exit Actions

From the master plan (lines 6375-6581):

| State | On Enter | On Exit |
|-------|----------|---------|
| Ready | Spawn QA prep if enabled | - |
| Blocked | Emit task_blocked event | - |
| QaPrepping | Spawn qa-prep agent | Mark qa_prep_complete |
| Executing | Set startedAt, spawn worker, emit event | Emit execution_ended |
| ExecutionDone | Set executionCompletedAt, emit event | - |
| QaRefining | Wait for QA prep, spawn qa-refiner | Emit refinement_completed |
| QaTesting | Spawn qa-tester, emit event | Emit testing_ended |
| QaPassed | Emit qa_passed | - |
| QaFailed | Notify qa_failed | - |
| PendingReview | Spawn reviewer | - |
| RevisionNeeded | Emit revision_needed | - |
| Approved | Set completedAt, emit approved, unblock dependents | - |
| Failed | Notify task_failed, emit event | - |
| Cancelled | Emit task_cancelled | - |

### 8. SQLite Integration Pattern

From the master plan (lines 7384-7392):

**Pattern: SQLite as source of truth, statig for transition validation**

statig supports serde serialization, but we use a **rehydration pattern** where:
1. SQLite stores the current state (string enum)
2. On load: create state machine with that initial state
3. Process events → statig validates transitions
4. On transition: persist new state to SQLite

### 9. State Serialization

From the master plan (lines 7490-7551):

```rust
impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Backlog => write!(f, "backlog"),
            State::Ready => write!(f, "ready"),
            State::Blocked => write!(f, "blocked"),
            State::Executing => write!(f, "executing"),
            State::ExecutionDone => write!(f, "execution_done"),
            State::QaRefining => write!(f, "qa_refining"),
            State::QaTesting => write!(f, "qa_testing"),
            State::QaPassed => write!(f, "qa_passed"),
            State::QaFailed(_) => write!(f, "qa_failed"),
            State::PendingReview => write!(f, "pending_review"),
            State::RevisionNeeded => write!(f, "revision_needed"),
            State::Approved => write!(f, "approved"),
            State::Failed(_) => write!(f, "failed"),
            State::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for State {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(State::Backlog),
            "ready" => Ok(State::Ready),
            // ... all variants
            _ => Err(AppError::InvalidStatus(s.to_string())),
        }
    }
}
```

### 10. State-Local Data

From the master plan (lines 7553-7591):

States with data (`qa_failed`, `failed`) need storage:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaFailedData {
    pub failures: Vec<QaFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedData {
    pub error: String,
}
```

**Database schema:**
```sql
CREATE TABLE task_state_data (
    task_id TEXT PRIMARY KEY REFERENCES tasks(id),
    state_type TEXT NOT NULL,    -- 'qa_failed' | 'failed'
    data TEXT NOT NULL,          -- JSON serialized state data
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 11. Cargo Dependencies

From the master plan (lines 6930-6934, 7645-7652):

```toml
[dependencies]
statig = { version = "0.3", features = ["async"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
```

## Implementation Notes

### Service Traits for Dependency Injection

Since Phase 4 (Agentic Client) implements the actual agent spawner, we define mock/stub traits here:

```rust
#[async_trait]
pub trait AgentSpawner: Send + Sync {
    async fn spawn(&self, agent_type: &str, task_id: &str);
    async fn spawn_background(&self, agent_type: &str, task_id: &str);
    async fn wait_for(&self, agent_type: &str, task_id: &str);
}

#[async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit(&self, event_type: &str, task_id: &str);
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, notification_type: &str, task_id: &str);
}
```

### Mock Implementations for Testing

```rust
pub struct MockAgentSpawner {
    pub spawn_calls: Arc<Mutex<Vec<(String, String)>>>,
}

pub struct MockEventEmitter {
    pub events: Arc<Mutex<Vec<(String, String)>>>,
}

pub struct MockNotifier {
    pub notifications: Arc<Mutex<Vec<(String, String)>>>,
}
```

### Testing Strategy

1. Unit tests for each state transition
2. Property-based tests for transition validity
3. Integration tests with in-memory repository
4. Happy path test (Backlog → Ready → Executing → Done → Review → Approved)
5. QA failure retry path test
6. Human override tests (ForceApprove, SkipQa)

## Task List

```json
[
  {
    "category": "setup",
    "description": "Add statig crate and tokio dependencies to Cargo.toml",
    "steps": [
      "Write test that imports statig::prelude::*",
      "Add statig = { version = \"0.3\", features = [\"async\"] } to Cargo.toml",
      "Add tokio = { version = \"1\", features = [\"full\"] } if not present",
      "Add tracing = \"0.1\" for transition logging",
      "Run cargo build to verify dependencies resolve"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskEvent enum with all transition triggers",
    "steps": [
      "Write tests for TaskEvent variants: user actions, agent signals, system signals",
      "Create src-tauri/src/domain/state_machine/mod.rs module",
      "Create src-tauri/src/domain/state_machine/events.rs",
      "Implement TaskEvent enum with all 14 variants from spec",
      "Derive Debug, Clone for TaskEvent",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Blocker and QaFailure structs",
    "steps": [
      "Write tests for Blocker struct creation and resolution",
      "Write tests for QaFailure struct",
      "Create src-tauri/src/domain/state_machine/types.rs",
      "Implement Blocker { id: String, resolved: bool }",
      "Implement QaFailure struct for test failure details",
      "Derive Debug, Clone, Serialize, Deserialize",
      "Run cargo test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create state-local data structs (QaFailedData, FailedData)",
    "steps": [
      "Write tests for QaFailedData serialization",
      "Write tests for FailedData serialization",
      "Add QaFailedData { failures: Vec<QaFailure> } to types.rs",
      "Add FailedData { error: String } to types.rs",
      "Implement Default trait for both",
      "Derive Debug, Clone, Serialize, Deserialize, Default",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create service traits for dependency injection (AgentSpawner, EventEmitter, Notifier)",
    "steps": [
      "Create src-tauri/src/domain/state_machine/services.rs",
      "Define AgentSpawner trait with spawn, spawn_background, wait_for methods",
      "Define EventEmitter trait with emit method",
      "Define Notifier trait with notify method",
      "Use async_trait for async method support",
      "Run cargo build to verify trait definitions compile"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create mock service implementations for testing",
    "steps": [
      "Write tests verifying mock services record calls",
      "Create src-tauri/src/domain/state_machine/mocks.rs",
      "Implement MockAgentSpawner with Arc<Mutex<Vec<...>>> to record calls",
      "Implement MockEventEmitter to record emitted events",
      "Implement MockNotifier to record notifications",
      "Add helper methods: new(), get_calls(), clear()",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create TaskServices container and TaskContext struct",
    "steps": [
      "Write tests for TaskContext creation and field access",
      "Create src-tauri/src/domain/state_machine/context.rs",
      "Implement TaskServices with Box<dyn AgentSpawner>, etc.",
      "Implement TaskContext with task_id, project_id, qa_enabled, blockers, etc.",
      "Add helper methods for blocker checking: has_unresolved_blockers(), etc.",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement TaskStateMachine with statig - idle states (Backlog, Ready, Blocked)",
    "steps": [
      "Write tests for Backlog → Ready transition",
      "Write tests for Backlog → Cancelled transition",
      "Write tests for Ready → Blocked (when blockers detected)",
      "Write tests for Blocked → Ready (when blockers resolved)",
      "Create src-tauri/src/domain/state_machine/machine.rs",
      "Use #[state_machine] macro with initial = \"State::backlog()\"",
      "Implement backlog, ready, blocked state functions",
      "Add #[action] enter_ready for QA prep spawning",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement execution superstate and states (Executing, ExecutionDone)",
    "steps": [
      "Write tests for Ready → Executing auto-transition",
      "Write tests for Executing → ExecutionDone (agent signals completion)",
      "Write tests for Executing → Failed (agent signals error)",
      "Write tests for Executing → Blocked (needs human input)",
      "Add #[superstate] execution for common handling",
      "Add #[state(superstate = \"execution\")] for executing, execution_done",
      "Add enter_executing action to spawn worker and emit events",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement QA superstate and states (QaRefining, QaTesting, QaPassed, QaFailed)",
    "steps": [
      "Write tests for ExecutionDone → QaRefining (when qa_enabled)",
      "Write tests for ExecutionDone → PendingReview (when !qa_enabled)",
      "Write tests for QaRefining → QaTesting",
      "Write tests for QaTesting → QaPassed / QaFailed",
      "Write tests for QaFailed → RevisionNeeded (retry)",
      "Write tests for QaFailed → PendingReview (human SkipQa)",
      "Add #[superstate] qa with common SkipQa handling",
      "Implement qa_refining with wait_for QA prep logic",
      "Implement qa_testing with passed/failed branching",
      "Implement qa_passed with auto-transition to review",
      "Implement qa_failed with state-local QaFailedData",
      "Add entry actions for each state",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement review superstate and states (PendingReview, RevisionNeeded)",
    "steps": [
      "Write tests for PendingReview → Approved (reviewer approves)",
      "Write tests for PendingReview → RevisionNeeded (reviewer rejects)",
      "Write tests for PendingReview → Approved (human ForceApprove)",
      "Write tests for RevisionNeeded → Executing (auto-transition)",
      "Add #[superstate] review",
      "Implement pending_review with reviewer spawning",
      "Implement revision_needed with feedback handling",
      "Add entry actions",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement terminal states (Approved, Failed, Cancelled)",
    "steps": [
      "Write tests for terminal state entry actions",
      "Write tests for Retry event from terminal states",
      "Write tests for Approved → Ready (re-run task)",
      "Implement approved state with completedAt and dependent unblocking",
      "Implement failed state with FailedData and error clearing on retry",
      "Implement cancelled state",
      "Add entry actions for each",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Add on_transition and on_dispatch hooks for logging",
    "steps": [
      "Write tests verifying transition logging",
      "Add on_transition callback to log from/to states",
      "Add on_dispatch callback to log state/event pairs",
      "Use tracing::info! and tracing::debug! macros",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement State Display and FromStr for SQLite serialization",
    "steps": [
      "Write tests for State → String → State roundtrip",
      "Write tests for all 14 state string representations",
      "Write tests for invalid string parsing returns AppError",
      "Implement Display for State enum",
      "Implement FromStr for State enum",
      "Handle state-local data variants (qa_failed, failed) with defaults",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "setup",
    "description": "Create task_state_data table migration",
    "steps": [
      "Write integration test for task_state_data table",
      "Add migration for CREATE TABLE task_state_data",
      "Include task_id, state_type, data (JSON), updated_at columns",
      "Add foreign key to tasks table",
      "Run migration and verify table exists",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement state-local data persistence helpers",
    "steps": [
      "Write tests for saving and loading QaFailedData",
      "Write tests for saving and loading FailedData",
      "Create helpers to serialize state data to JSON",
      "Create helpers to persist state data to task_state_data table",
      "Create helpers to load state with data from database",
      "Handle missing data with Default trait",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create TaskStateMachineRepository for SQLite integration",
    "steps": [
      "Write integration tests for load_with_state_machine",
      "Write integration tests for process_event",
      "Create src-tauri/src/infrastructure/sqlite/state_machine_repository.rs",
      "Implement load_with_state_machine to create SM from persisted state",
      "Implement process_event to handle event and persist new state",
      "Use rehydration pattern (SQLite source of truth, statig for validation)",
      "Wrap state changes in database transactions",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement atomic transition with side effects",
    "steps": [
      "Write tests for atomicity (side effect fails → rollback)",
      "Implement transition_atomically function",
      "Accept task_id, event, and side_effect closure",
      "Execute side effect within transaction",
      "Rollback on side effect failure",
      "Persist state change only on success",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Create happy path integration test",
    "steps": [
      "Write test: Backlog → Ready → Executing → ExecutionDone → PendingReview → Approved",
      "Use in-memory repository",
      "Verify each transition fires correct entry actions",
      "Verify status history is recorded",
      "Verify final state is Approved",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Create QA flow integration test",
    "steps": [
      "Write test with qa_enabled = true",
      "Verify ExecutionDone → QaRefining → QaTesting → QaPassed → PendingReview",
      "Write test for QA failure and retry path",
      "Verify RevisionNeeded → Executing loop",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Create human override integration tests",
    "steps": [
      "Write test for ForceApprove from PendingReview",
      "Write test for SkipQa from QaFailed",
      "Write test for Retry from Failed/Cancelled/Approved",
      "Verify error state is cleared on retry",
      "Run cargo test"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Export state machine module from domain layer",
    "steps": [
      "Add pub mod state_machine to src-tauri/src/domain/mod.rs",
      "Re-export key types: TaskStateMachine, TaskEvent, TaskContext, State",
      "Re-export service traits: AgentSpawner, EventEmitter, Notifier",
      "Re-export mocks for testing",
      "Update lib.rs if needed",
      "Run cargo build"
    ],
    "passes": false
  }
]
```
