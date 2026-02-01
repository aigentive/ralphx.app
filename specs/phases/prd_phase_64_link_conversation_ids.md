# RalphX - Phase 64: Link Conversation IDs to Task State History

## Overview

When navigating task history (executing → reviewing → re_executing → reviewing cycles), the UI cannot show the correct conversation for each historical state. Currently, `task_state_history` records transitions but has no `conversation_id`, and `chat_conversations` links to tasks but doesn't distinguish between cycles. This phase stores `conversation_id` and `agent_run_id` in state history metadata, enabling the UI to show the correct conversation for each historical state.

**Reference Plan:**
- `specs/plans/link_conversation_ids_to_task_state_history.md` - Detailed architecture analysis and implementation steps

## Goals

1. Store `conversation_id` and `agent_run_id` in `task_state_history.metadata` when entering conversation-spawning states
2. Expose metadata fields in the state transitions API
3. Wire conversation selection to history navigation in the UI

## Dependencies

### Phase 63 (Wire Review Issues to UI Detail Views) - Required

| Dependency | Why Needed |
|------------|------------|
| State history infrastructure | Phase 59 established state history; Phase 63 uses detail views that will consume this data |
| Detail view components | HumanReviewTaskDetail and other views need conversation context |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/link_conversation_ids_to_task_state_history.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/link_conversation_ids_to_task_state_history.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add repository method to update state history metadata",
    "plan_section": "Task 1: Backend - Add method to update state history metadata",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(task-repo): add update_latest_state_history_metadata method",
    "steps": [
      "Read specs/plans/link_conversation_ids_to_task_state_history.md section 'Task 1'",
      "Add StateHistoryMetadata struct with conversation_id and agent_run_id fields",
      "Add update_latest_state_history_metadata method to TaskRepository trait",
      "Implement method in SQLite repository with UPDATE query",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(task-repo): add update_latest_state_history_metadata method"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Capture conversation_id and agent_run_id in chat service after creation",
    "plan_section": "Task 2: Backend - Capture conversation_id and agent_run_id after creation",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(chat-service): capture conversation and agent_run IDs in state history",
    "steps": [
      "Read specs/plans/link_conversation_ids_to_task_state_history.md section 'Task 2'",
      "Locate send_message() in chat_service/mod.rs after agent_run is persisted",
      "Add conditional call to update_latest_state_history_metadata for TaskExecution and Review contexts",
      "Use best-effort pattern (ignore errors) to avoid breaking send_message",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(chat-service): capture conversation and agent_run IDs in state history"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Expose metadata in state transitions API response",
    "plan_section": "Task 3: Backend - Expose metadata in state history API",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(query): expose conversation_id and agent_run_id in state transitions API",
    "steps": [
      "Read specs/plans/link_conversation_ids_to_task_state_history.md section 'Task 3'",
      "Add conversation_id and agent_run_id fields to StateTransitionResponse struct",
      "Parse metadata JSON in get_task_state_transitions to extract the new fields",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(query): expose conversation_id and agent_run_id in state transitions API"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add metadata fields to state transition types",
    "plan_section": "Task 4: Frontend - Add metadata fields to state transition types",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(api): add conversationId and agentRunId to state transition types",
    "steps": [
      "Read specs/plans/link_conversation_ids_to_task_state_history.md section 'Task 4'",
      "Add conversation_id and agent_run_id to state transition Zod schema (optional strings)",
      "Add conversationId and agentRunId to StateTransition TypeScript type",
      "Add transform mapping from snake_case to camelCase",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add conversationId and agentRunId to state transition types"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Wire conversation selection to history navigation",
    "plan_section": "Task 5: Frontend - Wire conversation selection to history navigation",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(ui): wire conversation selection to state history navigation",
    "steps": [
      "Read specs/plans/link_conversation_ids_to_task_state_history.md section 'Task 5'",
      "Update StateTimelineNav to pass conversationId and agentRunId when calling onSelectState",
      "Update TaskDetailOverlay to track selected historical state metadata",
      "Add overrideConversationId and overrideAgentRunId props to useChatPanelContext",
      "Update IntegratedChatPanel to scroll to agent_run start position when override is set",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): wire conversation selection to state history navigation"
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
| **Store in metadata JSON column** | Avoids schema migration; existing column supports arbitrary JSON |
| **Track agent_run_id alongside conversation_id** | Conversations may be reused across cycles; agent_run_id uniquely identifies each execution |
| **Best-effort metadata update** | Don't fail send_message if metadata update fails; conversation creation is primary concern |
| **Use agent_run.started_at for scroll position** | ChatMessage doesn't have agent_run_id; use timestamp correlation instead |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] update_latest_state_history_metadata updates the most recent state history entry
- [ ] get_task_state_transitions returns conversation_id and agent_run_id from metadata

### Frontend - Run `npm run test`
- [ ] State transition types include conversationId and agentRunId
- [ ] StateTimelineNav passes metadata when selecting historical states

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Create task → execute → review → request_changes → re_execute → review → approve
- [ ] Open task detail overlay with history timeline
- [ ] Click `executing` state → chat shows correct conversation scrolled to run1
- [ ] Click `re_executing` state → chat shows same conversation scrolled to run3 (different position)
- [ ] Click `reviewing` states → chat shows review conversation at correct agent_run positions
- [ ] Non-conversation states (ready, pending_review, etc.) don't affect chat display

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (StateTimelineNav click → onSelectState callback)
- [ ] New metadata fields are passed through the component chain
- [ ] API wrappers return conversation_id and agent_run_id from backend
- [ ] State changes reflect in chat panel scroll position

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
