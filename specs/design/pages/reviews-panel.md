# Reviews Panel

The Reviews Panel is a right-sliding panel that displays pending code reviews, allowing users to filter by reviewer type, view review details, and take action (approve or request changes). It integrates with the DiffViewer for examining code changes.

**Design Inspiration:**
- GitHub's pull request review sidebar (review list, action buttons, status indicators)
- Linear's notification panel (slide-in animation, filter tabs, compact cards)
- Slack's thread panel (slide from right, contextual actions)

**Aesthetic Direction:** Professional utility with warmth. The panel should feel like a focused workspace within the app - elevated and distinct from the main content but clearly part of the same design language. Filter tabs provide quick navigation, review cards communicate status at a glance, and action buttons invite decisive interaction.

---

## Panel Structure

### Slide-In Panel Container

The panel slides in from the right edge of the viewport, overlaying the main content.

| Property | Value | Notes |
|----------|-------|-------|
| Width | 384px | Comfortable for review cards with action buttons |
| Height | 100% viewport | Full height from header to bottom |
| Background | `--bg-surface` | Mid-elevation for panel |
| Border left | 1px `--border-subtle` | Separates from main content |
| Shadow | `--shadow-md` | Creates depth from main content |
| Z-index | 50 | Above main content, below modals |
| Position | Fixed right 0 | Anchored to viewport right |

### Slide Animation

```css
@keyframes panel-slide-in {
  from {
    transform: translateX(100%);
    opacity: 0.8;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

@keyframes panel-slide-out {
  from {
    transform: translateX(0);
    opacity: 1;
  }
  to {
    transform: translateX(100%);
    opacity: 0.8;
  }
}

.reviews-panel {
  animation: panel-slide-in 300ms ease-out;
}

.reviews-panel-closing {
  animation: panel-slide-out 250ms ease-in forwards;
}
```

- Enter: slide from right (300ms, ease-out)
- Exit: slide to right (250ms, ease-in)
- Slight opacity shift for depth illusion

### Backdrop (Optional)

When panel is open, optionally dim the main content area.

```css
.panel-backdrop {
  background: rgba(0, 0, 0, 0.2);
  /* No blur - keep main content visible but dimmed */
}
```

- Click on backdrop closes panel
- Backdrop fades in/out with panel

---

## Header Section

### Layout

```
┌─────────────────────────────────────────────────────────────┐
│  Reviews                                    [12]       [✕]  │
└─────────────────────────────────────────────────────────────┘
```

### Structure

| Element | Styling | Notes |
|---------|---------|-------|
| Container | `px-4 py-3` | Consistent with panel padding |
| Border bottom | 1px `--border-subtle` | Separates from tabs |
| Background | transparent (inherits surface) | Clean |
| Height | 52px | Comfortable header |
| Flex | `flex items-center justify-between` | Title left, count + close right |

### Panel Title

| Property | Value |
|----------|-------|
| Text | "Reviews" |
| Font size | `text-lg` (18px) |
| Font weight | `font-semibold` (600) |
| Color | `--text-primary` |
| Letter spacing | `--tracking-tight` (-0.02em) |

### Count Badge

Shows total number of pending reviews across all filter states.

| Property | Value |
|----------|-------|
| Position | Right of title, before close button |
| Content | Number (e.g., "12") |
| Font | `text-xs`, `font-medium` |
| Background | `--accent-muted` |
| Text color | `--accent-primary` |
| Padding | 4px 8px |
| Border radius | `--radius-full` (pill shape) |
| Min-width | 24px (centered text) |

```tsx
<span className="inline-flex items-center justify-center min-w-[24px] px-2 py-0.5 text-xs font-medium rounded-full bg-[var(--accent-muted)] text-[var(--accent-primary)]">
  {totalCount}
</span>
```

### Close Button

| Property | Value |
|----------|-------|
| Position | Right edge of header |
| Icon | Lucide `X` (16px) |
| Size | 32px × 32px |
| Background | transparent |
| Hover | `--bg-hover` |
| Border radius | `--radius-md` (8px) |
| Color | `--text-muted` |
| Hover color | `--text-primary` |
| Focus | `--shadow-glow` |
| Transition | 150ms ease |

---

## Filter Tabs

### Tab Container

Using **shadcn Tabs** with pills variant styling.

```
┌─────────────────────────────────────────────────────────────┐
│  [  All (12) ]  [  AI (8)  ]  [  Human (4)  ]              │
└─────────────────────────────────────────────────────────────┘
```

