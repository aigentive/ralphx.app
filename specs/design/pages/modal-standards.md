# Modal Standards

Standardized design patterns for all modal dialogs in RalphX. All modals must follow these patterns to ensure visual consistency and premium feel.

---

## Overview

RalphX uses **shadcn/ui Dialog** (built on Radix UI) as the foundation for all modal dialogs. This document establishes consistent patterns for structure, sizing, animation, and behavior across all modals.

### Reference Apps for Modals
- **Linear**: Clean header/content/footer structure, subtle animations
- **Raycast**: Mac-native feel, backdrop blur, keyboard-first
- **Vercel Dashboard**: Consistent spacing, clear hierarchy

---

## Base Modal Pattern

All modals share these foundational characteristics.

### Structure
```
┌─────────────────────────────────────┐
│ [Icon] Title                    [X] │  ← DialogHeader
├─────────────────────────────────────┤
│                                     │
│         Content Area                │  ← DialogContent body
│                                     │
├─────────────────────────────────────┤
│                   [Cancel] [Primary]│  ← DialogFooter
└─────────────────────────────────────┘
```

### Backdrop Styling
```css
.modal-backdrop {
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}
```
- Semi-transparent black with blur for glass effect
- Click to close (unless destructive action is pending)
- No click-through to underlying content

### Content Container Styling
```css
.modal-content {
  background: var(--bg-elevated);  /* #242424 */
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-lg);  /* 12px */
  box-shadow: var(--shadow-lg);
}
```

### Animation Specifications
| Phase | Duration | Easing | Properties |
|-------|----------|--------|------------|
| Enter | 200ms | ease-out | scale: 0.95→1.0, opacity: 0→1 |
| Exit | 150ms | ease-in | scale: 1.0→0.95, opacity: 1→0 |

```css
/* Enter animation */
@keyframes modal-enter {
  from {
    opacity: 0;
    transform: translate(-50%, -50%) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translate(-50%, -50%) scale(1);
  }
}

/* Backdrop fade */
@keyframes backdrop-enter {
  from { opacity: 0; }
  to { opacity: 1; }
}
```

### Keyboard Behavior
| Key | Action | Notes |
|-----|--------|-------|
| Escape | Close modal | Unless destructive action pending or isProcessing |
| Tab | Cycle focus | Trapped within modal |
| Enter | Activate focused element | Submit form if input focused |

---

## Modal Sizes

### Size Variants

| Variant | Max Width | Tailwind Class | Use Case |
|---------|-----------|----------------|----------|
| Small | 384px | `max-w-sm` | Simple confirmations, alerts |
| Medium | 448px | `max-w-md` | Forms, single-purpose dialogs |
| Large | 512px | `max-w-lg` | Complex forms, multi-section content |
| XLarge | 640px | `max-w-xl` | Task detail, wizards |
| 2XLarge | 672px | `max-w-2xl` | Full-featured dialogs, side-by-side |

### Height Constraints
- **Default**: Content-driven height, max 90vh (`max-h-[90vh]`)
- **Scrollable content**: Use `overflow-y-auto` on content area only
- **Header/Footer**: Always fixed, never scroll

### Responsive Behavior
- Mobile (<640px): Full width with 16px margins (`mx-4`)
- Tablet (640-1024px): Use smaller variant if needed
- Desktop (>1024px): Use specified size

---

## Header Pattern

### Standard Header Structure
```tsx
<DialogHeader className="flex items-center justify-between px-6 py-4 border-b border-[var(--border-subtle)]">
  <div className="flex items-center gap-3">
    {icon && <span className="text-[var(--accent-primary)]">{icon}</span>}
    <DialogTitle className="text-lg font-semibold text-[var(--text-primary)] tracking-tight">
      {title}
    </DialogTitle>
  </div>
  <DialogClose className="p-1.5 rounded-md hover:bg-[var(--bg-hover)] transition-colors">
    <X className="w-4 h-4 text-[var(--text-muted)]" />
    <span className="sr-only">Close</span>
  </DialogClose>
</DialogHeader>
```

