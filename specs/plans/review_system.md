# Review System Planning

## Overview

This document captures planning for the task review system - what happens when tasks enter review states and how they progress through approval or revision.

---

## Kanban Column to Internal State Mapping

### Default RalphX Workflow

| Column ID | Display Name | Internal Status |
|-----------|--------------|-----------------|
| `draft` | Draft | `backlog` |
| `ready` | Ready | `ready` |
| `in_progress` | In Progress | `executing` |
| `in_review` | In Review | `pending_review` |
| `done` | Done | `approved` |

### All Internal States (14 total)

**Idle States:**
- `backlog` - Task created but not prioritized
- `ready` - Task prioritized and ready to start
- `blocked` - Task cannot proceed

**Active States:**
- `executing` - Work in progress
- `execution_done` - Work completed, awaiting next step
- `qa_refining` - QA refinement in progress
- `qa_testing` - QA testing in progress
- `qa_passed` - QA testing passed
- `qa_failed` - QA testing failed
- `pending_review` - Awaiting review
- `revision_needed` - Review requested changes

**Terminal States:**
- `approved` - Task completed and approved
- `failed` - Task failed
- `cancelled` - Task cancelled

### State Machine Transitions (from Rust)

Key transitions relevant to reviews:
- `ExecutionDone` → `QaRefining`, `PendingReview`
- `QaTesting` → `QaPassed`, `QaFailed`
- `PendingReview` → `Approved`, `RevisionNeeded`
- Terminal states → `Ready` (re-open)

### Locked Columns (Drag-Drop Validation)

Defined in `src/components/tasks/TaskBoard/validation.ts`:

**Cannot drag from:**
- `in_progress` (maps to `executing`)
- `in_review` (maps to `pending_review`)

**Cannot drop to:**
- `done` (maps to `approved`)
- `in_progress` (maps to `executing`)
- `in_review` (maps to `pending_review`)

These columns are system-managed - transitions must go through the state machine, not manual drag-drop.

### Locked Groups (New - State-Level Validation)

With multi-state columns, we need group-level locking in addition to column-level. Groups represent states within a column.

| Column | State/Group | Drag From? | Drop To? | Reason |
|--------|-------------|------------|----------|--------|
| Ready | `ready` | Yes | Yes | User can prioritize and start work |
| Ready | `revision_needed` | Yes | No | User can start re-work, but only review process can add here |
| In Progress | `executing` | No | No | System-managed (agent working) |
| In Progress | `re_executing` | No | No | System-managed (agent revising) |
| In Review | `pending_review` | No | No | System-managed (awaiting AI) |
| In Review | `reviewing` | No | No | System-managed (AI working) |
| In Review | `review_passed` | No | No | User interacts via Approve/Revise buttons, not drag |

**Implementation note:** Validation rules need to check both column AND state when determining if drag-drop is allowed.

### Key Files

| Purpose | File | Lines |
|---------|------|-------|
| TS Status enum | `src/types/status.ts` | 10-25 |
| Rust Status enum | `src-tauri/src/domain/entities/status.rs` | 14-44 |
| TS Workflow config | `src/types/workflow.ts` | 166-178 |
| Rust Workflow config | `src-tauri/src/domain/entities/workflow.rs` | 94-110 |
| Column component | `src/components/tasks/TaskBoard/Column.tsx` | 91 |
| Drag-drop validation | `src/components/tasks/TaskBoard/validation.ts` | 13-16 |
| Transition rules | `src-tauri/src/domain/entities/status.rs` | 49-76 |

---

## Proposed Review States

The current system has only `pending_review` mapping to the "In Review" column. We need new states to model the AI-powered review process, similar to how QA has multiple states (`qa_refining`, `qa_testing`, `qa_passed`, `qa_failed`).

### New States for AI Review

| State | Column | Description |
|-------|--------|-------------|
| `pending_review` | In Review | Task awaiting AI reviewer to pick it up (existing) |
| `reviewing` | In Review | AI agent is actively reviewing the task (NEW) |
| `review_passed` | In Review | AI approved; awaiting human confirmation (NEW) |
| `revision_needed` | Ready | AI/human requested revision; ready for re-execution (existing, new column mapping) |
| `re_executing` | In Progress | Worker is revising based on review feedback (NEW) |

### State Transitions

```
┌─────────────┐      ┌──────────────────────┐      ┌─────────────────────────────┐      ┌──────────┐
│   Ready     │      │     In Progress      │      │         In Review           │      │   Done   │
│   Column    │      │       Column         │      │          Column             │      │  Column  │
├─────────────┤      ├──────────────────────┤      ├─────────────────────────────┤      ├──────────┤
│             │      │                      │      │                             │      │          │
│  ┌───────┐  │      │  ┌────────────────┐  │      │  ┌───────────────────────┐  │      │          │
│  │ ready │──┼──────┼─▶│   executing    │──┼──────┼─▶│    pending_review     │  │      │          │
│  └───────┘  │      │  └────────────────┘  │      │  └───────────┬───────────┘  │      │          │
│             │      │                      │      │              │              │      │          │
│             │      │                      │      │              ▼              │      │          │
│             │      │                      │      │  ┌───────────────────────┐  │      │          │
│             │      │                      │      │  │      reviewing        │  │      │          │
│             │      │                      │      │  │    (AI working)       │  │      │          │
│             │      │                      │      │  └───────────┬───────────┘  │      │          │
│             │      │                      │      │              │              │      │          │
│             │      │                      │      │        ┌─────┴─────┐        │      │          │
│             │      │                      │      │        ▼           ▼        │      │          │
│             │      │                      │      │  ┌──────────┐ ┌──────────┐  │      │          │
│             │      │                      │      │  │  review  │ │ revision │  │      │          │
│             │      │                      │      │  │  passed  │ │  needed  │  │      │          │
│             │      │                      │      │  └────┬─────┘ └────┬─────┘  │      │          │
│             │      │                      │      │       │            │        │      │          │
│             │      │                      │      └───────┼────────────┼────────┘      │          │
│             │      │                      │              │            │               │          │
│             │      │                      │   Human      │            │               │          │
│             │      │                      │   approves   │            │               │          │
│             │      │                      │              ▼            │               │          │
│             │      │                      │        ┌──────────┐       │               │ ┌──────┐ │
│             │      │                      │        │ approved │───────┼───────────────┼▶│ done │ │
│             │      │                      │        └──────────┘       │               │ └──────┘ │
│             │      │                      │              ▲            │               │          │
│             │      │                      │              │            │               │          │
│  ┌────────┐ │      │  ┌────────────────┐  │              │            │               │          │
│  │revision│◀┼──────┼──│  re_executing  │◀─┼──────────────┼────────────┘               │          │
│  │_needed │─┼──────┼─▶│                │──┼──────────────┘                            │          │
│  └────────┘ │      │  └────────────────┘  │   (back to pending_review)               │          │
│             │      │                      │                                           │          │
└─────────────┘      └──────────────────────┘                                           └──────────┘

Human can also request revision from review_passed → revision_needed → re_executing → pending_review
```

### Key Design Decisions

1. **AI Review is a prerequisite for human approval**
   - Task cannot go directly from `pending_review` → `approved`
   - Must pass through AI review first: `pending_review` → `reviewing` → `review_passed`
   - Human then confirms: `review_passed` → `approved`

2. **Revision paths**
   - AI requests revision: `reviewing` → `revision_needed` (Ready column) → `re_executing` (In Progress) → `pending_review`
   - Human requests revision: `review_passed` → `revision_needed` → `re_executing` → `pending_review`
   - The cycle continues until human approves
   - Max revision cycles: configurable, default 5 (add to Review settings card)

3. **Multi-state columns with grouping**
   - Multiple states can map to the same column
   - Cards are grouped by state within each column
   - This provides visibility into *why* a task is in that column without adding columns

4. **Distinct states for revision work**
   - `revision_needed` (in Ready) vs `ready` (in Ready) - distinguishes fresh work from revisions
   - `re_executing` (in In Progress) vs `executing` (in In Progress) - distinguishes first attempt from revision
   - Allows tracking revision cycles and gives context to workers picking up tasks

