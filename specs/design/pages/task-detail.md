# Task Detail View

The Task Detail View displays comprehensive information about a single task in a modal dialog. It serves as the central hub for understanding task status, reading descriptions, reviewing state history, and viewing associated reviews and QA results.

**Design Inspiration:**
- Linear's issue detail panel (clean layout, clear hierarchy, keyboard shortcuts)
- GitHub's pull request detail (timeline-based history, status indicators)
- Raycast's command detail (focused modal, quick actions)

**Aesthetic Direction:** Professional utility with depth. The modal should feel substantial and elevated from the background, with clear visual hierarchy that guides the eye from title to status to content to history.

---

## Modal Structure

### Dialog Container

Using **shadcn Dialog** with custom sizing and glass effect.

```tsx
<Dialog>
  <DialogContent className="max-w-[640px] max-h-[80vh] overflow-hidden flex flex-col">
    {/* content */}
  </DialogContent>
</Dialog>
```

| Property | Value | Notes |
|----------|-------|-------|
| Max width | 640px | Comfortable reading width |
| Max height | 80vh | Leaves visual context of background |
| Background | `--bg-elevated` | Highest elevation level |
| Border | 1px `--border-subtle` | Subtle definition |
| Border radius | `--radius-lg` (12px) | Premium feel |
| Shadow | `--shadow-lg` | Strong floating effect |
| Overflow | hidden with internal scroll | Content area scrolls, header/footer fixed |

### Backdrop

```css
.backdrop {
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(8px);
}
```

- Semi-transparent black overlay
- Blur effect for glass feel
- Click to close (unless destructive action pending)
- Fade in: `opacity 0 → 1`, 200ms

### Open Animation

```css
@keyframes modal-enter {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.dialog-content {
  animation: modal-enter 200ms ease-out;
}
```

### Close Animation

```css
@keyframes modal-exit {
  from {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
  to {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
}
```

---

## Header Section

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│  [P1]  Task Title Here That Might Be Long          [Status] │
│        Category Badge   ────────────────────────────    [✕]  │
└──────────────────────────────────────────────────────────────┘
```

### Structure

| Element | Styling | Notes |
|---------|---------|-------|
| Container | `px-6 pt-6 pb-4` | Generous top padding |
| Border bottom | 1px `--border-subtle` | Separates from content |
| Background | transparent (inherits elevated) | Clean |

### Priority Indicator

| Property | Value |
|----------|-------|
| Position | Left of title, inline |
| Format | "P1", "P2", "P3" etc. |
| Font | `text-xs`, `font-mono`, `font-medium` |
| Background | `--bg-base` |
| Border radius | `--radius-sm` (4px) |
| Padding | 4px 8px |
| Colors by priority: | |
| P1 (Critical) | `--status-error` background, white text |
| P2 (High) | `--accent-primary` background, white text |
| P3 (Medium) | `--status-warning` background, `--bg-base` text |
| P4 (Low) | `--bg-hover` background, `--text-secondary` text |

```tsx
const priorityColors: Record<number, { bg: string; text: string }> = {
  1: { bg: 'var(--status-error)', text: 'white' },
  2: { bg: 'var(--accent-primary)', text: 'white' },
  3: { bg: 'var(--status-warning)', text: 'var(--bg-base)' },
  4: { bg: 'var(--bg-hover)', text: 'var(--text-secondary)' },
};
```

### Task Title

| Property | Value |
|----------|-------|
| Font size | `text-xl` (20px) |
| Font weight | `font-semibold` (600) |
| Color | `--text-primary` |
| Letter spacing | `--tracking-tight` (-0.02em) |
| Line height | `--leading-tight` (1.2) |
| Truncation | Single line, ellipsis |
| Max width | ~80% of container (room for close button) |

### Status Badge

Using **shadcn Badge** with custom status variants.

| Status | Background | Text Color |
|--------|------------|------------|
| backlog | `--bg-hover` | `--text-muted` |
| ready | `rgba(59, 130, 246, 0.15)` | `--status-info` |
| blocked | `rgba(245, 158, 11, 0.15)` | `--status-warning` |
| executing | `rgba(255, 107, 53, 0.15)` | `--accent-primary` |
| execution_done | `rgba(59, 130, 246, 0.15)` | `--status-info` |
| qa_refining | `rgba(255, 107, 53, 0.15)` | `--accent-primary` |
| qa_testing | `rgba(255, 107, 53, 0.15)` | `--accent-primary` |
| qa_passed | `rgba(16, 185, 129, 0.15)` | `--status-success` |
| qa_failed | `rgba(239, 68, 68, 0.15)` | `--status-error` |
| pending_review | `rgba(245, 158, 11, 0.15)` | `--status-warning` |
| revision_needed | `rgba(245, 158, 11, 0.15)` | `--status-warning` |
| approved | `rgba(16, 185, 129, 0.15)` | `--status-success` |
| failed | `rgba(239, 68, 68, 0.15)` | `--status-error` |
| cancelled | `--bg-hover` | `--text-muted` |

**Badge Styling:**
- Font: `text-xs`, `font-medium`
- Padding: 4px 8px
- Border radius: `--radius-md` (8px)
- Uppercase: No (sentence case, e.g., "Pending Review")

### Category Badge

| Property | Value |
|----------|-------|
| Background | `--bg-base` |
| Border | 1px `--border-subtle` |
| Text color | `--text-secondary` |
| Font | `text-xs`, `font-medium` |
| Padding | 4px 10px |
| Border radius | `--radius-sm` (4px) |
| Margin top | 8px |

### Close Button

| Property | Value |
|----------|-------|
| Position | Absolute, top-right |
| Icon | Lucide `X` (16px) |
| Size | 32px × 32px |
| Background | transparent |
| Hover | `--bg-hover` |
| Border radius | `--radius-md` (8px) |
| Color | `--text-muted` |
| Hover color | `--text-primary` |
| Focus | `--shadow-glow` |

```tsx
<button
  onClick={onClose}
  className="absolute top-4 right-4 p-2 rounded-lg text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)]"
