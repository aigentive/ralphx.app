# Welcome Screen & Project Creation Redesign

## Overview

Create an impressive, animated welcome screen for RalphX when no projects exist, featuring AI/coding-themed visuals with smooth animations. Also fix UX issues with the Create Project modal and add keyboard shortcuts.

## Design Direction: "Terminal Symphony"

**Aesthetic**: A sophisticated dark terminal environment with floating code fragments, subtle particle effects, and warm orange accents. Think of a luxurious IDE meets creative studio—code as art.

**Key Visual Elements**:
- Animated terminal cursor with typing effect
- Floating code snippets with syntax highlighting (CSS-only animations)
- Subtle particle system suggesting AI orchestration
- Warm orange glow accents (#ff6b35) used sparingly
- Staggered entry animations for content
- Atmospheric gradient backgrounds per design system

**Anti-AI-Slop Compliance**:
- ✅ Warm orange accent (#ff6b35)
- ✅ SF Pro typography
- ✅ Layered shadows for depth
- ✅ No purple/blue gradients
- ✅ No Inter font

---

## Implementation Tasks

### Task 1: Create WelcomeScreen Component (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(welcome): create WelcomeScreen component with hero section and animations`

**File**: `src/components/WelcomeScreen/WelcomeScreen.tsx`

**Features**:
1. **Hero Section**:
   - Large "RalphX" title with subtle glow animation
   - Tagline: "Autonomous AI Development, Orchestrated"
   - Staggered fade-in animation

2. **Visual Element - Terminal Canvas**:
   - Dark terminal window with glass effect
   - Typing cursor animation (CSS keyframes)
   - Floating code snippets with syntax colors (green comments, orange keywords, etc.)
   - Subtle scan line effect for retro-futuristic feel

3. **Particle Effect**:
   - Subtle floating dots/particles suggesting "AI neurons"
   - CSS-only animation (no external libraries)
   - Warm orange and white particles on dark background

4. **CTA Section**:
   - Primary "Create Your First Project" button with hover glow
   - Keyboard shortcut hint (⌘N on welcome, ⌘⇧N globally)

**Animations** (CSS keyframes in component `<style>` tag):
- `fadeSlideIn`: Staggered content entry
- `terminalBlink`: Cursor blink
- `codeFloat`: Floating code snippets
- `particleDrift`: Subtle particle movement
- `glowPulse`: Orange accent glow on button

### Task 2: Create WelcomeScreen Subcomponents (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(welcome): add TerminalCanvas and ParticleField subcomponents`

**Files**:
- `src/components/WelcomeScreen/TerminalCanvas.tsx` - The visual terminal with floating code
- `src/components/WelcomeScreen/ParticleField.tsx` - CSS-only particle effect
- `src/components/WelcomeScreen/index.tsx` - Re-exports

**TerminalCanvas Features**:
- Mock terminal header with traffic lights (red, yellow, green circles)
- Terminal body with dark gradient background
- Animated code lines that appear to be typing
- Floating code fragments that drift slowly

**ParticleField Features**:
- CSS-animated particles (20-30 elements)
- Random positioning and animation delays
- Warm orange and white colors with varying opacity

### Task 3: Add Keyboard Shortcuts
**Dependencies:** None (can run in parallel with Task 1-2)
**Atomic Commit:** `feat(shortcuts): add ⌘N and ⌘⇧N for project creation`

**File**: `src/hooks/useAppKeyboardShortcuts.ts` (modify)

**New Shortcuts**:
| Shortcut | Context | Action |
|----------|---------|--------|
| `⌘N` | Welcome screen only (no projects) | Open Create Project modal |
| `⌘Shift+N` | Anywhere (global) | Open Create Project modal |

**Implementation**:
- Add `openProjectWizard` callback to hook props
- Add `hasProjects` boolean to hook props (to know context)
- Add case for 'n'/'N' key:
  - With meta/ctrl + shift → always open wizard
  - With meta/ctrl only → open wizard only if on welcome screen (no projects)
- Ensure both shortcuts work correctly

### Task 4: Fix Create Project Modal Close Behavior
**Dependencies:** None (can run in parallel)
**Atomic Commit:** `fix(wizard): add ESC hint and verify close button visibility`

**File**: `src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx` (modify)

**Current Issue**: When `isFirstRun=false`, user cannot close the modal because there's no visible close affordance (only backdrop click works but isn't discoverable).

**Fix**:
1. When `isFirstRun=false`:
   - Ensure close button (X) is visible in header (already using `hideCloseButton={isFirstRun}`)
   - Add ESC key hint in footer ("Press ESC to cancel")
   - Verify `onClose` is properly wired

2. Review `hideCloseButton` logic in DialogContent to ensure it shows X button when not first run

### Task 5: Integrate WelcomeScreen into App.tsx
**Dependencies:** Task 1, Task 2, Task 3
**Atomic Commit:** `feat(app): integrate WelcomeScreen and keyboard shortcuts`

**File**: `src/App.tsx` (modify)

**Changes**:
1. Replace current empty state (lines 631-662) with `<WelcomeScreen />`
2. Pass `onCreateProject` prop to trigger wizard
3. Ensure keyboard shortcut hook gets `openProjectWizard` callback

---

## Task Dependency Graph

```
Task 1 (WelcomeScreen) ──┐
                        ├──► Task 5 (Integration)
Task 2 (Subcomponents) ──┤
                        │
Task 3 (Shortcuts) ──────┘

Task 4 (Modal Fix) ─────────► Independent
```

**Parallelization Strategy:**
- Tasks 1+2 can be done together (same component folder)
- Tasks 3 and 4 can run in parallel with 1+2
- Task 5 must wait for 1, 2, and 3

---

## File Structure

```
src/components/WelcomeScreen/
├── index.tsx                 # Re-exports
├── WelcomeScreen.tsx         # Main component (~300 LOC)
├── TerminalCanvas.tsx        # Visual terminal element (~150 LOC)
└── ParticleField.tsx         # CSS particle effect (~80 LOC)
```

---

## Critical Files to Modify

1. `src/App.tsx` - Replace empty state, add shortcut prop
2. `src/hooks/useAppKeyboardShortcuts.ts` - Add ⌘N shortcut
3. `src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx` - Review close button visibility
4. `src/components/ui/dialog.tsx` - Verify hideCloseButton behavior (may already work)

---

## Animation Keyframes (To be added in WelcomeScreen)

```css
/* Cursor blink */
@keyframes terminalBlink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

/* Staggered fade in */
@keyframes fadeSlideIn {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Floating code */
@keyframes codeFloat {
  0%, 100% {
    transform: translateY(0) rotate(0deg);
    opacity: 0.6;
  }
  50% {
    transform: translateY(-10px) rotate(2deg);
    opacity: 0.8;
  }
}

/* Particle drift */
@keyframes particleDrift {
  0% {
    transform: translate(0, 0);
    opacity: 0;
  }
  10% {
    opacity: 0.6;
  }
  90% {
    opacity: 0.6;
  }
  100% {
    transform: translate(var(--drift-x), var(--drift-y));
    opacity: 0;
  }
}

/* Button glow */
@keyframes glowPulse {
  0%, 100% {
    box-shadow: 0 0 20px rgba(255, 107, 53, 0.3), 0 0 40px rgba(255, 107, 53, 0.1);
  }
  50% {
    box-shadow: 0 0 30px rgba(255, 107, 53, 0.5), 0 0 60px rgba(255, 107, 53, 0.2);
  }
}
```

---

## Verification Plan

1. **Visual Testing**:
   - Launch app with no projects
   - Verify welcome screen appears with animations
   - Check ⌘N opens the create project modal
   - Check animations are smooth (60fps)

2. **UX Testing**:
   - From welcome screen, press ⌘N → modal opens
   - From any view with projects, press ⌘⇧N → modal opens
   - In modal (non-first-run), press ESC → modal closes
   - In modal (non-first-run), click X button → modal closes
   - In modal (first-run), ESC and backdrop click should NOT close

3. **Design Compliance**:
   - No purple/blue gradients
   - Orange accent (#ff6b35) used sparingly
   - SF Pro font
   - Layered shadows
   - Glass effects with backdrop-blur

---

## Notes

- All animations are CSS-only (no framer-motion) to match existing patterns
- Component keeps animations co-located via `<style>` tag
- Particle count is limited (20-30) for performance
- Uses existing design tokens from globals.css

---

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
