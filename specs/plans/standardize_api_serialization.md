# Plan: Standardize API Serialization Convention (snake_case Boundary)

## Decision: snake_case Boundary Pattern

**Backend** always outputs snake_case (Rust convention) → **Transform layer** converts → **Frontend** uses camelCase (JS convention)

## Current State

| Layer | Convention | Correct? |
|-------|-----------|----------|
| Backend (64%) | snake_case | ✅ |
| Backend (36%) | camelCase via rename_all | ❌ Remove |
| Frontend API schemas | snake_case | ✅ |
| Frontend display types | camelCase | ✅ |

## Phase 1: Remove `rename_all = "camelCase"` from Backend (17 structs) (BLOCKING)

**Dependencies:** None
**Atomic Commits:** Per-file `fix(api): remove camelCase serialization from <file>`

### Task 1.1: Fix TaskProposalResponse (BLOCKING - ROOT CAUSE)
**Dependencies:** None
**Atomic Commit:** `fix(api): remove camelCase serialization from ideation_commands_types`

File: `src-tauri/src/commands/ideation_commands/ideation_commands_types.rs`
- [ ] `TaskProposalResponse` - **ROOT CAUSE OF BUG**

### Task 1.2: Fix task_commands/types.rs
**Dependencies:** None (can run parallel with 1.1)
**Atomic Commit:** `fix(api): remove camelCase serialization from task_commands types`

File: `src-tauri/src/commands/task_commands/types.rs`
- [ ] `AnswerUserQuestionResponse`
- [ ] `InjectTaskResponse`
- [ ] `TaskResponse`
- [ ] `TaskListResponse`
- [ ] `StatusTransition`

### Task 1.3: Fix task_step_commands_types.rs
**Dependencies:** None (can run parallel)
**Atomic Commit:** `fix(api): remove camelCase serialization from task_step_commands_types`

File: `src-tauri/src/commands/task_step_commands_types.rs`
- [ ] `TaskStepResponse`

### Task 1.4: Fix execution_commands.rs
**Dependencies:** None (can run parallel)
**Atomic Commit:** `fix(api): remove camelCase serialization from execution_commands`

File: `src-tauri/src/commands/execution_commands.rs`
- [ ] `ExecutionStatusResponse`
- [ ] `ExecutionCommandResponse`

### Task 1.5: Fix project_commands.rs
**Dependencies:** None (can run parallel)
**Atomic Commit:** `fix(api): remove camelCase serialization from project_commands`

File: `src-tauri/src/commands/project_commands.rs`
- [ ] `ProjectResponse`

### Task 1.6: Fix test_data_commands.rs
**Dependencies:** None (can run parallel)
**Atomic Commit:** `fix(api): remove camelCase serialization from test_data_commands`

File: `src-tauri/src/commands/test_data_commands.rs`
- [ ] `SeedDataResponse`

### Task 1.7: Fix unified_chat_commands.rs
**Dependencies:** None (can run parallel)
**Atomic Commit:** `fix(api): remove camelCase serialization from unified_chat_commands`

File: `src-tauri/src/commands/unified_chat_commands.rs`
- [ ] `SendAgentMessageInput`
- [ ] `SendAgentMessageResponse`
- [ ] `QueueAgentMessageInput`
- [ ] `QueuedMessageResponse`
- [ ] `AgentConversationResponse`

### Task 1.8: Fix workflow_commands.rs
**Dependencies:** None (can run parallel)
**Atomic Commit:** `fix(api): remove camelCase serialization from workflow_commands`

File: `src-tauri/src/commands/workflow_commands.rs`
- [ ] `StateGroupResponse`
- [ ] `WorkflowColumnResponse`

## Phase 2: Audit & Fix Frontend Schemas

**Dependencies:** Phase 1 (backend changes must complete first to test against)

For each backend change, verify the corresponding frontend Zod schema expects snake_case:

