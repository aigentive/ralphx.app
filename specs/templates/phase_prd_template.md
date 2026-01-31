# RalphX - Phase {N}: {Phase Name}

## Overview

{1-2 paragraph description of what this phase implements and why.}

**Reference Plan:**
- `specs/plans/{plan_name}.md` - {Brief description of the detailed plan}

## Goals

1. {Goal 1}
2. {Goal 2}
3. {Goal 3}

## Dependencies

### Phase {N-1} ({Previous Phase Name}) - Required

| Dependency | Why Needed |
|------------|------------|
| {Component/Feature} | {Why this phase needs it} |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/{plan_name}.md`
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

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

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
2. **Read the ENTIRE implementation plan** at `specs/plans/{plan_name}.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend|frontend|mcp|agent|documentation",
    "description": "{What this task accomplishes}",
    "plan_section": "{Section name in the detailed plan}",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "feat(scope): description",
    "steps": [
      "Read specs/plans/{plan_name}.md section '{Section}'",
      "{Step 1}",
      "{Step 2}",
      "Run cargo test / npm run typecheck",
      "Commit: {type}({scope}): {message}"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "{Depends on task 1}",
    "plan_section": "{Section name}",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(scope): wire component",
    "steps": [
      "Read specs/plans/{plan_name}.md section '{Section}'",
      "{Step 1}",
      "{Step 2}",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(scope): wire component"
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
| **{Decision 1}** | {Why this approach was chosen} |
| **{Decision 2}** | {Why this approach was chosen} |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] {Verification item}

### Frontend - Run `npm run test`
- [ ] {Verification item}

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] {End-to-end flow 1}
- [ ] {End-to-end flow 2}

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (click handler, route, event listener)
- [ ] New component is imported AND rendered (not behind disabled flag)
- [ ] API wrappers call backend commands
- [ ] State changes reflect in UI

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
