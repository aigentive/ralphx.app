# RalphX - Phase 29: Unified Chat Resumption

## Overview

This phase fixes two critical issues in the chat system: missing `execution_state` wiring in unified chat commands (preventing proper state transitions for TaskExecution/Review chats), and the lack of chat resumption on app startup (leaving Ideation, Task, and Project chats abandoned after crashes).

All 5 chat types already use the same background processing path via `ChatService.send_message()`. This phase completes the architecture by ensuring state transitions work correctly and interrupted conversations resume automatically.

**Reference Plan:**
- `specs/plans/chat_resumption_unified.md` - Detailed implementation plan with code snippets and SQL queries

## Goals

1. Wire `execution_state` to all unified chat commands so TaskExecution/Review state transitions work properly
2. Add repository method and SQL query to detect interrupted conversations
3. Create `ChatResumptionRunner` to resume all chat types on startup (with deduplication against `StartupJobRunner`)

## Dependencies

### Phase 21 (Execution Control & Task Resumption) - Required

| Dependency | Why Needed |
|------------|------------|
| `ExecutionState` | Global pause/resume state that chat resumption respects |
| `StartupJobRunner` | Existing task resumption logic we must deduplicate against |
| `RunningAgentRegistry` | Track running agents across resumption |

### Phase 27 (Chat Architecture Refactor) - Required

| Dependency | Why Needed |
|------------|------------|
| Unified chat commands | `send_agent_message`, `queue_agent_message` etc. that need execution_state wiring |
| `ClaudeChatService` | Service with `.with_execution_state()` method we need to call |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/chat_resumption_unified.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/chat_resumption_unified.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Wire execution_state to unified chat commands",
    "plan_section": "Step 1: Wire execution_state to Unified Commands",
    "steps": [
      "Read specs/plans/chat_resumption_unified.md section 'Step 1'",
      "Modify create_chat_service() to accept execution_state parameter",
      "Add .with_execution_state(Arc::clone(execution_state)) call",
      "Update send_agent_message command to extract execution_state from Tauri state",
      "Update queue_agent_message, get_queued_agent_messages, delete_queued_agent_message",
      "Update stop_agent, is_agent_running commands",
      "Run cargo clippy && cargo test",
      "Commit: fix(chat): wire execution_state to unified chat commands"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add InterruptedConversation entity and repository trait method",
    "plan_section": "Step 2: Add Repository Method for Interrupted Conversations",
    "steps": [
      "Read specs/plans/chat_resumption_unified.md section 'Step 2'",
      "Add InterruptedConversation struct to src-tauri/src/domain/entities/agent_run.rs",
      "Add get_interrupted_conversations() method to AgentRunRepository trait",
      "Run cargo clippy && cargo test",
      "Commit: feat(domain): add InterruptedConversation entity and repo method"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Implement SQLite query for interrupted conversations",
    "plan_section": "Step 3: Implement SQLite Query",
    "steps": [
      "Read specs/plans/chat_resumption_unified.md section 'Step 3'",
      "Implement get_interrupted_conversations() in SqliteAgentRunRepository",
      "Query joins chat_conversations with agent_runs",
      "Filter: claude_session_id IS NOT NULL, status='cancelled', error='Orphaned on app restart'",
      "Only return latest run per conversation",
      "Write unit test for the query",
      "Run cargo clippy && cargo test",
      "Commit: feat(sqlite): implement interrupted conversations query"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Create ChatResumptionRunner with priority ordering",
    "plan_section": "Step 4: Create ChatResumptionRunner",
    "steps": [
      "Read specs/plans/chat_resumption_unified.md section 'Step 4'",
      "Create src-tauri/src/application/chat_resumption.rs",
      "Follow StartupJobRunner pattern for structure",
      "Implement prioritize_resumptions() - TaskExecution > Review > Task > Ideation > Project",
      "Implement is_handled_by_task_resumption() - skip if task in AGENT_ACTIVE_STATUSES",
      "Implement run() - skip if paused, get interrupted, sort, resume each",
      "Export from src-tauri/src/application/mod.rs",
      "Run cargo clippy && cargo test",
      "Commit: feat(application): add ChatResumptionRunner for startup resumption"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Integrate ChatResumptionRunner into startup flow",
    "plan_section": "Step 5: Integrate into Startup Flow",
    "steps": [
      "Read specs/plans/chat_resumption_unified.md section 'Step 5'",
      "In src-tauri/src/lib.rs, after StartupJobRunner.run()",
      "Create ChatResumptionRunner with all required repos and state",
      "Call chat_resumption.run().await",
      "Add logging for resumption activity",
      "Run cargo clippy && cargo test",
      "Commit: feat(startup): wire ChatResumptionRunner into app startup"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Write unit tests for resumption logic",
    "plan_section": "Verification - Unit Tests",
    "steps": [
      "Read specs/plans/chat_resumption_unified.md section 'Verification'",
      "Test execution_state wiring - verify transitions happen",
      "Test interrupted conversations query - verify correct filtering",
      "Test priority ordering - verify TaskExecution first",
      "Test deduplication - verify is_handled_by_task_resumption works",
      "Run cargo test",
      "Commit: test(chat): add unit tests for chat resumption"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Reuse ChatService.send_message()** | Already handles background processing, queue, and state transitions for all chat types |
| **Deduplicate with StartupJobRunner** | TaskExecution/Review with tasks in AGENT_ACTIVE_STATUSES are already handled by task resumption |
| **Priority ordering** | TaskExecution most critical (active work), Project least critical (general discussion) |
| **"Continue where you left off" message** | Simple prompt that works with --resume flag to continue Claude session |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] execution_state wiring test passes
- [ ] interrupted conversations query returns correct results
- [ ] priority ordering test passes
- [ ] deduplication test passes

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Build succeeds (`cargo build --release`)

### Manual Testing
- [ ] Start ideation chat, send message, force quit while agent running, restart - chat resumes
- [ ] Same test with task chat (non-execution) - chat resumes
- [ ] Same test with project chat - chat resumes
- [ ] TaskExecution chat with task in Executing state - handled by StartupJobRunner, not ChatResumptionRunner

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified: App startup in lib.rs
- [ ] ChatResumptionRunner is instantiated AND run() called
- [ ] get_interrupted_conversations() returns data correctly
- [ ] ClaudeChatService.send_message() is called for each resumption

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
