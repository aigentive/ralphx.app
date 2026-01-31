# RalphX - Phase 42: Welcome Screen Redesign

## Overview

Replace the current terminal-aesthetic welcome screen with a visually stunning "Agent Constellation" design that immediately communicates AI orchestration and multi-agent power. The new design features an animated agent network with 4 orbiting nodes (Orchestrator, Worker, QA, Reviewer) around a pulsing central hub, connected by glowing paths with traveling data particles.

This phase transforms the first impression of RalphX from "CLI tool" to "premium AI orchestration GUI" through dramatic animations, code rain backgrounds, and interactive visual elements.

**Reference Plan:**
- `specs/plans/welcome_screen_redesign_plan.md` - Detailed implementation plan with design concept, animation patterns, and component structure

## Goals

1. **Visual Impact** - Create a "wow factor" that immediately communicates AI-powered orchestration
2. **Agent Visualization** - Display 4 agent types (Orchestrator, Worker, QA, Reviewer) as animated nodes
3. **Premium Motion** - Implement smooth framer-motion animations with entry bursts, breathing glows, and hover effects
4. **Design System Compliance** - Use warm orange `#ff6b35` accent, no purple/blue gradients, SF Pro font

## Dependencies

### Phase 35 (Welcome Screen & Project Creation Redesign) - Foundation

