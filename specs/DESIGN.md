# RalphX Design System

The definitive design guide for RalphX. All UI work must follow these specifications.

---

## 1. Design Philosophy

### Premium 10x Designer Aesthetic

RalphX is a native Mac app that should feel like it was crafted by a world-class designer. Every pixel matters. Every interaction is intentional.

**What makes it feel premium:**
- **Intentionality** - Every element has a purpose; no visual noise
- **Restraint** - Strategic use of color, especially accent (5% rule)
- **Polish** - Micro-interactions, layered shadows, refined typography
- **Consistency** - Same patterns applied everywhere
- **Depth** - Not flat; surfaces have subtle elevation and dimensionality

### Reference Apps
- **Linear** - Board layout, card interactions, keyboard-first UX
- **Raycast** - Mac-native feel, glass effects, spacing discipline
- **Arc** - Spatial organization, bold typography, warm palette
- **Vercel Dashboard** - Typography scale, dark theme execution, status indicators

---

## 2. Anti-AI-Slop Guardrails

**NEVER use these patterns:**
- ❌ Purple or blue gradients (the #1 AI design cliché)
- ❌ Inter font (use SF Pro instead)
- ❌ Generic icon grids (4x4 squares of identical icons)
- ❌ High saturation colors on dark backgrounds
- ❌ Flat, lifeless surfaces without depth
- ❌ Rainbow gradients or excessive color variety
- ❌ Overly rounded corners (pill shapes everywhere)
- ❌ Generic stock-photo-style illustrations

**ALWAYS use these patterns:**
- ✅ Warm orange accent (#ff6b35) - distinctive and intentional
- ✅ Layered shadows for realistic depth
- ✅ Micro-interactions (hover lift, press scale)
- ✅ Typography refinement (letter-spacing, line-height)
- ✅ Strategic accent usage (5% of surface area max)
- ✅ Glass effects only for modals/overlays
- ✅ Subtle gradients (nearly imperceptible)

---

## 3. Color System

### Backgrounds
Dark grays, NOT pure black. Create depth through subtle variation.

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-base` | `#0f0f0f` | Page background, main canvas |
| `--bg-surface` | `#1a1a1a` | Cards, panels, elevated containers |
| `--bg-elevated` | `#242424` | Modals, popovers, highest elevation |
| `--bg-hover` | `#2d2d2d` | Hover states on interactive elements |

### Text
Off-white, NOT pure white. Reduce eye strain with softer contrast.

| Token | Value | Usage |
|-------|-------|-------|
| `--text-primary` | `#f0f0f0` | Headings, primary content, labels |
| `--text-secondary` | `#a0a0a0` | Descriptions, secondary info, timestamps |
| `--text-muted` | `#666666` | Placeholders, disabled, tertiary info |

### Accent
Warm orange - the signature RalphX color. Use sparingly for maximum impact.

| Token | Value | Usage |
|-------|-------|-------|
| `--accent-primary` | `#ff6b35` | Primary actions, active states, focus rings |
| `--accent-secondary` | `#ffa94d` | Secondary highlights, hover accents |
| `--accent-hover` | `#ff8050` | Hover state for accent buttons |
| `--accent-muted` | `rgba(255, 107, 53, 0.15)` | Subtle tinted backgrounds |

### Status Colors
Semantic colors for feedback and state communication.

| Token | Value | Usage |
|-------|-------|-------|
| `--status-success` | `#10b981` | Success states, completed, passed |
| `--status-warning` | `#f59e0b` | Warnings, caution, pending review |
| `--status-error` | `#ef4444` | Errors, failures, destructive actions |
| `--status-info` | `#3b82f6` | Information (use sparingly - blue) |

### Borders
Subtle dividers and containers. Never harsh lines.

| Token | Value | Usage |
|-------|-------|-------|
| `--border-subtle` | `rgba(255, 255, 255, 0.06)` | Dividers, separators, subtle cards |
| `--border-default` | `rgba(255, 255, 255, 0.1)` | Card borders, input borders |
| `--border-focus` | `rgba(255, 107, 53, 0.5)` | Focus states, selected items |

### Usage Guidelines
1. **5% Rule**: Accent color should cover ~5% of any given screen
2. **One Primary Button**: Only one accent button per section/card
3. **Status Over Accent**: Use status colors for feedback, not accent
4. **Borders Are Optional**: Often shadow alone is enough

---

## 4. Typography

### Font Families
System fonts for performance and Mac-native feel.

| Token | Value | Usage |
|-------|-------|-------|
| `--font-display` | SF Pro Display, -apple-system, sans-serif | Headings, titles |
| `--font-body` | SF Pro Text, -apple-system, sans-serif | Body text, UI labels |
| `--font-mono` | JetBrains Mono, Menlo, monospace | Code, diffs, IDs |

### Type Scale

| Class | Size | Usage |
|-------|------|-------|
| `text-xs` | 12px (0.75rem) | Timestamps, badges, fine print |
| `text-sm` | 14px (0.875rem) | Secondary text, descriptions |
| `text-base` | 16px (1rem) | Body text, primary content |
| `text-lg` | 18px (1.125rem) | Section titles, card headers |
| `text-xl` | 20px (1.25rem) | Page section headings |
| `text-2xl` | 24px (1.5rem) | Page titles |
| `text-3xl` | 30px (1.875rem) | Hero text (rare) |

### Letter Spacing

| Token | Value | Usage |
|-------|-------|-------|
| `--tracking-tight` | -0.02em | Headings, titles (tighter = premium) |
| `--tracking-normal` | 0 | Body text |
| `--tracking-wide` | 0.05em | Uppercase labels, badges |

### Line Heights

| Token | Value | Usage |
|-------|-------|-------|
| `--leading-tight` | 1.2 | Headings, single-line labels |
| `--leading-normal` | 1.5 | Body text, descriptions |
| `--leading-relaxed` | 1.65 | Long-form content, readability |

### Font Weights

| Weight | Value | Usage |
|--------|-------|-------|
| Normal | 400 | Body text, descriptions |
| Medium | 500 | Labels, subtle emphasis |
| Semibold | 600 | Headings, titles, buttons |

---

## 5. Spacing System

### Base Unit
4px base unit, 8pt grid for component alignment.

### Spacing Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--space-1` | 4px | Micro gaps (icon-to-text, badge padding) |
| `--space-2` | 8px | Tight spacing (list items, inline elements) |
| `--space-3` | 12px | Component padding (buttons, inputs) |
| `--space-4` | 16px | Card padding, section gaps |
| `--space-5` | 20px | Medium sections |
| `--space-6` | 24px | Large gaps, column gutters |
| `--space-8` | 32px | Section separations |
| `--space-10` | 40px | Page margins (sides) |
| `--space-12` | 48px | Major section breaks |
| `--space-16` | 64px | Hero spacing, page headers |

### Usage Categories
- **Micro** (4-8px): Inside badges, between icons and text
- **Component** (12-16px): Padding inside buttons, inputs, cards
- **Section** (24-32px): Between related groups
- **Page** (48-64px): Page margins, major breaks

---

## 6. Shadow System (Layered)

Shadows provide depth and hierarchy. Use multiple layers for realistic elevation.

### Shadow Tokens

| Token | CSS | Usage |
|-------|-----|-------|
| `--shadow-xs` | `0 1px 2px rgba(0,0,0,0.2), 0 1px 3px rgba(0,0,0,0.1)` | Subtle card lift |
| `--shadow-sm` | `0 1px 2px rgba(0,0,0,0.3), 0 2px 4px rgba(0,0,0,0.2)` | Hover states, dropdowns |
| `--shadow-md` | `0 4px 6px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2)` | Modals, floating panels |
| `--shadow-lg` | `0 10px 15px rgba(0,0,0,0.3), 0 20px 40px rgba(0,0,0,0.25)` | Large overlays |
| `--shadow-glow` | `0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary)` | Focus rings |

### Shadow Usage
- **Cards at rest**: `--shadow-xs` or no shadow (border-only)
- **Cards on hover**: `--shadow-sm` + translateY(-2px)
- **Dropdowns/Popovers**: `--shadow-sm` to `--shadow-md`
- **Modals**: `--shadow-md` to `--shadow-lg`
- **Focus states**: `--shadow-glow`

---

## 7. Border & Radius System

### Radius Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-sm` | 4px | Small elements (badges, chips) |
| `--radius-md` | 8px | Buttons, inputs, cards |
| `--radius-lg` | 12px | Larger cards, modals |
| `--radius-xl` | 16px | Hero sections, large containers |
| `--radius-full` | 9999px | Pills, avatars, circular buttons |

### Gradient Border Technique
For premium cards with subtle depth:

```css
.premium-card {
  border: 1px solid transparent;
  background:
    linear-gradient(var(--bg-elevated), var(--bg-elevated)) padding-box,
    linear-gradient(180deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.02) 100%) border-box;
  border-radius: var(--radius-md);
}
```

### Border Patterns
- **Subtle dividers**: `1px solid var(--border-subtle)`
- **Card outlines**: `1px solid var(--border-default)`
- **Focus/Selected**: `2px solid var(--accent-primary)` or use `--shadow-glow`
- **No border + shadow**: Often cleaner than bordered cards

---

## 8. Component Patterns

### Buttons

**Variants:**
| Variant | Background | Text | Usage |
|---------|------------|------|-------|
| Primary | `--accent-primary` | white | Main actions (1 per section) |
| Secondary | `--bg-surface` | `--text-primary` | Secondary actions |
| Ghost | transparent | `--text-secondary` | Tertiary, navigation |
| Destructive | `--status-error` | white | Delete, dangerous actions |

**States:**
- Hover: lighten bg, subtle shadow
- Active: `scale(0.98)`, darken bg
- Focus: `--shadow-glow`
- Disabled: 50% opacity, no pointer events

**Sizing:**
- Small: 28px height, text-sm, px-3
- Default: 36px height, text-sm, px-4
- Large: 44px height, text-base, px-6

### Cards

**Variants:**
| Variant | Background | Border | Shadow |
|---------|------------|--------|--------|
| Flat | `--bg-surface` | none | none |
| Outlined | `--bg-surface` | `--border-default` | none |
| Raised | `--bg-surface` | none | `--shadow-xs` |
| Glass | rgba(26,26,26,0.85) + blur(20px) | gradient | `--shadow-md` |

**Card Padding:** 16px (--space-4) default, 12px for compact cards

### Inputs

**States:**
| State | Border | Shadow | Background |
|-------|--------|--------|------------|
| Default | `--border-default` | none | `--bg-surface` |
| Hover | `--border-default` | none | `--bg-hover` |
| Focus | `--accent-primary` | `--shadow-glow` | `--bg-surface` |
| Error | `--status-error` | error glow | `--bg-surface` |
| Disabled | `--border-subtle` | none | `--bg-base` + 50% opacity |

**Input Sizing:** 36px height default, 12px horizontal padding

### Badges

**Variants:**
| Variant | Background | Text | Usage |
|---------|------------|------|-------|
| Default | `--bg-surface` | `--text-secondary` | Neutral labels |
| Primary | `--accent-muted` | `--accent-primary` | Highlighted |
| Success | success-muted | `--status-success` | Passed, complete |
| Warning | warning-muted | `--status-warning` | Pending, caution |
| Error | error-muted | `--status-error` | Failed, blocked |

**Badge Sizing:** text-xs, px-2 py-0.5, rounded-md

### Modals

**Structure:**
```
Backdrop: rgba(0,0,0,0.6) + backdrop-blur(8px)
Content: --bg-elevated + --shadow-lg + --radius-lg
Animation: scale(0.95)→scale(1) + opacity(0)→opacity(1), 200ms
Close: Escape key + backdrop click (unless destructive)
```

**Sizes:**
- Small (max-w-sm): Simple confirmations
- Medium (max-w-md): Forms, details
- Large (max-w-lg): Complex content
- Full (max-w-2xl): Task detail, wizards

**Spacing:** 24px padding (--space-6)

---

## 9. Motion & Micro-interactions

### Timing Functions

| Name | Value | Usage |
|------|-------|-------|
| `ease-smooth` | `cubic-bezier(0.4, 0, 0.2, 1)` | Default for most transitions |
| `ease-out` | `cubic-bezier(0, 0, 0.2, 1)` | Enter animations |
| `ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | Exit animations |
| `ease-spring` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Playful interactions |

### Durations

| Token | Value | Usage |
|-------|-------|-------|
| `--transition-fast` | 150ms | Hover states, micro-feedback |
| `--transition-normal` | 200ms | Most transitions |
| `--transition-slow` | 300ms | Complex animations, modals |

### Standard Interactions

**Hover Lift (Cards, Buttons):**
```css
.hoverable {
  transition: transform 150ms ease, box-shadow 150ms ease;
}
.hoverable:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-sm);
}
```

**Active Press:**
```css
.pressable:active {
  transform: scale(0.98);
}
```

**Focus Ring:**
```css
.focusable:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
}
```

**Drag State:**
```css
.dragging {
  transform: scale(1.02) rotate(2deg);
  box-shadow: var(--shadow-md);
  opacity: 0.9;
}
```

---

## 10. Icon Usage (Lucide)

### Size Guidelines

| Context | Size | Stroke | Usage |
|---------|------|--------|-------|
| Inline text | 16px | 1.5px | Badge icons, inline labels |
| Buttons | 18-20px | 2px | Button icons |
| Navigation | 20-24px | 2px | Nav items, prominent actions |
| Empty states | 48-64px | 1.5px | Illustration-style usage |

### Color
Icons inherit text color by default. Use `currentColor`.

### Common Icons by Category

**Status:**
- CheckCircle (success), XCircle (error), AlertCircle (warning)
- Clock (pending), Loader2 (loading - animated)

**Navigation:**
- LayoutGrid (kanban), Lightbulb (ideation), Settings
- Activity (activity stream), Puzzle (extensibility)

**Actions:**
- Plus (add), Trash2 (delete), Edit (edit), MoreHorizontal (menu)
- ChevronRight, ChevronDown (expandable)

**Task/Git:**
- GitBranch, GitCommit, GitMerge, GitPullRequest
- FolderOpen, File, FileCode

### Stroke Width Consistency
Always use stroke-width 1.5-2. Never use filled icons in Lucide.

---

## 11. Page-Specific Patterns

*This section is populated by Phase 13 design requirement tasks (Tasks 3-17).*

### Kanban Board

The Kanban view is the primary interface for task management. It should feel like a premium native Mac app with physical depth and dimensionality.

**Reference Inspiration**: Linear (card interactions, keyboard-first), Raycast (Mac-native feel, spacing discipline)

#### TaskBoard

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

#### Column

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

#### TaskCard

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

#### Component Hierarchy

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

#### acceptance_criteria

```json
[
  "TaskBoard fills available viewport height",
  "Horizontal scroll works with mouse wheel + trackpad",
  "Columns have consistent 300px width",
  "Column headers show task count with Badge",
  "TaskCards display priority stripe on left border",
  "TaskCards show hover lift animation (translateY -2px)",
  "Drag handle appears on card hover",
  "Dragging card shows rotation and elevated shadow",
  "Selected card has orange border and tinted background",
  "Empty columns show empty state with icon and text",
  "Drop zones highlight with orange glow during drag-over",
  "All interactive elements have visible focus states"
]
```

#### design_quality

```json
[
  "NO purple or blue gradients anywhere",
  "Background uses subtle warm radial gradient (not flat)",
  "Shadows are layered (multiple values) for realistic depth",
  "Orange accent used sparingly - only for selection, focus, drag indicators",
  "Typography uses SF Pro with proper tracking (-0.02em for titles)",
  "All spacing follows 4px/8px grid",
  "Glass effect on column headers uses backdrop-blur",
  "Micro-interactions feel snappy (150ms transitions)",
  "Cards have physical weight - shadows create depth",
  "Empty states use Lucide icons (not custom SVGs)",
  "Badge styling consistent with shadcn Badge component",
  "Focus rings use --shadow-glow pattern"
]
```

### Ideation View

The Ideation view is a two-panel interface for brainstorming and generating task proposals. It should feel like a premium AI-powered workspace with clear visual separation between conversation and proposals.

**Reference Inspiration**: Linear (clean panels, refined typography), Raycast (glass effects, Mac-native feel), ChatGPT (message bubbles, conversation flow)

#### Overall Layout

**Structure:**
- Two-panel horizontal split with resizable divider
- Chat panel on left (~50%), Proposals panel on right (~50%)
- Viewport-filling height (`calc(100vh - header)`)
- Minimum panel width: 320px
- Drag handle for resizing (4px wide, cursor: `ew-resize`)

**Background:**
- Subtle radial gradient similar to Kanban
- Gradient: `radial-gradient(ellipse at top left, rgba(255,107,53,0.02) 0%, var(--bg-base) 40%)`
- Creates visual warmth and depth

**Header:**
- Session title with glass effect background
- Action buttons (New Session, Archive) using shadcn Button (ghost variant)
- Lucide icons: `Plus` (new), `Archive` (archive)
- Height: 52px with proper vertical centering

```css
.ideation-header {
  backdrop-filter: blur(8px);
  background: rgba(26, 26, 26, 0.85);
  border-bottom: 1px solid var(--border-subtle);
}
```

#### Conversation Panel (Chat)

**Panel Container:**
- Background: `--bg-surface`
- Right border: `1px solid var(--border-subtle)`
- Subtle inner shadow for depth: `inset 0 0 80px rgba(0,0,0,0.1)`

**Panel Header:**
- Title: "Conversation" with Lucide `MessageSquare` icon
- Height: 40px
- Glass effect with subtle border bottom
- Icon + title left-aligned

```tsx
<div className="flex items-center gap-2 px-4 py-2 backdrop-blur-sm bg-[rgba(26,26,26,0.7)] border-b border-[var(--border-subtle)]">
  <MessageSquare className="w-4 h-4 text-[var(--text-secondary)]" />
  <h2 className="text-sm font-semibold text-[var(--text-primary)]">Conversation</h2>
