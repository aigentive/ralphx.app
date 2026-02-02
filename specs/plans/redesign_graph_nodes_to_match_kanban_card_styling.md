# Plan: Redesign Graph Nodes to Match Kanban Card Styling

## Design Gap Analysis

### Current Kanban Card Design (Target)
| Aspect | Implementation |
|--------|---------------|
| **Surface** | Glass morphism: `backdrop-blur(12px) saturate(150%)`, translucent bg `hsla(220 10% 14% / 0.85)` |
| **Border** | 1px subtle divider `hsla(220 10% 100% / 0.06)` + 3px left priority stripe |
| **Shadow** | Soft elevation: `0 2px 8px hsla(220 10% 0% / 0.25)` |
| **Priority indicator** | Left border stripe (3px) with priority-specific colors |
| **Active state animation** | Pulsing border glow for executing/reviewing (2s ease-in-out infinite) |
| **Activity dots** | 3 dots with staggered bounce (1.4s, 0s/0.2s/0.4s delays) for active states |
| **Status badges** | Top-right corner, 9px font, `px-1.5 py-px`, translucent colored backgrounds |
| **Drag visual** | Grip handle appears on hover (group-hover:opacity-100) |
| **Hover** | No scale, just interactive state for click |
| **Transitions** | 150ms ease for bg/transform/shadow |

### Current Graph Node Design (Gap)
| Aspect | Current State | Gap |
|--------|--------------|-----|
| **Surface** | Solid `hsla(220 10% 15% / 0.8)`, no blur | Missing glass morphism |
| **Border** | 2px solid, full border in status color | No left stripe pattern |
| **Shadow** | Only on executing state | Missing consistent soft shadow |
| **Priority indicator** | None | Missing left stripe |
| **Active state animation** | Only highlight/selected rings | Missing pulsing glow |
| **Activity dots** | None | Missing for executing/reviewing states |
| **Status badges** | Bottom of node, 10px, solid color bg | Wrong position, wrong style |
| **Hover** | scale-105/110 | Different from Kanban (no scale) |
| **Handles** | Basic gray circles | Could be more subtle |

---

## Redesign Tasks

### Task 1: Update Base Node Surface Styling (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `refactor(task-graph): apply glass morphism to node surface`
**Files:** `src/components/TaskGraph/nodes/TaskNode.tsx`, `TaskNodeCompact.tsx`, `nodeStyles.ts`

**Changes:**
1. Add glass morphism to base node style:
   ```typescript
   background: "hsla(220 10% 14% / 0.85)"
   backdropFilter: "blur(12px) saturate(150%)"
   WebkitBackdropFilter: "blur(12px) saturate(150%)"
   ```
2. Change border to subtle divider: `1px solid hsla(220 10% 100% / 0.06)`
3. Add consistent soft shadow: `0 2px 8px hsla(220 10% 0% / 0.25)`
4. Remove hover scale effect (keep hover shadow-lg only)

### Task 2: Add Left Priority Stripe
**Dependencies:** Task 1
**Atomic Commit:** `feat(task-graph): add left priority stripe to nodes`
**Files:** `nodeStyles.ts`, `TaskNode.tsx`, `TaskNodeCompact.tsx`

