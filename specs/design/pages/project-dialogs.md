# Project Dialogs

The Project Dialogs family handles all project-related workflows in RalphX: creating new projects, completing merge workflows for worktree mode, and re-running completed tasks. These dialogs share a common modal foundation while adapting their content and actions to each specific workflow.

**Design Inspiration:**
- Linear's project creation (clean wizard flow, progressive disclosure)
- GitHub's merge dialog (clear options, warning states for destructive actions)
- Raycast's settings dialogs (radio groups with descriptions, Mac-native feel)
- Vercel's deployment dialogs (completion summaries, next-step guidance)

**Aesthetic Direction:** Professional utility with clarity. Dialogs should feel lightweight yet substantial, with clear visual hierarchy guiding users through decisions. Radio options need sufficient breathing room, and destructive options must be clearly marked without being alarming.

---

## Common Modal Patterns

All project dialogs share these foundational patterns to ensure consistency across the application.

### Dialog Container

Using **shadcn Dialog** with custom sizing.

```tsx
<Dialog>
  <DialogContent className="max-w-lg">
    {/* content */}
  </DialogContent>
</Dialog>
```

| Property | Value | Notes |
|----------|-------|-------|
| Max width | 512px (max-w-lg) | Comfortable for forms without feeling cramped |
| Background | `--bg-surface` | Middle elevation level |
| Border | 1px `--border-subtle` | Subtle definition |
| Border radius | `--radius-xl` (16px) | Premium rounded feel |
| Shadow | `--shadow-lg` | Strong floating effect |
| Z-index | 50 | Above all content |

### Backdrop

```css
.backdrop {
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(8px);
}
```

| Property | Value |
|----------|-------|
| Background | `rgba(0, 0, 0, 0.5)` |
| Blur | `backdrop-filter: blur(8px)` |
| Click behavior | Closes dialog (except when destructive action pending) |
| Animation | Fade in, 200ms |

### Open Animation

```css
@keyframes dialog-enter {
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
  animation: dialog-enter 200ms ease-out;
}
```

### Close Animation

```css
@keyframes dialog-exit {
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

### Header Pattern

```
┌──────────────────────────────────────────────────────────────┐
│  [Icon]  Dialog Title                                    [✕] │
└──────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Container | `px-6 py-4`, border-bottom 1px `--border-subtle` |
| Icon | 20px, `--accent-primary` or status color |
| Title | `text-lg`, `font-semibold`, `--text-primary` |
| Close button | 32px hit area, 16px icon, `--text-muted` → `--text-primary` on hover |

### Footer Pattern

```
┌──────────────────────────────────────────────────────────────┐
│                                      [Cancel]  [Primary]     │
└──────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Container | `px-6 py-4`, border-top 1px `--border-subtle` |
| Alignment | `justify-end`, gap 12px |
| Cancel button | Ghost variant, `--bg-elevated`, `--text-primary` |
| Primary button | Accent variant, `--accent-primary`, white text |
| Button height | 36px |
| Button padding | `px-4 py-2` |
| Font | `text-sm`, `font-medium` |

### Close Button

| Property | Value |
|----------|-------|
| Position | Absolute top-right (top-4, right-4) |
| Icon | Lucide `X` (16px) |
| Size | 32px × 32px |
| Background | transparent |
| Hover | `--bg-hover` |
| Border radius | `--radius-md` (8px) |
| Color | `--text-muted` → `--text-primary` on hover |
| Focus | `--shadow-glow` |
| Transition | 150ms |

---

## Project Creation Wizard

The Project Creation Wizard guides users through creating a new project with Git mode selection.

### Purpose

Allow users to:
1. Select a working directory (primary action)
2. Optionally name their project (auto-inferred from folder name)
3. Choose between Local and Worktree git modes
4. Configure worktree-specific options (branch name, base branch)

### First-Run Experience

When no projects exist (empty project list), the Project Creation Wizard is shown as a **full-screen centered modal** instead of opening from a button:

| Property | Value |
|----------|-------|
| Trigger | Automatic on app launch when `projects.length === 0` |
| Positioning | Centered on screen (both axes) |
| Backdrop | Full screen with blur, no app chrome visible behind |
| Dismissal | Only via creating a project (no close button, no escape key) |
| Animation | Fade in from center (no translateY) |

This ensures users immediately understand they need to set up a project before using RalphX.

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│  Create New Project                                      [✕] │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Location *                                                  │
│  ┌────────────────────────────────────────────┐ ┌─────────┐ │
│  │ /Users/dev/my-app                          │ │ Browse  │ │
│  └────────────────────────────────────────────┘ └─────────┘ │
│                                                              │
│  Project Name (optional)                                     │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ my-app                                     [auto-fill] │   │
│  └──────────────────────────────────────────────────────┘   │
│  Inferred from folder name. Override if desired.            │
│                                                              │
│  ─────────────────────────────────────────────────────────  │
│                                                              │
│  Git Mode                                                    │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ◉ Local (default)                                     │   │
│  │   Work directly in your current branch                │   │
│  │   ⚠ Your uncommitted changes may be affected         │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ○ Isolated Worktree (recommended when actively coding)│   │
│  │   Creates separate worktree for RalphX to work in     │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                                      [Cancel]  [Create Project] │
└──────────────────────────────────────────────────────────────┘
```