</div>
```

**Message Area:**
- Scrollable with auto-scroll to newest
- Scroll behavior: `smooth`
- Padding: 16px (`--space-4`)
- Message spacing: 12px between messages

**Message Bubbles:**

*User Messages:*
- Right-aligned
- Background: `--accent-primary` (#ff6b35)
- Text: white
- Border radius: 12px (bottom-right: 4px for tail effect)
- Max-width: 85%
- Padding: 12px 16px
- Shadow: `--shadow-xs` for lift

*AI/Orchestrator Messages:*
- Left-aligned
- Background: `--bg-elevated`
- Border: `1px solid var(--border-subtle)`
- Border radius: 12px (bottom-left: 4px for tail effect)
- Max-width: 85%
- Padding: 12px 16px

```css
.message-user {
  border-radius: 12px 12px 4px 12px;
  background: var(--accent-primary);
  color: white;
  box-shadow: var(--shadow-xs);
}

.message-ai {
  border-radius: 12px 12px 12px 4px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
}
```

**Timestamps:**
- Size: 11px (`text-xs` - 1px smaller)
- Color: `--text-muted`
- Position: Below bubble, aligned to bubble edge
- Format: "2:34 PM" (short)

**Typing Indicator:**
- Three animated dots
- Color: `--text-muted`
- Animation: bounce with stagger (0.1s delay between each)
- Container: same styling as AI message bubble

```css
.typing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
  animation: bounce 1.4s infinite ease-in-out both;
}
.typing-dot:nth-child(1) { animation-delay: 0s; }
.typing-dot:nth-child(2) { animation-delay: 0.1s; }
.typing-dot:nth-child(3) { animation-delay: 0.2s; }

