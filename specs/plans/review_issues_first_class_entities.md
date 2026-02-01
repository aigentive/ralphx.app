# Plan: Review Issues as First-Class Entities

## Summary

Add structured issue tracking to reviews, mirroring the TaskStep pattern. Reviews create issues, execution addresses them, next review verifies them.

## Current State

- `ReviewIssue` struct exists in Rust domain but is embedded in ReviewNote.issues JSON
- No separate table, no lifecycle tracking, no MCP tools
- Frontend doesn't render issues (schema missing fields)
- Agents submit flat text notes, not structured issues

## Target State

```
Review Cycle with Issues:

Execute Attempt 1
    ↓
Review 1 → Creates Issues [I1, I2, I3] (status: open)
    ↓
Re-Execute Attempt 2
    - Gets open issues [I1, I2, I3]
    - Addresses I1, I2 (status: addressed)
    ↓
Review 2
    - Verifies I1 (status: verified)
    - I2 not fixed → reopens (status: open)
    - Creates new I4 (status: open)
    ↓
Re-Execute Attempt 3
    - Gets open issues [I2, I4]
    ...
```

---

## Phase 1: Backend - ReviewIssue Entity

### 1.1 Database Migration (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(backend): add review_issues table migration`

**File:** `src-tauri/src/infrastructure/sqlite/migrations/vN_review_issues.rs`

```sql
CREATE TABLE IF NOT EXISTS review_issues (
    id TEXT PRIMARY KEY,
    review_note_id TEXT NOT NULL,  -- which review created this
    task_id TEXT NOT NULL,
    step_id TEXT,                   -- optional link to step
    no_step_reason TEXT,            -- required if step_id is NULL (AI justification)

    -- Issue details
    title TEXT NOT NULL,
    description TEXT,
    severity TEXT NOT NULL CHECK (severity IN ('critical', 'major', 'minor', 'suggestion')),
    category TEXT CHECK (category IN ('bug', 'missing', 'quality', 'design')),

    -- Location (optional)
    file_path TEXT,
    line_number INTEGER,
    code_snippet TEXT,

    -- Status lifecycle
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'in_progress', 'addressed', 'verified', 'wontfix')),

    -- Resolution tracking
    resolution_notes TEXT,
    addressed_in_attempt INTEGER,    -- which attempt addressed this
    verified_by_review_id TEXT,      -- which review verified this

    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    FOREIGN KEY (task_id) REFERENCES tasks(id),
    FOREIGN KEY (review_note_id) REFERENCES review_notes(id),
    FOREIGN KEY (step_id) REFERENCES task_steps(id)
);

CREATE INDEX idx_review_issues_task ON review_issues(task_id);
CREATE INDEX idx_review_issues_status ON review_issues(status);
CREATE INDEX idx_review_issues_review_note ON review_issues(review_note_id);
```

### 1.2 Domain Entity (BLOCKING)
**Dependencies:** Task 1.1
**Atomic Commit:** `feat(backend): add ReviewIssue domain entity and enums`

**File:** `src-tauri/src/domain/review_issue.rs`

```rust
pub struct ReviewIssue {
    pub id: ReviewIssueId,
    pub review_note_id: ReviewNoteId,
    pub task_id: TaskId,

    // Step linking (optional but requires justification if None)
    pub step_id: Option<StepId>,
    pub no_step_reason: Option<String>,  // Required if step_id is None

    pub title: String,
    pub description: Option<String>,
    pub severity: IssueSeverity,
    pub category: Option<IssueCategory>,

    pub file_path: Option<String>,
    pub line_number: Option<i32>,
    pub code_snippet: Option<String>,

    pub status: IssueStatus,
    pub resolution_notes: Option<String>,
    pub addressed_in_attempt: Option<i32>,
    pub verified_by_review_id: Option<ReviewNoteId>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum IssueStatus {
    Open,
    InProgress,
    Addressed,
    Verified,
    WontFix,
}

pub enum IssueSeverity {
    Critical,
    Major,
    Minor,
    Suggestion,
}

pub enum IssueCategory {
    Bug,
    Missing,
    Quality,
    Design,
}
```

