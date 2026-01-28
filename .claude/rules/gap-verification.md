# Gap Verification Workflow

## When to Run

**Trigger:** All tasks in a phase have `"passes": true`

**Before:** Updating manifest to complete the phase

## Workflow

```
1. Read entire PRD + referenced specs/plans
2. Build mental model of what was supposed to be implemented
3. Run VERIFICATION CHECKS (below) for each feature
4. Gaps found? → Log to code-quality.md as P0 → Continue iterations
5. No gaps? → Phase complete, update manifest
```

## Verification Checks

### Check 1: Wiring Verification (CRITICAL)

**Bug class:** Orphaned Implementation — code exists but is never invoked

**For each new component/feature:**
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

**Example (Phase 20 bug):**
```
❌ `useViewRegistry` prop defaults to `false`
❌ `TaskDetailOverlay` never passes `useViewRegistry={true}`
❌ Result: Review views exist but are never rendered
```

**Explore prompt:**
```
For [feature name], verify wiring:
1. Find the entry point (user click, route, etc.)
2. Trace to the new component/function
3. Check if it's actually invoked (no disabled flags, no dead paths)
Report any orphaned implementations.
```

### Check 2: API Surface Verification

**Bug class:** Backend implemented but no frontend call

**For each new API endpoint/command:**
```
1. Backend: Verify command exists in Tauri
2. Frontend: Verify api wrapper calls the command
3. UI: Verify a component calls the api wrapper
```

**Explore prompt:**
```
For each new Tauri command in this phase:
1. Find the #[tauri::command] definition
2. Find the api wrapper in src/api/
3. Find the UI component that calls the wrapper
Report any commands with no frontend integration.
```

### Check 3: State Flow Verification

**Bug class:** State transitions implemented but not triggered

**For each new state/status:**
```
1. Verify transition TO this state is triggered somewhere
2. Verify transition FROM this state is handled
3. Verify UI reflects the state correctly
```

**Explore prompt:**
```
For each new InternalStatus in this phase:
1. Find where transition TO this status is triggered
2. Find where transition FROM this status is handled
3. Find UI that renders differently for this status
Report any orphaned states.
```

### Check 4: Event Verification

**Bug class:** Events emitted but not listened to (or vice versa)

**For each new event type:**
```
1. Backend: Verify event is emitted at the right time
2. Frontend: Verify useXxxEvents hook listens for it
3. UI: Verify hook is used in a mounted component
```

**Explore prompt:**
```
For each new event type in this phase:
1. Find where the event is emitted (backend)
2. Find the useXxxEvents hook that listens
3. Find the component that uses the hook
Report any events without complete emit→listen→UI chain.
```

### Check 5: Type Consistency Verification

**Bug class:** Type defined but not used, or type mismatch between layers

**For each new type/schema:**
```
1. Verify Rust type matches TypeScript type
2. Verify Zod schema validates correctly
3. Verify type is used where data flows
```

## P0 Logging Format

When gaps are found, add to `logs/code-quality.md`:

```markdown
### P0 - Critical (Phase Gaps)

- [ ] [P0] [Frontend] Orphaned Implementation: [description] - file:line
- [ ] [P0] [Backend] Missing frontend call: [command name] - file:line
- [ ] [P0] [Frontend] State not triggered: [status name] - file:line
```

## Explore Agent Prompt (Full Verification)

Use this prompt when running complete phase verification:

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
- [ ] [P0] [Frontend/Backend] [Bug class]: Description - file:line
```

## Common Failure Patterns

| Pattern | Symptom | Detection |
|---------|---------|-----------|
| **Optional flag trap** | Feature behind `enabled={false}` default | Grep for `= false` in new prop definitions |
| **Import-only** | Component imported but JSX never rendered | Grep import, then grep `<ComponentName` |
| **Export-only** | Function exported but never called | Grep export, then grep function name usage |
| **Dead hook** | Hook defined but not used in components | Grep `use[Name]` definition vs usage |
| **Partial chain** | Backend done, frontend missing | Grep `#[tauri::command]` vs `invoke(` |

## Integration with Quality Loop

Gap verification feeds into the quality improvement loop:

1. **P0 items from gap verification take priority** over all P1/P2/P3 items
2. Gap items are picked FIRST regardless of task size
3. A phase is NOT complete until all P0 items are resolved

See `.claude/rules/quality-improvement.md` for the full quality loop workflow.