@keyframes bounce {
  0%, 80%, 100% { transform: translateY(0); }
  40% { transform: translateY(-6px); }
}
```

**Empty State:**
- Centered vertically
- Lucide `MessageSquareText` icon (48px, `--text-muted`)
- Text: "Start the conversation" (font-medium)
- Subtext: "Describe your ideas and I'll help create task proposals" (text-sm, muted)
- Dashed border container (optional)

#### Chat Input

**Container:**
- Sticky at bottom
- Padding: 12px 16px
- Border top: `1px solid var(--border-subtle)`
- Background: `--bg-surface`

**Input Field:**
- Using shadcn Input with custom styling
- Multi-line textarea with auto-resize
- Min height: 44px, Max height: 120px
- Border radius: 8px
- Focus state: `--shadow-glow` ring

**Send Button:**
- Using shadcn Button (primary variant)
- Lucide `Send` icon (18px)
- Positioned right of input
- Size: 44px x 44px (touch-friendly)
- Disabled state: opacity 50%, no hover effects
- Loading state: Lucide `Loader2` with spin animation

**Attach Button (future):**
- Ghost button with Lucide `Paperclip` icon
- Positioned left of input
- Disabled state for now (not implemented)

```tsx
<div className="flex items-end gap-2 p-3 border-t border-[var(--border-subtle)] bg-[var(--bg-surface)]">
  <Button variant="ghost" size="icon" disabled className="shrink-0">
    <Paperclip className="w-5 h-5" />
  </Button>
  <Textarea
    placeholder="Send a message..."
    className="flex-1 resize-none min-h-[44px] max-h-[120px]"
  />
  <Button size="icon" disabled={!canSend} className="shrink-0">
    {isSending ? <Loader2 className="w-5 h-5 animate-spin" /> : <Send className="w-5 h-5" />}
  </Button>