### 1.3 Repository (BLOCKING)
**Dependencies:** Task 1.1, Task 1.2
**Atomic Commit:** `feat(backend): add ReviewIssue repository with CRUD operations`

**File:** `src-tauri/src/infrastructure/sqlite/review_issue_repository.rs`

Methods:
- `create(issue: ReviewIssue) -> Result<ReviewIssue>`
- `bulk_create(issues: Vec<ReviewIssue>) -> Result<Vec<ReviewIssue>>`
- `get_by_id(id: ReviewIssueId) -> Result<Option<ReviewIssue>>`
- `get_by_task_id(task_id: TaskId) -> Result<Vec<ReviewIssue>>`
- `get_open_by_task_id(task_id: TaskId) -> Result<Vec<ReviewIssue>>`
- `update_status(id, status, resolution_notes?) -> Result<ReviewIssue>`
- `get_summary(task_id: TaskId) -> Result<IssueProgressSummary>`

### 1.4 Service Layer (BLOCKING)
**Dependencies:** Task 1.3
**Atomic Commit:** `feat(backend): add ReviewIssue service with business logic`

**File:** `src-tauri/src/application/review_issue_service.rs`

Methods:
- `create_issues_from_review(review_note_id, task_id, issues: Vec<CreateIssueInput>)`
- `mark_issue_in_progress(issue_id)`
- `mark_issue_addressed(issue_id, resolution_notes, attempt_number)`
- `verify_issue(issue_id, review_note_id)`
- `reopen_issue(issue_id, reason)`
- `get_issue_progress(task_id) -> IssueProgressSummary`

---

## Phase 2: MCP Tools

### 2.1 Review Agent Tools (BLOCKING)
**Dependencies:** Task 1.4
**Atomic Commit:** `feat(backend): add Tauri commands for review issues`

**complete_review** (UPDATED) - Now includes structured issues
```typescript
{
  task_id: string,
  outcome: "approved" | "needs_changes" | "escalate",
  summary: string,           // Short summary for timeline display
  notes?: string,            // Optional detailed notes
  issues: [{                 // Required for needs_changes, optional for approved
    title: string,
    description?: string,
    severity: "critical" | "major" | "minor" | "suggestion",
    category?: "bug" | "missing" | "quality" | "design",
    step_id?: string,        // Link to step OR provide no_step_reason
    no_step_reason?: string, // Required if step_id is null
    file_path?: string,
    line_number?: number,
    code_snippet?: string
  }],
  fix_description?: string,
  escalation_reason?: string
}
```

**Validation:** If `outcome = "needs_changes"` and `issues` is empty → error.
**Validation:** For each issue, either `step_id` OR `no_step_reason` must be provided.

**verify_issue** - Mark issue as verified after re-review
```typescript
{ issue_id: string }
```

**reopen_issue** - Issue not actually fixed
```typescript
{ issue_id: string, reason: string }
```

### 2.2 Execute Agent Tools
**Dependencies:** Task 2.1
**Atomic Commit:** `feat(backend): add execute agent issue tracking commands`

**get_task_issues** - Get issues to address
```typescript
{ task_id: string, status_filter?: "open" | "all" }
// Returns: IssueWithContext[]
```

**mark_issue_in_progress** - Starting work on issue
```typescript
{ issue_id: string }
```

**mark_issue_addressed** - Completed work on issue
```typescript
{
  issue_id: string,
  resolution_notes: string,
  attempt_number: number
}
```

### 2.3 Update complete_review Tool
**Dependencies:** Task 2.1
**Atomic Commit:** `feat(backend): update CompleteReviewInput with structured issues`

**Note:** This task modifies `CompleteReviewInput` struct and adds validation. Since it changes an existing struct signature, all usages must be updated in the SAME task to maintain compilation unit integrity.

Modify `CompleteReviewInput` to include structured issues:

```rust
pub struct CompleteReviewInput {
    pub outcome: ReviewToolOutcome,
    pub summary: String,                    // Short summary for timeline
    pub notes: Option<String>,              // Optional detailed notes
    pub issues: Vec<ReviewIssueInput>,      // Structured issues list
    pub fix_description: Option<String>,
    pub escalation_reason: Option<String>,
}

pub struct ReviewIssueInput {
    pub title: String,
    pub description: Option<String>,
    pub severity: IssueSeverity,
    pub category: Option<IssueCategory>,
    pub step_id: Option<StepId>,
    pub no_step_reason: Option<String>,     // Required if step_id is None
    pub file_path: Option<String>,
    pub line_number: Option<i32>,
    pub code_snippet: Option<String>,
}
```

**Service validation:**
- If `outcome == NeedsChanges` and `issues.is_empty()` → Error
- For each issue: `step_id.is_some() || no_step_reason.is_some()` → else Error

---

## Phase 3: Frontend

### 3.1 Types & Schemas (BLOCKING)
**Dependencies:** Task 2.1 (backend commands must exist first)
**Atomic Commit:** `feat(frontend): add ReviewIssue types and Zod schemas`

**File:** `src/types/review-issue.ts`

```typescript
export const ReviewIssueSchema = z.object({
  id: z.string(),
  reviewNoteId: z.string(),
  taskId: z.string(),

  // Step linking
  stepId: z.string().nullable(),
  noStepReason: z.string().nullable(),  // Why no step link (required if stepId null)

  title: z.string(),
  description: z.string().nullable(),
  severity: z.enum(["critical", "major", "minor", "suggestion"]),
  category: z.enum(["bug", "missing", "quality", "design"]).nullable(),

  filePath: z.string().nullable(),
  lineNumber: z.number().nullable(),
  codeSnippet: z.string().nullable(),

  status: z.enum(["open", "in_progress", "addressed", "verified", "wontfix"]),
  resolutionNotes: z.string().nullable(),
  addressedInAttempt: z.number().nullable(),

  createdAt: z.string(),
  updatedAt: z.string(),
});

export const IssueProgressSummarySchema = z.object({
  taskId: z.string(),
  total: z.number(),
  open: z.number(),
  inProgress: z.number(),
  addressed: z.number(),
  verified: z.number(),
  bySeverity: z.object({
    critical: z.number(),
    major: z.number(),
    minor: z.number(),
    suggestion: z.number(),
  }),
});
```

### 3.2 API Layer (BLOCKING)
**Dependencies:** Task 3.1
**Atomic Commit:** `feat(frontend): add reviewIssuesApi with Tauri invocations`

**File:** `src/api/review-issues.ts`

```typescript
export const reviewIssuesApi = {
  getByTaskId: (taskId: string) =>
    typedInvokeWithTransform("get_task_issues", { taskId }, ...),

  getProgress: (taskId: string) =>
    typedInvokeWithTransform("get_issue_progress", { taskId }, ...),

  updateStatus: (issueId: string, status: IssueStatus, notes?: string) =>
    typedInvokeWithTransform("update_issue_status", { issueId, status, notes }, ...),
};
```

### 3.3 UI Components
**Dependencies:** Task 3.2
**Atomic Commit:** `feat(frontend): add IssueList and IssueTimeline components`

**IssueList** - Display issues with severity badges
```
src/components/Reviews/IssueList.tsx
├─ IssueCard (severity color, status badge, file:line link)
├─ IssueProgressBar (like StepProgressBar)
└─ GroupBy: severity | status | step
```

**IssueTimeline** - Show issue lifecycle across attempts
```
src/components/Reviews/IssueTimeline.tsx
├─ Created in Review 1
├─ Addressed in Attempt 2
├─ Verified in Review 2
```

**Integrate into StateHistoryTimeline**
- Show issues under each review entry
- Collapse/expand issue list
- Show issue diff between reviews (new, resolved, reopened)

---

## Phase 4: Workflow Integration

### 4.1 Review Agent Prompt Updates
**Dependencies:** Task 2.1, Task 2.3
**Atomic Commit:** `feat(plugin): update reviewer agent for structured issues`

Update reviewer agent to use structured issues:

```
When reviewing, create structured issues:
- Use create_review_issues tool
- Each issue needs: title, severity, category
- Link to step_id if issue relates to a specific step
- Include file_path:line_number for code issues
```

