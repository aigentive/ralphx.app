# RalphX - Phase 72: Graph Node Kanban Styling Parity

## Overview

This phase redesigns the Task Graph nodes to match the visual styling of Kanban cards, creating visual consistency across the two primary task views. The current graph nodes use a simpler styling approach (solid backgrounds, full-border status colors, hover scaling) that doesn't match the premium Kanban card design (glass morphism, priority stripes, activity dots, pulsing animations).

By achieving visual parity, users will experience a cohesive design language whether viewing tasks in the Kanban board or the dependency graph.

**Reference Plan:**
- `specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md` - Detailed gap analysis and implementation tasks for visual parity

## Goals

1. Apply glass morphism surface styling to graph nodes (backdrop blur, translucent backgrounds, soft shadows)
2. Add left priority stripe indicators matching Kanban card priority colors
3. Implement activity dots and pulsing border animations for active states (executing, reviewing)
4. Relocate status badges to top-right corner with Kanban-matching styling
5. Update hover/selection states to match Kanban behavior (no scale, Finder-like selection)

## Dependencies

### Phase 67 (Task Graph View) - Required

| Dependency | Why Needed |
|------------|------------|
| TaskNode.tsx | Base node component to be restyled |
| TaskNodeCompact.tsx | Compact node variant to be restyled |
| nodeStyles.ts | Status color definitions to extend with priority colors |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md`
2. Understand the Kanban card styling patterns and gap analysis
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
4. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `refactor:`

**Task Execution Order:**
- Task 1 is BLOCKING and must complete first
- Tasks 2-7 can be executed in parallel after Task 1 completes
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Apply glass morphism surface styling to graph nodes",
    "plan_section": "Task 1: Update Base Node Surface Styling",
    "blocking": [2, 3, 4, 5, 6, 7],
    "blockedBy": [],
    "atomic_commit": "refactor(task-graph): apply glass morphism to node surface",
    "steps": [
      "Read specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md section 'Task 1'",
      "Update nodeStyles.ts base styles with glass morphism: background hsla(220 10% 14% / 0.85), backdropFilter blur(12px) saturate(150%)",
      "Change border to subtle divider: 1px solid hsla(220 10% 100% / 0.06)",
      "Add consistent soft shadow: 0 2px 8px hsla(220 10% 0% / 0.25)",
      "Update TaskNode.tsx to use new surface styles",
      "Update TaskNodeCompact.tsx to use new surface styles",
      "Remove hover:scale-105/hover:scale-110 effects (keep hover:shadow-lg)",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(task-graph): apply glass morphism to node surface"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add left priority stripe to nodes",
    "plan_section": "Task 2: Add Left Priority Stripe",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(task-graph): add left priority stripe to nodes",
    "steps": [
      "Read specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md section 'Task 2'",
      "Add priority color constants to nodeStyles.ts (P1: red, P2: deep orange, P3: accent orange, P4: gray)",
      "Add getPriorityStripeColor() helper function",
      "Update TaskNode.tsx to apply borderLeft: 3px solid {priorityColor}",
      "Update TaskNodeCompact.tsx with same priority stripe",
      "Verify TaskNodeData type includes priority field (add if missing)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add left priority stripe to nodes"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add activity dots for active states",
    "plan_section": "Task 3: Add Activity Dots for Active States",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(task-graph): add activity dots for active states",
    "steps": [
      "Read specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md section 'Task 3'",
      "Create ActivityDots component or inline JSX for 3-dot indicator",
      "Add to TaskNode.tsx for executing, re_executing, reviewing states",
      "Position in top-right corner (absolute positioning)",
      "Apply staggered bounce animation (1.4s, delays 0s/0.2s/0.4s)",
      "Use orange color for executing, blue for reviewing",
      "Add same dots to TaskNodeCompact.tsx",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add activity dots for active states"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add pulsing border animation for active states",
    "plan_section": "Task 4: Add Pulsing Border Animation",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(task-graph): add pulsing border animation for active states",
    "steps": [
      "Read specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md section 'Task 4'",
      "Check globals.css for existing executing/reviewing pulse animations",
      "If missing, add @keyframes for executing-pulse (orange glow, 2s) and reviewing-pulse (blue glow, 2s)",
      "Update nodeStyles.ts getNodeStyle() to return animation property for executing/re_executing/reviewing",
      "Apply animation in TaskNode.tsx style prop",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add pulsing border animation for active states"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Relocate and restyle status badge",
    "plan_section": "Task 5: Relocate and Restyle Status Badge",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "refactor(task-graph): relocate status badge to top-right corner",
    "steps": [
      "Read specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md section 'Task 5'",
      "In TaskNode.tsx, relocate status badge from bottom to top-right corner",
      "Use absolute positioning within the node container",
      "Reduce font size to 9px (text-[9px])",
      "Use px-1.5 py-px padding",
      "Apply translucent colored backgrounds: rgba({statusColor}, 0.2)",
      "Add appropriate Lucide icons for each status (Clock, Loader2, CheckCircle, etc.)",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(task-graph): relocate status badge to top-right corner"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Make connection handles more subtle",
    "plan_section": "Task 6: Update Connection Handles",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "refactor(task-graph): make connection handles more subtle",
    "steps": [
      "Read specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md section 'Task 6'",
      "In TaskNode.tsx, update Handle components with smaller size: !w-1.5 !h-1.5",
      "Lower default opacity: !opacity-50",
      "Add hover state: hover:!opacity-100",
      "Add transition for smooth opacity change",
      "Apply same handle styling to TaskNodeCompact.tsx",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(task-graph): make connection handles more subtle"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Update hover and selection states",
    "plan_section": "Task 7: Update Hover/Selection States",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "refactor(task-graph): update hover and selection states",
    "steps": [
      "Read specs/plans/redesign_graph_nodes_to_match_kanban_card_styling.md section 'Task 7'",
      "In TaskNode.tsx, remove hover:scale-105/hover:scale-110 classes",
      "Keep hover:shadow-lg for elevation effect on hover",
      "Update selected state to use Finder-like blue selection:",
      "  - background: hsla(220 60% 50% / 0.25)",
      "  - border: 1px solid hsla(220 60% 60% / 0.3)",
      "Apply same changes to TaskNodeCompact.tsx",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(task-graph): update hover and selection states"
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
| **Shared priority colors** | Use same priority stripe colors as Kanban cards for visual consistency |
| **CSS animations over JS** | Use CSS keyframes for pulsing animations (already exist in globals.css for Kanban) |
| **Inline styles for dynamic values** | Status-dependent styles (colors, animations) applied via style prop, static styles via Tailwind |
| **Both node variants updated** | TaskNode and TaskNodeCompact both need styling updates for consistency |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run lint && npm run typecheck`
- [ ] All linting passes
- [ ] All type checking passes

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Open graph view with tasks in various states (backlog, executing, reviewing, approved)
- [ ] Compare visual parity with Kanban cards side-by-side
- [ ] Verify priority stripes match Kanban priority indicators (P1-P4)
- [ ] Verify activity dots animate for executing/reviewing nodes
- [ ] Verify pulsing border animations work for active states
- [ ] Verify hover states don't scale, just shadow elevation
- [ ] Verify selection states match Finder-like blue selection
- [ ] Verify handles remain functional for edge connections

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Glass morphism styles applied to all node instances
- [ ] Priority stripe visible when priority data exists
- [ ] Activity dots render for executing/re_executing/reviewing states
- [ ] Pulsing animation active for executing/reviewing nodes
- [ ] Status badge positioned in top-right corner
- [ ] Handles remain interactive for drag-to-connect

**Common failure modes to check:**
- [ ] No missing backdrop-filter browser prefixes (WebkitBackdropFilter)
- [ ] No animation names typos (must match globals.css keyframes)
- [ ] No priority stripe when priority data is undefined
- [ ] Selection ring not conflicting with priority stripe

See `.claude/rules/gap-verification.md` for full verification workflow.
