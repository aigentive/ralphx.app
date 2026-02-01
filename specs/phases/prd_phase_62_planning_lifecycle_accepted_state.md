# RalphX - Phase 62: Planning Lifecycle & Accepted State Visibility

## Overview

This phase transforms the ideation system's apply workflow from per-proposal selection to whole-plan acceptance. Currently, users can select individual proposals to apply, creating confusing partial states where some proposals are applied and others aren't. The dependency graph can break if dependent proposals aren't applied together.

The new behavior enforces "Accept Plan" as an all-or-nothing operation that:
1. Validates the dependency graph is complete before allowing acceptance
2. Preserves the full dependency structure when creating tasks
3. Moves the plan to a "History" section (visible, not hidden)
4. Enables read-only chat in accepted plans
5. Automatically blocks/unblocks tasks based on their dependencies during execution

**Reference Plan:**
- `specs/plans/planning_lifecycle_accepted_state_visibility.md` - Complete implementation plan with terminology changes, UI modifications, backend orchestration, and worker context enhancements

## Goals

1. **Accept entire plans, not individual proposals** - Replace per-proposal selection with plan-level acceptance that preserves the dependency graph
2. **Make accepted plans visible** - Add History section to PlanBrowser showing accepted and archived plans with read-only access
3. **Expose dependency context to workers** - Workers can see which tasks block them and which tasks they unblock
4. **Just-in-time task orchestration** - Automatically block/unblock tasks based on their dependency relationships during execution

## Dependencies

### Phase 61 (Migration Test Split) - Required

| Dependency | Why Needed |
|------------|------------|
| Clean test infrastructure | This phase adds new migrations for terminology changes |

### Phase 10 (Ideation) - Foundation

