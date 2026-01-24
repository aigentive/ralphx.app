# Extensibility View

Design requirements for the Extensibility View - a tabbed interface for managing Workflows, Artifacts, Research, and Methodologies.

---

## Overview

The Extensibility View is RalphX's power-user hub for customizing workflows, browsing artifacts, launching research sessions, and managing development methodologies. It should feel like a well-organized control center - capable but not overwhelming.

**Design Inspiration:**
- Linear's settings panels (clean sections, clear hierarchy)
- Raycast's extension browser (card-based browsing, search/filter)
- Notion's database views (tabs, toggle between display modes)

**Aesthetic Direction:** Professional utility with warmth. Clean, functional, but not cold or clinical. The warm orange accent should punctuate key actions without dominating.

---

## Layout Structure

### Overall Container

```
┌─────────────────────────────────────────────────────────────┐
│  [Workflows]  [Artifacts]  [Research]  [Methodologies]      │  ← Tab Navigation
├─────────────────────────────────────────────────────────────┤
│                                                             │
│                     Tab Content Area                        │
│                   (flex-1, overflow-auto)                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

- **Container**: Full height (`h-full`), flex column
- **Background**: `--bg-base` with subtle warm radial gradient in corner
- **Content padding**: 24px (`--space-6`)

### Background Treatment

```css
background:
  radial-gradient(
    ellipse 600px 400px at 100% 100%,
    rgba(255, 107, 53, 0.03) 0%,
    transparent 70%
  ),
  var(--bg-base);
```

---

## Tab Navigation

### Structure

Using **shadcn Tabs** with underline indicator style.

```tsx
<Tabs defaultValue="workflows" className="h-full flex flex-col">
  <TabsList className="...">
    <TabsTrigger value="workflows">
      <Workflow className="w-4 h-4 mr-2" />
      Workflows
    </TabsTrigger>
    {/* ... other tabs */}
  </TabsList>
  <TabsContent value="workflows" className="flex-1 overflow-auto">
    {/* content */}
  </TabsContent>
</Tabs>
```

### Tab Styling

| Property | Value | Notes |
|----------|-------|-------|
| Height | 44px | Larger touch targets |
| Padding | 16px horizontal, 12px vertical | Comfortable spacing |
| Font | 14px, font-medium | Clear but not heavy |
| Gap between tabs | 8px | Subtle separation |
| Border bottom | 1px `--border-subtle` | Full-width divider |

### Tab States

| State | Text Color | Border | Background |
|-------|------------|--------|------------|
| Default | `--text-muted` | transparent | transparent |
| Hover | `--text-secondary` | transparent | `--bg-hover` (subtle) |
| Active | `--text-primary` | 2px `--accent-primary` | transparent |
| Focus | `--text-primary` | 2px `--accent-primary` | `--shadow-glow` |

### Tab Icons (Lucide)

| Tab | Icon | Size |
|-----|------|------|
| Workflows | `Workflow` | 16px |
| Artifacts | `FileBox` | 16px |
| Research | `Search` | 16px |
| Methodologies | `BookOpen` | 16px |

### Tab Animation

- Underline slides with `transition-all duration-200 ease-smooth`
- Icon and text color transition together
- Active underline uses CSS `::after` pseudo-element for slide effect

---

## Workflows Tab

### Layout

```
┌────────────────────────────────────────────────────────────┐
│  Workflow Schemas                          [+ New Workflow]│
├────────────────────────────────────────────────────────────┤
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 🔶 Default Kanban                          [DEFAULT] │  │
│  │    Standard development workflow                     │  │
│  │    6 columns · Created Jan 2026                      │  │
│  │                                    [Edit] [Duplicate]│  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │    Sprint Workflow                                   │  │
│  │    Two-week sprint cycle                             │  │
│  │    8 columns · Created Jan 2026                      │  │
│  │                          [Edit] [Duplicate] [Delete] │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Header

- **Title**: "Workflow Schemas" - text-lg, font-semibold, `--text-primary`
- **New Workflow button**: shadcn Button (secondary variant)
  - Icon: Lucide `Plus` (16px)
  - Text: "New Workflow"