| Dependency | Why Needed |
|------------|------------|
| WelcomeScreen component | Base component to redesign |
| Props API (isOverlay, onClose, onStartProject) | Must maintain existing API |
| Keyboard shortcuts (⌘N, ⌘⇧W, Escape) | Must preserve functionality |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/welcome_screen_redesign_plan.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
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
2. **Read the ENTIRE implementation plan** at `specs/plans/welcome_screen_redesign_plan.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Install framer-motion dependency",
    "plan_section": "Task 1: Install framer-motion dependency",
    "blocking": [2, 3, 4, 7],
    "blockedBy": [],
    "atomic_commit": "feat(deps): add framer-motion for welcome screen animations",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 1'",
      "Run: npm install framer-motion",
      "Verify framer-motion appears in package.json dependencies",
      "Run npm run typecheck to verify no type errors",
      "Commit: feat(deps): add framer-motion for welcome screen animations"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Create CodeRain background component with drifting code fragments",
    "plan_section": "Task 2: Create CodeRain background component",
    "blocking": [8],
    "blockedBy": [1],
    "atomic_commit": "feat(welcome): create CodeRain background component",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 2' and 'Code Rain Background (INTENSE)'",
      "Create src/components/WelcomeScreen/CodeRain.tsx",
      "Implement 40-50 code fragments drifting downward with CSS keyframes",
      "Add parallax depth effect (large/near vs small/far fragments)",
      "Add varied speeds for depth perception",
      "Add occasional orange highlight on random fragments",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): create CodeRain background component"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Create AmbientParticles floating dots component",
    "plan_section": "Task 3: Create AmbientParticles component",
    "blocking": [8],
    "blockedBy": [1],
    "atomic_commit": "feat(welcome): create AmbientParticles floating dots",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 3' and 'Ambient Particles'",
      "Create src/components/WelcomeScreen/AmbientParticles.tsx",
      "Implement 30-40 tiny particles drifting randomly",
      "Add varied sizes (2px to 6px)",
      "Use color palette: white, orange accent (#ff6b35), agent colors at low opacity",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): create AmbientParticles floating dots"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Create CentralHub pulsing core with ripple rings",
    "plan_section": "Task 4: Create CentralHub component",
    "blocking": [5, 8],
    "blockedBy": [1],
    "atomic_commit": "feat(welcome): create CentralHub pulsing core component",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 4' and 'Central Hub (Command Center)'",
      "Create src/components/WelcomeScreen/CentralHub.tsx",
      "Implement pulsing core in center with glowing warm orange (#ff6b35)",
      "Add concentric animated rings rippling outward (sonar effect)",
      "Use Framer Motion for scale + opacity animations",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): create CentralHub pulsing core component"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Create ConnectionPaths SVG lines with glow effect",
    "plan_section": "Task 5: Create ConnectionPaths SVG component",
    "blocking": [6, 8],
    "blockedBy": [4],
    "atomic_commit": "feat(welcome): create ConnectionPaths SVG lines with glow",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 5' and 'Connection Lines with Heavy Data Flow'",
      "Create src/components/WelcomeScreen/ConnectionPaths.tsx",
      "Implement SVG paths connecting all agent positions through central hub",
      "Add soft glow effect on lines using SVG filter or CSS",
      "Accept agent positions as props for dynamic path generation",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): create ConnectionPaths SVG lines with glow"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Create DataPulse particles traveling along connection paths",
    "plan_section": "Task 6: Create DataPulse particles component",
    "blocking": [8],
    "blockedBy": [5],
    "atomic_commit": "feat(welcome): create DataPulse traveling particles",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 6' and 'Connection Lines with Heavy Data Flow'",
      "Create src/components/WelcomeScreen/DataPulse.tsx",
      "Implement multiple particles per path (5-8 simultaneously)",
      "Add particle trails (fading tail effect)",
      "Add variable speeds (fast and slow particles)",
      "Implement bidirectional flow on each path",
      "Use CSS offset-path animation for performance",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): create DataPulse traveling particles"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Create AgentNode component with glow and hover effects",
    "plan_section": "Task 7: Create AgentNode component",
    "blocking": [8],
    "blockedBy": [1],
    "atomic_commit": "feat(welcome): create AgentNode with glow and hover",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 7' and 'Agent Network (Hero Visual)'",
      "Create src/components/WelcomeScreen/AgentNode.tsx",
      "Implement icon + label display using Lucide icons (Brain, Code2, ShieldCheck, Eye)",
      "Add breathing glow animation (scale + box-shadow pulse) with Framer Motion",
      "Add dramatic hover effect (scale 1.25 + intense glow)",
      "Use spring physics on hover transitions",
      "Accept agent config as props (id, name, role, icon, color)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): create AgentNode with glow and hover"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Create AgentConstellation main orchestrator component",
    "plan_section": "Task 8: Create AgentConstellation orchestrator component",
    "blocking": [9],
    "blockedBy": [2, 3, 4, 5, 6, 7],
    "atomic_commit": "feat(welcome): create AgentConstellation main orchestrator",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 8' and 'Agent Configuration'",
      "Create src/components/WelcomeScreen/AgentConstellation.tsx",
      "Compose all visual elements: CodeRain, AmbientParticles, CentralHub, ConnectionPaths, DataPulse, AgentNode",
      "Implement AGENTS configuration array with 4 agents (Orchestrator, Worker, QA, Reviewer)",
      "Add staggered node entrance animation (fly in from edges with spring physics)",
      "Implement mouse parallax effect on entire scene",
      "Ensure proper layering: background → connections → hub → nodes → particles",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): create AgentConstellation main orchestrator"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Update WelcomeScreen to use AgentConstellation",
    "plan_section": "Task 9: Update WelcomeScreen to use AgentConstellation",
    "blocking": [10],
    "blockedBy": [8],
    "atomic_commit": "feat(welcome): integrate AgentConstellation into WelcomeScreen",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 9'",
      "Read current src/components/WelcomeScreen/WelcomeScreen.tsx",
      "Replace TerminalCanvas with AgentConstellation",
      "Update title to 'RalphX' with accent styling on 'X', subtitle 'Watch AI Build Your Software'",
      "Keep existing props API (isOverlay, onClose, onStartProject)",
      "Keep primary CTA: 'Start Your First Project' button",
      "Add keyboard hint ⌘N with idle pulse animation (3+ seconds)",
      "Ensure close button (X) only appears in overlay mode",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(welcome): integrate AgentConstellation into WelcomeScreen"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "frontend",
    "description": "Delete deprecated TerminalCanvas and ParticleField components",
    "plan_section": "Task 10: Delete deprecated components and update exports",
    "blocking": [11],
    "blockedBy": [9],
    "atomic_commit": "refactor(welcome): remove deprecated TerminalCanvas and ParticleField",
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 10'",
      "Delete src/components/WelcomeScreen/TerminalCanvas.tsx",
      "Delete src/components/WelcomeScreen/ParticleField.tsx",
      "Update src/components/WelcomeScreen/index.tsx if needed (should still export WelcomeScreen)",
      "Verify no import errors in the codebase",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(welcome): remove deprecated TerminalCanvas and ParticleField"
    ],
    "passes": false
  },
  {
    "id": 11,
    "category": "frontend",
    "description": "Visual and functional verification of complete welcome screen",
    "plan_section": "Task 11: Visual and functional verification",
    "blocking": [],
    "blockedBy": [10],
    "atomic_commit": null,
    "steps": [
      "Read specs/plans/welcome_screen_redesign_plan.md section 'Task 11: Visual and functional verification'",
      "Start dev server if not running: npm run tauri dev",
      "VISUAL: Verify nodes fly in dramatically from edges on mount",
      "VISUAL: Verify central hub pulses with ripple rings emanating",
      "VISUAL: Verify 4 agent nodes have glowing halos",
      "VISUAL: Verify connection paths visible with soft glow",
      "VISUAL: Verify multiple particles traveling along paths",
      "VISUAL: Verify code rain (40-50 fragments drifting)",
      "VISUAL: Verify ambient floating dots throughout scene",
      "VISUAL: Verify node hover scales up with intense glow burst",
      "VISUAL: Verify mouse parallax shifts scene subtly",
      "FUNCTIONAL: Click 'Start Your First Project' → wizard opens",
      "FUNCTIONAL: Press ⌘N → same behavior (first-run state)",
      "FUNCTIONAL: Press ⌘⇧W (with existing projects) → overlay toggles",
      "FUNCTIONAL: Press Escape on overlay → closes correctly",
      "FUNCTIONAL: Close button (X) appears only on overlay mode",
      "PERFORMANCE: Verify smooth 60fps - no jank on animations",
      "DESIGN: Confirm warm orange #ff6b35 is primary accent (not purple/blue)",
      "DESIGN: Confirm SF Pro font (not Inter)",
      "Report any issues found or mark task as verified"
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
| **Framer Motion for complex animations** | Production-ready animation library with spring physics, gesture support, and performance optimizations |
| **CSS keyframes for high-frequency elements** | Code rain and data pulses use CSS for better performance with many simultaneous animations |
| **Layered component composition** | Background → connections → hub → nodes → particles for proper visual depth |
| **AGENTS configuration array** | Centralized config for agent data (id, name, role, icon, color, position) enables easy modification |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] WelcomeScreen renders without errors
- [ ] AgentConstellation renders all 4 agent nodes
- [ ] Props API maintained (isOverlay, onClose, onStartProject)

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Entry animation: Nodes fly in dramatically from edges
- [ ] Central hub: Pulsing core with ripple rings emanating
- [ ] Agent nodes: 4 nodes with glowing halos, hover works
- [ ] Connection paths: Visible lines with soft glow between nodes
- [ ] Data pulses: Multiple particles traveling along each path
- [ ] Code rain: Dense code fragments drifting (40-50 fragments)
- [ ] Ambient particles: Floating dots throughout scene
- [ ] Mouse parallax: Scene shifts subtly with mouse movement
- [ ] Click "Start Your First Project" → wizard opens
- [ ] Press ⌘N → same behavior (first-run state)
- [ ] Press ⌘⇧W (with existing projects) → overlay toggles
- [ ] Press Escape on overlay → closes correctly
- [ ] Smooth 60fps - no jank on animations

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] AgentConstellation is imported AND rendered in WelcomeScreen
- [ ] All sub-components (CodeRain, CentralHub, etc.) are rendered
- [ ] No optional props defaulting to `false` or disabled
- [ ] TerminalCanvas and ParticleField fully removed (no dead imports)

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
