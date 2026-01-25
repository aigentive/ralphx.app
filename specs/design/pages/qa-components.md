# QA Components

The QA Components provide visual feedback for test execution, results display, and screenshot verification. These components work together to communicate test status at a glance while providing deep-dive capabilities for debugging failures.

**Design Inspiration:**
- Cypress Test Runner (status badges, step-by-step results, screenshot gallery)
- Percy Visual Review (comparison mode, zoom controls, diff overlays)
- GitHub Actions (test status indicators, collapsible details, timeline)
- Storybook Testing (compact status, expandable panels, grid layouts)

**Aesthetic Direction:** Clinical precision with warm accessibility. QA components must communicate status instantly - green means good, red means attention needed. The design should feel professional and trustworthy, with clear visual hierarchy that guides the eye to what matters most. Status indicators are prominent but not overwhelming; details are available on demand without cluttering the primary view.

---

## TaskQABadge

A compact, inline badge that appears on TaskCards to indicate QA status. Must be scannable at a glance without dominating the card layout.

### Badge Design

| Property | Value | Notes |
|----------|-------|-------|
| Display | inline-flex | Aligns icon and text |
| Padding | 4px 8px | Compact but touchable |
| Border radius | `--radius-sm` (4px) | Consistent with other badges |
| Font size | `text-xs` (12px) | Small but readable |
| Font weight | `font-medium` (500) | Slight emphasis |
| Height | ~22px | Doesn't overwhelm card content |
| Min-width | fit-content | Natural width per status |
| Gap | 4px | Icon to text spacing |

### Status States

Using **shadcn Badge** with custom variant styling. Each status has distinct visual treatment.

| Status | Background | Text Color | Icon | Label |
|--------|------------|------------|------|-------|
| pending | `--bg-hover` | `--text-muted` | `Clock` | "QA Pending" |
| preparing | `rgba(245, 158, 11, 0.15)` | `--status-warning` | `Loader2` (spin) | "Preparing" |
| ready | `rgba(59, 130, 246, 0.15)` | `--status-info` | `CheckCircle` | "QA Ready" |
| testing | `--accent-muted` | `--accent-primary` | `Loader2` (spin) | "Testing" |
| passed | `rgba(16, 185, 129, 0.15)` | `--status-success` | `CheckCircle` | "Passed" |
| failed | `rgba(239, 68, 68, 0.15)` | `--status-error` | `XCircle` | "Failed" |
| skipped | `--bg-hover` | `--text-muted` | `MinusCircle` | "Skipped" |

### Badge Styling

```tsx
const statusStyles = {
  pending: "bg-[var(--bg-hover)] text-[var(--text-muted)]",
  preparing: "bg-amber-500/15 text-[var(--status-warning)]",
  ready: "bg-blue-500/15 text-[var(--status-info)]",
  testing: "bg-[var(--accent-muted)] text-[var(--accent-primary)]",
  passed: "bg-emerald-500/15 text-[var(--status-success)]",
  failed: "bg-red-500/15 text-[var(--status-error)]",
  skipped: "bg-[var(--bg-hover)] text-[var(--text-muted)]",
};

<Badge className={cn("inline-flex items-center gap-1", statusStyles[status])}>
  <StatusIcon className="w-3 h-3" />
  {statusLabel}
</Badge>
```

### Icon Specifications

| Status | Lucide Icon | Size | Animation |
|--------|-------------|------|-----------|
| pending | `Clock` | 12px | None |
| preparing | `Loader2` | 12px | `animate-spin` |
| ready | `CheckCircle` | 12px | None |
| testing | `Loader2` | 12px | `animate-spin` |
| passed | `CheckCircle` | 12px | None |
| failed | `XCircle` | 12px | None |
| skipped | `MinusCircle` | 12px | None |

### Compact Variant

For tighter spaces (e.g., smaller cards), use icon-only mode with tooltip.

| Property | Value |
|----------|-------|
| Width | 22px |
| Height | 22px |
| Padding | 4px |
| Icon size | 14px |
| Tooltip | Full status label |

```tsx
<Tooltip content={statusLabel}>
  <Badge variant="outline" className={cn("p-1", statusStyles[status])}>
    <StatusIcon className="w-3.5 h-3.5" />
  </Badge>
</Tooltip>
```

---

## TaskDetailQAPanel

A tabbed panel embedded within the TaskDetailView modal, showing acceptance criteria, test results, and screenshots. Provides the deep-dive experience for understanding QA outcomes.

### Panel Container

| Property | Value | Notes |
|----------|-------|-------|
| Background | transparent | Inherits from modal |
| Padding | 0 | Tabs handle their own padding |
| Border | none | Modal provides definition |
| Min-height | 300px | Ensures visibility |
| Flex | 1 | Takes available space |