### Header Specifications
| Element | Style | Notes |
|---------|-------|-------|
| Container padding | 24px horizontal, 16px vertical | `px-6 py-4` |
| Title | text-lg (18px), font-semibold (600), --text-primary | Tight tracking (-0.02em) |
| Subtitle (optional) | text-sm (14px), --text-muted | Below title, 4px gap |
| Icon (optional) | 20px, accent or semantic color | Before title |
| Close button | 16px icon, 6px padding | Top-right, hover state |
| Border | 1px solid --border-subtle | Bottom border only |

### Header Variants

**Standard (title only):**
```tsx
<DialogTitle>Modal Title</DialogTitle>
```

**With Icon:**
```tsx
<div className="flex items-center gap-3">
  <CheckCircle className="w-5 h-5 text-[var(--status-success)]" />
  <DialogTitle>Success</DialogTitle>
</div>
```

**With Subtitle:**
```tsx
<DialogHeader>
  <DialogTitle>Edit Proposal</DialogTitle>
  <DialogDescription>Make changes to your proposal details.</DialogDescription>
</DialogHeader>
```

---

## Footer Pattern

### Standard Footer Structure
```tsx
<DialogFooter className="flex items-center justify-end gap-3 px-6 py-4 border-t border-[var(--border-subtle)]">
  <Button variant="ghost" onClick={onClose} disabled={isProcessing}>
    Cancel
  </Button>
  <Button onClick={onConfirm} disabled={!canSubmit || isProcessing}>
    {isProcessing ? <Loader2 className="w-4 h-4 mr-2 animate-spin" /> : null}
    {primaryLabel}
  </Button>
</DialogFooter>
```

### Footer Specifications
| Element | Style | Notes |
|---------|-------|-------|
| Container padding | 24px horizontal, 16px vertical | `px-6 py-4` |
| Button gap | 12px | `gap-3` |
| Alignment | Right-aligned | `justify-end` |
| Border | 1px solid --border-subtle | Top border only |

### Button Patterns by Action Type

| Action Type | Primary Button | Cancel Button |
|-------------|----------------|---------------|
| Create/Add | Primary variant (accent) | Ghost |
| Update/Save | Primary variant (accent) | Ghost |
| Delete/Discard | Destructive variant (red) | Ghost |
| Confirm (neutral) | Secondary variant | Ghost |

### Loading State
```tsx
<Button disabled={isProcessing}>
  {isProcessing && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
  {isProcessing ? "Processing..." : "Submit"}
</Button>
```

### Multi-Action Footers
For modals with more than two actions:
```tsx
<DialogFooter className="flex items-center justify-between">
  <Button variant="ghost" size="sm">
    Learn More
  </Button>
  <div className="flex gap-3">
    <Button variant="ghost">Cancel</Button>
    <Button>Continue</Button>
  </div>
</DialogFooter>
```

---

## Content Area Patterns

### Spacing
- **Padding**: 24px all sides (`p-6`)
- **Section gaps**: 20-24px between major sections (`space-y-5` or `space-y-6`)
- **Element gaps**: 12-16px between related elements (`space-y-3` or `space-y-4`)

### Scrollable Content
```tsx
<div className="px-6 py-5 max-h-[60vh] overflow-y-auto">
  {/* Scrollable content */}
</div>
```

### Form Layout
```tsx
<div className="space-y-4">
  <div className="space-y-2">
    <Label htmlFor="name">Name</Label>
    <Input id="name" placeholder="Enter name..." />
  </div>
  <div className="space-y-2">
    <Label htmlFor="description">Description</Label>
    <Textarea id="description" placeholder="Describe..." />
  </div>
</div>
```

### Dividers
Use sparingly to separate logical sections:
```tsx
<div className="h-px bg-[var(--border-subtle)]" />
```

---

## Specific Modal Implementations

### AskUserQuestionModal

**Purpose:** Agent questions requiring user selection/input

**Size:** Medium (`max-w-md`)

**Structure:**
```
┌─────────────────────────────────────┐
│ [Header Question]                   │
├─────────────────────────────────────┤
│ Question text                       │
│                                     │
│ ○ Option 1                          │
│   Description text                  │
│                                     │
│ ○ Option 2                          │
│   Description text                  │
│                                     │
│ ○ Other                             │
│   [Text input when selected]        │
├─────────────────────────────────────┤
│                      [Submit Answer]│
└─────────────────────────────────────┘
```

**Key Requirements:**
- Radio buttons (single select) or Checkboxes (multi-select) using shadcn
- Always include "Other" option with conditional text input
- Submit button disabled until valid selection
- Use semantic button color (--status-success for positive action)
- Header shows question category, body shows full question