| Dependency | Why Needed |
|------------|------------|
| Ideation view and session management | This phase modifies the ideation UI and session/plan concepts |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/planning_lifecycle_accepted_state_visibility.md`
2. Understand the terminology changes and task dependency graph
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
2. **Read the ENTIRE implementation plan** at `specs/plans/planning_lifecycle_accepted_state_visibility.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Rename Converted status to Accepted with database migration",
    "plan_section": "Task 0: Terminology Migration (Converted → Accepted)",
    "blocking": [2, 3, 5, 6, 8],
    "blockedBy": [],
    "atomic_commit": "feat(ideation): rename Converted status to Accepted with migration",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 0'",
      "Update SessionStatus enum in src-tauri/src/domain/entities/ideation/types.rs: Converted → Accepted",
      "Update all Rust code referencing SessionStatus::Converted to use SessionStatus::Accepted",
      "Create migration in src-tauri/src/infrastructure/sqlite/migrations/ to update existing 'converted' rows to 'accepted'",
      "Update src/types/ideation.ts: change 'converted' to 'accepted' in SessionStatus type",
      "Update all frontend code referencing 'converted' status to use 'accepted'",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): rename Converted status to Accepted with migration"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add useDependencyGraphComplete hook for graph validation",
    "plan_section": "Task 10: Track Dependency Graph Completeness",
    "blocking": [3],
    "blockedBy": [],
    "atomic_commit": "feat(hooks): add useDependencyGraphComplete hook",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 10'",
      "Create src/hooks/useDependencyGraphComplete.ts",
      "Implement hook to check: all proposals have tier, no dangling dependencies, valid DAG",
      "Export from src/hooks/index.ts",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): add useDependencyGraphComplete hook"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Replace per-proposal selection with Accept Plan button in toolbar",
    "plan_section": "Task 1: ProposalsToolbar.tsx - Accept Plan Button",
    "blocking": [7],
    "blockedBy": [1, 2],
    "atomic_commit": "feat(ideation): replace per-proposal selection with Accept Plan button",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 1'",
      "Modify src/components/Ideation/ProposalsToolbar.tsx",
      "Remove proposal selection state and checkbox logic",
      "Add useDependencyGraphComplete hook usage",
      "Replace Apply button with 'Accept Plan' button disabled when graph incomplete",
      "Add visual feedback (AlertCircle tooltip) when graph incomplete",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): replace per-proposal selection with Accept Plan button"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Rename SessionBrowser to PlanBrowser with history section",
    "plan_section": "Task 2: PlanBrowser.tsx (rename from SessionBrowser)",
    "blocking": [5],
    "blockedBy": [1],
    "atomic_commit": "refactor(ideation): rename SessionBrowser to PlanBrowser with history section",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 2'",
      "Rename src/components/Ideation/SessionBrowser.tsx to PlanBrowser.tsx",
      "Update all imports across the codebase (grep for SessionBrowser)",
      "Add historyPlans prop to interface",
      "Add collapsible History section with status badges (accepted/archived)",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(ideation): rename SessionBrowser to PlanBrowser with history section"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Rename IdeationView to PlanningView with read-only support",
    "plan_section": "Task 3: PlanningView.tsx (rename from IdeationView)",
    "blocking": [],
    "blockedBy": [1, 4],
    "atomic_commit": "refactor(ideation): rename IdeationView to PlanningView with read-only support",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 3'",
      "Rename src/views/IdeationView.tsx to PlanningView.tsx",
      "Update all imports across the codebase (grep for IdeationView)",
      "Add activePlans and historyPlans useMemo computations",
      "Add isReadOnly computed from plan status",
      "Pass historyPlans to PlanBrowser and isReadOnly to ProposalsToolbar",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(ideation): rename IdeationView to PlanningView with read-only support"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Add task link to ProposalCard for accepted plans",
    "plan_section": "Task 4: ProposalCard.tsx - Show Task Link (Accepted Plans)",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(ideation): add task link to proposals in accepted plans",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 4'",
      "Modify src/components/Ideation/ProposalCard.tsx",
      "Add 'View Task →' button when proposal.createdTaskId exists",
      "Implement navigateToTask to switch to kanban and select task",
      "Disable edit/delete menu when plan is accepted",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add task link to proposals in accepted plans"
    ],
    "passes": true
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Remove sort dropdown and selection checkboxes from proposal list",
    "plan_section": "Task 6: Remove Redundant Sort Control",
    "blocking": [],
    "blockedBy": [3],
    "atomic_commit": "refactor(ideation): remove sort dropdown, tier order is canonical",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 6'",
      "Remove sort dropdown from ProposalsToolbar.tsx",
      "Remove selection checkboxes from TieredProposalList.tsx",
      "Keep filter by status if present",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(ideation): remove sort dropdown, tier order is canonical"
    ],
    "passes": true
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Rename ApplyModal to AcceptModal with full plan accept",
    "plan_section": "Task 5: AcceptModal.tsx (rename from ApplyModal)",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "refactor(ideation): rename ApplyModal to AcceptModal with full plan accept",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 5'",
      "Rename src/components/Ideation/ApplyModal.tsx to AcceptModal.tsx",
      "Update all imports across the codebase (grep for ApplyModal)",
      "Remove proposal selection checkboxes",
      "Show all proposals that will become tasks",
      "Keep dependency graph preview and target column selector",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(ideation): rename ApplyModal to AcceptModal with full plan accept"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "backend",
    "description": "Add dependency context fields to TaskContext for worker",
    "plan_section": "Task 8: Expose Dependency Context to Worker",
    "blocking": [10, 11],
    "blockedBy": [],
    "atomic_commit": "feat(backend): add dependency context to TaskContext for worker",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 8'",
      "Update src-tauri/src/domain/entities/task_context.rs: add blocked_by, blocks, tier, priority_score fields",
      "Update src-tauri/src/application/task_context_service.rs to query task dependencies",
      "Add priority_score to TaskProposalSummary in proposal.rs",
      "Update MCP response to include new fields",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add dependency context to TaskContext for worker"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "agent",
    "description": "Update worker agent prompt with dependency context guidance",
    "plan_section": "Task 9: Update Worker Agent Prompt",
    "blocking": [],
    "blockedBy": [9],
    "atomic_commit": "docs(plugin): add dependency context guidance to worker agent",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 9'",
      "Update ralphx-plugin/agents/worker.md",
      "Add 'Task Dependencies' section explaining blocked_by, blocks, tier fields",
      "Add decision flow: check blocked_by first, stop if not empty",
      "Explain tier context for priority understanding",
      "Commit: docs(plugin): add dependency context guidance to worker agent"
    ],
    "passes": false
  },
  {
    "id": 11,
    "category": "backend",
    "description": "Implement automatic task blocking/unblocking based on dependencies",
    "plan_section": "Task 11: Just-in-Time Execution Orchestration",
    "blocking": [],
    "blockedBy": [9],
    "atomic_commit": "feat(backend): add automatic blocking/unblocking based on dependency graph",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 11'",
      "Update src-tauri/src/application/apply_service.rs: set initial Blocked/Ready based on deps on plan accept",
      "Update src-tauri/src/domain/services/transition_handler.rs: on Done transition, check and unblock dependents",
      "Add get_blocking_tasks and get_incomplete_blockers helper functions",
      "Emit task:unblocked event when task becomes ready",
      "Update blocked_reason with remaining blocker names",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add automatic blocking/unblocking based on dependency graph"
    ],
    "passes": false
  },
  {
    "id": 12,
    "category": "frontend",
    "description": "Add read-only chat mode for accepted plans",
    "plan_section": "Task 7: Read-only Chat for Accepted Plans",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(plugin): add read-only chat mode for accepted plans",
    "steps": [
      "Read specs/plans/planning_lifecycle_accepted_state_visibility.md section 'Task 7'",
      "Identify chat spawn logic in frontend (likely in chat hooks or spawn utilities)",
      "Add plan status check before spawning chat",
      "If plan status is 'accepted': add --allowed-tools flag with read-only tool list",
      "Read-only tools: get_proposal, get_plan, get_session, list_* tools",
      "Exclude: update_proposal, remove_proposal, add_proposal, create_proposal",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(plugin): add read-only chat mode for accepted plans"
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
| **Accept entire plan, not individual proposals** | Prevents dependency graph breakage from partial applies, clearer mental model |
| **History section instead of hiding accepted plans** | Users can see what was accepted, navigate to created tasks, continue read-only conversations |
| **Read-only MCP access via --allowed-tools** | Simpler than server-side blocking, leverages existing Claude CLI capability |
| **Automatic blocking/unblocking on task completion** | Workers shouldn't start blocked tasks; system handles orchestration |
| **Terminology: Ideation→Planning, Session→Plan, Apply→Accept** | Clearer intent, simpler mental model |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Migration updates existing 'converted' rows to 'accepted'
- [ ] TaskContext includes blocked_by, blocks, tier, priority_score
- [ ] On plan accept, tasks with blockers get Blocked status
- [ ] On task completion, dependent tasks auto-unblock when all blockers done

### Frontend - Run `npm run test`
- [ ] useDependencyGraphComplete returns false for incomplete graphs
- [ ] Accept Plan button disabled when graph incomplete
- [ ] History section shows accepted and archived plans
- [ ] ProposalCard shows task link for accepted plans

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Create plan with 3 proposals + dependencies, verify Accept button disabled until graph complete
- [ ] Accept plan → all proposals become tasks with preserved dependencies
- [ ] Plan moves to History section immediately
- [ ] Click accepted plan → view read-only with task links
- [ ] Chat works in accepted plan but modify tools blocked
- [ ] Accept plan with tier 2+ tasks → verify blocked status and auto-unblock on blocker completion
- [ ] Toast notification appears when task becomes ready

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Accept Plan button wired to onAcceptPlan handler
- [ ] PlanBrowser receives historyPlans prop and renders History section
- [ ] ProposalCard View Task button navigates to task
- [ ] TaskContext MCP response includes dependency fields
- [ ] TransitionHandler triggers unblock check on Done transition

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