### Tab Navigation

Using **shadcn Tabs** with underline indicator style.

```
┌─────────────────────────────────────────────────────────────────────┐
│  [Acceptance Criteria (5)]   [Test Results (8)]   [Screenshots (3)] │
│  ═══════════════════════                                            │
└─────────────────────────────────────────────────────────────────────┘
```

#### Tab List Styling

| Property | Value |
|----------|-------|
| Border bottom | 1px `--border-subtle` |
| Gap | 4px between tabs |
| Margin bottom | 16px |

#### Tab Trigger Styling

| State | Background | Text | Border |
|-------|------------|------|--------|
| Default | transparent | `--text-secondary` | none |
| Hover | transparent | `--text-primary` | none |
| Active | transparent | `--text-primary` | 2px bottom `--accent-primary` |
| Focus | transparent | `--text-primary` | `--shadow-glow` |

| Property | Value |
|----------|-------|
| Font size | `text-sm` (14px) |
| Font weight | `font-medium` (500) |
| Padding | 8px 12px |
| Transition | 150ms ease |

#### Tab Count Badges

| Property | Value |
|----------|-------|
| Position | After tab label |
| Format | "(N)" inline with label |
| Font size | `text-xs` |
| Color | Inherits tab text color |
| Opacity | 0.8 on inactive tabs |

---

### Acceptance Criteria Tab

Displays acceptance criteria as a checklist with pass/fail indicators derived from test results.

#### Criteria Card Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  [✓]   AC-001   [functional]   [testable]                          │
│        User can view task details in modal                         │
└─────────────────────────────────────────────────────────────────────┘
```

| Property | Value |
|----------|-------|
| Background | `--bg-elevated` |
| Padding | 12px |
| Border radius | `--radius-md` (8px) |
| Margin bottom | 8px |
| Gap | 12px (icon to content) |

#### Status Indicator

Left-side icon showing criterion status.

| Status | Icon | Color | Size |
|--------|------|-------|------|
| passed | `CheckCircle` | `--status-success` | 16px |
| failed | `XCircle` | `--status-error` | 16px |
| pending | `Circle` | `--text-muted` | 16px |
| running | `Circle` | `--status-info` | 16px + `animate-pulse` |

#### Criterion Metadata

| Element | Styling |
|---------|---------|
| ID badge | `text-xs`, `font-medium`, `--bg-hover` bg, `--text-secondary` text, `--radius-sm`, px-1.5 py-0.5 |
| Type badge | `text-xs`, `--bg-hover` bg, `--text-muted` text, `--radius-sm`, px-1.5 py-0.5 |
| Testable badge | `text-xs`, `--status-info` bg, `--bg-base` text, `--radius-sm`, px-1.5 py-0.5 |
| Gap between badges | 6px |

#### Criterion Description

| Property | Value |
|----------|-------|
| Font size | `text-sm` (14px) |
| Color | `--text-primary` |
| Line height | `--leading-normal` (1.5) |
| Margin top | 6px |

#### Empty State

```tsx
<div className="py-8 text-center text-[var(--text-muted)] text-sm">
  No acceptance criteria defined
</div>
```

---

### Test Results Tab

Shows individual test step results with expandable failure details.

#### Overall Status Banner

| Property | Value |
|----------|-------|
| Background | `--bg-elevated` |
| Padding | 12px 16px |
| Border radius | `--radius-md` |
| Flex | `justify-between items-center` |
| Margin bottom | 16px |

**Left side:**
- Label "Overall:" in `text-sm`, `font-medium`, `--text-secondary`
- Status badge using defined status colors

**Right side:**
- Progress summary: "5/8" format
- Font: `text-sm`, `--text-muted`

```tsx
<div className="flex items-center justify-between p-4 rounded-lg bg-[var(--bg-elevated)]">
  <div className="flex items-center gap-2">
    <span className="text-sm font-medium text-[var(--text-secondary)]">Overall:</span>
    <Badge variant={statusVariant}>{overall_status}</Badge>
  </div>
  <span className="text-sm text-[var(--text-muted)]">{passed}/{total}</span>
</div>
```

#### Step Result Card

```
┌─────────────────────────────────────────────────────────────────────┐
│  [✓]   step-001   [📷 Has screenshot]                               │
│        Verify login button is visible and clickable                 │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Error: Element not found: #login-button                    │    │
│  │  ─────────────────────────────────────────────────────────  │    │
│  │  Expected: Button visible        Actual: Element not found  │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