>
  <X className="w-4 h-4" />
</button>
```

---

## Content Sections

### Scrollable Container

```tsx
<ScrollArea className="flex-1 overflow-y-auto">
  <div className="px-6 py-4 space-y-6">
    {/* sections */}
  </div>
</ScrollArea>
```

Using **shadcn ScrollArea** for custom scrollbar styling.

| Property | Value |
|----------|-------|
| Scrollbar width | 6px |
| Scrollbar color | `--bg-hover` |
| Scrollbar hover | `--text-muted` |
| Scrollbar radius | `--radius-full` |

### Description Section

| Property | Value |
|----------|-------|
| Font | `text-sm` (14px) |
| Color | `--text-secondary` |
| Line height | `--leading-relaxed` (1.65) |
| Margin top | 0 (first section) |
| Max lines | None (full display) |
| Word break | `break-words` |

**Empty State:**
- Italic text: "No description provided"
- Color: `--text-muted`

### Steps/Checklist Section (if applicable)

When task has steps or acceptance criteria.

```
┌──────────────────────────────────────────────────────────────┐
│  Steps                                                        │
│  ────────────────────────────────────────────────────────────│
│  ☑ Set up project structure                                  │
│  ☑ Implement core logic                                      │
│  ☐ Write tests                                               │
│  ☐ Update documentation                                      │
└──────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Section title | `text-sm`, `font-medium`, `--text-primary`, margin-bottom 12px |
| Checkbox | Lucide `CheckSquare` (completed) or `Square` (pending), 16px |
| Completed | `--status-success` icon color, `line-through` text (optional) |
| Pending | `--text-muted` icon color, normal text |
| Step text | `text-sm`, `--text-secondary` |
| Gap between items | 8px |

### Reviews Section

```
┌──────────────────────────────────────────────────────────────┐
│  Reviews                                                      │
│  ────────────────────────────────────────────────────────────│
│  🤖 AI Review          ┌─────────────────┐                   │
│                        │    Approved     │                   │
│                        └─────────────────┘                   │
│                                                               │
│  👤 Human Review       ┌─────────────────┐                   │
│                        │    Pending      │                   │
│                        └─────────────────┘                   │
└──────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Section title | `text-sm`, `font-medium`, `--text-primary`, margin-bottom 12px |
| Review card | Using shadcn Card, padding 12px |
| Card background | `--bg-surface` |
| Card border | 1px `--border-subtle` |
| Card border radius | `--radius-md` (8px) |
| Reviewer icon | 🤖 or 👤, or Lucide `Bot` / `User` (16px) |
| Reviewer label | `text-sm`, `font-medium`, `--text-primary` |
| Status badge | shadcn Badge with status color |
| Gap between cards | 8px |

**Status Badge Colors:**
- pending: `--bg-hover`, `--text-muted`
- approved: success variant (green)
- changes_requested: warning variant (amber)
- rejected: destructive variant (red)

**Fix Task Indicator:**
If fix tasks exist, show indicator below reviews.

| Property | Value |
|----------|-------|
| Icon | Lucide `Wrench` (16px) |
| Color | `--status-warning` |
| Text | "N fix task(s)" |
| Font | `text-sm`, `--text-secondary` |

### QA Section (if applicable)

When task has QA data.

```
┌──────────────────────────────────────────────────────────────┐
│  QA Results                                                   │
│  ────────────────────────────────────────────────────────────│
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ✓ Form validation works correctly                     │  │
│  │  ✓ Error messages display properly                     │  │
│  │  ✗ Submit button disabled state not working            │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                               │
│  [View Screenshots]                                           │
└──────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Section title | `text-sm`, `font-medium`, `--text-primary` |
| Criteria list | Same styling as Steps section |
| Passed | `--status-success` check icon |
| Failed | `--status-error` X icon |
| Screenshots button | Ghost button with Lucide `Image` icon |

