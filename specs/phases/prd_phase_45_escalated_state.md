# RalphX - Phase 45: Escalated State for Review System

## Overview

When the AI reviewer chooses `escalate`, it currently transitions to `RevisionNeeded` (same as `needs_changes`), which triggers automatic re-execution. This defeats the purpose of escalation — the AI is explicitly saying it can't decide and needs human judgment.

This phase adds a dedicated `Escalated` state that behaves like `ReviewPassed` (requires human action to proceed), but indicates the AI couldn't make the call. The human can then either approve the task or request changes.

**Reference Plan:**
- `specs/plans/add_escalated_state_for_review_system.md` - Detailed implementation plan with code snippets and dependency graph

## Goals

1. Add `Escalated` variant to `InternalStatus` enum with proper transitions
2. Update state machine to handle `Escalated` state with human approval/rejection events
3. Wire HTTP handlers to transition to `Escalated` on escalate decision
4. Add frontend support with dedicated UI component and workflow column

## Dependencies

### Phase 20 (Review System) - Required

| Dependency | Why Needed |
|------------|------------|
| `InternalStatus::Reviewing` | Source state for escalation transition |
| `ReviewToolOutcome::Escalate` | The decision type we're fixing the transition for |
| `approve_task` / `request_task_changes` handlers | Human action endpoints we'll extend |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/add_escalated_state_for_review_system.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/add_escalated_state_for_review_system.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add Escalated variant to InternalStatus enum with transitions and tests",
    "plan_section": "1. Backend: Add Escalated Status Variant",
    "blocking": [2, 3, 5, 6],
    "blockedBy": [],
    "atomic_commit": "feat(status): add Escalated variant to InternalStatus enum",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '1. Backend: Add Escalated Status Variant'",
      "Add Escalated variant to InternalStatus enum after ReviewPassed",
      "Update valid_transitions(): Reviewing => &[ReviewPassed, RevisionNeeded, Escalated]",
      "Add Escalated => &[Approved, RevisionNeeded] transition rules",
      "Update as_str(): Add InternalStatus::Escalated => 'escalated'",
      "Update from_str(): Add 'escalated' => Ok(InternalStatus::Escalated)",
      "Update all_variants(): Add Escalated to the list",
      "Add transition tests: Reviewing->Escalated, Escalated->Approved, Escalated->RevisionNeeded",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(status): add Escalated variant to InternalStatus enum"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add Escalated state to state machine with dispatch and transitions",
    "plan_section": "2. Backend: Update State Machine Types and Transitions",
    "blocking": [3, 4],
    "blockedBy": [1],
    "atomic_commit": "feat(state-machine): add Escalated state with transitions",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '2. Backend: Update State Machine Types and Transitions'",
      "Add Escalated to State enum in types.rs after ReviewPassed",
      "Add dispatch case: State::Escalated => self.escalated(event)",
      "Implement escalated() handler in transitions.rs with HumanApprove->Approved, HumanRequestChanges->RevisionNeeded, Cancel->Cancelled",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(state-machine): add Escalated state with transitions"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add Escalated status/state conversion mappings in transition service",
    "plan_section": "3. Backend: Update Status/State Conversion Functions",
    "blocking": [5],
    "blockedBy": [1, 2],
    "atomic_commit": "feat(transition-service): add Escalated status/state mapping",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '3. Backend: Update Status/State Conversion Functions'",
      "Add InternalStatus::Escalated => State::Escalated in internal_status_to_state()",
      "Add State::Escalated => InternalStatus::Escalated in state_to_internal_status()",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(transition-service): add Escalated status/state mapping"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Add side effects for Escalated state (event emission and notification)",
    "plan_section": "4. Backend: Add Side Effects for Escalated State",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "feat(side-effects): add event and notification for Escalated state",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '4. Backend: Add Side Effects for Escalated State'",
      "Add State::Escalated match arm in side_effects.rs after ReviewPassed handler",
      "Emit 'review:escalated' event with task_id",
      "Call notifier.notify_with_message with escalation message",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(side-effects): add event and notification for Escalated state"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Update HTTP handler to transition to Escalated on escalate decision",
    "plan_section": "5. Backend: Update HTTP Handler for Escalate",
    "blocking": [],
    "blockedBy": [1, 3],
    "atomic_commit": "feat(reviews): transition to Escalated on escalate decision",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '5. Backend: Update HTTP Handler for Escalate'",
      "Split the combined NeedsChanges|Escalate match arm in complete_review handler",
      "Keep NeedsChanges => InternalStatus::RevisionNeeded",
      "Change Escalate => InternalStatus::Escalated",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(reviews): transition to Escalated on escalate decision"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Update approve_task and request_task_changes to accept Escalated status",
    "plan_section": "6. Backend: Update approve_task/request_task_changes Validation",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(reviews): allow approve/request_changes from Escalated status",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '6. Backend: Update approve_task/request_task_changes Validation'",
      "Update approve_task condition: accept both ReviewPassed AND Escalated",
      "Update request_task_changes condition: accept both ReviewPassed AND Escalated",
      "Update error messages to mention both 'review_passed' and 'escalated' as valid statuses",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(reviews): allow approve/request_changes from Escalated status"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Add escalated to InternalStatus TypeScript types and status groups",
    "plan_section": "7. Frontend: Add Status to TypeScript Types",
    "blocking": [8, 9, 10],
    "blockedBy": [],
    "atomic_commit": "feat(types): add escalated to InternalStatus type",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '7. Frontend: Add Status to TypeScript Types'",
      "Add 'escalated' to InternalStatusSchema after 'review_passed'",
      "Add 'escalated' to REVIEW_STATUSES array",
      "Add 'escalated' to ACTIVE_STATUSES array",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(types): add escalated to InternalStatus type"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Create EscalatedTaskDetail component with warning banner and human action buttons",
    "plan_section": "8. Frontend: Create EscalatedTaskDetail Component",
    "blocking": [9],
    "blockedBy": [7],
    "atomic_commit": "feat(components): add EscalatedTaskDetail view",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '8. Frontend: Create EscalatedTaskDetail Component'",
      "Create src/components/tasks/detail-views/EscalatedTaskDetail.tsx",
      "Use HumanReviewTaskDetail as reference but with warning styling",
      "Add banner: 'AI ESCALATED TO HUMAN' with AlertTriangle icon",
      "Add description: 'AI reviewer couldn't make a decision - needs your input'",
      "Include same Approve/Request Changes buttons as HumanReviewTaskDetail",
      "Display escalation reason from review notes",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(components): add EscalatedTaskDetail view"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Register EscalatedTaskDetail in TaskDetailPanel view registry",
    "plan_section": "9. Frontend: Update View Registry",
    "blocking": [],
    "blockedBy": [7, 8],
    "atomic_commit": "feat(TaskDetailPanel): register EscalatedTaskDetail view",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '9. Frontend: Update View Registry'",
      "Import EscalatedTaskDetail in TaskDetailPanel.tsx",
      "Add 'escalated: EscalatedTaskDetail' to TASK_DETAIL_VIEWS record",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(TaskDetailPanel): register EscalatedTaskDetail view"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "frontend",
    "description": "Add Escalated column group to workflow schema with warning accent",
    "plan_section": "10. Frontend: Update Workflow Column Grouping",
    "blocking": [],
    "blockedBy": [7],
    "atomic_commit": "feat(workflow): add Escalated column group",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '10. Frontend: Update Workflow Column Grouping'",
      "Add new column group after 'ready_approval' in workflow.ts",
      "Set id: 'escalated', label: 'Escalated', statuses: ['escalated']",
      "Set icon: 'AlertTriangle', accentColor: 'hsl(var(--warning))'",
      "Set canDragFrom: false, canDropTo: false",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(workflow): add Escalated column group"
    ],
    "passes": false
  },
  {
    "id": 11,
    "category": "mcp",
    "description": "Update MCP tool descriptions to mention escalated status",
    "plan_section": "11. MCP: Update Tools Description",
    "blocking": [],
    "blockedBy": [1, 7],
    "atomic_commit": "docs(mcp): update approve_task and request_task_changes descriptions",
    "steps": [
      "Read specs/plans/add_escalated_state_for_review_system.md section '11. MCP: Update Tools Description'",
      "Update approve_task description to mention 'escalated' as valid status",
      "Update request_task_changes description to mention 'escalated' as valid status",
      "Commit: docs(mcp): update approve_task and request_task_changes descriptions"
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
| **Escalated behaves like ReviewPassed** | Both require human action to proceed, ensuring escalation doesn't auto-trigger re-execution |
| **Separate column for Escalated** | Visual distinction from ReviewPassed - warning styling vs success styling |
| **Shared approve/request_changes handlers** | Both states lead to the same outcomes (Approved or RevisionNeeded), so reuse existing handlers |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Test `Reviewing → Escalated` transition is valid
- [ ] Test `Escalated → Approved` transition is valid
- [ ] Test `Escalated → RevisionNeeded` transition is valid
- [ ] Test `Escalated` state dispatches to escalated() handler
- [ ] Test escalated() handles HumanApprove, HumanRequestChanges, Cancel events

### Frontend - Run `npm run test`
- [ ] InternalStatus type includes 'escalated'
- [ ] EscalatedTaskDetail component renders correctly
- [ ] TASK_DETAIL_VIEWS maps 'escalated' to EscalatedTaskDetail

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Create a task that requires escalation (security-sensitive, unclear requirements)
- [ ] Verify AI review chooses "escalate"
- [ ] Verify task lands in "Escalated" state, NOT "RevisionNeeded"
- [ ] Verify "Escalated" column appears in Kanban with warning styling
- [ ] Verify EscalatedTaskDetail shows warning banner and escalation reason
- [ ] Verify Approve button transitions to Approved
- [ ] Verify Request Changes button transitions to RevisionNeeded and triggers re-execution

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified: escalate decision in complete_review handler
- [ ] New component EscalatedTaskDetail is imported AND rendered (registered in TASK_DETAIL_VIEWS)
- [ ] approve_task and request_task_changes accept Escalated status
- [ ] State changes reflect in UI (Escalated column, warning styling)

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