| Property | Value |
|----------|-------|
| Background | `--bg-elevated` |
| Padding | 12px |
| Border radius | `--radius-md` |
| Margin bottom | 8px |

#### Failure Details Box

Appears only for failed steps with error/expected/actual information.

| Property | Value |
|----------|-------|
| Background | `--bg-base` |
| Border | 1px `--status-error` at 30% opacity |
| Border radius | `--radius-md` |
| Padding | 8px |
| Margin | 12px top, 28px left (indented from icon) |

**Error message:**
- Font: `text-xs`
- Color: `--status-error`
- Margin bottom: 8px

**Expected/Actual grid:**
- Display: `grid grid-cols-2`
- Gap: 8px
- Labels: `text-xs`, `--text-muted`
- Values: `text-xs`, `--text-primary`, margin-top 2px

#### Screenshot Indicator

| Property | Value |
|----------|-------|
| Icon | Lucide `Image` (12px) |
| Label | "Has screenshot" |
| Font | `text-xs` |
| Color | `--text-muted` |
| Gap | 4px |
| Display | Only when screenshot present |

---

### Screenshots Tab

Displays the ScreenshotGallery component with appropriate configuration.

| Property | Value |
|----------|-------|
| Columns | 3 (default) |
| Empty message | "No screenshots captured" |
| Container | Full tab panel width |

---

## QASettingsPanel

A settings section for configuring QA behavior. Part of the main Settings view but can also appear in context-specific locations.

### Section Header

| Property | Value |
|----------|-------|
| Title | "QA Settings" |
| Font | `text-lg`, `font-medium` (500) |
| Color | `--text-primary` |
| Icon | Lucide `FlaskConical` or `TestTube2` (20px) optional |
| Margin bottom | 16px |

### Settings Card Container

Using **shadcn Card** for grouping related settings.

| Property | Value |
|----------|-------|
| Background | `--bg-surface` |
| Border | 1px `--border-subtle` |
| Border radius | `--radius-lg` (12px) |
| Padding | 0 (rows handle padding) |

### Setting Row

```
┌─────────────────────────────────────────────────────────────────────┐
│  Enable QA System                                            [===] │
│  Master toggle for the QA system. When disabled, no tests run.     │
├─────────────────────────────────────────────────────────────────────┤
│     Auto-QA for UI Tasks                                     [===] │
│     Automatically enable QA for tasks in UI categories.            │
└─────────────────────────────────────────────────────────────────────┘
```

| Property | Value |
|----------|-------|
| Padding | 16px horizontal, 12px vertical |
| Border bottom | 1px `--border-subtle` (except last) |
| Flex | `justify-between items-start` |
| Gap | 16px |

**Indented rows (sub-settings):**
- Margin left: 24px
- Indicates dependency on parent toggle

### Setting Label

| Property | Value |
|----------|-------|
| Font size | `text-sm` (14px) |
| Font weight | `font-medium` (500) |
| Color | `--text-primary` (normal), `--text-muted` (disabled) |
| Line height | `--leading-tight` (1.2) |

### Setting Description

| Property | Value |
|----------|-------|
| Font size | `text-xs` (12px) |
| Color | `--text-muted` |
| Line height | `--leading-normal` (1.5) |
| Margin top | 2px |
| Max width | 320px (prevents excessive line length) |

### Toggle Switch

Using **shadcn Switch** with custom styling.

| Property | Value |
|----------|-------|
| Width | 44px |
| Height | 24px |
| Track off | `--bg-hover` |
| Track on | `--accent-primary` |
| Thumb | white, 20px |
| Border radius | `--radius-full` |
| Transition | 200ms ease |

**States:**
| State | Track | Cursor |
|-------|-------|--------|
| Off | `--bg-hover` | pointer |
| On | `--accent-primary` | pointer |
| Disabled | 50% opacity | not-allowed |
| Focus | `--shadow-glow` ring | pointer |

```tsx
<Switch
  checked={enabled}
  onCheckedChange={setEnabled}
  disabled={parentDisabled}
  className="data-[state=checked]:bg-[var(--accent-primary)]"
/>
```

### URL Input Field

For browser testing URL configuration.

| Property | Value |
|----------|-------|
| Label | "Browser Testing URL" |
| Placeholder | "http://localhost:1420" |
| Type | url |
| Margin left | 24px (indented with parent) |

Using **shadcn Input** with custom styling:

| State | Border | Background | Shadow |
|-------|--------|------------|--------|
| Default | `--border-subtle` | `--bg-elevated` | none |
| Hover | `--border-default` | `--bg-elevated` | none |
| Focus | `--accent-primary` | `--bg-elevated` | `--shadow-glow` |
| Disabled | `--border-subtle` | `--bg-base` | none + 50% opacity |

