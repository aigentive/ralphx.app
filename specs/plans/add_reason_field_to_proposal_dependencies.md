# Plan: Add Reason Field to Proposal Dependencies

## Overview

Persist the AI's dependency reasoning so users see *why* dependencies exist when hovering over dependency badges.

**Current state:** `DependencySuggestion` (line 72-78 of `types.rs`) already receives `reason: Option<String>` from the dependency-suggester agent, but it's discarded.

**Goal:** Store and display reasons like "API needs database tables to exist".

## Implementation Steps

### Step 1: Database Migration (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(migrations): add reason column to proposal_dependencies`

**File:** `src-tauri/src/infrastructure/sqlite/migrations/v2_add_dependency_reason.rs` (NEW)

```rust
use rusqlite::Connection;
use crate::error::AppResult;
use super::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(
        conn,
        "proposal_dependencies",
        "reason",
        "TEXT DEFAULT NULL"
    )?;
    Ok(())
}
```

**File:** `src-tauri/src/infrastructure/sqlite/migrations/mod.rs`
- Add `mod v2_add_dependency_reason;`
- Register in `MIGRATIONS` array (version: 2, name: "add_dependency_reason")
- Bump `SCHEMA_VERSION` to 2

### Step 2: Repository Trait (BLOCKING)
**Dependencies:** Step 1
**Atomic Commit:** `feat(domain): add reason parameter to ProposalDependencyRepository`

**File:** `src-tauri/src/domain/repositories/proposal_dependency_repository.rs`

Update `add_dependency` signature:
```rust
async fn add_dependency(
    &self,
    proposal_id: &TaskProposalId,
    depends_on_id: &TaskProposalId,
    reason: Option<&str>,  // ADD
) -> AppResult<()>;
```

Update `get_all_for_session` return type (use tuple with 3 elements for simplicity):
```rust
async fn get_all_for_session(
    &self,
    session_id: &IdeationSessionId,
) -> AppResult<Vec<(TaskProposalId, TaskProposalId, Option<String>)>>;
```

Update mock implementation in tests (same file).

### Step 3: SQLite Repository Implementation
**Dependencies:** Step 1, Step 2
**Atomic Commit:** `feat(sqlite): implement reason storage in proposal_dependency_repo`

**File:** `src-tauri/src/infrastructure/sqlite/sqlite_proposal_dependency_repo.rs`

Update `add_dependency`:
- Accept `reason: Option<&str>` parameter
- Add `reason` to INSERT statement

Update `get_all_for_session`:
- SELECT now includes `reason` column
- Return 3-tuple `(proposal_id, depends_on_id, reason)`

### Step 4: HTTP Handler (BLOCKING)
**Dependencies:** Step 3
**Atomic Commit:** `feat(http): pass dependency reason through API layer`

**File:** `src-tauri/src/http_server/handlers/ideation.rs`

Update `apply_proposal_dependencies` (around line 480):
```rust
// Currently:
state.app_state.proposal_dependency_repo.add_dependency(&proposal_id, &depends_on_id).await?;

// Change to:
state.app_state.proposal_dependency_repo.add_dependency(
    &proposal_id,
    &depends_on_id,
    suggestion.reason.as_deref(),
).await?;
```

Update `analyze_session_dependencies` to include reason in edge response.

### Step 5: HTTP Response Types
**Dependencies:** Step 4
**Atomic Commit:** `feat(http): add reason field to DependencyEdgeResponse`

**File:** `src-tauri/src/http_server/types.rs`

Find `DependencyEdgeResponse` (or similar edge response struct) and add:
```rust
pub reason: Option<String>,
```

### Step 6: Frontend Schemas
**Dependencies:** Step 5
**Atomic Commit:** `feat(api): add reason to DependencyGraphEdgeResponseSchema`

**File:** `src/api/ideation.schemas.ts`

Update `DependencyGraphEdgeResponseSchema`:
```typescript
export const DependencyGraphEdgeResponseSchema = z.object({
  from: z.string(),
  to: z.string(),
  reason: z.string().nullable(),  // ADD
});
```

### Step 7: Frontend Types
**Dependencies:** Step 6
**Atomic Commit:** `feat(types): add reason to DependencyGraphEdge interface`

**File:** `src/types/ideation.ts`

