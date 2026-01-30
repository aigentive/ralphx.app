# RalphX - Phase 34: Proposal Edit Modal Redesign

## Overview

Transform the current utilitarian `ProposalEditModal` into a **10x designer-level** experience that embodies the "Refined Studio" aesthetic while maintaining full functionality. This phase implements the "Editorial Blueprint" design concept with glass morphism, micro-interactions, and spatial asymmetry to create a premium editing experience.

**Reference Plan:**
- `specs/plans/proposal_edit_modal_redesign.md` - Complete design specifications, component details, and animation specs

## Goals

1. **Premium Visual Design** - Glass morphism, layered depth, and refined typography
2. **Spatial Innovation** - Two-column metadata layout breaking vertical monotony
3. **Delightful Interactions** - Staggered animations, hover reveals, visual complexity selector
4. **Maintained Functionality** - All existing features work, tests pass, accessibility preserved

## Dependencies

### Phase 10 (Ideation) - Required

| Dependency | Why Needed |
|------------|------------|
| ProposalEditModal component | Base component to redesign |
| Proposal CRUD operations | Must continue working after redesign |
| Ideation types (TaskProposal, Complexity) | Type definitions used by component |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/proposal_edit_modal_redesign.md`
2. Understand the design specifications and component structure
3. Reference the color palette and animation timings
4. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Verify visual appearance matches design specs
4. Run linters for modified code only: `npm run lint && npm run typecheck`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/proposal_edit_modal_redesign.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Expand modal width and add enhanced header with subtitle",
    "plan_section": "Task 1: Expand modal and add header subtitle",
    "blocking": [2, 4, 6],
    "blockedBy": [],
    "atomic_commit": "feat(ideation): expand modal width and add header subtitle",
    "steps": [
      "Read specs/plans/proposal_edit_modal_redesign.md section 'Task 1'",
      "Expand DialogContent from max-w-lg to max-w-2xl",
      "Add subtitle 'Refine your task proposal' in muted text below title",
      "Wrap Edit3 icon in orange background pill (bg-[#ff6b35]/10 rounded-full p-1.5)",
      "Add ambient glow styles to modal background",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): expand modal width and add header subtitle"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Create two-column metadata panel with glass effect",
    "plan_section": "Task 2: Create two-column metadata section",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(ideation): create two-column metadata panel",
    "steps": [
      "Read specs/plans/proposal_edit_modal_redesign.md section 'Task 2'",
      "Create GlassCard wrapper component (inline) with frosted glass styling",
      "Implement CSS Grid layout: grid-cols-[1fr_auto_1fr]",
      "Place Category + Priority Override dropdowns in left column",
      "Add subtle vertical divider (1px, rgba(255,255,255,0.08))",
      "Right column placeholder for ComplexitySelector",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): create two-column metadata panel"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Implement visual 5-dot ComplexitySelector component",
    "plan_section": "Task 3: Implement ComplexitySelector visual component",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "feat(ideation): add visual 5-dot complexity selector",
    "steps": [
      "Read specs/plans/proposal_edit_modal_redesign.md section 'Task 3'",
      "Create inline ComplexitySelector component",
      "Render 5 circles for trivial→very_complex with COMPLEXITIES array",
      "Orange fill (#ff6b35) for selected, transparent for others",
      "Add hover:scale-125 transition and cursor-pointer",
      "Show complexity label below dots (e.g., 'Moderate')",
      "Add tooltip on hover showing full label",
      "Wire to existing complexity state and setComplexity",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add visual 5-dot complexity selector"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Redesign steps list with circled numbers and glass container",
    "plan_section": "Task 4: Redesign steps list with EditableListItem",
    "blocking": [5],
    "blockedBy": [1],
    "atomic_commit": "feat(ideation): redesign steps list with circled numbers",
    "steps": [
      "Read specs/plans/proposal_edit_modal_redesign.md section 'Task 4'",
      "Wrap steps in GlassCard container",
      "Create circled number prefix function: CIRCLED_NUMBERS = ['①','②','③','④','⑤','⑥','⑦','⑧','⑨','⑩']",
      "Add circled number prefix before each step input",
      "Convert delete button to hover-reveal (opacity-0 group-hover:opacity-100)",
      "Replace header Plus button with centered dashed-border add button",
      "Style add button: border-dashed border-white/20 hover:border-[#ff6b35]/50",
      "Add elegant empty state with Layers icon and message",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): redesign steps list with circled numbers"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Redesign acceptance criteria list with checkmarks",
    "plan_section": "Task 5: Redesign acceptance criteria list",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(ideation): redesign criteria list with checkmarks",
    "steps": [
      "Read specs/plans/proposal_edit_modal_redesign.md section 'Task 5'",
      "Wrap acceptance criteria in GlassCard container (reuse from Task 4)",
      "Add checkmark prefix (✓ or Check icon) to each criterion",
      "Apply same hover-reveal delete button pattern",
      "Add centered dashed-border add button",
      "Add elegant empty state with CheckCircle icon",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): redesign criteria list with checkmarks"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Apply glass effect styling to all input fields",
    "plan_section": "Task 6: Add glass input styling to all fields",
    "blocking": [7],
    "blockedBy": [1],
    "atomic_commit": "feat(ideation): apply glass effect to all input fields",
    "steps": [
      "Read specs/plans/proposal_edit_modal_redesign.md section 'Task 6'",
      "Update inputClasses constant with glass effect:",
      "  - background: rgba(0, 0, 0, 0.3)",
      "  - border: 1px solid rgba(255, 255, 255, 0.08)",
      "  - border-radius: 8px",
      "Update focus states: border-[#ff6b35]/50 + ring-[#ff6b35]/10",
      "Apply to Title input, Description textarea, step inputs, criteria inputs",
      "Update select dropdowns with matching glass styling",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): apply glass effect to all input fields"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Add modal entry animations with staggered content reveal",
    "plan_section": "Task 7: Add entry animations and staggered timing",
    "blocking": [8],
    "blockedBy": [6],
    "atomic_commit": "feat(ideation): add modal entry animations",
    "steps": [
      "Read specs/plans/proposal_edit_modal_redesign.md section 'Task 7'",
      "Add <style> tag with @keyframes modal-slide-up animation",
      "Animation: from opacity:0 translateY(20px) scale(0.98) to opacity:1 translateY(0) scale(1)",
      "Duration: 250ms, easing: cubic-bezier(0.16, 1, 0.3, 1)",
      "Implement staggered content animation using animation-delay",
      "Each section gets 50ms additional delay (0ms, 50ms, 100ms, etc.)",
      "Add animation classes to Title, Description, Metadata, Steps, Criteria sections",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add modal entry animations"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Add micro-interactions and ambient corner glow",
    "plan_section": "Task 8: Add micro-interactions and ambient glow",
    "blocking": [],
    "blockedBy": [7],
    "atomic_commit": "feat(ideation): add micro-interactions and ambient glow",
    "steps": [
      "Read specs/plans/proposal_edit_modal_redesign.md section 'Task 8'",
      "Add input focus micro-interaction: hover:scale-[1.01] transition-transform",
      "Add button hover: -translate-y-px shadow-lg transition-all",
      "Ensure delete buttons have fade-in on row hover (opacity transition)",
      "Add complexity dots hover: scale-125 transition",
      "Add ambient warm glow to modal corners using pseudo-elements or gradient",
      "Glow color: rgba(255, 107, 53, 0.05) with blur",
      "Verify all data-testid attributes are preserved for tests",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add micro-interactions and ambient glow"
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
| **Inline sub-components** | GlassCard, ComplexitySelector, EditableListItem kept inline to avoid file explosion for single-use components |
| **CSS-in-JS via Tailwind** | Consistent with project patterns; no separate CSS files needed |
| **<style> tag for keyframes** | Co-locates animation definitions with component per project convention |
| **Visual complexity selector** | 5-dot visual scale more intuitive than dropdown, matches Linear/Figma aesthetic |
| **Hover-reveal delete buttons** | Reduces visual clutter, modern pattern from Notion/Linear |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] ProposalEditModal tests still pass
- [ ] All data-testid attributes preserved

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Open proposal edit modal - verify glass/frosted appearance
- [ ] Check two-column metadata layout renders correctly
- [ ] Click complexity dots - verify selection changes
- [ ] Add/remove steps - verify circled numbers and animations
- [ ] Add/remove criteria - verify checkmarks and animations
- [ ] Verify modal entry animation is smooth
- [ ] Hover over inputs - verify subtle scale effect
- [ ] Hover over rows - verify delete button appears
- [ ] Tab through form - verify focus states with orange glow
- [ ] Save changes - verify functionality unchanged

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] ComplexitySelector wired to complexity state
- [ ] All form fields still save to proposal
- [ ] Modal open/close animations work

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.

## Design Quality Checklist

- [ ] NO purple/blue gradients
- [ ] NO Inter font (uses SF Pro via system)
- [ ] Warm orange accent `#ff6b35` used strategically
- [ ] Layered shadows for depth
- [ ] Glass/blur effects for premium feel
- [ ] Tight letter-spacing on headings
- [ ] Staggered entry animations work
- [ ] Micro-interactions on hover/focus work
- [ ] Visual hierarchy through spacing and typography
- [ ] Unique, memorable design (not cookie-cutter)