### Task 2.1: Verify/fix TaskResponse schema
**Dependencies:** Task 1.2
**Atomic Commit:** `fix(api): update TaskResponse schema to expect snake_case`

| Backend Struct | Frontend Schema Location | Action |
|----------------|-------------------------|--------|
| TaskResponse | `src/types/task.ts` | Check/fix |

### Task 2.2: Verify/fix ExecutionStatusResponse schema
**Dependencies:** Task 1.4
**Atomic Commit:** `fix(api): update ExecutionStatusResponse schema to expect snake_case`

| Backend Struct | Frontend Schema Location | Action |
|----------------|-------------------------|--------|
| ExecutionStatusResponse | `src/types/events.ts` or API | Check/fix |

### Task 2.3: Verify/fix ProjectResponse schema
**Dependencies:** Task 1.5
**Atomic Commit:** `fix(api): update ProjectResponse schema to expect snake_case`

| Backend Struct | Frontend Schema Location | Action |
|----------------|-------------------------|--------|
| ProjectResponse | `src/types/project.ts` | Check/fix |

### Task 2.4: Verify/fix QueuedMessageResponse schema
**Dependencies:** Task 1.7
**Atomic Commit:** `fix(api): update QueuedMessageResponse schema to expect snake_case`

| Backend Struct | Frontend Schema Location | Action |
|----------------|-------------------------|--------|
| QueuedMessageResponse | `src/api/chat.ts` | Check/fix |

### Task 2.5: Verify/fix TaskStepResponse schema
**Dependencies:** Task 1.3
**Atomic Commit:** `fix(api): update TaskStepResponse schema to expect snake_case`

| Backend Struct | Frontend Schema Location | Action |
|----------------|-------------------------|--------|
| TaskStepResponse | `src/types/task-step.ts` | Check/fix |

### Task 2.6: Verify TaskProposalResponse schema (already done)
**Dependencies:** Task 1.1
**Status:** ✅ Already snake_case in `src/api/ideation.schemas.ts`

## Phase 3: Ensure Transform Functions Exist

**Dependencies:** Phase 2

Each API module should have:
1. **Schema** (snake_case) - validates backend response
2. **Transform** - converts snake_case → camelCase
3. **Type** (camelCase) - used in components

### Task 3.1: Audit transform coverage
**Dependencies:** Phase 2 complete
**Atomic Commit:** `feat(api): add missing transform functions for snake_case conversion`

Example pattern from `src/api/ideation.ts`:
```typescript
// Schema (snake_case)
const TaskProposalResponseSchema = z.object({
  session_id: z.string(),
  suggested_priority: z.string(),
});

// Transform
function transformProposal(raw): TaskProposalResponse {
  return {
    sessionId: raw.session_id,
    suggestedPriority: raw.suggested_priority,
  };
}

// API call
async list(sessionId: string): Promise<TaskProposalResponse[]> {
  const raw = await typedInvoke("list_session_proposals", { sessionId }, schema);
  return raw.map(transformProposal);
}
```

## Phase 4: Document Convention in Code Quality Standards

**Dependencies:** Phase 1-3 (document after implementation is stable)

### Task 4.1: Update code-quality-standards.md (BLOCKING for future work)
**Dependencies:** Phase 3
**Atomic Commit:** `docs: add API serialization convention to code quality standards`

Add to `.claude/rules/code-quality-standards.md`:

```markdown
## API Serialization Convention

### The snake_case Boundary Pattern

| Layer | Convention | Example |
|-------|-----------|---------|
| Rust backend | snake_case | `session_id`, `created_at` |
| Frontend Zod schema | snake_case | `z.object({ session_id: z.string() })` |
| Transform function | converts | `sessionId: raw.session_id` |
| Frontend types | camelCase | `interface { sessionId: string }` |

### Backend Rules
- **NEVER** use `#[serde(rename_all = "camelCase")]` on response structs
- Rust structs serialize to snake_case by default (correct)
- Input structs may use `#[serde(rename_all = "camelCase")]` for Tauri param convenience