Update `DependencyGraphEdge`:
```typescript
export interface DependencyGraphEdge {
  from: string;
  to: string;
  reason?: string;  // ADD
}
```

### Step 8: Frontend Transform
**Dependencies:** Step 6, Step 7
**Atomic Commit:** `feat(transforms): pass through dependency reason`

**File:** `src/api/ideation.transforms.ts`

Edges transform passes through reason (minimal change since field names match).

### Step 9: UI - ProposalCard Tooltip (BLOCKING)
**Dependencies:** Step 8
**Atomic Commit:** `feat(ProposalCard): display dependency reasons in tooltip`

**File:** `src/components/Ideation/ProposalCard.tsx`

Update props to accept dependency details:
```typescript
interface DependencyDetail {
  proposalId: string;
  title: string;
  reason?: string;
}

// Add to props
dependsOnDetails?: DependencyDetail[];
```

Update tooltip content (around line 195-210):
```tsx
<TooltipContent>
  <div className="space-y-1 text-xs">
    <div className="font-medium">Depends on {dependsOnDetails.length} proposal{dependsOnDetails.length !== 1 ? 's' : ''}:</div>
    {dependsOnDetails.map(dep => (
      <div key={dep.proposalId} className="text-[var(--text-muted)]">
        • {dep.title}{dep.reason && `: ${dep.reason}`}
      </div>
    ))}
  </div>
</TooltipContent>
```

### Step 10: UI - IdeationView Data Flow
**Dependencies:** Step 9
**Atomic Commit:** `feat(IdeationView): build and pass dependency details to ProposalCard`

**File:** `src/components/Ideation/IdeationView.tsx`

Update `useMemo` that builds dependency info to include reasons from edges:
```typescript
const { dependencyCounts, dependencyDetails, criticalPathSet } = useMemo(() => {
  // Build map: proposalId -> array of { title, reason } for each dependency
  const details: Record<string, DependencyDetail[]> = {};
  for (const edge of dependencyGraph.edges) {
    const targetProposal = proposals.find(p => p.id === edge.to);
    if (!details[edge.from]) details[edge.from] = [];
    details[edge.from].push({
      proposalId: edge.to,
      title: targetProposal?.title ?? 'Unknown',
      reason: edge.reason,
    });
  }
  // ...existing counts logic
}, [dependencyGraph, proposals]);
```

Pass `dependsOnDetails` to ProposalCard.

## Files Modified

| Layer | File | Change |
|-------|------|--------|
| Migration | `v2_add_dependency_reason.rs` | NEW |
| Migration | `migrations/mod.rs` | Register + bump version |
| Domain | `proposal_dependency_repository.rs` | Add reason param, update return type |
| Infra | `sqlite_proposal_dependency_repo.rs` | SQL changes |
| HTTP | `handlers/ideation.rs` | Pass reason, include in response |
| HTTP | `types.rs` | Add reason to edge response |
| API | `ideation.schemas.ts` | Add reason to schema |
| Types | `types/ideation.ts` | Add reason to edge type |
| Transform | `ideation.transforms.ts` | Pass through reason |
| UI | `ProposalCard.tsx` | Enhanced tooltip |
| UI | `IdeationView.tsx` | Build dependency details |

## Verification

1. **Migration test**: Run `cargo test` - verify column added to `proposal_dependencies`
2. **Backend test**: Create dependency with reason via API, verify it's stored and returned
3. **Frontend test**: Hover over dependency badge - should show proposal names + reasons
4. **E2E**: Run dependency-suggester agent on session with proposals - reasons should persist and display

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

## Task Dependency Graph

```
Step 1 (Migration) ─────┬──► Step 2 (Trait) ───► Step 3 (SQLite) ───► Step 4 (Handler) ───► Step 5 (Response Types)
                        │                                                                           │
                        │                                                                           ▼
                        │                                                               Step 6 (Schemas) ───► Step 7 (Types) ───┐
                        │                                                                                                       │
                        │                                                                                                       ▼
                        │                                                                               Step 8 (Transform) ───► Step 9 (ProposalCard) ───► Step 10 (IdeationView)
```

**Critical Path:** Steps 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10

**Parallel Opportunities:**
- Steps 6 and 7 can run in parallel after Step 5
- Step 8 depends on both 6 and 7 completing