</div>
```

**Helper Text:**
- Below input: "Enter to send, Shift+Enter for new line"
- Size: 11px, color: `--text-muted`

#### Proposals Panel

**Panel Container:**
- Background: `--bg-surface` (slightly different shade for distinction)
- No left border (divider provides separation)

**Panel Header:**
- Title: "Task Proposals" with Lucide `ListTodo` icon
- Count badge: shadcn Badge (secondary variant) showing proposal count
- Height: 40px

```tsx
<div className="flex items-center justify-between px-4 py-2 border-b border-[var(--border-subtle)]">
  <div className="flex items-center gap-2">
    <ListTodo className="w-4 h-4 text-[var(--text-secondary)]" />
    <h2 className="text-sm font-semibold">Task Proposals</h2>
  </div>
  <Badge variant="secondary">{count}</Badge>
</div>
```

**Toolbar:**
- Horizontal layout with selection count on left
- Action buttons on right: Select All, Deselect All, Sort by Priority, Clear All
- Using shadcn Button (ghost variant, icon-only with tooltips)
- Lucide icons:
  - `CheckSquare` (select all)
  - `Square` (deselect all)
  - `ArrowUpDown` (sort by priority)
  - `Trash2` (clear all)
- Separator between select buttons and other actions

**Proposal List:**
- Scrollable area
- Spacing: 8px between cards
- Padding: 16px

#### ProposalCard

**Card Structure:**
- Using shadcn Card with custom styling
- Padding: 12px (`--space-3`)
- Border radius: 8px (`--radius-md`)
- Background: `--bg-elevated`
- Border: `1px solid var(--border-subtle)`
- Shadow: `--shadow-xs` for subtle lift

**Selection Checkbox:**
- Using shadcn Checkbox (not native checkbox)
- Positioned top-left
- Size: 18px
- Custom accent color when checked: `--accent-primary`

```tsx
<Checkbox
  checked={proposal.selected}
  onCheckedChange={() => onSelect(proposal.id)}
  className="data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