Integrate with `TaskDetailQAPanel` component for full QA display.

---

## State History Timeline

### Section Structure

```
┌──────────────────────────────────────────────────────────────┐
│  History                                                      │
│  ────────────────────────────────────────────────────────────│
│  │                                                            │
│  ●──── Approved                                   2 min ago  │
│  │     by: AI Reviewer                                       │
│  │     "Code meets all requirements"                         │
│  │                                                            │
│  ○──── Changes Requested                          1 hour ago │
│  │     by: Human Reviewer                                    │
│  │     "Please add error handling"                           │
│  │                                                            │
│  ○──── Submitted for Review                       2 hours ago│
│  │     by: Worker Agent                                      │
│  │                                                            │
└──────────────────────────────────────────────────────────────┘
```

### Timeline Layout

| Property | Value |
|----------|-------|
| Section title | `text-sm`, `font-medium`, `--text-primary` |
| Container | padding 16px, `--bg-surface`, `--radius-md` |
| Vertical line | 2px wide, `--border-subtle`, left offset 5px |
| Entry gap | 16px between entries |

### Timeline Entry

```tsx
<div className="relative pl-6">
  {/* Vertical connector line */}
  <div className="absolute left-[5px] top-3 bottom-0 w-0.5 bg-[var(--border-subtle)]" />

  {/* Status dot */}
  <div
    className="absolute left-0 top-1 w-3 h-3 rounded-full border-2 border-[var(--bg-elevated)]"
    style={{ backgroundColor: outcomeColor }}
  />

  {/* Content */}
  <div className="flex items-center justify-between gap-2">
    <span className="font-medium text-sm">{outcomeLabel}</span>
    <span className="text-xs text-[var(--text-muted)]">{relativeTime}</span>
  </div>
  <div className="text-xs text-[var(--text-secondary)] mt-0.5">
    by: {actorLabel}
  </div>
  {notes && (
    <div className="text-xs text-[var(--text-secondary)] mt-1 italic">
      "{notes}"
    </div>
  )}
</div>
```

### Timeline Dot

| Property | Value |
|----------|-------|
| Size | 12px (outer), 8px (inner colored) |
| Border | 2px `--bg-elevated` (creates ring effect) |
| Position | absolute left-0, centered on first line |

**Dot Colors by Outcome:**
| Outcome | Color |
|---------|-------|
| approved | `--status-success` |
| changes_requested | `--status-warning` |
| rejected | `--status-error` |
| state_change | `--text-muted` |

**Current/Latest Entry:**
- Larger dot (16px)
- Subtle glow: `0 0 0 4px rgba(color, 0.2)`
- Filled solid (no ring effect)

### Timeline Connector

| Property | Value |
|----------|-------|
| Width | 2px |
| Color | `--border-subtle` |
| Position | Left 5px (centered under dots) |
| Extends from | First entry to last entry |
| Last entry | No line below (hide connector) |

### Timestamp

| Property | Value |
|----------|-------|
| Format | Relative ("2 min ago", "1 hour ago", "3 days ago") |
| Font | `text-xs` |
| Color | `--text-muted` |
| Position | Right-aligned |
| Fallback | "Just now" for < 1 minute |

### Actor Label

| Type | Label |
|------|-------|
| ai | "AI Reviewer" or "Worker Agent" |
| human | "Human Reviewer" or "User" |
| system | "System" |

### Notes Quote

| Property | Value |
|----------|-------|
| Font | `text-xs`, italic |
| Color | `--text-secondary` |
| Prefix | Opening quote mark |
| Max lines | 2, with ellipsis |
| Background | Optional: very subtle `--bg-base` for emphasis |

### Empty State

