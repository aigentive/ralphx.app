# Multi-Stream RALPH Architecture

## Overview

Split the current single RALPH loop into multiple focused streams to:
- Prevent gaming/scope avoidance (each stream has ONE job)
- Ensure large refactoring work gets done (dedicated refactor stream)
- Maintain backlog hygiene automatically
- Enable future parallelization

## Phase 1: Sequential Implementation

### 1.1 Folder Structure

Create `streams/` directory with dedicated folders per stream:

```
streams/
├── features/
│   ├── PROMPT.md        # Stream-specific instructions
│   ├── backlog.md       # P0 items (gaps to fix)
│   └── activity.md      # Execution log
│
├── refactor/
│   ├── PROMPT.md
│   ├── backlog.md       # P1 items (large file splits)
│   └── activity.md
│
├── polish/
│   ├── PROMPT.md
│   ├── backlog.md       # P2/P3 items (cleanup)
│   └── activity.md
│
├── verify/
│   ├── PROMPT.md
│   └── activity.md      # No backlog - produces to features/backlog.md
│
├── hygiene/
│   ├── PROMPT.md
│   └── activity.md      # No backlog - maintains others
│
└── archive/
    └── completed.md     # Old done items moved here
```

### 1.2 Stream Definitions

| Stream | Focus | Reads | Writes | Completion |
|--------|-------|-------|--------|------------|
| **features** | PRD tasks + P0 fixes | `specs/manifest.json`, PRDs, `streams/features/backlog.md` | Source code, PRD passes, activity | All PRD tasks pass |
| **refactor** | P1 large splits only | `streams/refactor/backlog.md` | Source code, marks `[x]`, activity | Backlog empty |
| **polish** | P2/P3 cleanup only | `streams/polish/backlog.md` | Source code, marks `[x]`, activity | Backlog empty |
| **verify** | Gap detection | Completed PRDs, source code | `streams/features/backlog.md` (append P0), activity | No gaps found |
| **hygiene** | Backlog maintenance | All backlog files | Archive old items, refill via Explore, activity | Maintenance done |

### 1.3 Modified ralph.sh

Update `ralph.sh` to accept stream name and model selection:

```bash
./ralph.sh <stream> [max_iterations]

# Examples:
./ralph.sh features 50
./ralph.sh refactor 20
ANTHROPIC_MODEL=sonnet ./ralph.sh polish 30
ANTHROPIC_MODEL=opus ./ralph.sh verify 5
```

Key changes:
- Read prompt from `streams/<stream>/PROMPT.md`
- Support `ANTHROPIC_MODEL` env var (default: opus)
- Pass model to claude via `--model` flag
- Completion signal per stream (features uses `<promise>COMPLETE</promise>`, others use backlog empty)

### 1.4 Orchestrator Script

Create `ralph-orchestrator.sh` for sequential round-robin with per-stream model config:

```bash
#!/bin/bash
# ralph-orchestrator.sh - Run all streams sequentially
# Usage: ./ralph-orchestrator.sh
# Override models: MODEL_FEATURES=sonnet MODEL_REFACTOR=opus ./ralph-orchestrator.sh

# Per-stream model configuration (override via env vars)
MODEL_FEATURES=${MODEL_FEATURES:-opus}    # Features = opus (most critical)
MODEL_REFACTOR=${MODEL_REFACTOR:-sonnet}  # Refactor = sonnet (cost savings)
MODEL_POLISH=${MODEL_POLISH:-sonnet}      # Polish = sonnet (cost savings)
MODEL_VERIFY=${MODEL_VERIFY:-sonnet}      # Verify = sonnet (cost savings)
MODEL_HYGIENE=${MODEL_HYGIENE:-sonnet}    # Hygiene = sonnet (cost savings)

while true; do
  # Feature work (most iterations per cycle)
  ANTHROPIC_MODEL=$MODEL_FEATURES ./ralph.sh features 5

  # Quality work (fewer iterations)
  ANTHROPIC_MODEL=$MODEL_REFACTOR ./ralph.sh refactor 2
  ANTHROPIC_MODEL=$MODEL_POLISH ./ralph.sh polish 2

  # Maintenance (occasional)
  ANTHROPIC_MODEL=$MODEL_VERIFY ./ralph.sh verify 1
  ANTHROPIC_MODEL=$MODEL_HYGIENE ./ralph.sh hygiene 1

  # Brief pause between cycles
  sleep 10
done
```

### 1.5 Stream PROMPT Files

