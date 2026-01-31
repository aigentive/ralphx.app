# Plan: Add Dedicated `Escalated` State for Review System

## Problem Statement

When the AI reviewer chooses `escalate`, it currently transitions to `RevisionNeeded` (same as `needs_changes`), which triggers automatic re-execution. This defeats the purpose of escalation — forcing human decision.

**Current behavior:**
- `approved` → `review_passed` (human confirms)
- `needs_changes` → `revision_needed` (auto re-execute)
- `escalate` → `revision_needed` ❌ (auto re-execute — wrong!)

**Desired behavior:**
- `escalate` → `escalated` (human must decide — like `review_passed`)

## Implementation Approach

Add a new `Escalated` state that behaves like `ReviewPassed` (requires human action to proceed), but indicates the AI couldn't make the call.

---

## Changes Required

### 1. Backend: Add `Escalated` Status Variant (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(status): add Escalated variant to InternalStatus enum`

**File:** `src-tauri/src/domain/entities/status.rs`

**Changes:**
- Add `Escalated` variant to `InternalStatus` enum (after `ReviewPassed`, ~line 38)
- Update `valid_transitions()`:
  - Line 72: `Reviewing => &[ReviewPassed, RevisionNeeded, Escalated]`
  - Add: `Escalated => &[Approved, RevisionNeeded]` (same as ReviewPassed)
- Update `as_str()`: Add `InternalStatus::Escalated => "escalated"`
- Update `from_str()`: Add `"escalated" => Ok(InternalStatus::Escalated)`
- Update `all_variants()`: Add `Escalated` to the list
- Add transition tests for new state

### 2. Backend: Update State Machine Types and Transitions (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(state-machine): add Escalated state with transitions`

**File:** `src-tauri/src/domain/state_machine/machine/types.rs`

**Changes:**
1. Add `Escalated` to `State` enum (line 37, after ReviewPassed):
   ```rust
   // Review states
   PendingReview,
   Reviewing,
   ReviewPassed,
   Escalated,  // NEW
   RevisionNeeded,
   ```

2. Add dispatch case in `dispatch()` function (~line 100):
   ```rust
   State::ReviewPassed => self.review_passed(event),
   State::Escalated => self.escalated(event),  // NEW
   State::RevisionNeeded => self.revision_needed(event),
   ```

**File:** `src-tauri/src/domain/state_machine/machine/transitions.rs`

**Changes (after review_passed handler ~line 199):**
```rust
/// Escalated state - AI couldn't decide, awaiting human decision
pub fn escalated(&mut self, event: &TaskEvent) -> Response {
    match event {
        TaskEvent::HumanApprove => Response::Transition(State::Approved),
        TaskEvent::HumanRequestChanges { feedback } => {
            self.context.review_feedback = Some(feedback.clone());
            Response::Transition(State::RevisionNeeded)
        }
        TaskEvent::Cancel => Response::Transition(State::Cancelled),
        _ => Response::NotHandled,
    }
}
```

### 3. Backend: Update Status/State Conversion Functions
**Dependencies:** Task 1, Task 2
**Atomic Commit:** `feat(transition-service): add Escalated status/state mapping`

**File:** `src-tauri/src/application/task_transition_service.rs`

**Changes in `internal_status_to_state()` (lines 145-162):**
Add after line 156 (`ReviewPassed => State::ReviewPassed`):
```rust
InternalStatus::Escalated => State::Escalated,
```

**Changes in `state_to_internal_status()` (lines 168-186):**
Add after line 180 (`State::ReviewPassed => InternalStatus::ReviewPassed`):
```rust
State::Escalated => InternalStatus::Escalated,
```

### 4. Backend: Add Side Effects for Escalated State
**Dependencies:** Task 2
**Atomic Commit:** `feat(side-effects): add event and notification for Escalated state`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

**Changes (after ReviewPassed handler ~line 219):**
```rust
State::Escalated => {
    // Emit 'review:escalated' event
    self.machine.context.services.event_emitter
        .emit("review:escalated", &self.machine.context.task_id)
        .await;

    // Notify user that AI escalated review
    self.machine.context.services.notifier
        .notify_with_message(
            "review:escalated",
            &self.machine.context.task_id,
            "AI review escalated. Please review and decide.",
        )
        .await;
}
```

### 5. Backend: Update HTTP Handler for Escalate
**Dependencies:** Task 1, Task 3
**Atomic Commit:** `feat(reviews): transition to Escalated on escalate decision`

**File:** `src-tauri/src/http_server/handlers/reviews.rs`

**Changes at lines 141-158 (split the combined match arm):**
```rust
// BEFORE (line 150):
ReviewToolOutcome::NeedsChanges | ReviewToolOutcome::Escalate => {
    transition_service
        .transition_task(&task_id, InternalStatus::RevisionNeeded)
        ...
}

// AFTER:
ReviewToolOutcome::NeedsChanges => {
    transition_service
        .transition_task(&task_id, InternalStatus::RevisionNeeded)
        ...
    InternalStatus::RevisionNeeded
}
ReviewToolOutcome::Escalate => {
    transition_service
        .transition_task(&task_id, InternalStatus::Escalated)
        ...
    InternalStatus::Escalated
}
```

### 6. Backend: Update approve_task/request_task_changes Validation
**Dependencies:** Task 1
**Atomic Commit:** `feat(reviews): allow approve/request_changes from Escalated status`

**File:** `src-tauri/src/http_server/handlers/reviews.rs`

**Changes:**
- Line 250: Update condition to accept both `ReviewPassed` AND `Escalated`
- Line 331: Same update for `request_task_changes`

```rust
// BEFORE (line 250):
if task.internal_status != InternalStatus::ReviewPassed {

// AFTER:
if task.internal_status != InternalStatus::ReviewPassed
    && task.internal_status != InternalStatus::Escalated {
    // Updated error message to include escalated status
```