**Note:** The `[✕]` close button is hidden when shown as first-run experience (no projects exist).

### Content Area

| Property | Value |
|----------|-------|
| Padding | `px-6 py-5` |
| Gap between sections | 20px (`space-y-5`) |

### Input Fields

Fields are displayed in this order:
1. **Location** (required) - folder picker with Browse button
2. **Project Name** (optional) - auto-inferred from folder name

#### Label

| Property | Value |
|----------|-------|
| Font | `text-sm`, `font-medium` |
| Color | `--text-secondary` |
| Margin bottom | 6px (`space-y-1.5`) |
| Required indicator | `*` suffix for required fields |
| Optional indicator | "(optional)" suffix in `--text-muted` |

#### Location Field (First, Required)

| Property | Value |
|----------|-------|
| Label | "Location *" |
| Placeholder | "Select a folder..." |
| Read-only | Yes (populated via Browse button only) |
| Validation | Required - shows error if empty on submit |

#### Input (using shadcn Input)

| Property | Value |
|----------|-------|
| Height | 40px |
| Padding | `px-3 py-2` |
| Font | `text-sm` |
| Background | `--bg-base` |
| Border | 1px `--border-subtle` |
| Border radius | `--radius-lg` (12px) |
| Color | `--text-primary` |
| Placeholder color | `--text-muted` |
| Focus border | `--accent-primary` |
| Focus ring | `--shadow-glow` |
| Error border | `--status-error` |
| Disabled opacity | 0.5 |

#### Project Name Field (Second, Optional)

| Property | Value |
|----------|-------|
| Label | "Project Name (optional)" |
| Auto-inference | When location is selected, extract folder name (e.g., `/Users/dev/my-app` → `my-app`) |
| Placeholder | Shows inferred name in `--text-muted` when empty |
| Editable | Yes - user can override the inferred name |
| Helper text | "Inferred from folder name. Override if desired." |
| Helper text style | `text-xs`, `--text-muted`, 4px margin top |

#### Auto-Inference Behavior

When user selects a folder via Browse:
1. Extract the last path segment (folder name)
2. If Project Name field is empty OR matches previous inferred value, update it
3. If user has manually typed a custom name, do NOT override
4. Track `isNameManuallySet` state to determine override behavior

#### Error Message

| Property | Value |
|----------|-------|
| Font | `text-xs` |
| Color | `--status-error` |
| Margin top | 6px |

#### Browse Button

| Property | Value |
|----------|-------|
| Icon | Lucide `FolderOpen` (16px) |
| Background | `--bg-elevated` |
| Color | `--text-primary` |
| Padding | `px-3 py-2` |
| Border radius | `--radius-lg` (12px) |
| Hover | `--bg-hover` |
| Gap icon-to-text | 8px |
| Height | Match input (40px) |

### Divider

| Property | Value |
|----------|-------|
| Height | 1px |
| Color | `--border-subtle` |
| Margin | 20px vertical (handled by `space-y-5`) |

### Git Mode Radio Group

#### Section Label

| Property | Value |
|----------|-------|
| Font | `text-sm`, `font-medium` |
| Color | `--text-secondary` |
| Margin bottom | 12px |

#### Radio Option Card

```tsx
<label
  className={cn(
    "flex gap-3 p-3 rounded-lg cursor-pointer transition-colors",
    selected ? "bg-[var(--bg-elevated)]" : "bg-transparent"
  )}
  style={{
    border: `1px solid ${selected ? 'var(--accent-primary)' : 'var(--border-subtle)'}`,
  }}
>
  {/* radio + content */}
</label>
```

| Property | Default State | Selected State |
|----------|---------------|----------------|
| Background | transparent | `--bg-elevated` |
| Border | 1px `--border-subtle` | 1px `--accent-primary` |
| Border radius | `--radius-lg` (12px) | `--radius-lg` (12px) |
| Padding | 12px | 12px |
| Transition | 150ms | 150ms |

#### Radio Indicator

| Property | Value |
|----------|-------|
| Size | 16px outer |
| Border | 2px, `--border-subtle` (unselected) or `--accent-primary` (selected) |
| Inner dot | 8px, `--accent-primary` (only when selected) |
| Position | Top-aligned with first line of text |
| Flex shrink | 0 |

