# Plan: Fix Escalation Reason Not Displaying

## Problem Summary

When an AI reviewer escalates a task via `complete_review`, the escalation reason/feedback is sent and stored, but the `EscalatedTaskDetail` UI component shows "No escalation reason provided" instead of displaying the actual feedback.

## Root Cause Analysis

**Two response paths exist - only one parses issues:**

| Path | Location | Parses Issues? | Used By |
|------|----------|----------------|---------|
| HTTP endpoint | `reviews.rs:get_review_notes` | YES | MCP server |
| Tauri command | `review_commands.rs:get_task_state_history` | NO | Frontend |

**The Tauri command's `ReviewNoteResponse` struct in `review_commands_types.rs:67-88`:**
- Missing the `issues` field entirely
- `From<ReviewNote>` impl passes raw notes without parsing

**Frontend expects (`reviews-api.schemas.ts:60-68`):**
```typescript
ReviewNoteResponseSchema = z.object({
  notes: z.string().nullable().optional(),
  issues: z.array(ReviewIssueSchema).nullable().optional(), // EXPECTS THIS
});
```

**Data flow:**
1. AI reviewer sends `complete_review` with `feedback` + `issues`
2. HTTP handler embeds issues as JSON in notes: `{"issues":[...]}\nfeedback_text`
3. `ReviewNote` stored with combined text in `notes` field
4. Tauri command returns raw notes without parsing issues out
5. Frontend receives notes with embedded JSON, but `issues` is undefined
6. UI shows raw JSON in notes or "No escalation reason" if notes is null

## Fix Strategy

**Option A (Recommended):** Parse issues in Tauri command response
- Add `issues` field to `ReviewNoteResponse` in `review_commands_types.rs`
- Reuse or extract `parse_issues_from_notes` logic from `reviews.rs`
- Update `get_task_state_history` to parse issues before returning

**Option B:** Move parsing to frontend
- Keep backend returning raw notes
- Add parsing logic to frontend hook/component
- Less ideal: duplicates logic across languages

## Implementation Plan

### Task 1: Add issues field to Tauri ReviewNoteResponse (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(reviews): add ReviewIssue type and issues field to ReviewNoteResponse`

**File:** `src-tauri/src/commands/review_commands_types.rs`

Add to `ReviewNoteResponse` struct:
```rust
pub struct ReviewNoteResponse {
    pub id: String,
    pub task_id: String,
    pub reviewer: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issues: Option<Vec<ReviewIssue>>,  // ADD THIS
    pub created_at: String,
}
```

Add `ReviewIssue` struct (or reuse from HTTP types):
```rust
#[derive(Debug, Serialize)]
pub struct ReviewIssue {
    pub severity: String,
    pub file: Option<String>,
    pub line: Option<i32>,
    pub description: String,
}
```

### Task 2: Extract parsing helper to shared location (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `refactor(reviews): extract parse_issues_from_notes to shared module`

**Current location:** `src-tauri/src/http_server/handlers/reviews.rs:425-489`

**Move to:** New file or existing shared module (e.g., `commands/review_helpers.rs`)

The helper `parse_issues_from_notes` parses the embedded JSON format:
```
{"issues":[...]}\n<feedback_text>
```

**Note:** Task 1 and Task 2 can be executed in parallel (no dependency between them).

### Task 3: Update get_task_state_history command

**Dependencies:** Task 1, Task 2
**Atomic Commit:** `feat(reviews): parse issues in get_task_state_history response`

**File:** `src-tauri/src/commands/review_commands.rs`

Update the command to parse issues:
```rust
pub async fn get_task_state_history(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewNoteResponse>, String> {
    let task_id = TaskId::from_string(task_id);
    state
        .review_repo
        .get_notes_by_task_id(&task_id)
        .await
        .map(|notes| {
            notes.into_iter().map(|note| {
                let (issues, clean_notes) = parse_issues_from_notes(&note.notes);
                ReviewNoteResponse {
                    id: note.id.as_str().to_string(),
                    task_id: note.task_id.as_str().to_string(),
                    reviewer: note.reviewer.to_string(),
                    outcome: note.outcome.to_string(),
                    notes: clean_notes,
                    issues,
                    created_at: note.created_at.to_rfc3339(),
                }
            }).collect()
        })
        .map_err(|e| e.to_string())
}
```

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/review_commands_types.rs` | Add `issues` field and `ReviewIssue` type |
| `src-tauri/src/commands/review_commands.rs` | Update command to parse issues |
| `src-tauri/src/http_server/handlers/reviews.rs` | Extract `parse_issues_from_notes` to shared location |

## Verification

1. Run backend tests: `cargo test -p ralphx-lib`
2. Run frontend tests: `npm run test:run`
3. Manual test:
   - Create a task and run to review stage
   - Have AI reviewer escalate with feedback and issues
   - Verify `EscalatedTaskDetail` shows:
     - Escalation reason text (not JSON)
     - Issues list with severity badges

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