| Property | Value |
|----------|-------|
| Height | 36px |
| Padding | 8px 12px |
| Font size | `text-sm` |
| Border radius | `--radius-md` |
| Width | 100% |
| Max width | 400px |
| Margin top | 8px |

### Error Banner

Displays when settings update fails.

| Property | Value |
|----------|-------|
| Background | `rgba(239, 68, 68, 0.1)` |
| Border | 1px `--status-error` at 30% opacity |
| Border radius | `--radius-md` |
| Padding | 12px |
| Font | `text-sm` |
| Color | `--status-error` |
| Margin bottom | 16px |

```tsx
{error && (
  <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-sm text-[var(--status-error)]">
    {error}
  </div>
)}
```

---

## ScreenshotGallery

A professional screenshot gallery with thumbnail grid, lightbox viewer, and Expected vs Actual comparison mode.

### Thumbnail Grid

| Property | Value |
|----------|-------|
| Display | `grid` |
| Columns | 2, 3, or 4 (configurable) |
| Gap | 12px |
| Default columns | 3 |

#### Grid Column Classes

```tsx
const gridCols = {
  2: "grid-cols-2",
  3: "grid-cols-3",
  4: "grid-cols-4",
};
```

### Thumbnail Card

| Property | Value |
|----------|-------|
| Aspect ratio | 16:9 (video) |
| Background | `--bg-elevated` |
| Border radius | `--radius-lg` (12px) |
| Overflow | hidden |
| Cursor | pointer |

#### Thumbnail States

| State | Effect |
|-------|--------|
| Default | No ring, no shadow |
| Hover | Ring 2px `--accent-primary`, offset 2px, scale(1.05) image |
| Focus | Ring 2px `--accent-primary`, offset 2px |
| Active | scale(0.98) |

```css
.thumbnail {
  transition: all 200ms ease;
}

.thumbnail:hover {
  box-shadow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
}

.thumbnail:hover img {
  transform: scale(1.05);
}
```

#### Thumbnail Image

| Property | Value |
|----------|-------|
| Object fit | cover |
| Width/Height | 100% |
| Transition | transform 300ms |

#### Thumbnail Overlay

Gradient overlay on hover showing label and timestamp.

| Property | Value |
|----------|-------|
| Background | `linear-gradient(to top, rgba(0,0,0,0.7), transparent)` |
| Position | absolute, bottom 0 |
| Padding | 8px |
| Opacity | 0 → 1 on hover |
| Transition | 200ms |

#### Status Indicators

Pass/fail icons overlaid on thumbnails.

**Passed indicator:**
| Property | Value |
|----------|-------|
| Position | absolute, top-right (8px offset) |
| Background | `--status-success` |
| Size | 24px circle |
| Icon | Lucide `Check` (14px, white) |
| Padding | 5px |
| Border radius | `--radius-full` |

**Failed indicator:**
| Property | Value |
|----------|-------|
| Position | absolute, top-right (8px offset) |
| Background | `--status-error` |
| Size | 24px circle |
| Icon | Lucide `X` (14px, white) |

**Comparison available indicator:**
| Property | Value |
|----------|-------|
| Position | absolute, top-left (8px offset) |
| Background | `--accent-primary` |
| Size | 24px |
| Icon | Lucide `GitCompare` or custom compare icon (14px, white) |
| Tooltip | "Comparison available" |

### Empty State

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                         ┌───────────┐                               │
│                         │   (📷)    │                               │
│                         └───────────┘                               │
│                     No screenshots captured                         │
│           Screenshots will appear here when captured                │
│                        during QA testing                            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Container | `flex flex-col items-center justify-center`, padding 48px |
| Icon container | 64px circle, `--bg-elevated` bg |
| Icon | Lucide `Image` (32px), `--text-muted` |
| Title | `text-sm`, `font-medium`, `--text-secondary` |
| Subtitle | `text-xs`, `--text-muted`, 60% opacity, margin-top 4px |

---

### Lightbox

Full-screen overlay for detailed screenshot viewing.

#### Lightbox Backdrop

| Property | Value |
|----------|-------|
| Position | fixed inset-0 |
| Background | `rgba(0, 0, 0, 0.95)` |
| Z-index | 50 |
| Display | flex flex-col |

#### Lightbox Header

```
┌─────────────────────────────────────────────────────────────────────┐
│  step-001.png   [Failed]        1 / 5        [Compare] [-] [+] [✕] │
└─────────────────────────────────────────────────────────────────────┘
```