| Property | Value |
|----------|-------|
| Container | `px-4 py-3`, `--bg-base` subtle background strip |
| Border bottom | 1px `--border-subtle` |
| Gap between tabs | 4px |
| Tab padding | 8px 12px |

### Tab Button States

| State | Background | Text Color | Border |
|-------|------------|------------|--------|
| Default | transparent | `--text-secondary` | none |
| Hover | `--bg-hover` | `--text-primary` | none |
| Active | `--bg-elevated` | `--text-primary` | 1px `--border-subtle` |
| Focus | transparent | `--text-primary` | `--shadow-glow` |

### Tab Styling

| Property | Value |
|----------|-------|
| Font size | `text-sm` (14px) |
| Font weight | `font-medium` (500) |
| Border radius | `--radius-md` (8px) |
| Transition | 150ms ease (background, color) |
| Min-width | 64px (consistent sizing) |

### Tab Count Badges

Each tab shows its filtered count in parentheses or as a badge suffix.

| Property | Value |
|----------|-------|
| Format | "All (12)" or just "All" with count badge |
| Count font | `text-xs` |
| Count color | inherits from tab text |
| Count style | Slightly muted opacity (0.8) on inactive tabs |

```tsx
const tabs = [
  { key: 'all', label: 'All', count: totalCount },
  { key: 'ai', label: 'AI', count: aiCount, icon: Bot },
  { key: 'human', label: 'Human', count: humanCount, icon: User },
];
```

### Tab Indicator

When using underline variant instead of pills:

| Property | Value |
|----------|-------|
| Position | Bottom of active tab |
| Width | Tab content width (not full tab) |
| Height | 2px |
| Color | `--accent-primary` |
| Animation | Slide to active tab (200ms ease-out) |

---

## Review Cards

### Card Container

Using **shadcn Card** as base with custom styling.

| Property | Value | Notes |
|----------|-------|-------|
| Background | `--bg-elevated` | Highest elevation for cards |
| Border | 1px `--border-subtle` | Subtle definition |
| Border radius | `--radius-md` (8px) | Consistent with system |
| Padding | 16px | Comfortable spacing |
| Margin | 12px between cards | Stack spacing |
| Shadow | none at rest | Border provides definition |

### Card Hover State

| Property | Value |
|----------|-------|
| Transform | `translateY(-1px)` | Subtle lift |
| Shadow | `--shadow-xs` | Light elevation |
| Border color | `rgba(255, 255, 255, 0.1)` | Slightly brighter |
| Transition | 150ms ease |

```css
.review-card {
  transition: transform 150ms ease, box-shadow 150ms ease, border-color 150ms ease;
}

.review-card:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-xs);
  border-color: rgba(255, 255, 255, 0.1);
}
```

### Card Layout

```
┌─────────────────────────────────────────────────────────────┐
│  Task Title That Might Be Quite Long and Needs...          │
│  ─────────────────────────────────────────────────────────  │
│  [Pending]  🤖 AI Review                    Attempt 1 of 3 │
│  ─────────────────────────────────────────────────────────  │
│  "Review notes preview text that might be longer..."       │
│                                        [View Full]         │
│  ─────────────────────────────────────────────────────────  │
│  [View Diff]              [Request Changes]  [Approve]     │
└─────────────────────────────────────────────────────────────┘
```

### Task Title

| Property | Value |
|----------|-------|
| Font size | `text-sm` (14px) |
| Font weight | `font-semibold` (600) |
| Color | `--text-primary` |
| Truncation | Single line with ellipsis |
| Max width | 100% (container width) |
| Line height | `--leading-tight` (1.2) |

### Status Row

Contains status badge, reviewer indicator, and fix attempt counter.

| Property | Value |
|----------|-------|
| Margin top | 8px |
| Gap between items | 8px |
| Flex wrap | wrap (for narrow panels) |
| Align items | center |

### Reviewer Type Indicator

| Property | Value |
|----------|-------|
| Icon | Lucide `Bot` for AI, `User` for Human (16px) |
| Alternative | Emoji 🤖 / 👤 |
| Label | "AI Review" / "Human Review" |
| Font | `text-xs` |
| Color | `--text-secondary` |
| Gap | 4px between icon and label |

```tsx
<span className="inline-flex items-center gap-1 text-xs text-[var(--text-secondary)]">
  {type === 'ai' ? <Bot className="w-4 h-4" /> : <User className="w-4 h-4" />}
  {type === 'ai' ? 'AI Review' : 'Human Review'}
</span>
```

