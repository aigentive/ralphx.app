# Multi-Stream RALPH Orchestration

Parallel stream execution with tmux and zero-idle API usage via fswatch.

## Prerequisites

```bash
brew install tmux fswatch
```

Verify installation:
```bash
tmux -V        # Should show 3.x+
fswatch --version
```

## Quick Start

```bash
# Start all streams (creates tmux session, detaches)
./ralph-tmux.sh

# Attach to watch progress
./ralph-tmux.sh attach

# Check status without attaching
./ralph-tmux.sh status

# Stop all streams gracefully
./ralph-tmux.sh stop

# Restart a single stream
./ralph-tmux.sh restart features

# Restart all streams
./ralph-tmux.sh restart
```

## Pane Layout

```
┌─────────────────────────────────────────────────────────────┐
│ [0] STATUS (keybindings, refresh info)                      │
├───────────────────────────────────┬─────────────────────────┤
│                                   │ [2] REFACTOR (sonnet)   │
│                                   │ P1 large file splits    │
│                                   ├─────────────────────────┤
│ [1] FEATURES (opus)               │ [3] POLISH (sonnet)     │
│ PRD tasks + P0 gap fixes          │ P2/P3 cleanup           │
│ Main stream - 60% width           ├─────────────────────────┤
│                                   │ [4] VERIFY (sonnet)     │
│                                   │ Gap detection           │
│                                   ├─────────────────────────┤
│                                   │ [5] HYGIENE (sonnet)    │
│                                   │ Backlog maintenance     │
└───────────────────────────────────┴─────────────────────────┘
```

## Tmux Key Bindings

| Action | Keys | Description |
|--------|------|-------------|
| Detach | `Ctrl+b d` | Exit tmux, streams keep running |
| Switch pane | `Ctrl+b 0-5` | Jump to pane AND zoom (0=status, 1-5=streams) |
| Next pane | `Ctrl+b o` | Cycle through panes |
| Scroll mode | `Ctrl+b [` | View history (arrows/PgUp to scroll, `q` to exit) |
| Zoom toggle | `Ctrl+b z` | Full-screen current pane (toggle) |
| Graceful stop | `Ctrl+b S` | Stop after current tasks complete |

> **Note:** `Ctrl+b <number>` switches to pane AND zooms it automatically. Press `Ctrl+b z` to unzoom.

## Streams

| Stream | Model | Purpose | Watches |
|--------|-------|---------|---------|
| **features** | opus | PRD tasks, P0 gap fixes | `backlog.md`, `manifest.json` |
| **refactor** | sonnet | P1 large file splits | `backlog.md` |
| **polish** | sonnet | P2/P3 cleanup, type fixes | `backlog.md` |
| **verify** | sonnet | Gap detection in completed phases | `manifest.json` |
| **hygiene** | sonnet | Backlog maintenance, refilling | All backlogs (10min delay) |

## fswatch Behavior

Streams use file watching instead of polling. **Zero API calls when idle.**

### Lifecycle

1. Stream starts → runs initial cycle
2. Work found → executes task → commits → continues
3. No work (IDLE) → exits cleanly
4. fswatch waits for file change
5. File changes → runs new cycle → repeat

### What Triggers Each Stream

| Stream | Triggered By |
|--------|--------------|
| **features** | P0 added to backlog, phase completed, manifest change |
| **refactor** | Items added to refactor backlog by hygiene |
| **polish** | Items added to polish backlog by hygiene |
| **verify** | Phase marked complete in manifest |
| **hygiene** | Any backlog change (10-minute debounce) |

### Manual Trigger

```bash
# Trigger features stream
touch streams/features/backlog.md

# Trigger verify stream
touch specs/manifest.json
```

## Daily Workflow

### Morning

```bash
./ralph-tmux.sh        # Start all streams
./ralph-tmux.sh attach # Watch for a few minutes
Ctrl+b d               # Detach, let it run
```

### During Day

```bash
./ralph-tmux.sh status # Quick check without attaching
./ralph-tmux.sh attach # Check detailed progress
```

### End of Day

```bash
./ralph-tmux.sh stop   # Graceful shutdown
```

## Troubleshooting

### Stream Crashed

```bash
./ralph-tmux.sh restart features  # Restart single stream
```

### All Streams Stuck

```bash
./ralph-tmux.sh stop
./ralph-tmux.sh
```

### Can't Attach (No Session)

```bash
./ralph-tmux.sh status  # Check if session exists
./ralph-tmux.sh         # Start fresh
```

### Session Exists But Can't Create

```bash
./ralph-tmux.sh stop    # Kill existing session
./ralph-tmux.sh         # Start fresh
```

## File Structure

```
streams/
├── README.md              # This file
├── features/
│   ├── PROMPT.md          # Stream prompt
│   ├── backlog.md         # P0 items (gaps)
│   └── activity.md        # Activity log
├── refactor/
│   ├── PROMPT.md
│   ├── backlog.md         # P1 items (large files)
│   └── activity.md
├── polish/
│   ├── PROMPT.md
│   ├── backlog.md         # P2/P3 items (cleanup)
│   └── activity.md
├── verify/
│   ├── PROMPT.md
│   └── activity.md
├── hygiene/
│   ├── PROMPT.md
│   └── activity.md
└── archive/
    └── completed.md       # Archived completed items

scripts/
├── stream-watch-features.sh
├── stream-watch-refactor.sh
├── stream-watch-polish.sh
├── stream-watch-verify.sh
└── stream-watch-hygiene.sh
```

## Related Files

- `ralph-tmux.sh` - Main launcher
- `ralph-tmux-status.sh` - Header status display
- `ralph-streams.sh` - Stream runner
- `.claude/rules/stream-*.md` - Stream workflow rules
