# Plan: Fix Escalation Data Not Showing in UI

## Problem Analysis

When AI Reviewer escalates a task via `complete_review` MCP tool, the escalation details (feedback, issues) don't appear in the UI. The UI shows "No escalation reason provided" instead of the actual feedback.

### Root Causes Identified

**1. Field Name Mismatch (CRITICAL)**
- MCP tool sends: `{ task_id, decision, feedback, issues }`
- Backend expects: `{ task_id, decision, comments }` (no `feedback` field)
- Result: `req.comments` is `None` → defaults to "No comments provided"

**File locations:**
- MCP tool definition: `ralphx-plugin/ralphx-mcp-server/src/tools.ts:412` → `feedback` field
- Backend struct: `src-tauri/src/http_server/types.rs:278` → `comments: Option<String>`
- Handler: `src-tauri/src/http_server/handlers/reviews.rs:52` → `req.comments.unwrap_or_else(|| "No comments provided")`

**2. Issues Array Completely Ignored**
- MCP tool accepts: `issues: [{ severity, file, line, description }]`
- Backend struct: No `issues` field at all
- Result: Detailed issue data from AI reviewer is silently discarded

### Current Data Flow

```
AI Reviewer → complete_review MCP tool
    ├── task_id: "..." ✅ Processed
    ├── decision: "escalate" ✅ Processed
    ├── feedback: "Why I'm escalating..." ❌ IGNORED (field name mismatch)
    └── issues: [{severity, description}] ❌ IGNORED (not in struct)

Backend stores:
    └── ReviewNote.notes = "No comments provided" ← This is what UI shows
```

## Solution: Full Fix

Fix field name mismatch AND add issues array support.

## Implementation Steps

### Step 1: Backend - Fix Request Struct (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(http_server): add ReviewIssue struct and fix field name mismatch`

**File: `src-tauri/src/http_server/types.rs`**

Add `ReviewIssue` struct and update `CompleteReviewRequest`:
```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReviewIssue {
    pub severity: String,  // "critical" | "major" | "minor" | "suggestion"
    pub file: Option<String>,
    pub line: Option<u32>,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteReviewRequest {
    pub task_id: String,
    pub decision: String,
    pub feedback: Option<String>,  // Renamed from comments
    pub issues: Option<Vec<ReviewIssue>>,  // NEW
}
```

### Step 2: Backend - Update Handler (BLOCKING)
**Dependencies:** Step 1
**Atomic Commit:** `fix(http_server): store issues in review notes`

**File: `src-tauri/src/http_server/handlers/reviews.rs`**

1. Change `req.comments` → `req.feedback` (line 52)
2. Store issues in review note (serialize to JSON in notes field, or structured)

Options for storing issues:
- **Option A (simple):** Serialize to JSON, prepend to notes string
- **Option B (structured):** Store as JSON in a new `issues_json` column on review_notes

Recommend Option A for simplicity - no migration needed.

### Step 3: Backend - Update Response Types (BLOCKING)
**Dependencies:** Step 1, Step 2
**Atomic Commit:** `fix(http_server): include issues in review note response`

**File: `src-tauri/src/http_server/types.rs`**

Update `ReviewNoteResponse` to include issues:
```rust
#[derive(Debug, Serialize)]
pub struct ReviewNoteResponse {
    pub id: String,
    pub reviewer: String,
    pub outcome: String,
    pub notes: Option<String>,
    pub issues: Option<Vec<ReviewIssue>>,  // NEW
    pub created_at: String,
}
```

**File: `src-tauri/src/http_server/handlers/reviews.rs`**

In `get_review_notes`, parse issues from notes and include in response.

### Step 4: Frontend - Update Types (BLOCKING)
**Dependencies:** Step 3
**Atomic Commit:** `fix(api): add ReviewIssue schema and update response types`

**File: `src/lib/tauri/reviews-api.schemas.ts`**

Add issue schema and update response:
```typescript
export const ReviewIssueSchema = z.object({
  severity: z.enum(["critical", "major", "minor", "suggestion"]),
  file: z.string().nullable(),
  line: z.number().nullable(),
  description: z.string(),
});

export const ReviewNoteResponseSchema = z.object({
  id: z.string(),
  reviewer: z.string(),
  outcome: z.string(),
  notes: z.string().nullable(),
  issues: z.array(ReviewIssueSchema).nullable(),  // NEW
  created_at: z.string(),
});
```

### Step 5: Frontend - Add Issues Display Component
**Dependencies:** Step 4
**Atomic Commit:** `feat(components): add ReviewIssuesList for escalation details`

**File: `src/components/tasks/detail-views/EscalatedTaskDetail.tsx`**

Add `ReviewIssuesList` component:
```tsx
function ReviewIssuesList({ issues }: { issues: ReviewIssue[] }) {
  const severityColors = {
    critical: "var(--status-error)",
    major: "var(--status-warning)",
    minor: "var(--text-muted)",
    suggestion: "var(--accent-primary)",
  };

  return (
    <div className="space-y-2">
      {issues.map((issue, i) => (
        <div key={i} className="flex items-start gap-2 p-2 rounded bg-black/20">
          <span style={{ color: severityColors[issue.severity] }}>
            {issue.severity.toUpperCase()}
          </span>
          <div>
            {issue.file && <span className="text-white/50">{issue.file}:{issue.line}</span>}
            <p>{issue.description}</p>
          </div>
        </div>
      ))}
    </div>
  );
}
```

Integrate into `AIEscalationReasonCard` or as separate section.

## Verification

1. Run backend tests: `cargo test` in `src-tauri/`
2. Run frontend typecheck: `npm run typecheck` in root
3. Start dev server manually
4. Trigger an escalation via AI reviewer with feedback and issues
5. Open escalated task detail view
6. Verify:
   - Escalation reason shows actual feedback text (not "No escalation reason provided")
   - Issues list displays with severity badges and file:line references

## Critical Files

| File | Change |
|------|--------|
| `src-tauri/src/http_server/types.rs` | Add `ReviewIssue` struct, rename `comments`→`feedback`, add `issues` field |
| `src-tauri/src/http_server/handlers/reviews.rs` | Update field reference, store/retrieve issues |
| `src/lib/tauri/reviews-api.schemas.ts` | Add `ReviewIssueSchema`, update response schema |
| `src/components/tasks/detail-views/EscalatedTaskDetail.tsx` | Add `ReviewIssuesList` component |

## Notes

- The UI component `EscalatedTaskDetail` is correctly wired to display `review.notes`
- The `AIEscalationReasonCard` already has the structure to show escalation details
- Main work is fixing the backend field name and adding issues throughout the stack

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
