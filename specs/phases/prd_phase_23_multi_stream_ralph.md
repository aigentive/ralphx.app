# RalphX - Phase 23: Multi-Stream RALPH Architecture

## Overview

This phase splits the current single RALPH loop into multiple focused streams. Each stream has ONE job, preventing gaming/scope avoidance and ensuring all work types get done. The architecture enables future parallelization while starting with sequential execution.

**Reference Plan:**
- `specs/plans/multi-stream-ralph-architecture.md` - Detailed architecture with folder structure, stream definitions, and PROMPT file contents

## Goals

1. Create `.claude/rules/stream-*.md` files as single source of truth for each stream's workflow
2. Create `streams/` directory with thin PROMPT.md wrappers that reference the rules
3. Each stream has dedicated backlog.md (where applicable) and activity.md
4. Migrate existing `logs/code-quality.md` content to appropriate stream backlogs
5. Update `ralph.sh` to accept stream name argument
6. Create `ralph-orchestrator.sh` for sequential round-robin execution
7. Remove old `quality-improvement.md` after verification (full replacement, not deprecation)

## Dependencies

### Phase 22 (Execution Bar Real-time) - Completed

No blocking dependencies. This phase modifies automation infrastructure, not application code.

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/multi-stream-ralph-architecture.md`
2. Understand the stream definitions and folder structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Verify the file is created/modified correctly
4. Run appropriate linting if code files are modified
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/multi-stream-ralph-architecture.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "infrastructure",
    "description": "Create streams/ folder structure and copy ralph.sh",
    "plan_section": "1.1 Folder Structure",
    "steps": [
      "Read specs/plans/multi-stream-ralph-architecture.md section '1.1'",
      "IMPORTANT: Copy ralph.sh to ralph-streams.sh first (we're using ralph.sh to run this phase):",
      "  - cp ralph.sh ralph-streams.sh",
      "Create directory structure:",
      "  - streams/features/",
      "  - streams/refactor/",
      "  - streams/polish/",
      "  - streams/verify/",
      "  - streams/hygiene/",
      "  - streams/archive/",
      "Verify with ls -la streams/",
      "Commit: chore(streams): create multi-stream folder structure and copy ralph.sh"
    ],
    "passes": true
  },
  {
    "category": "rules",
    "description": "Create .claude/rules/stream-features.md",
    "plan_section": "1.5 Stream PROMPT Files - features",
    "steps": [
      "Read specs/plans/multi-stream-ralph-architecture.md section '1.5' for features stream",
      "Read .claude/rules/quality-improvement.md for P0 handling patterns",
      "Create .claude/rules/stream-features.md with:",
      "  - Overview: PRD tasks + P0 gap fixes",
      "  - Rules: ONE task per iteration, P0 blocks PRD work, no quality improvement work",
      "  - Workflow: P0 check → manifest → find task → execute → log → commit → STOP",
      "  - Phase complete signal: <promise>COMPLETE</promise>",
      "  - Reference to gap-verification.md for P0 context",
      "Commit: docs(rules): add features stream workflow"
    ],
    "passes": true
  },
  {
    "category": "rules",
    "description": "Create .claude/rules/stream-refactor.md",
    "plan_section": "1.5 Stream PROMPT Files - refactor",
    "steps": [
      "Read specs/plans/multi-stream-ralph-architecture.md section '1.5' for refactor stream",
      "Read .claude/rules/quality-improvement.md for P1 handling patterns",
      "Read .claude/rules/code-quality-standards.md for LOC limits",
      "Create .claude/rules/stream-refactor.md with:",
      "  - Overview: P1 large file splits and architectural refactors only",
      "  - Rules: ONE P1 item per iteration, ONLY P1 work, cannot skip to easier work",
      "  - Workflow: read backlog → find first [ ] → execute → lint → mark [x] → log → commit → STOP",
      "  - Backlog empty signal: <promise>COMPLETE</promise>",
      "  - Reference to code-quality-standards.md for LOC limits",
      "Commit: docs(rules): add refactor stream workflow"
    ],
    "passes": true
  },
  {
    "category": "rules",
    "description": "Create .claude/rules/stream-polish.md",
    "plan_section": "1.5 Stream PROMPT Files - polish",
    "steps": [
      "Read specs/plans/multi-stream-ralph-architecture.md section '1.5' for polish stream",
      "Read .claude/rules/quality-improvement.md for P2/P3 handling patterns",
      "Create .claude/rules/stream-polish.md with:",
      "  - Overview: P2/P3 cleanup, type fixes, lint fixes, small extractions",
      "  - Rules: ONE P2/P3 item per iteration, ONLY backlog work, cannot skip",
      "  - Workflow: read backlog → find first [ ] → execute → lint → mark [x] → log → commit → STOP",
      "  - Backlog empty signal: <promise>COMPLETE</promise>",
      "Commit: docs(rules): add polish stream workflow"
    ],
    "passes": true
  },
  {
    "category": "rules",
    "description": "Create .claude/rules/stream-verify.md",
    "plan_section": "1.5 Stream PROMPT Files - verify",
    "steps": [
      "Read specs/plans/multi-stream-ralph-architecture.md section '1.5' for verify stream",
      "Read .claude/rules/gap-verification.md for verification checks",
      "Create .claude/rules/stream-verify.md with:",
      "  - Overview: Gap detection in completed phases, produces P0 items",
      "  - Rules: scan for gaps, output P0 items to features/backlog.md, do NOT fix anything",
      "  - Workflow: read manifest → check completed phases → run verification checks → append P0s → log → STOP",
      "  - Verification checks: WIRING, API, STATE, EVENTS (reference gap-verification.md)",
      "  - No backlog (produces to features/backlog.md)",
      "Commit: docs(rules): add verify stream workflow"
    ],
    "passes": false
  },
  {
    "category": "rules",
    "description": "Create .claude/rules/stream-hygiene.md",
    "plan_section": "1.5 Stream PROMPT Files - hygiene",
    "steps": [
      "Read specs/plans/multi-stream-ralph-architecture.md section '1.5' for hygiene stream",
      "Read .claude/rules/quality-improvement.md for deferred validation and archive patterns",
      "Create .claude/rules/stream-hygiene.md with:",
      "  - Overview: Backlog maintenance, refill via Explore, archive completed items",
      "  - Rules: maintain backlogs, do NOT fix code (that's other streams' job)",
      "  - Workflow: archive >10 [x] items → refill <3 active via Explore → validate strikethroughs → log → STOP",
      "  - Explore agent prompts for P1 and P2/P3 discovery",
      "  - No backlog (maintains others)",
      "Commit: docs(rules): add hygiene stream workflow"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Create streams/*/PROMPT.md wrappers",
    "plan_section": "1.5 Stream PROMPT Files",
    "steps": [
      "Create streams/features/PROMPT.md with @ references:",
      "  - @specs/manifest.json",
      "  - @streams/features/backlog.md",
      "  - @.claude/rules/stream-features.md",
      "Create streams/refactor/PROMPT.md with @ references:",
      "  - @streams/refactor/backlog.md",
      "  - @.claude/rules/stream-refactor.md",
      "Create streams/polish/PROMPT.md with @ references:",
      "  - @streams/polish/backlog.md",
      "  - @.claude/rules/stream-polish.md",
      "Create streams/verify/PROMPT.md with @ references:",
      "  - @specs/manifest.json",
      "  - @streams/features/backlog.md (to append P0s)",
      "  - @.claude/rules/stream-verify.md",
      "Create streams/hygiene/PROMPT.md with @ references:",
      "  - @streams/refactor/backlog.md",
      "  - @streams/polish/backlog.md",
      "  - @streams/archive/completed.md",
      "  - @.claude/rules/stream-hygiene.md",
      "Commit: docs(streams): add PROMPT.md wrappers for all streams"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Create initial backlog and activity files",
    "plan_section": "1.1 Folder Structure",
    "steps": [
      "Create streams/features/backlog.md with header: '# Features Backlog (P0 - Critical Gaps)'",
      "Create streams/features/activity.md with header: '# Features Stream Activity'",
      "Create streams/refactor/backlog.md with header: '# Refactor Backlog (P1 - Large Splits)'",
      "Create streams/refactor/activity.md with header: '# Refactor Stream Activity'",
      "Create streams/polish/backlog.md with header: '# Polish Backlog (P2/P3 - Cleanup)'",
      "Create streams/polish/activity.md with header: '# Polish Stream Activity'",
      "Create streams/verify/activity.md with header: '# Verify Stream Activity'",
      "Create streams/hygiene/activity.md with header: '# Hygiene Stream Activity'",
      "Create streams/archive/completed.md with header: '# Archived Completed Items'",
      "Commit: chore(streams): create initial backlog and activity files"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Migrate logs/code-quality.md to stream backlogs",
    "plan_section": "1.6 Migration from Current System",
    "steps": [
      "Read logs/code-quality.md",
      "Identify P0 items (Critical/Phase Gaps) → move to streams/features/backlog.md",
      "Identify P1 items (High Impact) → move to streams/refactor/backlog.md",
      "Identify P2/P3 items → move to streams/polish/backlog.md",
      "Identify completed [x] items → move to streams/archive/completed.md",
      "Verify no items lost in migration (count before/after)",
      "DO NOT delete or modify logs/code-quality.md yet (cleanup is final task)",
      "Commit: chore(streams): migrate code-quality.md to stream backlogs"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Update ralph-streams.sh to support stream argument and model selection",
    "plan_section": "1.3 Modified ralph.sh",
    "steps": [
      "Read ralph-streams.sh (copied from ralph.sh in task 1)",
      "Add STREAM=$1 argument parsing",
      "Add validation: stream must be one of features|refactor|polish|verify|hygiene",
      "Add MODEL environment variable support:",
      "  - Read ANTHROPIC_MODEL env var (default: opus)",
      "  - Pass --model flag to claude command",
      "  - Usage: ANTHROPIC_MODEL=sonnet ./ralph-streams.sh refactor 5",
      "Change prompt source from PROMPT.md to streams/${STREAM}/PROMPT.md",
      "Keep backward compatibility: if no stream arg, use legacy PROMPT.md",
      "Update completion detection per stream type",
      "Test syntax: bash -n ralph-streams.sh",
      "Commit: feat(ralph): add stream argument and model selection to ralph-streams.sh"
    ],
    "passes": false
  },
  {
    "category": "infrastructure",
    "description": "Create ralph-orchestrator.sh with per-stream model config",
    "plan_section": "1.4 Orchestrator Script",
    "steps": [
      "Read specs/plans/multi-stream-ralph-architecture.md section '1.4'",
      "Create ralph-orchestrator.sh with:",
      "  - Shebang and description",
      "  - Configurable model per stream at top of script:",
      "    MODEL_FEATURES=${MODEL_FEATURES:-opus}",
      "    MODEL_REFACTOR=${MODEL_REFACTOR:-sonnet}",
      "    MODEL_POLISH=${MODEL_POLISH:-sonnet}",
      "    MODEL_VERIFY=${MODEL_VERIFY:-sonnet}",
      "    MODEL_HYGIENE=${MODEL_HYGIENE:-sonnet}",
      "  - While true loop",
      "  - Each stream call uses ralph-streams.sh: ANTHROPIC_MODEL=$MODEL_FEATURES ./ralph-streams.sh features 5",
      "  - Features stream: 5 iterations (opus by default - most critical)",
      "  - Refactor stream: 2 iterations (sonnet by default)",
      "  - Polish stream: 2 iterations (sonnet by default)",
      "  - Verify stream: 1 iteration (sonnet by default)",
      "  - Hygiene stream: 1 iteration (sonnet by default)",
      "  - Sleep between cycles",
      "  - Usage comment: MODEL_FEATURES=sonnet ./ralph-orchestrator.sh",
      "Make executable: chmod +x ralph-orchestrator.sh",
      "Test syntax: bash -n ralph-orchestrator.sh",
      "Commit: feat(ralph): add orchestrator with per-stream model configuration"
    ],
    "passes": false
  },
  {
    "category": "cleanup",
    "description": "Verify streams and remove legacy files",
    "plan_section": "1.6 Migration from Current System",
    "steps": [
      "VERIFICATION FIRST - confirm everything works:",
      "  - Test: ./ralph-streams.sh features 1 (should read from streams/features/)",
      "  - Test: ./ralph-streams.sh refactor 1 (should read from streams/refactor/)",
      "  - Compare logs/code-quality.md items vs stream backlogs (no items lost)",
      "  - Verify P0 rules in stream-features.md",
      "  - Verify P1 rules in stream-refactor.md",
      "  - Verify P2/P3 rules in stream-polish.md",
      "  - Verify deferred validation in stream-hygiene.md",
      "  - Verify Explore prompts in stream-hygiene.md",
      "Keep ralph.sh as reference (no replacement needed)",
      "Delete legacy files:",
      "  - rm PROMPT.md (no longer used - streams have their own)",
      "  - rm .claude/rules/quality-improvement.md (replaced by stream-*.md)",
      "  - rm logs/code-quality.md (content now in stream backlogs)",
      "Commit: chore(cleanup): remove legacy PROMPT.md, quality-improvement.md, code-quality.md"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Rules in .claude/rules/** | Single source of truth for workflows, consistent with existing patterns |
| **Thin PROMPT.md wrappers** | Just @ references, rules live in canonical location |
| **5 focused streams** | Separation of concerns prevents gaming/scope avoidance |
| **Dedicated backlogs per stream** | Clear ownership, no contention on single file |
| **P0 only in features stream** | Critical gaps must be fixed before new work |
| **Verify produces, features consumes** | Clear data flow, verify doesn't fix |
| **Hygiene refills backlogs** | Automated maintenance prevents starvation |
| **Sequential before parallel** | Simpler debugging, parallel is Phase 2 |
| **Full replacement, not deprecation** | Old files removed after verification, no lingering cruft |

---

## Verification Checklist

**Manual verification after completing all tasks:**

### Rules Files
- [ ] `.claude/rules/stream-features.md` exists with P0 + PRD workflow
- [ ] `.claude/rules/stream-refactor.md` exists with P1 workflow
- [ ] `.claude/rules/stream-polish.md` exists with P2/P3 workflow
- [ ] `.claude/rules/stream-verify.md` exists with gap detection workflow
- [ ] `.claude/rules/stream-hygiene.md` exists with maintenance workflow

### Folder Structure
- [ ] `streams/` directory exists with 6 subdirectories
- [ ] Each stream has PROMPT.md with correct @ references
- [ ] features, refactor, polish have backlog.md
- [ ] All streams have activity.md
- [ ] archive has completed.md

### Migration
- [ ] All P0 items in streams/features/backlog.md
- [ ] All P1 items in streams/refactor/backlog.md
- [ ] All P2/P3 items in streams/polish/backlog.md
- [ ] All completed items in streams/archive/completed.md
- [ ] No items lost (count verification)

### Scripts
- [ ] `./ralph-streams.sh features 1` reads from streams/features/PROMPT.md
- [ ] `./ralph-streams.sh refactor 1` reads from streams/refactor/PROMPT.md
- [ ] `ANTHROPIC_MODEL=sonnet ./ralph-streams.sh polish 1` uses sonnet model
- [ ] ralph-orchestrator.sh is executable and calls ralph-streams.sh
- [ ] bash -n passes for both scripts
- [ ] `ralph.sh` kept as reference (unchanged)

### Cleanup (after final task)
- [ ] `PROMPT.md` deleted (streams have their own)
- [ ] `.claude/rules/quality-improvement.md` deleted
- [ ] `logs/code-quality.md` deleted
- [ ] No orphaned references to deleted files
