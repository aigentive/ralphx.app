# RalphX - Phase 40: Tiered Proposal View

## Overview

The current proposals display shows dependency information as simple count badges (`←3 →2`), which tells users *how many* dependencies exist but fails to communicate ordering, relationships, and plan flow. This phase reorganizes proposals into **execution tiers** based on topological depth, replaces count badges with inline dependency names (including Phase 39's reason text), and adds visual tier connectors to show the implementation flow.

**Reference Plan:**
- `specs/plans/enhanced_proposal_dependencies_ux.md` - Detailed design for tiered view with inline dependency details

## Goals

1. Group proposals by topological tier (Foundation → Core → Integration) to show execution order
2. Replace dependency count badges with actual proposal names and reasons
3. Add collapsible tier sections with auto-collapse for large plans (5+ proposals)
4. Add visual tier connectors highlighting the critical path

## Dependencies

### Phase 39 (Dependency Reason Field) - Required

| Dependency | Why Needed |
|------------|------------|
| `reason` field on edges | Inline dependency display needs reason text to explain WHY dependencies exist |
| `getDependencyReason()` helper | Hook needs to retrieve reasons for display in ProposalCard |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/enhanced_proposal_dependencies_ux.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/enhanced_proposal_dependencies_ux.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Add useDependencyTiers hook for topological grouping",
    "plan_section": "Task 1: Compute Topological Tiers",
    "blocking": [5],
    "blockedBy": [],
    "atomic_commit": "feat(hooks): add useDependencyTiers hook for topological grouping",
    "steps": [
      "Read specs/plans/enhanced_proposal_dependencies_ux.md section 'Task 1'",
      "Add useDependencyTiers() function to src/hooks/useDependencyGraph.ts",
      "Accept DependencyGraph as input, return Map<proposalId, tierLevel>",
      "Tier 0 = no dependencies (inDegree === 0)",
      "Tier N = max(tier of dependencies) + 1",
      "Handle cycles gracefully (assign to highest possible tier)",
      "Add unit tests for tier computation and cycle handling",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): add useDependencyTiers hook for topological grouping"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Verify DependencyGraphEdge has reason field (Phase 39 provides)",
    "plan_section": "Task 2: Update DependencyGraph Types for Reasons",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "chore(ideation): verify edge reason types from Phase 39",
    "steps": [
      "Read specs/plans/enhanced_proposal_dependencies_ux.md section 'Task 2'",
      "Verify src/types/ideation.ts has reason?: string on DependencyGraphEdge",
      "Verify src/api/ideation.schemas.ts includes reason in edge schema",
      "Verify src/api/ideation.transforms.ts passes reason through",
      "If missing, add the reason field (Phase 39 should have done this)",
      "Verify useDependencyGraph provides getDependencyReason(fromId, toId) helper",
      "Run npm run lint && npm run typecheck",
      "Commit: chore(ideation): verify edge reason types from Phase 39"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Create ProposalTierGroup collapsible tier component",
    "plan_section": "Task 3: Create ProposalTierGroup Component",
    "blocking": [5],
    "blockedBy": [],
    "atomic_commit": "feat(ideation): create ProposalTierGroup collapsible tier component",
    "steps": [
      "Read specs/plans/enhanced_proposal_dependencies_ux.md section 'Task 3'",
      "Create src/components/Ideation/ProposalTierGroup.tsx",
      "Props: tier number, label, children, defaultCollapsed, proposalCount",
      "Render collapsible section with ChevronDown/ChevronRight icon",
      "Header format: 'Tier {N}: {label}' with expand/collapse toggle",
      "Labels: 'Foundation' (0), 'Core' (1), 'Integration' (2+)",
      "Style: border-left accent (#ff6b35), subtle indentation",
      "Auto-collapse when proposalCount >= 5",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): create ProposalTierGroup collapsible tier component"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Display dependency names and reasons in ProposalCard",
    "plan_section": "Task 4: Enhance ProposalCard Dependency Display",
    "blocking": [5],
    "blockedBy": [2],
    "atomic_commit": "feat(ideation): display dependency names and reasons in ProposalCard",
    "steps": [
      "Read specs/plans/enhanced_proposal_dependencies_ux.md section 'Task 4'",
      "Add new prop to ProposalCard: dependsOnProposals?: Array<{ id: string; title: string; reason?: string }>",
      "Replace count badge (←N) with inline names: '← {title1}, {title2}'",
      "Truncate with '+N more' if more than 2 dependencies",
      "Add Tooltip showing full list with names and reasons",
      "Add expandable section (chevron) for full dependency details + reasons",
      "Keep existing blocksCount display as simple badge",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): display dependency names and reasons in ProposalCard"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Create TieredProposalList orchestration component",
    "plan_section": "Task 5: Create TieredProposalList Component",
    "blocking": [6],
    "blockedBy": [1, 3, 4],
    "atomic_commit": "feat(ideation): create TieredProposalList orchestration component",
    "steps": [
      "Read specs/plans/enhanced_proposal_dependencies_ux.md section 'Task 5'",
      "Create src/components/Ideation/TieredProposalList.tsx",
      "Props: proposals, dependencyGraph, selectedId, highlightedIds, onSelect, onEdit, etc.",
      "Use useDependencyTiers() to compute tier assignments",
      "Group proposals by tier level",
      "Render ProposalTierGroup for each tier (0, 1, 2+)",
      "Pass dependsOnProposals to each ProposalCard by looking up edge targets",
      "Maintain sortOrder as tiebreaker within same tier",
      "Preserve selection and highlight functionality",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): create TieredProposalList orchestration component"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Integrate TieredProposalList into IdeationView",
    "plan_section": "Task 6: Replace List in IdeationView",
    "blocking": [7],
    "blockedBy": [5],
    "atomic_commit": "feat(ideation): integrate TieredProposalList into IdeationView",
    "steps": [
      "Read specs/plans/enhanced_proposal_dependencies_ux.md section 'Task 6'",
      "Open src/components/Ideation/IdeationView.tsx",
      "Replace sortedProposals.map() rendering with <TieredProposalList />",
      "Pass all existing props: selection, highlights, handlers",
      "Remove sortOrder-based sorting (tiers now define primary order)",
      "Keep drag-to-reorder functionality WITHIN tiers",
      "Verify critical path highlighting still works",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): integrate TieredProposalList into IdeationView"
    ],
    "passes": true
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Add SVG tier connectors with critical path highlight",
    "plan_section": "Task 7: Style Tier Connectors",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "feat(ideation): add SVG tier connectors with critical path highlight",
    "steps": [
      "Read specs/plans/enhanced_proposal_dependencies_ux.md section 'Task 7'",
      "Add SVG connector rendering to TieredProposalList.tsx",
      "Draw subtle dashed lines between tier sections",
      "Use var(--border-subtle) for normal connectors",
      "Use #ff6b35 solid for critical path connectors",
      "Keep styling minimal - visual rhythm, not clutter",
      "Test with various proposal counts and tier distributions",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add SVG tier connectors with critical path highlight"
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
| **Replace flat list entirely** | Toggle adds complexity; tiered view is strictly better for understanding dependencies |
| **Auto-collapse at 5+ proposals** | Keeps view manageable while showing small tiers fully expanded |
| **sortOrder as tiebreaker within tier** | Preserves user control over ordering while respecting dependency hierarchy |
| **SVG connectors between tiers only** | Connecting individual proposals would create visual clutter |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] useDependencyTiers correctly assigns tier 0 to nodes with no dependencies
- [ ] useDependencyTiers correctly computes tier N = max(deps) + 1
- [ ] useDependencyTiers handles cycles without crashing

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Create ideation session with complex dependencies (5+ proposals, multiple tiers)
- [ ] Verify proposals group correctly by tier level
- [ ] Verify tier labels display correctly (Foundation, Core, Integration)
- [ ] Verify tiers with 5+ proposals auto-collapse
- [ ] Click to expand collapsed tier
- [ ] Verify dependency names display inline (not counts)
- [ ] Hover over dependency to see tooltip with reason text
- [ ] Expand dependency section to see full details
- [ ] Verify critical path highlighting spans across tiers
- [ ] Verify selection and highlighting still work
- [ ] Verify drag-to-reorder works within tiers

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (IdeationView renders TieredProposalList)
- [ ] TieredProposalList is imported AND rendered (not behind disabled flag)
- [ ] useDependencyTiers hook is called with actual dependency graph
- [ ] ProposalCard receives dependsOnProposals prop with real data
- [ ] Tier connectors render with correct styling

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
