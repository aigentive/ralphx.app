# RalphX - Phase 14: Design Implementation

## Overview

This phase implements the premium designs documented in Phase 13's `specs/DESIGN.md`. Each task takes the design requirements for a specific page/component and applies them, migrating to shadcn/ui components, replacing icons with Lucide, and adding polish.

**Reference:**
- `specs/DESIGN.md` - Master design document (created in Phase 13)
- `specs/DESIGN_OVERHAUL_PLAN.md` - Design overhaul strategy

## Goals

1. Migrate all components to shadcn/ui equivalents
2. Replace all inline SVG icons with Lucide React
3. Apply premium styling (shadows, gradients, animations)
4. Implement micro-interactions (hover lift, press scale, focus rings)
5. Visually verify each page against design requirements
6. Achieve consistent, polished, 10x-designer quality throughout

## Dependencies

- Phase 13 must be complete (DESIGN.md exists with all sections)
- shadcn/ui components installed and configured
- Lucide React installed

## Implementation Pattern

Each task follows this pattern:

1. Read the design requirements from `specs/DESIGN.md` → relevant section
2. Migrate existing components to use shadcn/ui primitives
3. Replace inline SVG icons with Lucide equivalents
4. Apply premium styling (shadows, gradients, borders)
5. Add micro-interactions (transitions, hover states, animations)
6. Run `npm run lint && npm run typecheck`
7. Start `npm run tauri dev`
8. Use agent-browser to capture verification screenshots
9. Verify against `acceptance_criteria` from DESIGN.md
10. Verify against `design_quality` from DESIGN.md
11. Fix any issues using `/frontend-design` skill
12. Commit with descriptive message

## Anti-AI-Slop Verification (EVERY TASK)

Before marking any task complete, verify:

- ❌ NO purple/blue gradients anywhere
- ❌ NO Inter font (must be SF Pro)
- ❌ NO generic icon grids
- ❌ NO high saturation on dark backgrounds
- ❌ NO flat, lifeless surfaces
- ✅ Warm orange accent (#ff6b35) used appropriately
- ✅ Layered shadows create depth
- ✅ Micro-interactions feel polished
- ✅ Typography has proper letter-spacing and line-height
- ✅ Accent color follows 5% rule

---

## Task List

**IMPORTANT: Work on ONE task per iteration.** Find the first task with `"passes": false`, complete it, update `"passes": true`, commit, and stop.

**NOTE:** Tasks below are boilerplate. Before starting Phase 14, verify each page's requirements exist in `specs/DESIGN.md` from Phase 13.

```json
[
  {
    "category": "implementation",
    "description": "Implement Kanban Board premium design",
    "steps": [
      "Read specs/DESIGN.md → Kanban section",
      "Migrate TaskBoard to use subtle gradient background",
      "Migrate Column to use glass effect header with shadcn Badge for count",
      "Migrate TaskCard to use shadcn Card with layered shadows",
      "Replace all status icons with Lucide equivalents",
      "Add hover lift effect: translateY(-2px) + shadow elevation",
      "Add priority left border stripe",
      "Add drag state: scale(1.02), rotate(2deg)",
      "Add drop zone orange glow effect",
      "Create empty column state with Lucide icon",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots:",
      "  - screenshots/impl-kanban-overview.png",
      "  - screenshots/impl-kanban-card-hover.png",
      "  - screenshots/impl-kanban-card-drag.png",
      "  - screenshots/impl-kanban-empty-column.png",
      "Verify against acceptance_criteria from DESIGN.md",
      "Verify against design_quality from DESIGN.md",
      "Use /frontend-design skill to fix any issues",
      "Commit: feat: implement premium Kanban board design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Ideation View premium design",
    "steps": [
      "Read specs/DESIGN.md → Ideation section",
      "Implement balanced two-panel split with resize handle",
      "Migrate ChatPanel messages to proper bubble styling",
      "Migrate ChatInput to shadcn Input with Send button",
      "Migrate ProposalCard to shadcn Card with Checkbox",
      "Migrate PriorityBadge to shadcn Badge with color variants",
      "Replace all icons with Lucide equivalents",
      "Add selection state styling",
      "Add typing indicator animation",
      "Create empty states with Lucide icons",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Ideation view design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Settings View premium design",
    "steps": [
      "Read specs/DESIGN.md → Settings section",
      "Organize sections in shadcn Cards",
      "Migrate toggles to shadcn Switch",
      "Migrate inputs to shadcn Input",
      "Migrate dropdowns to shadcn Select",
      "Add section headers with Lucide icons",
      "Add proper label styling with shadcn Label",
      "Ensure consistent spacing (32px between sections)",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Settings view design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Activity Stream View premium design",
    "steps": [
      "Read specs/DESIGN.md → Activity section",
      "Implement viewport-filling scrollable layout",
      "Add search/filter bar at top",
      "Style activity entries with timestamps and icons",
      "Implement expandable tool call details",
      "Add syntax highlighting for JSON outputs",
      "Add copy button for outputs",
      "Color-code different entry types",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Activity view design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Extensibility View premium design",
    "steps": [
      "Read specs/DESIGN.md → Extensibility section",
      "Migrate tab navigation to shadcn Tabs",
      "Style Workflows tab with shadcn Card",
      "Style Artifacts tab with grid layout and type badges",
      "Style Research tab with progress indicators",
      "Style Methodologies tab with activation toggles",
      "Replace all icons with Lucide equivalents",
      "Add proper active states and hover effects",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Extensibility view design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Task Detail View premium design",
    "steps": [
      "Read specs/DESIGN.md → Task Detail section",
      "Migrate to shadcn Dialog with custom sizing",
      "Add backdrop blur glass effect",
      "Add scale animation on open",
      "Style header with title, status badge, priority",
      "Style content sections (description, steps, reviews, QA)",
      "Style StateHistoryTimeline with connected dots",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Task Detail design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Reviews Panel premium design",
    "steps": [
      "Read specs/DESIGN.md → Reviews section",
      "Implement right slide-in panel with animation",
      "Migrate filter tabs to shadcn Tabs (pills variant)",
      "Migrate ReviewCard to shadcn Card",
      "Style reviewer type indicators and status badges",
      "Style action buttons (View Diff, Approve, Request Changes)",
      "Add fix attempt counter styling",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Reviews panel design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Chat Panel premium design",
    "steps": [
      "Read specs/DESIGN.md → Chat Panel section",
      "Implement resizable panel with drag handle",
      "Style message bubbles (user vs assistant)",
      "Add proper markdown rendering",
      "Add code block syntax highlighting",
      "Migrate input to shadcn Input",
      "Style Send button with keyboard hint",
      "Add context indicator in header",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Chat panel design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement QA Components premium design",
    "steps": [
      "Read specs/DESIGN.md → QA section",
      "Migrate TaskQABadge to shadcn Badge with color variants",
      "Add Lucide icons per QA state",
      "Style TaskDetailQAPanel with tab interface",
      "Migrate QASettingsPanel to use shadcn Switch and Input",
      "Style ScreenshotGallery thumbnail grid",
      "Implement lightbox with keyboard navigation",
      "Add Expected vs Actual comparison view",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium QA components design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Project Sidebar premium design",
    "steps": [
      "Read specs/DESIGN.md → Project Sidebar section",
      "Style fixed sidebar with proper width and border",
      "Style project list with git mode indicators",
      "Style WorktreeStatus with Lucide GitBranch icon",
      "Style New Project button with shadcn Button",
      "Style navigation items with Lucide icons",
      "Add active state with accent indicator",
      "Add hover effects",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Project Sidebar design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Project Dialogs premium design",
    "steps": [
      "Read specs/DESIGN.md → Project Dialogs section",
      "Migrate all dialogs to shadcn Dialog",
      "Add backdrop blur and scale animation",
      "Style Project Creation Wizard with proper form layout",
      "Style Merge Workflow Dialog with radio options",
      "Style Task Re-run Dialog with warning states",
      "Ensure consistent header and footer patterns",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Project dialogs design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Diff Viewer premium design",
    "steps": [
      "Read specs/DESIGN.md → Diff Viewer section",
      "Migrate tabs to shadcn Tabs",
      "Style file tree with collapsible directories",
      "Add Lucide icons for file status (Plus, Edit, Minus, ArrowRight)",
      "Style diff panel with proper line highlighting",
      "Add syntax highlighting for code",
      "Style commit list with short SHA and metadata",
      "Add Open in IDE button with Lucide ExternalLink",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Diff Viewer design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Execution Control Bar premium design",
    "steps": [
      "Read specs/DESIGN.md → Execution Control section",
      "Style fixed bottom bar with subtle top border",
      "Add animated status dot (pulsing when running)",
      "Style running/queued counts display",
      "Migrate control buttons to shadcn Button with Tooltip",
      "Add Lucide icons (Pause, Play, Square)",
      "Style disabled states appropriately",
      "Add current task name display",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Execution Control design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Implement Header and Navigation premium design",
    "steps": [
      "Read specs/DESIGN.md → Header section",
      "Style fixed header with proper height and border",
      "Add Tauri drag region",
      "Style view navigation with shadcn Button (ghost)",
      "Add active state with accent indicator",
      "Style Project Selector with shadcn DropdownMenu",
      "Style Chat/Reviews toggle buttons with count badges",
      "Add Lucide icons throughout",
      "Add keyboard shortcut tooltips (Cmd+1/2/3/4/5/K)",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: implement premium Header design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Standardize all modals to common pattern",
    "steps": [
      "Read specs/DESIGN.md → Modal Standards section",
      "Audit all existing modals in the app",
      "Migrate AskUserQuestionModal to shadcn Dialog",
      "Migrate ReviewNotesModal to shadcn Dialog",
      "Migrate ProposalEditModal to shadcn Dialog",
      "Ensure consistent backdrop, animation, sizing",
      "Ensure consistent header pattern (title, close button)",
      "Ensure consistent footer pattern (cancel, primary action)",
      "Replace all icons with Lucide equivalents",
      "Run npm run lint && npm run typecheck",
      "Start npm run tauri dev",
      "Use agent-browser to capture screenshots of each modal",
      "Verify against acceptance_criteria and design_quality",
      "Commit: feat: standardize all modals to premium pattern"
    ],
    "passes": false
  },
  {
    "category": "verification",
    "description": "Final visual verification and polish pass",
    "steps": [
      "Start npm run tauri dev",
      "Use agent-browser to capture comprehensive screenshots:",
      "  - Each main view (Kanban, Ideation, Extensibility, Activity, Settings)",
      "  - Each modal (Task Detail, Reviews, Project dialogs, QA)",
      "  - Each side panel (Reviews, Chat)",
      "  - Various states (hover, selected, loading, empty)",
      "Review all screenshots against specs/DESIGN.md",
      "Verify anti-AI-slop compliance throughout:",
      "  - No purple gradients",
      "  - No Inter font",
      "  - Warm orange accent used correctly",
      "  - Layered shadows everywhere",
      "  - Micro-interactions feel polished",
      "Fix any remaining issues using /frontend-design skill",
      "Update specs/DESIGN.md with any final notes",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: complete Phase 14 design implementation"
    ],
    "passes": false
  }
]
```

---

## Verification Checklist

After completing all tasks, verify the following across the entire app:

### Typography
- [ ] SF Pro fonts used throughout (not Inter)
- [ ] Proper letter-spacing on headings (-0.02em)
- [ ] Proper letter-spacing on labels (0.05em uppercase)
- [ ] Line heights comfortable (1.2 headings, 1.5 body)
- [ ] Font weights create clear hierarchy

### Colors
- [ ] Warm orange accent (#ff6b35) used strategically (5% rule)
- [ ] No purple or blue gradients anywhere
- [ ] Dark grays (#0f0f0f, #1a1a1a, #242424) not pure black
- [ ] Off-white text (#f0f0f0) not pure white
- [ ] Status colors intuitive and consistent

### Depth & Shadows
- [ ] Layered shadows on all cards and elevated elements
- [ ] Glass effects on modals and overlays
- [ ] Gradient borders on premium cards (hover state)
- [ ] No flat, lifeless surfaces

### Interactions
- [ ] Hover lift on cards (translateY(-2px))
- [ ] Press scale on buttons (scale(0.98))
- [ ] Focus rings visible and styled with accent
- [ ] Transitions smooth (150-200ms)
- [ ] Drag states visually clear

### Icons
- [ ] All icons are Lucide React
- [ ] Consistent sizing (16px inline, 20px buttons, 24px nav)
- [ ] Colors inherit from context
- [ ] No inline SVG icons remaining

### Components
- [ ] All primitives are shadcn/ui
- [ ] Consistent button variants used correctly
- [ ] Consistent badge variants for status/category/priority
- [ ] All modals follow standard pattern

### Overall
- [ ] Feels like Linear, Raycast, or Arc quality
- [ ] Native Mac app feel
- [ ] Information dense but not cluttered
- [ ] Every element intentional and polished