#### Option Label

| Property | Value |
|----------|-------|
| Font | `text-sm`, `font-medium` |
| Color | `--text-primary` |

#### Option Description

| Property | Value |
|----------|-------|
| Font | `text-xs` |
| Color | `--text-muted` |
| Margin top | 2px |

#### Warning Indicator

| Property | Value |
|----------|-------|
| Icon | Lucide `AlertTriangle` (14px) |
| Color | `--status-warning` |
| Font | `text-xs` |
| Gap icon-to-text | 6px |
| Margin top | 6px |
| Display | Only on Local option |

### Worktree-Specific Fields

When Worktree mode is selected, additional fields appear with animation.

#### Nested Fields Container

| Property | Value |
|----------|-------|
| Margin top | 12px |
| Gap between fields | 12px (`space-y-3`) |
| Animation | Slide down + fade in, 200ms |

#### Branch Name Input

Same styling as main input fields.

| Property | Value |
|----------|-------|
| Label | "Branch name" |
| Placeholder | `ralphx/feature-name` |
| Auto-generated | From project name: `ralphx/{slug}` |

#### Base Branch Select

Using **shadcn Select**.

| Property | Value |
|----------|-------|
| Label | "Base branch" |
| Options | Fetched from git repository |
| Default | "main" or "master" if available |
| Loading state | "Loading branches..." with disabled state |
| Empty state | "No branches available" |
| Chevron icon | Lucide `ChevronDown` (12px) |

#### Worktree Path Display

| Property | Value |
|----------|-------|
| Background | `--bg-base` |
| Border radius | `--radius-lg` (12px) |
| Padding | `px-3 py-2` |
| Icon | Lucide `GitBranch` (14px), `--text-muted` |
| Title | "Worktree location", `text-xs`, `font-medium`, `--text-muted` |
| Path | `text-sm`, `--text-primary`, truncated |
| Format | `~/ralphx-worktrees/{folder-name}` |

### Error Banner

| Property | Value |
|----------|-------|
| Background | `rgba(239, 68, 68, 0.1)` |
| Border radius | `--radius-lg` (12px) |
| Padding | `px-3 py-2` |
| Icon | Lucide `AlertTriangle` (14px) |
| Color | `--status-error` |
| Font | `text-sm` |
| Gap icon-to-text | 8px |

### Footer Buttons

| Button | Variant | State Handling |
|--------|---------|----------------|
| Cancel | Ghost (`--bg-elevated`) | Disabled when creating |
| Create Project | Primary (`--accent-primary`) | Disabled when creating or validation errors |
| Creating... | Primary (disabled) | Shows when `isCreating` |

---

## Merge Workflow Dialog

The Merge Workflow Dialog appears when a project completes in worktree mode, offering options for handling the completed work.

### Purpose

