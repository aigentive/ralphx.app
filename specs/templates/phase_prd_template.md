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
4. Run `npm run lint && npm run typecheck` and `cargo clippy --all-targets --all-features -- -D warnings`
5. Commit with descriptive message

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
    "category": "backend|frontend|mcp|agent|documentation",
    "description": "{What this task accomplishes}",
    "plan_section": "{Section name in the detailed plan}",
    "steps": [
      "Read specs/plans/{plan_name}.md section '{Section}'",
      "{Step 1}",
      "{Step 2}",
      "Run cargo test / npm run typecheck",
      "Commit: {type}({scope}): {message}"
    ],
    "passes": false
  }
]
```

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

### Build Verification
- [ ] `npm run lint` passes
- [ ] `npm run typecheck` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `npm run build` succeeds
- [ ] `cargo build --release` succeeds

### Integration Testing
- [ ] {End-to-end flow 1}
- [ ] {End-to-end flow 2}
