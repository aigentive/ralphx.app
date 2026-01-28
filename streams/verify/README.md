# Verify Stream

Gap detection in completed phases. Uses **Sonnet** model.

## Purpose

- Scan completed phases for implementation gaps
- Find orphaned code (exists but not wired up)
- Output P0 items for features stream to fix

## Verification Checks

### 1. WIRING (Critical)
Find code that exists but is never invoked.

```
Entry point → Call path → New code → Actually invoked?
```

**Red flags:**
- Optional props defaulting to `false`
- Components imported but never rendered
- Functions exported but never called
- Hooks defined but not used

### 2. API Surface
Backend implemented but no frontend call.

```
#[tauri::command] → api wrapper → UI component
```

### 3. STATE Flow
State transitions implemented but not triggered.

```
Transition TO state → Transition FROM state → UI reflects state
```

### 4. EVENTS
Events emitted but not listened to.

```
Backend emits → Hook listens → Component uses hook
```

## Workflow

```
1. Read specs/manifest.json
2. Find phases with status: "complete"
3. For each completed phase:
   a. Read phase PRD
   b. Run verification checks on each feature
   c. Document gaps found
4. Append P0 items to streams/features/backlog.md
5. Commit → STOP
```

## Watched Files

- `specs/manifest.json` - Phase completions
- `specs/phases/` - PRD changes

## Output Signals

- `<promise>COMPLETE</promise>` - No gaps found
- `<promise>IDLE</promise>` - No completed phases to verify

## P0 Output Format

```markdown
- [ ] [Frontend/Backend] [Bug class]: Description - file:line
```

## Files

- `PROMPT.md` - Stream prompt
- `activity.md` - Activity log

## Related

- Rules: `.claude/rules/stream-verify.md`
- Gap verification: `.claude/rules/gap-verification.md`
