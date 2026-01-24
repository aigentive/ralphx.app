# Phase 13 & 14: UI/Design Overhaul Plan

## Overview

Transform RalphX from "ugly and unpolished" to a premium, beautiful Mac app that looks like it was designed by a 10x designer. This involves:

1. **Phase 13**: Design system setup + page-by-page design requirements
2. **Phase 14**: Implementation of those designs using the Ralph loop

**Key decisions:**
- Adopt **Lucide React** for icons (replacing inline SVGs)
- Adopt **shadcn/ui** fully (migrate all 57 components)
- Create comprehensive Design Document at `specs/DESIGN.md`
- Each page gets visual verification like Phase 12

---

## Current State (from exploration)

### Pages/Views to Polish

**Note:** Phase 12 (Reconciliation) adds several new UI components. This design phase must include all of them.

| View | Components | Source | Status |
|------|------------|--------|--------|
| Kanban | TaskBoard, Column, TaskCard | Phases 5-6 | Complete but flat |
| Ideation | IdeationView, ChatPanel, ProposalList | Phase 10 | Complete but generic |
| Extensibility | 4 tabs (Workflows, Artifacts, Research, Methodologies) | Phase 11 | Partial |
| Task Detail | TaskDetailView, StateHistoryTimeline | Phase 6 | Complete |
| Reviews Panel | ReviewsPanel, ReviewCard, ReviewNotesModal | Phase 9 | Complete |
| Chat Panel | ChatPanel, ChatMessage, ChatInput | Phase 10 | Complete |
| Settings | SettingsView (Execution, Model, Review, Supervisor sections) | **Phase 12** | New |
| Activity | ActivityView (real-time agent execution) | **Phase 12** | New |
| QA Components | TaskQABadge, TaskDetailQAPanel, QASettingsPanel | Phase 8 | Complete |
| Screenshot Gallery | ScreenshotGallery with lightbox and comparison | **Phase 12** | New |
| Modals | AskUserQuestionModal, TaskRerunDialog | Mixed | Partial |
| Diff Viewer | DiffViewer (Changes/History tabs, file tree) | **Phase 12** | New |
| Project Sidebar | ProjectSidebar with project list, WorktreeStatus | **Phase 12** | New |
| Project Dialogs | ProjectCreationWizard, MergeWorkflowDialog | **Phase 12** | New |
| Project Selector | ProjectSelector dropdown in header | **Phase 12** | New |
| Header/Nav | App.tsx header with Activity/Settings navigation | **Phase 12** | Updated |
| Execution | ExecutionControlBar | Phase 7 | Complete |

### Phase 12 UI Components (must be included in design overhaul)

These components are being added by Phase 12 and will need design polish:

1. **Project Sidebar** - Left sidebar with project list, status indicators, navigation
2. **Activity Stream View** - Real-time agent execution monitoring with expandable tool calls
3. **Settings View** - Configuration for execution, model, review, supervisor settings
4. **Project Creation Wizard** - Git mode selection (Local vs Worktree), folder picker
5. **Merge Workflow Dialog** - Post-completion options (merge, rebase, PR, keep, discard)
6. **Task Re-run Dialog** - Options when moving Done task back to Planned
7. **Diff Viewer** - Split-view with Changes/History tabs, file tree, syntax highlighting
8. **Screenshot Gallery** - Thumbnail grid, lightbox, Expected vs Actual comparison
9. **Project Selector** - Dropdown replacing hardcoded project name in header

