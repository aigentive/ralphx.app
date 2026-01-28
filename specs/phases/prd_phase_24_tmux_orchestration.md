# RalphX - Phase 24: Tmux-Based Multi-Stream Orchestration

## Overview

This phase adds tmux integration to run all 5 RALPH streams simultaneously in split terminal panes with real-time visibility. Uses `fswatch` for file-based triggering - streams only run when work exists, resulting in zero wasted API calls when idle.

**Reference Plan:**
- `specs/plans/tmux-orchestration.md` - Detailed architecture with pane layout, fswatch integration, and script implementations

## Goals

1. Install and configure tmux for multi-pane stream visibility
2. Create `ralph-tmux.sh` launcher with start/stop/attach/restart commands
3. Create fswatch-based wrapper scripts for each stream (zero idle API calls)
4. Add IDLE detection to stream workflows (exit cleanly when no work)
5. Create status header pane showing aggregate orchestrator state
6. Document tmux usage for daily workflow

## Dependencies

### Phase 23 (Multi-Stream RALPH Architecture) - Completed

Required components from Phase 23:
- `streams/*/PROMPT.md` - Stream-specific prompts
- `streams/*/backlog.md` - Stream backlogs to watch
- `ralph-streams.sh` - Stream-aware runner with model selection
- `.claude/rules/stream-*.md` - Stream workflow definitions

### External Dependencies

