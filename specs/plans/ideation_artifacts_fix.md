# Plan: Fix Ideation Artifacts Functionality

## Summary

The ideation artifacts UI elements are rendered but clicking them does nothing. The root cause is a **missing wiring** between backend mutations and frontend state updates.

## Root Cause Analysis

### Issue 1: Selection (Checkboxes, Bulk Select/Deselect) - BROKEN

**Flow:**
1. Click checkbox → `onSelectProposal(id)` → `handleSelectProposal` → `toggleSelection.mutate(proposalId)`
2. Backend `toggle_proposal_selection` updates DB ✅
3. Backend does NOT emit `proposal:updated` event ❌
4. Frontend mutation invalidates wrong queries (`proposalKeys.lists()` instead of `ideationKeys.sessionWithData(sessionId)`) ❌

**Why it breaks:** The frontend gets its proposals from `useIdeationSession` query (`ideationKeys.sessionWithData(sessionId)`), but the mutation doesn't invalidate this query because it only has the `proposalId`, not the `sessionId`.

### Issue 2: Sort by Priority - SHOULD WORK

**Flow:**
1. Click sort → `handleSortByPriority` → sorts locally → `reorder.mutate({ sessionId, proposalIds })`
2. Backend `reorder_proposals` updates DB ✅
3. Mutation invalidates `ideationKeys.sessionWithData(sessionId)` ✅

This SHOULD work. If it doesn't, the issue is elsewhere (possibly backend not returning updated data).

### Issue 3: Edit Proposal - ORPHANED

**Flow:**
1. Click edit button → `onEdit(proposalId)` → `handleEditProposal`
2. Handler is empty TODO ❌
3. `ProposalEditModal` exists but is never rendered ❌

## Solution

### Fix 1: Backend - Add Event Emission (RECOMMENDED)

Add `proposal:updated` event emission to commands that don't emit events:

**File:** `src-tauri/src/commands/ideation_commands/ideation_commands_proposals.rs`

1. **`toggle_proposal_selection`** (line 227-250): Add event emission after updating
2. **`set_proposal_selection`** (line 254-265): Add event emission after updating
3. **`reorder_proposals`** (line 269-285): Add event emission for each updated proposal (or a batch event)

This is the cleanest fix because:
- The event system already exists and works for `update_task_proposal`
- The frontend already listens for these events in `useProposalEvents`
- No changes needed in frontend mutation code

### Fix 2: Frontend - Wire Up Edit Modal

**File:** `src/App.tsx`

1. Add state for editing proposal:
   ```typescript
   const [editingProposalId, setEditingProposalId] = useState<string | null>(null);
   ```

2. Derive editing proposal from store:
   ```typescript
   const editingProposal = editingProposalId
     ? allProposals[editingProposalId] ?? null
     : null;
   ```

3. Get `updateProposal` from mutations:
   ```typescript
   const { toggleSelection, deleteProposal, reorder, updateProposal } = useProposalMutations();
   ```

4. Update `handleEditProposal`:
   ```typescript
   const handleEditProposal = useCallback((proposalId: string) => {
     setEditingProposalId(proposalId);
   }, []);
   ```

5. Add save handler:
   ```typescript
   const handleSaveProposal = useCallback(
     async (proposalId: string, data: UpdateProposalInput) => {
       try {
         await updateProposal.mutateAsync({ proposalId, changes: data });
         setEditingProposalId(null);
         toast.success("Proposal updated");
       } catch {
         toast.error("Failed to update proposal");
       }
     },
     [updateProposal]
   );
   ```

6. Import and render modal:
   ```typescript
   import { ProposalEditModal } from "@/components/Ideation";

   // In JSX:
   <ProposalEditModal
     proposal={editingProposal}
     onSave={handleSaveProposal}
     onCancel={() => setEditingProposalId(null)}
     isSaving={updateProposal.isPending}
   />
   ```

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/ideation_commands/ideation_commands_proposals.rs` | Add event emission to `toggle_proposal_selection`, `set_proposal_selection`, `reorder_proposals` |
| `src/App.tsx` | Add edit modal state, handlers, render `ProposalEditModal` |

## Implementation Order

1. **Backend events** (Fix 1) - This will fix selection/bulk operations
2. **Edit modal wiring** (Fix 2) - This completes edit functionality

## Verification

1. **Selection:**
   - Click checkbox on a proposal → selection indicator and checkbox should toggle
   - Click "Select All" → all proposals should show selected
   - Click "Deselect All" → all proposals should show deselected

2. **Sort:**
   - Click "Sort by Priority" → proposals reorder by priority score (highest first)

3. **Edit:**
   - Hover proposal, click edit icon → modal opens with proposal data
   - Modify fields, click Save → modal closes, proposal updates, "Modified" badge appears

4. **Apply:**
   - Select proposals, click Apply dropdown, choose target → tasks created in Kanban

## Notes

- The `updateProposal` mutation in `useProposals.ts` DOES invalidate `ideationKeys.sessionWithData(sessionId)` (line 117-119), so edit should work once wired
- The backend `update_task_proposal` command DOES emit `proposal:updated` event (line 191), so the event system will also update the store
