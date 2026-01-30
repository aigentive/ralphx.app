# Plan: Dramatically Redesign ProposalEditModal

## Overview

Transform the current utilitarian `ProposalEditModal` into a **10x designer-level** experience that embodies the "Refined Studio" aesthetic while maintaining full functionality.

## Current State Analysis

The current modal is functional but generic:
- Basic shadcn Dialog with standard form fields
- Flat layout with repetitive input patterns
- No visual hierarchy or spatial interest
- Generic add/remove buttons for arrays
- Standard dropdowns feel uninspired

## Design Direction: **"Editorial Blueprint"**

**Concept**: The proposal edit modal should feel like editing a premium architectural blueprint or a high-end creative brief. Think Figma's property panels meets Linear's refined polish meets editorial magazine layouts.

**Key differentiators**:
1. **Spatial Asymmetry** - Break the monotonous vertical stack with a two-column metadata section
2. **Micro-interactions** - Smooth entry animations, hover reveals, focus transitions
3. **Visual Rhythm** - Numbered steps with elegant typography, refined spacing
4. **Glass Depth** - Layered translucent panels that create depth without heavy gradients
5. **Purposeful Animation** - Staggered reveals, subtle scale on hover

## Design Specifications

### Modal Container
- **Size**: `max-w-2xl` (672px) - more horizontal room for two-column layout
- **Background**: Frosted glass effect with subtle warm ambient glow at corners
- **Border**: Gradient border technique (white 8% to 2% top-to-bottom)
- **Shadow**: Layered `--shadow-lg` for premium depth

### Header Section
- **Layout**: Icon + Title left, subtle close button right
- **Icon**: Edit3 with subtle orange background pill (not just icon color)
- **Title**: 18px SF Pro Display, semibold, -0.02em tracking
- **Subtitle**: "Refine your task proposal" in muted text

### Content Layout (Revolutionary)

**Two-Zone Architecture**:

```
┌─────────────────────────────────────────────────────────────┐
│ [🎯] Edit Proposal                                        [×]│
│ Refine your task proposal                                    │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Title                                                       │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ Implement user authentication                          │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  Description                                                 │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                                                        │  │
│  │                                                        │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──── Metadata ─────────────┬──── Estimation ────────────┐  │
│  │                           │                            │  │
│  │  Category                 │  Complexity                │  │
│  │  [Feature ▾]              │  ○ ○ ○ ● ○                 │  │
│  │                           │  Moderate                   │  │
│  │  Priority Override        │                            │  │
│  │  [Auto (High) ▾]          │                            │  │
│  │                           │                            │  │
│  └───────────────────────────┴────────────────────────────┘  │
│                                                              │
│  Implementation Steps                                        │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ① Set up authentication provider               [−]    │  │
│  │  ② Create login/signup forms                    [−]    │  │
│  │  ③ Implement session management                 [−]    │  │
│  │                                                        │  │
│  │         ┌─────────────────────────────────┐           │  │
│  │         │  + Add another step             │           │  │
│  │         └─────────────────────────────────┘           │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  Acceptance Criteria                                         │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  ✓ Users can sign up with email                [−]    │  │
│  │  ✓ Users can log in with credentials           [−]    │  │
│  │                                                        │  │
│  │         ┌─────────────────────────────────────────┐    │  │
│  │         │  + Add acceptance criterion            │    │  │
│  │         └─────────────────────────────────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│                                      [Cancel]   [Save Changes]│
└──────────────────────────────────────────────────────────────┘
```

### Component Details

#### 1. Glass Input Fields
```css
.glass-input {
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  transition: all 180ms ease;
}
.glass-input:focus {
  border-color: rgba(255, 107, 53, 0.5);
  box-shadow: 0 0 0 3px rgba(255, 107, 53, 0.1);
}
```

#### 2. Metadata Panel (Two-Column)
- Frosted glass card with `backdrop-blur-xl`
- Left side: Category + Priority Override dropdowns
- Right side: Visual complexity selector (5-dot scale)
- Subtle divider line between columns

#### 3. Complexity Visual Selector (NEW)
Replace dropdown with elegant 5-dot visual scale:
- 5 circles: trivial → very_complex
- Orange fill indicates current selection
- Click to select, hover shows label tooltip
- Clean, Linear-inspired design

#### 4. Steps & Criteria Lists
- **Numbered circles** (①②③) for steps using circled numbers
- **Checkmark prefix** (✓) for acceptance criteria
- **Glass card container** with slight border
- **Inline delete button** that appears on hover (not always visible)
- **Add button**: Centered, dashed border, full-width click target
- **Empty state**: Elegant message with icon

#### 5. Custom Select Dropdowns
- Glass background with blur
- Orange accent on focus
- Smooth dropdown animation
- Custom chevron icon styling

### Animation Specifications

#### Entry Animation (Modal Open)
```css
@keyframes modal-slide-up {
  from {
    opacity: 0;
    transform: translateY(20px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
/* Duration: 250ms, easing: cubic-bezier(0.16, 1, 0.3, 1) */
```

#### Staggered Content Animation
- Each section fades in with 50ms delay between sections
- Creates elegant reveal effect

#### Micro-interactions
- Input focus: subtle scale(1.01) + glow
- Button hover: translateY(-1px) + shadow increase
- Delete button: fade in on row hover
- Complexity dots: scale(1.2) on hover