/>
```

**Content Layout:**
- Title: `text-sm`, `font-medium`, `--text-primary`
- Description: `text-xs`, `--text-secondary`, 2-line clamp
- Badge row: flex wrap, gap-1.5

**Priority Badge:**
- Using shadcn Badge with semantic variants
- Colors by priority:
  - Critical: destructive variant (red background)
  - High: warning variant (orange background - custom)
  - Medium: default variant with accent tint
  - Low: secondary variant (gray)

```tsx
const priorityVariant = {
  critical: "destructive",
  high: "warning", // custom variant with orange
  medium: "default",
  low: "secondary"
};
```

**Category Badge:**
- shadcn Badge (secondary variant)
- Smaller padding for compactness

**Dependency Indicators:**
- Small Lucide icons: `ArrowUp` (depends on), `ArrowDown` (blocks)
- Color: `--text-muted`
- Text: "Depends on 2" / "Blocks 3"

**Hover State:**
- Card lifts: `transform: translateY(-2px)`
- Shadow increases: `--shadow-sm`
- Border lightens slightly
- Transition: 150ms ease

**Selected State:**
- Border: `2px solid var(--accent-primary)`
- Background: `var(--accent-muted)` (subtle orange tint)
- Box shadow: `0 0 0 3px rgba(255,107,53,0.15)` for glow

```css
.proposal-card.selected {
  border: 2px solid var(--accent-primary);
  background: var(--accent-muted);
  box-shadow: 0 0 0 3px rgba(255,107,53,0.15);
}
```

**Action Buttons (Edit/Remove):**
- Visible on hover only (opacity transition)
- Using shadcn Button (ghost variant, size="icon-sm")
- Lucide icons: `Pencil` (edit), `X` (remove)
- Position: top-right corner

**Drag Handle:**
- Lucide `GripVertical` icon
- Visible on hover
- Position: left side of card
- Color: `--text-muted`, hover: `--text-secondary`
- Cursor: `grab` (dragging: `grabbing`)

**Drag State:**
- Same as TaskCard: `scale(1.02) rotate(2deg)`
- Shadow: `--shadow-md`
- Opacity: 0.9
- Z-index: 50

#### Empty State (Proposals)

- Centered vertically
- Lucide `Lightbulb` icon (48px, `--text-muted`)
- Main text: "No proposals yet" (font-medium)
- Subtext: "Chat with the orchestrator to generate task proposals" (text-sm)
- Dashed border container
- Padding: 48px

```tsx
<div className="flex flex-col items-center justify-center h-full p-12 text-center">
  <div className="p-4 rounded-lg border-2 border-dashed border-[var(--border-subtle)]">
    <Lightbulb className="w-12 h-12 mx-auto mb-4 text-[var(--text-muted)]" />
    <p className="font-medium text-[var(--text-secondary)]">No proposals yet</p>
    <p className="text-sm text-[var(--text-muted)] mt-1">
      Chat with the orchestrator to generate task proposals
    </p>
  </div>