| Property | Value |
|----------|-------|
| Background | `linear-gradient(to bottom, rgba(0,0,0,0.5), transparent)` |
| Padding | 16px 24px |
| Flex | `justify-between items-center` |

**Filename and status:**
- Filename: `text-base`, `font-medium`, white
- Status badge: same as TaskQABadge but with lighter backgrounds for dark mode
- Gap: 12px

**Counter:**
- Format: "1 / 5"
- Font: `text-sm`
- Color: `white/60`

**Controls:**
- Compare toggle: button with icon + "Compare" label
- Zoom out: Lucide `ZoomOut`
- Zoom in: Lucide `ZoomIn`
- Zoom display: percentage, 48px width
- Close: Lucide `X`
- Button size: 36px
- Button background: `white/10` hover: `white/20`
- Button color: white
- Gap: 8px

#### Main Image View

| Property | Value |
|----------|-------|
| Container | flex-1, flex items-center justify-center |
| Max dimensions | 90vw × 75vh |
| Object fit | contain |
| Border radius | `--radius-lg` |
| Shadow | `--shadow-lg` (0 20px 40px rgba(0,0,0,0.5)) |
| Cursor | grab (if zoomed), default otherwise |

**Zoom and pan:**
- Zoom levels: 0.5× to 4× (25% increments)
- Pan: click and drag when zoomed > 1×
- Reset: "0" key or double-click

#### Navigation Arrows

| Property | Value |
|----------|-------|
| Position | absolute, left/right 16px, vertically centered |
| Size | 48px circle |
| Background | `rgba(0, 0, 0, 0.5)` |
| Hover | `rgba(0, 0, 0, 0.7)` |
| Icon | Lucide `ChevronLeft` / `ChevronRight` (24px) |
| Color | white |
| Disabled | 30% opacity, no pointer events |
| Transition | 150ms |

#### Comparison View

When comparison mode is active, shows Expected and Actual side-by-side.

```
┌────────────────────────────┐   │   ┌────────────────────────────┐
│                            │   │   │                            │
│         EXPECTED           │   │   │          ACTUAL            │
│                            │   │   │                            │
│      [screenshot]          │   │   │      [screenshot]          │
│                            │   │   │                            │
│   Expected: "Button blue"  │   │   │   Actual: "Button red"     │
│                            │   │   │                            │
└────────────────────────────┘   │   └────────────────────────────┘
```

| Property | Value |
|----------|-------|
| Container | flex gap-6, max-width 95vw |
| Panel width | flex-1 each |
| Divider | 1px `white/20`, self-stretch |

**Panel labels:**
| Property | Value |
|----------|-------|
| Position | above each image |
| Font | `text-xs`, `font-semibold`, uppercase, `tracking-wider` |
| "Expected" | `--status-success` bg at 20%, `--status-success` text |
| "Actual" | `--status-error` bg at 20%, `--status-error` text |
| Padding | 8px 12px |
| Border radius | `--radius-sm` |
| Margin bottom | 12px |

**Expected/Actual text:**
| Property | Value |
|----------|-------|
| Position | below panel label |
| Font | `text-sm` |
| Color | `white/60` |
| Truncate | max-width 300px |

**Image border:**
| Property | Value |
|----------|-------|
| Expected | 1px `--status-success` at 30% |
| Actual | 1px `--status-error` at 30% |
| Border radius | `--radius-lg` |

**No expected placeholder:**
```tsx
<div className="flex-1 flex items-center justify-center rounded-lg border border-dashed border-white/20 bg-white/5 min-h-[200px]">
  <div className="text-center text-white/40">
    <Image className="w-12 h-12 mx-auto mb-2 opacity-50" />
    <p className="text-sm">No expected screenshot</p>
  </div>
</div>
```

#### Failure Details Footer

Displayed when viewing a failed screenshot.

| Property | Value |
|----------|-------|
| Position | bottom of lightbox |
| Background | `linear-gradient(to top, rgba(0,0,0,0.5), transparent)` |
| Padding | 16px 24px |

**Details container:**
| Property | Value |
|----------|-------|
| Background | `--status-error` at 10% |
| Border | 1px `--status-error` at 30% |
| Border radius | `--radius-lg` |
| Padding | 16px |
| Max width | 768px |
| Margin | auto (centered) |

#### Thumbnail Strip

Bottom navigation when multiple screenshots.

| Property | Value |
|----------|-------|
| Container | flex justify-center gap-2 |
| Background | `rgba(0, 0, 0, 0.5)` |
| Padding | 16px 24px |

**Mini thumbnails:**
| Property | Value |
|----------|-------|
| Size | 64px × 40px |
| Border radius | `--radius-sm` |
| Object fit | cover |
| Current | Ring 2px `--accent-primary`, offset 2px black |
| Other | 50% opacity, hover: 80% |
| Transition | 150ms |