Also update error messages to mention `escalated` as a valid status.

### 7. Frontend: Add Status to TypeScript Types (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(types): add escalated to InternalStatus type`

**File:** `src/types/status.ts`

**Changes:**
- Line 21: Add `"escalated"` to `InternalStatusSchema` (after `"review_passed"`)
- Lines 71-75: Add `"escalated"` to `REVIEW_STATUSES`
- Lines 48-57: Add `"escalated"` to `ACTIVE_STATUSES`

### 8. Frontend: Create EscalatedTaskDetail Component
**Dependencies:** Task 7
**Atomic Commit:** `feat(components): add EscalatedTaskDetail view`

**File:** `src/components/tasks/detail-views/EscalatedTaskDetail.tsx` (NEW)

Create a component similar to `HumanReviewTaskDetail` but with different messaging:
- Banner: "⚠️ AI ESCALATED TO HUMAN" (instead of "AI REVIEW PASSED")
- Description: "AI reviewer couldn't make a decision - needs your input"
- Same Approve/Request Changes buttons as HumanReviewTaskDetail
- Display escalation reason from review notes

### 9. Frontend: Update View Registry
**Dependencies:** Task 7, Task 8
**Atomic Commit:** `feat(TaskDetailPanel): register EscalatedTaskDetail view`

**File:** `src/components/tasks/TaskDetailPanel.tsx`

**Changes at lines 69-94:**
```typescript
const TASK_DETAIL_VIEWS: Record<InternalStatus, React.ComponentType<TaskDetailProps>> = {
  // ... existing mappings ...
  escalated: EscalatedTaskDetail,
};
```

### 10. Frontend: Update Workflow Column Grouping
**Dependencies:** Task 7
**Atomic Commit:** `feat(workflow): add Escalated column group`

**File:** `src/types/workflow.ts`

**Changes at lines 383-392 (add new group after "ready_approval"):**
```typescript
{
  id: "ready_approval",
  label: "Ready for Approval",
  statuses: ["review_passed"],
  icon: "CheckCircle",
  accentColor: "hsl(var(--success))",
  canDragFrom: false,
  canDropTo: false,
},
// ADD NEW GROUP:
{
  id: "escalated",
  label: "Escalated",
  statuses: ["escalated"],
  icon: "AlertTriangle",
  accentColor: "hsl(var(--warning))",
  canDragFrom: false,
  canDropTo: false,
},
```

### 11. MCP: Update Tools Description
**Dependencies:** Task 1, Task 7
**Atomic Commit:** `docs(mcp): update approve_task and request_task_changes descriptions`

**File:** `ralphx-plugin/ralphx-mcp-server/src/tools.ts`

**Changes:**
- Lines 454-458: Update `approve_task` description to mention `escalated` status
- Lines 475-480: Update `request_task_changes` description to mention `escalated` status

---

## Task Dependency Graph

```
Task 1 (Backend: Status Variant) ──────┬──────────────────────────────────────────┐
                                       │                                          │
                                       ▼                                          ▼
Task 2 (Backend: State Machine) ──► Task 3 (Backend: Conversion) ──► Task 5 (Backend: HTTP Handler)
         │                                                                        │
         ▼                                                                        │
Task 4 (Backend: Side Effects)                                                    │
                                                                                  ▼
                                                                       Task 6 (Backend: Validation)

Task 7 (Frontend: Types) ──────────┬──────────────────────────────────────────────┐
                                   │                                              │
                                   ▼                                              ▼
Task 8 (Frontend: Component) ──► Task 9 (Frontend: Registry)      Task 10 (Frontend: Workflow)

Task 1 + Task 7 ──────────────────────────────────────────────────► Task 11 (MCP: Docs)
```

**Parallelization opportunities:**
- Tasks 1 and 7 can run in parallel (backend vs frontend, no dependencies)
- Tasks 4 and 5 can run in parallel (both depend on Task 2, no mutual dependency)
- Tasks 9 and 10 can run in parallel (both depend on Task 7, no mutual dependency)

---

## Files Modified Summary

| Layer | File | Type |
|-------|------|------|
| Backend | `src-tauri/src/domain/entities/status.rs` | Modify |
| Backend | `src-tauri/src/http_server/handlers/reviews.rs` | Modify |
| Backend | `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | Modify |
| Backend | `src-tauri/src/application/task_transition_service.rs` | Modify |
| Backend | `src-tauri/src/domain/state_machine/machine/types.rs` | Modify |
| Backend | `src-tauri/src/domain/state_machine/machine/transitions.rs` | Modify |
| Frontend | `src/types/status.ts` | Modify |
| Frontend | `src/components/tasks/detail-views/EscalatedTaskDetail.tsx` | Create |
| Frontend | `src/components/tasks/TaskDetailPanel.tsx` | Modify |
| Frontend | `src/types/workflow.ts` | Modify |
| MCP | `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | Modify |

---

## Verification

1. **Unit tests (Backend):**
   - `cargo test` for new status transitions
   - Test `Reviewing → Escalated` transition
   - Test `Escalated → Approved` transition
   - Test `Escalated → RevisionNeeded` transition

2. **Type checking:**
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `npm run lint && npm run typecheck`

3. **Manual verification:**
   - Create a task that requires escalation (security-sensitive, unclear requirements)
   - Verify AI review chooses "escalate"
   - Verify task lands in "Escalated" state, NOT "RevisionNeeded"
   - Verify "In Review" column shows the escalated task
   - Verify Approve/Request Changes buttons work from escalated state

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Use absolute paths to project root for all lock file and git operations
- Lock is stale only if SAME content AND >30 seconds old
