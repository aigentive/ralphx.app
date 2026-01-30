# Welcome Screen Redesign Plan

## Problem
Current welcome screen has a **terminal aesthetic** that:
- Makes RalphX look like a CLI tool (shows `ralphx init --agent orchestrator` command)
- Doesn't convey AI orchestration and multi-agent power visually
- Lacks the interactive "wow factor" that communicates intelligence

## Goal
Create a visually stunning, interactive welcome screen that immediately communicates:
- **AI-powered** - intelligent autonomous agents working together
- **Orchestration** - multiple agents coordinating on tasks
- **Visual** - this is a premium GUI app, not a terminal
- **Powerful** - capable of autonomous software development

---

## Design Concept: "Agent Constellation"

### Visual Metaphor
An animated **agent network** showing the orchestration flow:
- **Center Hub** = User's project (the heart)
- **Agent Nodes** = 4 agents orbiting the center (Orchestrator, Worker, QA, Reviewer)
- **Connection Lines** = Animated paths showing task flow
- **Data Pulses** = Particles traveling along connections representing activity

### Layout
```
┌──────────────────────────────────────────────────────────────┐
│  [Code Rain Background - subtle, sophisticated]              │
│                                                              │
│                     ┌─────────────────┐                      │
│                     │   Ralph*X*      │ ← Title with accent  │
│                     │ Watch AI Build  │                      │
│                     │  Your Software  │                      │
│                     └─────────────────┘                      │
│                                                              │
│           [Worker]                     [QA]                  │
│              ○─────────────────────────○                     │
│              │\                       /│                     │
│              │ \       [Hub]        / │                      │
│              │  \        ●        /   │                      │
│              │   \      / \     /     │                      │
│              │    \   /    \  /       │                      │
│              ○──────●──────●──────────○                      │
│         [Orchestrator]          [Reviewer]                   │
│                                                              │
│              ┌─────────────────────────────┐                 │
│              │ Start Your First Project   │ ← Primary CTA    │
│              └─────────────────────────────┘                 │
│                         ⌘N                                   │
└──────────────────────────────────────────────────────────────┘
```

### Core Elements (DRAMATIC VERSION)

#### 1. **Agent Network** (Hero Visual)
4 agent nodes arranged in a diamond pattern around a **glowing central hub**:

| Agent | Position | Icon | Color | Role |
|-------|----------|------|-------|------|
| Orchestrator | Bottom-left | Brain | `#ff6b35` (accent) | Plans & coordinates |
| Worker | Top-left | Code2 | `#4ade80` (green) | Writes code |
| QA | Top-right | ShieldCheck | `#60a5fa` (blue) | Validates quality |
| Reviewer | Bottom-right | Eye | `#f59e0b` (amber) | Reviews changes |

**Animations (ENHANCED):**
- **Orbital rotation**: Nodes slowly orbit around center (30s full rotation)
- **Breathing glow**: Nodes pulse with glowing halos (scale + box-shadow)
- **Entry burst**: On mount, nodes fly in from edges with trails
- **Hover**: Node zooms + bright glow + connection paths intensify

#### 2. **Central Hub (Command Center)**
- Pulsing core in the center representing "your project"
- Concentric animated rings rippling outward (like sonar)
- Glowing warm orange core with subtle particle emission
- All connections flow through this hub

#### 3. **Connection Lines with Heavy Data Flow**
- **Multiple particles per path** (5-8 particles traveling simultaneously)
- **Particle trails**: Each particle leaves a fading tail
- **Variable speeds**: Some particles fast (urgent), some slow (background)
- **Bidirectional flow**: Particles travel both ways on each path
- **Glow effect**: Lines themselves have soft glow matching particle color

#### 4. **Code Rain Background (INTENSE)**
- **40-50 code fragments** (doubled from subtle version)
- **Multiple depths**: Some large/near, some small/far (parallax)
- **Varied speeds**: Fast + slow fragments for depth perception
- **Occasional highlight**: Random fragments briefly glow orange
- **Code snippets** (more variety):
  - `agent.spawn('worker')`
  - `await orchestrate()`
  - `task.complete()`
  - `review.approve()`
  - `{ status: 'executing' }`
  - `pipeline.next()`
  - `commit.push()`