</div>
```

#### Apply Section

**Container:**
- Fixed at bottom of proposals panel
- Border top: `1px solid var(--border-subtle)`
- Background: `--bg-surface`
- Padding: 12px 16px
- Height: 56px

**Layout:**
- Selection count on left: "3 selected"
- Apply dropdown on right

**Apply Button:**
- Using shadcn DropdownMenu with Button trigger
- Button: primary variant when enabled, secondary when disabled
- Chevron down icon (Lucide `ChevronDown`)
- Text: "Apply to..."

**Dropdown Menu:**
- shadcn DropdownMenuContent
- Options: Draft, Backlog, Todo
- Each with description text
- Lucide icons for each column type

```tsx
<DropdownMenu>
  <DropdownMenuTrigger asChild>
    <Button disabled={!canApply}>
      Apply to
      <ChevronDown className="w-4 h-4 ml-1" />
    </Button>
  </DropdownMenuTrigger>
  <DropdownMenuContent align="end">
    <DropdownMenuItem onClick={() => handleApply("draft")}>
      <FileEdit className="w-4 h-4 mr-2" />
      Draft
    </DropdownMenuItem>
    <DropdownMenuItem onClick={() => handleApply("backlog")}>
      <Inbox className="w-4 h-4 mr-2" />
      Backlog
    </DropdownMenuItem>
    <DropdownMenuItem onClick={() => handleApply("todo")}>
      <ListTodo className="w-4 h-4 mr-2" />
      Todo
    </DropdownMenuItem>
  </DropdownMenuContent>
