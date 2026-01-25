# RalphX - Phase 14: Design Implementation

## Overview

This phase implements the premium designs documented in Phase 13. Each task takes the design requirements from a specific page design document and applies them, migrating to shadcn/ui components, replacing icons with Lucide, and adding polish.

**Reference:**
- `specs/DESIGN.md` - Master design system (colors, typography, spacing, shadows, patterns)
- `specs/design/pages/*.md` - Page-specific design requirements

## Goals

1. Migrate all components to shadcn/ui equivalents
2. Replace all inline SVG icons with Lucide React
3. Apply premium styling (shadows, gradients, animations)
4. Implement micro-interactions (hover lift, press scale, focus rings)
5. Achieve consistent, polished, 10x-designer quality throughout

## Dependencies

- Phase 13 must be complete (DESIGN.md and page docs exist)
- shadcn/ui components installed and configured
- Lucide React installed

## Implementation Pattern

Each task follows this pattern:

1. Read the page-specific design doc from `specs/design/pages/`
2. Implement according to the doc's **Component Hierarchy**, **Styling**, and **Structure**
3. Verify against the doc's **Acceptance Criteria**
4. Verify against the doc's **Design Quality Checklist**
5. Run `npm run lint && npm run typecheck`
6. Fix any issues using `/frontend-design` skill
7. Commit with descriptive message

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