**shadcn Components:**
- Dialog, DialogHeader, DialogContent, DialogFooter
- RadioGroup, RadioGroupItem (or Checkbox for multi-select)
- Label
- Input (for "Other" text)
- Button

### TaskDetailView (as Modal)

**Purpose:** Display comprehensive task details

**Size:** XLarge (`max-w-xl`)

**Structure:**
```
┌─────────────────────────────────────────┐
│ Task Title                    [Badge] X │
├─────────────────────────────────────────┤
│ Priority: ■■■□□  Status: In Progress    │
├─────────────────────────────────────────┤
│ Description                             │
│ Full task description text...           │
├─────────────────────────────────────────┤
│ ▼ Steps (3/5 complete)                  │
│   ☑ Step 1                              │
│   ☑ Step 2                              │
│   ☐ Step 3                              │
├─────────────────────────────────────────┤
│ ▼ Reviews                               │
│   [Review cards...]                     │
├─────────────────────────────────────────┤
│ ▼ State History                         │
│   Timeline visualization...             │
├─────────────────────────────────────────┤
│                              [Close]    │
└─────────────────────────────────────────┘
```

**Key Requirements:**
- Scrollable content area with collapsible sections
- Status badge using shadcn Badge with semantic colors
- Priority indicator with visual blocks
- State history as vertical timeline
- Sections collapsible via shadcn Collapsible

**shadcn Components:**
- Dialog, DialogHeader, DialogContent, DialogFooter
- Badge (for status)
- Collapsible, CollapsibleTrigger, CollapsibleContent
- Checkbox (for steps)
- ScrollArea

### ReviewNotesModal

**Purpose:** Add notes and optional fix description for reviews

**Size:** Medium (`max-w-md`)

**Structure:**
```
┌─────────────────────────────────────┐
│ Add Review Notes                  X │
├─────────────────────────────────────┤
│ Notes *                             │
│ ┌─────────────────────────────────┐ │
│ │                                 │ │
│ │                                 │ │
│ └─────────────────────────────────┘ │
│                                     │
│ Fix Description (optional)          │
│ ┌─────────────────────────────────┐ │
│ │                                 │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│                   [Cancel] [Submit] │
└─────────────────────────────────────┘
```

**Key Requirements:**
- Two textarea fields with labels
- Conditional rendering of fix description based on props
- Required indicator (*) on notes field
- Submit disabled when required field empty

**shadcn Components:**
- Dialog, DialogHeader, DialogContent, DialogFooter
- Label
- Textarea
- Button

### ProposalEditModal

**Purpose:** Edit proposal details (title, description, steps, etc.)

**Size:** Large (`max-w-lg`)

**Structure:**
```
┌─────────────────────────────────────────┐
│ Edit Proposal                         X │
├─────────────────────────────────────────┤
│ Title *                                 │
│ [________________________________]      │
│                                         │
│ Description                             │
│ [________________________________]      │
│ [________________________________]      │
│                                         │
│ Category      Priority                  │
│ [Dropdown ▾]  [● ● ● ○ ○]               │
│                                         │
│ Steps                                   │
│ • Step 1                          [×]   │
│ • Step 2                          [×]   │
│ [+ Add Step]                            │
│                                         │
│ Acceptance Criteria                     │
│ • Criteria 1                      [×]   │
│ [+ Add Criteria]                        │
├─────────────────────────────────────────┤
│                   [Cancel] [Save]       │
└─────────────────────────────────────────┘
```

**Key Requirements:**
- Auto-focus on title input when modal opens
- Dynamic list management for steps and criteria
- Category dropdown using shadcn Select
- Priority selector (visual 5-dot scale)
- Scrollable content for long forms

**shadcn Components:**
- Dialog, DialogHeader, DialogContent, DialogFooter
- Input, Textarea
- Label
- Select, SelectTrigger, SelectContent, SelectItem
- Button
- ScrollArea

### MergeWorkflowDialog

**Purpose:** Post-completion workflow options for worktree projects

**Size:** Large (`max-w-lg`)