</DropdownMenu>
```

#### Resize Handle

**Structure:**
- 4px wide invisible hit area
- Visible line: 1px, centered
- Color: `--border-subtle`, hover: `--accent-primary`
- Cursor: `ew-resize`
- Height: 100% of panel

**Visual States:**
- Default: subtle line
- Hover: accent color, slight glow
- Dragging: accent color, stronger glow

```css
.resize-handle {
  width: 4px;
  cursor: ew-resize;
  position: relative;
  background: transparent;
}

.resize-handle::after {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  width: 1px;
  background: var(--border-subtle);
  transition: all 150ms ease;
}

.resize-handle:hover::after {
  background: var(--accent-primary);
  box-shadow: 0 0 8px rgba(255,107,53,0.3);
}
```

#### Component Hierarchy

```
IdeationView
├── IdeationHeader (glass effect)
│   ├── SessionTitle
│   └── ActionButtons (New, Archive)
├── ResizeHandle
├── ConversationPanel
│   ├── PanelHeader (icon + title)
│   ├── MessageList (scrollable)
│   │   ├── ChatMessage (user - right aligned)
│   │   ├── ChatMessage (ai - left aligned)
│   │   └── TypingIndicator (if loading)
│   └── ChatInput
│       ├── AttachButton (disabled)
│       ├── Textarea (auto-resize)
│       └── SendButton
└── ProposalsPanel
    ├── PanelHeader (icon + title + count)
    ├── Toolbar (select/sort/clear actions)
    ├── ProposalList (scrollable, sortable)
    │   └── ProposalCard (×N)
    │       ├── Checkbox
    │       ├── Title + Description
    │       ├── Badges (Priority, Category)
    │       ├── DependencyInfo
    │       └── ActionButtons (Edit, Remove)
    ├── EmptyState (if no proposals)
    └── ApplySection
        ├── SelectionCount
        └── ApplyDropdown