5. **Locked groups (state-level drag-drop validation)**
   - Just like columns can be locked, individual state groups within columns can be locked
   - Most system-managed states are locked (all of In Progress and In Review)
   - User can only freely drag from `ready` and `revision_needed` groups
   - Prevents accidental state corruption while allowing legitimate user actions

### Column Mapping Update (Multi-State per Column)

| Column ID | Display Name | Internal Statuses | Grouping Purpose |
|-----------|--------------|-------------------|------------------|
| `draft` | Draft | `backlog` | - |
| `ready` | Ready | `ready`, `revision_needed` | Fresh vs. Needs Revision |
| `in_progress` | In Progress | `executing`, `re_executing` | First attempt vs. Revision |
| `in_review` | In Review | `pending_review`, `reviewing`, `review_passed` | AI review stages |
| `done` | Done | `approved` | - |

**Key insight:** Using distinct states that map to the same column provides visibility into *why* the task is there without adding more columns.

### Complete State List (Current vs Proposed)

| Current State | Keep? | Proposed Change |
|---------------|-------|-----------------|
| `backlog` | Yes | No change |
| `ready` | Yes | No change |
| `blocked` | Yes | No change |
| `executing` | Yes | No change (first attempt) |
| `execution_done` | Remove | Transitional state can be eliminated (see below) |
| `qa_refining` | Yes | No change |
| `qa_testing` | Yes | No change |
| `qa_passed` | Yes | No change |
| `qa_failed` | Yes | No change |
| `pending_review` | Yes | No change |
| `revision_needed` | Yes | Maps to Ready column (was unmapped) |
| `approved` | Yes | No change |
| `failed` | Yes | No change |
| `cancelled` | Yes | No change |

**New States:**

| New State | Column | Purpose |
|-----------|--------|---------|
| `reviewing` | In Review | AI agent actively reviewing |
| `review_passed` | In Review | AI approved, awaiting human |
| `re_executing` | In Progress | Worker revising after failed review |

---

## UI Considerations

### Grouping Across All Multi-State Columns

Cards should be visually grouped by state within each column. This provides immediate context about *why* a task is in that column.

#### Ready Column

```
┌─────────────────────────────┐
│           Ready             │
├─────────────────────────────┤
│ ▾ Fresh Tasks (3)           │
│   ┌─────────────────┐       │
│   │ Task A          │       │
│   └─────────────────┘       │
│   ┌─────────────────┐       │
│   │ Task B          │       │
│   └─────────────────┘       │
│   ┌─────────────────┐       │
│   │ Task C          │       │
│   └─────────────────┘       │
├─────────────────────────────┤
│ ▾ Needs Revision (2)        │
│   ┌─────────────────┐       │
│   │ Task D  ↩️       │       │
│   │ "Fix auth bug"  │       │
│   └─────────────────┘       │
│   ┌─────────────────┐       │
│   │ Task E  ↩️       │       │
│   │ "Add tests"     │       │
│   └─────────────────┘       │
└─────────────────────────────┘
```

#### In Progress Column

```
┌─────────────────────────────┐
│        In Progress          │
├─────────────────────────────┤
│ ▾ First Attempt (2)         │
│   ┌─────────────────┐       │
│   │ Task F  🔄      │       │
│   └─────────────────┘       │
│   ┌─────────────────┐       │
│   │ Task G  🔄      │       │
│   └─────────────────┘       │
├─────────────────────────────┤
│ ▾ Revising (1)              │
│   ┌─────────────────┐       │
│   │ Task H  🔁      │       │
│   │ Attempt #2      │       │
│   └─────────────────┘       │
└─────────────────────────────┘
```

#### In Review Column

```
┌─────────────────────────────┐
│        In Review            │
├─────────────────────────────┤
│ ▾ Waiting for AI (2)        │
│   ┌─────────────────┐       │
│   │ Task I          │       │
│   └─────────────────┘       │
│   ┌─────────────────┐       │
│   │ Task J          │       │
│   └─────────────────┘       │
├─────────────────────────────┤
│ ▾ AI Reviewing (1)          │
│   ┌─────────────────┐       │
│   │ Task K  🔄      │       │
│   └─────────────────┘       │
├─────────────────────────────┤
│ ▾ Ready for Approval (1)    │
│   ┌─────────────────┐       │
│   │ Task L  ✓ AI    │       │
│   │ [Approve] [Revise]      │
│   └─────────────────┘       │
└─────────────────────────────┘
```

### Visual Differentiators

| State | Column | Badge/Icon | Color Accent | Group Label |
|-------|--------|------------|--------------|-------------|
| `ready` | Ready | None | Neutral | "Fresh Tasks" |
| `revision_needed` | Ready | ↩️ Retry | Orange/Warning | "Needs Revision" |
| `executing` | In Progress | 🔄 Spinner | Blue | "First Attempt" |
| `re_executing` | In Progress | 🔁 Cycle | Orange | "Revising" |
| `pending_review` | In Review | Clock | Neutral | "Waiting for AI" |
| `reviewing` | In Review | 🔄 Spinner | Blue | "AI Reviewing" |
| `review_passed` | In Review | ✓ AI | Green | "Ready for Approval" |

### Human Actions

For tasks in `review_passed` state:
- **Approve** button → transitions to `approved` (Done column)
- **Request Revision** button → transitions to `revision_needed` (Ready column)

For tasks in `revision_needed` state:
- Clicking the task shows review feedback from AI/human
- Starting execution transitions to `re_executing`
- Shows revision attempt count (e.g., "Attempt #2")

### Task Metadata

~~Track on each task:~~
~~- `revision_count: number` - how many times task has been sent back for revision~~
~~- `revision_feedback: string[]` - array of feedback from each revision request~~

**Decision:** No additional task metadata needed. Derive from existing `review_notes` table:

| Data | How to Derive |
|------|---------------|
| Revision count | `COUNT(review_notes) WHERE outcome = 'changes_requested' AND task_id = ?` |
| Revision feedback | `SELECT notes FROM review_notes WHERE task_id = ? ORDER BY created_at` |

**Frontend access:**
- `useTaskStateHistory(taskId)` hook
- `get_task_state_history` command

**Worker agent access (NEW MCP tool):**
- `get_review_notes(task_id)` - Dedicated tool for workers to fetch revision feedback
- Worker must call this before starting re-execution work
- See "MCP Tool: get_review_notes" section below

---

## Settings Configuration

### Review Settings Card (existing UI)

Add to the Review settings card:

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `maxRevisionCycles` | number | 5 | Maximum revision attempts before task is escalated/failed |

When revision count >= `maxRevisionCycles`:
- Query: `COUNT(review_notes) WHERE outcome = 'changes_requested' AND task_id = ?`
- If exceeded: Task transitions to `failed` state
- Notification sent to user
- Task shows "Max revisions exceeded" indicator

---

## Resolved Questions

- [x] **Should `review_failed` be a visible state or immediately transition?**
  - **Decision:** `review_failed` is a visible state that maps to the Ready column
  - Provides visibility into tasks that need revision vs. fresh tasks
  - Same approach for `re_executing` in the In Progress column

- [x] **Should there be a way to skip AI review (manual override)?**
  - **Decision:** No. AI review is mandatory.

- [x] **How do we handle the case where the human disagrees with AI approval?**
  - **Decision:** Human has final say. When AI marks `review_passed`, human must still click Approve to transition to Done. Human can click "Request Revision" instead if they disagree.

- [x] **Should revision attempt count be tracked and displayed?**
  - **Decision:** Yes. Store as task metadata. May integrate into re-execution flow later (e.g., provide context to worker about previous attempts).

- [x] **What's the max revision cycles before escalation/failure?**
  - **Decision:** Configurable in settings, default to 5 attempts. Add to the existing Review settings card in the UI.