**Structure:**
```
┌─────────────────────────────────────────┐
│ ✓ Project Complete: ProjectName       X │
├─────────────────────────────────────────┤
│ RalphX made 5 commits on branch:        │
│ feature/task-123                        │
│                                         │
│ [View Diff] [View Commits]              │
├─────────────────────────────────────────┤
│ What would you like to do?              │
│                                         │
│ ○ Merge to main                         │
│   Creates a merge commit...             │
│                                         │
│ ○ Rebase onto main                      │
│   Replays commits on top...             │
│                                         │
│ ○ Create Pull Request                   │
│   Opens GitHub to create PR...          │
│                                         │
│ ○ Keep worktree                         │
│   Leave as-is and merge later...        │
│                                         │
│ ○ Discard changes (destructive)         │
│   Delete worktree and branch...         │
│   ⚠ This cannot be undone              │
├─────────────────────────────────────────┤
│                   [Cancel] [Continue]   │
└─────────────────────────────────────────┘
```

**Key Requirements:**
- Success icon in header (status-success color)
- Summary section with commit count and branch name
- Action buttons for viewing diff/commits
- Radio options with icons and descriptions
- Destructive option styled differently (red text/border)
- Warning message for destructive selection
- Secondary confirmation for discard action
- Processing state disables all interactions

**shadcn Components:**
- Dialog, DialogHeader, DialogContent, DialogFooter
- RadioGroup, RadioGroupItem
- Label
- Button
- Badge

### TaskRerunDialog

**Purpose:** Options when moving completed task back to planned

**Size:** Medium (`max-w-md`)

**Structure:**
```
┌─────────────────────────────────────────┐
│ Re-run Task                           X │
├─────────────────────────────────────────┤
│ Commit: abc1234                         │
│ "feat: implemented feature X"           │
│                                         │
│ ⚠ This commit has dependent commits     │
├─────────────────────────────────────────┤
│ ○ Keep changes (Recommended)            │
│   Keep existing commit, retry task      │
│                                         │
│ ○ Revert commit                         │
│   Undo commit before retrying           │
│   ⚠ May affect dependent commits        │
│                                         │
│ ○ Create new task                       │
│   Create fresh task, keep commit        │
├─────────────────────────────────────────┤
│                   [Cancel] [Continue]   │
└─────────────────────────────────────────┘
```

**Key Requirements:**
- Display commit info (SHA in monospace, message)
- Warning for dependent commits
- "Recommended" badge on default option
- Warning on revert option
- Clear descriptions for each choice

**shadcn Components:**
- Dialog, DialogHeader, DialogContent, DialogFooter
- RadioGroup, RadioGroupItem
- Label
- Badge (for "Recommended")
- Button

### ProjectCreationWizard

**Purpose:** Create new project with git mode selection

**Size:** Large (`max-w-lg`)

**Structure:**
```
┌─────────────────────────────────────────┐
│ Create New Project                    X │
├─────────────────────────────────────────┤
│ Project Name *                          │
│ [________________________________]      │
│                                         │
│ Working Directory *                     │
│ [__________________________] [Browse]   │
│                                         │
│ Git Mode                                │
│ ○ Local                                 │
│   Work directly on current branch       │
│                                         │
│ ● Worktree (Recommended)                │
│   Create isolated worktree branch       │
│                                         │
│ ┌─ Worktree Options ─────────────────┐  │
│ │ Base Branch                        │  │
│ │ [main ▾]                           │  │
│ │                                    │  │
│ │ Branch Name                        │  │
│ │ [ralphx/project-name]              │  │
│ └────────────────────────────────────┘  │
├─────────────────────────────────────────┤
│                   [Cancel] [Create]     │
└─────────────────────────────────────────┘
```

**Key Requirements:**
- Multi-step wizard feel (single view with conditional fields)
- Folder picker integration (Browse button)
- Git mode radio selection with "Recommended" indicator
- Conditional worktree options (only visible when worktree selected)
- Branch dropdown populated dynamically
- Auto-generated branch name from project name
- Validation states and error messages
- Processing state during creation

**shadcn Components:**
- Dialog, DialogHeader, DialogContent, DialogFooter
- Input
- Label
- RadioGroup, RadioGroupItem
- Select, SelectTrigger, SelectContent, SelectItem
- Button
- Badge (for "Recommended")

### ApplyModal (Ideation)

**Purpose:** Apply selected proposals to Kanban board

**Size:** Large (`max-w-lg`)