### Workflow Card

Using **shadcn Card** with custom styling.

```tsx
<Card className="group hover:shadow-sm transition-all duration-150">
  <CardContent className="p-4">
    {/* content */}
  </CardContent>
</Card>
```

| Property | Value |
|----------|-------|
| Background | `--bg-surface` |
| Border | 1px `--border-subtle` |
| Border radius | `--radius-md` (8px) |
| Padding | 16px |
| Margin between cards | 12px |
| Hover | `translateY(-1px)`, `--shadow-xs` |

### Card Content

1. **Header Row**
   - Active indicator: 8px circle, `--accent-primary` (only for active workflow)
   - Name: text-sm, font-medium, `--text-primary`
   - Default badge: shadcn Badge (secondary), "DEFAULT" text

2. **Description**
   - text-sm, `--text-secondary`
   - Max 2 lines, truncate with ellipsis

3. **Metadata Row**
   - Column count: "N columns"
   - Created date: "Created MMM YYYY"
   - text-xs, `--text-muted`
   - Separator: `·` character

4. **Action Buttons** (appear on hover, group-hover)
   - Edit: Ghost button, Lucide `Edit` icon
   - Duplicate: Ghost button, Lucide `Copy` icon
   - Delete: Ghost button, Lucide `Trash2` icon (not for default)
   - Buttons: 28px height, icon only with tooltip

### Workflow Editor (Modal)

When editing, open in a **shadcn Dialog** (medium size, max-w-md).

- Form fields using shadcn Input, Label
- Column list with drag handles (Lucide `GripVertical`)
- Status mapping using shadcn Select
- Save/Cancel buttons in footer

### Empty State

```
┌────────────────────────────────────────────────────────────┐
│                                                            │
│                    ┌─────────────────┐                     │
│                    │   (Workflow)    │                     │
│                    │    48px icon    │                     │
│                    │    dashed       │                     │
│                    └─────────────────┘                     │
│                                                            │
│              No custom workflows yet                       │
│         Create a workflow to organize tasks                │
│                                                            │
│                   [+ Create Workflow]                      │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

---

## Artifacts Tab

### Layout

```
┌────────────────────────────────────────────────────────────┐
│  [🔍 Search artifacts...]          [≡ List] [⊞ Grid] [⇵]  │
├──────────────┬─────────────────────────────────────────────┤
│              │                                             │
│  Buckets     │  Artifacts in "Project Docs"               │
│  ──────────  │                                             │
│  ▸ All (24)  │  ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  ▸ System (8)│  │ 📄       │ │ 📊       │ │ 📋       │   │
│  ▸ PRDs (6)  │  │ PRD.md   │ │ Data.csv │ │ Notes    │   │
│  ▸ Docs (10) │  │ 2.4 KB   │ │ 1.1 MB   │ │ 512 B    │   │
│              │  └──────────┘ └──────────┘ └──────────┘   │
│              │                                             │
└──────────────┴─────────────────────────────────────────────┘
```

### Search & Filter Bar

- **Search input**: shadcn Input with Lucide `Search` icon prefix
  - Placeholder: "Search artifacts..."
  - Width: flex-1

- **View toggle**: Two icon buttons (List/Grid)
  - List: Lucide `List`
  - Grid: Lucide `LayoutGrid`
  - Active state: `--bg-hover` background

- **Sort dropdown**: shadcn DropdownMenu
  - Icon: Lucide `ArrowUpDown`
  - Options: Name, Date, Size, Type

### Bucket Sidebar

| Property | Value |
|----------|-------|
| Width | 200px, fixed |
| Background | `--bg-surface` |
| Border right | 1px `--border-subtle` |
| Padding | 12px |

**Bucket Item:**
- Height: 36px
- Padding: 8px 12px
- Border radius: `--radius-sm`
- Selected: `--bg-hover` background, `--text-primary` text
- Hover: `--bg-hover` background
- Count badge: right-aligned, text-xs, `--text-muted`
- System badge: "S" badge after name for system buckets

### Artifact Cards (Grid View)

```tsx
<Card className="group cursor-pointer hover:border-accent-primary/30">
  <CardContent className="p-3 text-center">
    <FileIcon type={artifact.type} className="w-8 h-8 mx-auto mb-2" />
    <p className="text-sm truncate">{artifact.name}</p>
    <p className="text-xs text-muted">{formatSize(artifact.size)}</p>
  </CardContent>