```json
[
  {
    "category": "bugfix",
    "description": "Fix project creation - implement folder selection and git branch fetching",
    "steps": [
      "Read src/App.tsx to understand current stubbed implementations (handleBrowseFolder, handleFetchBranches, handleCreateProject)",
      "Install @tauri-apps/plugin-dialog if not present: npm install @tauri-apps/plugin-dialog",
      "Add 'dialog' plugin to src-tauri/Cargo.toml and src-tauri/capabilities/default.json",
      "Implement handleBrowseFolder in App.tsx using Tauri dialog.open({ directory: true })",
      "Create get_git_branches command in src-tauri/src/commands/project_commands.rs that executes 'git branch -a' in the working directory",
      "Register get_git_branches in src-tauri/src/lib.rs invoke_handler",
      "Add getGitBranches wrapper in src/lib/tauri.ts",
      "Implement handleFetchBranches in App.tsx to call getGitBranches",
      "Update handleCreateProject in App.tsx to call api.projects.create() instead of creating mock data",
      "Reference specs/design/pages/project-dialogs.md for updated design requirements",
      "Update ProjectCreationWizard form order: Location field FIRST, Project Name SECOND",
      "Make Project Name optional - auto-infer from folder name when location is selected (e.g., /Users/dev/my-app → 'my-app')",
      "Track isNameManuallySet state - only auto-update name if user hasn't typed custom value",
      "Allow user to override inferred project name if desired",
      "If no projects exist (empty project list), show ProjectCreationWizard centered on screen (both axes) as default view",
      "In first-run mode: hide close button, cancel button, disable Escape and backdrop click",
      "Test folder selection opens native macOS folder picker",
      "Test project name auto-populates from selected folder name",
      "Test branch dropdown shows real branches from selected folder",
      "Test project creation persists to database and survives page refresh",
      "Test first-launch shows centered project creation dialog",
      "Run npm run lint && npm run typecheck",
      "Run cargo test in src-tauri",
      "Commit: fix: implement project creation with folder selection and git branches"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Kanban Board premium design",
    "design_doc": "specs/design/pages/kanban-board.md",
    "steps": [
      "Read specs/design/pages/kanban-board.md for complete design spec",
      "Implement TaskBoard with radial gradient background and horizontal scroll-snap",
      "Implement Column with glass effect header, Badge for count, and empty state",
      "Implement TaskCard with priority stripe, layered shadows, hover lift, drag state",
      "Use Lucide icons: GripVertical, Inbox, CheckCircle, etc.",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Kanban board design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Ideation View premium design",
    "design_doc": "specs/design/pages/ideation-view.md",
    "steps": [
      "Read specs/design/pages/ideation-view.md for complete design spec",
      "Implement two-panel resizable layout with drag handle (min 320px per panel)",
      "Implement ConversationPanel with message bubbles, typing indicator, chat input",
      "Implement ProposalsPanel with header, toolbar, ProposalCard list, apply section",
      "Style user messages (right-aligned, orange bg) vs AI messages (left-aligned)",
      "Use Lucide icons: MessageSquare, ListTodo, Send, Lightbulb, GripVertical, etc.",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Ideation view design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Settings View premium design",
    "design_doc": "specs/design/pages/settings-view.md",
    "steps": [
      "Read specs/design/pages/settings-view.md for complete design spec",
      "Implement glass effect header with Settings icon and saving indicator",
      "Implement section cards (Execution, Model, Review, Supervisor) with gradient borders",
      "Use shadcn Switch, Input, Select for form controls",
      "Implement master toggle → sub-settings disabled pattern",
      "Use Lucide icons: Settings, Zap, Brain, FileSearch, Shield, Loader2",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Settings view design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Activity Stream View premium design",
    "design_doc": "specs/design/pages/activity-stream.md",
    "steps": [
      "Read specs/design/pages/activity-stream.md for complete design spec",
      "Implement header with Activity icon and alert badge",
      "Implement search input and filter tabs (All, Thinking, Tool Calls, Results, Text, Errors)",
      "Implement activity entries with type-specific styling (left border, background tint)",
      "Implement expandable details with JSON syntax highlighting and copy button",
      "Implement auto-scroll behavior and 'Scroll to latest' banner",
      "Use Lucide icons: Activity, Brain, Terminal, CheckCircle, MessageSquare, AlertCircle, Search, Copy",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Activity view design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Extensibility View premium design",
    "design_doc": "specs/design/pages/extensibility-view.md",
    "steps": [
      "Read specs/design/pages/extensibility-view.md for complete design spec",
      "Migrate to shadcn Tabs component",
      "Implement Workflows tab with workflow cards",
      "Implement Artifacts tab with grid layout and type badges",
      "Implement Research tab with progress indicators",
      "Implement Methodologies tab with activation toggles",
      "Use Lucide icons per tab type",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Extensibility view design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Task Detail View premium design",
    "design_doc": "specs/design/pages/task-detail.md",
    "steps": [
      "Read specs/design/pages/task-detail.md for complete design spec",
      "Migrate to shadcn Dialog with XLarge sizing (max-w-xl)",
      "Implement backdrop blur and scale animation",
      "Implement header with title, status Badge, priority indicator",
      "Implement content sections with shadcn Collapsible (Description, Steps, Reviews, QA)",
      "Implement StateHistoryTimeline with connected dots",
      "Use Lucide icons: X, CheckCircle, ChevronDown, etc.",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Task Detail design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Reviews Panel premium design",
    "design_doc": "specs/design/pages/reviews-panel.md",
    "steps": [
      "Read specs/design/pages/reviews-panel.md for complete design spec",
      "Implement slide-in panel with animation",
      "Implement filter tabs using shadcn Tabs (pills variant)",
      "Implement ReviewCard with reviewer type indicator, status badge, action buttons",
      "Style fix attempt counter",
      "Use Lucide icons: CheckCircle, XCircle, AlertCircle, Eye, etc.",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Reviews panel design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Chat Panel premium design",
    "design_doc": "specs/design/pages/chat-panel.md",
    "steps": [
      "Read specs/design/pages/chat-panel.md for complete design spec",
      "Implement resizable panel with drag handle",
      "Implement message bubbles with asymmetric corners (user vs assistant)",
      "Implement markdown rendering and code block syntax highlighting",
      "Implement chat input with shadcn Input, attach button, Send button",
      "Implement context indicator in header",
      "Use Lucide icons: MessageSquare, Send, Paperclip, Loader2, Code",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Chat panel design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement QA Components premium design",
    "design_doc": "specs/design/pages/qa-components.md",
    "steps": [
      "Read specs/design/pages/qa-components.md for complete design spec",
      "Implement TaskQABadge with shadcn Badge and state-specific icons/colors",
      "Implement TaskDetailQAPanel with tabs interface",
      "Implement QASettingsPanel with shadcn Switch and Input",
      "Implement ScreenshotGallery with thumbnail grid and lightbox",
      "Implement Expected vs Actual comparison view",
      "Use Lucide icons per QA state: Clock, PlayCircle, CheckCircle, XCircle, AlertTriangle",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium QA components design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Project Sidebar premium design",
    "design_doc": "specs/design/pages/project-sidebar.md",
    "steps": [
      "Read specs/design/pages/project-sidebar.md for complete design spec",
      "Implement fixed sidebar with proper width (240px) and border",
      "Implement project list with git mode indicators and dirty status dots",
      "Implement WorktreeStatus with GitBranch icon",
      "Implement New Project button with shadcn Button",
      "Implement navigation items with active state accent indicator",
      "Use Lucide icons: FolderOpen, GitBranch, Plus, ChevronRight",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Project Sidebar design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Project Dialogs premium design",
    "design_doc": "specs/design/pages/project-dialogs.md",
    "steps": [
      "Read specs/design/pages/project-dialogs.md for complete design spec",
      "Migrate all dialogs to shadcn Dialog with backdrop blur and scale animation",
      "Implement ProjectCreationWizard with form validation, git mode radio selection",
      "Implement MergeWorkflowDialog with radio options and warning states",
      "Implement TaskRerunDialog with commit info and recommended badge",
      "Ensure consistent header/footer patterns per Modal Standards",
      "Use Lucide icons: FolderOpen, GitMerge, GitBranch, RefreshCw, AlertCircle",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Project dialogs design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Diff Viewer premium design",
    "design_doc": "specs/design/pages/diff-viewer.md",
    "steps": [
      "Read specs/design/pages/diff-viewer.md for complete design spec",
      "Migrate to shadcn Tabs component",
      "Implement file tree with collapsible directories and file status icons",
      "Implement diff panel with line numbers, proper line highlighting, syntax highlighting",
      "Implement commit list with short SHA and metadata",
      "Add Open in IDE button",
      "Use Lucide icons: Plus, Edit, Minus, ArrowRight, FolderOpen, File, ExternalLink",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Diff Viewer design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Execution Control Bar premium design",
    "design_doc": "specs/design/pages/execution-control-bar.md",
    "steps": [
      "Read specs/design/pages/execution-control-bar.md for complete design spec",
      "Implement fixed bottom bar with shadow and border-top",
      "Implement animated status dot (pulsing green when running, amber when paused)",
      "Implement running/queued counts display",
      "Implement Pause/Resume button with shadcn Button and Tooltip",
      "Implement Stop button with destructive styling",
      "Implement current task name display with Loader2 spinner",
      "Use Lucide icons: Pause, Play, Square, Loader2",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Execution Control design"
    ],
    "passes": true
  },
  {
    "category": "implementation",
    "description": "Implement Header and Navigation premium design",
    "design_doc": "specs/design/pages/header-navigation.md",
    "steps": [
      "Read specs/design/pages/header-navigation.md for complete design spec",
      "Implement fixed 48px header with shadow and Tauri drag region",
      "Implement RalphX branding with accent color",
      "Implement view navigation with shadcn Button (ghost), active state styling",
      "Implement Project Selector with shadcn DropdownMenu",
      "Implement Chat toggle with ⌘K shortcut badge",
      "Implement Reviews toggle with pending count badge",
      "Register keyboard shortcuts (⌘1-5 for views, ⌘K for chat)",
      "Use Lucide icons: LayoutGrid, Lightbulb, Puzzle, Activity, SlidersHorizontal, FolderOpen, ChevronDown, MessageSquare, CheckCircle",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: implement premium Header design"
    ],
    "passes": false
  },
  {
    "category": "implementation",
    "description": "Standardize all modals to common pattern",
    "design_doc": "specs/design/pages/modal-standards.md",
    "steps": [
      "Read specs/design/pages/modal-standards.md for complete design spec",
      "Audit all existing modals in the app for consistency",
      "Migrate AskUserQuestionModal to shadcn Dialog (Medium size)",
      "Migrate ReviewNotesModal to shadcn Dialog (Medium size)",
      "Migrate ProposalEditModal to shadcn Dialog (Large size)",
      "Ensure consistent backdrop (rgba(0,0,0,0.6) + blur(8px))",
      "Ensure consistent animation (scale 0.95→1, 200ms)",
      "Ensure consistent header pattern (icon, title, close button)",
      "Ensure consistent footer pattern (cancel ghost, primary right-aligned)",
      "Use Lucide icons: X, Loader2, CheckCircle, AlertCircle, etc.",
      "Run npm run lint && npm run typecheck",
      "Commit: feat: standardize all modals to premium pattern"
    ],
    "passes": false
  },
  {
    "category": "verification",
    "description": "Final anti-AI-slop compliance check",
    "steps": [
      "Run npm run lint && npm run typecheck",
      "Final anti-AI-slop compliance check:",
      "  - NO purple/blue gradients anywhere",
      "  - NO Inter font (must be SF Pro)",
      "  - Warm orange accent (#ff6b35) used correctly",
      "  - Layered shadows on all elevated elements",
      "  - Micro-interactions feel polished",
      "Document any remaining issues",
      "Fix any remaining issues using /frontend-design skill",
      "Commit: feat: complete Phase 14 design implementation"
    ],
    "passes": false
  }
]
```