- [x] **Should grouping be collapsible in the UI?**
  - **Decision:** Yes. Groups within columns should be collapsible.

- [x] **How does the existing `revision_needed` state relate to the new `review_failed`?**
  - **Decision:** Consolidate. Keep `revision_needed` as the state name (it already exists). No need for a separate `review_failed` state.

- [x] **How long does AI review typically take? Do we need timeout handling?**
  - **Decision:** Defer to Supervisor system (planned separately). Out of scope for this design.

---

## Implementation: Remove `execution_done` State

Currently `execution_done` is a transitional state that immediately auto-transitions to either `qa_refining` or `pending_review` based on `qa_enabled`. It can be eliminated.

### Current Flow
```
executing --[ExecutionComplete]--> execution_done --[auto]--> qa_refining OR pending_review
```

### Proposed Flow
```
executing --[ExecutionComplete]--> qa_refining OR pending_review (directly)
```

### Files to Modify

| File | Line(s) | Change |
|------|---------|--------|
| `src-tauri/src/domain/state_machine/machine.rs` | 130 | Change `ExecutionComplete => Response::Transition(State::ExecutionDone)` to check `qa_enabled` and transition to `QaRefining` or `PendingReview` directly |
| `src-tauri/src/domain/state_machine/machine.rs` | 144-151 | Remove `execution_done()` method |
| `src-tauri/src/domain/state_machine/machine.rs` | 288 | Remove `State::ExecutionDone => self.execution_done(event)` dispatch |
| `src-tauri/src/domain/state_machine/machine.rs` | 344, 367 | Remove `ExecutionDone` name mappings |
| `src-tauri/src/domain/state_machine/transition_handler.rs` | 295-305 | Remove `ExecutionDone` case from `check_auto_transition()` |
| `src-tauri/src/domain/entities/status.rs` | 18 | Remove `ExecutionDone` from `InternalStatus` enum |
| `src-tauri/src/domain/entities/status.rs` | 59 | Remove `ExecutionDone => &[QaRefining, PendingReview]` valid transitions |
| `src-tauri/src/domain/entities/status.rs` | 357-362 | Remove `execution_done_transitions()` test |
| `src/types/status.ts` | 15 | Remove `"execution_done"` from `InternalStatusSchema` |
| `src/types/status.ts` | 46-53 | Remove from `ACTIVE_STATUSES` |
| `src/hooks/useTaskExecutionState.ts` | 39 | Remove `execution_done` from phase check |

### Logic Change in `machine.rs`

The `executing()` method needs to handle `ExecutionComplete` with QA branching:

```rust
// In executing() method, line ~130
TaskEvent::ExecutionComplete => {
    if self.context.qa_enabled {
        Response::Transition(State::QaRefining)
    } else {
        Response::Transition(State::PendingReview)
    }
}
```

### Test Updates

| Test File | Test Name | Change |
|-----------|-----------|--------|
| `status.rs` | `execution_done_transitions()` | Remove |
| `transition_handler.rs` | `test_execution_done_auto_transition_to_qa_refining()` | Update to test `executing` → `qa_refining` directly |
| `transition_handler.rs` | `test_execution_done_auto_transition_to_pending_review_without_qa()` | Update to test `executing` → `pending_review` directly |
| `transition_handler.rs` | `test_execution_done_with_qa_prep_complete_skips_wait()` | Update |
| `transition_handler.rs` | `test_execution_done_to_pending_review_starts_ai_review()` | Update |

- [x] **What data does the AI reviewer produce? (comments, suggestions, stored where?)**
  - **Finding:** We already have comprehensive review infrastructure.
  - AI reviewer stores feedback in the `notes` field of `Review` and/or creates `ReviewNote` entries.
  - See "Existing Review Infrastructure" section below.

---

## Existing Review Infrastructure

We already have a well-designed review system in place.

### Database Tables

**`reviews`** - Individual review sessions
```sql
CREATE TABLE reviews (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    reviewer_type TEXT NOT NULL,     -- 'ai' or 'human'
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, approved, changes_requested, rejected
    notes TEXT,                      -- Reviewer feedback (arbitrary text)
    created_at DATETIME,
    completed_at DATETIME
);
```

**`review_notes`** - Review history (multiple per task)
```sql
CREATE TABLE review_notes (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    reviewer TEXT NOT NULL,          -- 'ai' or 'human'
    outcome TEXT NOT NULL,           -- approved, changes_requested, rejected
    notes TEXT,                      -- Feedback text
    created_at DATETIME
);
```

**`review_actions`** - Actions taken during review
```sql
CREATE TABLE review_actions (
    id TEXT PRIMARY KEY,
    review_id TEXT NOT NULL,
    action_type TEXT NOT NULL,       -- created_fix_task, moved_to_backlog, approved
    target_task_id TEXT,             -- For fix task creation
    created_at DATETIME
);
```

### Rust Entities

**File:** `src-tauri/src/domain/entities/review.rs`

| Entity | Purpose |
|--------|---------|
| `Review` | Main review record with `reviewer_type: ReviewerType` (Ai\|Human), `status: ReviewStatus`, `notes: Option<String>` |
| `ReviewNote` | Historical feedback per task with `reviewer: ReviewerType`, `outcome: ReviewOutcome`, `notes: Option<String>` |
| `ReviewAction` | Actions taken (created fix task, moved to backlog, approved) |

### Enums

| Enum | Values |
|------|--------|
| `ReviewerType` | `Ai`, `Human` |
| `ReviewStatus` | `Pending`, `Approved`, `ChangesRequested`, `Rejected` |
| `ReviewOutcome` | `Approved`, `ChangesRequested`, `Rejected` |
| `ReviewActionType` | `CreatedFixTask`, `MovedToBacklog`, `Approved` |

### Key Methods

```rust
impl Review {
    fn new(project_id, task_id, reviewer_type: ReviewerType) -> Self;
    fn approve(&mut self, notes: Option<String>);
    fn request_changes(&mut self, notes: String);
    fn reject(&mut self, notes: String);
}

impl ReviewNote {
    fn new(task_id, reviewer: ReviewerType, outcome: ReviewOutcome) -> Self;
    fn with_notes(task_id, reviewer, outcome, notes: String) -> Self;
}
```

### How AI Reviewer Stores Feedback

1. **During review:** AI creates a `Review` with `reviewer_type: Ai`, status `Pending`
2. **On completion:** AI calls `review.approve(Some("detailed feedback"))` or `review.request_changes("issues found...")`
3. **History tracking:** Create `ReviewNote::with_notes()` entries for each review attempt
4. **Multiple reviews:** Task can have multiple `ReviewNote` entries over time (revision cycles)

### No Schema Changes Needed

The existing infrastructure supports our new states:
- `reviewing` state: Active `Review` with status `Pending`, `reviewer_type: Ai`
- `review_passed` state: `Review` with status `Approved`, awaiting human confirmation
- `revision_needed` state: `Review` with status `ChangesRequested`
- Revision history: Multiple `ReviewNote` entries per task

---

## Existing Implementation Analysis

### What Already Exists

| Component | Status | Location |
|-----------|--------|----------|
| Review Commands (Tauri) | ✅ Full | `src-tauri/src/commands/review_commands.rs` |
| Reviewer Agent | ✅ Defined | `ralphx-plugin/agents/reviewer.md` |
| MCP Tool Definition | ✅ Full | `ralphx-mcp-server/src/tools.ts:292-334` |
| MCP Tool Scoping | ✅ Full | `ralphx-mcp-server/src/tools.ts:356-391` |
| HTTP Endpoint Route | ✅ Registered | `src-tauri/src/http_server.rs:327` |
| HTTP Handler Logic | ❌ **STUB** | `src-tauri/src/http_server.rs:879-901` |
| Domain Entities | ✅ Full | `src-tauri/src/domain/entities/review.rs` |
| Tool Input Schema | ✅ Full | `src-tauri/src/domain/tools/complete_review.rs` |
| Review Service | ✅ Exists | `src-tauri/src/application/review_service.rs` |
| SQLite Repository | ✅ Exists | `src-tauri/src/infrastructure/sqlite/sqlite_review_repo.rs` |