### Keyboard Navigation

| Key | Action |
|-----|--------|
| Escape | Close lightbox |
| ArrowLeft | Previous image |
| ArrowRight | Next image |
| + / = | Zoom in |
| - | Zoom out |
| 0 | Reset zoom |
| c | Toggle comparison mode |

---

## Lucide Icons Used

| Icon | Usage | Size |
|------|-------|------|
| `Clock` | Pending status | 12-16px |
| `Loader2` | Preparing/testing status (animated) | 12-16px |
| `CheckCircle` | Ready/passed status | 12-16px |
| `XCircle` | Failed status | 12-16px |
| `MinusCircle` | Skipped status | 12-16px |
| `Circle` | Pending criteria indicator | 16px |
| `Image` | Screenshot indicator, empty state | 12-32px |
| `FlaskConical` | QA settings header (optional) | 20px |
| `TestTube2` | Alternative QA settings icon | 20px |
| `ZoomIn` | Lightbox zoom control | 20px |
| `ZoomOut` | Lightbox zoom control | 20px |
| `ChevronLeft` | Lightbox navigation | 24px |
| `ChevronRight` | Lightbox navigation | 24px |
| `X` | Close lightbox | 24px |
| `GitCompare` | Comparison mode toggle | 20px |
| `Check` | Thumbnail passed indicator | 14px |
| `AlertTriangle` | Failure indicator | 16px |

---

## Component Hierarchy

```
TaskQABadge (standalone inline component)
├── Badge (shadcn)
│   ├── StatusIcon (Lucide icon per status)
│   └── StatusLabel (text)
└── Tooltip (for compact variant)

TaskDetailQAPanel
├── Tabs (shadcn)
│   ├── TabsList
│   │   ├── TabsTrigger "Acceptance Criteria" + count
│   │   ├── TabsTrigger "Test Results" + count
│   │   └── TabsTrigger "Screenshots" + count
│   │
│   ├── TabsContent [criteria]
│   │   ├── AcceptanceCriteriaTab
│   │   │   ├── EmptyState (conditional)
│   │   │   └── CriterionCard[] (mapped)
│   │   │       ├── StatusIcon (left)
│   │   │       ├── MetadataRow
│   │   │       │   ├── IDBadge
│   │   │       │   ├── TypeBadge
│   │   │       │   └── TestableBadge (conditional)
│   │   │       └── Description
│   │   │
│   ├── TabsContent [results]
│   │   ├── TestResultsTab
│   │   │   ├── OverallStatusBanner
│   │   │   │   ├── StatusLabel + Badge
│   │   │   │   └── ProgressSummary
│   │   │   ├── EmptyState (conditional)
│   │   │   └── StepResultCard[] (mapped)
│   │   │       ├── StatusIcon
│   │   │       ├── MetadataRow (ID + screenshot indicator)
│   │   │       ├── Description
│   │   │       └── FailureDetailsBox (conditional)
│   │   │           ├── ErrorMessage
│   │   │           └── Expected/Actual grid
│   │   │
│   └── TabsContent [screenshots]
│       └── ScreenshotGallery
│
└── ActionButtons (conditional, for failed QA)
    ├── RetryButton
    └── SkipButton

QASettingsPanel
├── SectionHeader
│   ├── Icon (optional)
│   └── Title
├── ErrorBanner (conditional)
└── Card (shadcn)
    └── SettingRowList
        ├── SettingRow (master toggle)
        │   ├── LabelGroup
        │   │   ├── Label
        │   │   └── Description
        │   └── Switch (shadcn)
        ├── SettingRow (indented sub-toggle) × N
        └── URLInputRow (indented)
            ├── Label
            ├── Description
            └── Input (shadcn)

ScreenshotGallery
├── EmptyState (conditional)
│   ├── IconContainer
│   ├── Title
│   └── Subtitle
├── ThumbnailGrid
│   └── Thumbnail[] (mapped)
│       ├── Image (or placeholder)
│       ├── HoverOverlay
│       │   ├── Label
│       │   └── Timestamp
│       ├── PassedIndicator (conditional)
│       ├── FailedIndicator (conditional)
│       └── ComparisonIndicator (conditional)
│
└── Lightbox (conditional, when image selected)
    ├── Header
    │   ├── FilenameAndStatus
    │   ├── Counter
    │   └── Controls
    │       ├── CompareToggle
    │       ├── ZoomOut
    │       ├── ZoomLevel
    │       ├── ZoomIn
    │       └── Close
    ├── MainContent
    │   ├── SingleImageView (default)
    │   │   ├── Image
    │   │   ├── PrevArrow
    │   │   └── NextArrow
    │   └── ComparisonView (when compare mode)
    │       ├── ExpectedPanel
    │       │   ├── Label
    │       │   ├── ExpectedText
    │       │   └── Image (or placeholder)
    │       ├── Divider
    │       └── ActualPanel
    │           ├── Label
    │           ├── ActualText
    │           └── Image
    ├── FailureDetails (conditional footer)
    └── ThumbnailStrip (when multiple images)
        └── MiniThumbnail[] (mapped)
```