### Status Badge

Using **shadcn Badge** with status variants.

| Status | Background | Text Color | Icon |
|--------|------------|------------|------|
| pending | `--bg-hover` | `--text-secondary` | Lucide `Clock` |
| approved | `rgba(16, 185, 129, 0.15)` | `--status-success` | Lucide `CheckCircle` |
| changes_requested | `rgba(245, 158, 11, 0.15)` | `--status-warning` | Lucide `AlertCircle` |
| rejected | `rgba(239, 68, 68, 0.15)` | `--status-error` | Lucide `XCircle` |

**Badge Styling:**
| Property | Value |
|----------|-------|
| Font | `text-xs`, `font-medium` |
| Padding | 4px 8px |
| Border radius | `--radius-sm` (4px) |
| Icon size | 12px |
| Icon gap | 4px |

```tsx
<Badge variant={statusVariant} className="flex items-center gap-1">
  <StatusIcon className="w-3 h-3" />
  {statusLabel}
</Badge>
```

### Fix Attempt Counter

Shows when task has failed reviews and is on a fix attempt.

| Property | Value |
|----------|-------|
| Format | "Attempt N of M" |
| Font | `text-xs`, `font-medium` |
| Padding | 4px 8px |
| Border radius | `--radius-sm` (4px) |
| Normal state | `--status-warning` background, `--bg-base` text |
| At max attempts | `--status-error` background, white text |

```tsx
<span
  className={cn(
    "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
    atMax
      ? "bg-[var(--status-error)] text-white"
      : "bg-[var(--status-warning)] text-[var(--bg-base)]"
  )}
  data-at-max={atMax}
>
  Attempt {attempt} of {max}
</span>
```

### Notes Preview

Shows first 2 lines of review notes with expansion option.

| Property | Value |
|----------|-------|
| Container | margin-top 12px |
| Font | `text-sm` (14px) |
| Color | `--text-secondary` |
| Line height | `--leading-normal` (1.5) |
| Max lines | 2, with ellipsis |
| Style | Italic, with opening quote |
| Background | Optional: subtle `--bg-base` with 8px padding and `--radius-sm` |

**"View Full" Link:**
| Property | Value |
|----------|-------|
| Position | Below notes preview, right-aligned |
| Font | `text-xs` |
| Color | `--accent-primary` |
| Hover | underline |
| Cursor | pointer |

```tsx
{review.notes && (
  <div className="mt-3">
    <p className="text-sm text-[var(--text-secondary)] italic line-clamp-2">
      "{review.notes}"
    </p>
    {review.notes.length > 100 && (
      <button
        onClick={onViewFull}
        className="text-xs text-[var(--accent-primary)] hover:underline mt-1"
      >
        View Full
      </button>
    )}
  </div>
)}
```

---

## Action Buttons

### Button Row

| Property | Value |
|----------|-------|
| Container | margin-top 16px |
| Flex | `flex flex-wrap gap-2` |
| Justify | space-between (View Diff left, actions right) |

### View Diff Button

| Property | Value |
|----------|-------|
| Variant | Ghost/secondary |
| Background | `--bg-hover` |
| Text color | `--text-primary` |
| Hover | `--bg-base` |
| Icon | Lucide `GitCompare` or `FileCode` (16px, optional) |
| Padding | 8px 12px |
| Border radius | `--radius-md` |
| Font | `text-sm`, `font-medium` |

### Request Changes Button

| Property | Value |
|----------|-------|
| Variant | Warning/secondary |
| Background | `--status-warning` |
| Text color | `--bg-base` (dark on amber) |
| Hover | darken 10% |
| Active | `scale(0.98)` |
| Icon | Lucide `MessageSquare` (16px, optional) |
| Padding | 8px 12px |
| Border radius | `--radius-md` |
| Font | `text-sm`, `font-medium` |

### Approve Button

| Property | Value |
|----------|-------|
| Variant | Success/primary |
| Background | `--status-success` |
| Text color | white |
| Hover | darken 10% |
| Active | `scale(0.98)` |
| Icon | Lucide `Check` (16px, optional) |
| Padding | 8px 12px |
| Border radius | `--radius-md` |
| Font | `text-sm`, `font-medium` |

