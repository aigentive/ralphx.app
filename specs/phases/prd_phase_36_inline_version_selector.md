# RalphX - Phase 36: Inline Version Selector for Plan History

## Overview

Replace the modal-based plan version history with an inline version selector dropdown in `PlanDisplay`. This provides a better UX by allowing users to browse plan versions without leaving the ideation context. Additionally, auto-expand the plan when there are no proposals, making the plan content more discoverable.

**Reference Plan:**
- `specs/plans/inline_version_selector_for_plan_history.md` - Detailed implementation plan with code snippets and UI patterns

## Goals

1. Inline version selection for plan history (no modal)
2. Proper markdown rendering for historical versions
3. Auto-expand plan when no proposals exist
4. Net reduction in code complexity (~100 LOC removed)

## Dependencies

### Phase 35 (Welcome Screen & Project Creation Redesign) - Required

| Dependency | Why Needed |
|------------|------------|
| Ideation UI | Plan display is part of ideation view |
| Artifact API | Version fetching uses existing artifact API |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/inline_version_selector_for_plan_history.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/inline_version_selector_for_plan_history.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Add inline version selector dropdown to PlanDisplay",
    "plan_section": "1. PlanDisplay.tsx — Add Version Selector",
    "blocking": [2, 3, 4],
    "blockedBy": [],
    "atomic_commit": "feat(ideation): add inline version selector to PlanDisplay",
    "steps": [
      "Read specs/plans/inline_version_selector_for_plan_history.md section '1. PlanDisplay.tsx — Add Version Selector'",
      "Add state: selectedVersion, historicalContent, isLoadingVersion",
      "Add useEffect to fetch historical version via artifactApi.getAtVersion when selection changes",
      "Replace History button with DropdownMenu using ConversationSelector pattern",
      "Add version banner with 'Back to latest' button when viewing historical",
      "Update content display to use historicalContent ?? planContent",
      "Remove onViewHistory prop from component interface",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add inline version selector to PlanDisplay"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Remove PlanHistoryDialog from IdeationView and add auto-expand",
    "plan_section": "2. IdeationView.tsx — Remove Modal + Auto-expand",
    "blocking": [4],
    "blockedBy": [1],
    "atomic_commit": "refactor(ideation): remove PlanHistoryDialog, add auto-expand",
    "steps": [
      "Read specs/plans/inline_version_selector_for_plan_history.md section '2. IdeationView.tsx — Remove Modal + Auto-expand'",
      "Remove PlanHistoryDialog import",
      "Remove planHistoryDialog, handleViewHistoricalPlan, handleClosePlanHistoryDialog from destructured handlers",
      "Delete <PlanHistoryDialog> render block",
      "Remove onViewHistory prop from <PlanDisplay>",
      "Add useEffect for auto-expand: setIsPlanExpanded(true) when planArtifact exists and proposals.length === 0",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(ideation): remove PlanHistoryDialog, add auto-expand"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Remove plan history modal state from useIdeationHandlers",
    "plan_section": "3. useIdeationHandlers.ts — Remove Modal State",
    "blocking": [4],
    "blockedBy": [1],
    "atomic_commit": "refactor(ideation): remove plan history modal state",
    "steps": [
      "Read specs/plans/inline_version_selector_for_plan_history.md section '3. useIdeationHandlers.ts — Remove Modal State'",
      "Remove planHistoryDialog state",
      "Remove handleViewHistoricalPlan callback",
      "Remove handleClosePlanHistoryDialog callback",
      "Remove from return object",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(ideation): remove plan history modal state"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Delete unused PlanHistoryDialog.tsx file",
    "plan_section": "4. Delete PlanHistoryDialog.tsx",
    "blocking": [],
    "blockedBy": [2, 3],
    "atomic_commit": "chore(ideation): delete unused PlanHistoryDialog",
    "steps": [
      "Read specs/plans/inline_version_selector_for_plan_history.md section '4. Delete PlanHistoryDialog.tsx'",
      "Verify no remaining imports of PlanHistoryDialog in codebase",
      "Delete src/components/ideation/PlanHistoryDialog.tsx",
      "Run npm run lint && npm run typecheck",
      "Commit: chore(ideation): delete unused PlanHistoryDialog"
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
| **Inline dropdown over modal** | Better UX - users stay in context, version switching is faster |
| **ConversationSelector pattern** | Consistent with existing UI patterns, proven dropdown design |
| **Auto-expand when no proposals** | Makes plan content discoverable, reduces clicks for new sessions |
| **Delete modal entirely** | Clean removal, reduces code complexity by ~171 LOC |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] Version selector renders when plan.metadata.version > 1
- [ ] Historical content fetches correctly via artifactApi.getAtVersion
- [ ] Markdown renders properly (not plain text)

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Version selector appears when plan has version > 1
- [ ] Selecting a historical version fetches and displays content inline
- [ ] Markdown renders correctly (headings, code blocks, lists)
- [ ] "Back to latest" button returns to current version
- [ ] Auto-expand works when no proposals exist but plan exists
- [ ] Modal fully removed (no PlanHistoryDialog references anywhere)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Version dropdown triggers fetch on selection change
- [ ] Historical content displays in existing markdown renderer
- [ ] Auto-expand effect runs when conditions are met (planArtifact && !proposals)

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