---

## Acceptance Criteria

### Functional Requirements

#### TaskQABadge
1. [ ] Badge displays correct icon and label for each of 7 status states
2. [ ] Badge uses status-appropriate colors (warning/success/error/neutral)
3. [ ] Preparing and testing states show animated spinner
4. [ ] Badge is hidden when task doesn't need QA (needsQA=false)
5. [ ] Compact variant shows icon-only with tooltip
6. [ ] Badge fits inline on TaskCard without disrupting layout

#### TaskDetailQAPanel
7. [ ] Panel displays 3 tabs: Acceptance Criteria, Test Results, Screenshots
8. [ ] Tab counts update based on actual data
9. [ ] Tabs are keyboard navigable (arrow keys)
10. [ ] Active tab has distinct visual indicator (underline)
11. [ ] Acceptance criteria show pass/fail status derived from test results
12. [ ] Criteria cards display ID, type, testable badge, and description
13. [ ] Test results show overall status banner with pass/fail count
14. [ ] Individual step results show status, ID, screenshot indicator
15. [ ] Failed steps show expandable failure details box
16. [ ] Failure details include error message, expected, and actual values
17. [ ] Screenshots tab displays ScreenshotGallery component
18. [ ] Empty states display for each tab when no data
19. [ ] Loading skeleton displays while data loads
20. [ ] Action buttons (Retry/Skip) appear when QA failed

#### QASettingsPanel
21. [ ] Panel displays section header with title
22. [ ] Master toggle enables/disables entire QA system
23. [ ] Sub-settings are indented and disabled when master is off
24. [ ] Toggle switches animate smoothly on state change
25. [ ] URL input validates URL format
26. [ ] URL updates on blur or Enter key
27. [ ] Settings persist across sessions
28. [ ] Error banner displays when settings update fails
29. [ ] Loading skeleton displays while settings load
30. [ ] Disabled toggles show reduced opacity and no cursor

#### ScreenshotGallery
31. [ ] Thumbnail grid displays with configurable columns (2, 3, or 4)
32. [ ] Thumbnails maintain 16:9 aspect ratio
33. [ ] Passed/failed indicators overlay on thumbnails
34. [ ] Comparison indicator shows when expected screenshot exists
35. [ ] Hover reveals label and timestamp overlay
36. [ ] Clicking thumbnail opens lightbox
37. [ ] Empty state displays when no screenshots

#### Lightbox
38. [ ] Lightbox opens with smooth animation
39. [ ] Header shows filename, status, counter, and controls
40. [ ] Image displays centered with max dimensions
41. [ ] Zoom in/out buttons change scale
42. [ ] Zoom level displays as percentage
43. [ ] Arrow keys navigate between images
44. [ ] Navigation arrows disabled at boundaries
45. [ ] Compare toggle switches between single and comparison view
46. [ ] Comparison view shows Expected and Actual side-by-side
47. [ ] Failure details footer shows error info for failed screenshots
48. [ ] Thumbnail strip shows all images with current highlighted
49. [ ] Escape key closes lightbox
50. [ ] Keyboard shortcuts work (arrows, +, -, 0, c)
51. [ ] Pan works when zoomed in (click and drag)

---

## Design Quality Checklist

### Colors & Theming

1. [ ] NO purple or blue gradients anywhere
2. [ ] Status colors match design system (success/warning/error/info)
3. [ ] Accent color (`#ff6b35`) used sparingly for focus and active states
4. [ ] Badge backgrounds use status colors at 15% opacity
5. [ ] Dark mode uses proper gray scale (not pure black/white)
6. [ ] Lightbox backdrop is near-black (95%) not pure black

### Typography

7. [ ] Badge text uses `text-xs` with `font-medium`
8. [ ] Section headers use `text-lg` with `font-medium`
9. [ ] Setting labels use `text-sm` with `font-medium`
10. [ ] Descriptions use `text-xs` with `--text-muted`
11. [ ] All text sizes follow type scale
12. [ ] Font weights: 400 body, 500 labels, 600 titles