### Reviewer Agent Definition

**File:** `ralphx-plugin/agents/reviewer.md`

```yaml
name: ralphx-reviewer
description: Reviews code changes for quality and correctness
model: sonnet
max_iterations: 10
tools: [Read, Grep, Glob, Bash]  # Filesystem tools only
skills: [code-review-checklist]
```

**Review Process:**
1. Gather Context
2. Examine Changes (git diff)
3. Run Checks (tests + linting)
4. Identify Issues
5. Provide Feedback via `complete_review` MCP tool

**Output Format:**
- `status`: approve | needs_changes | escalate
- `confidence`: float
- `issues`: array of findings
- `suggestions`: array of improvements

### MCP Tool: `complete_review`

**Definition** (`ralphx-mcp-server/src/tools.ts:292-334`):
```typescript
{
  name: "complete_review",
  description: "Submit a code review decision...",
  inputSchema: {
    task_id: string,
    decision: "approved" | "needs_changes" | "escalate",
    feedback: string,
    issues?: [{ severity, file, line, description }]
  }
}
```

**Tool Scoping** (`tools.ts:356-391`):
```typescript
TOOL_ALLOWLIST = {
  "reviewer": ["complete_review"],  // Only this tool
  // ... other agents
}
```

---

## Enhanced Scoping: Task-Level Enforcement

### Problem

Current scoping only controls *which tools* an agent can use. It doesn't prevent an agent from operating on the wrong task. An agent could accidentally (or maliciously) call `complete_review` with a different task ID than the one it was assigned.

### Solution: Environment-Based Task Scoping

Pass the assigned task ID as an environment variable when spawning the agent:

```bash
RALPHX_AGENT_TYPE=reviewer RALPHX_TASK_ID=task-123 claude --agent reviewer ...
```

The MCP server then validates that any tool call's `task_id` parameter matches the assigned task.

### Implementation

**1. Set Environment Variable When Spawning**

When the system spawns a reviewer agent for a specific task:

```rust
// In agent spawning code
let env_vars = vec![
    ("RALPHX_AGENT_TYPE", "reviewer"),
    ("RALPHX_TASK_ID", task_id.as_str()),
];
spawn_claude_agent(config, env_vars);
```

**2. Validate in MCP Server**

**File:** `ralphx-mcp-server/src/index.ts`

```typescript
const RALPHX_TASK_ID = process.env.RALPHX_TASK_ID;

function validateTaskScope(toolName: string, args: Record<string, unknown>): string | null {
  // Only validate tools that have task_id parameter
  const taskScopedTools = ["complete_review", "update_task", "add_task_note"];

  if (!taskScopedTools.includes(toolName)) {
    return null; // No validation needed
  }

  if (!RALPHX_TASK_ID) {
    return null; // No task scope set, allow (backward compatibility)
  }

  const providedTaskId = args.task_id as string;
  if (providedTaskId !== RALPHX_TASK_ID) {
    return `ERROR: Task scope violation. You are assigned to task "${RALPHX_TASK_ID}" but attempted to modify task "${providedTaskId}". Please use the correct task ID.`;
  }

  return null; // Validation passed
}

// In tool handler
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  // Check tool allowlist (existing)
  if (!isToolAllowed(name)) {
    return { content: [{ type: "text", text: `Tool not available for ${AGENT_TYPE}` }], isError: true };
  }

  // Check task scope (NEW)
  const scopeError = validateTaskScope(name, args);
  if (scopeError) {
    return { content: [{ type: "text", text: scopeError }], isError: true };
  }

  // Proceed with tool execution
  // ...
});
```

**3. Helpful Error Message**

When validation fails, return actionable feedback:

```
ERROR: Task scope violation.
You are assigned to task "task-abc-123" but attempted to modify task "task-xyz-789".

Your assigned task details:
- Task ID: task-abc-123
- You should only call complete_review with this task_id.

Please correct your tool call and try again.
```

### Scope Validation Matrix

| Tool | Has task_id? | Validate? |
|------|--------------|-----------|
| `complete_review` | Yes | ✅ Enforce |
| `update_task` | Yes | ✅ Enforce |
| `add_task_note` | Yes | ✅ Enforce |
| `get_task_details` | Yes | ✅ Enforce (read-only but still scoped) |
| `get_task_context` | Yes | ✅ Enforce |
| `list_tasks` | No (project-level) | ❌ Skip |
| `suggest_task` | No (creates new) | ❌ Skip |
| `create_task_proposal` | No (ideation) | ❌ Skip |

### Benefits

1. **Correctness** - Agents can't accidentally modify wrong tasks
2. **Security** - Prevents rogue agent behavior
3. **Debugging** - Clear error messages help identify issues
4. **Auditability** - Easy to trace which agent was assigned to which task

### Files to Modify

| File | Change |
|------|--------|
| `ralphx-mcp-server/src/index.ts` | Add `validateTaskScope()` function and call it |
| `src-tauri/src/infrastructure/agents/claude/` | Pass `RALPHX_TASK_ID` env var when spawning |
| `src-tauri/src/application/execution_chat_service.rs` | Include task_id in spawn config |

---

## MCP Tool: `get_review_notes`

### Purpose

Dedicated tool for worker agents to fetch previous review feedback before starting re-execution work. Separate from `get_task_context` to keep concerns clean.

### Tool Definition

**File:** `ralphx-mcp-server/src/tools.ts`

```typescript
{
  name: "get_review_notes",
  description: "Get all review feedback for a task. Call this before re-executing a task to understand what needs to be fixed.",
  inputSchema: {
    type: "object",
    properties: {
      task_id: {
        type: "string",
        description: "The task ID to get review notes for"
      }
    },
    required: ["task_id"]
  }
}
```

### Response Schema

```typescript
{
  task_id: string,
  revision_count: number,              // Derived count
  max_revisions: number,               // From settings
  reviews: [
    {
      id: string,
      reviewer: "ai" | "human",
      outcome: "approved" | "changes_requested" | "rejected",
      notes: string | null,
      created_at: string               // ISO datetime
    }
  ]
}
```

### Tool Scoping

Add to worker allowlist:

```typescript
TOOL_ALLOWLIST = {
  worker: [
    "get_task_context",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "get_review_notes",    // NEW
    // ... step tools
  ],
  // ...
}
```

### Worker Agent Instructions Update

**File:** `ralphx-plugin/agents/worker.md`

Add to worker instructions:

```markdown
## Before Starting Re-Execution Work

If this task is a revision (check `RALPHX_TASK_STATE` env var equals `re_executing`):

1. **MUST** call `get_task_context(task_id)` - understand the task
2. **MUST** call `get_review_notes(task_id)` - understand what to fix
3. Read all previous feedback carefully
4. Address each issue mentioned in the review notes
5. Do not repeat the same mistakes
```

### HTTP Endpoint

**File:** `src-tauri/src/application/http_server.rs`

```rust
// Route
.route("/api/review_notes/:task_id", get(get_review_notes))

// Handler
async fn get_review_notes(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<ReviewNotesResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(task_id);

    // Get all review notes for task
    let notes = state.review_repo
        .get_notes_by_task(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get settings for max_revisions
    let settings = state.review_settings_repo
        .get()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Count revisions (changes_requested outcomes)
    let revision_count = notes.iter()
        .filter(|n| n.outcome == ReviewOutcome::ChangesRequested)
        .count();

    Ok(Json(ReviewNotesResponse {
        task_id: task_id.to_string(),
        revision_count: revision_count as u32,
        max_revisions: settings.max_revision_cycles,
        reviews: notes.into_iter().map(|n| n.into()).collect(),
    }))
}
```

### Tauri Commands (Human Actions)

**File:** `src-tauri/src/commands/review_commands.rs`