```tsx
<div className="flex flex-col items-center justify-center py-8 text-center">
  <History className="w-8 h-8 text-[var(--text-muted)] mb-2 opacity-50" />
  <p className="text-sm text-[var(--text-muted)]">No history yet</p>
  <p className="text-xs text-[var(--text-muted)] mt-1">
    Status changes will appear here
  </p>
</div>
```

- Icon: Lucide `History` (32px, dashed style)
- Color: `--text-muted`, 50% opacity
- Text: "No history yet"

### Loading State

```tsx
<div className="flex justify-center py-8">
  <Loader2 className="w-6 h-6 text-[var(--text-muted)] animate-spin" />
</div>
```

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Escape | Close modal |
| Tab | Navigate focusable elements |
| Shift+Tab | Navigate backwards |

---

## Lucide Icons Used

| Icon | Usage | Size |
|------|-------|------|
| `X` | Close button | 16px |
| `CheckSquare` | Completed step | 16px |
| `Square` | Pending step | 16px |
| `CheckCircle` | Passed criteria | 16px |
| `XCircle` | Failed criteria | 16px |
| `Bot` | AI reviewer (alternative to emoji) | 16px |
| `User` | Human reviewer (alternative to emoji) | 16px |
| `Wrench` | Fix tasks indicator | 16px |
| `Image` | View screenshots button | 16px |
| `History` | Empty history state | 32px |
| `Loader2` | Loading spinner | 24px |

---

## Component Hierarchy

```
TaskDetailModal
├── Dialog (shadcn)
│   ├── DialogOverlay (backdrop blur)
│   └── DialogContent
│       ├── Header
│       │   ├── PriorityBadge
│       │   ├── TaskTitle
│       │   ├── StatusBadge (shadcn Badge)
│       │   ├── CategoryBadge
│       │   └── CloseButton (Lucide X)
│       │
│       ├── ScrollArea (shadcn)
│       │   ├── DescriptionSection
│       │   │   └── DescriptionText | EmptyState
│       │   │
│       │   ├── StepsSection (conditional)
│       │   │   ├── SectionTitle
│       │   │   └── StepItem[]
│       │   │       ├── Checkbox icon
│       │   │       └── StepText
│       │   │
│       │   ├── ReviewsSection (conditional)
│       │   │   ├── SectionTitle
│       │   │   ├── ReviewCard[]
│       │   │   │   ├── ReviewerIcon
│       │   │   │   ├── ReviewerLabel
│       │   │   │   └── StatusBadge
│       │   │   └── FixTaskIndicator (conditional)
│       │   │
│       │   ├── QASection (conditional)
│       │   │   ├── SectionTitle
│       │   │   ├── CriteriaList
│       │   │   └── ScreenshotsButton
│       │   │
│       │   └── HistorySection
│       │       ├── SectionTitle
│       │       ├── LoadingSpinner (conditional)
│       │       ├── EmptyState (conditional)
│       │       └── TimelineContainer
│       │           └── TimelineEntry[]
│       │               ├── VerticalConnector
│       │               ├── StatusDot
│       │               ├── OutcomeLabel + Timestamp
│       │               ├── ActorLabel
│       │               └── NotesQuote (conditional)
│       │
│       └── Footer (optional, for actions)
│           └── ActionButtons
```

---

## Acceptance Criteria

### Functional Requirements

1. [ ] Modal opens with scale + fade animation
2. [ ] Modal closes on Escape key
3. [ ] Modal closes on backdrop click
4. [ ] Close button closes modal with proper focus management
5. [ ] Task title displays correctly (truncated if long)
6. [ ] Priority badge shows correct level (P1-P4) with appropriate color
7. [ ] Status badge shows current internal status with correct color
8. [ ] Category badge displays task category
9. [ ] Description section shows full task description
10. [ ] Empty description shows "No description provided" placeholder
11. [ ] Steps/checklist renders when task has steps
12. [ ] Completed steps show check icon and optional strikethrough
13. [ ] Reviews section shows all associated reviews
14. [ ] Each review shows reviewer type (AI/Human) and status
15. [ ] Fix task indicator shows when fix tasks exist
16. [ ] QA section integrates with TaskDetailQAPanel when QA data exists
17. [ ] History timeline loads and displays state transitions
18. [ ] Timeline shows loading spinner while fetching
19. [ ] Timeline shows empty state when no history
20. [ ] Timeline entries show outcome, actor, timestamp, and notes
21. [ ] Timestamps display in relative format ("2 min ago")
22. [ ] Latest timeline entry has enhanced styling
23. [ ] Content area scrolls when content exceeds modal height
24. [ ] All interactive elements are keyboard accessible
25. [ ] Focus trapped within modal while open
26. [ ] ARIA attributes present for accessibility

