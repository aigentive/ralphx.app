# Verify Stream

## Overview

The verify stream handles **gap detection in completed phases**. It scans for bugs where code exists but isn't properly wired up, then produces P0 items for the features stream to fix.

**Focus:** Find orphaned implementations, missing API integrations, and incomplete event chains in completed phases.

## Rules

1. **Scan for gaps, do NOT fix anything** — fixing is the features stream's job
2. **Output P0 items to streams/features/backlog.md** — verify produces, features consumes
3. **ONE verification pass per iteration, then STOP**
4. **No backlog** — this stream reads completed phases and outputs to features/backlog.md
5. **Follow git workflow rules** — see `.claude/rules/git-workflow.md` (Recovery Check does not apply — this stream doesn't write code)
6. **Skip already-verified phases SILENTLY** — if activity log shows "No gaps found" for a phase, do NOT re-verify, do NOT log, do NOT commit. Just skip it entirely.

## Workflow

```
1. Read specs/manifest.json

2. Find phases with status: "complete"

3. Read streams/verify/activity.md to find already-verified phases
   → Phase has "No gaps found" entry? → SKIP SILENTLY (no log, no commit, no output)
   → Only verify phases with NO prior "No gaps found" log entry

4. No unverified phases remain? → Output IDLE signal → END (no log, no commit)

5. For each UNVERIFIED completed phase:
   a. Read the phase PRD
   b. For each feature/component implemented, run verification checks:
      - WIRING: Is it invoked from entry point?
      - API: Does frontend call backend command?
      - STATE: Are transitions triggered?
      - EVENTS: Are events emitted AND listened?
   c. Document any gaps found

6. Gaps found? → Append to streams/features/backlog.md as P0 items

7. Log findings to streams/verify/activity.md

8. Commit if changes made: chore(verify): add P0 items from phase N verification

9. STOP
```

## Verification Checks

Reference: `.claude/rules/gap-verification.md`

### Check 1: WIRING (Critical)

**Bug class:** Orphaned Implementation — code exists but is never invoked

```
1. Identify the ENTRY POINT (where user interaction starts)
2. Trace the call path from entry point to new code
3. Verify the new code is ACTUALLY INVOKED (not behind disabled flag)
```

**Red flags:**
- Optional props that default to `false` or disabled
- Components imported but never rendered
- Functions exported but never called
- Feature flags that are never enabled
- Hooks defined but not used in any component

### Check 2: API Surface

**Bug class:** Backend implemented but no frontend call

```
1. Backend: Verify command exists in Tauri (#[tauri::command])
2. Frontend: Verify api wrapper calls the command (invoke())
3. UI: Verify a component calls the api wrapper
```

### Check 3: STATE Flow

**Bug class:** State transitions implemented but not triggered

```
1. Verify transition TO this state is triggered somewhere
2. Verify transition FROM this state is handled
3. Verify UI reflects the state correctly
```

### Check 4: EVENTS

**Bug class:** Events emitted but not listened to (or vice versa)

```
1. Backend: Verify event is emitted at the right time
2. Frontend: Verify useXxxEvents hook listens for it
3. UI: Verify hook is used in a mounted component
```

## P0 Output Format

When gaps are found, append to `streams/features/backlog.md`:

```markdown
- [ ] [Frontend/Backend] [Bug class]: Description - file:line
```

Examples:
```markdown
- [ ] [Frontend] Orphaned: useViewRegistry prop never enabled - src/components/tasks/TaskDetailOverlay.tsx:508
- [ ] [Backend] Missing frontend call: approve_fix_task command - src-tauri/src/commands/review_commands.rs:198
- [ ] [Frontend] State not triggered: Reviewing status never reached - src/components/tasks/TaskCard.tsx:45
- [ ] [Backend] Event not listened: queue_changed never subscribed - src-tauri/src/commands/task_commands.rs:512
```

## Common Failure Patterns

| Pattern | Symptom | Detection |
|---------|---------|-----------|
| **Optional flag trap** | Feature behind `enabled={false}` default | Grep for `= false` in new prop definitions |
| **Import-only** | Component imported but JSX never rendered | Grep import, then grep `<ComponentName` |
| **Export-only** | Function exported but never called | Grep export, then grep function name usage |
| **Dead hook** | Hook defined but not used in components | Grep `use[Name]` definition vs usage |
| **Partial chain** | Backend done, frontend missing | Grep `#[tauri::command]` vs `invoke(` |

## Explore Agent Prompt

Use this prompt for phase verification:

```
Verify Phase [N] implementation against PRD at [path]:

1. WIRING: For each new component, trace from entry point. Is it actually invoked?
   - Check for optional props defaulting to false/disabled
   - Check for components imported but not rendered

2. API: For each new command, verify frontend calls it
   - Tauri command → api wrapper → UI component

3. STATE: For each new status, verify transitions trigger correctly
   - Transition TO the state exists
   - Transition FROM the state exists
   - UI reflects the state

4. EVENTS: For each new event, verify emit + listen + UI update
   - Backend emits → Hook listens → Component uses hook

Report as P0 items:
- [ ] [Frontend/Backend] [Bug class]: Description - file:line
```

## No Gaps Found

If verification finds no gaps in this pass:

Output: `<promise>COMPLETE</promise>`

This signals that verification completed successfully with no new P0 items.

## IDLE Detection

When there are **no unverified phases** (all completed phases already have "No gaps found" in activity log, OR all phases are pending/active):

Output: `<promise>IDLE</promise>`

This signals the fswatch wrapper to take over and wait for manifest.json changes (new phase completions).

## Signal Output Rules

**CRITICAL:** Completion signals must be output as a **standalone final statement**.

- Output the signal as your LAST message content
- Do NOT quote or mention the signal syntax elsewhere in your output
- When discussing signals in logs/activity, refer to them as "the IDLE signal" or "the COMPLETE signal" — never the actual `<promise>` tags

## Activity Log Format

Log entries go in `streams/verify/activity.md`:

```markdown
### YYYY-MM-DD HH:MM:SS - Phase [N] Verification
**Phases Checked:** [list of phase numbers]

**Checks Run:**
- WIRING: [count] features checked
- API: [count] commands verified
- STATE: [count] statuses verified
- EVENTS: [count] events verified

**Gaps Found:** [count]
- [list gaps if any]

**Result:** [N] P0 items added to features/backlog.md | No gaps found
```

## Reference

- Detailed verification workflow: `.claude/rules/gap-verification.md`
- Features stream (consumes P0s): `.claude/rules/stream-features.md`