```tsx
<div className="flex flex-wrap gap-2 mt-4">
  <Button
    variant="ghost"
    size="sm"
    onClick={() => onViewDiff(review.id)}
    className="bg-[var(--bg-hover)] hover:bg-[var(--bg-base)]"
  >
    <GitCompare className="w-4 h-4 mr-1.5" />
    View Diff
  </Button>

  {isPending && (
    <>
      <Button
        size="sm"
        onClick={() => onRequestChanges(review.id)}
        className="bg-[var(--status-warning)] text-[var(--bg-base)] hover:opacity-90"
      >
        Request Changes
      </Button>
      <Button
        size="sm"
        onClick={() => onApprove(review.id)}
        className="bg-[var(--status-success)] text-white hover:opacity-90"
      >
        Approve
      </Button>
    </>
  )}
</div>
```

### Button States

| State | Effect |
|-------|--------|
| Disabled | 50% opacity, no pointer events |
| Loading | Lucide `Loader2` spinner replacing icon, disabled |
| Focus | `--shadow-glow` |

---

## Detail View (Diff Integration)

When "View Diff" is clicked, the panel switches to detail view.

### Detail Header

```
┌─────────────────────────────────────────────────────────────┐
│  [←]  Task Title Here                                       │
│       AI Review • pending                                   │
│  ─────────────────────────────────────────────────────────  │
│                           [Request Changes]  [Approve]      │
└─────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Back button | Lucide `ChevronLeft` (16px), `--text-secondary` |
| Title | `text-base`, `font-semibold`, truncate |
| Subtitle | `text-xs`, `--text-muted`, format: "AI Review • pending" |
| Actions | Same as card actions, compact layout |
| Divider | 1px `--border-subtle` below |

### DiffViewer Integration

The detail view embeds the `DiffViewer` component.

| Property | Value |
|----------|-------|
| Container | Flex-1 to fill remaining height |
| Min height | 0 (allow shrinking for flex) |
| Default tab | "changes" (uncommitted changes) |
| Tabs | Changes, History |

---

## Empty State

When no reviews match the current filter.

### Layout

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│                    ┌─────────────┐                          │
│                    │   (icon)    │                          │
│                    └─────────────┘                          │
│                  No pending reviews                         │
│              All reviews have been handled                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Container | `flex flex-col items-center justify-center`, padding 48px |
| Icon | Lucide `CheckCircle2` (dashed variant) or `Inbox` (48px) |
| Icon color | `--text-muted`, 50% opacity |
| Title | "No pending reviews", `text-sm`, `font-medium`, `--text-secondary` |
| Subtitle | "All reviews have been handled", `text-xs`, `--text-muted` |

```tsx
<div className="flex flex-col items-center justify-center p-12 text-center">
  <CheckCircle2
    className="w-12 h-12 mb-3 opacity-50"
    style={{ color: 'var(--text-muted)' }}
    strokeDasharray="4 4"
  />
  <p className="text-sm font-medium text-[var(--text-secondary)]">
    No pending reviews
  </p>
  <p className="text-xs text-[var(--text-muted)] mt-1">
    All reviews have been handled
  </p>
</div>
```

---

## Loading State

### Initial Load

When panel first opens and reviews are loading.

```tsx
<div className="flex items-center justify-center p-12">
  <Loader2
    className="w-6 h-6 animate-spin"
    style={{ color: 'var(--accent-primary)' }}
  />
</div>
```

### Card Skeleton

For individual card loading states (if needed).

```tsx
<div className="p-4 rounded-lg border bg-[var(--bg-elevated)] border-[var(--border-subtle)]">
  <Skeleton className="h-4 w-3/4 mb-2" />
  <div className="flex gap-2 mt-2">
    <Skeleton className="h-5 w-16 rounded-sm" />
    <Skeleton className="h-5 w-20 rounded-sm" />
  </div>
  <Skeleton className="h-3 w-full mt-3" />
  <Skeleton className="h-3 w-2/3 mt-1" />