### Spacing & Layout

13. [ ] Badge padding: 4px 8px
14. [ ] Tab padding: 8px 12px
15. [ ] Card padding: 12px
16. [ ] Grid gap: 12px between thumbnails
17. [ ] Lightbox padding: 16-24px
18. [ ] Sub-settings indented 24px
19. [ ] 8pt grid alignment maintained

### Shadows & Depth

20. [ ] Cards use `--bg-elevated` not shadows at rest
21. [ ] Thumbnails gain ring on hover (not shadow)
22. [ ] Focus states use `--shadow-glow`
23. [ ] Lightbox images have `--shadow-lg`
24. [ ] Lightbox backdrop creates sense of depth

### Borders & Radius

25. [ ] Badges: `--radius-sm` (4px)
26. [ ] Cards: `--radius-md` (8px)
27. [ ] Thumbnails: `--radius-lg` (12px)
28. [ ] Lightbox images: `--radius-lg` (12px)
29. [ ] Toggle switches: `--radius-full`
30. [ ] Failure details boxes have semantic border color

### Motion & Interactions

31. [ ] Spinner icons use `animate-spin`
32. [ ] Running status uses `animate-pulse`
33. [ ] Thumbnail hover: ring + scale image (200ms)
34. [ ] Toggle animation: 200ms ease
35. [ ] Lightbox open/close: fade in (200ms)
36. [ ] Zoom transitions: 200ms ease
37. [ ] Tab indicator: 150ms transition

### Icons

38. [ ] All icons from Lucide library
39. [ ] Badge icons: 12px
40. [ ] Status icons: 16px
41. [ ] Button icons: 16-20px
42. [ ] Navigation arrows: 24px
43. [ ] Empty state icons: 32-48px
44. [ ] Icons inherit appropriate colors

### Accessibility

45. [ ] Color contrast meets WCAG AA (4.5:1)
46. [ ] Focus states visible on all interactive elements
47. [ ] Tabs have proper ARIA attributes
48. [ ] Thumbnails have alt text
49. [ ] Lightbox is keyboard navigable
50. [ ] Status icons accompanied by text labels
51. [ ] Loading states use appropriate ARIA live regions

---

## Implementation Notes

### shadcn Components to Use

- `Badge` (status badges throughout)
- `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent` (panel tabs)
- `Card` (settings panel container)
- `Switch` (toggle switches)
- `Input` (URL input)
- `Label` (form labels)
- `Tooltip` (compact badge, button hints)
- `Skeleton` (loading states)
- `ScrollArea` (scrollable lists)

### CSS Custom Properties

```css
/* QA-specific */
--qa-badge-height: 22px;
--qa-thumbnail-aspect: 16 / 9;
--qa-lightbox-z: 50;

/* From DESIGN.md */
--bg-surface: #1a1a1a;
--bg-elevated: #242424;
--bg-hover: #2d2d2d;
--bg-base: #0f0f0f;
--text-primary: #f0f0f0;
--text-secondary: #a0a0a0;
--text-muted: #666666;
--accent-primary: #ff6b35;
--accent-muted: rgba(255, 107, 53, 0.15);
--border-subtle: rgba(255, 255, 255, 0.06);
--status-success: #10b981;
--status-warning: #f59e0b;
--status-error: #ef4444;
--status-info: #3b82f6;
--radius-sm: 4px;
--radius-md: 8px;
--radius-lg: 12px;
--radius-full: 9999px;
--shadow-glow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
--shadow-lg: 0 10px 15px rgba(0,0,0,0.3), 0 20px 40px rgba(0,0,0,0.25);
```

### Animation Keyframes

```css
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

### Lightbox Portal

Lightbox should render in a portal to escape parent stacking contexts:

```tsx
import { createPortal } from 'react-dom';

{isOpen && createPortal(
  <Lightbox ... />,
  document.body
)}
```

---

## References

- [DESIGN.md](../DESIGN.md) - Master design system
- [specs/DESIGN_OVERHAUL_PLAN.md](../../DESIGN_OVERHAUL_PLAN.md) - Design overhaul strategy
- [task-detail.md](./task-detail.md) - Task Detail modal (contains QA panel)
- [settings-view.md](./settings-view.md) - Settings view (contains QA settings)
- Cypress Test Runner - Test status and results reference
- Percy Visual Review - Screenshot comparison reference
- shadcn/ui Badge - https://ui.shadcn.com/docs/components/badge
- shadcn/ui Tabs - https://ui.shadcn.com/docs/components/tabs
- shadcn/ui Switch - https://ui.shadcn.com/docs/components/switch
- Lucide icons - https://lucide.dev/icons/