### Current Design Tokens
- Colors: warm orange accent (#ff6b35), dark grays, off-white
- Typography: SF Pro (NOT Inter)
- Spacing: 8pt grid
- Anti-AI-slop: no purple gradients, no generic icons

### Problems Identified
- Flat, lifeless surfaces (no depth)
- Weak visual hierarchy
- Inconsistent borders and shadows
- Missing micro-interactions
- Generic component styling
- Typography lacks refinement
- 57 custom components with varying quality

---

## Phase 13 Structure

### Task 1: Foundation Setup

**Create Design Document + Install Dependencies**

```json
{
  "category": "design",
  "description": "Set up design foundation: Lucide, shadcn/ui, and Design Document",
  "steps": [
    "Install Lucide React: npm install lucide-react",
    "Initialize shadcn/ui: npx shadcn@latest init",
    "Configure shadcn with existing Tailwind config",
    "Map design tokens to shadcn CSS variables:",
    "  - --accent-primary (#ff6b35) → --primary",
    "  - --bg-base (#0f0f0f) → --background",
    "  - --bg-elevated (#242424) → --card",
    "  - --text-primary (#f0f0f0) → --foreground",
    "  - --border-subtle → --border",
    "Create specs/DESIGN.md with:",
    "  - Design Philosophy (premium 10x designer aesthetic)",
    "  - Anti-AI-Slop Guardrails",
    "  - Color System (all tokens with usage guidelines)",
    "  - Typography (SF Pro hierarchy, letter-spacing, line-heights)",
    "  - Spacing System (4px base, 8pt grid)",
    "  - Shadow System (layered shadows for depth)",
    "  - Component Patterns (buttons, cards, inputs per shadcn)",
    "  - Motion & Micro-interactions",
    "  - Icon Usage (Lucide guidelines)",
    "  - Page-Specific Patterns placeholder",
    "Add core shadcn components:",
    "  npx shadcn@latest add button card dialog dropdown-menu",
    "  npx shadcn@latest add input label tabs tooltip popover",
    "  npx shadcn@latest add select checkbox toggle badge",
    "  npx shadcn@latest add scroll-area separator skeleton",
    "Customize shadcn components for warm orange accent",
    "Run npm run lint && npm run typecheck",
    "Commit: chore: setup Lucide icons and shadcn/ui foundation"
  ],
  "output": "specs/DESIGN.md, src/components/ui/",
  "passes": false
}
```

### Task 2-17: Page-Specific Design Tasks

Each page task follows this pattern:

1. Read specs/DESIGN.md
2. Use /frontend-design skill to plan the redesign
3. Document specific requirements for that page in specs/DESIGN.md
4. Create before/after mockup descriptions
5. Define acceptance_criteria and design_quality arrays
6. Visual verification via screenshots

#### Task 2: Kanban Board

```json
{
  "category": "design",
  "description": "Design requirements for Kanban Board",
  "steps": [
    "Read specs/DESIGN.md for design guidelines",
    "Use /frontend-design skill to plan Kanban redesign",
    "Document requirements in specs/DESIGN.md → Kanban section:",
    "  TaskBoard:",
    "  - Subtle gradient background (not flat)",
    "  - Horizontal scroll with fade edges",
    "  - 24px gutters between columns",
    "  Column:",
    "  - Glass effect header with warm orange accent",
    "  - Task count badge using shadcn Badge",
    "  - Drop zone with orange glow on drag-over",
    "  - Empty state with Lucide icon + helpful text",
    "  TaskCard:",
    "  - Use shadcn Card with custom styling",
    "  - Layered shadow for depth",
    "  - Hover: translateY(-2px) + shadow elevation",
    "  - Priority as colored left border stripe",
    "  - Badges using shadcn Badge variants",
    "  - Drag state: scale(1.02), rotate(2deg), high shadow",
    "Define acceptance_criteria array (functional)",
    "Define design_quality array (visual standards)",
    "Specify screenshot locations for verification",
    "Commit: docs: add Kanban design requirements to DESIGN.md"
  ],
  "passes": false
}
```

#### Task 3: Ideation View

```json
{
  "category": "design",
  "description": "Design requirements for Ideation View",
  "steps": [
    "Read specs/DESIGN.md for design guidelines",
    "Use /frontend-design skill to plan Ideation redesign",
    "Document requirements in specs/DESIGN.md → Ideation section:",
    "  Layout:",
    "  - Balanced two-panel split (resizable)",
    "  - Clear visual separation",
    "  ChatPanel:",
    "  - User messages: right-aligned, warm accent background",
    "  - AI messages: left-aligned, elevated surface",
    "  - Typing indicator with animated dots",
    "  - Sticky input at bottom using shadcn Input",
    "  ProposalList:",
    "  - Use shadcn Card for ProposalCard",
    "  - Checkbox using shadcn Checkbox",
    "  - PriorityBadge using shadcn Badge with color variants",
    "  - Selection state with border + subtle background",
    "  - Apply button prominent, fixed position",
    "Define acceptance_criteria and design_quality arrays",
    "Commit: docs: add Ideation design requirements to DESIGN.md"
  ],
  "passes": false
}
```

#### Tasks 4-17: Remaining Pages

| Task | Page | Key shadcn Components |
|------|------|----------------------|
| 4 | Settings View | Card, Input, Toggle, Select, Label |
| 5 | Activity View | Card, ScrollArea, Badge |
| 6 | Extensibility View | Tabs, Card |
| 7 | Task Detail View | Dialog, Card, Badge, ScrollArea |
| 8 | Reviews Panel | Card, Badge, Button, ScrollArea |
| 9 | Chat Panel | Input, Button, ScrollArea |
| 10 | QA Components | Card, Badge, Dialog (gallery) |
| 11 | Project Sidebar | Button, ScrollArea, Separator |
| 12 | Project Dialogs | Dialog, Input, Button, Select |
| 13 | Diff Viewer | Tabs, ScrollArea, Card |
| 14 | Execution Control | Button, Badge, Tooltip |
| 15 | Header/Navigation | Button, Tooltip, DropdownMenu |
| 16 | All Modals | Dialog (standardized pattern) |
| 17 | Final Consistency Check | Full walkthrough |

---

## Phase 14 Structure

Phase 14 implements the designs documented in Phase 13.

### Task Pattern

Each implementation task:

1. Read the design requirements from specs/DESIGN.md
2. Migrate components to shadcn equivalents
3. Replace inline SVG icons with Lucide
4. Apply premium styling (shadows, gradients, animations)
5. Run lint + typecheck
6. Visual verification via agent-browser screenshots
7. Compare to design requirements
8. Commit

```json
{
  "category": "implementation",
  "description": "Implement Kanban Board design",
  "steps": [
    "Read specs/DESIGN.md → Kanban section",
    "Migrate TaskCard to use shadcn Card",
    "Replace status icons with Lucide equivalents",
    "Add layered shadows and hover animations",
    "Implement Column glass effect header",
    "Add drop zone glow effect",
    "Create empty state with Lucide icon",
    "Run npm run lint && npm run typecheck",
    "Start npm run tauri dev",
    "Use agent-browser to capture screenshots:",
    "  - screenshots/impl-kanban-overview.png",
    "  - screenshots/impl-kanban-card-hover.png",
    "  - screenshots/impl-kanban-drag.png",
    "Verify against acceptance_criteria",
    "Verify against design_quality",
    "Commit: feat: implement premium Kanban board design"
  ],
  "passes": false
}
```

---

## Premium Design Principles (for specs/DESIGN.md)

### What Makes It Premium

1. **Layered Shadows** (not flat):
   ```css
   --shadow-sm: 0 1px 2px rgba(0,0,0,0.15), 0 1px 3px rgba(0,0,0,0.1);
   --shadow-md: 0 4px 6px rgba(0,0,0,0.1), 0 8px 16px rgba(0,0,0,0.1);
   ```

2. **Gradient Borders** (subtle depth):
   ```css
   border: 1px solid transparent;
   background: linear-gradient(var(--bg-elevated), var(--bg-elevated)) padding-box,
               linear-gradient(180deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.02) 100%) border-box;
   ```

3. **Micro-interactions**:
   - Hover lift: `translateY(-2px)` + shadow elevation
   - Active press: `scale(0.98)`
   - Focus ring: `0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary)`

4. **Typography Refinement**:
   - Titles: `letter-spacing: -0.02em`
   - Labels: `letter-spacing: 0.05em; text-transform: uppercase`
   - Line heights: 1.2 (headings), 1.5 (body)

5. **Strategic Accent Usage** (5% rule):
   - Active states only
   - One primary button per section
   - Selected indicators
   - NOT large surfaces

6. **Glass Effects** (modals/overlays only):
   ```css
   background: rgba(26, 26, 26, 0.85);
   backdrop-filter: blur(20px) saturate(180%);
   ```

### Reference Apps
- Linear (board, cards, interactions)
- Raycast (Mac-native feel)
- Arc (spatial organization)
- Vercel Dashboard (typography, spacing)

---

## File Changes Summary

### New Files
- `specs/DESIGN.md` - Master design document
- `src/components/ui/*.tsx` - shadcn components
- `specs/phases/prd_phase_13_design.md` - Phase 13 PRD
- `specs/phases/prd_phase_14_implementation.md` - Phase 14 PRD

### Modified Files
- `tailwind.config.js` - shadcn integration
- `src/styles/globals.css` - Updated tokens
- `package.json` - Lucide + shadcn deps
- All component files (migration to shadcn)

### Critical Files for Implementation
1. `src/styles/globals.css` - Design tokens
2. `tailwind.config.js` - Tailwind/shadcn config
3. `src/components/tasks/TaskBoard/TaskCard.tsx` - Most visible component
4. `src/components/ui/` - New shadcn components
5. `src/App.tsx` - Header/navigation

---

## Manifest Updates

Add to `specs/manifest.json`:

```json
{
  "phase": 13,
  "name": "Design System",
  "prd": "specs/phases/prd_phase_13_design.md",
  "status": "pending",
  "description": "Setup Lucide + shadcn, create Design Document with page requirements"
},
{
  "phase": 14,
  "name": "Design Implementation",
  "prd": "specs/phases/prd_phase_14_implementation.md",
  "status": "pending",
  "description": "Implement premium designs from Phase 13"
}
```

---

## Verification

Each task includes:

1. **Functional acceptance_criteria** (what must work)
2. **Visual design_quality** (what must look like)
3. **Screenshots** via agent-browser for visual verification
4. **Anti-AI-slop check** (no purple, no Inter, no generic icons)

---

## Status

✅ **COMPLETED:**
1. ✅ Created `specs/phases/prd_phase_13_design.md` with 17 tasks
2. ✅ Created `specs/phases/prd_phase_14_implementation.md` as boilerplate (16 tasks)
3. ✅ Updated `specs/manifest.json` with phases 13 & 14
4. ⏳ Phase 13 runs via Ralph loop, documenting all design requirements
5. ⏳ Phase 14 runs via Ralph loop, implementing all designs

## Next Steps (Automatic via Ralph Loop)

Once Phase 12 completes:
1. Ralph loop will auto-transition to Phase 13
2. Phase 13 Task 1: Install Lucide + shadcn/ui
3. Phase 13 Task 2: Create specs/DESIGN.md
4. Phase 13 Tasks 3-16: Document design requirements per page
5. Phase 13 Task 17: Final consistency check
6. Auto-transition to Phase 14
7. Phase 14: Implement all designs with visual verification