</div>
```

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Escape | Close panel |
| Tab | Navigate between filter tabs and cards |
| Enter | Activate focused element |
| Arrow Up/Down | Navigate between review cards |
| 1/2/3 | Switch to tab (All/AI/Human) |

---

## Lucide Icons Used

| Icon | Usage | Size |
|------|-------|------|
| `X` | Close panel button | 16px |
| `ChevronLeft` | Back button in detail view | 16px |
| `Bot` | AI reviewer indicator | 16px |
| `User` | Human reviewer indicator | 16px |
| `Clock` | Pending status | 12px |
| `CheckCircle` | Approved status | 12px |
| `AlertCircle` | Changes requested status | 12px |
| `XCircle` | Rejected status | 12px |
| `GitCompare` | View diff button (alternative: `FileCode`) | 16px |
| `MessageSquare` | Request changes (optional icon) | 16px |
| `Check` | Approve button (optional icon) | 16px |
| `Loader2` | Loading spinner | 24px |
| `CheckCircle2` | Empty state icon | 48px |
| `Inbox` | Alternative empty state icon | 48px |

---

## Component Hierarchy

```
ReviewsPanel
├── PanelContainer (fixed right, slide animation)
│   │
│   ├── Header
│   │   ├── PanelTitle ("Reviews")
│   │   ├── CountBadge (total pending count)
│   │   └── CloseButton (Lucide X)
│   │
│   ├── FilterTabs (shadcn Tabs)
│   │   ├── TabsList
│   │   │   ├── TabTrigger "All" + count
│   │   │   ├── TabTrigger "AI" + count + Bot icon
│   │   │   └── TabTrigger "Human" + count + User icon
│   │   └── (Tab content is the scrollable list below)
│   │
│   ├── ScrollArea (shadcn, content container)
│   │   ├── LoadingSpinner (conditional)
│   │   ├── EmptyState (conditional)
│   │   └── ReviewCardList
│   │       └── ReviewCard[] (mapped from filtered reviews)
│   │           ├── TaskTitle (truncated)
│   │           ├── StatusRow
│   │           │   ├── ReviewStatusBadge
│   │           │   ├── ReviewerTypeIndicator (Bot/User icon)
│   │           │   └── FixAttemptCounter (conditional)
│   │           ├── NotesPreview (conditional)
│   │           │   ├── NotesText (italic, 2 lines max)
│   │           │   └── ViewFullLink
│   │           └── ActionButtons (conditional - only if pending)
│   │               ├── ViewDiffButton
│   │               ├── RequestChangesButton
│   │               └── ApproveButton
│   │
│   └── DetailView (conditional - when viewing diff)
│       ├── DetailHeader
│       │   ├── BackButton
│       │   ├── TaskInfo (title + subtitle)
│       │   └── ActionButtons
│       └── DiffViewerContainer
│           └── DiffViewer (existing component)
│
└── PanelBackdrop (optional, closes panel on click)

ReviewNotesModal (separate component, opened by actions)
├── Dialog (shadcn)
│   ├── DialogHeader
│   │   ├── Title ("Request Changes" or "Approve with Notes")
│   │   └── CloseButton
│   ├── DialogContent
│   │   ├── NotesTextarea (required for request changes)
│   │   └── FixDescriptionTextarea (optional, for request changes)
│   └── DialogFooter
│       ├── CancelButton
│       └── SubmitButton
```

---

## Acceptance Criteria

### Functional Requirements

1. [ ] Panel slides in from right edge when opened
2. [ ] Panel slides out to right when closed
3. [ ] Panel closes on Escape key press
4. [ ] Panel closes on backdrop click (if backdrop enabled)
5. [ ] Close button closes panel with focus management
6. [ ] Header shows "Reviews" title and total pending count
7. [ ] Count badge updates when reviews change
8. [ ] Filter tabs switch between All, AI, and Human reviews
9. [ ] Tab counts show filtered review counts
10. [ ] Active tab has distinct styling (elevated background)
11. [ ] Review cards display for each pending review
12. [ ] Card shows task title (truncated if long)
13. [ ] Card shows review status badge with icon
14. [ ] Card shows reviewer type indicator (AI/Human)
15. [ ] Card shows fix attempt counter when applicable
16. [ ] Counter turns red when at max attempts
17. [ ] Notes preview shows first 2 lines of review notes
18. [ ] "View Full" link appears for long notes
19. [ ] View Diff button switches to detail view
20. [ ] Detail view shows DiffViewer with changes
21. [ ] Back button returns to list view
22. [ ] Approve button triggers approval flow (with optional notes)
23. [ ] Request Changes button opens modal for notes input
24. [ ] Empty state displays when no reviews match filter
25. [ ] Loading spinner displays while fetching reviews
26. [ ] Review events trigger list refresh

---

## Design Quality Checklist

### Colors & Theming

1. [ ] NO purple or blue gradients anywhere
2. [ ] Panel background uses `--bg-surface`
3. [ ] Card background uses `--bg-elevated`
4. [ ] Status badge colors match defined system colors (success/warning/error)
5. [ ] Accent color (`#ff6b35`) used only for count badge and focus states
6. [ ] Reviewer indicators use neutral secondary colors
7. [ ] Action buttons use semantic colors (green for approve, amber for changes)