#### 5. **Ambient Particles**
- **Floating dots**: 30-40 tiny particles drifting randomly
- **Varied sizes**: 2px to 6px
- **Color palette**: White, orange accent, agent colors at low opacity
- **Connection sparks**: Occasional bright sparks where particles meet lines

#### 6. **Interactive Elements**
- **Mouse parallax**: Entire scene shifts subtly with mouse movement
- **Node hover**: Dramatic zoom + glow burst + particles attracted to cursor
- **Click ripple**: Clicking anywhere creates a ripple effect
- **Keyboard hint glow**: `⌘N` text pulses when idle for 3+ seconds

---

## Technical Implementation

### Dependencies
**Add:** `framer-motion` (not currently installed)
- Handles complex animation orchestration
- Built-in gesture support (hover, tap, drag)
- Performance-optimized transforms

### File Structure
```
src/components/WelcomeScreen/
├── WelcomeScreen.tsx          # Main container (keep existing props API)
├── AgentConstellation.tsx     # Main orchestration - all visual elements
├── AgentNode.tsx              # Individual agent node with glow/hover
├── CentralHub.tsx             # Pulsing center with ripple rings
├── ConnectionPaths.tsx        # SVG paths with glow effect
├── DataPulse.tsx              # Particles traveling along paths
├── CodeRain.tsx               # Dense background code fragments
├── AmbientParticles.tsx       # Floating random particles
├── index.tsx                  # Barrel export (unchanged)
```

**DELETE:**
- `TerminalCanvas.tsx` - replaced by AgentConstellation
- `ParticleField.tsx` - replaced by CodeRain + AmbientParticles

### Agent Configuration
```typescript
const AGENTS = [
  {
    id: 'orchestrator',
    name: 'Orchestrator',
    role: 'Plans & coordinates work',
    icon: Brain,
    color: '#ff6b35',
    position: { x: 25, y: 70 }  // % from center
  },
  {
    id: 'worker',
    name: 'Worker',
    role: 'Writes code',
    icon: Code2,
    color: '#4ade80',
    position: { x: 25, y: 30 }
  },
  {
    id: 'qa',
    name: 'QA Refiner',
    role: 'Validates quality',
    icon: ShieldCheck,
    color: '#60a5fa',
    position: { x: 75, y: 30 }
  },
  {
    id: 'reviewer',
    name: 'Reviewer',
    role: 'Reviews changes',
    icon: Eye,
    color: '#f59e0b',
    position: { x: 75, y: 70 }
  },
]
```

### Animation Approach

**Using Framer Motion for:**
1. Node entrance (staggered fade + scale in)
2. Node breathing (infinite scale animation)
3. Hover interactions (scale, glow)
4. Tooltip appearance (fade + slide)

**Using CSS for:**
1. Code rain drift (simple translateY keyframes)
2. Data pulse travel along paths (CSS offset-path + animation)
3. Glow effects (box-shadow animation)

### Key Framer Motion Patterns

```tsx
// DRAMATIC node entrance - fly in from edges with overshoot
<motion.div
  initial={{
    opacity: 0,
    scale: 0,
    x: originOffscreen.x,  // Start off-screen
    y: originOffscreen.y
  }}
  animate={{
    opacity: 1,
    scale: 1,
    x: finalPosition.x,
    y: finalPosition.y
  }}
  transition={{
    type: "spring",
    stiffness: 100,
    damping: 10,
    delay: index * 0.2
  }}
/>

// Orbital rotation around center
<motion.div
  animate={{ rotate: 360 }}
  transition={{
    duration: 30,
    repeat: Infinity,
    ease: "linear"
  }}
  style={{ transformOrigin: "center center" }}
/>

// Pulsing glow effect (scale + shadow)
<motion.div
  animate={{
    scale: [1, 1.08, 1],
    boxShadow: [
      "0 0 20px rgba(255,107,53,0.3)",
      "0 0 40px rgba(255,107,53,0.6)",
      "0 0 20px rgba(255,107,53,0.3)"
    ]
  }}
  transition={{
    duration: 2,
    repeat: Infinity,
    ease: "easeInOut"
  }}
/>

// Hover with spring physics
<motion.div
  whileHover={{
    scale: 1.25,
    boxShadow: "0 0 60px rgba(255,107,53,0.8)"
  }}
  transition={{ type: "spring", stiffness: 400, damping: 15 }}
/>

// Central hub ripple rings
<motion.div
  animate={{
    scale: [1, 2.5],
    opacity: [0.6, 0]
  }}
  transition={{
    duration: 2,
    repeat: Infinity,
    ease: "easeOut"
  }}
/>

// Mouse parallax effect
const { scrollYProgress } = useScroll()
const x = useTransform(mouseX, [0, window.innerWidth], [-20, 20])
const y = useTransform(mouseY, [0, window.innerHeight], [-20, 20])
```

