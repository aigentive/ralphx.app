# Execution Control Bar

The Execution Control Bar is a fixed-position control panel at the bottom of the Kanban view that provides real-time execution status and controls for starting, pausing, resuming, and stopping task execution. It displays running/queued task counts and provides an animated status indicator.

**Design Inspiration:**
- Vercel Dashboard deployment controls (minimal, status-forward design)
- Linear's execution feedback (subtle pulsing indicators, clean typography)
- Raycast command palette footer (fixed positioning, secondary actions)
- Spotify's playback controls (iconic play/pause/stop, clear state communication)

**Aesthetic Direction:** Mission control elegance. The bar should feel like a refined command center - providing critical execution feedback at a glance without demanding attention. Status is communicated through color and motion (pulsing dot), while controls are immediately accessible but visually subordinate to the status information. The warm orange accent appears only for active execution states.

---

## Layout Structure

### Bar Container

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ● Running: 2/4  •  Queued: 7  │  task-name...           │  ⏸ Pause │ ⏹ Stop │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Container Properties

| Property | Value | Notes |
|----------|-------|-------|
| Position | Fixed at bottom of Kanban view | Sticky within content area |
| Height | 48px | Comfortable touch target |
| Width | 100% of parent container | Full-width bar |
| Background | `--bg-surface` | Elevated from board background |
| Border top | 1px `--border-subtle` | Subtle separation from content |
| Box shadow | `0 -2px 8px rgba(0,0,0,0.15)` | Lifted appearance |
| Padding | 0 16px | Consistent with page margins |
| Display | Flex, align-items center, justify-content space-between |
| Z-index | 10 | Above board content, below modals |

---

## Status Section (Left)

### Status Container

| Property | Value |
|----------|-------|
| Display | Flex, align-items center |
| Gap | 16px between groups |

### Status Indicator (Animated Dot)

The status indicator is a small circular dot that communicates execution state through color and animation.

