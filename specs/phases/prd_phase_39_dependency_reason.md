# RalphX - Phase 39: Dependency Reason Field

## Overview

This phase persists the AI's dependency reasoning so users see *why* dependencies exist when hovering over dependency badges. Currently, `DependencySuggestion` receives `reason: Option<String>` from the dependency-suggester agent, but it's discarded. This phase stores and displays reasons like "API needs database tables to exist".

**Reference Plan:**
- `specs/plans/add_reason_field_to_proposal_dependencies.md` - Detailed implementation steps for adding reason field across all layers

## Goals

1. Add `reason` column to `proposal_dependencies` table via database migration
2. Thread reason through repository trait, SQLite implementation, and HTTP layer
3. Display dependency reasons in ProposalCard tooltips in the Ideation UI

## Dependencies

### Phase 38 (Dependency Graph & Priority Assessment Integration) - Required

| Dependency | Why Needed |
|------------|------------|
| Dependency graph infrastructure | The reason field extends the existing dependency storage system |
| `DependencySuggestion` type with `reason` field | The input type already receives reason from the AI agent |
| ProposalCard dependency badges | UI component that will display the reasons |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/add_reason_field_to_proposal_dependencies.md`
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
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/add_reason_field_to_proposal_dependencies.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add database migration for reason column",
    "plan_section": "Step 1: Database Migration",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "feat(migrations): add reason column to proposal_dependencies",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 1: Database Migration'",
      "Create new file src-tauri/src/infrastructure/sqlite/migrations/v2_add_dependency_reason.rs",
      "Implement migrate() using helpers::add_column_if_not_exists for 'reason' TEXT DEFAULT NULL",
      "Update migrations/mod.rs: add mod v2_add_dependency_reason",
      "Register in MIGRATIONS array (version: 2, name: 'add_dependency_reason')",
      "Bump SCHEMA_VERSION to 2",
      "Run cargo test to verify migration works",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: feat(migrations): add reason column to proposal_dependencies"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Update ProposalDependencyRepository trait with reason parameter",
    "plan_section": "Step 2: Repository Trait",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(domain): add reason parameter to ProposalDependencyRepository",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 2: Repository Trait'",
      "Update add_dependency signature to accept reason: Option<&str>",
      "Update get_all_for_session return type to Vec<(TaskProposalId, TaskProposalId, Option<String>)>",
      "Update mock implementation in tests to match new signatures",
      "Run cargo test",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: feat(domain): add reason parameter to ProposalDependencyRepository"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Implement reason storage in SQLite repository",
    "plan_section": "Step 3: SQLite Repository Implementation",
    "blocking": [4],
    "blockedBy": [1, 2],
    "atomic_commit": "feat(sqlite): implement reason storage in proposal_dependency_repo",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 3: SQLite Repository Implementation'",
      "Update add_dependency to accept reason: Option<&str> and include in INSERT",
      "Update get_all_for_session to SELECT reason column and return 3-tuple",
      "Run cargo test",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: feat(sqlite): implement reason storage in proposal_dependency_repo"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Update HTTP handler to pass and return reason",
    "plan_section": "Step 4: HTTP Handler",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(http): pass dependency reason through API layer",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 4: HTTP Handler'",
      "Update apply_proposal_dependencies to pass suggestion.reason.as_deref() to add_dependency",
      "Update analyze_session_dependencies to include reason in edge response",
      "Run cargo test",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: feat(http): pass dependency reason through API layer"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Add reason field to HTTP response types",
    "plan_section": "Step 5: HTTP Response Types",
    "blocking": [6],
    "blockedBy": [4],
    "atomic_commit": "feat(http): add reason field to DependencyEdgeResponse",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 5: HTTP Response Types'",
      "Find DependencyEdgeResponse (or similar edge response struct) in types.rs",
      "Add pub reason: Option<String> field",
      "Run cargo test",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: feat(http): add reason field to DependencyEdgeResponse"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Update frontend Zod schemas for reason field",
    "plan_section": "Step 6: Frontend Schemas",
    "blocking": [7, 8],
    "blockedBy": [5],
    "atomic_commit": "feat(api): add reason to DependencyGraphEdgeResponseSchema",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 6: Frontend Schemas'",
      "Update DependencyGraphEdgeResponseSchema in src/api/ideation.schemas.ts",
      "Add reason: z.string().nullable()",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add reason to DependencyGraphEdgeResponseSchema"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Update frontend TypeScript types",
    "plan_section": "Step 7: Frontend Types",
    "blocking": [8],
    "blockedBy": [6],
    "atomic_commit": "feat(types): add reason to DependencyGraphEdge interface",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 7: Frontend Types'",
      "Update DependencyGraphEdge interface in src/types/ideation.ts",
      "Add reason?: string field",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(types): add reason to DependencyGraphEdge interface"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Update frontend transform to pass through reason",
    "plan_section": "Step 8: Frontend Transform",
    "blocking": [9],
    "blockedBy": [6, 7],
    "atomic_commit": "feat(transforms): pass through dependency reason",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 8: Frontend Transform'",
      "Update edges transform in src/api/ideation.transforms.ts to include reason",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(transforms): pass through dependency reason"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Update ProposalCard to display dependency reasons in tooltip",
    "plan_section": "Step 9: UI - ProposalCard Tooltip",
    "blocking": [10],
    "blockedBy": [8],
    "atomic_commit": "feat(ProposalCard): display dependency reasons in tooltip",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 9: UI - ProposalCard Tooltip'",
      "Add DependencyDetail interface: { proposalId: string; title: string; reason?: string }",
      "Add dependsOnDetails?: DependencyDetail[] to ProposalCard props",
      "Update tooltip content to show dependency titles and reasons",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ProposalCard): display dependency reasons in tooltip"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "frontend",
    "description": "Wire dependency details from IdeationView to ProposalCard",
    "plan_section": "Step 10: UI - IdeationView Data Flow",
    "blocking": [],
    "blockedBy": [9],
    "atomic_commit": "feat(IdeationView): build and pass dependency details to ProposalCard",
    "steps": [
      "Read specs/plans/add_reason_field_to_proposal_dependencies.md section 'Step 10: UI - IdeationView Data Flow'",
      "Update useMemo to build dependencyDetails map from edges with reasons",
      "Pass dependsOnDetails to each ProposalCard",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(IdeationView): build and pass dependency details to ProposalCard"
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
| **Use existing helpers for migration** | `add_column_if_not_exists` ensures idempotent migrations |
| **3-tuple return for get_all_for_session** | Simpler than introducing a new struct, consistent with existing pattern |
| **DependencyDetail interface in UI** | Clean separation of concerns, props are explicit about what data they need |
| **Reason optional throughout** | Backward compatible - existing dependencies without reasons still work |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Migration adds `reason` column to `proposal_dependencies` table
- [ ] `add_dependency` accepts and stores reason
- [ ] `get_all_for_session` returns reason in tuples

### Frontend - Run `npm run test`
- [ ] Zod schema validates reason as nullable string
- [ ] Transform passes reason through correctly

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Create ideation session with proposals
- [ ] Run dependency-suggester agent with proposals that have clear dependency relationships
- [ ] Hover over dependency badge on ProposalCard
- [ ] Tooltip shows proposal titles AND reasons (e.g., "API Service: Needs database schema to exist")

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Reason flows from agent → HTTP handler → database → HTTP response → frontend
- [ ] ProposalCard receives dependsOnDetails prop with reasons populated
- [ ] Tooltip renders reasons when present, gracefully handles missing reasons

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
