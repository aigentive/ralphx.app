# RalphX - Phase 30: Ideation Artifacts Fix

## Overview

The ideation artifacts UI elements (selection checkboxes, edit button, sort by priority) are rendered but clicking them does nothing. The root cause is **missing event emission** in backend commands and **orphaned wiring** for the edit modal.

This phase fixes the broken connection between backend mutations and frontend state updates by adding event emission to selection/reorder commands and wiring up the existing `ProposalEditModal` component.

**Reference Plan:**
- `specs/plans/ideation_artifacts_fix.md` - Root cause analysis and implementation details

## Goals

1. Fix proposal selection (checkboxes, bulk select/deselect) by emitting events from backend
2. Wire up the existing `ProposalEditModal` component so users can edit proposals
3. Ensure sort by priority reflects changes in the UI

## Dependencies

### Phase 10 (Ideation) - Required

| Dependency | Why Needed |
|------------|------------|
| `ProposalEditModal` component | Already exists but not wired up |
| `useProposalEvents` hook | Already listens for `proposal:updated` events |
| `toggle_proposal_selection` command | Needs event emission added |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/ideation_artifacts_fix.md`
2. Understand the root cause analysis for each issue
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
2. **Read the ENTIRE implementation plan** at `specs/plans/ideation_artifacts_fix.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Add event emission to toggle_proposal_selection command",
    "plan_section": "Fix 1: Backend - Add Event Emission",
    "steps": [
      "Read specs/plans/ideation_artifacts_fix.md section 'Fix 1'",
      "In ideation_commands_proposals.rs, modify toggle_proposal_selection (line 227-250)",
      "After update_selection call, fetch the updated proposal from the repo",
      "Emit 'proposal:updated' event with the proposal data (same pattern as update_task_proposal)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(ideation): emit proposal:updated event on selection toggle"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add event emission to set_proposal_selection command",
    "plan_section": "Fix 1: Backend - Add Event Emission",
    "steps": [
      "Read specs/plans/ideation_artifacts_fix.md section 'Fix 1'",
      "In ideation_commands_proposals.rs, modify set_proposal_selection (line 254-265)",
      "After update_selection call, fetch the updated proposal from the repo",
      "Emit 'proposal:updated' event with the proposal data",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(ideation): emit proposal:updated event on selection set"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add event emission to reorder_proposals command",
    "plan_section": "Fix 1: Backend - Add Event Emission",
    "steps": [
      "Read specs/plans/ideation_artifacts_fix.md section 'Fix 1'",
      "In ideation_commands_proposals.rs, modify reorder_proposals (line 269-285)",
      "After reorder call, emit 'proposals:reordered' event with sessionId and proposalIds",
      "Alternatively, fetch updated proposals and emit individual 'proposal:updated' events",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(ideation): emit event on proposal reorder"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Wire up ProposalEditModal in App.tsx",
    "plan_section": "Fix 2: Frontend - Wire Up Edit Modal",
    "steps": [
      "Read specs/plans/ideation_artifacts_fix.md section 'Fix 2'",
      "In App.tsx, add editingProposalId state: useState<string | null>(null)",
      "Derive editingProposal from allProposals using editingProposalId",
      "Add updateProposal to the useProposalMutations destructuring",
      "Update handleEditProposal to call setEditingProposalId(proposalId)",
      "Add handleSaveProposal callback that calls updateProposal.mutateAsync",
      "Import ProposalEditModal from @/components/Ideation",
      "Render ProposalEditModal with proposal, onSave, onCancel, isSaving props",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(ideation): wire up ProposalEditModal for editing proposals"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Event emission over query invalidation** | The event system already exists and works for `update_task_proposal`. Using events keeps the pattern consistent and doesn't require the frontend to know about session IDs in mutations. |
| **Wire existing modal instead of creating new** | `ProposalEditModal` already exists with full functionality. Only the wiring in App.tsx is missing. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `toggle_proposal_selection` emits `proposal:updated` event
- [ ] `set_proposal_selection` emits `proposal:updated` event
- [ ] `reorder_proposals` emits appropriate event

### Frontend - Run `npm run test`
- [ ] `ProposalEditModal` renders when editingProposalId is set
- [ ] Modal closes after successful save

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Click checkbox on a proposal → selection indicator and checkbox toggle
- [ ] Click "Select All" → all proposals show selected
- [ ] Click "Deselect All" → all proposals show deselected
- [ ] Click "Sort by Priority" → proposals reorder by priority score (highest first)
- [ ] Hover proposal, click edit icon → modal opens with proposal data
- [ ] Modify fields, click Save → modal closes, proposal updates, "Modified" badge appears
- [ ] Select proposals, click Apply dropdown, choose target → tasks created in Kanban

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (click handler, route, event listener)
- [ ] New component is imported AND rendered (not behind disabled flag)
- [ ] API wrappers call backend commands
- [ ] State changes reflect in UI

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