| State | Color | Animation |
|-------|-------|-----------|
| Running | `--status-success` (#10b981) | Pulsing glow (scale 1→1.3→1, opacity 1→0.7→1) |
| Paused | `--status-warning` (#f59e0b) | Static (no animation) |
| Stopped/Idle | `--text-muted` (#666666) | Static (no animation) |

#### Indicator Styling

| Property | Value |
|----------|-------|
| Size | 8px × 8px |
| Border radius | `--radius-full` (circle) |
| Transition | background-color 200ms ease |

#### Pulse Animation (Running State)

```css
@keyframes status-pulse {
  0%, 100% {
    transform: scale(1);
    box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.4);
  }
  50% {
    transform: scale(1.15);
    box-shadow: 0 0 0 4px rgba(16, 185, 129, 0);
  }
}

.status-indicator.running {
  animation: status-pulse 2s ease-in-out infinite;
}
```

### Running Count Display

| Property | Value |
|----------|-------|
| Font | `text-sm` (14px), `font-medium` (500) |
| Color | `--text-primary` |
| Format | "Running: X/Y" where X is current, Y is max concurrent |
| Letter spacing | `--tracking-normal` |

```tsx
<span className="text-sm font-medium text-[var(--text-primary)]">
  Running: {runningCount}/{maxConcurrent}
</span>
```

### Separator

| Property | Value |
|----------|-------|
| Content | "•" (bullet) |
| Color | `--text-muted` |
| Margin | 0 (handled by gap) |

### Queued Count Display

| Property | Value |
|----------|-------|
| Font | `text-sm` (14px), `font-normal` (400) |
| Color | `--text-secondary` |
| Format | "Queued: X" |

```tsx
<span className="text-sm text-[var(--text-secondary)]">
  Queued: {queuedCount}
</span>
```

---

## Progress Section (Center)

### Current Task Display

Shows the name of the currently executing task (optional, shown when tasks are running).

| Property | Value |
|----------|-------|
| Display | Flex, align-items center |
| Position | Center of bar |
| Max width | 40% of bar width |
| Visibility | Hidden when no tasks running |

### Task Name

| Property | Value |
|----------|-------|
| Font | `text-sm` (14px), `font-normal` |
| Color | `--text-secondary` |
| Truncation | Single line, ellipsis |
| Max width | 100% of container |
| Prefix | Lucide `Loader2` icon (16px, animated spin) when running |

```tsx
{runningCount > 0 && currentTaskName && (
  <div className="flex items-center gap-2 max-w-[40%]">
    <Loader2 className="w-4 h-4 animate-spin text-[var(--accent-primary)]" />
    <span className="text-sm text-[var(--text-secondary)] truncate">
      {currentTaskName}
    </span>
  </div>
)}
```

### Progress Indicator (Optional Enhancement)

When determinable progress is available:

| Property | Value |
|----------|-------|
| Type | Thin progress bar |
| Height | 2px |
| Background | `--bg-hover` (track) |
| Fill | `--accent-primary` (progress) |
| Border radius | `--radius-full` |
| Width | 120px |
| Position | Inline with task name |
| Animation | 300ms ease-out on progress change |

---

## Control Section (Right)

### Control Container

| Property | Value |
|----------|-------|
| Display | Flex, align-items center |
| Gap | 8px between buttons |

### Pause/Resume Button

Using **shadcn Button** with ghost variant and custom styling.

#### Default State (Running → Show Pause)

| Property | Value |
|----------|-------|
| Icon | Lucide `Pause` (18px) |
| Label | "Pause" |
| Variant | Ghost |
| Size | Default (36px height) |
| Padding | 12px 16px |
| Color | `--text-primary` |
| Background | transparent |
| Border | 1px `--border-default` |
| Border radius | `--radius-md` (8px) |

#### Paused State (Show Resume)

| Property | Value |
|----------|-------|
| Icon | Lucide `Play` (18px) |
| Label | "Resume" |
| Background | `--accent-muted` (subtle orange tint) |
| Border | 1px `--accent-primary` at 30% opacity |
| Color | `--accent-primary` |

#### Hover States

| State | Background | Border |
|-------|------------|--------|
| Default hover | `--bg-hover` | `--border-default` |
| Paused hover | Increase `--accent-muted` intensity | `--accent-primary` at 50% |

#### Disabled/Loading State

| Property | Value |
|----------|-------|
| Opacity | 0.5 |
| Cursor | not-allowed |
| Icon | Replace with `Loader2` (spinning) |

```tsx
<Button
  variant="ghost"
  size="default"
  onClick={onPauseToggle}
  disabled={isLoading}
  className={cn(
    "gap-2 border",
    isPaused
      ? "bg-[var(--accent-muted)] border-[var(--accent-primary)]/30 text-[var(--accent-primary)]"
      : "border-[var(--border-default)] text-[var(--text-primary)]"
  )}
>
  {isLoading ? (
    <Loader2 className="w-[18px] h-[18px] animate-spin" />
  ) : isPaused ? (
    <Play className="w-[18px] h-[18px]" />
  ) : (
    <Pause className="w-[18px] h-[18px]" />
  )}
  {isPaused ? "Resume" : "Pause"}
</Button>
```

### Stop Button

Using **shadcn Button** with custom destructive styling.

#### Enabled State (Tasks Running)

| Property | Value |
|----------|-------|
| Icon | Lucide `Square` (16px, filled appearance) |
| Label | "Stop" |
| Variant | Custom (destructive-secondary) |
| Background | `rgba(239, 68, 68, 0.15)` (error at 15%) |
| Border | 1px `--status-error` at 30% opacity |
| Color | `--status-error` |
| Hover background | `rgba(239, 68, 68, 0.25)` |
| Hover border | `--status-error` at 50% opacity |

#### Disabled State (No Tasks Running)

| Property | Value |
|----------|-------|
| Background | `--bg-hover` |
| Border | 1px `--border-subtle` |
| Color | `--text-muted` |
| Opacity | 0.5 |
| Cursor | not-allowed |

```tsx
<Button
  variant="ghost"
  size="default"
  onClick={onStop}
  disabled={!canStop || isLoading}
  className={cn(
    "gap-2 border",
    canStop
      ? "bg-[rgba(239,68,68,0.15)] border-[var(--status-error)]/30 text-[var(--status-error)] hover:bg-[rgba(239,68,68,0.25)] hover:border-[var(--status-error)]/50"
      : "bg-[var(--bg-hover)] border-[var(--border-subtle)] text-[var(--text-muted)] opacity-50"
  )}
>
  <Square className="w-4 h-4 fill-current" />
  Stop
</Button>
```

### Tooltips

Both buttons should have **shadcn Tooltip** for additional context.

| Button | Tooltip Content |
|--------|-----------------|
| Pause | "Pause execution (tasks in progress will complete)" |
| Resume | "Resume execution from queue" |
| Stop | "Stop all running tasks immediately" |
| Stop (disabled) | "No tasks currently running" |

---

## State Variations

### Idle State (No Tasks)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ○ Running: 0/4  •  Queued: 0                           │  ⏸ Pause │ ⏹ Stop │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Status dot | `--text-muted`, no animation |
| Pause button | Enabled but grayed, no immediate effect |
| Stop button | Disabled |

### Running State

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ● Running: 2/4  •  Queued: 7  │  ◌ Implementing auth...   │  ⏸ Pause │ ⏹ Stop │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Status dot | `--status-success`, pulsing animation |
| Task name | Shown with spinning loader |
| Pause button | Enabled, default styling |
| Stop button | Enabled, destructive styling |

### Paused State

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ● Running: 0/4  •  Queued: 7                           │  ▶ Resume │ ⏹ Stop │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Status dot | `--status-warning`, no animation |
| Pause button | Shows "Resume" with Play icon, accent styling |
| Stop button | Disabled (no running tasks) |

### Loading State (Action in Progress)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ● Running: 2/4  •  Queued: 7  │  ◌ Implementing auth...   │  ◌ Pausing... │ ⏹ Stop │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Active button | Shows `Loader2` spinner, disabled |
| Other buttons | Disabled during action |

---

## Responsive Behavior

### Compact Mode (< 640px)

| Change | Value |
|--------|-------|
| Labels | Hide button labels, show icons only |
| Task name | Hide completely |
| Queued count | Move to tooltip on running count |
| Button size | Reduce to icon-only (32px × 32px) |

```tsx
<Button variant="ghost" size="icon" className="w-8 h-8">
  <Pause className="w-4 h-4" />
</Button>
```

### Standard Mode (>= 640px)

Full layout as specified above.

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Cmd + P` | Toggle pause/resume |
| `Cmd + Shift + S` | Stop all tasks (with confirmation) |

Shortcuts should be shown in tooltips:
- Pause tooltip: "Pause execution ⌘P"
- Stop tooltip: "Stop all ⌘⇧S"

---

## Micro-interactions

### Status Dot Transition

```css
.status-indicator {
  transition: background-color 200ms ease, transform 200ms ease;
}
```

### Button Press Feedback

```css
.control-button:active {
  transform: scale(0.96);
}
```

### Hover Transitions

```css
.control-button {
  transition: background-color 150ms ease, border-color 150ms ease, transform 150ms ease;
}
```

### Task Name Reveal

When a task starts running, the task name slides in from right:

```css
@keyframes slide-in {
  from {
    opacity: 0;
    transform: translateX(8px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.task-name-container {
  animation: slide-in 200ms ease-out;
}
```

---

## Lucide Icons Used

| Icon | Usage | Size |
|------|-------|------|
| `Pause` | Pause button (running state) | 18px |
| `Play` | Resume button (paused state) | 18px |
| `Square` | Stop button (filled style) | 16px |
| `Loader2` | Loading spinner, task progress | 16-18px |

---

## Component Hierarchy

```
ExecutionControlBar
├── BarContainer (fixed positioning, shadow)
│   │
│   ├── StatusSection (left)
│   │   ├── StatusIndicator
│   │   │   └── AnimatedDot (conditional pulse)
│   │   ├── RunningCount
│   │   │   └── "Running: X/Y"
│   │   ├── Separator ("•")
│   │   └── QueuedCount
│   │       └── "Queued: X"
│   │
│   ├── ProgressSection (center, conditional)
│   │   ├── Loader2 (spinning icon)
│   │   ├── TaskName (truncated)
│   │   └── ProgressBar (optional)
│   │
│   └── ControlSection (right)
│       ├── Tooltip (shadcn)
│       │   └── PauseResumeButton (shadcn Button)
│       │       ├── Pause/Play icon
│       │       └── Label text
│       │
│       └── Tooltip (shadcn)
│           └── StopButton (shadcn Button)
│               ├── Square icon
│               └── Label text
```

---

## Acceptance Criteria

### Functional Requirements

1. [ ] Bar is fixed at bottom of Kanban view
2. [ ] Bar has subtle top border and shadow for elevation
3. [ ] Status indicator displays correct color per state (green/amber/gray)
4. [ ] Status indicator pulses when tasks are running
5. [ ] Running count shows "X/Y" format (current/max concurrent)
6. [ ] Queued count updates in real-time
7. [ ] Current task name displays when tasks are running
8. [ ] Task name truncates with ellipsis for long names
9. [ ] Pause button toggles to Resume when paused
10. [ ] Pause button shows accent styling when in paused state
11. [ ] Stop button is disabled when no tasks are running
12. [ ] Stop button has destructive (red) styling when enabled
13. [ ] Both buttons show loading state during action
14. [ ] Tooltips appear on button hover
15. [ ] Keyboard shortcuts work (Cmd+P for pause, Cmd+Shift+S for stop)
16. [ ] Responsive layout hides labels on small screens
17. [ ] All state transitions have smooth animations

---

## Design Quality Checklist

### Colors & Theming

1. [ ] NO purple or blue gradients anywhere
2. [ ] Background uses `--bg-surface` (elevated from board)
3. [ ] Status indicator uses semantic colors (green running, amber paused, gray idle)
4. [ ] Warm orange accent (`--accent-primary`) only for paused state button
5. [ ] Stop button uses `--status-error` with low opacity background
6. [ ] Text colors follow hierarchy (primary, secondary, muted)

### Typography

7. [ ] All text uses SF Pro (`--font-body`)
8. [ ] Running count: `text-sm`, `font-medium`
9. [ ] Queued count: `text-sm`, `font-normal`
10. [ ] Button labels: `text-sm`, `font-medium`
11. [ ] Task name: `text-sm`, `font-normal`

### Spacing & Layout

12. [ ] Bar height: 48px
13. [ ] Horizontal padding: 16px
14. [ ] Gap between status elements: 16px
15. [ ] Gap between buttons: 8px
16. [ ] Status indicator size: 8px
17. [ ] Button height: 36px
18. [ ] 8pt grid alignment maintained throughout

### Shadows & Depth

19. [ ] Bar has top shadow for floating effect
20. [ ] No excessive shadows on buttons (border-based definition)
21. [ ] Focus states use `--shadow-glow`

### Borders & Radius

22. [ ] Bar has 1px top border (`--border-subtle`)
23. [ ] Button border radius: `--radius-md` (8px)
24. [ ] Buttons have 1px border for definition
25. [ ] Status indicator: `--radius-full`

### Motion & Interactions

26. [ ] Status dot pulse animation: 2s ease-in-out infinite
27. [ ] Button hover transitions: 150ms ease
28. [ ] Button press: scale(0.96)
29. [ ] Loading spinner: CSS rotate animation
30. [ ] State transitions: 200ms ease
31. [ ] Task name slide-in: 200ms ease-out

### Icons

32. [ ] All icons from Lucide library
33. [ ] Pause/Play icons: 18px
34. [ ] Stop icon: 16px (filled style)
35. [ ] Loader icon: 16-18px
36. [ ] Icons inherit color from parent text

### Accessibility

37. [ ] Color contrast meets WCAG AA (4.5:1)
38. [ ] Focus states visible on all buttons
39. [ ] Buttons have aria-label for screen readers
40. [ ] Keyboard navigation works (Tab between buttons)
41. [ ] Disabled buttons have aria-disabled
42. [ ] Status communicated via aria-live region

---

## Implementation Notes

### shadcn Components to Use

- `Button` (pause/resume and stop buttons)
- `Tooltip`, `TooltipTrigger`, `TooltipContent` (button hints)

### CSS Custom Properties

```css
/* Execution Control specific */
--execution-bar-height: 48px;
--status-dot-size: 8px;

/* Reused from DESIGN.md */
--bg-surface: #1a1a1a;
--bg-hover: #2d2d2d;
--text-primary: #f0f0f0;
--text-secondary: #a0a0a0;
--text-muted: #666666;
--accent-primary: #ff6b35;
--accent-muted: rgba(255, 107, 53, 0.15);
--status-success: #10b981;
--status-warning: #f59e0b;
--status-error: #ef4444;
--border-subtle: rgba(255, 255, 255, 0.06);
--border-default: rgba(255, 255, 255, 0.1);
--radius-md: 8px;
--radius-full: 9999px;
--shadow-glow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
```

### Animation Keyframes

```css
@keyframes status-pulse {
  0%, 100% {
    transform: scale(1);
    box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.4);
  }
  50% {
    transform: scale(1.15);
    box-shadow: 0 0 0 4px rgba(16, 185, 129, 0);
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes slide-in-right {
  from {
    opacity: 0;
    transform: translateX(8px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}
```

### State Management

The component receives state from parent via props:
- `runningCount`, `maxConcurrent`, `queuedCount` - execution counts
- `isPaused` - pause state
- `isLoading` - action in progress
- `currentTaskName` - optional, for progress display
- `onPauseToggle`, `onStop` - action handlers

### Accessibility Enhancements

```tsx
<div
  role="region"
  aria-label="Execution controls"
  aria-live="polite"
>
  <div aria-label={`${runningCount} tasks running out of ${maxConcurrent}, ${queuedCount} queued`}>
    {/* Status content */}
  </div>

  <Button
    aria-label={isPaused ? "Resume execution" : "Pause execution"}
    aria-pressed={isPaused}
  >
    {/* Button content */}
  </Button>

  <Button
    aria-label="Stop all running tasks"
    aria-disabled={!canStop}
  >
    {/* Button content */}
  </Button>
</div>
```

---

## References

- [DESIGN.md](../DESIGN.md) - Master design system
- [specs/DESIGN_OVERHAUL_PLAN.md](../../DESIGN_OVERHAUL_PLAN.md) - Design overhaul strategy
- [kanban-board.md](./kanban-board.md) - Parent view for execution bar
- Vercel Dashboard - Deployment status and controls
- Linear - Execution feedback patterns
- shadcn/ui Button - https://ui.shadcn.com/docs/components/button
- shadcn/ui Tooltip - https://ui.shadcn.com/docs/components/tooltip
- Lucide icons - https://lucide.dev/icons/