Present users with options to:
1. Merge changes to main branch
2. Rebase onto main for linear history
3. Create a Pull Request for review
4. Keep the worktree for manual handling
5. Discard all changes (destructive)

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│  [✓]  Project Complete: My Project                       [✕] │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  RalphX made 5 commits on branch: ralphx/feature-name       │
│                                                              │
│  [View Diff]  [View Commits]                                 │
│                                                              │
│  ─────────────────────────────────────────────────────────  │
│                                                              │
│  What would you like to do?                                  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ◉ [🔀] Merge to main                                  │   │
│  │        Creates a merge commit preserving branch history│   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ○ [⊕] Rebase onto main                               │   │
│  │       Replays commits on top of main for linear history│  │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ○ [↗] Create Pull Request                             │   │
│  │       Opens GitHub to create a PR for code review     │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ○ [⊞] Keep worktree                                   │   │
│  │       Leave as-is and merge manually later            │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ○ [🗑] Discard changes                                 │   │
│  │       Delete the worktree and branch permanently      │   │
│  │       ⚠ This cannot be undone. All commits lost.     │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                                      [Cancel]  [Continue]    │
└──────────────────────────────────────────────────────────────┘
```

### Header

| Element | Styling |
|---------|---------|
| Icon | Lucide `CheckCircle` (20px), `--status-success` |
| Title format | "Project Complete: {project.name}" |
| Title font | `text-lg`, `font-semibold`, `--text-primary` |
| Gap icon-to-title | 12px |

### Completion Summary

| Property | Value |
|----------|-------|
| Font | `text-sm`, `--text-secondary` |
| Commit count | `font-medium`, `--text-primary` |
| Branch name | `font-mono`, `font-medium`, `--accent-primary` |
| Format | "RalphX made N commit(s) on branch: {branch}" |

### Action Buttons Row

| Property | Value |
|----------|-------|
| Layout | Flex row, gap 8px |
| Margin top | 16px |

#### View Diff Button

| Property | Value |
|----------|-------|
| Icon | Lucide `FileDiff` or `Diff` (14px) |
| Background | `--bg-elevated` |
| Color | `--text-primary` |
| Padding | `px-3 py-2` |
| Font | `text-sm`, `font-medium` |
| Border radius | `--radius-lg` (12px) |
| Hover | `--bg-hover` |
| Gap icon-to-text | 8px |

#### View Commits Button

Same styling as View Diff.

| Property | Value |
|----------|-------|
| Icon | Lucide `GitCommit` (14px) |

### Question Label

| Property | Value |
|----------|-------|
| Font | `text-sm`, `font-medium` |
| Color | `--text-secondary` |
| Margin | 20px top (after divider), 12px bottom |

### Merge Option Cards

Each option follows the radio card pattern with icons.

| Option | Icon | Label | Description |
|--------|------|-------|-------------|
| merge | `GitMerge` (16px) | "Merge to main" | "Creates a merge commit preserving branch history" |
| rebase | Custom rebase icon (16px) | "Rebase onto main" | "Replays commits on top of main for linear history" |
| create_pr | `GitPullRequest` (16px) | "Create Pull Request" | "Opens GitHub to create a PR for code review" |
| keep_worktree | Custom worktree icon (16px) | "Keep worktree" | "Leave as-is and merge manually later" |
| discard | `Trash2` (16px) | "Discard changes" | "Delete the worktree and branch permanently" |

#### Standard Option Card

| Property | Default State | Selected State |
|----------|---------------|----------------|
| Background | transparent | `--bg-elevated` |
| Border | 1px `--border-subtle` | 1px `--accent-primary` |
| Icon color | `--text-primary` | `--text-primary` |

#### Destructive Option Card (Discard)

| Property | Default State | Selected State |
|----------|---------------|----------------|
| Background | transparent | `--bg-elevated` |
| Border | 1px `--border-subtle` | 1px `--status-error` |
| Icon color | `--status-error` | `--status-error` |
| Label color | `--status-error` | `--status-error` |
| Radio indicator border | `--border-subtle` | `--status-error` |
| Radio indicator dot | N/A | `--status-error` |

#### Destructive Warning

| Property | Value |
|----------|-------|
| Display | Only when discard is selected |
| Icon | Lucide `AlertTriangle` (14px) |
| Color | `--status-warning` |
| Font | `text-xs` |
| Margin top | 8px |
| Text | "This cannot be undone. All commits will be lost." |

### Discard Confirmation Banner

When discard is selected and user clicks Continue.

| Property | Value |
|----------|-------|
| Background | `rgba(239, 68, 68, 0.1)` |
| Border radius | `--radius-lg` (12px) |
| Padding | `px-3 py-2` |
| Icon | Lucide `AlertTriangle` (14px) |
| Color | `--status-error` |
| Font | `text-sm` |
| Text | "Are you sure? Click 'Confirm Discard' to permanently delete the worktree and branch." |

### Footer Button States

| State | Primary Button Label | Primary Button Color |
|-------|---------------------|---------------------|
| Default | "Continue" | `--accent-primary` |
| Discard selected (not confirmed) | "Continue" | `--accent-primary` |
| Discard selected (confirmation shown) | "Confirm Discard" | `--status-error` |
| Processing | "Processing..." | `--bg-hover` (disabled) |

---

## Task Re-run Dialog

The Task Re-run Dialog appears when dragging a completed (Done) task back to Planned, offering options for handling the previous work.

### Purpose

Help users decide how to handle a task that was previously completed:
1. Keep changes and run again (AI iterates on current state)
2. Revert the commit before running (undo previous work)
3. Create a new task instead (keep original completed)

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│  [↻]  Re-run Task                                        [✕] │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  "Implement user authentication"                             │
│                                                              │
│  This task was completed with commit: abc123f                │
│  "Add login and registration endpoints"                      │
│                                                              │
│  ─────────────────────────────────────────────────────────  │
│                                                              │
│  How should we handle the previous work?                     │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ◉ [✓] Keep changes, run task again      [Recommended] │   │
│  │       AI will see current code state and make         │   │
│  │       additional changes if needed                    │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ○ [↩] Revert commit, then run task                   │   │
│  │       Undo the previous work before re-executing      │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ○ [+] Create new task instead                         │   │
│  │       Original task stays completed, new task created │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ ⚠ Warning: Other commits depend on this one. Reverting│ │
│  │   may cause conflicts or break code that was built on │ │
│  │   top of these changes.                               │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                                   [Cancel]  [Confirm Re-run] │
└──────────────────────────────────────────────────────────────┘
```