**Structure:**
```
┌─────────────────────────────────────────┐
│ Apply Proposals                       X │
├─────────────────────────────────────────┤
│ Selected Proposals (3)                  │
│ ┌─────────────────────────────────────┐ │
│ │ • Proposal title 1                  │ │
│ │ • Proposal title 2                  │ │
│ │ • Proposal title 3                  │ │
│ └─────────────────────────────────────┘ │
│                                         │
│ Dependency Preview                      │
│ [Visual dependency graph]               │
│                                         │
│ ⚠ Circular dependency detected          │
│                                         │
│ Target Column                           │
│ [Backlog ▾]                             │
│                                         │
│ ☑ Preserve dependencies                 │
├─────────────────────────────────────────┤
│                   [Cancel] [Apply]      │
└─────────────────────────────────────────┘
```

**Key Requirements:**
- Summary of selected proposals (scrollable list)
- Visual dependency graph preview
- Warning detection and display
- Target column dropdown
- Preserve dependencies checkbox
- Apply button disabled if warnings present

**shadcn Components:**
- Dialog, DialogHeader, DialogContent, DialogFooter
- ScrollArea
- Select, SelectTrigger, SelectContent, SelectItem
- Checkbox
- Label
- Button
- Alert (for warnings)

---

## Accessibility Requirements

### Focus Management
- Focus trapped within modal when open
- Initial focus on first interactive element (or close button)
- Return focus to trigger element on close

### ARIA Attributes
```tsx
<Dialog>
  <DialogContent
    role="dialog"
    aria-modal="true"
    aria-labelledby="dialog-title"
    aria-describedby="dialog-description"
  >
    <DialogTitle id="dialog-title">Title</DialogTitle>
    <DialogDescription id="dialog-description">Description</DialogDescription>
  </DialogContent>
</Dialog>
```

### Keyboard Navigation
- Tab cycles through interactive elements
- Escape closes modal (when not destructive/processing)
- Enter submits form or activates focused button
- Arrow keys navigate within RadioGroup/Select

### Screen Reader Support
- Descriptive button labels
- Error messages announced via `aria-live="polite"`
- Loading states announced
- Close button has `sr-only` text

---

## Testing Requirements

### Data Attributes
All modals must include these for testing:
```tsx
data-testid="[modal-name]-modal"
data-testid="modal-overlay"
data-testid="modal-content"
data-testid="dialog-close"
data-testid="cancel-button"
data-testid="confirm-button"
```

### Test Scenarios
1. Opens and closes correctly
2. Escape key closes modal
3. Backdrop click closes modal (unless destructive)
4. Focus trapped within modal
5. Form validation works
6. Submit disabled until valid
7. Loading state displays correctly
8. Error state displays correctly

---

## Component Hierarchy

```
Modal System
├── Dialog (shadcn/Radix wrapper)
│   ├── DialogTrigger
│   ├── DialogPortal
│   │   ├── DialogOverlay
│   │   │   └── Backdrop (blur + semi-transparent)
│   │   └── DialogContent
│   │       ├── DialogHeader
│   │       │   ├── Icon (optional, Lucide)
│   │       │   ├── DialogTitle
│   │       │   ├── DialogDescription (optional)
│   │       │   └── DialogClose (X button)
│   │       ├── Content Area
│   │       │   ├── Form elements (Input, Select, etc.)
│   │       │   ├── Lists (RadioGroup, Checkbox groups)
│   │       │   ├── Alerts/Warnings
│   │       │   └── Custom content
│   │       └── DialogFooter
│   │           ├── Secondary Button (ghost)
│   │           └── Primary Button
│   └── DialogClose
│
├── Specific Implementations
│   ├── AskUserQuestionModal
│   │   └── Uses: Dialog, RadioGroup/Checkbox, Input, Button
│   ├── TaskDetailView (modal mode)
│   │   └── Uses: Dialog, Badge, Collapsible, ScrollArea
│   ├── ReviewNotesModal
│   │   └── Uses: Dialog, Textarea, Label, Button
│   ├── ProposalEditModal
│   │   └── Uses: Dialog, Input, Textarea, Select, Button
│   ├── MergeWorkflowDialog
│   │   └── Uses: Dialog, RadioGroup, Badge, Button
│   ├── TaskRerunDialog
│   │   └── Uses: Dialog, RadioGroup, Badge, Button
│   ├── ProjectCreationWizard
│   │   └── Uses: Dialog, Input, RadioGroup, Select, Button
│   └── ApplyModal
│       └── Uses: Dialog, ScrollArea, Select, Checkbox, Button
│
└── Shared Patterns
    ├── Size variants (sm, md, lg, xl, 2xl)
    ├── Animation (scale + opacity)
    ├── Backdrop (blur + dark overlay)
    ├── Header/Footer patterns
    └── Focus management
```

