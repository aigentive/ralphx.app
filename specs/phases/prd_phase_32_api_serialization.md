# RalphX - Phase 32: Standardize API Serialization Convention

## Overview

This phase standardizes the API serialization boundary between backend and frontend. Currently, 36% of backend response structs use `#[serde(rename_all = "camelCase")]`, creating an inconsistent API contract where some responses use snake_case and others use camelCase. This inconsistency causes bugs like the TaskProposalResponse parsing failure.

The fix establishes a clear convention: **Backend always outputs snake_case** (Rust's default), and the **frontend transform layer converts to camelCase** (JS convention). This aligns with industry best practices where each layer uses its native convention.

**Reference Plan:**
- `specs/plans/standardize_api_serialization.md` - Detailed implementation plan with struct inventory, frontend schema mapping, and documentation updates

## Goals

1. Remove all `#[serde(rename_all = "camelCase")]` from backend response structs (17 structs across 8 files)
2. Verify and fix frontend Zod schemas to expect snake_case
3. Ensure transform functions exist for all affected API responses
4. Document the convention in code quality standards and CLAUDE.md files

## Dependencies

### Phase 31 (Ideation Performance Optimization) - Required

| Dependency | Why Needed |
|------------|------------|
| Stable ideation system | TaskProposalResponse fix directly affects ideation proposals display |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/standardize_api_serialization.md`
2. Understand the snake_case boundary pattern
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Remove `#[serde(rename_all = "camelCase")]` or fix schema as specified
3. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
4. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol:**

Reference: `.claude/rules/commit-lock.md`

1. Establish project root: `PROJECT_ROOT="$(git rev-parse --show-toplevel)"`
2. Acquire lock before `git add` (see commit-lock.md § Protocol)
3. Stage and commit using `git -C "$PROJECT_ROOT"`
4. Release lock after commit: `rm -f "$PROJECT_ROOT/.commit-lock"`

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks 1-8 have `"blockedBy": []` and can run in parallel
- Tasks 9-13 depend on their corresponding backend task
- Task 14 (transform audit) depends on all schema tasks (9-13)
- Tasks 15-17 (documentation) depend on task 14

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/standardize_api_serialization.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Remove camelCase serialization from TaskProposalResponse (ROOT CAUSE)",
    "plan_section": "Task 1.1: Fix TaskProposalResponse",
    "blocking": [9],
    "blockedBy": [],
    "atomic_commit": "fix(api): remove camelCase serialization from ideation_commands_types",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 1.1'",
      "Open src-tauri/src/commands/ideation_commands/ideation_commands_types.rs",
      "Remove #[serde(rename_all = \"camelCase\")] from TaskProposalResponse struct",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(api): remove camelCase serialization from ideation_commands_types"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Remove camelCase serialization from task_commands types (5 structs)",
    "plan_section": "Task 1.2: Fix task_commands/types.rs",
    "blocking": [10],
    "blockedBy": [],
    "atomic_commit": "fix(api): remove camelCase serialization from task_commands types",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 1.2'",
      "Open src-tauri/src/commands/task_commands/types.rs",
      "Remove #[serde(rename_all = \"camelCase\")] from: AnswerUserQuestionResponse, InjectTaskResponse, TaskResponse, TaskListResponse, StatusTransition",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(api): remove camelCase serialization from task_commands types"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Remove camelCase serialization from TaskStepResponse",
    "plan_section": "Task 1.3: Fix task_step_commands_types.rs",
    "blocking": [11],
    "blockedBy": [],
    "atomic_commit": "fix(api): remove camelCase serialization from task_step_commands_types",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 1.3'",
      "Open src-tauri/src/commands/task_step_commands_types.rs",
      "Remove #[serde(rename_all = \"camelCase\")] from TaskStepResponse",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(api): remove camelCase serialization from task_step_commands_types"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Remove camelCase serialization from execution_commands (2 structs)",
    "plan_section": "Task 1.4: Fix execution_commands.rs",
    "blocking": [12],
    "blockedBy": [],
    "atomic_commit": "fix(api): remove camelCase serialization from execution_commands",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 1.4'",
      "Open src-tauri/src/commands/execution_commands.rs",
      "Remove #[serde(rename_all = \"camelCase\")] from: ExecutionStatusResponse, ExecutionCommandResponse",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(api): remove camelCase serialization from execution_commands"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Remove camelCase serialization from ProjectResponse",
    "plan_section": "Task 1.5: Fix project_commands.rs",
    "blocking": [13],
    "blockedBy": [],
    "atomic_commit": "fix(api): remove camelCase serialization from project_commands",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 1.5'",
      "Open src-tauri/src/commands/project_commands.rs",
      "Remove #[serde(rename_all = \"camelCase\")] from ProjectResponse",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(api): remove camelCase serialization from project_commands"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Remove camelCase serialization from SeedDataResponse",
    "plan_section": "Task 1.6: Fix test_data_commands.rs",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(api): remove camelCase serialization from test_data_commands",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 1.6'",
      "Open src-tauri/src/commands/test_data_commands.rs",
      "Remove #[serde(rename_all = \"camelCase\")] from SeedDataResponse",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(api): remove camelCase serialization from test_data_commands"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "backend",
    "description": "Remove camelCase serialization from unified_chat_commands (5 structs)",
    "plan_section": "Task 1.7: Fix unified_chat_commands.rs",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(api): remove camelCase serialization from unified_chat_commands",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 1.7'",
      "Open src-tauri/src/commands/unified_chat_commands.rs",
      "Remove #[serde(rename_all = \"camelCase\")] from: SendAgentMessageInput, SendAgentMessageResponse, QueueAgentMessageInput, QueuedMessageResponse, AgentConversationResponse",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(api): remove camelCase serialization from unified_chat_commands"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "backend",
    "description": "Remove camelCase serialization from workflow_commands (2 structs)",
    "plan_section": "Task 1.8: Fix workflow_commands.rs",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(api): remove camelCase serialization from workflow_commands",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 1.8'",
      "Open src-tauri/src/commands/workflow_commands.rs",
      "Remove #[serde(rename_all = \"camelCase\")] from: StateGroupResponse, WorkflowColumnResponse",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(api): remove camelCase serialization from workflow_commands"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Verify TaskProposalResponse schema expects snake_case (already done)",
    "plan_section": "Task 2.6: Verify TaskProposalResponse schema",
    "blocking": [14],
    "blockedBy": [1],
    "atomic_commit": "chore(api): verify TaskProposalResponse schema uses snake_case",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 2.6'",
      "Open src/api/ideation.schemas.ts",
      "Verify schema fields use snake_case (session_id, suggested_priority, etc.)",
      "If already snake_case, mark task as passes: true",
      "If camelCase, update to snake_case and add/verify transform function",
      "Run npm run lint && npm run typecheck",
      "Commit: chore(api): verify TaskProposalResponse schema uses snake_case"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "frontend",
    "description": "Verify/fix TaskResponse schema to expect snake_case",
    "plan_section": "Task 2.1: Verify/fix TaskResponse schema",
    "blocking": [14],
    "blockedBy": [2],
    "atomic_commit": "fix(api): update TaskResponse schema to expect snake_case",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 2.1'",
      "Open src/types/task.ts",
      "Check if TaskResponse schema expects snake_case fields",
      "If camelCase, update to snake_case and ensure transform function exists",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(api): update TaskResponse schema to expect snake_case"
    ],
    "passes": false
  },
  {
    "id": 11,
    "category": "frontend",
    "description": "Verify/fix TaskStepResponse schema to expect snake_case",
    "plan_section": "Task 2.5: Verify/fix TaskStepResponse schema",
    "blocking": [14],
    "blockedBy": [3],
    "atomic_commit": "fix(api): update TaskStepResponse schema to expect snake_case",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 2.5'",
      "Open src/types/task-step.ts",
      "Check if TaskStepResponse schema expects snake_case fields",
      "If camelCase, update to snake_case and ensure transform function exists",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(api): update TaskStepResponse schema to expect snake_case"
    ],
    "passes": false
  },
  {
    "id": 12,
    "category": "frontend",
    "description": "Verify/fix ExecutionStatusResponse schema to expect snake_case",
    "plan_section": "Task 2.2: Verify/fix ExecutionStatusResponse schema",
    "blocking": [14],
    "blockedBy": [4],
    "atomic_commit": "fix(api): update ExecutionStatusResponse schema to expect snake_case",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 2.2'",
      "Search for ExecutionStatusResponse in src/types/events.ts or src/api/",
      "Check if schema expects snake_case fields",
      "If camelCase, update to snake_case and ensure transform function exists",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(api): update ExecutionStatusResponse schema to expect snake_case"
    ],
    "passes": false
  },
  {
    "id": 13,
    "category": "frontend",
    "description": "Verify/fix ProjectResponse schema to expect snake_case",
    "plan_section": "Task 2.3: Verify/fix ProjectResponse schema",
    "blocking": [14],
    "blockedBy": [5],
    "atomic_commit": "fix(api): update ProjectResponse schema to expect snake_case",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 2.3'",
      "Open src/types/project.ts",
      "Check if ProjectResponse schema expects snake_case fields",
      "If camelCase, update to snake_case and ensure transform function exists",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(api): update ProjectResponse schema to expect snake_case"
    ],
    "passes": false
  },
  {
    "id": 14,
    "category": "frontend",
    "description": "Audit and add missing transform functions",
    "plan_section": "Task 3.1: Audit transform coverage",
    "blocking": [15],
    "blockedBy": [9, 10, 11, 12, 13],
    "atomic_commit": "feat(api): add missing transform functions for snake_case conversion",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Phase 3'",
      "For each API module (ideation, tasks, projects, execution, chat), verify:",
      "  - Schema exists with snake_case fields",
      "  - Transform function converts snake_case → camelCase",
      "  - API wrapper applies transform before returning",
      "Add any missing transform functions following the pattern in the plan",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add missing transform functions for snake_case conversion"
    ],
    "passes": false
  },
  {
    "id": 15,
    "category": "documentation",
    "description": "Add API serialization convention to code quality standards",
    "plan_section": "Task 4.1: Update code-quality-standards.md",
    "blocking": [16, 17],
    "blockedBy": [14],
    "atomic_commit": "docs: add API serialization convention to code quality standards",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 4.1'",
      "Open .claude/rules/code-quality-standards.md",
      "Add new section '## API Serialization Convention' with:",
      "  - The snake_case Boundary Pattern table",
      "  - Backend Rules (NEVER use rename_all = camelCase)",
      "  - Frontend Rules (schemas expect snake_case, transforms convert)",
      "Commit: docs: add API serialization convention to code quality standards"
    ],
    "passes": false
  },
  {
    "id": 16,
    "category": "documentation",
    "description": "Add API schema convention to frontend CLAUDE.md",
    "plan_section": "Task 4.2: Update src/CLAUDE.md",
    "blocking": [],
    "blockedBy": [15],
    "atomic_commit": "docs: add API schema convention to frontend CLAUDE.md",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 4.2'",
      "Open src/CLAUDE.md",
      "Add section '### API Schema Convention (CRITICAL)'",
      "Include: Zod schemas use snake_case, transforms convert to camelCase",
      "Reference the code-quality-standards.md for full pattern",
      "Commit: docs: add API schema convention to frontend CLAUDE.md"
    ],
    "passes": false
  },
  {
    "id": 17,
    "category": "documentation",
    "description": "Add response serialization convention to backend CLAUDE.md",
    "plan_section": "Task 4.3: Update src-tauri/CLAUDE.md",
    "blocking": [],
    "blockedBy": [15],
    "atomic_commit": "docs: add response serialization convention to backend CLAUDE.md",
    "steps": [
      "Read specs/plans/standardize_api_serialization.md section 'Task 4.3'",
      "Open src-tauri/CLAUDE.md",
      "Add section '### Response Serialization (CRITICAL)'",
      "Include: NEVER use rename_all = camelCase, Rust default is correct",
      "Commit: docs: add response serialization convention to backend CLAUDE.md"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **snake_case at API boundary** | Rust's default serialization; no annotation required; matches industry practice |
| **Transform layer in frontend** | Keeps each layer using its native convention (Rust=snake, JS=camel) |
| **Remove rename_all, don't add** | Less code, fewer bugs, matches Rust convention |
| **Document in code quality standards** | Prevents future inconsistencies; enforced by CLAUDE.md reference |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All 17 structs have `#[serde(rename_all = "camelCase")]` removed
- [ ] No compilation errors
- [ ] All existing tests pass

### Frontend - Run `npm run test`
- [ ] All Zod schemas expect snake_case fields
- [ ] Transform functions exist for all affected types
- [ ] No TypeScript errors

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Load ideation session with proposals (ROOT CAUSE fix verification)
- [ ] Create new proposal via chat
- [ ] Verify tasks load in Kanban view
- [ ] Test execution status display
- [ ] Test project selection/display

### Wiring Verification

**For each affected API response, verify the full path from backend to UI:**

- [ ] TaskProposalResponse: ideation_commands → ideation.schemas.ts → IdeationView
- [ ] TaskResponse: task_commands → task.ts → TaskCard/TaskBoard
- [ ] ExecutionStatusResponse: execution_commands → events.ts → ExecutionBar
- [ ] ProjectResponse: project_commands → project.ts → ProjectSelector

**Common failure modes to check:**
- [ ] No Zod schemas still expecting camelCase
- [ ] No transform functions missing
- [ ] No API wrappers returning raw (untransformed) data

See `.claude/rules/gap-verification.md` for full verification workflow.