### Header

| Element | Styling |
|---------|---------|
| Icon | Lucide `RefreshCw` or `RotateCcw` (20px), `--accent-primary` |
| Title | "Re-run Task" |
| Title font | `text-lg`, `font-semibold`, `--text-primary` |

### Task Info Section

#### Task Title

| Property | Value |
|----------|-------|
| Font | `text-base`, `font-medium` |
| Color | `--text-primary` |
| Quotes | Wrapped in quotation marks |

#### Commit Info

| Property | Value |
|----------|-------|
| Font | `text-sm`, `--text-secondary` |
| SHA | `font-mono`, `font-medium`, `--accent-primary` |
| SHA format | Short (7 characters) |
| Message | `text-sm`, italic, `--text-muted` |
| Message quotes | Wrapped in quotation marks |

### Question Label

| Property | Value |
|----------|-------|
| Font | `text-sm`, `font-medium` |
| Color | `--text-secondary` |
| Text | "How should we handle the previous work?" |
| Margin | 20px top (after divider), 12px bottom |

### Re-run Option Cards

| Option | Icon | Label | Description | Special |
|--------|------|-------|-------------|---------|
| keep_changes | `Check` (16px) | "Keep changes, run task again" | "AI will see current code state and make additional changes if needed" | Recommended badge |
| revert_commit | `Undo` (16px) | "Revert commit, then run task" | "Undo the previous work before re-executing" | Warning state when selected + dependent commits |
| create_new | `Plus` (16px) | "Create new task instead" | "Original task stays completed, new task created" | N/A |

#### Recommended Badge

| Property | Value |
|----------|-------|
| Text | "Recommended" |
| Background | `rgba(255, 107, 53, 0.15)` |
| Color | `--accent-primary` |
| Font | `text-xs` |
| Padding | `px-1.5 py-0.5` |
| Border radius | `--radius-sm` (4px) |
| Position | Inline after label |

#### Warning Option Card (Revert when dependent commits exist)

| Property | Default State | Selected State |
|----------|---------------|----------------|
| Background | transparent | `--bg-elevated` |
| Border | 1px `--border-subtle` | 1px `--status-warning` |
| Icon color | `--text-primary` | `--status-warning` |
| Label color | `--text-primary` | `--status-warning` |
| Radio indicator border | `--border-subtle` | `--status-warning` |
| Radio indicator dot | N/A | `--status-warning` |

### Dependent Commits Warning Banner

Only displayed when:
1. Revert option is selected AND
2. `commitInfo.hasDependentCommits` is true

| Property | Value |
|----------|-------|
| Background | `rgba(245, 158, 11, 0.1)` |
| Border radius | `--radius-lg` (12px) |
| Padding | `px-3 py-2.5` |
| Icon | Lucide `AlertTriangle` (14px), flex-shrink-0 |
| Color | `--status-warning` |
| Font | `text-sm` |
| Text | "Warning: Other commits depend on this one. Reverting may cause conflicts or break code that was built on top of these changes." |
| Icon alignment | Top-aligned with text |
| Gap icon-to-text | 8px |

### Footer

| Button | Label |
|--------|-------|
| Cancel | "Cancel" |
| Primary | "Confirm Re-run" |
| Processing | "Processing..." |

---

## Lucide Icons Used

| Icon | Component | Usage | Size |
|------|-----------|-------|------|
| `X` | All dialogs | Close button | 16px |
| `FolderOpen` | ProjectCreationWizard | Browse button | 16px |
| `AlertTriangle` | All dialogs | Warning indicators | 14px |
| `ChevronDown` | ProjectCreationWizard | Select dropdown | 12px |
| `GitBranch` | ProjectCreationWizard | Worktree path display | 14px |
| `CheckCircle` | MergeWorkflowDialog | Completion header | 20px |
| `GitMerge` | MergeWorkflowDialog | Merge option | 16px |
| `GitPullRequest` | MergeWorkflowDialog | Create PR option | 16px |
| `Trash2` | MergeWorkflowDialog | Discard option | 16px |
| `FileDiff` | MergeWorkflowDialog | View Diff button | 14px |
| `GitCommit` | MergeWorkflowDialog | View Commits button | 14px |
| `RefreshCw` | TaskRerunDialog | Header icon | 20px |
| `Check` | TaskRerunDialog | Keep changes option | 16px |
| `Undo` | TaskRerunDialog | Revert option | 16px |
| `Plus` | TaskRerunDialog | Create new option | 16px |

---

## Component Hierarchy