</Card>
```

| Property | Value |
|----------|-------|
| Grid | 3-4 columns, responsive |
| Gap | 12px |
| Card size | ~120px min-width |
| Border radius | `--radius-md` |
| Hover | border color shift to `--accent-primary` at 30% opacity |

**File Type Icons (Lucide):**
- Markdown: `FileText`
- JSON: `FileJson`
- Image: `Image`
- Code: `FileCode`
- Default: `File`

### Artifact Cards (List View)

- Full-width rows
- Icon + name + type badge + size + date
- 44px row height
- Hover background: `--bg-hover`

### Preview Panel (Future Enhancement)

When artifact selected, show preview panel on right (400px width):
- Document preview for text files
- Image preview for images
- Metadata display for binary files

### Empty States

**No bucket selected:**
"Select a bucket to view artifacts"

**Empty bucket:**
"No artifacts in this bucket"
With Lucide `FileBox` icon (48px, dashed style)

---

## Research Tab

### Layout

```
┌────────────────────────────────────────────────────────────┐
│                                                            │
│                   Launch New Research                      │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Research Question                                    │  │
│  │ ┌──────────────────────────────────────────────────┐ │  │
│  │ │ What are the best practices for...              │ │  │
│  │ │                                                  │ │  │
│  │ └──────────────────────────────────────────────────┘ │  │
│  │                                                      │  │
│  │ ┌────────────────────┐ ┌────────────────────┐       │  │
│  │ │ Context (optional) │ │ Scope (optional)   │       │  │
│  │ └────────────────────┘ └────────────────────┘       │  │
│  │                                                      │  │
│  │ Research Depth                                       │  │
│  │ ┌─────────────┐ ┌─────────────┐                     │  │
│  │ │ Quick Scan  │ │ Standard ✓  │                     │  │
│  │ │ 25 iter, 1h │ │ 100 iter 4h │                     │  │
│  │ └─────────────┘ └─────────────┘                     │  │
│  │ ┌─────────────┐ ┌─────────────┐                     │  │
│  │ │ Deep Dive   │ │ Exhaustive  │                     │  │
│  │ │ 250 iter 8h │ │ 500 iter 24h│                     │  │
│  │ └─────────────┘ └─────────────┘                     │  │
│  │ ┌─────────────┐                                     │  │
│  │ │ Custom      │  [Iterations: 100] [Timeout: 4h]   │  │
│  │ └─────────────┘                                     │  │
│  │                                                      │  │
│  │                          [Cancel]  [🚀 Launch]      │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ─────────────────────────────────────────────────────────│
│                                                            │
│  Recent Research Sessions                                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 🔍 Best practices for state management    [Complete] │  │
│  │    Standard · 45 iterations · 2h 15m · Jan 24       │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Research Launcher Card

Using **shadcn Card** as container.

| Property | Value |
|----------|-------|
| Max width | 600px, centered |
| Padding | 24px |
| Background | `--bg-surface` |
| Border radius | `--radius-lg` |
| Shadow | `--shadow-xs` |

### Form Fields

**Research Question (required):**
- shadcn Textarea (3 rows)
- Label: font-medium, `--text-primary`
- Placeholder: "What do you want to research?"
- Focus ring: `--shadow-glow`

**Context & Scope (optional):**
- Two-column grid, gap-12px
- shadcn Input
- Label: text-xs, `--text-secondary`
- Placeholder text in muted color

### Depth Preset Selector

Radio button group with card-style options.