| Command | Purpose |
|---------|---------|
| `get_pending_reviews(project_id)` | List pending reviews |
| `get_review_by_id(review_id)` | Get single review |
| `get_reviews_by_task_id(task_id)` | Get reviews for task |
| `approve_review(input)` | Human approves review |
| `request_changes(input)` | Human requests changes |
| `reject_review(input)` | Human rejects review |

---

## Critical Gap: HTTP Handler Implementation

**File:** `src-tauri/src/http_server.rs:879-901`

The `complete_review` HTTP handler is a **STUB**:

```rust
async fn complete_review(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CompleteReviewRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    // TODO: Implement review submission logic
    // For now, just acknowledge the review
    Ok(Json(SuccessResponse {
        success: true,
        message: "Review submitted successfully".to_string(),
    }))
}
```

### What the Handler Needs to Do

1. **Parse request** - Extract task_id, decision, feedback, issues
2. **Validate** - Ensure task is in `reviewing` state
3. **Create/Update Review record** - Use `ReviewService`
4. **Handle decision outcomes:**
   - `approved` → Create Review with status Approved, transition task to `review_passed`
   - `needs_changes` → Create Review with status ChangesRequested, transition task to `revision_needed`
   - `escalate` → Create Review with escalation flag, notify supervisor
5. **Create ReviewNote** - Store feedback in history
6. **Trigger state transition** - Via TransitionHandler
7. **Emit events** - `review:completed`, `task:status_changed`
8. **Return response** - Success/failure with details

---

## Implementation Tasks

### 1. Implement `complete_review` HTTP Handler

**File:** `src-tauri/src/http_server.rs`

```rust
async fn complete_review(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CompleteReviewRequest>,
) -> Result<Json<CompleteReviewResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(req.task_id);

    // 1. Get task and validate state
    let task = state.task_repo.get_by_id(&task_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    if task.internal_status != InternalStatus::Reviewing {
        return Err((StatusCode::BAD_REQUEST, "Task not in reviewing state".to_string()));
    }

    // 2. Create review record
    let review = Review::new(task.project_id.clone(), task_id.clone(), ReviewerType::Ai);

    // 3. Process decision
    match req.decision.as_str() {
        "approved" => {
            review.approve(Some(req.feedback.clone()));
            // Transition to review_passed
        },
        "needs_changes" => {
            review.request_changes(req.feedback.clone());
            // Transition to revision_needed
        },
        "escalate" => {
            // Handle escalation
        },
        _ => return Err((StatusCode::BAD_REQUEST, "Invalid decision".to_string())),
    }

    // 4. Save review
    state.review_repo.create(review).await?;

    // 5. Create review note for history
    let note = ReviewNote::with_notes(task_id, ReviewerType::Ai, outcome, req.feedback);
    state.review_repo.create_note(note).await?;

    // 6. Trigger state transition
    // ... use TransitionHandler

    // 7. Emit events
    state.event_emitter.emit("review:completed", ...);

    Ok(Json(CompleteReviewResponse { success: true, ... }))
}
```

### 2. Add New States to State Machine

**Files to modify:**
- `src-tauri/src/domain/entities/status.rs` - Add `Reviewing`, `ReviewPassed`, `ReExecuting`
- `src-tauri/src/domain/state_machine/machine.rs` - Add handlers
- `src-tauri/src/domain/state_machine/events.rs` - Add events
- `src/types/status.ts` - Add to TypeScript enum

### 3. Update Transition Rules

Add valid transitions:
- `PendingReview` → `Reviewing` (AI picks up)
- `Reviewing` → `ReviewPassed` (AI approves)
- `Reviewing` → `RevisionNeeded` (AI requests changes)
- `ReviewPassed` → `Approved` (Human approves)
- `ReviewPassed` → `RevisionNeeded` (Human requests changes)
- `RevisionNeeded` → `ReExecuting` (Worker picks up)
- `ReExecuting` → `PendingReview` (Re-submitted)

### 4. Wire Up State Entry Actions

| State | Entry Action |
|-------|--------------|
| `Reviewing` | Mark review as in-progress |
| `ReviewPassed` | Notify human for approval |
| `RevisionNeeded` | Increment revision count, store feedback |
| `ReExecuting` | Spawn worker with revision context |

### 5. Update Column Mapping

Modify workflow configuration to support multi-state columns:
- `src/types/workflow.ts`
- `src-tauri/src/domain/entities/workflow.rs`

---

## Frontend UI Analysis

### What Exists

| Component | Status | Location |
|-----------|--------|----------|
| Reviews Button | ✅ Full | `src/App.tsx:632-677` |
| ReviewsPanel | ✅ Full | `src/components/reviews/ReviewsPanel.tsx` |
| ReviewCard | ✅ Full | `src/components/reviews/ReviewCard.tsx` |
| ReviewStatusBadge | ✅ Full | `src/components/reviews/ReviewStatusBadge.tsx` |
| ReviewNotesModal | ✅ Defined | `src/components/reviews/ReviewNotesModal.tsx` |
| DiffViewer | ✅ Full | `src/components/diff/DiffViewer.tsx` |
| StateHistoryTimeline | ✅ Full | `src/components/tasks/StateHistoryTimeline.tsx` |
| usePendingReviews | ✅ Full | `src/hooks/useReviews.ts` |
| useReviewsByTaskId | ✅ Full | `src/hooks/useReviews.ts` |
| useTaskStateHistory | ✅ Full | `src/hooks/useReviews.ts` |
| useGitDiff | ⚠️ Mock | `src/hooks/useGitDiff.ts` |
| reviewStore | ✅ Full | `src/stores/reviewStore.ts` |
| Approve/Reject Mutations | ❌ Missing | TODO comments in App.tsx |

### Current UI Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Header Toolbar                                                          │
│   [Reviews ⓷]  ← Button with pending count badge                       │
└───────┬─────────────────────────────────────────────────────────────────┘
        │ click
        ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                                              │ ReviewsPanel (slide-in)  │
│                                              │ ┌─────────────────────┐  │
│            Main Content                      │ │ Reviews        [X]  │  │
│            (Kanban, etc.)                    │ ├─────────────────────┤  │
│                                              │ │ [All] [AI] [Human]  │  │
│                                              │ ├─────────────────────┤  │
│                                              │ │ ┌─────────────────┐ │  │
│                                              │ │ │ ReviewCard      │ │  │
│                                              │ │ │ - Task title    │ │  │
│                                              │ │ │ - Status badge  │ │  │
│                                              │ │ │ - Notes preview │ │  │
│                                              │ │ │ [Diff] [✓] [↩]  │ │  │
│                                              │ │ └─────────────────┘ │  │
│                                              │ │ ┌─────────────────┐ │  │
│                                              │ │ │ ReviewCard...   │ │  │
│                                              │ │ └─────────────────┘ │  │
│                                              │ └─────────────────────┘  │
└──────────────────────────────────────────────┴──────────────────────────┘
```

### ReviewsPanel Features

- **Header**: Title, count badge, close button
- **Filter Tabs**: All | AI (Bot icon) | Human (User icon) with counts
- **List View**: ScrollArea with ReviewCard items
- **Detail View**: DiffViewer with file tree + diff content
- **Actions**: View Diff, Request Changes, Approve buttons

### ReviewCard Shows

- Task title
- ReviewStatusBadge (pending/approved/changes_requested/rejected)
- ReviewerTypeIndicator (AI/Human icon)
- FixAttemptCounter (e.g., "Attempt 2/5")
- Notes preview (truncated with "View Full")
- Action buttons (when pending)

### DiffViewer Features

- **Tabs**: Changes | History
- **Changes Tab**: File tree (left) + Diff view (right)
- **History Tab**: Commit list (left) + Commit diff (right)
- File status indicators (A=added, M=modified, D=deleted, R=renamed)
- Syntax highlighting
- "Open in IDE" button

### What's Missing

1. **Approve/Reject Mutations**
   ```tsx
   // App.tsx has TODO comments:
   onApprove={(reviewId) => {
     console.log("Approve review:", reviewId);
     // TODO: Call approveReview mutation
   }}
   ```

2. **ReviewNotesModal Integration** - Defined but not wired up to ReviewsPanel

3. **Git Backend Integration** - useGitDiff returns mock data

4. **API Wrappers** - Need `api.reviews.approve()` and `api.reviews.requestChanges()`

---

## UI Design Decision: Hybrid Approach ✓

**Decision:** Keep floating panel for list, open large modal for detailed review.

### User Flow

```
1. Click "Reviews" button     2. See pending list         3. Click "Review" on card
   in header toolbar             in floating panel           to open detail modal
        │                              │                            │
        ▼                              ▼                            ▼