---

## Page Design Documents Reference

Each page has a detailed design specification in `specs/design/pages/`:

| Page/Component | Design Doc | Key Elements |
|----------------|------------|--------------|
| Kanban Board | [kanban-board.md](../design/pages/kanban-board.md) | TaskBoard, Column, TaskCard, drag/drop states |
| Ideation View | [ideation-view.md](../design/pages/ideation-view.md) | Two-panel layout, chat bubbles, ProposalCard |
| Settings View | [settings-view.md](../design/pages/settings-view.md) | Section cards, Switch/Input/Select controls |
| Activity Stream | [activity-stream.md](../design/pages/activity-stream.md) | Search/filter, entry types, expandable details |
| Extensibility View | [extensibility-view.md](../design/pages/extensibility-view.md) | Tabs, workflow/artifact/research/methodology cards |
| Task Detail | [task-detail.md](../design/pages/task-detail.md) | Modal, collapsible sections, timeline |
| Reviews Panel | [reviews-panel.md](../design/pages/reviews-panel.md) | Slide-in panel, ReviewCard, filter tabs |
| Chat Panel | [chat-panel.md](../design/pages/chat-panel.md) | Resizable panel, message bubbles, input |
| QA Components | [qa-components.md](../design/pages/qa-components.md) | QA badges, screenshot gallery, lightbox |
| Project Sidebar | [project-sidebar.md](../design/pages/project-sidebar.md) | Project list, git status, navigation |
| Project Dialogs | [project-dialogs.md](../design/pages/project-dialogs.md) | Creation wizard, merge dialog, re-run dialog |
| Diff Viewer | [diff-viewer.md](../design/pages/diff-viewer.md) | File tree, diff panel, commit list |
| Execution Control Bar | [execution-control-bar.md](../design/pages/execution-control-bar.md) | Status dot, counts, pause/stop buttons |
| Header & Navigation | [header-navigation.md](../design/pages/header-navigation.md) | Nav items, project selector, toggle buttons |
| Modal Standards | [modal-standards.md](../design/pages/modal-standards.md) | Base patterns, sizes, header/footer |

Each design doc contains:
- **Layout Structure** - Container properties, positioning
- **Component Hierarchy** - Visual tree of components
- **Styling Details** - CSS properties, Tailwind classes
- **State Variations** - Default, hover, active, disabled
- **Lucide Icons** - Specific icons to use with sizes
- **Acceptance Criteria** - Functional requirements checklist
- **Design Quality Checklist** - Visual quality requirements

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