### CSS Animations (for high-frequency elements)

```css
/* Data pulse traveling along path - using CSS offset-path */
@keyframes travelPath {
  0% { offset-distance: 0%; opacity: 0; }
  5% { opacity: 1; }
  95% { opacity: 1; }
  100% { offset-distance: 100%; opacity: 0; }
}

.data-pulse {
  offset-path: path("M 100,100 Q 150,50 200,100");
  animation: travelPath 3s linear infinite;
}

/* Code rain drift with varied speeds */
@keyframes codeDrift {
  0% { transform: translateY(-100%) translateX(0); opacity: 0; }
  10% { opacity: var(--fragment-opacity); }
  90% { opacity: var(--fragment-opacity); }
  100% { transform: translateY(100vh) translateX(20px); opacity: 0; }
}

/* Particle glow pulse */
@keyframes particleGlow {
  0%, 100% { filter: blur(0px); opacity: 0.4; }
  50% { filter: blur(2px); opacity: 0.8; }
}
```

---

## Implementation Tasks

### Task 1: Install framer-motion dependency (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(deps): add framer-motion for welcome screen animations`

**Files:**
- `package.json` - EDIT - Add `framer-motion` dependency

**Steps:**
1. Run `npm install framer-motion`
2. Verify installation in package.json

---

### Task 2: Create CodeRain background component (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(welcome): create CodeRain background component`

**Files:**
- NEW: `src/components/WelcomeScreen/CodeRain.tsx` - CREATE

**Acceptance Criteria:**
- 40-50 code fragments drifting downward
- Multiple depths (parallax effect)
- Varied speeds for depth perception
- Occasional orange highlight on random fragments
- CSS keyframe animations for performance

---

### Task 3: Create AmbientParticles component
**Dependencies:** Task 1
**Atomic Commit:** `feat(welcome): create AmbientParticles floating dots`

**Files:**
- NEW: `src/components/WelcomeScreen/AmbientParticles.tsx` - CREATE

**Acceptance Criteria:**
- 30-40 tiny particles drifting randomly
- Varied sizes (2px to 6px)
- Color palette: white, orange accent, agent colors at low opacity
- Smooth random movement

---

### Task 4: Create CentralHub component (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(welcome): create CentralHub pulsing core component`

**Files:**
- NEW: `src/components/WelcomeScreen/CentralHub.tsx` - CREATE

**Acceptance Criteria:**
- Pulsing core in center
- Concentric animated rings rippling outward (sonar effect)
- Glowing warm orange core (`#ff6b35`)
- Framer Motion animations

---

### Task 5: Create ConnectionPaths SVG component (BLOCKING)
**Dependencies:** Task 4
**Atomic Commit:** `feat(welcome): create ConnectionPaths SVG lines with glow`

**Files:**
- NEW: `src/components/WelcomeScreen/ConnectionPaths.tsx` - CREATE

**Acceptance Criteria:**
- SVG paths connecting all agent positions through central hub
- Soft glow effect on lines
- Accepts agent positions as props

---

### Task 6: Create DataPulse particles component
**Dependencies:** Task 5
**Atomic Commit:** `feat(welcome): create DataPulse traveling particles`

**Files:**
- NEW: `src/components/WelcomeScreen/DataPulse.tsx` - CREATE

**Acceptance Criteria:**
- Multiple particles per path (5-8 simultaneously)
- Particle trails (fading tail)
- Variable speeds (fast + slow)
- Bidirectional flow on each path
- CSS offset-path animation

---

### Task 7: Create AgentNode component (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(welcome): create AgentNode with glow and hover`

