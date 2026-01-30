# RalphX - Phase 35: Welcome Screen & Project Creation Redesign

## Overview

Create an impressive, animated welcome screen for RalphX when no projects exist, featuring AI/coding-themed visuals with smooth animations. This phase also fixes UX issues with the Create Project modal and adds keyboard shortcuts for quick project creation.

The welcome screen follows the "Terminal Symphony" design direction—a sophisticated dark terminal environment with floating code fragments, subtle particle effects, and warm orange accents (#ff6b35). The aesthetic evokes a luxurious IDE meets creative studio, presenting code as art.

**Reference Plan:**
- `specs/plans/welcome_screen_project_creation_redesign.md` - Detailed implementation plan with component structure, animations, and design specifications

## Goals

1. Create an impressive first-run experience that showcases RalphX's AI development capabilities
2. Add keyboard shortcuts (⌘N, ⌘⇧N) for quick project creation
3. Fix UX issues with the Create Project modal close behavior
4. Maintain design system compliance (no purple/blue gradients, warm orange accent, SF Pro typography)

## Dependencies

### Phase 14 (Design Implementation) - Required

| Dependency | Why Needed |
|------------|------------|
| shadcn/ui components | Dialog, Button, and other UI components used in wizard |
| Design tokens (globals.css) | CSS variables for colors, shadows, and typography |
| Glass morphism patterns | Established backdrop-blur and gradient patterns |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/welcome_screen_project_creation_redesign.md`
2. Understand the animation keyframes and component structure
3. Follow Anti-AI-Slop guidelines (no purple gradients, no Inter font)
4. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol:**

Reference: `.claude/rules/commit-lock.md`

1. Establish project root: `PROJECT_ROOT="$(git rev-parse --show-toplevel)"`
2. Acquire lock before `git add` (see commit-lock.md § Protocol)
3. Stage and commit using `git -C "$PROJECT_ROOT"`
4. Release lock after commit: `rm -f "$PROJECT_ROOT/.commit-lock"`

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/welcome_screen_project_creation_redesign.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Create WelcomeScreen main component with hero section and animations",
    "plan_section": "Task 1: Create WelcomeScreen Component",
    "blocking": [2, 5],
    "blockedBy": [],
    "atomic_commit": "feat(welcome): create WelcomeScreen component with hero section and animations",
    "steps": [
      "Read specs/plans/welcome_screen_project_creation_redesign.md section 'Task 1'",
      "Create src/components/WelcomeScreen/WelcomeScreen.tsx",
      "Implement hero section with RalphX title and tagline",
      "Implement terminal canvas placeholder (detailed in task 2)",
      "Implement particle field placeholder (detailed in task 2)",
      "Implement CTA button with glow animation",
      "Add CSS keyframes in <style> tag (fadeSlideIn, terminalBlink, glowPulse)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): create WelcomeScreen component with hero section and animations"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Create TerminalCanvas and ParticleField subcomponents",
    "plan_section": "Task 2: Create WelcomeScreen Subcomponents",
    "blocking": [5],
    "blockedBy": [1],
    "atomic_commit": "feat(welcome): add TerminalCanvas and ParticleField subcomponents",
    "steps": [
      "Read specs/plans/welcome_screen_project_creation_redesign.md section 'Task 2'",
      "Create src/components/WelcomeScreen/TerminalCanvas.tsx with terminal header (traffic lights), dark body, typing animation, floating code fragments",
      "Create src/components/WelcomeScreen/ParticleField.tsx with CSS-animated particles (20-30 elements), random positioning, warm orange/white colors",
      "Create src/components/WelcomeScreen/index.tsx with re-exports",
      "Add codeFloat and particleDrift keyframes",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): add TerminalCanvas and ParticleField subcomponents"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add keyboard shortcuts for project creation (⌘N, ⌘⇧N)",
    "plan_section": "Task 3: Add Keyboard Shortcuts",
    "blocking": [5],
    "blockedBy": [],
    "atomic_commit": "feat(shortcuts): add ⌘N and ⌘⇧N for project creation",
    "steps": [
      "Read specs/plans/welcome_screen_project_creation_redesign.md section 'Task 3'",
      "Modify src/hooks/useAppKeyboardShortcuts.ts",
      "Add openProjectWizard callback to hook props interface",
      "Add hasProjects boolean to hook props interface",
      "Add case for 'n'/'N' key with meta/ctrl: shift → always open, no shift → only on welcome screen",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(shortcuts): add ⌘N and ⌘⇧N for project creation"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Fix Create Project modal close behavior (ESC hint, X button visibility)",
    "plan_section": "Task 4: Fix Create Project Modal Close Behavior",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(wizard): add ESC hint and verify close button visibility",
    "steps": [
      "Read specs/plans/welcome_screen_project_creation_redesign.md section 'Task 4'",
      "Modify src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx",
      "Verify hideCloseButton={isFirstRun} shows X when isFirstRun=false",
      "Add ESC key hint in footer when isFirstRun=false ('Press ESC to cancel')",
      "Verify onClose is properly wired to escape key handling",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(wizard): add ESC hint and verify close button visibility"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Integrate WelcomeScreen into App.tsx and wire keyboard shortcuts",
    "plan_section": "Task 5: Integrate WelcomeScreen into App.tsx",
    "blocking": [],
    "blockedBy": [1, 2, 3],
    "atomic_commit": "feat(app): integrate WelcomeScreen and keyboard shortcuts",
    "steps": [
      "Read specs/plans/welcome_screen_project_creation_redesign.md section 'Task 5'",
      "Import WelcomeScreen component in src/App.tsx",
      "Replace current empty state (hasNoProjects block) with <WelcomeScreen onCreateProject={handleOpenProjectWizard} />",
      "Update useAppKeyboardShortcuts call to pass openProjectWizard and hasProjects props",
      "Verify keyboard shortcuts work correctly in both contexts",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(app): integrate WelcomeScreen and keyboard shortcuts"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **CSS-only animations** | Matches existing patterns, no external libraries like framer-motion |
| **Co-located <style> tag** | Animations owned by component, not parent injection |
| **20-30 particles limit** | Performance constraint for smooth 60fps animations |
| **Conditional keyboard shortcuts** | ⌘N context-aware (welcome only), ⌘⇧N always available |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] WelcomeScreen renders without errors
- [ ] Keyboard shortcuts register correctly

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Launch app with no projects → welcome screen appears with animations
- [ ] Verify animations are smooth (60fps target)
- [ ] From welcome screen, press ⌘N → modal opens
- [ ] From any view with projects, press ⌘⇧N → modal opens
- [ ] In modal (non-first-run), press ESC → modal closes
- [ ] In modal (non-first-run), click X button → modal closes
- [ ] In modal (first-run), ESC and backdrop click should NOT close

### Design Compliance
- [ ] No purple/blue gradients present
- [ ] Orange accent (#ff6b35) used sparingly
- [ ] SF Pro font used (not Inter)
- [ ] Layered shadows present
- [ ] Glass effects with backdrop-blur working

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] WelcomeScreen is imported AND rendered in App.tsx (not behind disabled flag)
- [ ] onCreateProject prop properly wired to handleOpenProjectWizard
- [ ] Keyboard shortcuts trigger correct callbacks
- [ ] TerminalCanvas and ParticleField imported AND rendered in WelcomeScreen

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