```tsx
<RadioGroup defaultValue="standard" className="grid grid-cols-2 gap-3">
  <RadioGroupItem value="quick-scan" className="...">
    <Label>
      <Zap className="w-4 h-4 mb-1" />
      Quick Scan
      <span className="text-xs text-muted">25 iterations, 1h</span>
    </Label>
  </RadioGroupItem>
  {/* ... */}
</RadioGroup>
```

| Preset | Icon | Iterations | Timeout |
|--------|------|------------|---------|
| Quick Scan | `Zap` | 25 | 1h |
| Standard | `Target` | 100 | 4h |
| Deep Dive | `Telescope` | 250 | 8h |
| Exhaustive | `Microscope` | 500 | 24h |
| Custom | `Sliders` | User-defined | User-defined |

**Preset Card Styling:**
- Border: 1px `--border-subtle`
- Selected: 2px `--accent-primary` border
- Padding: 12px
- Border radius: `--radius-md`
- Icon color: `--text-muted` (selected: `--accent-primary`)

### Custom Depth Inputs

Revealed when "Custom" is selected (slide-down animation).

- Two-column grid
- shadcn Input with number type
- Labels: "Max Iterations", "Timeout (hours)"
- Min values validated

### Action Buttons

- **Cancel**: Ghost button, left-aligned in footer
- **Launch**: Primary button with Lucide `Rocket` icon
  - Loading state: "Launching..." with Lucide `Loader2` (animated spin)

### Progress Indicator (When Research Running)

Replace launcher with progress display:

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  🔍 Researching: "Best practices for state management"  │
│                                                          │
│  ████████████████░░░░░░░░░░░░░░  45 / 100 iterations    │
│                                                          │
│  Current: Analyzing React Query patterns...              │
│                                                          │
│  Elapsed: 2h 15m     Remaining: ~1h 45m                 │
│                                                          │
│                              [Pause]  [Stop]             │
└──────────────────────────────────────────────────────────┘
```

- Progress bar: `--accent-primary` fill on `--bg-base` track
- Current step: text-sm, `--text-secondary`
- Time display: text-xs, `--text-muted`

### Recent Sessions List

Below the launcher, showing completed/paused research.

- Session cards with:
  - Icon: Lucide `Search`
  - Question (truncated)
  - Status badge: Complete (green), Paused (amber), Failed (red)
  - Metadata: Depth preset, iterations, duration, date
- Click to view results (future: expand to show findings)

---

## Methodologies Tab

### Layout

```
┌────────────────────────────────────────────────────────────┐
│  Development Methodologies                                 │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 🔶 BMAD (Breakthrough Method for AI Development)    │  │
│  │                                              [ACTIVE] │  │
│  │                                                      │  │
│  │ A structured approach to AI-assisted development    │  │
│  │ with clear phases and agent coordination.           │  │
│  │                                                      │  │
│  │ 5 phases · 3 agents · Default Kanban workflow       │  │
│  │                                                      │  │
│  │                                        [Deactivate]  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │    GSD (Get Stuff Done)                              │  │
│  │                                                      │  │
│  │ Minimal process methodology focused on shipping     │  │
│  │ quickly with lightweight review cycles.             │  │
│  │                                                      │  │
│  │ 3 phases · 2 agents · Sprint workflow               │  │
│  │                                                      │  │
│  │                                          [Activate]  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Header

- **Title**: "Development Methodologies" - text-lg, font-semibold
- Optional: "Choose how RalphX organizes work" subtitle

### Methodology Card

Using **shadcn Card** with interactive styling.

| Property | Value |
|----------|-------|
| Background | `--bg-surface` |
| Border | 1px `--border-subtle` |
| Active border | 2px `--accent-primary` |
| Border radius | `--radius-lg` (12px) |
| Padding | 20px |
| Margin between cards | 16px |
| Cursor | pointer |
| Hover | `--shadow-xs`, slight `translateY(-1px)` |

### Card Content

1. **Header Row**
   - Active indicator: 10px filled circle, `--accent-primary` (pulsing glow animation for active)
   - Name: text-base, font-semibold, `--text-primary`
   - Active badge: shadcn Badge (success variant), "ACTIVE"