### Frontend Rules
- API schemas in `src/api/*.schemas.ts` expect **snake_case**
- Display types in `src/types/*.ts` use **camelCase**
- Transform functions in `src/api/*.transforms.ts` bridge the gap
- Every API wrapper must apply transforms before returning
```

### Task 4.2: Update src/CLAUDE.md
**Dependencies:** Task 4.1
**Atomic Commit:** `docs: add API schema convention to frontend CLAUDE.md`

Add to `src/CLAUDE.md` (Frontend section):

```markdown
### API Schema Convention (CRITICAL)
- Zod schemas for backend responses use **snake_case** (match Rust)
- Transform functions convert to **camelCase** for components
- See `.claude/rules/code-quality-standards.md` for full pattern
```

### Task 4.3: Update src-tauri/CLAUDE.md
**Dependencies:** Task 4.1
**Atomic Commit:** `docs: add response serialization convention to backend CLAUDE.md`

Add to `src-tauri/CLAUDE.md` (Backend section):

```markdown
### Response Serialization (CRITICAL)
- **NEVER** use `#[serde(rename_all = "camelCase")]` on response structs
- Rust's default snake_case serialization is correct
- Frontend handles case conversion via transform layer
```

## Verification

**Dependencies:** All phases complete

1. **Fix the immediate bug:**
   - Remove `#[serde(rename_all = "camelCase")]` from `TaskProposalResponse`
   - Restart app, verify proposals load

2. **Run tests:**
   ```bash
   cd src-tauri && cargo test
   cd .. && npm test
   ```

3. **Manual verification:**
   - Load ideation session with proposals
   - Create new proposal via chat
   - Verify tasks load in Kanban view
   - Test all affected features (execution status, projects, etc.)

## Files to Modify

### Backend (remove rename_all):
- `src-tauri/src/commands/ideation_commands/ideation_commands_types.rs`
- `src-tauri/src/commands/task_commands/types.rs`
- `src-tauri/src/commands/task_step_commands_types.rs`
- `src-tauri/src/commands/execution_commands.rs`
- `src-tauri/src/commands/project_commands.rs`
- `src-tauri/src/commands/test_data_commands.rs`
- `src-tauri/src/commands/unified_chat_commands.rs`
- `src-tauri/src/commands/workflow_commands.rs`

### Frontend (verify/fix schemas):
- `src/types/task.ts`
- `src/types/task-step.ts`
- `src/types/project.ts`
- `src/types/events.ts`
- `src/api/chat.ts`

### Documentation:
- `.claude/rules/code-quality-standards.md`
- `src/CLAUDE.md`
- `src-tauri/CLAUDE.md`

## Execution Order

1. **First:** Fix `TaskProposalResponse` (immediate bug fix) - Task 1.1
2. **Second:** Fix remaining 16 backend structs (Tasks 1.2-1.8, can run parallel)
3. **Third:** Audit and fix affected frontend schemas (Phase 2)
4. **Fourth:** Add documentation (Phase 4)
5. **Fifth:** Run full test suite and manual verification

## Commit Lock Workflow (Parallel Agent Coordination)

Reference: `.claude/rules/commit-lock.md`

### Before Committing
```bash
# 1. Establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Check/acquire lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Read lock content, wait 3s, retry up to 30s
  # If stale (same content >30s), delete and proceed
fi

# 3. Create lock
echo "<stream-name> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 4. Stage and commit
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

### After Committing
```bash
# ALWAYS release lock (success or failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Lock Rules
1. Acquire lock BEFORE `git add`
2. Release lock AFTER commit (success OR failure)
3. Stale = same content + >30 sec old
4. Never force-delete active lock from another agent

### Parallel Execution Notes
- Tasks 1.1-1.8 can all run in parallel (different files, no dependencies)
- Phase 2 tasks depend on their corresponding Phase 1 task
- Phase 4 documentation should wait until implementation is verified
- Each task should acquire commit lock independently when ready to commit