---

## Design Quality Checklist

### Colors & Theming

1. [ ] NO purple or blue gradients anywhere
2. [ ] Modal background uses `--bg-elevated`
3. [ ] Status badge colors match defined system colors
4. [ ] Priority badge uses semantic colors appropriately
5. [ ] Accent color (`#ff6b35`) used sparingly - only for P2 priority and focus states
6. [ ] Timeline dots use appropriate outcome colors

### Typography

7. [ ] Title uses SF Pro with `--tracking-tight`
8. [ ] All text sizes follow type scale (text-xs through text-xl)
9. [ ] Font weights: 400 for body, 500 for labels, 600 for titles
10. [ ] Description uses `--leading-relaxed` for readability
11. [ ] Monospace font for priority badge

### Spacing & Layout

12. [ ] Modal padding: 24px horizontal, 24px top, 16px between sections
13. [ ] 8pt grid alignment maintained throughout
14. [ ] Consistent 12px gap between section items
15. [ ] 16px gap between timeline entries
16. [ ] Close button positioned with 16px offset from edges

### Shadows & Depth

17. [ ] Modal uses `--shadow-lg` for strong floating effect
18. [ ] Backdrop uses blur effect for glass feel
19. [ ] Focus rings use `--shadow-glow`
20. [ ] Timeline dots on elevated background have subtle ring effect

### Borders & Radius

21. [ ] Modal border radius: `--radius-lg` (12px)
22. [ ] Badge border radius: `--radius-md` (8px) for status, `--radius-sm` (4px) for others
23. [ ] Section cards use `--radius-md`
24. [ ] Timeline container uses `--radius-md`

### Motion & Interactions

25. [ ] Modal open animation: scale(0.95) → scale(1), 200ms
26. [ ] Modal close animation: reverse of open
27. [ ] Close button hover transitions smoothly (150ms)
28. [ ] Timeline entries do not have hover effects (read-only)
29. [ ] Loading spinner uses `animate-spin`
30. [ ] All transitions use `ease-out` timing function

### Icons

31. [ ] All icons from Lucide library
32. [ ] Close button icon: 16px
33. [ ] Inline icons: 16px
34. [ ] Empty state icons: 32px with reduced opacity
35. [ ] Icons inherit appropriate colors

### Accessibility

36. [ ] Color contrast meets WCAG AA (4.5:1)
37. [ ] Focus states visible on all interactive elements
38. [ ] Modal has role="dialog" and aria-modal="true"
39. [ ] Close button has aria-label="Close"
40. [ ] Screen reader announcements for status changes

---

## Implementation Notes

### shadcn Components to Use

- `Dialog`, `DialogContent`, `DialogOverlay`, `DialogClose`
- `Badge` (with custom variants for statuses)
- `ScrollArea`
- `Button` (ghost variant for close)
- `Card` (for review cards, optional)
- `Skeleton` (for loading states)

### CSS Custom Properties

```css
/* Modal-specific */
--modal-max-width: 640px;
--modal-max-height: 80vh;

/* From DESIGN.md */
--bg-elevated: #242424;
--bg-surface: #1a1a1a;
--bg-hover: #2d2d2d;
--text-primary: #f0f0f0;
--text-secondary: #a0a0a0;
--text-muted: #666666;
--accent-primary: #ff6b35;
--border-subtle: rgba(255, 255, 255, 0.06);
--status-success: #10b981;
--status-warning: #f59e0b;
--status-error: #ef4444;
--status-info: #3b82f6;
--radius-sm: 4px;
--radius-md: 8px;
--radius-lg: 12px;
--shadow-lg: 0 10px 15px rgba(0,0,0,0.3), 0 20px 40px rgba(0,0,0,0.25);
--shadow-glow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
```

### Animation Keyframes

```css
@keyframes modal-enter {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes modal-exit {
  from {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
  to {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
}
```

---

## References

- [DESIGN.md](../DESIGN.md) - Master design system
- [specs/DESIGN_OVERHAUL_PLAN.md](../../DESIGN_OVERHAUL_PLAN.md) - Design overhaul strategy
- [modal-standards.md](./modal-standards.md) - Modal patterns (when complete)
- Linear.app - Issue detail panel reference
- GitHub - Pull request detail reference
- shadcn/ui Dialog - https://ui.shadcn.com/docs/components/dialog
- Lucide icons - https://lucide.dev/icons/