**streams/features/PROMPT.md:**
```markdown
@specs/manifest.json @streams/features/backlog.md

# Features Stream

## Rules
- ONE task per iteration, then STOP
- P0 items in backlog.md BLOCK all PRD work - fix first
- No quality improvement work (that's other streams' job)

## Workflow
1. Check streams/features/backlog.md for P0 items
   → If P0 exists: fix it, mark [x], commit, STOP
2. Read specs/manifest.json → find active phase PRD
3. Find first task with passes: false
4. Execute task following PRD steps
5. Log to streams/features/activity.md
6. Set passes: true, commit
7. STOP

## Phase Complete?
All tasks pass → Output <promise>COMPLETE</promise>
```

**streams/refactor/PROMPT.md:**
```markdown
@streams/refactor/backlog.md

# Refactor Stream

## Rules
- ONE P1 item per iteration, then STOP
- ONLY do P1 work from backlog.md
- Cannot skip to easier work (there is none)

## Workflow
1. Read streams/refactor/backlog.md
2. Find first unchecked [ ] item
3. Execute the file split/refactoring
4. Run cargo clippy / npm run lint
5. Mark [x] in backlog.md
6. Log to streams/refactor/activity.md
7. Commit: refactor(scope): description
8. STOP

## Backlog Empty?
No unchecked items → Output <promise>COMPLETE</promise>
```

**streams/polish/PROMPT.md:**
```markdown
@streams/polish/backlog.md

# Polish Stream

## Rules
- ONE P2/P3 item per iteration, then STOP
- ONLY do work from backlog.md
- Cannot skip to other work

## Workflow
1. Read streams/polish/backlog.md
2. Find first unchecked [ ] item
3. Execute the cleanup/extraction
4. Run linters
5. Mark [x] in backlog.md
6. Log to streams/polish/activity.md
7. Commit: refactor(scope): description
8. STOP

## Backlog Empty?
No unchecked items → Output <promise>COMPLETE</promise>
```

**streams/verify/PROMPT.md:**
```markdown
# Verify Stream

## Rules
- Scan for gaps in completed phases
- Output P0 items to streams/features/backlog.md
- Do NOT fix anything (that's features' job)

## Workflow
1. Read specs/manifest.json for completed phases
2. For each completed phase:
   a. Read the PRD
   b. For each feature/component implemented:
      - Check WIRING: Is it invoked from entry point?
      - Check API: Does frontend call backend command?
      - Check STATE: Are transitions triggered?
      - Check EVENTS: Are events emitted AND listened?
3. Gaps found? → Append to streams/features/backlog.md as P0
4. Log findings to streams/verify/activity.md
5. Commit if changes made
6. STOP

## Reference
See .claude/rules/gap-verification.md for detailed checks
```

**streams/hygiene/PROMPT.md:**
```markdown
@streams/refactor/backlog.md @streams/polish/backlog.md @streams/archive/completed.md

# Hygiene Stream

## Rules
- Maintain backlog health
- Refill empty backlogs via Explore
- Archive old completed items
- Do NOT fix code (that's other streams' job)

## Workflow
1. Check each backlog for >10 [x] items → move to archive/completed.md
2. Check refactor/backlog.md has <3 active items?
   → Run Explore agent for P1 issues, append results
3. Check polish/backlog.md has <3 active items?
   → Run Explore agent for P2/P3 issues, append results
4. Validate 2-3 strikethrough items (do they still exist?)
   → If fixed: increment counter, archive at :2
   → If still exists: unmark, make active
5. Log to streams/hygiene/activity.md
6. Commit if changes made
7. STOP
```

### 1.6 Migration from Current System

1. Create streams/ folder structure
2. Migrate current logs/code-quality.md content:
   - P0 items → streams/features/backlog.md
   - P1 items → streams/refactor/backlog.md
   - P2/P3 items → streams/polish/backlog.md
   - Completed items → streams/archive/completed.md
3. Update ralph.sh to support stream argument
4. Create orchestrator script
5. Update CLAUDE.md to reference new structure
6. Deprecate old PROMPT.md (or keep for single-stream mode)

---

## Phase 2: Parallel Architecture (Future Reference)

### 2.1 Worktree Setup

Each code-modifying stream gets its own worktree:

```bash
# Setup script
git worktree add ../ralphx-features -b features/active
git worktree add ../ralphx-refactor -b refactor/active
git worktree add ../ralphx-polish -b polish/active
```