**Files:**
- NEW: `src/components/WelcomeScreen/AgentNode.tsx` - CREATE

**Acceptance Criteria:**
- Icon + label display
- Breathing glow animation (scale + box-shadow pulse)
- Dramatic hover effect (scale 1.25 + intense glow)
- Spring physics on hover
- Accepts agent config as props (id, name, role, icon, color)

---

### Task 8: Create AgentConstellation orchestrator component (BLOCKING)
**Dependencies:** Task 2, Task 3, Task 4, Task 5, Task 6, Task 7
**Atomic Commit:** `feat(welcome): create AgentConstellation main orchestrator`

**Files:**
- NEW: `src/components/WelcomeScreen/AgentConstellation.tsx` - CREATE

**Acceptance Criteria:**
- Composes all visual elements (CodeRain, AmbientParticles, CentralHub, ConnectionPaths, DataPulse, AgentNode)
- Staggered node entrance animation (fly in from edges)
- Mouse parallax effect on entire scene
- AGENTS configuration array
- Proper layering (background → connections → hub → nodes → particles)

---

### Task 9: Update WelcomeScreen to use AgentConstellation (BLOCKING)
**Dependencies:** Task 8
**Atomic Commit:** `feat(welcome): integrate AgentConstellation into WelcomeScreen`

**Files:**
- `src/components/WelcomeScreen/WelcomeScreen.tsx` - REWRITE

**Acceptance Criteria:**
- Replace TerminalCanvas with AgentConstellation
- New title: "Ralph**X**" with accent on X, "Watch AI Build Your Software"
- Keep existing props API (isOverlay, onClose, onStartProject)
- Primary CTA: "Start Your First Project" button
- Keyboard hint: ⌘N with idle pulse animation
- Close button (X) only in overlay mode

---

### Task 10: Delete deprecated components and update exports
**Dependencies:** Task 9
**Atomic Commit:** `refactor(welcome): remove deprecated TerminalCanvas and ParticleField`

**Files:**
- `src/components/WelcomeScreen/TerminalCanvas.tsx` - DELETE
- `src/components/WelcomeScreen/ParticleField.tsx` - DELETE
- `src/components/WelcomeScreen/index.tsx` - EDIT (if needed)

**Acceptance Criteria:**
- Old components removed
- No import errors
- Barrel export unchanged (still exports WelcomeScreen)

---

### Task 11: Visual and functional verification
**Dependencies:** Task 10
**Atomic Commit:** None (verification only)

**Verification Checklist:**

**Visual Quality:**
- [ ] Entry animation: Nodes fly in dramatically from edges
- [ ] Central hub: Pulsing core with ripple rings emanating
- [ ] Agent nodes: 4 nodes with glowing halos
- [ ] Connection paths: Visible lines with soft glow between nodes
- [ ] Data pulses: Multiple particles traveling along each path
- [ ] Code rain: Dense code fragments drifting (40-50 fragments)
- [ ] Ambient particles: Floating dots throughout scene
- [ ] Hover interaction: Node scales up with intense glow burst
- [ ] Mouse parallax: Scene shifts subtly with mouse movement

**Functional:**
- [ ] Click "Start Your First Project" → wizard opens
- [ ] Press `⌘N` → same behavior (first-run state)
- [ ] Press `⌘⇧W` (with existing projects) → overlay toggles
- [ ] Press `Escape` on overlay → closes correctly
- [ ] Close button (X) appears only on overlay mode

**Performance:**
- [ ] Smooth 60fps - no jank on animations
- [ ] No excessive CPU usage

**Design System:**
- [ ] Warm orange `#ff6b35` is primary accent (not purple/blue)
- [ ] Uses SF Pro font (not Inter)
- [ ] Glass effects use backdrop-blur correctly

---

## Design System Compliance

| Requirement | Implementation |
|-------------|----------------|
| Accent color `#ff6b35` | Orchestrator node, X in title, CTA button |
| No purple/blue gradients | Status colors only (green, blue, amber for agents) |
| SF Pro font | Inherited from globals.css |
| Layered shadows | Premium card shadow on nodes |
| Glass effects | Subtle backdrop-blur on node tooltips |
| Premium motion | Smooth easing, subtle transforms |

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
