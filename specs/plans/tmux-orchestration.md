# Phase 24: Tmux-Based Multi-Stream Orchestration

## Overview

Add tmux integration to run all 5 RALPH streams simultaneously in split terminal panes with real-time visibility.

**Dependency:** Phase 23 (Multi-Stream RALPH Architecture) must be complete first.

---

## Your Workflow: Before vs After

### Current Workflow (Single Stream)
```
1. Open terminal
2. Run ./ralph.sh 50
3. Watch single output
4. Can't see other work happening
5. Must run streams sequentially
```

### New Workflow (With Tmux)
```
1. Open iTerm
2. Run ./ralph-tmux.sh
3. See ALL 5 streams running simultaneously in split panes
4. Detach anytime (Ctrl+b d) - streams keep running
5. Reattach later (./ralph-tmux.sh attach) to check progress
6. Stop gracefully when done (./ralph-tmux.sh stop)
```

**Key Benefits:**
- See what each stream is doing in real-time
- Streams run in parallel (faster overall progress)
- Detach/reattach without stopping work
- Easy restart of individual crashed streams
- Clear visual separation between streams

---

## Tmux Installation Guide (macOS + iTerm + ZSH)

### Step 1: Install Homebrew (if not installed)
```bash
# Check if Homebrew is installed
brew --version

# If not installed, run:
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

### Step 2: Install tmux
```bash
brew install tmux
```

### Step 3: Verify Installation
```bash
tmux -V
# Should output: tmux 3.x
```

### Step 4: Basic tmux Test
```bash
# Start a test session
tmux new -s test

# You're now inside tmux (notice the green bar at bottom)
# Press Ctrl+b then d to detach
# Run: tmux attach -t test to reattach
# Run: tmux kill-session -t test to end
```

That's it! tmux works with iTerm and ZSH out of the box.

---

## Pane Layout Design

```
┌─────────────────────────────────────────────────────────────┐
│          RALPH ORCHESTRATOR STATUS (header pane)            │
├──────────────────────────────┬──────────────────────────────┤
│ [1] FEATURES (opus)          │ [2] REFACTOR (sonnet)        │
│                              │                              │
│ Most critical stream         │ P1 large file splits         │
│ PRD tasks + P0 gap fixes     │                              │
├───────────────┬──────────────┼──────────────────────────────┤
│ [3] POLISH    │ [4] VERIFY   │ [5] HYGIENE                  │
│ (sonnet)      │ (sonnet)     │ (sonnet)                     │
│ P2/P3 cleanup │ Gap detect   │ Backlog maintenance          │
└───────────────┴──────────────┴──────────────────────────────┘
```

**Pane Navigation:**
- `Ctrl+b 1` → Jump to features pane
- `Ctrl+b 2` → Jump to refactor pane
- `Ctrl+b 3` → Jump to polish pane
- `Ctrl+b 4` → Jump to verify pane
- `Ctrl+b 5` → Jump to hygiene pane

---

## Essential Tmux Commands

| Action | Keys | When to Use |
|--------|------|-------------|
| **Detach** | `Ctrl+b d` | Leave tmux running, do other work |
| **Switch pane** | `Ctrl+b 1-5` | Jump to specific stream |
| **Next pane** | `Ctrl+b o` | Cycle through panes |
| **Scroll up** | `Ctrl+b [` | View history (arrows/PgUp, `q` to exit) |
| **Zoom pane** | `Ctrl+b z` | Full-screen one pane (toggle) |

---

## Implementation Tasks

### Task 1: Create ralph-tmux.sh launcher
Create main script that:
- Creates tmux session with 6 panes (header + 5 streams)
- Starts each stream with correct model (opus/sonnet)
- Supports: start, attach, stop, restart, status commands

**File:** `ralph-tmux.sh`

### Task 2: Create ralph-tmux-status.sh header
Create status display script for header pane:
- Shows uptime, total iterations
- Shows backlog counts (P0/P1/P2-P3)
- Shows quick-reference commands
- Auto-refreshes every 5 seconds

**File:** `ralph-tmux-status.sh`

### Task 3: Add stream visual differentiation
- Set pane titles (stream name + model)
- Add stream identification to ralph-streams.sh output
- Configure tmux pane borders

### Task 4: Add error handling
- Graceful shutdown (Ctrl+C to each pane)
- Individual stream restart capability
- Exit status preservation in panes

### Task 5: Create user documentation
- Add tmux quick-reference to README or CLAUDE.md
- Document daily workflow commands

---

## Files to Create

| File | Purpose |
|------|---------|
| `ralph-tmux.sh` | Main launcher script |
| `ralph-tmux-status.sh` | Header status display |

## Files to Modify

| File | Changes |
|------|---------|
| `ralph-streams.sh` | Add pane title setting, stream identification in output |
| `CLAUDE.md` | Add tmux usage section |

---

## Daily Usage Commands

```bash
# Start orchestrator (runs in background)
./ralph-tmux.sh

# Attach to watch progress
./ralph-tmux.sh attach

# Check status without attaching
./ralph-tmux.sh status

# Stop all streams gracefully
./ralph-tmux.sh stop

# Restart a crashed stream
./ralph-tmux.sh restart features

# Restart all streams
./ralph-tmux.sh restart
```

---

## Verification

1. **tmux installed:** `tmux -V` shows version
2. **Session creates:** `./ralph-tmux.sh` creates 6-pane layout
3. **Streams run:** Each pane shows stream output
4. **Detach works:** `Ctrl+b d` returns to normal terminal
5. **Reattach works:** `./ralph-tmux.sh attach` shows streams still running
6. **Stop works:** `./ralph-tmux.sh stop` gracefully terminates all
7. **Restart works:** `./ralph-tmux.sh restart features` restarts single stream

---

## Phase 23 Dependency

This phase requires these Phase 23 tasks to be complete:
- Tasks 2-6: Stream rule files (`.claude/rules/stream-*.md`)
- Tasks 7-8: Stream PROMPT.md and activity.md files
- Task 9: Migration of code-quality.md to stream backlogs
- **Task 10: ralph-streams.sh with stream argument support** (critical)

Phase 23 task 11 (`ralph-orchestrator.sh` for sequential execution) becomes optional - tmux replaces it with parallel execution.