Directory layout:
```
/ralphx/           # Main repo (verify, hygiene run here)
/ralphx-features/  # Features worktree
/ralphx-refactor/  # Refactor worktree
/ralphx-polish/    # Polish worktree
```

### 2.2 Parallel ralph.sh

```bash
#!/bin/bash
STREAM=$1
MAX_ITERATIONS=$2
WORKTREE_PATH="../ralphx-${STREAM}"

# Code-modifying streams use worktrees
if [[ "$STREAM" =~ ^(features|refactor|polish)$ ]]; then
  cd "$WORKTREE_PATH"
fi

for ((i=1; i<=MAX_ITERATIONS; i++)); do
  # Sync from main before each iteration
  git fetch origin main
  git rebase origin/main || {
    git rebase --abort
    sleep 30
    continue
  }

  # Run iteration
  claude -p "$(cat streams/${STREAM}/PROMPT.md)" --dangerously-skip-permissions

  # Push to main after each iteration
  git push origin HEAD:main || {
    # Push failed (someone else pushed), retry next iteration
    continue
  }

  sleep 5
done
```

### 2.3 Parallel Launcher

```bash
#!/bin/bash
# ralph-parallel.sh - Launch all streams in parallel

# Code-modifying streams (continuous)
./ralph.sh features 1000 &
FEATURES_PID=$!

./ralph.sh refactor 1000 &
REFACTOR_PID=$!

./ralph.sh polish 1000 &
POLISH_PID=$!

# Maintenance streams (periodic on main repo)
while true; do
  ./ralph.sh verify 5
  ./ralph.sh hygiene 5
  sleep 1800  # Every 30 minutes
done &
MAINT_PID=$!

# Cleanup on exit
trap "kill $FEATURES_PID $REFACTOR_PID $POLISH_PID $MAINT_PID" EXIT

wait
```

### 2.4 Merge Strategy

- Each stream rebases onto main before starting work
- Each stream pushes to main after completing one unit of work
- If push fails (someone else pushed first), rebase and retry
- Conflicts on source files: rebase handles most cases
- Conflicts on log files: unlikely (different sections/appends)

### 2.5 Parallel Considerations

**Benefits:**
- 3x throughput (3 code streams in parallel)
- No blocking between streams

**Challenges:**
- Merge conflicts when streams touch same files
- More disk space (3 full checkouts)
- More complex debugging
- Need monitoring to detect stuck streams

**When to Upgrade to Parallel:**
- Sequential throughput becomes bottleneck
- Large backlogs that need faster processing
- Multiple developers want different streams running

---

## Files to Create/Modify

### New Files
- [ ] `streams/features/PROMPT.md`
- [ ] `streams/features/backlog.md`
- [ ] `streams/features/activity.md`
- [ ] `streams/refactor/PROMPT.md`
- [ ] `streams/refactor/backlog.md`
- [ ] `streams/refactor/activity.md`
- [ ] `streams/polish/PROMPT.md`
- [ ] `streams/polish/backlog.md`
- [ ] `streams/polish/activity.md`
- [ ] `streams/verify/PROMPT.md`
- [ ] `streams/verify/activity.md`
- [ ] `streams/hygiene/PROMPT.md`
- [ ] `streams/hygiene/activity.md`
- [ ] `streams/archive/completed.md`
- [ ] `ralph-orchestrator.sh`

### Modified Files
- [ ] `ralph.sh` - Add stream argument support
- [ ] `CLAUDE.md` - Reference new streams architecture
- [ ] `.claude/rules/quality-improvement.md` - Update for new structure (or deprecate)

### Migrated Content
- [ ] `logs/code-quality.md` → split into stream backlogs
- [ ] `logs/activity.md` → keep as historical, new logs per stream

---

## Verification

### Sequential Mode Testing
1. Run `./ralph.sh features 1` - verify it reads from streams/features/
2. Run `./ralph.sh refactor 1` - verify it only picks P1 items
3. Run `./ralph.sh polish 1` - verify it only picks P2/P3 items
4. Run `./ralph.sh verify 1` - verify it outputs to features/backlog.md
5. Run `./ralph.sh hygiene 1` - verify it archives and refills
6. Run `./ralph-orchestrator.sh` for 1 full cycle

### Stream Isolation Testing
- Confirm features stream cannot pick P1/P2/P3 items
- Confirm refactor stream cannot pick PRD tasks
- Confirm each stream only writes to its own activity.md

### Migration Testing
- Verify all items from old code-quality.md appear in new backlogs
- Verify no items lost in migration
