# RalphX - Phase 75: Wire Up Merge Chat Context in UI

## Overview

When a task enters the `Merging` state, the chat panel doesn't switch to show the merge conversation, leaving users unable to see what the merger agent is doing. This phase wires up the merge conversation context to the chat panel, completing the chat context triad (execution, review, merge).

**Reference Plan:**
- `specs/plans/fix_wire_up_merge_chat_context_in_ui.md` - Detailed implementation plan with code snippets

## Goals

1. Add `isMergeMode` detection based on `MERGE_STATUSES` in IntegratedChatPanel
2. Query and display merge conversations when task is in merge state
3. Show "Merge #N" titles in conversation selector

## Dependencies

### Phase 66 (Per-Task Git Branch Isolation) - Required

| Dependency | Why Needed |
|------------|------------|
| Merge states (`pending_merge`, `merging`) | This phase displays chat for those states |
| Merger agent conversations | Must exist to be displayed |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_wire_up_merge_chat_context_in_ui.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_wire_up_merge_chat_context_in_ui.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Add merge mode detection and context hook support",
    "plan_section": "Task 1: Add merge mode detection and context hook support",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(chat): add merge mode detection to IntegratedChatPanel and useChatPanelContext",
    "steps": [
      "Read specs/plans/fix_wire_up_merge_chat_context_in_ui.md sections '1a. IntegratedChatPanel.tsx' and '1b. useChatPanelContext.ts'",
      "In IntegratedChatPanel.tsx: Import MERGE_STATUSES from @/types/status",
      "Add isMergeMode detection after isReviewMode (line ~90)",
      "Add merge conversations query after review conversations query (~135)",
      "Update conversations selector to include merge mode (line ~138-142)",
      "Pass isMergeMode to useChatPanelContext hook props",
      "Update ConversationSelector contextType to include merge mode",
      "In useChatPanelContext.ts: Add isMergeMode to hook props interface",
      "Update currentContextType computation to handle merge mode",
      "Update storeContextKey to include merge context key",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): add merge mode detection to IntegratedChatPanel and useChatPanelContext"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add merge title formatting to ConversationSelector",
    "plan_section": "Task 2: Add merge title formatting to ConversationSelector",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(chat): add merge conversation title formatting",
    "steps": [
      "Read specs/plans/fix_wire_up_merge_chat_context_in_ui.md section 'ConversationSelector.tsx'",
      "In ConversationSelector.tsx: Find getConversationTitle function (~line 72-93)",
      "Add case 'merge': return `Merge #${index + 1}` to the switch statement",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): add merge conversation title formatting"
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
| **Single compilation unit for Task 1** | IntegratedChatPanel passes `isMergeMode` to useChatPanelContext, requiring both files to be modified together |
| **Follow existing execution/review pattern** | Merge mode uses identical query + context switching pattern established by execution and review modes |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] Chat panel switches to merge context when task is in `pending_merge` or `merging` status

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Create a task with a branch that will have merge conflicts
- [ ] Approve the task → transitions to PendingMerge → Merging
- [ ] Verify: Chat panel switches to show merge conversation
- [ ] Verify: Can see merger agent's activity in real-time
- [ ] Verify: Conversation selector shows "Merge #1" title

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (task enters merge state)
- [ ] isMergeMode correctly detects MERGE_STATUSES
- [ ] mergeConversations query fetches with correct context type
- [ ] Context type switches to "merge" in useChatPanelContext
- [ ] ConversationSelector displays "Merge #N" titles

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