- **tmux** - Terminal multiplexer (`brew install tmux`)
- **fswatch** - File system watcher (`brew install fswatch`)

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/tmux-orchestration.md`
2. Understand the pane layout and fswatch architecture
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Test the script/functionality
4. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/tmux-orchestration.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "infrastructure",
    "description": "Verify prerequisites (tmux and fswatch installed)",
    "plan_section": "Tmux Installation Guide",
    "steps": [
      "Check tmux is installed: tmux -V",
      "Check fswatch is installed: fswatch --version",
      "If either missing, document installation commands in activity log",
      "Create scripts/ directory if it doesn't exist",
      "Commit: chore(tmux): verify prerequisites and create scripts directory"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Create ralph-tmux.sh main launcher",
    "plan_section": "Implementation Tasks - Task 1",
    "steps": [
      "Read specs/plans/tmux-orchestration.md for launcher design",
      "Create ralph-tmux.sh with:",
      "  - SESSION_NAME='ralph'",
      "  - check_tmux() function to verify tmux installed",
      "  - create_session() function to create 6-pane layout:",
      "    - Pane 0: header (status)",
      "    - Pane 1: features (top-left)",
      "    - Pane 2: refactor (top-right)",
      "    - Pane 3: polish (bottom-left)",
      "    - Pane 4: verify (bottom-center)",
      "    - Pane 5: hygiene (bottom-right)",
      "  - Enable mouse support: tmux set-option mouse on",
      "  - Set history limit: tmux set-option history-limit 50000",
      "  - Subcommands: start (default), attach, stop, restart, status",
      "  - attach_session() function",
      "  - stop_all() function (send Ctrl+C to each pane, then kill session)",
      "  - show_status() function",
      "Make executable: chmod +x ralph-tmux.sh",
      "Test syntax: bash -n ralph-tmux.sh",
      "Commit: feat(tmux): create ralph-tmux.sh launcher with pane layout"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Create fswatch wrapper for features stream",
    "plan_section": "File Watcher Architecture",
    "steps": [
      "Read specs/plans/tmux-orchestration.md for fswatch design",
      "Create scripts/stream-watch-features.sh with:",
      "  - STREAM='features'",
      "  - MODEL='opus'",
      "  - WATCH_FILES='streams/features/backlog.md specs/manifest.json'",
      "  - Echo stream name and status on start",
      "  - Run initial cycle: ANTHROPIC_MODEL=$MODEL ./ralph-streams.sh $STREAM 50",
      "  - Echo IDLE status when waiting",
      "  - fswatch -o $WATCH_FILES | while read loop",
      "  - Echo 'File change detected' on trigger",
      "  - Re-run ralph-streams.sh on each trigger",
      "Make executable: chmod +x scripts/stream-watch-features.sh",
      "Test syntax: bash -n scripts/stream-watch-features.sh",
      "Commit: feat(tmux): add fswatch wrapper for features stream"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Create fswatch wrappers for refactor, polish, verify, hygiene streams",
    "plan_section": "File Watcher Architecture",
    "steps": [
      "Create scripts/stream-watch-refactor.sh:",
      "  - MODEL='sonnet'",
      "  - WATCH_FILES='streams/refactor/backlog.md'",
      "Create scripts/stream-watch-polish.sh:",
      "  - MODEL='sonnet'",
      "  - WATCH_FILES='streams/polish/backlog.md'",
      "Create scripts/stream-watch-verify.sh:",
      "  - MODEL='sonnet'",
      "  - WATCH_FILES='specs/manifest.json'",
      "Create scripts/stream-watch-hygiene.sh:",
      "  - MODEL='sonnet'",
      "  - WATCH_FILES='streams/refactor/backlog.md streams/polish/backlog.md streams/archive/completed.md'",
      "Make all executable: chmod +x scripts/stream-watch-*.sh",
      "Test syntax for all: bash -n scripts/stream-watch-*.sh",
      "Commit: feat(tmux): add fswatch wrappers for all streams"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Create ralph-tmux-status.sh header display",
    "plan_section": "Implementation Tasks - Task 3",
    "steps": [
      "Read specs/plans/tmux-orchestration.md for status display design",
      "Create ralph-tmux-status.sh with:",
      "  - Track START_TIME for uptime calculation",
      "  - While true loop with clear + display + sleep 5",
      "  - Display:",
      "    - RALPH MULTI-STREAM ORCHESTRATOR header (use RalphX orange if possible)",
      "    - Uptime in hours:minutes",
      "    - Current time",
      "    - Backlog counts: P0 (features), P1 (refactor), P2/P3 (polish)",
      "    - Quick-reference keys: Ctrl+b [0-5], Ctrl+b d, Ctrl+b [",
      "  - Count items with: grep -c '^- \\[ \\]' file 2>/dev/null || echo 0",
      "Make executable: chmod +x ralph-tmux-status.sh",
      "Test syntax: bash -n ralph-tmux-status.sh",
      "Commit: feat(tmux): add status header display script"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Wire stream wrappers into ralph-tmux.sh",
    "plan_section": "Implementation Tasks - Task 1",
    "steps": [
      "Edit ralph-tmux.sh create_session() function:",
      "  - Pane 0 runs: ./ralph-tmux-status.sh",
      "  - Pane 1 runs: ./scripts/stream-watch-features.sh",
      "  - Pane 2 runs: ./scripts/stream-watch-refactor.sh",
      "  - Pane 3 runs: ./scripts/stream-watch-polish.sh",
      "  - Pane 4 runs: ./scripts/stream-watch-verify.sh",
      "  - Pane 5 runs: ./scripts/stream-watch-hygiene.sh",
      "  - Set pane titles using tmux select-pane -T",
      "Update stop_all() to also kill fswatch processes",
      "Add restart_stream() function for single-stream restart",
      "Test: ./ralph-tmux.sh creates session with all panes",
      "Test: ./ralph-tmux.sh stop cleanly terminates everything",
      "Commit: feat(tmux): wire stream wrappers into launcher"
    ],
    "passes": false
  },
  {
    "category": "rules",
    "description": "Add IDLE detection to stream rules",
    "plan_section": "File Watcher Architecture - Stream Lifecycle",
    "steps": [
      "Edit .claude/rules/stream-features.md:",
      "  - Add: 'No active phase AND no P0 items → Output <promise>IDLE</promise>, STOP'",
      "Edit .claude/rules/stream-refactor.md:",
      "  - Add: 'Backlog empty → Output <promise>IDLE</promise>, STOP'",
      "Edit .claude/rules/stream-polish.md:",
      "  - Add: 'Backlog empty → Output <promise>IDLE</promise>, STOP'",
      "Edit .claude/rules/stream-verify.md:",
      "  - Add: 'No completed phases to verify → Output <promise>IDLE</promise>, STOP'",
      "Edit .claude/rules/stream-hygiene.md:",
      "  - Add: 'All backlogs healthy (>3 items, <10 completed) → Output <promise>IDLE</promise>, STOP'",
      "Commit: docs(rules): add IDLE detection to all stream workflows"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Update ralph-streams.sh for IDLE detection",
    "plan_section": "Implementation Tasks - Task 6",
    "steps": [
      "Read ralph-streams.sh current implementation",
      "Update completion detection to also check for IDLE:",
      "  - if echo \"$output\" | grep -q '<promise>COMPLETE</promise>\\|<promise>IDLE</promise>'",
      "  - Log which signal was detected",
      "Add stream name prefix to output: echo \"[$STREAM] message\"",
      "Test: Verify IDLE detection works (mock test or manual)",
      "Commit: feat(ralph): add IDLE detection and stream prefix to ralph-streams.sh"
    ],
    "passes": false
  },
  {
    "category": "documentation",
    "description": "Add tmux documentation to CLAUDE.md",
    "plan_section": "Implementation Tasks - Task 7",
    "steps": [
      "Edit CLAUDE.md to add Tmux Orchestration section:",
      "  - Prerequisites: brew install tmux fswatch",
      "  - Daily commands: ./ralph-tmux.sh, attach, stop, restart",
      "  - Key bindings: Ctrl+b d (detach), Ctrl+b [0-5] (switch pane), Ctrl+b z (zoom)",
      "  - Pane layout reference",
      "  - fswatch behavior explanation",
      "Commit: docs: add tmux orchestration guide to CLAUDE.md"
    ],
    "passes": false
  },
  {
    "category": "verification",
    "description": "End-to-end verification of tmux orchestration",
    "plan_section": "Verification",
    "steps": [
      "Verify tmux installed: tmux -V",
      "Verify fswatch installed: fswatch --version",
      "Test session creation: ./ralph-tmux.sh",
      "Verify 6 panes created with correct layout",
      "Verify header shows status information",
      "Test detach: Ctrl+b d",
      "Test reattach: ./ralph-tmux.sh attach",
      "Test file watch: touch streams/features/backlog.md (should trigger features)",
      "Test stop: ./ralph-tmux.sh stop (all processes terminate)",
      "Test single restart: ./ralph-tmux.sh restart features",
      "Document any issues in activity log",
      "Commit: docs: complete Phase 24 verification"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **fswatch over polling** | Zero API calls when idle, instant response to file changes |
| **Wrapper scripts per stream** | Clean separation, easy to customize per-stream behavior |
| **6-pane layout** | Header for status + 5 stream panes, features gets largest space |
| **IDLE signal** | Clean exit for fswatch to take over, distinguishes from COMPLETE |
| **tmux over alternatives** | Industry standard, works with iTerm, supports detach/reattach |
| **Status header pane** | At-a-glance overview without consuming stream output space |

---

## Verification Checklist

**Manual verification after completing all tasks:**

### Prerequisites
- [ ] `tmux -V` shows version 3.x+
- [ ] `fswatch --version` shows version

### Scripts
- [ ] `ralph-tmux.sh` is executable and passes syntax check
- [ ] `ralph-tmux-status.sh` is executable and passes syntax check
- [ ] All `scripts/stream-watch-*.sh` are executable and pass syntax check

### Functionality
- [ ] `./ralph-tmux.sh` creates 6-pane tmux session
- [ ] Header pane shows uptime and backlog counts
- [ ] Each stream pane shows stream name and status
- [ ] `Ctrl+b d` detaches without stopping streams
- [ ] `./ralph-tmux.sh attach` reattaches to running session
- [ ] `touch streams/features/backlog.md` triggers features stream
- [ ] `./ralph-tmux.sh stop` terminates all processes cleanly
- [ ] `./ralph-tmux.sh restart features` restarts single stream

### Documentation
- [ ] CLAUDE.md has Tmux Orchestration section
- [ ] Daily workflow commands documented
- [ ] Key bindings documented