```
ProjectCreationWizard
├── Dialog (shadcn)
│   ├── DialogOverlay (backdrop blur)
│   └── DialogContent
│       ├── Header
│       │   ├── Title ("Create New Project")
│       │   └── CloseButton (Lucide X) [hidden in first-run mode]
│       │
│       ├── ContentArea
│       │   ├── LocationField (FIRST - required)
│       │   │   ├── Label ("Location *")
│       │   │   ├── Input (shadcn, read-only)
│       │   │   └── BrowseButton
│       │   │       ├── FolderOpenIcon
│       │   │       └── "Browse"
│       │   │
│       │   ├── ProjectNameField (SECOND - optional)
│       │   │   ├── Label ("Project Name (optional)")
│       │   │   ├── Input (shadcn, auto-populated)
│       │   │   └── HelperText ("Inferred from folder name...")
│       │   │
│       │   ├── Divider
│       │   │
│       │   ├── GitModeSection
│       │   │   ├── SectionLabel ("Git Mode")
│       │   │   │
│       │   │   ├── RadioOption (Local)
│       │   │   │   ├── RadioIndicator
│       │   │   │   ├── Label + Description
│       │   │   │   └── WarningIndicator (AlertTriangle)
│       │   │   │
│       │   │   └── RadioOption (Worktree)
│       │   │       ├── RadioIndicator
│       │   │       ├── Label + Description
│       │   │       └── WorktreeFields (conditional)
│       │   │           ├── BranchNameInput
│       │   │           ├── BaseBranchSelect
│       │   │           └── WorktreePathDisplay
│       │   │               ├── GitBranchIcon
│       │   │               └── PathText
│       │   │
│       │   └── ErrorBanner (conditional)
│       │
│       └── Footer
│           ├── CancelButton [hidden in first-run mode]
│           └── CreateButton

MergeWorkflowDialog
├── Dialog (shadcn)
│   ├── DialogOverlay
│   └── DialogContent
│       ├── Header
│       │   ├── CheckCircleIcon
│       │   ├── Title ("Project Complete: {name}")
│       │   └── CloseButton
│       │
│       ├── ContentArea
│       │   ├── CompletionSummary
│       │   │   ├── CommitCount
│       │   │   └── BranchName
│       │   │
│       │   ├── ActionButtonsRow
│       │   │   ├── ViewDiffButton
│       │   │   └── ViewCommitsButton
│       │   │
│       │   ├── Divider
│       │   │
│       │   ├── QuestionLabel
│       │   │
│       │   ├── OptionsContainer
│       │   │   ├── MergeOptionCard
│       │   │   ├── RebaseOptionCard
│       │   │   ├── CreatePROptionCard
│       │   │   ├── KeepWorktreeOptionCard
│       │   │   └── DiscardOptionCard (destructive)
│       │   │       └── DestructiveWarning (conditional)
│       │   │
│       │   ├── DiscardConfirmationBanner (conditional)
│       │   │
│       │   └── ErrorBanner (conditional)
│       │
│       └── Footer
│           ├── CancelButton
│           └── ContinueButton / ConfirmDiscardButton

TaskRerunDialog
├── Dialog (shadcn)
│   ├── DialogOverlay
│   └── DialogContent
│       ├── Header
│       │   ├── RefreshIcon
│       │   ├── Title ("Re-run Task")
│       │   └── CloseButton
│       │
│       ├── ContentArea
│       │   ├── TaskInfoSection
│       │   │   ├── TaskTitle
│       │   │   └── CommitInfo
│       │   │       ├── CommitSHA
│       │   │       └── CommitMessage
│       │   │
│       │   ├── Divider
│       │   │
│       │   ├── QuestionLabel
│       │   │
│       │   ├── OptionsContainer
│       │   │   ├── KeepChangesOptionCard
│       │   │   │   └── RecommendedBadge
│       │   │   ├── RevertCommitOptionCard (warning state conditional)
│       │   │   └── CreateNewOptionCard
│       │   │
│       │   ├── DependentCommitsWarning (conditional)
│       │   │
│       │   └── ErrorBanner (conditional)
│       │
│       └── Footer
│           ├── CancelButton
│           └── ConfirmRerunButton
```

---

## Acceptance Criteria

### General Dialog Behavior

1. [ ] All dialogs open with scale + fade animation (0.95 → 1.0, 200ms)
2. [ ] All dialogs close on Escape key
3. [ ] All dialogs close on backdrop click (except during confirmation states)
4. [ ] Close button closes dialog with proper focus management
5. [ ] Focus is trapped within dialog while open
6. [ ] All interactive elements are keyboard accessible
7. [ ] Tab order follows visual order
8. [ ] ARIA attributes present: role="dialog", aria-modal="true", aria-labelledby
9. [ ] Close button has aria-label="Close"

### Project Creation Wizard

