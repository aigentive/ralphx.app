# RalphX - Phase 9: Review & Supervision

## Overview

This phase implements the review system (AI and human review workflows), human-in-the-loop features (AskUserQuestion handling, review points, task injection, loop interruption), and the UI components for review management. The review system ensures quality through AI-powered code review with human escalation, while human-in-the-loop features enable user control over the autonomous execution flow.

## Dependencies

- Phase 1 (Foundation) - TypeScript types, Rust entities
- Phase 2 (Data Layer) - Repository pattern, SQLite
- Phase 3 (State Machine) - `pending_review`, `revision_needed` states
- Phase 5 (Frontend Core) - Event system, stores
- Phase 6 (Kanban UI) - TaskCard integration
- Phase 7 (Agent System) - Reviewer agent definition, supervisor events

## Scope

### Included
- Review database schema (reviews, review_actions, review_notes tables)
- ReviewRepository trait and SQLite implementation
- ReviewService for orchestrating review workflow
- AI Review flow (auto-review on `execution_done` or `qa_passed`)
- Human Review UI (Reviews panel, diff viewer, approval actions)
- Fix task creation and approval workflow
- AskUserQuestion UI component
- Review points (before destructive, after complex, manual)
- Task injection during execution
- Loop interruption (pause, resume, stop)
- State history timeline in task detail view
- Configuration settings for review behavior

### Excluded
- Supervisor pattern detection (Phase 7)
- Event bus infrastructure (Phase 7)
- Orchestrator chat interface (Phase 10)
- Custom workflow schemas (Phase 11)

---

## Detailed Requirements

### Review System Overview

When a task status becomes `execution_done` (or `qa_passed` if QA is enabled), an AI Review agent automatically verifies the work.

**State Transitions for Review:**
```
execution_done ──► pending_review ──► approved (terminal)
        │                  │
        │ [qa_enabled]     └──► revision_needed ──► executing (rework)
        ▼
   qa_refining → qa_testing → qa_passed ──► pending_review
```

### AI Review (Automatic)

**What AI Review Checks:**
- Code compiles/builds without errors
- Tests pass (if applicable)
- Task acceptance criteria met
- No obvious regressions introduced
- Code quality (basic linting)

**AI Review Outcomes:**
| Outcome | Action | Description |
|---------|--------|-------------|
| **Pass** | Status → `approved` | Work verified, task complete |
| **Fail (fixable)** | Creates fix task, original → `revision_needed` | Auto-create fix task |
| **Escalate** | Status → `blocked`, notify user | Needs human decision |
| **Uncertain** | Status → `blocked`, notify user | Low confidence |

**When AI Escalates:**
- Code works but design decision needed
- Multiple valid approaches, user should choose
- Security-sensitive changes
- Breaking changes to public API
- AI confidence below threshold

### Review Configuration Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `ai_review_enabled` | `true` | Enable automatic AI review after task completion |
| `ai_review_auto_fix` | `true` | Auto-create fix tasks (false = send to backlog) |
| `require_fix_approval` | `false` | Require human approval of AI-proposed fixes |
| `require_human_review` | `false` | Require human approval even after AI approves |
| `max_fix_attempts` | `3` | Max AI fix attempts before giving up → backlog |

### Fix Task Approval Flow

When `require_fix_approval: true`:
```
AI Review finds issues
       ↓
Creates fix task with status: pending_approval
       ↓
Human sees in Reviews panel
       ↓
┌──────┴──────────┬───────────────┐
↓                 ↓               ↓
Approve      Reject w/        Dismiss
   ↓         feedback            ↓
planned          ↓            backlog
   ↓             ↓           (give up)
executes    AI proposes
            alternative
               ↓
         (repeat until
          max_fix_attempts
          or approved)
```

**Rejection with Feedback:**
When human rejects a proposed fix:
1. Original fix task → `rejected` status
2. AI receives human's feedback in context
3. AI proposes new fix task considering feedback
4. Attempt counter increments
5. If `attempt >= max_fix_attempts`:
   - Original task → `backlog`
   - Notification: "Max fix attempts reached, needs manual intervention"

### Human Review (Manual)