### Typography

8. [ ] Panel title uses `text-lg` with `--tracking-tight`
9. [ ] Card titles use `text-sm` with `font-semibold`
10. [ ] All text sizes follow type scale (text-xs through text-lg)
11. [ ] Font weights: 400 for body, 500 for labels, 600 for titles
12. [ ] Notes preview uses `--leading-normal` for readability

### Spacing & Layout

13. [ ] Panel width: 384px
14. [ ] Header padding: 16px horizontal, 12px vertical
15. [ ] Card padding: 16px
16. [ ] Card margin: 12px between cards
17. [ ] 8pt grid alignment maintained throughout
18. [ ] 8px gap between status row items
19. [ ] 16px gap before action buttons

### Shadows & Depth

20. [ ] Panel has `--shadow-md` for floating effect
21. [ ] Cards have no shadow at rest (border provides definition)
22. [ ] Cards gain `--shadow-xs` on hover
23. [ ] Focus rings use `--shadow-glow`
24. [ ] Backdrop has subtle dimming (no blur)

### Borders & Radius

25. [ ] Panel left border: 1px `--border-subtle`
26. [ ] Card border radius: `--radius-md` (8px)
27. [ ] Badge border radius: `--radius-sm` (4px)
28. [ ] Tab border radius: `--radius-md` (8px)
29. [ ] Button border radius: `--radius-md` (8px)

### Motion & Interactions

30. [ ] Panel slide animation: 300ms ease-out (in), 250ms ease-in (out)
31. [ ] Card hover lift: translateY(-1px), 150ms
32. [ ] Tab transitions: 150ms ease
33. [ ] Button active: scale(0.98)
34. [ ] All transitions use appropriate easing
35. [ ] Loading spinner uses `animate-spin`

### Icons

36. [ ] All icons from Lucide library
37. [ ] Close button icon: 16px
38. [ ] Reviewer icons: 16px
39. [ ] Status badge icons: 12px
40. [ ] Button icons: 16px
41. [ ] Empty state icon: 48px
42. [ ] Icons inherit appropriate colors

### Accessibility

43. [ ] Color contrast meets WCAG AA (4.5:1)
44. [ ] Focus states visible on all interactive elements
45. [ ] Panel has role="complementary" or appropriate ARIA
46. [ ] Close button has aria-label="Close reviews panel"
47. [ ] Tabs have proper ARIA attributes (role="tablist", etc.)
48. [ ] Review cards are keyboard navigable

---

## Implementation Notes

### shadcn Components to Use

- `Card`, `CardContent`, `CardHeader` (review cards)
- `Badge` (status badges, count badges)
- `Button` (all action buttons)
- `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent` (filter tabs)
- `ScrollArea` (scrollable card list)
- `Skeleton` (loading states)
- `Dialog`, `DialogContent`, `DialogHeader`, `DialogFooter` (notes modal)
- `Textarea` (notes input)
- `Separator` (optional dividers)

### CSS Custom Properties

```css
/* Panel-specific */
--panel-width: 384px;
--panel-slide-duration: 300ms;

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
--radius-sm: 4px;
--radius-md: 8px;
--shadow-xs: 0 1px 2px rgba(0,0,0,0.2), 0 1px 3px rgba(0,0,0,0.1);
--shadow-md: 0 4px 6px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2);
--shadow-glow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
```

### Animation Keyframes

```css
@keyframes panel-slide-in {
  from {
    transform: translateX(100%);
    opacity: 0.8;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

@keyframes panel-slide-out {
  from {
    transform: translateX(0);
    opacity: 1;
  }
  to {
    transform: translateX(100%);
    opacity: 0.8;
  }
}
```

---

## References

- [DESIGN.md](../DESIGN.md) - Master design system
- [specs/DESIGN_OVERHAUL_PLAN.md](../../DESIGN_OVERHAUL_PLAN.md) - Design overhaul strategy
- [modal-standards.md](./modal-standards.md) - Modal patterns (when complete)
- [task-detail.md](./task-detail.md) - Task Detail modal reference
- GitHub Pull Request sidebar - Review list reference
- Linear Notification panel - Slide-in panel reference
- shadcn/ui Tabs - https://ui.shadcn.com/docs/components/tabs
- shadcn/ui Card - https://ui.shadcn.com/docs/components/card
- Lucide icons - https://lucide.dev/icons/