---

## Lucide Icons for Modals

| Icon | Usage |
|------|-------|
| X | Close button |
| CheckCircle | Success header |
| AlertCircle | Warning header |
| XCircle | Error header |
| Info | Info header |
| Loader2 | Loading spinner (animate-spin) |
| ChevronDown | Collapsible trigger |
| Plus | Add item button |
| Trash2 | Delete/remove item |
| GitMerge | Merge option |
| GitBranch | Branch/worktree option |
| GitPullRequest | PR option |
| RefreshCw | Retry/re-run option |
| FolderOpen | Directory picker |

---

## Migration Notes

### Current State
Most modals are custom-built with:
- Fixed positioning and manual overlay
- Inline SVG icons
- CSS variable styling
- Manual focus management
- Custom animation handling

### Target State
Migrate to shadcn Dialog pattern with:
- Radix-based Dialog primitives
- Lucide React icons
- Tailwind + CSS variables
- Built-in accessibility
- Standardized animations

### Migration Priority
1. **High**: AskUserQuestionModal (most frequently seen)
2. **High**: ProjectCreationWizard (user onboarding)
3. **Medium**: MergeWorkflowDialog (critical decision point)
4. **Medium**: TaskRerunDialog (critical decision point)
5. **Medium**: ProposalEditModal (frequent use)
6. **Low**: ReviewNotesModal (less frequent)
7. **Low**: ApplyModal (less frequent)

---

## Acceptance Criteria

```json
{
  "acceptance_criteria": [
    "All modals use shadcn Dialog as base component",
    "Backdrop has blur effect and correct opacity (rgba(0,0,0,0.6) + blur(8px))",
    "Content uses --bg-elevated background with --border-subtle border",
    "Animation plays on open (scale 0.95→1, opacity 0→1, 200ms)",
    "Animation plays on close (reverse of open, 150ms)",
    "Escape key closes modal when not processing",
    "Backdrop click closes modal when not destructive",
    "Focus trapped within modal",
    "First interactive element receives focus on open",
    "Focus returns to trigger on close",
    "Headers follow standard pattern (title + optional icon + close button)",
    "Footers follow standard pattern (cancel ghost + primary right-aligned)",
    "Loading state shows spinner and disables interactions",
    "Error states display with appropriate styling",
    "All modals have correct data-testid attributes",
    "ARIA attributes properly set (role, aria-modal, aria-labelledby)",
    "Inline SVG icons replaced with Lucide React",
    "Consistent sizing per modal purpose (sm/md/lg/xl/2xl)",
    "Mobile responsive with appropriate margins",
    "Keyboard navigation works (Tab, Enter, Escape, Arrows)"
  ]
}
```

---

## Design Quality Checklist

```json
{
  "design_quality": [
    "NO purple/blue gradients - use warm orange accent sparingly",
    "NO Inter font - use SF Pro for text",
    "Layered shadows create depth (--shadow-lg for modal content)",
    "Backdrop blur provides glass effect separation",
    "Scale animation feels premium and intentional",
    "Header has proper visual hierarchy (icon, title, close)",
    "Footer buttons properly aligned with correct variants",
    "Consistent 24px padding throughout (--space-6)",
    "Border radius 12px (--radius-lg) on modal content",
    "Close button hover state visible but subtle",
    "Primary button uses accent color (one per modal)",
    "Destructive actions use error color variant",
    "Text hierarchy: title (lg/semibold), description (sm/secondary), body (base/primary)",
    "Spacing follows 8pt grid system",
    "Icons sized appropriately (16px for buttons, 20px for headers)",
    "Loading spinner animation smooth (animate-spin)",
    "Error/warning messages have appropriate semantic colors",
    "Selected states use accent border or background tint",
    "Disabled states have reduced opacity (50%)",
    "All hover states have visible feedback"
  ]
}
```
