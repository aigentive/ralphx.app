# RalphX - Phase 20: Review System

## Overview

This phase implements the AI-powered review system for RalphX tasks. When a task completes execution, it enters review where an AI reviewer agent examines the work. Upon AI approval, the task awaits human confirmation before being marked complete. If the AI or human requests changes, the task cycles back for revision.

**Reference Plan:**
- `specs/plans/review_system.md` - Complete implementation plan with architecture, state diagrams, and specifications

## Goals

1. Add new review states: `reviewing`, `review_passed`, `re_executing`
2. Remove transitional `execution_done` state (logic moves to `executing` handler)
3. Implement the `complete_review` HTTP handler (currently a stub)
4. Add `get_review_notes` MCP tool for workers to fetch revision feedback
5. Add task-level MCP scoping (RALPHX_TASK_ID validation)
6. Add `review` context type for live chat with AI reviewer
7. Implement live/historical chat mode (disable input for completed reviews)
8. Create state-specific task detail views (View Registry Pattern)
9. Add column grouping UI for multi-state columns
10. Wire up approve/reject mutations in ReviewsPanel

## Dependencies

### Phase 19 (Task Execution Experience) - Required

| Dependency | Why Needed |
|------------|------------|
| Task steps data model | Steps shown in task detail views |
| TaskFullView with split layout | Review views extend this pattern |
| TaskChatPanel component | Embedded chat for reviewer interaction |
| useTaskExecutionState hook | State-based UI switching |
| Step progress tracking | Shows in revision context |
| MCP tool infrastructure | New review tools follow same pattern |
| ExecutionChatService (now ChatService) | Unified chat handles review context |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/review_system.md`
2. Understand the architecture, state flow, and component structure
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
2. **Read the ENTIRE implementation plan** at `specs/plans/review_system.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Add new review states to InternalStatus enum (Rust)",
    "plan_section": "Proposed Review States",
    "steps": [
      "Read specs/plans/review_system.md section 'Proposed Review States'",
      "Update src-tauri/src/domain/entities/status.rs:",
      "  - Add `Reviewing` variant after PendingReview (AI actively reviewing)",
      "  - Add `ReviewPassed` variant after Reviewing (AI approved, awaiting human)",
      "  - Add `ReExecuting` variant after RevisionNeeded (worker revising after feedback)",
      "Update all_variants() to include new states",
      "Update as_str() and FromStr for snake_case serialization:",
      "  - Reviewing => 'reviewing'",
      "  - ReviewPassed => 'review_passed'",
      "  - ReExecuting => 're_executing'",
      "Run cargo test",
      "Commit: feat(status): add Reviewing, ReviewPassed, ReExecuting states"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add valid transitions for new review states",
    "plan_section": "State Transitions",
    "steps": [
      "Read specs/plans/review_system.md section 'State Transitions'",
      "Update src-tauri/src/domain/entities/status.rs valid_transitions():",
      "  - PendingReview => &[Reviewing] (remove direct Approved/RevisionNeeded)",
      "  - Reviewing => &[ReviewPassed, RevisionNeeded]",
      "  - ReviewPassed => &[Approved, RevisionNeeded]",
      "  - RevisionNeeded => &[ReExecuting, Cancelled] (update from Executing)",
      "  - ReExecuting => &[PendingReview, Failed, Blocked]",
      "Write unit tests for all new transitions:",
      "  - test pending_review_to_reviewing()",
      "  - test reviewing_to_review_passed()",
      "  - test reviewing_to_revision_needed()",
      "  - test review_passed_to_approved()",
      "  - test review_passed_to_revision_needed()",
      "  - test revision_needed_to_re_executing()",
      "  - test re_executing_to_pending_review()",
      "Run cargo test",
      "Commit: feat(status): add valid transitions for review states"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Remove execution_done state from InternalStatus",
    "plan_section": "Implementation: Remove execution_done State",
    "steps": [
      "Read specs/plans/review_system.md section 'Remove execution_done State'",
      "Update src-tauri/src/domain/entities/status.rs:",
      "  - Remove `ExecutionDone` variant from InternalStatus enum",
      "  - Remove from all_variants()",
      "  - Remove from as_str() and FromStr",
      "  - Remove from valid_transitions()",
      "  - Update Executing transitions: &[QaRefining, PendingReview, Failed, Blocked]",
      "Remove execution_done_transitions() test",
      "Update any tests that reference ExecutionDone",
      "Run cargo test",
      "Commit: refactor(status): remove ExecutionDone transitional state"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add State enum variants and handlers in machine.rs",
    "plan_section": "State Machine Implementation",
    "steps": [
      "Update src-tauri/src/domain/state_machine/machine.rs:",
      "  - Add State::Reviewing variant",
      "  - Add State::ReviewPassed variant",
      "  - Add State::ReExecuting variant",
      "  - Remove State::ExecutionDone variant",
      "Implement state handlers:",
      "  - reviewing(&mut self, event: &TaskEvent) -> Response",
      "    - ReviewComplete { approved: true, .. } => Transition(ReviewPassed)",
      "    - ReviewComplete { approved: false, feedback } => Transition(RevisionNeeded)",
      "    - Cancel => Transition(Cancelled)",
      "  - review_passed(&mut self, event: &TaskEvent) -> Response",
      "    - HumanApprove => Transition(Approved)",
      "    - HumanRequestChanges { feedback } => Transition(RevisionNeeded)",
      "  - re_executing(&mut self, event: &TaskEvent) -> Response",
      "    - ExecutionComplete => check qa_enabled, go to QaRefining or PendingReview",
      "    - ExecutionFailed { error } => Transition(Failed)",
      "    - BlockerDetected => Transition(Blocked)",
      "Update dispatch() to route to new handlers",
      "Remove execution_done() handler",
      "Update executing() handler: ExecutionComplete now checks qa_enabled directly",
      "Run cargo test",
      "Commit: feat(machine): add review state handlers, remove execution_done"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add new TaskEvents for review transitions",
    "plan_section": "Task Events",
    "steps": [
      "Update src-tauri/src/domain/state_machine/events.rs:",
      "  - Add StartReview event (PendingReview -> Reviewing)",
      "  - Add HumanApprove event (ReviewPassed -> Approved)",
      "  - Add HumanRequestChanges { feedback: String } event (ReviewPassed -> RevisionNeeded)",
      "  - Add StartRevision event (RevisionNeeded -> ReExecuting)",
      "Update event serialization/deserialization",
      "Write unit tests for new events",
      "Run cargo test",
      "Commit: feat(events): add review-related TaskEvents"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add entry/exit actions for review states in TransitionHandler",
    "plan_section": "TransitionHandler - Side Effects",
    "steps": [
      "Update src-tauri/src/domain/state_machine/transition_handler.rs:",
      "In on_enter():",
      "  - State::Reviewing => spawn reviewer agent via ChatService",
      "  - State::ReviewPassed => emit 'review:ai_approved' event, notify user",
      "  - State::ReExecuting => spawn worker agent with revision context",
      "In on_exit():",
      "  - State::Reviewing => log review duration",
      "Remove ExecutionDone handling from check_auto_transition()",
      "Add auto-transition: RevisionNeeded -> ReExecuting (when worker available)",
      "Update on_enter for PendingReview: use StartReview event instead of direct spawn",
      "Write tests for entry actions",
      "Run cargo test",
      "Commit: feat(transition): add entry/exit actions for review states"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Implement complete_review HTTP handler",
    "plan_section": "Critical Gap: HTTP Handler Implementation",
    "steps": [
      "Read specs/plans/review_system.md section 'Critical Gap: HTTP Handler Implementation'",
      "Update src-tauri/src/http_server.rs complete_review handler (lines 879-901):",
      "  1. Parse CompleteReviewRequest (task_id, decision, feedback, issues)",
      "  2. Get task and validate state is Reviewing:",
      "     - let task = state.task_repo.get_by_id(&task_id).await?;",
      "     - if task.internal_status != InternalStatus::Reviewing { return Err(400) }",
      "  3. Get or create Review record for this task",
      "  4. Map decision to ReviewToolOutcome:",
      "     - 'approved' => ReviewToolOutcome::Approved",
      "     - 'needs_changes' => ReviewToolOutcome::NeedsChanges",
      "     - 'escalate' => ReviewToolOutcome::Escalate",
      "  5. Create CompleteReviewInput from request",
      "  6. Call review_service.process_review_result(review, input)",
      "  7. Create ReviewNote for history tracking",
      "  8. Trigger state transition via TransitionHandler:",
      "     - approved => ReviewComplete { approved: true }",
      "     - needs_changes => ReviewComplete { approved: false, feedback }",
      "  9. Emit events: 'review:completed', 'task:status_changed'",
      "  10. Return CompleteReviewResponse with success, new_status, fix_task_id (if created)",
      "Add CompleteReviewResponse struct",
      "Write integration tests",
      "Run cargo test",
      "Commit: feat(http): implement complete_review handler"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add get_review_notes HTTP endpoint",
    "plan_section": "MCP Tool: get_review_notes",
    "steps": [
      "Read specs/plans/review_system.md section 'MCP Tool: get_review_notes'",
      "Update src-tauri/src/http_server.rs:",
      "  - Add route: GET /api/review_notes/:task_id",
      "  - Add ReviewNotesResponse struct:",
      "    - task_id: String",
      "    - revision_count: u32 (count of changes_requested outcomes)",
      "    - max_revisions: u32 (from settings)",
      "    - reviews: Vec<ReviewNoteResponse>",
      "  - Implement handler:",
      "    - Fetch notes via review_repo.get_notes_by_task_id()",
      "    - Get max_revisions from project settings",
      "    - Calculate revision_count",
      "    - Return ReviewNotesResponse",
      "Write tests for endpoint",
      "Run cargo test",
      "Commit: feat(http): add get_review_notes endpoint"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add maxRevisionCycles to review settings",
    "plan_section": "Settings Configuration",
    "steps": [
      "Check if review settings exist in project settings",
      "If not, add to src-tauri/src/domain/entities/project.rs or settings entity:",
      "  - max_revision_cycles: u32 (default 5)",
      "Update project settings repository to handle new field",
      "Add migration if needed for new column",
      "Update get_review_notes handler to use this setting",
      "Update TransitionHandler to check revision count:",
      "  - In RevisionNeeded entry, count changes_requested for task",
      "  - If count >= max_revision_cycles, transition to Failed instead",
      "Write tests",
      "Run cargo test",
      "Commit: feat(settings): add maxRevisionCycles setting"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Add get_review_notes MCP tool definition",
    "plan_section": "MCP Tool: get_review_notes",
    "steps": [
      "Update ralphx-plugin/ralphx-mcp-server/src/tools.ts:",
      "  - Add get_review_notes tool definition after complete_review:",
      "    - name: 'get_review_notes'",
      "    - description: 'Get all review feedback for a task...'",
      "    - inputSchema: { task_id: string (required) }",
      "  - Add to TOOL_ALLOWLIST for 'worker' agent",
      "Update ralphx-plugin/ralphx-mcp-server/src/index.ts:",
      "  - Add handler for get_review_notes tool",
      "  - Call GET /api/review_notes/:task_id",
      "  - Return formatted response",
      "Run npm run build in ralphx-mcp-server",
      "Commit: feat(mcp): add get_review_notes tool"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Add task-level MCP scoping (RALPHX_TASK_ID validation)",
    "plan_section": "Enhanced Scoping: Task-Level Enforcement",
    "steps": [
      "Read specs/plans/review_system.md section 'Enhanced Scoping: Task-Level Enforcement'",
      "Update ralphx-plugin/ralphx-mcp-server/src/index.ts:",
      "  - Read RALPHX_TASK_ID from process.env",
      "  - Create validateTaskScope(toolName, args) function:",
      "    - Define taskScopedTools = ['complete_review', 'update_task', 'add_task_note', 'get_task_details', 'get_task_context', 'get_review_notes']",
      "    - If tool not in list, return null (no validation)",
      "    - If RALPHX_TASK_ID not set, return null (backward compat)",
      "    - If args.task_id !== RALPHX_TASK_ID, return error message",
      "  - Call validateTaskScope before executing tool",
      "  - Return helpful error: 'Task scope violation. You are assigned to task X but attempted to modify task Y'",
      "Update agent spawning in chat_service.rs to pass RALPHX_TASK_ID env var",
      "Write tests for scope validation",
      "Run npm run build in ralphx-mcp-server",
      "Commit: feat(mcp): add task-level scope validation"
    ],
    "passes": true
  },
  {
    "category": "agent",
    "description": "Update worker agent prompt with revision instructions",
    "plan_section": "Worker Agent Instructions Update",
    "steps": [
      "Update ralphx-plugin/agents/worker.md:",
      "  - Add '## Before Starting Re-Execution Work' section",
      "  - Instruct: Check RALPHX_TASK_STATE env var",
      "  - If 're_executing':",
      "    - MUST call get_task_context(task_id)",
      "    - MUST call get_review_notes(task_id)",
      "    - Read all previous feedback carefully",
      "    - Address each issue mentioned",
      "    - Do not repeat same mistakes",
      "  - Add example flow for revision work",
      "Commit: docs(agent): add revision instructions to worker prompt"
    ],
    "passes": true
  },
  {
    "category": "agent",
    "description": "Update reviewer agent prompt for state transitions",
    "plan_section": "Reviewer Agent Definition",
    "steps": [
      "Update ralphx-plugin/agents/reviewer.md:",
      "  - Add section on using complete_review tool properly",
      "  - Instruct: After review, MUST call complete_review with:",
      "    - decision: 'approved' | 'needs_changes' | 'escalate'",
      "    - feedback: detailed explanation",
      "    - issues: array of specific issues found",
      "  - Add guidance on when to approve vs request changes",
      "  - Add escalation criteria",
      "Commit: docs(agent): update reviewer prompt for complete_review"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add new states to TypeScript InternalStatus",
    "plan_section": "Frontend Types",
    "steps": [
      "Update src/types/status.ts:",
      "  - Add 'reviewing' to InternalStatusSchema enum",
      "  - Add 'review_passed' to InternalStatusSchema enum",
      "  - Add 're_executing' to InternalStatusSchema enum",
      "  - Remove 'execution_done' from InternalStatusSchema enum",
      "  - Update ACTIVE_STATUSES to include reviewing, review_passed, re_executing",
      "  - Update EXECUTING_STATUSES if it exists",
      "  - Add REVIEW_STATUSES = ['pending_review', 'reviewing', 'review_passed'] as const",
      "  - Add isReviewStatus(status) helper function",
      "Run npm run typecheck",
      "Commit: feat(types): add review states to InternalStatus"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add review context type to chat system",
    "plan_section": "Live Chat with AI Reviewer - Implementation",
    "steps": [
      "Update src/types/chat-conversation.ts:",
      "  - Add 'review' to CONTEXT_TYPE_VALUES array",
      "Update src/types/chat.ts:",
      "  - Add ReviewChatContext interface:",
      "    - type: 'review'",
      "    - taskId: string",
      "    - reviewId: string",
      "Run npm run typecheck",
      "Commit: feat(types): add review context type for chat"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add Review context type to ChatService",
    "plan_section": "Live Chat with AI Reviewer",
    "steps": [
      "Update src-tauri/src/application/chat_service.rs:",
      "  - Add ChatContextType::Review variant",
      "  - Update get_agent_name(): Review => 'reviewer'",
      "  - Update get_assistant_role(): Review => MessageRole::Reviewer (add if needed)",
      "  - Update initial prompt for Review context:",
      "    - 'RalphX Review Session. Task ID: {id}. You are reviewing...'",
      "Run cargo test",
      "Commit: feat(chat): add Review context type to ChatService"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Update useChat hook for review context routing",
    "plan_section": "Live Chat with AI Reviewer",
    "steps": [
      "Update src/hooks/useChat.ts:",
      "  - Update buildContextKey() (lines 21-33):",
      "    - Add case 'review': return `review:${contextId}`",
      "  - Update getContextTypeAndId() (lines 60-83):",
      "    - Add case for review view:",
      "      - return { contextType: 'review', contextId: taskId }",
      "Run npm run typecheck",
      "Commit: feat(hooks): add review context routing to useChat"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Implement live/historical chat mode",
    "plan_section": "Chat as Live Interaction + Historical Log",
    "steps": [
      "Read specs/plans/review_system.md section 'State-Based Chat Behavior'",
      "Update src/components/tasks/TaskChatPanel.tsx:",
      "  - Add isLive computed value based on task state:",
      "    - execution: live if executing or re_executing",
      "    - review: live if reviewing",
      "    - Otherwise: historical (read-only)",
      "  - Conditionally render ChatInput only when isLive",
      "  - Show 'Chat ended - Review completed' message when historical",
      "  - Update styling for historical mode (dimmed input area)",
      "Create ChatModeIndicator component:",
      "  - Shows 'Live' badge with pulse when active",
      "  - Shows 'Completed' badge when historical",
      "Run npm run typecheck",
      "Commit: feat(chat): implement live/historical mode"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Update context derivation in TaskFullView",
    "plan_section": "State → View Mapping",
    "steps": [
      "Update src/components/tasks/TaskFullView.tsx (lines 158-171):",
      "  - Update contextType derivation:",
      "    - reviewing, review_passed => 'review'",
      "    - executing, re_executing, qa_* => 'task_execution'",
      "    - Others => 'task'",
      "  - Pass contextType to TaskChatPanel",
      "Run npm run typecheck",
      "Commit: feat(fullview): update context derivation for review states"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create BasicTaskDetail component",
    "plan_section": "View Components - BasicTaskDetail",
    "steps": [
      "Create src/components/tasks/detail-views/BasicTaskDetail.tsx:",
      "  - Props: task: Task",
      "  - Render: status badge, title, priority, category",
      "  - Render: description section",
      "  - Render: StepList if task has steps",
      "  - Use existing TaskDetailPanel patterns",
      "  - No edit buttons (parent handles)",
      "Export from src/components/tasks/detail-views/index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add BasicTaskDetail component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create RevisionTaskDetail component",
    "plan_section": "View Components - RevisionTaskDetail",
    "steps": [
      "Create src/components/tasks/detail-views/RevisionTaskDetail.tsx:",
      "  - Props: task: Task",
      "  - Fetch review notes via useTaskStateHistory(task.id)",
      "  - Render: REVISION NEEDED banner (orange/warning)",
      "  - Render: 'Attempt #N' badge from revision count",
      "  - Render: Review Feedback section:",
      "    - AI/Human icon based on reviewer",
      "    - Feedback text",
      "    - Issues list with file:line references",
      "  - Render: Description, StepList (with steps that need revision highlighted)",
      "Export from index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add RevisionTaskDetail component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create ExecutionTaskDetail component",
    "plan_section": "View Components - ExecutionTaskDetail",
    "steps": [
      "Create src/components/tasks/detail-views/ExecutionTaskDetail.tsx:",
      "  - Props: task: Task",
      "  - Use useTaskSteps(task.id) for steps",
      "  - Use useStepProgress(task.id) for progress",
      "  - Use useTaskStateHistory(task.id) for revision context (if re_executing)",
      "  - Render: Live indicator badge (red dot + 'Live')",
      "  - Render: Progress bar with percentage",
      "  - If re_executing: render 'Addressing Review Feedback' banner with feedback",
      "  - Render: StepList with current step highlighted",
      "  - Render: Description section",
      "Export from index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add ExecutionTaskDetail component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create ReviewingTaskDetail component",
    "plan_section": "View Components - ReviewingTaskDetail",
    "steps": [
      "Create src/components/tasks/detail-views/ReviewingTaskDetail.tsx:",
      "  - Props: task: Task",
      "  - Render: 'AI REVIEW IN PROGRESS' banner (blue)",
      "  - Render: Review steps indicator:",
      "    - Gathering context (spinner if active)",
      "    - Examining changes",
      "    - Running checks",
      "    - Generating feedback",
      "  - Render: Files Under Review section (if available from git diff)",
      "  - Render: Description section",
      "Export from index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add ReviewingTaskDetail component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create HumanReviewTaskDetail component",
    "plan_section": "View Components - HumanReviewTaskDetail",
    "steps": [
      "Create src/components/tasks/detail-views/HumanReviewTaskDetail.tsx:",
      "  - Props: task: Task",
      "  - Fetch reviews via useReviewsByTaskId(task.id)",
      "  - Render: 'AI REVIEW PASSED' banner (green) with 'Awaiting your approval'",
      "  - Render: AI Review Summary:",
      "    - Confidence percentage (if available)",
      "    - Summary text",
      "    - Checklist of passed items",
      "  - Render: 'View Diff' link (opens DiffViewer or ReviewDetailModal)",
      "  - Render: Previous Attempts section (if revision_count > 0)",
      "  - Render: Action buttons: [Approve] [Request Changes]",
      "  - Wire buttons to useReviewMutations",
      "Export from index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add HumanReviewTaskDetail component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create WaitingTaskDetail component",
    "plan_section": "View Components - WaitingTaskDetail",
    "steps": [
      "Create src/components/tasks/detail-views/WaitingTaskDetail.tsx:",
      "  - Props: task: Task",
      "  - Use useTaskSteps(task.id)",
      "  - Render: 'WAITING FOR AI REVIEWER' banner with clock icon",
      "  - Render: Work Completed section:",
      "    - Submitted time (relative)",
      "    - Files changed count",
      "    - 'All steps completed' indicator",
      "  - Render: StepList (all completed)",
      "  - Render: Description",
      "Export from index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add WaitingTaskDetail component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create CompletedTaskDetail component",
    "plan_section": "View Components - CompletedTaskDetail",
    "steps": [
      "Create src/components/tasks/detail-views/CompletedTaskDetail.tsx:",
      "  - Props: task: Task",
      "  - Use useTaskStateHistory(task.id)",
      "  - Render: 'COMPLETED' banner (green) with approval info",
      "  - Render: Final Summary section",
      "  - Render: Review History timeline:",
      "    - Human approved timestamp",
      "    - AI approved timestamp",
      "    - Any previous revision requests",
      "  - Render: Action buttons: [View Final Diff] [Reopen Task]",
      "Export from index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add CompletedTaskDetail component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Implement View Registry Pattern in TaskDetailPanel",
    "plan_section": "Implementation Approach: View Registry Pattern",
    "steps": [
      "Update src/components/tasks/TaskDetailPanel.tsx:",
      "  - Import all detail view components",
      "  - Create TASK_DETAIL_VIEWS registry:",
      "    - backlog: BasicTaskDetail",
      "    - ready: BasicTaskDetail",
      "    - blocked: BasicTaskDetail",
      "    - revision_needed: RevisionTaskDetail",
      "    - executing: ExecutionTaskDetail",
      "    - re_executing: ExecutionTaskDetail",
      "    - pending_review: WaitingTaskDetail",
      "    - reviewing: ReviewingTaskDetail",
      "    - review_passed: HumanReviewTaskDetail",
      "    - approved: CompletedTaskDetail",
      "    - failed: BasicTaskDetail (or create FailedTaskDetail)",
      "    - cancelled: BasicTaskDetail",
      "  - Update render to use registry:",
      "    - const ViewComponent = TASK_DETAIL_VIEWS[task.internalStatus] ?? BasicTaskDetail",
      "    - return <ViewComponent task={task} />",
      "Run npm run typecheck",
      "Commit: feat(components): implement View Registry Pattern"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add useReviewMutations hook",
    "plan_section": "Implementation Plan: UI - Phase 1",
    "steps": [
      "Create src/hooks/useReviewMutations.ts:",
      "  - useApproveReview(reviewId: string, notes?: string) mutation:",
      "    - Calls api.reviews.approve(reviewId, { notes })",
      "    - Invalidates reviewKeys and taskKeys",
      "    - Shows success toast",
      "  - useRequestChanges(reviewId: string, notes: string, fixDescription?: string) mutation:",
      "    - Calls api.reviews.requestChanges(reviewId, { notes, fixDescription })",
      "    - Invalidates reviewKeys and taskKeys",
      "    - Shows success toast",
      "Export from src/hooks/index.ts",
      "Run npm run typecheck",
      "Commit: feat(hooks): add useReviewMutations hook"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add review API wrappers",
    "plan_section": "Implementation Plan: UI - Phase 1",
    "steps": [
      "Update src/lib/tauri.ts, add to api.reviews namespace:",
      "  - approve: (reviewId: string, input: { notes?: string }) => invoke('approve_review', { reviewId, ...input })",
      "  - requestChanges: (reviewId: string, input: { notes: string, fixDescription?: string }) => invoke('request_changes', { reviewId, ...input })",
      "Run npm run typecheck",
      "Commit: feat(api): add review approve/requestChanges wrappers"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Wire up ReviewsPanel approve/reject handlers",
    "plan_section": "What's Missing",
    "steps": [
      "Update src/App.tsx review handlers (around lines 632-677):",
      "  - Import useReviewMutations",
      "  - Replace TODO comments with actual mutation calls:",
      "    - onApprove: approveReviewMutation.mutate(reviewId)",
      "    - onRequestChanges: requestChangesMutation.mutate(reviewId, notes)",
      "  - Handle loading states",
      "  - Handle errors with toast",
      "Run npm run typecheck",
      "Commit: feat(app): wire up ReviewsPanel approve/reject mutations"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create ColumnGroup component",
    "plan_section": "UI Considerations - Grouping",
    "steps": [
      "Create src/components/tasks/TaskBoard/ColumnGroup.tsx:",
      "  - Props: label: string, count: number, icon?: ReactNode, accentColor?: string, collapsed?: boolean, onToggle?: () => void, children: ReactNode",
      "  - Render: collapsible header with:",
      "    - Chevron icon (rotates when collapsed)",
      "    - Group label (e.g., 'Fresh Tasks', 'Needs Revision')",
      "    - Count badge",
      "    - Optional icon (retry icon for revision_needed)",
      "  - Render: children only when not collapsed",
      "  - Apply accent color to left border when expanded",
      "Export from TaskBoard/index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add ColumnGroup component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add group configuration to workflow types",
    "plan_section": "Column Mapping Update (Multi-State per Column)",
    "steps": [
      "Update src/types/workflow.ts:",
      "  - Add StateGroup interface:",
      "    - id: string",
      "    - label: string",
      "    - statuses: InternalStatus[]",
      "    - icon?: string",
      "    - accentColor?: string",
      "    - canDragFrom?: boolean",
      "    - canDropTo?: boolean",
      "  - Add groups field to WorkflowColumn:",
      "    - groups?: StateGroup[]",
      "  - Define default groups for each column:",
      "    - Ready: 'Fresh Tasks' (ready), 'Needs Revision' (revision_needed)",
      "    - In Progress: 'First Attempt' (executing), 'Revising' (re_executing)",
      "    - In Review: 'Waiting for AI' (pending_review), 'AI Reviewing' (reviewing), 'Ready for Approval' (review_passed)",
      "Run npm run typecheck",
      "Commit: feat(types): add group configuration to workflow"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Update Column component for grouping",
    "plan_section": "Grouping Across All Multi-State Columns",
    "steps": [
      "Update src/components/tasks/TaskBoard/Column.tsx:",
      "  - Import ColumnGroup component",
      "  - Get column groups from workflow config",
      "  - If column has groups:",
      "    - Group tasks by internalStatus",
      "    - Render ColumnGroup for each group with tasks",
      "    - Track collapsed state in localStorage per column-group",
      "  - If no groups: render tasks directly (existing behavior)",
      "  - Pass group info to drag-drop validation",
      "Update src/components/tasks/TaskBoard/validation.ts:",
      "  - Update canDragFrom to check group.canDragFrom",
      "  - Update canDropTo to check group.canDropTo",
      "Run npm run typecheck",
      "Commit: feat(kanban): implement column grouping"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add review state badges to TaskCard",
    "plan_section": "Visual Differentiators",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Import Lucide icons: RotateCcw, Eye, CheckCircle, Clock",
      "  - Add state badge rendering based on internalStatus:",
      "    - revision_needed: orange badge with RotateCcw icon",
      "    - pending_review: neutral badge with Clock icon",
      "    - reviewing: blue badge with animated spinner",
      "    - review_passed: green badge with CheckCircle + 'AI' text",
      "    - re_executing: orange badge with cycle icon + 'Attempt #N'",
      "  - Position badge in top-right corner of card",
      "  - Add CSS for badge animations",
      "Run npm run typecheck",
      "Commit: feat(card): add review state badges to TaskCard"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add review state animations to CSS",
    "plan_section": "Visual Differentiators",
    "steps": [
      "Update src/styles/globals.css:",
      "  - Add @keyframes reviewing-pulse (subtle blue pulse)",
      "  - Add .task-card-reviewing class with animation",
      "  - Add .task-card-revision class with orange left border",
      "  - Add .task-card-review-passed class with green accent",
      "  - Add .badge-reviewing animation (spinner)",
      "Run npm run lint",
      "Commit: feat(styles): add review state animations"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create ReviewDetailModal component",
    "plan_section": "UI Design Decision: Hybrid Approach",
    "steps": [
      "Create src/components/reviews/ReviewDetailModal.tsx:",
      "  - Props: taskId: string, reviewId: string, onClose: () => void",
      "  - Full-width modal (max-w-7xl or 90vw)",
      "  - Two-column layout:",
      "    - Left (300px fixed): Task context, AI review summary, review history, revision count",
      "    - Right (flex-1): DiffViewer component",
      "  - Footer: [Approve] [Request Changes] buttons",
      "  - Wire buttons to useReviewMutations",
      "  - Close on successful action",
      "Export from src/components/reviews/index.ts",
      "Run npm run typecheck",
      "Commit: feat(components): add ReviewDetailModal component"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Update ReviewsPanel to open ReviewDetailModal",
    "plan_section": "User Flow",
    "steps": [
      "Update src/components/reviews/ReviewsPanel.tsx:",
      "  - Add state: selectedReviewId: string | null",
      "  - Update ReviewCard to show 'Review' button",
      "  - On 'Review' click: setSelectedReviewId(review.id)",
      "  - Render ReviewDetailModal when selectedReviewId is set",
      "  - On modal close: setSelectedReviewId(null)",
      "Run npm run typecheck",
      "Commit: feat(reviews): open ReviewDetailModal from ReviewsPanel"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Update workflow configuration for new states",
    "plan_section": "Column Mapping Update",
    "steps": [
      "Update src-tauri/src/domain/entities/workflow.rs:",
      "  - Update column status mappings:",
      "    - 'ready' column: [Ready, RevisionNeeded]",
      "    - 'in_progress' column: [Executing, ReExecuting]",
      "    - 'in_review' column: [PendingReview, Reviewing, ReviewPassed]",
      "  - Add group definitions matching frontend",
      "Run cargo test",
      "Commit: feat(workflow): update column mappings for review states"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Remove execution_done** | Simplifies state machine; transition logic moves to executing handler |
| **Three new review states** | Mirrors QA pattern; provides visibility into review progress |
| **Review context type** | Enables live chat with reviewer; consistent with execution chat |
| **Task-level MCP scoping** | Prevents agents from modifying wrong tasks; safety + auditability |
| **View Registry Pattern** | Each state gets appropriate UI; easy to extend for new states |
| **Column grouping** | Visibility into why tasks are in a column without adding columns |
| **Live/historical chat mode** | Input enabled during active process, disabled after completion |
| **Hybrid review UI** | Quick access via panel, detailed review in modal |
| **maxRevisionCycles setting** | Prevents infinite revision loops; configurable per project |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] New states serialize/deserialize correctly
- [ ] All valid transitions work
- [ ] Invalid transitions are rejected
- [ ] complete_review handler processes all decisions
- [ ] get_review_notes returns correct data
- [ ] TransitionHandler entry actions fire correctly
- [ ] Revision cycle limit enforced

### Frontend - Run `npm run test`
- [ ] useReviewMutations.test.ts passes
- [ ] Review API wrappers work
- [ ] State-specific views render correctly
- [ ] Column grouping displays tasks correctly
- [ ] Chat mode switches based on state

### MCP - Test manually
- [ ] complete_review tool works end-to-end
- [ ] get_review_notes returns review history
- [ ] Task-level scoping blocks wrong task modifications

### Build Verification
- [ ] `npm run lint` passes
- [ ] `npm run typecheck` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `npm run build` succeeds
- [ ] `cargo build --release` succeeds

### Integration Testing
- [ ] Task execution → review → approve flow works
- [ ] Task execution → review → request changes → re-execute flow works
- [ ] Human can chat with reviewer during review
- [ ] Review history visible in completed tasks
- [ ] Max revision cycles triggers failure state