2. **Description**
   - text-sm, `--text-secondary`
   - 2-3 lines, line-height: 1.5
   - Margin: 12px top/bottom

3. **Stats Row**
   - Format: "N phases · N agents · Workflow name"
   - text-xs, `--text-muted`
   - Icons optional: `Layers` for phases, `Users` for agents, `Workflow` for workflow

4. **Action Button** (right-aligned in header)
   - Active methodology: "Deactivate" - Ghost button, destructive intent
   - Inactive: "Activate" - Primary button with accent color
   - Loading state during activation with Lucide `Loader2`

### Active State Enhancement

```css
.methodology-card-active {
  border: 2px solid var(--accent-primary);
  background: linear-gradient(
    135deg,
    var(--bg-surface) 0%,
    rgba(255, 107, 53, 0.03) 100%
  );
}

.active-indicator {
  animation: pulse-glow 2s ease-in-out infinite;
}

@keyframes pulse-glow {
  0%, 100% { box-shadow: 0 0 0 0 rgba(255, 107, 53, 0.4); }
  50% { box-shadow: 0 0 0 4px rgba(255, 107, 53, 0.1); }
}
```

### Click to Select

Clicking card (not activate/deactivate button) selects for details view:
- Selected: Shows methodology details panel below or to the right
- Details include:
  - Full description
  - Phase breakdown with column mappings
  - Agent profiles list
  - Associated skills
  - Templates

### Empty State