┌──────────────────┐         ┌──────────────────┐         ┌────────────────────────┐
│ [Reviews ⓷]     │         │ ReviewsPanel     │         │ ReviewDetailModal      │
│                  │   →     │ ┌──────────────┐ │   →     │ (90% viewport)         │
│                  │         │ │ ReviewCard   │ │         │                        │
│                  │         │ │ [Review]     │ │         │ Context | DiffViewer   │
│                  │         │ └──────────────┘ │         │ [Approve] [Request ↩]  │
└──────────────────┘         └──────────────────┘         └────────────────────────┘
```

### ReviewDetailModal Layout

```
┌───────────────────────────────────────────────────────────────────────┐
│ Review: Fix authentication bug                                  [X]   │
├────────────────────────────────┬──────────────────────────────────────┤
│ Context (300px fixed)          │ Changes (flex-1)                     │
│ ┌────────────────────────────┐ │ ┌──────────────────────────────────┐ │
│ │ Task Details               │ │ │ [Changes] [History]              │ │
│ │ ┌────────────────────────┐ │ │ ├──────────────────────────────────┤ │
│ │ │ Description            │ │ │ │ File Tree    │ Diff View        │ │
│ │ │ Priority: High         │ │ │ │ ├─ auth.ts   │ @@ -10,6 +10,12  │ │
│ │ │ Category: Feature      │ │ │ │ ├─ login.tsx │ + new code       │ │
│ │ └────────────────────────┘ │ │ │ └─ test.ts   │ - old code       │ │
│ └────────────────────────────┘ │ │              │                   │ │
│ ┌────────────────────────────┐ │ │              │ [Open in IDE]     │ │
│ │ AI Review                  │ │ └──────────────────────────────────┘ │
│ │ ✓ Passed                   │ │                                      │
│ │ Confidence: 92%            │ │                                      │
│ │ "Implementation looks..."  │ │                                      │
│ └────────────────────────────┘ │                                      │
│ ┌────────────────────────────┐ │                                      │
│ │ Review History             │ │                                      │
│ │ • AI approved 5m ago       │ │                                      │
│ │ • Worker submitted 10m ago │ │                                      │
│ └────────────────────────────┘ │                                      │
│ ┌────────────────────────────┐ │                                      │
│ │ Revision: Attempt 1/5      │ │                                      │
│ └────────────────────────────┘ │                                      │
├────────────────────────────────┴──────────────────────────────────────┤
│                        [Approve ✓]          [Request Changes ↩]       │
└───────────────────────────────────────────────────────────────────────┘
```

### Benefits

- Quick access via floating panel from any page
- Full context and diff space in modal
- No routing changes needed
- Reuses existing DiffViewer component
- Clear action buttons in modal footer

---

## Specialized Task Detail Views by State

When a user clicks a task card in the Kanban, the detail view should be tailored to the task's internal state. Different states need different information and actions.

### Layout Architecture

The app already has a split layout:

```
┌────────────────────────────────────┬────────────────────────────────┐
│  LEFT PANE (resizable)             │  RIGHT PANE                    │
│                                    │                                │
│  Kanban Board                      │  Chat Component                │
│       ↓ (on task click)            │  (already exists with          │
│  Task Detail View                  │   context switching)           │
│  (state-specific)                  │                                │
└────────────────────────────────────┴────────────────────────────────┘
```

- **Left pane**: Shows Kanban OR Task Detail View (not both)
- **Right pane**: Chat component with context switching (execution/review/task/etc.)
- Task detail views should NOT include chat - that's handled by the right pane

### State → View Mapping

| Internal State | Column | Detail View (Left) | Chat Context (Right) | Chat Mode |
|----------------|--------|--------------------|-----------------------|-----------|
| `backlog` | Draft | Description, steps, priority | `task` | Live |
| `ready` | Ready | Description, steps, ready indicator | `task` | Live |
| `revision_needed` | Ready | Description, steps, **review feedback** | `task` | Live |
| `executing` | In Progress | Step tracker, progress bar | `execution` | **Live** |
| `re_executing` | In Progress | Step tracker, **previous feedback** | `execution` | **Live** |
| `pending_review` | In Review | Work summary, waiting indicator | `execution` | Historical |
| `reviewing` | In Review | Review progress indicator | `review` | **Live** |
| `review_passed` | In Review | AI summary, **Approve/Revise buttons** | `review` | Historical |
| `approved` | Done | Final summary, history | `review` or `execution` | Historical |

### View Components (Left Pane Only)

These components render in the LEFT pane when a task is clicked. The RIGHT pane shows the Chat component with appropriate context.

#### BasicTaskDetail (backlog, ready)
```
┌─────────────────────────────────────┐
│ Task Title                    [Edit]│
├─────────────────────────────────────┤
│ Status: Ready    Priority: High     │
│ Category: Feature                   │
├─────────────────────────────────────┤
│ Description                         │
│ Lorem ipsum dolor sit amet...       │
├─────────────────────────────────────┤
│ Steps                               │
│ ☐ Step 1                           │
│ ☐ Step 2                           │
│ ☐ Step 3                           │
└─────────────────────────────────────┘
```
Chat context: `task` | Mode: Live

#### RevisionTaskDetail (revision_needed)
```
┌─────────────────────────────────────┐
│ Task Title              [Attempt 2] │
├─────────────────────────────────────┤
│ ⚠️ REVISION NEEDED                  │
├─────────────────────────────────────┤
│ Review Feedback to Address          │
│ ┌─────────────────────────────────┐ │
│ │ 🤖 AI Review - 5m ago           │ │
│ │ "Missing error handling in..."  │ │
│ │                                 │ │
│ │ Issues:                         │ │
│ │ • auth.ts:42 - null check       │ │
│ │ • login.tsx - validation        │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ Description                         │
│ Lorem ipsum...                      │
├─────────────────────────────────────┤
│ Steps                               │
│ ✓ Step 1 (completed)               │
│ ☐ Step 2 (needs revision)          │
│ ☐ Step 3                           │
└─────────────────────────────────────┘
```
Chat context: `task` | Mode: Live

#### ExecutionTaskDetail (executing, re_executing)
```
┌─────────────────────────────────────┐
│ Task Title                 [Live 🔴]│
├─────────────────────────────────────┤
│ Progress: Step 2 of 4               │
│ ████████████░░░░░░░░ 50%           │
├─────────────────────────────────────┤
│ (if re_executing)                   │
│ ⚠️ Addressing Review Feedback       │
│ ┌─────────────────────────────────┐ │
│ │ "Missing error handling..."     │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ Steps                               │
│ ✓ Step 1                           │
│ ▶ Step 2 (in progress)             │
│ ☐ Step 3                           │
│ ☐ Step 4                           │
├─────────────────────────────────────┤
│ Description                         │
│ Lorem ipsum...                      │
└─────────────────────────────────────┘
```
Chat context: `execution` | Mode: **Live** (agent streams here)

#### ReviewingTaskDetail (reviewing)
```
┌─────────────────────────────────────┐
│ Task Title            [Reviewing 🔄]│
├─────────────────────────────────────┤
│ 🤖 AI REVIEW IN PROGRESS            │
├─────────────────────────────────────┤
│ Review Steps                        │
│ ✓ Gathering context                 │
│ ▶ Examining changes                 │
│ ☐ Running checks                    │
│ ☐ Generating feedback               │
├─────────────────────────────────────┤
│ Files Under Review                  │
│ • src/auth.ts                       │
│ • src/login.tsx                     │
│ • tests/auth.test.ts                │
├─────────────────────────────────────┤
│ Description                         │
│ Lorem ipsum...                      │
└─────────────────────────────────────┘
```
Chat context: `review` | Mode: **Live** (can interact with reviewer)

#### HumanReviewTaskDetail (review_passed)
```
┌─────────────────────────────────────┐
│ Task Title            [Ready ✓]     │
├─────────────────────────────────────┤
│ ✅ AI REVIEW PASSED                 │
│ Awaiting your approval              │
├─────────────────────────────────────┤
│ AI Review Summary                   │
│ ┌─────────────────────────────────┐ │
│ │ 🤖 Confidence: 94%              │ │
│ │ "Implementation looks good..."  │ │
│ │                                 │ │
│ │ ✓ Tests passing                 │ │
│ │ ✓ No linting errors             │ │
│ │ ✓ Code follows patterns         │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ Changes Made                        │
│ [View Diff →]                       │
├─────────────────────────────────────┤
│ Previous Attempts (if any)          │
│ • Attempt 1: Changes requested      │
├─────────────────────────────────────┤
│ [Approve ✓]      [Request Changes ↩]│
└─────────────────────────────────────┘
```
Chat context: `review` | Mode: Historical (read-only log of review)

#### WaitingTaskDetail (pending_review)
```
┌─────────────────────────────────────┐
│ Task Title         [Pending Review] │
├─────────────────────────────────────┤
│ ⏳ WAITING FOR AI REVIEWER          │
├─────────────────────────────────────┤
│ Work Completed                      │
│ ┌─────────────────────────────────┐ │
│ │ Submitted 2m ago                │ │
│ │ 4 files changed                 │ │
│ │ All steps completed             │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ Steps                               │
│ ✓ Step 1                           │
│ ✓ Step 2                           │
│ ✓ Step 3                           │
├─────────────────────────────────────┤
│ Description                         │
│ Lorem ipsum...                      │
└─────────────────────────────────────┘
```
Chat context: `execution` | Mode: Historical (read-only log of execution)

#### CompletedTaskDetail (approved)
```
┌─────────────────────────────────────┐
│ Task Title              [Done ✓]    │
├─────────────────────────────────────┤
│ ✅ COMPLETED                        │
│ Approved 2h ago by Human            │
├─────────────────────────────────────┤
│ Final Summary                       │
│ Lorem ipsum...                      │
├─────────────────────────────────────┤
│ Review History                      │
│ ┌─────────────────────────────────┐ │
│ │ ✓ Human approved - 2h ago      │ │
│ │ ✓ AI approved - 2h ago         │ │
│ │ ↩ AI changes requested - 5h ago│ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│ [View Final Diff]  [Reopen Task]    │
└─────────────────────────────────────┘
```
Chat context: `review` | Mode: Historical

### Implementation Approach: View Registry Pattern ✓

```tsx
const TASK_DETAIL_VIEWS: Record<InternalStatus, React.ComponentType<TaskDetailProps>> = {
  backlog: BasicTaskDetail,
  ready: BasicTaskDetail,
  blocked: BasicTaskDetail,
  revision_needed: RevisionTaskDetail,
  executing: ExecutionTaskDetail,
  re_executing: ExecutionTaskDetail,
  pending_review: WaitingTaskDetail,
  reviewing: ReviewingTaskDetail,
  review_passed: HumanReviewTaskDetail,
  qa_refining: QATaskDetail,
  qa_testing: QATaskDetail,
  qa_passed: QATaskDetail,
  qa_failed: QATaskDetail,
  approved: CompletedTaskDetail,
  failed: FailedTaskDetail,
  cancelled: CancelledTaskDetail,
};