### 4.2 Execute Agent Prompt Updates
**Dependencies:** Task 2.2
**Atomic Commit:** `feat(plugin): update worker agent for issue tracking`

Update worker agent to track issue resolution:

```
Before starting work:
- Call get_task_issues to see open issues
- Prioritize by severity (critical first)

While working:
- Call mark_issue_in_progress when starting on an issue
- Call mark_issue_addressed with resolution notes when done

End of execution:
- All open issues should be addressed or have notes explaining why not
```

### 4.3 Progress Tracking
**Dependencies:** Task 3.3
**Atomic Commit:** `feat(frontend): integrate issue progress into StateHistoryTimeline`

**IssueProgressSummary** structure (mirrors StepProgressSummary):

```typescript
{
  taskId: string,
  total: number,
  open: number,
  inProgress: number,
  addressed: number,
  verified: number,
  wontfix: number,
  percentResolved: number,  // (addressed + verified + wontfix) / total
  bySeverity: {
    critical: { total, open, resolved },
    major: { total, open, resolved },
    minor: { total, open, resolved },
    suggestion: { total, open, resolved },
  }
}
```

---

## File Modifications Summary

| Layer | File | Change |
|-------|------|--------|
| Migration | `vN_review_issues.rs` | NEW - create table |
| Domain | `review_issue.rs` | NEW - entity + enums |
| Domain | `mod.rs` | Export new module |
| Repository | `review_issue_repository.rs` | NEW - CRUD + queries |
| Service | `review_issue_service.rs` | NEW - business logic |
| Commands | `review_commands.rs` | Add issue commands |
| MCP | `ralphx-mcp-server/` | Add issue tools |
| Frontend | `src/types/review-issue.ts` | NEW - schemas |
| Frontend | `src/api/review-issues.ts` | NEW - API layer |
| Frontend | `src/components/Reviews/IssueList.tsx` | NEW - UI |
| Frontend | `StateHistoryTimeline.tsx` | Integrate issues |

---

## Verification

1. **Backend:**
   - Migration runs successfully
   - `cargo test` passes for new repository/service
   - Tauri commands work via dev tools

2. **MCP:**
   - Agent can call `create_review_issues`
   - Agent can call `get_task_issues`, `mark_issue_addressed`

3. **Frontend:**
   - Issues display in review timeline
   - Progress bar shows issue resolution
   - Status updates work

4. **E2E:**
   - Review creates issues → Re-execute addresses them → Next review verifies
   - Issue history shows complete lifecycle

---

## Task Dependency Graph

```
Phase 1 (Backend):
  1.1 Migration ─────────────┐
          │                  │
          ▼                  │
  1.2 Domain Entity ─────────┤
          │                  │
          ▼                  │
  1.3 Repository ────────────┤
          │                  │
          ▼                  │
  1.4 Service ───────────────┘
          │
          ▼
Phase 2 (MCP/Commands):
  2.1 Review Agent Tools ────┬────────────────┐
          │                  │                │
          ▼                  ▼                │
  2.2 Execute Tools    2.3 Update complete_review
          │                  │                │
          │                  │                │
          ▼                  ▼                │
Phase 3 (Frontend):                           │
  3.1 Types & Schemas ◄──────────────────────┘
          │
          ▼
  3.2 API Layer
          │
          ▼
  3.3 UI Components
          │
          ▼
Phase 4 (Integration):
  4.1 Review Agent Prompts ◄── 2.1, 2.3
  4.2 Execute Agent Prompts ◄── 2.2
  4.3 Progress Tracking ◄── 3.3
```

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Compilation Unit Notes

**Task 2.3 is critical:** Modifying `CompleteReviewInput` changes an existing struct. All callers of `complete_review` must be updated in the same task to maintain compilation. This includes:
- The handler that receives the input
- Any test files that construct the input
- Any MCP tool definitions that use this struct

**Frontend schema changes:** Task 3.1 adds new types (additive), so it compiles independently. However, if existing schemas are modified, ensure all usages are updated in the same task.