### Color Application

| Element | Color |
|---------|-------|
| Modal background | `rgba(18, 18, 18, 0.95)` + `backdrop-blur(32px)` |
| Input background | `rgba(0, 0, 0, 0.3)` |
| Input border | `rgba(255, 255, 255, 0.08)` |
| Focus ring | `rgba(255, 107, 53, 0.5)` border + `rgba(255, 107, 53, 0.1)` glow |
| Section card | `rgba(255, 255, 255, 0.03)` |
| Accent elements | `#ff6b35` (warm orange) |
| Muted text | `rgba(255, 255, 255, 0.4)` |

## Implementation Tasks

### Task 1: Expand modal and add header subtitle (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(ideation): expand modal width and add header subtitle`

- Expand modal width to `max-w-2xl`
- Add header subtitle "Refine your task proposal" in muted text
- Update icon container with orange background pill

Files: `src/components/Ideation/ProposalEditModal.tsx`

### Task 2: Create two-column metadata section (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(ideation): create two-column metadata panel`

- Create CSS Grid layout for metadata section
- Left column: Category + Priority Override dropdowns
- Right column: Placeholder for complexity selector
- Glass card container with frosted effect
- Subtle divider line between columns

Files: `src/components/Ideation/ProposalEditModal.tsx`

### Task 3: Implement ComplexitySelector visual component (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `feat(ideation): add visual 5-dot complexity selector`

- Create inline `ComplexitySelector` component
- 5 circles representing trivial → very_complex
- Orange fill for selected, hover states with scale
- Tooltip on hover showing label
- Wire to existing complexity state

Files: `src/components/Ideation/ProposalEditModal.tsx`

### Task 4: Redesign steps list with EditableListItem (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(ideation): redesign steps list with circled numbers`

- Add circled number prefix (①②③) to each step
- Create glass card container for steps section
- Hover-reveal delete button (not always visible)
- Centered dashed-border add button
- Elegant empty state with icon

Files: `src/components/Ideation/ProposalEditModal.tsx`

### Task 5: Redesign acceptance criteria list
**Dependencies:** Task 4
**Atomic Commit:** `feat(ideation): redesign criteria list with checkmarks`

- Add checkmark prefix (✓) to each criterion
- Reuse glass card container pattern from Task 4
- Hover-reveal delete button
- Centered dashed-border add button
- Elegant empty state with icon

Files: `src/components/Ideation/ProposalEditModal.tsx`

### Task 6: Add glass input styling to all fields
**Dependencies:** Task 1
**Atomic Commit:** `feat(ideation): apply glass effect to all input fields`

- Update inputClasses with glass effect styling
- Background: rgba(0, 0, 0, 0.3)
- Border: rgba(255, 255, 255, 0.08)
- Focus: Orange border + glow ring
- Apply to Title, Description, Steps, Criteria inputs

Files: `src/components/Ideation/ProposalEditModal.tsx`

### Task 7: Add entry animations and staggered timing
**Dependencies:** Tasks 1-6
**Atomic Commit:** `feat(ideation): add modal entry animations`

- Add @keyframes modal-slide-up animation
- Implement staggered content animation (50ms delay per section)
- Add CSS for opacity/transform transitions
- Ensure smooth modal open/close

Files: `src/components/Ideation/ProposalEditModal.tsx`

### Task 8: Add micro-interactions and ambient glow
**Dependencies:** Task 7
**Atomic Commit:** `feat(ideation): add micro-interactions and ambient glow`

- Input focus: subtle scale(1.01) + glow
- Button hover: translateY(-1px) + shadow increase
- Delete button: fade in on row hover
- Complexity dots: scale(1.2) on hover
- Add ambient warm glow to modal corners

Files: `src/components/Ideation/ProposalEditModal.tsx`

## Verification

1. **Visual verification**: Open modal, verify premium look & feel
2. **Functionality**: All fields work (title, desc, category, steps, criteria, priority, complexity)
3. **Animations**: Smooth entry, staggered reveals, micro-interactions
4. **Responsiveness**: Works at various modal sizes
5. **Accessibility**: Focus management, keyboard navigation maintained
6. **Tests**: Existing tests should still pass (data-testid preserved)

## Design Quality Checklist

- [x] NO purple/blue gradients
- [x] NO Inter font (uses SF Pro via system)
- [x] Warm orange accent `#ff6b35` used strategically
- [x] Layered shadows for depth
- [x] Glass/blur effects for premium feel
- [x] Tight letter-spacing on headings
- [x] Staggered entry animations
- [x] Micro-interactions on hover/focus
- [x] Visual hierarchy through spacing and typography
- [x] Unique, memorable design (not cookie-cutter)

## Commit Lock Workflow (Parallel Agent Coordination)

Reference: `.claude/rules/commit-lock.md`

### Before Committing
```bash
# 1. Establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Check/acquire lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Read lock content, wait 3s, retry up to 30s
  # If stale (same content >30s), delete and proceed
fi

# 3. Create lock
echo "<stream-name> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 4. Stage and commit
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

### After Committing
```bash
# ALWAYS release lock (success or failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Lock Rules
1. Acquire lock BEFORE `git add`
2. Release lock AFTER commit (success OR failure)
3. Stale = same content + >30 sec old
4. Never force-delete active lock from another agent