function TaskDetailPanel({ task }: { task: Task }) {
  const ViewComponent = TASK_DETAIL_VIEWS[task.internalStatus] ?? BasicTaskDetail;
  return <ViewComponent task={task} />;
}
```

**Benefits:**
- Easy to add new views for new states
- Clear mapping, easy to reason about
- Each view component is self-contained
- Fallback to BasicTaskDetail for unmapped states

### Data Requirements by View

| View | Hooks/Data Needed |
|------|-------------------|
| BasicTaskDetail | `useTask`, `useTaskSteps` |
| RevisionTaskDetail | `useTask`, `useTaskSteps`, **`useTaskStateHistory`** |
| ExecutionTaskDetail | `useTask`, `useTaskSteps`, `useExecutionStream`, **`useTaskStateHistory`** (if re_executing) |
| HumanReviewTaskDetail | `useTask`, `useReviewsByTaskId`, `useGitDiff` |
| CompletedTaskDetail | `useTask`, `useTaskStateHistory`, `useGitDiff` |

### Integration Points

1. **TaskCard click** → Opens appropriate detail view based on `task.internalStatus`
2. **State transitions** → Detail view updates reactively via TanStack Query invalidation
3. **ReviewsPanel "Review" button** → Opens `HumanReviewTaskDetail` (or ReviewDetailModal)
4. **Column grouping** → Groups show tasks with matching views

---

## Live Chat with AI Reviewer

### Concept

When a task is in `reviewing` state, the human can interact with the AI reviewer in real-time via the chat panel. This allows:
- Asking questions about the review in progress
- Providing additional context the AI might need
- Guiding the review focus ("pay attention to X")
- Intervening if the AI is going down the wrong path

### Chat Context Types

The chat system already supports multiple context types. Add `review` context:

| Context Type | Agent | Use Case |
|--------------|-------|----------|
| `execution` | worker | Human ↔ Worker during task execution |
| `review` | reviewer | Human ↔ AI Reviewer during review |
| `task` | chat-task | General task discussion |
| `project` | chat-project | Project-level queries |
| `ideation` | orchestrator-ideation | Ideation session |

### Full Layout: Task Detail + Chat

The existing split layout handles this. When viewing a task in `reviewing` state:

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Header                                                                  │
├────────────────────────────────────┬────────────────────────────────────┤
│ LEFT PANE                          │ RIGHT PANE                         │
│ ReviewingTaskDetail component      │ Chat component (context: review)   │
│ ┌────────────────────────────────┐ │ ┌────────────────────────────────┐ │
│ │ Task Title       [Reviewing 🔄]│ │ │ 🤖 Starting review...          │ │
│ │                                │ │ │                                │ │
│ │ 🤖 AI REVIEW IN PROGRESS       │ │ │ 🤖 Examining git diff...       │ │
│ │                                │ │ │                                │ │
│ │ Review Steps                   │ │ │ 🤖 Found 3 files changed.      │ │
│ │ ✓ Gathering context            │ │ │    Running tests now...        │ │
│ │ ▶ Examining changes            │ │ │                                │ │
│ │ ☐ Running checks               │ │ │ 👤 Check error handling in     │ │
│ │ ☐ Generating feedback          │ │ │    auth.ts please              │ │
│ │                                │ │ │                                │ │
│ │ Files Under Review             │ │ │ 🤖 Good point, I'll pay extra  │ │
│ │ • src/auth.ts                  │ │ │    attention to error cases... │ │
│ │ • src/login.tsx                │ │ │                                │ │
│ │                                │ │ ├────────────────────────────────┤ │
│ │ Description                    │ │ │ Type a message...         [↵]  │ │
│ └────────────────────────────────┘ │ └────────────────────────────────┘ │
└────────────────────────────────────┴────────────────────────────────────┘
  ← resizable →
```

- **Left pane**: `ReviewingTaskDetail` component (state-specific view)
- **Right pane**: Existing `Chat` component with `context: review`, mode: live

### Implementation

**1. Add Review Context to Chat Types**

**File:** `src/types/chat.ts`

```typescript
export type ChatContextType =
  | 'execution'
  | 'review'      // NEW
  | 'task'
  | 'project'
  | 'ideation';

export interface ReviewChatContext {
  type: 'review';
  taskId: string;
  reviewId: string;
}
```

**2. Update ExecutionChatService for Review**