**Changes:**
1. Add `borderLeft: "3px solid {priorityColor}"` to node style
2. Map priority to stripe colors (same as Kanban):
   - P1: `hsl(0 70% 55%)` (red)
   - P2: `hsl(25 90% 55%)` (deep orange)
   - P3: `hsl(14 100% 60%)` (accent orange #ff6b35)
   - P4: `hsl(220 10% 35%)` (gray)
3. Update `TaskNodeData` type if priority not already passed

### Task 3: Add Activity Dots for Active States
**Dependencies:** Task 1
**Atomic Commit:** `feat(task-graph): add activity dots for active states`
**Files:** `TaskNode.tsx`, `TaskNodeCompact.tsx`

**Changes:**
1. Add 3-dot activity indicator for `executing`, `re_executing`, `reviewing` states
2. Position: top-right corner (like Kanban)
3. Animation: staggered bounce (1.4s, offsets 0s/0.2s/0.4s)
4. Colors: orange for executing, blue for reviewing

### Task 4: Add Pulsing Border Animation
**Dependencies:** Task 1
**Atomic Commit:** `feat(task-graph): add pulsing border animation for active states`
**Files:** `nodeStyles.ts`, `globals.css`

**Changes:**
1. For `executing`/`re_executing`: add `--animation-executing-pulse` (2s, orange glow)
2. For `reviewing`: add `--animation-reviewing-pulse` (2s, blue glow)
3. Apply via style object: `animation: "var(--animation-executing-pulse)"`

### Task 5: Relocate and Restyle Status Badge
**Dependencies:** Task 1
**Atomic Commit:** `refactor(task-graph): relocate status badge to top-right corner`
**Files:** `TaskNode.tsx`

**Changes:**
1. Move badge from bottom to top-right corner (absolute positioning)
2. Reduce font to 9px, use `px-1.5 py-px`
3. Use translucent colored backgrounds: `rgba({color}, 0.2)`
4. Add appropriate icon for each status (Clock, Loader2, CheckCircle, etc.)

### Task 6: Update Connection Handles
**Dependencies:** Task 1
**Atomic Commit:** `refactor(task-graph): make connection handles more subtle`
**Files:** `TaskNode.tsx`, `TaskNodeCompact.tsx`

**Changes:**
1. Make handles more subtle: smaller size, lower opacity
2. Style: `!w-1.5 !h-1.5 !opacity-50 hover:!opacity-100`
3. Transition on hover

### Task 7: Update Hover/Selection States
**Dependencies:** Task 1
**Atomic Commit:** `refactor(task-graph): update hover and selection states`
**Files:** `TaskNode.tsx`, `TaskNodeCompact.tsx`

**Changes:**
1. Remove `hover:scale-105`/`hover:scale-110`
2. Keep `hover:shadow-lg` for elevation effect
3. Selected state: use Finder-like blue selection (match Kanban)
   - `background: hsla(220 60% 50% / 0.25)`
   - `border: 1px solid hsla(220 60% 60% / 0.3)`

---

## Priority Order

1. **Task 1** (Base surface) - Foundation for all other changes
2. **Task 2** (Priority stripe) - Key visual hierarchy element
3. **Task 5** (Badge relocation) - Aligns with Kanban pattern
4. **Task 3** (Activity dots) - Active state feedback
5. **Task 4** (Pulsing animation) - Polish for active states
6. **Task 7** (Hover/selection) - Interaction consistency
7. **Task 6** (Handles) - Minor polish

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/components/TaskGraph/nodes/nodeStyles.ts` | Priority colors, glass morphism base, animation refs |
| `src/components/TaskGraph/nodes/TaskNode.tsx` | Surface, stripe, badge, dots, hover, handles |
| `src/components/TaskGraph/nodes/TaskNodeCompact.tsx` | Surface, stripe, dots, hover, handles |
| `src/styles/globals.css` | Animation keyframes (already exist, verify parity) |

---

## Verification

1. Open graph view with tasks in various states (backlog, executing, reviewing, approved)
2. Compare visual parity with Kanban cards
3. Verify priority stripes match Kanban priority indicators
4. Verify activity dots animate for executing/reviewing nodes
5. Verify pulsing border animations work
6. Verify hover/selection states match Kanban behavior
7. Verify handles remain functional for edge connections

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Graph

```
Task 1 (Base surface) ─┬─► Task 2 (Priority stripe)
                       ├─► Task 3 (Activity dots)
                       ├─► Task 4 (Pulsing animation)
                       ├─► Task 5 (Badge relocation)
                       ├─► Task 6 (Handles)
                       └─► Task 7 (Hover/selection)
```

**Execution Strategy:** Task 1 is the foundation and must complete first. Tasks 2-7 can be executed in parallel after Task 1, as they modify different aspects of the node components without interdependencies.