```
┌────────────────────────────────────────────────────────────┐
│                                                            │
│                    ┌─────────────────┐                     │
│                    │   (BookOpen)    │                     │
│                    │    48px icon    │                     │
│                    │    dashed       │                     │
│                    └─────────────────┘                     │
│                                                            │
│              No methodologies available                    │
│       Configure methodologies in the plugin directory      │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

---

## Component Hierarchy

```
ExtensibilityView
├── Tabs (shadcn)
│   ├── TabsList
│   │   ├── TabsTrigger[workflows] + Icon(Workflow)
│   │   ├── TabsTrigger[artifacts] + Icon(FileBox)
│   │   ├── TabsTrigger[research] + Icon(Search)
│   │   └── TabsTrigger[methodologies] + Icon(BookOpen)
│   │
│   ├── TabsContent[workflows]
│   │   └── WorkflowsPanel
│   │       ├── Header (title + NewWorkflowButton)
│   │       ├── WorkflowCardList
│   │       │   └── WorkflowCard[]
│   │       │       ├── ActiveIndicator
│   │       │       ├── Name + DefaultBadge
│   │       │       ├── Description
│   │       │       ├── Metadata (columns, date)
│   │       │       └── ActionButtons (edit, duplicate, delete)
│   │       └── EmptyState (if no workflows)
│   │
│   ├── TabsContent[artifacts]
│   │   └── ArtifactsPanel
│   │       ├── SearchFilterBar
│   │       │   ├── SearchInput
│   │       │   ├── ViewToggle (list/grid)
│   │       │   └── SortDropdown
│   │       ├── SplitLayout
│   │       │   ├── BucketSidebar
│   │       │   │   └── BucketItem[]
│   │       │   └── ArtifactDisplay
│   │       │       ├── ArtifactGrid (grid view)
│   │       │       │   └── ArtifactCard[]
│   │       │       └── ArtifactList (list view)
│   │       │           └── ArtifactRow[]
│   │       └── EmptyState (various contexts)
│   │
│   ├── TabsContent[research]
│   │   └── ResearchPanel
│   │       ├── ResearchLauncher (Card)
│   │       │   ├── QuestionInput (Textarea)
│   │       │   ├── ContextScopeRow (2 Inputs)
│   │       │   ├── DepthPresetSelector (RadioGroup)
│   │       │   │   └── PresetCard[] + CustomCard
│   │       │   ├── CustomDepthInputs (conditional)
│   │       │   └── ActionButtons (Cancel, Launch)
│   │       ├── ResearchProgress (when running)
│   │       │   ├── ProgressBar
│   │       │   ├── CurrentStep
│   │       │   └── ControlButtons (Pause, Stop)
│   │       └── RecentSessionsList
│   │           └── SessionCard[]
│   │
│   └── TabsContent[methodologies]
│       └── MethodologiesPanel
│           ├── Header (title)
│           ├── MethodologyCardList
│           │   └── MethodologyCard[]
│           │       ├── ActiveIndicator (pulsing)
│           │       ├── Name + ActiveBadge
│           │       ├── Description
│           │       ├── Stats (phases, agents, workflow)
│           │       └── ActivateButton
│           ├── MethodologyDetails (when selected)
│           └── EmptyState (if no methodologies)
```

---

## Lucide Icons Used

| Icon | Usage |
|------|-------|
| `Workflow` | Workflows tab, workflow cards |
| `FileBox` | Artifacts tab |
| `Search` | Research tab, search input |
| `BookOpen` | Methodologies tab |
| `Plus` | Add buttons |
| `Edit` | Edit action |
| `Copy` | Duplicate action |
| `Trash2` | Delete action |
| `GripVertical` | Drag handles |
| `List` | List view toggle |
| `LayoutGrid` | Grid view toggle |
| `ArrowUpDown` | Sort button |
| `FileText` | Markdown files |
| `FileJson` | JSON files |
| `FileCode` | Code files |
| `Image` | Image files |
| `File` | Generic file |
| `Zap` | Quick scan preset |
| `Target` | Standard preset |
| `Telescope` | Deep dive preset |
| `Microscope` | Exhaustive preset |
| `Sliders` | Custom preset |
| `Rocket` | Launch button |
| `Loader2` | Loading states (animated) |
| `Layers` | Phase count |
| `Users` | Agent count |
| `CheckCircle` | Complete status |
| `PauseCircle` | Paused status |
| `XCircle` | Failed status |
| `ChevronRight` | Expand/navigate |

---

## Acceptance Criteria

### Functional Requirements

1. [ ] Tab navigation works correctly with keyboard (Tab, Arrow keys, Enter)
2. [ ] All four tabs render their respective content panels
3. [ ] Tab state persists during session (not on reload)
4. [ ] ARIA attributes present: role="tablist", role="tab", aria-selected
5. [ ] Only one methodology can be active at a time
6. [ ] Activate/deactivate mutations update UI optimistically
7. [ ] Workflow editor opens in modal dialog
8. [ ] Workflow changes save correctly to backend
9. [ ] Bucket selection filters artifact list correctly
10. [ ] Artifact grid/list view toggle works
11. [ ] Search input filters artifacts by name
12. [ ] Research launcher validates required question field
13. [ ] Custom depth inputs appear when "Custom" selected
14. [ ] Launch button disabled when form invalid
15. [ ] Loading states shown during async operations
16. [ ] Error states handled gracefully with user feedback
17. [ ] Empty states render for each panel when data is empty
18. [ ] Cards are keyboard navigable (Tab, Enter to select)
19. [ ] Focus management correct when opening/closing modals
20. [ ] Responsive behavior: tabs stack on narrow screens

---

## Design Quality Checklist

### General

1. [ ] Background uses subtle warm gradient, not flat `--bg-base`
2. [ ] Tab navigation is 44px height with clear active indicator
3. [ ] Active tab has 2px `--accent-primary` underline with slide animation
4. [ ] Tab icons are 16px, match text color
5. [ ] Content area has 24px padding
6. [ ] All cards use shadcn Card with consistent styling
7. [ ] Card hover states include `translateY(-1px)` and shadow elevation
8. [ ] Empty states use 48px dashed icons
9. [ ] Primary actions use `--accent-primary` button
10. [ ] Secondary actions use ghost or secondary button variants

### Typography

11. [ ] Section titles: text-lg, font-semibold, `--text-primary`
12. [ ] Card titles: text-sm or text-base, font-medium
13. [ ] Descriptions: text-sm, `--text-secondary`, leading-relaxed
14. [ ] Metadata: text-xs, `--text-muted`
15. [ ] No Inter font - all text uses SF Pro (system font)
16. [ ] Titles have `letter-spacing: -0.02em`

### Colors & Theming

17. [ ] No purple or blue gradients anywhere
18. [ ] Accent color is warm orange `#ff6b35`
19. [ ] Active methodology has orange border and pulsing indicator
20. [ ] Status badges use semantic colors (success, warning, error)