10. [ ] Browse button receives visual focus indicator on open (Location is primary action)
11. [ ] Location field is required, shows error message when empty on submit
12. [ ] Browse button opens system folder picker via Tauri dialog API
13. [ ] When folder is selected, Project Name auto-populates from folder name
14. [ ] Project Name is optional - uses inferred name if left empty
15. [ ] User can override inferred Project Name by typing
16. [ ] Overridden Project Name is preserved when Location changes (tracks `isNameManuallySet`)
17. [ ] Git Mode defaults to "Local"
18. [ ] Local mode shows warning about uncommitted changes
19. [ ] Worktree mode shows additional fields with slide animation
20. [ ] Branch name auto-generates from project name in format `ralphx/{slug}`
21. [ ] Base branch dropdown fetches branches from git repository
22. [ ] Base branch dropdown shows loading state while fetching
23. [ ] Base branch defaults to "main" or "master" if available
24. [ ] Worktree path displays generated path
25. [ ] Form validates Location field on submit (Project Name optional)
26. [ ] Create button shows loading state while creating
27. [ ] Error banner displays API errors
28. [ ] Form resets when dialog is reopened

### First-Run Experience (No Projects)

29. [ ] When projects list is empty, wizard shows immediately on app launch
30. [ ] Wizard is centered on screen (both horizontal and vertical axes)
31. [ ] Close button is hidden in first-run mode
32. [ ] Escape key does not close dialog in first-run mode
33. [ ] Backdrop click does not close dialog in first-run mode
34. [ ] Cancel button is hidden in first-run mode
35. [ ] Only way to dismiss is to successfully create a project
36. [ ] After first project created, normal dialog behavior resumes

### Merge Workflow Dialog

26. [ ] Header shows CheckCircle icon in success green
27. [ ] Completion summary shows correct commit count (singular/plural)
28. [ ] Completion summary shows branch name in monospace with accent color
29. [ ] View Diff button triggers onViewDiff callback
30. [ ] View Commits button triggers onViewCommits callback
31. [ ] Default selection is "merge" option
32. [ ] All 5 options are selectable
33. [ ] Discard option uses error/destructive styling
34. [ ] Discard option shows inline warning when selected
35. [ ] Clicking Continue with discard shows confirmation banner
36. [ ] Confirmation banner text warns about permanent deletion
37. [ ] Continue button changes to "Confirm Discard" after first click
38. [ ] Confirm Discard button uses error color
39. [ ] Selecting different option clears confirmation state
40. [ ] Processing state disables all interactions
41. [ ] Error banner displays workflow errors
42. [ ] Dialog resets to default state when reopened

### Task Re-run Dialog

43. [ ] Header shows RefreshCw icon in accent color
44. [ ] Task title displays in quotes with proper styling
45. [ ] Commit SHA shows in monospace with accent color
46. [ ] Commit message shows in italics
47. [ ] Default selection is "keep_changes" option
48. [ ] Keep changes option shows "Recommended" badge
49. [ ] Recommended badge uses muted accent styling
50. [ ] Revert option shows warning styling when selected AND hasDependentCommits is true
51. [ ] Dependent commits warning banner appears when revert selected + hasDependentCommits
52. [ ] Create new option has no special states
53. [ ] Processing state disables all interactions
54. [ ] Error banner displays re-run errors
55. [ ] Dialog resets to default state when reopened

---

## Design Quality Checklist

### Colors & Theming

1. [ ] NO purple or blue gradients anywhere
2. [ ] Dialog background uses `--bg-surface`
3. [ ] Input backgrounds use `--bg-base`
4. [ ] Selected radio cards use `--bg-elevated`
5. [ ] Accent color (`#ff6b35`) used only for:
   - Selected radio borders (non-destructive)
   - Focus rings
   - Branch names
   - Commit SHAs
   - Recommended badge
