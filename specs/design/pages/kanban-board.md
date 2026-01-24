# Kanban Board

The Kanban view is the primary interface for task management. It should feel like a premium native Mac app with physical depth and dimensionality.

**Reference Inspiration**: Linear (card interactions, keyboard-first), Raycast (Mac-native feel, spacing discipline)

## TaskBoard

**Layout & Structure:**
- Viewport-filling height (`calc(100vh - header - control-bar)`)
- Horizontal scroll with CSS scroll-snap for column alignment
- Fade edges at overflow boundaries (gradient masks)
- 24px (`--space-6`) gutters between columns

**Background:**
- Subtle radial gradient, NOT flat `--bg-base`
- Gradient: `radial-gradient(ellipse at top, rgba(255,107,53,0.03) 0%, var(--bg-base) 50%)`
- Creates warmth without being distracting

**Scroll Behavior:**
- Horizontal scroll with momentum (native Mac feel)
- Scroll snap to column start: `scroll-snap-type: x mandatory`
- Overflow fade: 32px gradient fade on left/right edges when content overflows

```css
.task-board {
  display: flex;
  gap: var(--space-6);
  padding: var(--space-6);
  overflow-x: auto;
  scroll-snap-type: x proximity;
  background: radial-gradient(ellipse at top, rgba(255,107,53,0.03) 0%, var(--bg-base) 50%);
  min-height: calc(100vh - 48px - 48px); /* header + control bar */
}

/* Fade edges */
.task-board::before,
.task-board::after {
  content: '';
  position: sticky;
  width: 32px;
  flex-shrink: 0;
  pointer-events: none;
}
.task-board::before { left: 0; background: linear-gradient(to right, var(--bg-base), transparent); }
.task-board::after { right: 0; background: linear-gradient(to left, var(--bg-base), transparent); }
```

## Column

**Structure:**
- Fixed width: 300px (min-width 280px, max-width 320px)
- Flex column layout with gap for cards
- Scroll snap alignment: `scroll-snap-align: start`

**Header:**
- Glass effect background: `rgba(26,26,26,0.85)` + `backdrop-filter: blur(12px)`
- Warm orange accent dot (6px) before column title
- Title: `text-sm`, `font-semibold`, `--tracking-tight`
- Task count: shadcn Badge (default variant) showing count

```tsx
<div className="column-header flex items-center gap-2 px-3 py-2 rounded-lg backdrop-blur-md bg-[rgba(26,26,26,0.85)]">
  <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-primary)]" />
  <h3 className="text-sm font-semibold tracking-tight flex-1">{title}</h3>
  <Badge variant="secondary">{count}</Badge>
</div>
```

**Drop Zone:**
- Default: transparent, dashed border (`--border-subtle`)
- Drag-over: Orange glow border + subtle tinted background
- Drag-over styles:
  ```css
  .column-drop-zone.drag-over {
    border: 2px dashed var(--accent-primary);
    background: var(--accent-muted);
    box-shadow: inset 0 0 20px rgba(255,107,53,0.1);
  }
  ```

**Empty State:**
- Centered vertically in column
- Dashed border container (2px dashed `--border-subtle`)
- Lucide icon: `Inbox` (24px, `--text-muted`)
- Text: "No tasks" (`text-sm`, `--text-muted`)
- Padding: 24px

```tsx
<div className="flex flex-col items-center justify-center gap-3 p-6 border-2 border-dashed border-[var(--border-subtle)] rounded-lg">
  <Inbox className="w-6 h-6 text-[var(--text-muted)]" />
  <p className="text-sm text-[var(--text-muted)]">No tasks</p>
</div>
```

## TaskCard

**Base Structure:**
- shadcn Card component with custom styling
- Padding: 12px (`--space-3`)
- Border radius: 8px (`--radius-md`)
- Background: `--bg-surface`
- Cursor: `grab` (when draggable)

**Shadow & Depth:**
- Rest state: `--shadow-xs` for subtle lift
- Creates physical card feel, not flat

**Priority Indicator:**
- 3px colored left border stripe (full height)
- Colors by priority:
  - Critical: `--status-error` (#ef4444)
  - High: `--status-warning` (#f59e0b)
  - Medium: `--accent-primary` (#ff6b35)
  - Low: `--text-muted` (#666666)
  - None: transparent (no stripe)

```css
.task-card {
  border-left: 3px solid var(--priority-color, transparent);
}
```

**Content Layout:**
- Title: `text-sm`, `font-medium`, `--text-primary`, single line with ellipsis
- Description: `text-xs`, `--text-secondary`, 2 line clamp
- Badge row: Flex wrap, gap-1.5, margin-top-2

**Badges:**
- Status badge: shadcn Badge with semantic variant
- QA badge: shadcn Badge (compact, shows QA state)
- All badges: `text-xs`, consistent height

**Hover State:**
```css
.task-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-sm);
  border-color: var(--border-default);
  transition: all 150ms ease;
}
```

**Drag Handle:**
- Lucide `GripVertical` icon (16px)
- Positioned top-right corner of card
- Visible only on hover (opacity transition)
- Color: `--text-muted`, hover: `--text-secondary`

```tsx
<div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity cursor-grab">
  <GripVertical className="w-4 h-4 text-[var(--text-muted)] hover:text-[var(--text-secondary)]" />
</div>
```

**Drag State:**
```css
.task-card.dragging {
  transform: scale(1.02) rotate(2deg);
  box-shadow: var(--shadow-md);
  opacity: 0.9;
  cursor: grabbing;
  z-index: 50;
}
```

**Selected State:**
```css
.task-card.selected {
  border: 2px solid var(--accent-primary);
  background: var(--accent-muted);
  box-shadow: 0 0 0 4px rgba(255,107,53,0.15);
}
```

**Keyboard Focus:**
```css
.task-card:focus-visible {
  outline: none;
  box-shadow: var(--shadow-glow);
}
```

## Component Hierarchy

```
TaskBoard
├── Column (Backlog)
│   ├── ColumnHeader (glass effect + badge)
│   └── DropZone
│       ├── TaskCard
│       │   ├── PriorityStripe (left border)
│       │   ├── DragHandle (hover-visible)
│       │   ├── Title
│       │   ├── Description
│       │   └── BadgeRow (Status, QA)
│       └── TaskCard...
├── Column (Ready)
│   └── ...
├── Column (In Progress)
│   └── ...
└── Column (Done)
    └── ...
```

## Acceptance Criteria

- TaskBoard fills available viewport height
- Horizontal scroll works with mouse wheel + trackpad
- Columns have consistent 300px width
- Column headers show task count with Badge
- TaskCards display priority stripe on left border
- TaskCards show hover lift animation (translateY -2px)
- Drag handle appears on card hover
- Dragging card shows rotation and elevated shadow
- Selected card has orange border and tinted background
- Empty columns show empty state with icon and text
- Drop zones highlight with orange glow during drag-over
- All interactive elements have visible focus states

## Design Quality Checklist

- NO purple or blue gradients anywhere
- Background uses subtle warm radial gradient (not flat)
- Shadows are layered (multiple values) for realistic depth
- Orange accent used sparingly - only for selection, focus, drag indicators
- Typography uses SF Pro with proper tracking (-0.02em for titles)
- All spacing follows 4px/8px grid
- Glass effect on column headers uses backdrop-blur
- Micro-interactions feel snappy (150ms transitions)
- Cards have physical weight - shadows create depth
- Empty states use Lucide icons (not custom SVGs)
- Badge styling consistent with shadcn Badge component
- Focus rings use --shadow-glow pattern