### Interactions

21. [ ] All interactive elements have visible focus states
22. [ ] Buttons have press feedback (`scale(0.98)`)
23. [ ] Cards have hover lift animation (150ms ease)
24. [ ] Tab underline slides smoothly between tabs
25. [ ] Loading states use `Loader2` with spin animation
26. [ ] Transitions use `ease-smooth` timing function
27. [ ] Modal open uses scale animation (0.95 → 1.0)

### Shadows & Depth

28. [ ] Cards at rest: `--shadow-xs` or border only
29. [ ] Cards on hover: `--shadow-sm`
30. [ ] Modals: `--shadow-lg`
31. [ ] Focus rings: `--shadow-glow`
32. [ ] Active methodology card has subtle gradient background

### Spacing

33. [ ] 8pt grid alignment maintained
34. [ ] Card padding: 16-20px
35. [ ] Gap between cards: 12-16px
36. [ ] Section gaps: 24-32px
37. [ ] Button padding: 12-16px horizontal

### Accessibility

38. [ ] Color contrast meets WCAG AA (4.5:1 for text)
39. [ ] All interactive elements are keyboard accessible
40. [ ] ARIA labels on icon-only buttons
41. [ ] Screen reader text for status indicators

---

## Implementation Notes

### shadcn Components to Use

- `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent`
- `Card`, `CardContent`, `CardHeader`, `CardFooter`
- `Button` (primary, secondary, ghost, destructive variants)
- `Badge` (default, success, warning, destructive variants)
- `Input`, `Textarea`, `Label`
- `Select`, `SelectTrigger`, `SelectContent`, `SelectItem`
- `Dialog`, `DialogContent`, `DialogHeader`, `DialogFooter`
- `RadioGroup`, `RadioGroupItem`
- `ScrollArea`
- `Tooltip`, `TooltipTrigger`, `TooltipContent`
- `DropdownMenu`, `DropdownMenuTrigger`, `DropdownMenuContent`
- `Skeleton` (for loading states)

### CSS Custom Properties to Reference

```css
/* From DESIGN.md */
--bg-base: #0f0f0f;
--bg-surface: #1a1a1a;
--bg-elevated: #242424;
--bg-hover: #2d2d2d;
--text-primary: #f0f0f0;
--text-secondary: #a0a0a0;
--text-muted: #666666;
--accent-primary: #ff6b35;
--border-subtle: rgba(255, 255, 255, 0.06);
--border-default: rgba(255, 255, 255, 0.1);
--radius-sm: 4px;
--radius-md: 8px;
--radius-lg: 12px;
--shadow-xs: 0 1px 2px rgba(0,0,0,0.2), 0 1px 3px rgba(0,0,0,0.1);
--shadow-sm: 0 1px 2px rgba(0,0,0,0.3), 0 2px 4px rgba(0,0,0,0.2);
--shadow-glow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
```

### Key Animation Keyframes

```css
@keyframes pulse-glow {
  0%, 100% { box-shadow: 0 0 0 0 rgba(255, 107, 53, 0.4); }
  50% { box-shadow: 0 0 0 4px rgba(255, 107, 53, 0.1); }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
```

---

## References

- [DESIGN.md](../DESIGN.md) - Master design system
- [specs/DESIGN_OVERHAUL_PLAN.md](../../DESIGN_OVERHAUL_PLAN.md) - Design overhaul strategy
- Linear.app - Settings panel reference
- Raycast - Extension browser reference
- shadcn/ui documentation - Component API
- Lucide icons - https://lucide.dev/icons/