User reviews work via the Reviews panel:
- See what changed (diff viewer integration)
- Add notes/feedback (stored in DB)
- Actions: **Approve**, **Request Changes** (creates task), **Reject** (marks failed)

### Database Schema

**Reviews Table:**
```sql
CREATE TABLE reviews (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  task_id TEXT NOT NULL REFERENCES tasks(id),
  reviewer_type TEXT NOT NULL,     -- 'ai' or 'human'
  status TEXT DEFAULT 'pending',   -- 'pending', 'approved', 'changes_requested', 'rejected'
  notes TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  completed_at DATETIME
);
```

**Review Actions Table:**
```sql
CREATE TABLE review_actions (
  id TEXT PRIMARY KEY,
  review_id TEXT NOT NULL REFERENCES reviews(id),
  action_type TEXT NOT NULL,       -- 'created_fix_task', 'moved_to_backlog', 'approved'
  target_task_id TEXT,             -- ID of created fix task, if applicable
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**Review Notes Table:**
```sql
CREATE TABLE review_notes (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  reviewer TEXT NOT NULL,          -- 'ai' or 'human'
  outcome TEXT NOT NULL,           -- 'approved', 'changes_requested', 'rejected'
  notes TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Reviewer Agent Prompt

```xml
<role>
You are a code reviewer evaluating completed work against requirements.
Your job is to verify quality, not to reimplement.
</role>

<task>
## Review Task: {task.title}

## Original Requirements:
{task.description}

## Acceptance Criteria:
{task.steps}

## Changes Made:
{git_diff}
</task>

<review_checklist>
1. Does the implementation meet all acceptance criteria?
2. Does the code compile/build without errors?
3. Are there any obvious bugs or regressions?
4. Is the code quality acceptable (no major issues)?
5. Are there any security concerns?
</review_checklist>

<decisions>
Based on your review, choose ONE outcome:
- APPROVE: All criteria met, code quality acceptable
- NEEDS_CHANGES: Issues found that can be fixed automatically
  - Describe the specific issues
  - Propose a fix task description
- ESCALATE: Needs human review (security-sensitive, design decision, unclear requirements)
  - Explain why human input is needed
</decisions>

<output_format>
Use the complete_review tool with:
- outcome: "approved" | "needs_changes" | "escalate"
- notes: Your detailed review notes
- fix_description: (if needs_changes) Description for fix task
</output_format>
```

### complete_review Tool Schema

```typescript
interface CompleteReviewInput {
  outcome: 'approved' | 'needs_changes' | 'escalate';
  notes: string;
  fix_description?: string;  // Required if outcome is 'needs_changes'
  escalation_reason?: string; // Required if outcome is 'escalate'
}
```

### Event Types

**Review Events:**
```typescript
interface ReviewEvent {
  taskId: string;
  reviewId: string;
  type: 'started' | 'completed' | 'needs_human' | 'fix_proposed';
  outcome?: 'approved' | 'changes_requested' | 'escalated';
}
```

**Rust Event Emission:**
```rust
pub fn emit_review_event(app: &AppHandle, task_id: &str, review_id: &str, event_type: &str) {
    app.emit("review:update", serde_json::json!({
        "taskId": task_id,
        "reviewId": review_id,
        "type": event_type,
    })).unwrap();
}
```

### Reviews Panel UI

```
┌─────────────────────────────────────────────────┐
│  Reviews                                    [x] │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │ ⚠ Task: "Add user authentication"         │ │
│  │   Status: Needs Human Review              │ │
│  │   Reason: Security-sensitive changes      │ │
│  │   [View Diff] [Approve] [Request Changes] │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │ 🔄 Task: "Fix login validation"           │ │
│  │   Status: Fix Proposed                    │ │
│  │   Attempt: 2 of 3                         │ │
│  │   [View Fix] [Approve Fix] [Reject]       │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Task Detail View - State History Timeline

```
┌─────────────────────────────────────────────────┐
│  History                                        │
│                                                 │
│  ● Approved                          2 min ago │
│    └─ by: user                                 │
│    └─ "Looks good, nice work"                  │
│                                                 │
│  ● Escalated to human review        15 min ago │
│    └─ by: ai_reviewer                          │
│    └─ "Security-sensitive: adds auth bypass"   │
│                                                 │
│  ● In Review                        18 min ago │
│    └─ by: system                               │
│                                                 │
│  ● Done                             25 min ago │
│    └─ by: ai_worker                            │
│    └─ "Completed in 3 tool calls"              │
│                                                 │
└─────────────────────────────────────────────────┘
```

### AskUserQuestion Handling

When the agent uses the `AskUserQuestion` tool, the UI must handle it specially:

**How it works:**
1. Agent calls `AskUserQuestion` tool with options
2. Execution pauses, task status → `blocked`
3. Chat UI renders interactive question component
4. User selects answer or types custom response
5. Answer sent back to agent, execution resumes

**UI Component:**
```
┌─────────────────────────────────────────────────┐
│  Agent is asking:                               │
│                                                 │
│  "Which authentication method should we use?"  │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │ ○ JWT tokens (Recommended)              │   │
│  │ ○ Session cookies                        │   │
│  │ ○ OAuth only                             │   │
│  │ ○ Other: [________________]              │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  [Submit Answer]                                │
└─────────────────────────────────────────────────┘
```

**Implementation:**
- Parse tool call parameters: `question`, `options`, `header`, `multiSelect`
- Render as radio buttons (single select) or checkboxes (multi-select)
- Always include "Other" option with text input
- On submit: resume agent with selected answer

**AskUserQuestion Schema:**
```typescript
interface AskUserQuestionPayload {
  taskId: string;
  question: string;
  header: string;
  options: Array<{
    label: string;
    description: string;
  }>;
  multiSelect: boolean;
}

interface AskUserQuestionResponse {
  taskId: string;
  selectedOptions: string[];
  customResponse?: string;
}
```

### Human-in-the-Loop Features

**Review Points:**
1. **Before Destructive** - Auto-inserted before tasks that delete files/configs
2. **After Complex** - Optional, for tasks marked as complex
3. **Manual** - User-defined review points on specific tasks

**Task Injection:**
- User can add tasks mid-loop via chat or UI
- Option: Send to **Backlog** (deferred) or **Planned** (immediate queue)
- If Planned, inserted at correct priority position
- "Make next" option → highest priority

**Loop Interruption:**
- Pause button stops after current task completes
- Resume continues from next planned task
- "Stop" cancels current execution (with cleanup)

### Execution Control Bar

```
┌─────────────────────────────────────────────────┐
│  Running: 1/2  │  Queued: 3  │  [⏸ Pause] [⏹ Stop] │
└─────────────────────────────────────────────────┘
```

- No "Start" button - tasks auto-execute when `planned`
- Shows: current/max concurrent agents
- Queued tasks count
- Global Pause toggle (stops picking up new tasks)
- Stop button (with confirmation for active tasks)

---

## Implementation Notes

### Key Design Decisions

1. **Two-tier review**: AI review first, human escalation only when needed
2. **Configurable behavior**: All review settings can be toggled per-project
3. **Fix task workflow**: AI can propose fixes, human approves or rejects
4. **Max fix attempts**: Prevents infinite fix loops (default: 3)
5. **State history audit**: Full trail of who/what changed task status

### File Size Limits

- Review service: 200 lines max
- Review panel component: 150 lines max
- AskUserQuestion component: 100 lines max
- State history timeline: 80 lines max

### Testing Strategy

- Unit tests for ReviewService logic
- Unit tests for fix task creation workflow
- Integration tests with MockAgenticClient for AI review
- Component tests for Reviews panel, AskUserQuestion
- E2E tests for full review flow

### Anti-AI-Slop Guardrails

- Reviews panel uses design system colors, not generic styles
- No purple gradients in review status badges
- Warm orange for pending review, green for approved, red for rejected

---

## Task List

```json
[
  {
    "category": "setup",
    "description": "Create reviews table migration",
    "steps": [
      "Write integration test for migration",
      "Create migration file for reviews table",
      "Define columns: id, project_id, task_id, reviewer_type, status, notes, created_at, completed_at",
      "Run migration",
      "Verify table created with correct schema"
    ],
    "passes": true
  },
  {
    "category": "setup",
    "description": "Create review_actions table migration",
    "steps": [
      "Write integration test for migration",
      "Create migration file for review_actions table",
      "Define columns: id, review_id, action_type, target_task_id, created_at",
      "Add foreign key constraint to reviews table",
      "Run migration",
      "Verify table created with correct schema"
    ],
    "passes": true
  },
  {
    "category": "setup",
    "description": "Create review_notes table migration",
    "steps": [
      "Write integration test for migration",
      "Create migration file for review_notes table",
      "Define columns: id, task_id, reviewer, outcome, notes, created_at",
      "Add foreign key constraint to tasks table",
      "Run migration",
      "Verify table created with correct schema"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Review and ReviewAction domain entities",
    "steps": [
      "Write unit tests for entity serialization/deserialization",
      "Create src-tauri/src/domain/entities/review.rs",
      "Define Review struct with all fields",
      "Define ReviewAction struct with all fields",
      "Define ReviewStatus enum: Pending, Approved, ChangesRequested, Rejected",
      "Define ReviewerType enum: Ai, Human",
      "Define ReviewActionType enum: CreatedFixTask, MovedToBacklog, Approved",
      "Implement serde traits for all types",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewNote domain entity",
    "steps": [
      "Write unit tests for entity serialization",
      "Create ReviewNote struct in review.rs",
      "Define ReviewOutcome enum: Approved, ChangesRequested, Rejected",
      "Implement serde traits",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewRepository trait",
    "steps": [
      "Write unit tests for repository methods",
      "Create src-tauri/src/domain/repositories/review_repo.rs",
      "Define async trait methods: create, get_by_id, get_by_task_id, get_pending, update",
      "Add method: add_action(review_id, action) for tracking actions",
      "Add method: add_note(note) for storing review notes",
      "Add method: get_notes_by_task_id(task_id) for history",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement SqliteReviewRepository",
    "steps": [
      "Write integration tests with test database",
      "Create src-tauri/src/infrastructure/sqlite/review_repo.rs",
      "Implement all repository methods",
      "Handle Review::from_row for SQLite deserialization",
      "Handle ReviewAction::from_row for SQLite deserialization",
      "Handle ReviewNote::from_row for SQLite deserialization",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewConfig settings",
    "steps": [
      "Write unit tests for config defaults and serialization",
      "Create src-tauri/src/domain/config/review_config.rs",
      "Define ReviewConfig struct with: ai_review_enabled, ai_review_auto_fix, require_fix_approval, require_human_review, max_fix_attempts",
      "Implement Default trait with values from master plan",
      "Add to AppConfig or project settings",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement complete_review tool for reviewer agent",
    "steps": [
      "Write unit tests for tool input validation",
      "Create src-tauri/src/domain/tools/complete_review.rs",
      "Define CompleteReviewInput struct: outcome, notes, fix_description, escalation_reason",
      "Implement validation: fix_description required if needs_changes",
      "Implement validation: escalation_reason required if escalate",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewService - core review orchestration",
    "steps": [
      "Write unit tests for review flow logic",
      "Create src-tauri/src/application/review_service.rs",
      "Implement start_ai_review(task_id) method",
      "Implement process_review_result(review_id, result) method",
      "Implement create_fix_task(original_task_id, description) method",
      "Wire up ReviewConfig to control behavior",
      "Keep file under 200 lines",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewService - fix task workflow",
    "steps": [
      "Write unit tests for fix task approval flow",
      "Add approve_fix_task(fix_task_id) method",
      "Add reject_fix_task(fix_task_id, feedback) method",
      "Implement fix attempt counter tracking",
      "Implement max_fix_attempts check with backlog fallback",
      "Add notification emission for max attempts reached",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewService - human review methods",
    "steps": [
      "Write unit tests for human review flow",
      "Add start_human_review(task_id) method",
      "Add approve_human_review(review_id, notes) method",
      "Add request_changes(review_id, notes, fix_description) method",
      "Add reject_human_review(review_id, notes) method",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Integrate ReviewService with state machine transitions",
    "steps": [
      "Write integration tests for state transitions",
      "Update TaskStateMachine entry actions for pending_review state",
      "Call ReviewService.start_ai_review when entering pending_review",
      "Handle AI review completion via process_review_result",
      "Emit review:update events on state changes",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Tauri commands for reviews",
    "steps": [
      "Write integration tests for Tauri commands",
      "Create get_pending_reviews command",
      "Create get_review_by_id command",
      "Create get_reviews_by_task_id command",
      "Create approve_review command",
      "Create request_changes command",
      "Create reject_review command",
      "Create get_task_state_history command",
      "Run tauri dev to verify commands work"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Tauri commands for fix tasks",
    "steps": [
      "Write integration tests for Tauri commands",
      "Create approve_fix_task command",
      "Create reject_fix_task command",
      "Create get_fix_task_attempts command",
      "Run tauri dev to verify commands work"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Review TypeScript types",
    "steps": [
      "Write unit tests for Zod schema validation",
      "Create src/types/review.ts",
      "Define Review, ReviewAction, ReviewNote interfaces",
      "Define ReviewStatus, ReviewerType, ReviewOutcome types",
      "Create Zod schemas for runtime validation",
      "Export all types and schemas",
      "Run npm run typecheck to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewConfig TypeScript types",
    "steps": [
      "Write unit tests for Zod schema validation",
      "Create ReviewConfig interface in src/types/review.ts",
      "Define all config fields with defaults",
      "Create Zod schema with default values",
      "Run npm run typecheck to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Tauri API wrappers for reviews",
    "steps": [
      "Write unit tests for API functions",
      "Create src/api/reviews.ts",
      "Implement getPendingReviews() function",
      "Implement getReviewById(id) function",
      "Implement getReviewsByTaskId(taskId) function",
      "Implement approveReview(reviewId, notes) function",
      "Implement requestChanges(reviewId, notes, fixDescription) function",
      "Implement rejectReview(reviewId, notes) function",
      "Implement getTaskStateHistory(taskId) function",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Tauri API wrappers for fix tasks",
    "steps": [
      "Write unit tests for API functions",
      "Add to src/api/reviews.ts",
      "Implement approveFixTask(taskId) function",
      "Implement rejectFixTask(taskId, feedback) function",
      "Implement getFixTaskAttempts(taskId) function",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement reviewStore with Zustand",
    "steps": [
      "Write unit tests for store actions",
      "Create src/stores/reviewStore.ts",
      "Define state: pendingReviews, selectedReviewId, isLoading",
      "Implement actions: fetchPendingReviews, selectReview, approveReview, requestChanges, rejectReview",
      "Use immer middleware for immutable updates",
      "Keep file under 100 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useReviews hook",
    "steps": [
      "Write unit tests for hook behavior",
      "Create src/hooks/useReviews.ts",
      "Use TanStack Query for fetching pending reviews",
      "Add usePendingReviews() hook",
      "Add useReviewsByTaskId(taskId) hook",
      "Add useTaskStateHistory(taskId) hook",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useReviewEvents hook",
    "steps": [
      "Write unit tests for hook behavior",
      "Create useReviewEvents() in src/hooks/useEvents.ts",
      "Listen for review:update Tauri events",
      "Invalidate TanStack Query cache on review events",
      "Add to EventProvider",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewStatusBadge component",
    "steps": [
      "Write component tests",
      "Create src/components/reviews/ReviewStatusBadge.tsx",
      "Display badge with status-appropriate color (pending: orange, approved: green, rejected: red)",
      "Show icon for each status",
      "Follow design system tokens",
      "Keep file under 50 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewCard component",
    "steps": [
      "Write component tests",
      "Create src/components/reviews/ReviewCard.tsx",
      "Display task title, review status, reason/notes",
      "Show reviewer type (AI/Human) indicator",
      "Add action buttons: View Diff, Approve, Request Changes",
      "For fix tasks: show attempt counter",
      "Keep file under 100 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewsPanel component",
    "steps": [
      "Write component tests",
      "Create src/components/reviews/ReviewsPanel.tsx",
      "List pending reviews using ReviewCard components",
      "Show empty state when no pending reviews",
      "Add filter tabs: All, AI Review, Human Review",
      "Keep file under 150 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ReviewNotesModal component",
    "steps": [
      "Write component tests",
      "Create src/components/reviews/ReviewNotesModal.tsx",
      "Text area for adding review notes",
      "Optional fix description field for Request Changes",
      "Submit and Cancel buttons",
      "Keep file under 80 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement StateHistoryTimeline component",
    "steps": [
      "Write component tests",
      "Create src/components/tasks/StateHistoryTimeline.tsx",
      "Fetch task state history via useTaskStateHistory hook",
      "Display timeline with status transitions",
      "Show actor (user, system, ai_worker, ai_reviewer, ai_supervisor)",
      "Show reason/notes for each transition",
      "Display relative timestamps",
      "Keep file under 80 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement TaskDetailView with state history",
    "steps": [
      "Write component tests",
      "Create src/components/tasks/TaskDetailView.tsx",
      "Display task title, description, category",
      "Show current status with StatusBadge",
      "Include StateHistoryTimeline component",
      "Show associated reviews",
      "Show related fix tasks if any",
      "Keep file under 150 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement AskUserQuestion types and store",
    "steps": [
      "Write unit tests for types and store",
      "Create src/types/ask-user-question.ts",
      "Define AskUserQuestionPayload interface",
      "Define AskUserQuestionResponse interface",
      "Create Zod schemas for validation",
      "Add to uiStore: activeQuestion, submitAnswer action",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useAskUserQuestion hook",
    "steps": [
      "Write unit tests for hook behavior",
      "Create src/hooks/useAskUserQuestion.ts",
      "Listen for agent:ask_user_question Tauri events",
      "Store question payload in uiStore",
      "Implement submitAnswer function to send response back to agent",
      "Clear question after answer submitted",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement AskUserQuestionModal component",
    "steps": [
      "Write component tests",
      "Create src/components/modals/AskUserQuestionModal.tsx",
      "Display question header and text",
      "Render options as radio buttons (single select) or checkboxes (multi-select)",
      "Always include 'Other' option with text input",
      "Submit button sends answer via useAskUserQuestion hook",
      "Keep file under 100 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Tauri command for answering questions",
    "steps": [
      "Write integration test for command",
      "Create answer_user_question Tauri command",
      "Accept task_id, selected_options, custom_response",
      "Resume agent execution with answer",
      "Update task status from blocked to previous state",
      "Run tauri dev to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement ExecutionControlBar component",
    "steps": [
      "Write component tests",
      "Create src/components/execution/ExecutionControlBar.tsx",
      "Show running tasks count: 'Running: X/Y'",
      "Show queued tasks count",
      "Add Pause toggle button",
      "Add Stop button with confirmation dialog",
      "Keep file under 80 lines",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement Tauri commands for execution control",
    "steps": [
      "Write integration tests for commands",
      "Create pause_execution command (stops picking up new tasks)",
      "Create resume_execution command",
      "Create stop_execution command (cancels current tasks)",
      "Create get_execution_status command",
      "Run tauri dev to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement execution control store and hooks",
    "steps": [
      "Write unit tests for store and hooks",
      "Add to uiStore: isPaused, runningTasks, queuedTasks",
      "Create useExecutionStatus hook",
      "Create usePauseExecution hook",
      "Create useStopExecution hook",
      "Run npm run test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement task injection functionality",
    "steps": [
      "Write integration tests for task injection",
      "Add inject_task Tauri command with options: backlog or planned",
      "Add make_next option for highest priority",
      "Update priority calculation to accommodate injected tasks",
      "Emit task:created event",
      "Run tauri dev to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement review points detection",
    "steps": [
      "Write unit tests for review point detection",
      "Create src-tauri/src/domain/review/review_points.rs",
      "Implement is_destructive_task(task) to detect file deletions, config changes",
      "Implement should_auto_insert_review_point(task, config) logic",
      "Add needs_review_point field to Task entity",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "integration",
    "description": "Integration test: AI review approve flow",
    "steps": [
      "Create integration test file",
      "Set up task in pending_review state",
      "Mock reviewer agent with APPROVE outcome",
      "Trigger AI review via ReviewService",
      "Verify task transitions to approved",
      "Verify review record created with correct status",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "integration",
    "description": "Integration test: AI review needs_changes flow",
    "steps": [
      "Create integration test file",
      "Set up task in pending_review state",
      "Mock reviewer agent with NEEDS_CHANGES outcome",
      "Trigger AI review via ReviewService",
      "Verify fix task created",
      "Verify original task transitions to revision_needed",
      "Verify review_action record created",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: AI review escalate flow",
    "steps": [
      "Create integration test file",
      "Set up task in pending_review state",
      "Mock reviewer agent with ESCALATE outcome",
      "Trigger AI review via ReviewService",
      "Verify task transitions to blocked",
      "Verify review record has needs_human status",
      "Verify notification emitted",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: fix task rejection and retry",
    "steps": [
      "Create integration test file",
      "Set up fix task from previous AI review",
      "Reject fix task with feedback",
      "Verify new fix task proposed",
      "Verify attempt counter incremented",
      "Reject until max_fix_attempts reached",
      "Verify original task moved to backlog",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: human review flow",
    "steps": [
      "Create integration test file",
      "Set up task requiring human review (require_human_review: true)",
      "Complete AI review with approve",
      "Verify task stays in pending_review (waiting for human)",
      "Call approve_human_review",
      "Verify task transitions to approved",
      "Verify review notes saved",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: AskUserQuestion flow",
    "steps": [
      "Create integration test file",
      "Set up executing task",
      "Emit agent:ask_user_question event",
      "Verify task transitions to blocked",
      "Call answer_user_question command",
      "Verify task resumes execution",
      "Verify answer sent to agent",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: execution pause and resume",
    "steps": [
      "Create integration test file",
      "Set up multiple planned tasks",
      "Call pause_execution command",
      "Verify no new tasks picked up",
      "Verify running tasks continue",
      "Call resume_execution command",
      "Verify queue processing resumes",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: Reviews panel end-to-end",
    "steps": [
      "Create component integration test",
      "Render ReviewsPanel with mock pending reviews",
      "Verify reviews displayed correctly",
      "Click Approve on a review",
      "Verify approveReview API called",
      "Verify review removed from list",
      "Run npm run test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Integrate ReviewsPanel with App layout",
    "steps": [
      "Add ReviewsPanel to main app layout",
      "Add toggle button/icon in header or sidebar",
      "Show badge with pending review count",
      "Position as slide-out panel or modal",
      "Run tauri dev to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Integrate ExecutionControlBar with App layout",
    "steps": [
      "Add ExecutionControlBar to main app layout",
      "Position at bottom or top of TaskBoard",
      "Connect to execution state",
      "Run tauri dev to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Integrate AskUserQuestionModal with App",
    "steps": [
      "Add AskUserQuestionModal to App",
      "Connect to uiStore.activeQuestion",
      "Show modal when question received",
      "Hide modal after answer submitted",
      "Run tauri dev to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Add TaskCard click to open TaskDetailView",
    "steps": [
      "Update TaskCard to be clickable",
      "Open TaskDetailView modal/panel on click",
      "Show state history timeline",
      "Show associated reviews",
      "Run tauri dev to verify"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Visual verification of review components",
    "steps": [
      "Start app with tauri dev",
      "Create test tasks with various review states",
      "Capture screenshot of ReviewsPanel",
      "Capture screenshot of TaskDetailView with state history",
      "Capture screenshot of AskUserQuestionModal",
      "Capture screenshot of ExecutionControlBar",
      "Verify design system compliance (colors, no AI-slop)",
      "Document any visual issues"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Export review modules",
    "steps": [
      "Update src-tauri/src/domain/mod.rs to export review module",
      "Update src-tauri/src/infrastructure/mod.rs to export review repository",
      "Update src-tauri/src/application/mod.rs to export ReviewService",
      "Update src-tauri/src/lib.rs to register all review Tauri commands",
      "Verify all public APIs are exported",
      "Run cargo build to verify"
    ],
    "passes": false
  }
]
```