```

#### acceptance_criteria

```json
[
  "Two-panel layout fills available viewport height",
  "Panels are resizable with drag handle",
  "Minimum panel width is 320px",
  "User messages align right with warm orange background",
  "AI messages align left with elevated background",
  "Message bubbles have asymmetric border radius (tail effect)",
  "Timestamps appear below each message bubble",
  "Typing indicator shows animated dots during AI response",
  "Auto-scroll to newest message works smoothly",
  "Chat input supports multi-line with auto-resize",
  "Enter sends message, Shift+Enter adds newline",
  "Send button shows loading spinner while sending",
  "Proposal cards use shadcn Card component",
  "Selection checkbox uses shadcn Checkbox",
  "Selected proposals have orange border and tinted background",
  "Drag-and-drop reordering works with visual feedback",
  "Priority badges use correct semantic colors",
  "Action buttons appear on card hover",
  "Apply dropdown shows column options",
  "Empty states show appropriate Lucide icons and text"
]
```

#### design_quality

```json
[
  "NO purple or blue gradients anywhere",
  "Background uses subtle warm radial gradient (not flat)",
  "Message bubbles have asymmetric corners for tail effect",
  "Shadows are layered for realistic depth",
  "Orange accent used sparingly - only for user messages, selection, and primary buttons",
  "Typography uses SF Pro with proper tracking",
  "All spacing follows 4px/8px grid",
  "Glass effect on headers uses backdrop-blur",
  "Micro-interactions feel snappy (150ms transitions)",
  "Proposal cards lift on hover (translateY -2px)",
  "Resize handle glows orange on hover",
  "Empty states use Lucide icons (Lightbulb, MessageSquareText)",
  "Focus rings use --shadow-glow pattern",
  "Typing indicator animation is smooth and playful",
  "Panel headers have consistent height and alignment"
]
```

### Settings View
*Design requirements documented in Task 5*

### Activity Stream View
*Design requirements documented in Task 6*

### Extensibility View
*Design requirements documented in Task 7*

### Task Detail View
*Design requirements documented in Task 8*

### Reviews Panel
*Design requirements documented in Task 9*

### Chat Panel (Global)
*Design requirements documented in Task 10*

### QA Components
*Design requirements documented in Task 11*

### Project Sidebar
*Design requirements documented in Task 12*

### Project Dialogs
*Design requirements documented in Task 13*

### Diff Viewer
*Design requirements documented in Task 14*

### Execution Control Bar
*Design requirements documented in Task 15*

### Header and Navigation
*Design requirements documented in Task 16*

### Modal Standards
*Design requirements documented in Task 17*

---

## 12. shadcn/ui Integration

RalphX uses shadcn/ui as the component foundation. All shadcn components are customized to use RalphX design tokens.

### Installed Components
- Button, Card, Dialog, Dropdown Menu
- Input, Label, Tabs, Tooltip, Popover
- Select, Checkbox, Switch, Badge
- Scroll Area, Separator, Skeleton

### CSS Variable Mapping
shadcn variables are mapped to RalphX tokens in `globals.css`:
- `--primary` → `--accent-primary`
- `--background` → `--bg-base`
- `--card` → `--bg-elevated`
- `--foreground` → `--text-primary`
- `--ring` → `--accent-primary`

### Component Location
All shadcn components live in `src/components/ui/`.

### Usage Pattern
```tsx
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Dialog, DialogContent, DialogTrigger } from '@/components/ui/dialog';
```

---

## 13. Accessibility

### Color Contrast
- Text on backgrounds: minimum 4.5:1 ratio
- Large text (18px+): minimum 3:1 ratio
- All colors tested against WCAG AA

### Focus States
- All interactive elements have visible focus
- Focus ring: 2px offset, accent color
- Never remove outline without replacement

### Keyboard Navigation
- Tab order follows visual order
- Escape closes modals/popovers
- Arrow keys for lists and menus
- Enter/Space activates buttons

### Screen Readers
- Semantic HTML elements
- ARIA labels where needed
- Icon-only buttons have aria-label

---

## References

- **Design Overhaul Plan**: `specs/DESIGN_OVERHAUL_PLAN.md`
- **Global CSS**: `src/styles/globals.css`
- **shadcn Components**: `src/components/ui/`
- **Lucide Icons**: https://lucide.dev/icons/