6. [ ] Warning states use `--status-warning` (#f59e0b)
7. [ ] Destructive states use `--status-error` (#ef4444)
8. [ ] Success states use `--status-success` (#10b981)

### Typography

9. [ ] All fonts use SF Pro (system font)
10. [ ] Dialog titles: `text-lg`, `font-semibold`, `--text-primary`
11. [ ] Section labels: `text-sm`, `font-medium`, `--text-secondary`
12. [ ] Input labels: `text-sm`, `font-medium`, `--text-secondary`
13. [ ] Option labels: `text-sm`, `font-medium`, `--text-primary`
14. [ ] Option descriptions: `text-xs`, `--text-muted`
15. [ ] Warning/error text: `text-xs` or `text-sm`
16. [ ] Monospace font for SHAs and branch names (JetBrains Mono)
17. [ ] Button text: `text-sm`, `font-medium`

### Spacing & Layout

18. [ ] Dialog padding: 24px horizontal (px-6)
19. [ ] Content padding: 20px vertical (py-5)
20. [ ] Section gap: 20px (space-y-5)
21. [ ] Radio option gap: 8px (space-y-2)
22. [ ] Radio card padding: 12px
23. [ ] 8pt grid alignment maintained throughout
24. [ ] Button gap: 12px (gap-3)
25. [ ] Icon-to-text gap: 8px (gap-2)
26. [ ] Label-to-input gap: 6px (space-y-1.5)

### Shadows & Depth

27. [ ] Dialog uses `--shadow-lg` for floating effect
28. [ ] Backdrop uses blur effect (8px)
29. [ ] Focus rings use `--shadow-glow`
30. [ ] No additional shadows on radio cards (flat with border)

### Borders & Radius

31. [ ] Dialog border radius: `--radius-xl` (16px)
32. [ ] Input border radius: `--radius-lg` (12px)
33. [ ] Radio card border radius: `--radius-lg` (12px)
34. [ ] Button border radius: `--radius-lg` (12px)
35. [ ] Badge border radius: `--radius-sm` (4px)
36. [ ] Banner border radius: `--radius-lg` (12px)
37. [ ] Radio indicator: fully round (rounded-full)

### Motion & Interactions

38. [ ] Dialog open animation: scale(0.95) → scale(1), 200ms ease-out
39. [ ] Dialog close animation: scale(1) → scale(0.95), 150ms ease-in
40. [ ] Worktree fields slide + fade animation on toggle
41. [ ] All hover states transition: 150ms
42. [ ] Radio card hover: background change to `--bg-hover` (when not selected)
43. [ ] Button hover: appropriate color shift
44. [ ] Close button hover: background to `--bg-hover`, color to `--text-primary`
45. [ ] Loading states use `cursor-not-allowed` and reduced opacity

### Icons

46. [ ] All icons from Lucide library
47. [ ] Header icons: 20px
48. [ ] Option icons: 16px
49. [ ] Inline icons (warning, folder, chevron): 14px or 12px
50. [ ] Close button icon: 16px
51. [ ] Icons use appropriate colors (inherit, status, accent)
52. [ ] Stroke width consistent (1.5-2px)

### Accessibility

53. [ ] Color contrast meets WCAG AA (4.5:1 for text)
54. [ ] Focus states visible on all interactive elements
55. [ ] Radio groups use proper input + label pattern
56. [ ] Hidden inputs for screen readers (sr-only class)
57. [ ] Error messages associated with inputs via aria-describedby
58. [ ] Loading states announced to screen readers
59. [ ] Disabled states have 50% opacity and no pointer events

---

## Implementation Notes

### shadcn Components to Use

- `Dialog`, `DialogContent`, `DialogOverlay`, `DialogHeader`, `DialogFooter`
- `Input` (for text inputs)
- `Select`, `SelectContent`, `SelectItem`, `SelectTrigger`, `SelectValue`
- `Button` (ghost and primary variants)
- `Label`

### Custom Components Needed

- `RadioOptionCard` - Reusable radio option with icon, label, description, warning
- `InputField` - Input with label and error message
- `SelectField` - Select with label and error message
- `WorktreePathDisplay` - Read-only display of generated worktree path

### CSS Custom Properties

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
--status-success: #10b981;
--status-warning: #f59e0b;
--status-error: #ef4444;
--radius-sm: 4px;
--radius-md: 8px;
--radius-lg: 12px;
--radius-xl: 16px;
--shadow-lg: 0 10px 15px rgba(0,0,0,0.3), 0 20px 40px rgba(0,0,0,0.25);
--shadow-glow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
```

### Animation Keyframes

```css
@keyframes dialog-enter {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

@keyframes dialog-exit {
  from {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
  to {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
}

@keyframes field-expand {
  from {
    opacity: 0;
    height: 0;
    margin-top: 0;
  }
  to {
    opacity: 1;
    height: auto;
    margin-top: 12px;
  }
}
```

---

## References

- [DESIGN.md](../DESIGN.md) - Master design system
- [specs/DESIGN_OVERHAUL_PLAN.md](../../DESIGN_OVERHAUL_PLAN.md) - Design overhaul strategy
- [modal-standards.md](./modal-standards.md) - Modal patterns (when complete)
- Linear.app - Project creation dialogs
- GitHub - Merge workflow dialogs
- Raycast - Settings and confirmation dialogs
- shadcn/ui Dialog - https://ui.shadcn.com/docs/components/dialog
- shadcn/ui Input - https://ui.shadcn.com/docs/components/input
- shadcn/ui Select - https://ui.shadcn.com/docs/components/select
- Lucide icons - https://lucide.dev/icons/