**File:** `src-tauri/src/application/execution_chat_service.rs`

Add support for spawning/resuming review agent conversations:
- `spawn_reviewer_with_persistence(task_id, review_id)`
- `queue_message_to_reviewer(review_id, message)`
- Reuse existing message queue infrastructure

**3. Task Detail Views (Left Pane)**

Task detail components render in the left pane only. They don't include chat - the existing layout handles that.

```tsx
// ReviewingTaskDetail renders in LEFT pane
function ReviewingTaskDetail({ task }: { task: Task }) {
  const review = useActiveReview(task.id);

  return (
    <div className="p-4 space-y-4">
      <TaskHeader task={task} badge="Reviewing 🔄" />
      <ReviewProgressIndicator review={review} />
      <FilesUnderReview files={review.files} />
      <TaskDescription description={task.description} />
    </div>
  );
}

// ExecutionTaskDetail renders in LEFT pane
function ExecutionTaskDetail({ task }: { task: Task }) {
  const { steps, progress } = useTaskSteps(task.id);
  const reviewNotes = useTaskStateHistory(task.id); // For re_executing

  return (
    <div className="p-4 space-y-4">
      <TaskHeader task={task} badge={task.internalStatus === 're_executing' ? 'Revising 🔁' : 'Live 🔴'} />
      <ProgressBar progress={progress} />
      {task.internalStatus === 're_executing' && (
        <RevisionFeedbackBanner notes={reviewNotes} />
      )}
      <StepsList steps={steps} />
      <TaskDescription description={task.description} />
    </div>
  );
}
```

**4. Chat Context Switching (Right Pane)**

The existing chat component needs to switch context based on task state:

```tsx
// In the layout component that manages both panes
function TaskLayout({ task }: { task: Task }) {
  const chatContext = useMemo(() => {
    switch (task.internalStatus) {
      case 'executing':
      case 're_executing':
        return { type: 'execution', id: task.id };
      case 'reviewing':
        return { type: 'review', id: task.id };
      default:
        return { type: 'task', id: task.id };
    }
  }, [task.internalStatus, task.id]);

  const chatMode = useMemo(() => {
    const liveStates = ['executing', 're_executing', 'reviewing'];
    return liveStates.includes(task.internalStatus) ? 'live' : 'historical';
  }, [task.internalStatus]);

  return (
    <SplitPane>
      <LeftPane>
        <TaskDetailView task={task} />
      </LeftPane>
      <RightPane>
        <ChatComponent context={chatContext} mode={chatMode} />
      </RightPane>
    </SplitPane>
  );
}
```

### Message Flow

```
Human types message in ChatPanel
    ↓
Frontend calls queueMessageToAgent(contextType, contextId, message)
    ↓
Backend adds to ExecutionMessageQueue
    ↓
Agent (via --resume) picks up queued message
    ↓
Agent responds, streams back
    ↓
Frontend receives via execution:chunk events
    ↓
ChatPanel displays response
```

### Benefits

- Human can guide AI reviewer in real-time
- Catch issues early before review completes
- Provide context AI might not have
- Collaborative human-AI review process
- Same UX pattern as execution chat (consistency)

### Chat as Live Interaction + Historical Log

The chat panel serves dual purposes:

**During Process (Live)**
- Human can send messages
- Agent streams responses
- Real-time collaboration

**After Process Completes (Read-Only Log)**
- Chat input disabled
- Full conversation history preserved
- User can scroll through to see what happened
- Serves as audit trail / detailed log

```
┌────────────────────────────────────────────────────────────────────┐
│ Review Completed ✓                                                 │
├────────────────────────────────────────────────────────────────────┤
│ 🤖 Starting review of task-123...                                  │
│                                                                    │
│ 🤖 Examining git diff... Found 3 files changed.                    │
│                                                                    │
│ 👤 Make sure to check the error handling in auth.ts                │
│                                                                    │
│ 🤖 Good point. I'll pay extra attention to error handling.         │
│    Checking auth.ts now...                                         │
│                                                                    │
│ 🤖 Running tests... All 24 tests passing.                          │
│                                                                    │
│ 🤖 Review complete. Decision: APPROVED                             │
│    - Code follows project patterns                                 │
│    - Error handling is adequate                                    │
│    - Tests cover the changes                                       │
├────────────────────────────────────────────────────────────────────┤
│ [Chat ended - Review completed]                     (input disabled)│
└────────────────────────────────────────────────────────────────────┘
```

### State-Based Chat Behavior

| Task State | Chat Mode | Input | Purpose |
|------------|-----------|-------|---------|
| `executing` | Live | Enabled | Collaborate with worker |
| `re_executing` | Live | Enabled | Collaborate with worker |
| `reviewing` | Live | Enabled | Collaborate with reviewer |
| `pending_review` | Historical | Disabled | View execution log |
| `review_passed` | Historical | Disabled | View review log |
| `revision_needed` | Historical | Disabled | View review feedback log |
| `approved` | Historical | Disabled | View full history |

### Implementation: Chat Mode Detection

```tsx
function ChatPanel({ contextType, contextId, taskId }: ChatPanelProps) {
  const task = useTask(taskId);

  const isLive = useMemo(() => {
    if (contextType === 'execution') {
      return task.internalStatus === 'executing' || task.internalStatus === 're_executing';
    }
    if (contextType === 'review') {
      return task.internalStatus === 'reviewing';
    }
    return false;
  }, [contextType, task.internalStatus]);

  return (
    <div className="flex flex-col h-full">
      {/* Message history - always visible */}
      <ScrollArea className="flex-1">
        <MessageList messages={messages} />
      </ScrollArea>

      {/* Input - conditional */}
      {isLive ? (
        <ChatInput onSend={handleSend} />
      ) : (
        <div className="p-3 text-center text-muted border-t">
          Chat ended - {contextType === 'review' ? 'Review' : 'Execution'} completed
        </div>
      )}
    </div>
  );
}
```

### Note: Execution Chat May Need Same Treatment

This pattern (live → historical) should be verified/implemented for execution chat as well:
- `executing` / `re_executing` → Live chat with worker
- After completion → Read-only log of what happened

**Files to check:**
- `src/components/chat/ChatPanel.tsx` - Does it handle read-only mode?
- `src/components/tasks/ExecutionTaskDetail.tsx` - Does it show historical chat?
- Conversation persistence - Are execution conversations saved for later viewing?

---

## Implementation Plan: UI

### Phase 1: Wire Up Existing Components

1. **Add mutations** to `src/hooks/useReviews.ts`:
   ```tsx
   useApproveReview(reviewId, notes?)
   useRequestChanges(reviewId, notes, fixDescription?)
   ```

2. **Add API wrappers** to `src/lib/tauri.ts`:
   ```tsx
   api.reviews.approve(reviewId, input)
   api.reviews.requestChanges(reviewId, input)
   ```

3. **Connect App.tsx handlers** to actual mutations

4. **Integrate ReviewNotesModal** into ReviewsPanel for feedback collection

### Phase 2: Large Review Modal

1. **Create ReviewDetailModal** component:
   - Full-width modal (max-w-7xl or 90vw)
   - Left pane: Task context, AI review summary, history
   - Right pane: DiffViewer (existing component)
   - Footer: Approve / Request Changes buttons

2. **Update ReviewsPanel** to open modal instead of inline detail view

3. **Wire up modal actions** to mutations

### Phase 3: Git Backend Integration

1. **Implement Tauri commands**:
   - `get_git_changes(projectPath)` → FileChange[]
   - `get_git_commits(projectPath, limit)` → Commit[]
   - `get_file_diff(projectPath, filePath, commitSha?)` → DiffData

2. **Replace mock data** in useGitDiff with Tauri calls

### Phase 4: State Integration

1. **Connect to new review states** (`reviewing`, `review_passed`, `revision_needed`)
2. **Update TaskCard** to show review state badges
3. **Add grouping UI** in columns for multi-state display

---

## Open Questions

(None remaining - all questions resolved)
