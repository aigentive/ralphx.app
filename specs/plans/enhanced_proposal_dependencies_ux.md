# Plan: Enhanced Proposal Dependencies UX for Ideation Session

## Problem Statement

The current proposals display shows dependency information as simple count badges (`←3 →2`), which tells users *how many* dependencies exist but fails to communicate:
1. **Ordering** - Which proposals should be executed first?
2. **Relationships** - Which *specific* proposals block which others?
3. **Plan flow** - How do all the pieces fit together as a coherent implementation plan?

## Current Implementation

**Location:** `src/components/Ideation/IdeationView.tsx`, `ProposalCard.tsx`

**What exists:**
- Proposals sorted by `sortOrder` (drag-to-reorder), NOT by dependency order
- ProposalCard shows `←N` (depends on count) and `→N` (blocks count) as tiny badges
- Critical path highlighted with orange bottom border
- Full graph visualization exists in `DependencyVisualization.tsx` but only used in ApplyModal
- Phase 39 adding `reason` field to explain WHY dependencies exist

## Design Approach: Tiered View with Inline Details

### Core Concept: Topological Layers

Reorganize proposals into **execution tiers** based on dependency depth:

```
┌─ Tier 0: Foundation ─────────────────────────────────────┐
│ No dependencies - can start immediately                   │
│ ┌──────────────────┐  ┌──────────────────┐               │
│ │ Setup Database   │  │ Define Types     │               │
│ └──────────────────┘  └──────────────────┘               │
└──────────────────────────────────────────────────────────┘

┌─ Tier 1: Core ───────────────────────────────────────────┐
│ Depends only on Tier 0 items                              │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ Implement Service                                     │ │
│ │ ← Setup Database, Define Types                        │ │
│ └──────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘

┌─ Tier 2: Integration ────────────────────────────────────┐
│ Depends on Tier 1 items                                   │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ Build UI Component                                    │ │
│ │ ← Implement Service  "Needs service API for data"     │ │
│ └──────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

### Key UX Improvements

1. **Visual Hierarchy via Tiers**
   - Group proposals by topological level (depth from root)
   - Tier headers with subtle styling to create visual rhythm
   - Collapsible tiers for large plans

2. **Inline Dependency Names (not counts)**
   - Replace `←3` with `← API Design, Type Defs, Config Setup`
   - Show actual proposal titles, truncated if needed
   - On hover/expand: full titles + reason text

3. **Dependency Flow Indicators**
   - Small connector lines between tiers (not between individual items)
   - Critical path items highlighted across tiers
   - Clear "flows to" visual direction (top → bottom)

4. **Expandable Dependency Details**
   - Click proposal to expand dependency section
   - Shows: full dependency list with titles + reason text (Phase 39)
   - Shows: what depends on this (reverse direction)

### Auto-Collapse for Large Plans

Tiers with 5+ proposals auto-collapse by default:
- Shows tier header with proposal count: "Tier 1: Core (7 proposals)"
- Click to expand
- Keeps the view manageable without hiding small tiers

### Prerequisite: Phase 39

This implementation should happen **after Phase 39 completes** so dependency reasons are available from the start. The reason text explains WHY dependencies exist, which is crucial for the inline dependency display.

## Implementation Plan

### Task 1: Compute Topological Tiers (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(hooks): add useDependencyTiers hook for topological grouping`
**File:** `src/hooks/useDependencyGraph.ts`

Add `useDependencyTiers()` hook that:
- Takes `DependencyGraph` as input
- Returns `Map<proposalId, tierLevel>`
- Tier 0 = no dependencies (inDegree === 0)
- Tier N = max(tier of dependencies) + 1
- Handle cycles gracefully (assign to highest possible tier)

### Task 2: Update DependencyGraph Types for Reasons (BLOCKING)
**Dependencies:** None (Phase 39 prerequisite)
**Atomic Commit:** `feat(types): add reason field to DependencyGraphEdge`
**Files:** `src/types/ideation.ts`, `src/api/ideation.schemas.ts`, `src/api/ideation.transforms.ts`

This is Phase 39 work (in progress). Ensure:
- `DependencyGraphEdge` includes `reason?: string`
- Edge data flows from backend through transforms
- Hook provides `getDependencyReason(fromId, toId)` helper

### Task 3: Create ProposalTierGroup Component (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(ideation): create ProposalTierGroup collapsible tier component`
**File:** `src/components/Ideation/ProposalTierGroup.tsx` (new)

- Renders a collapsible tier section
- Header: "Tier {N}: {label}" with expand/collapse
- Labels: "Foundation" (0), "Core" (1), "Integration" (2+), etc.
- Subtle visual styling (border-left accent, indentation)

### Task 4: Enhance ProposalCard Dependency Display (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `feat(ideation): display dependency names and reasons in ProposalCard`
**File:** `src/components/Ideation/ProposalCard.tsx`

Replace count badges with inline dependency names:
- New prop: `dependsOnProposals?: Array<{ id: string; title: string; reason?: string }>`
- Display: `← {title1}, {title2}` truncated with "+N more"
- Tooltip shows full list
- Expandable section shows full details + reasons

### Task 5: Create TieredProposalList Component (BLOCKING)
**Dependencies:** Task 1, Task 3, Task 4
**Atomic Commit:** `feat(ideation): create TieredProposalList orchestration component`
**File:** `src/components/Ideation/TieredProposalList.tsx` (new)

- Groups proposals by tier using `useDependencyTiers()`
- Renders ProposalTierGroup for each tier
- Passes dependency details to each ProposalCard
- Maintains existing selection/highlight functionality

### Task 6: Replace List in IdeationView (BLOCKING)
**Dependencies:** Task 5
**Atomic Commit:** `feat(ideation): integrate TieredProposalList into IdeationView`
**File:** `src/components/Ideation/IdeationView.tsx`

- Replace `sortedProposals.map()` with `<TieredProposalList />`
- Pass all existing props (selection, highlights, handlers)
- Remove sortOrder-based sorting (tiers define order now)
- Keep drag-to-reorder WITHIN tiers (maintain sortOrder as tiebreaker)

### Task 7: Style Tier Connectors
**Dependencies:** Task 6
**Atomic Commit:** `feat(ideation): add SVG tier connectors with critical path highlight`
**File:** `src/components/Ideation/TieredProposalList.tsx`

- Add subtle SVG connectors between tiers
- Critical path highlight flows through connectors
- Keep styling minimal (dashed lines, accent color for critical)

## Files to Modify

| File | Change |
|------|--------|
| `src/hooks/useDependencyGraph.ts` | Add `useDependencyTiers()` hook |
| `src/types/ideation.ts` | Edge reason type (Phase 39 provides) |
| `src/components/Ideation/ProposalCard.tsx` | Inline dependency display with names + reasons |
| `src/components/Ideation/IdeationView.tsx` | Replace flat list with TieredProposalList |
| `src/components/Ideation/ProposalTierGroup.tsx` | New: collapsible tier section |
| `src/components/Ideation/TieredProposalList.tsx` | New: orchestrates tiered layout |

## Sequencing

1. **Phase 39 completes first** (adds reason field to dependencies)
2. Tasks 1-2: Hooks and types (backend data ready)
3. Tasks 3-5: New components (ProposalTierGroup, TieredProposalList, ProposalCard updates)
4. Tasks 6-7: Integration (replace in IdeationView, add connectors)

## Testing Strategy

1. **Unit Tests**
   - `useDependencyTiers`: correct tier assignment, cycle handling
   - ProposalCard: renders dependency names correctly

2. **Integration Tests**
   - TieredProposalList: groups correctly, handles empty tiers
   - View toggle: switches between modes, preserves selection

3. **Manual Verification**
   - Create ideation session with complex dependencies
   - Verify tier grouping makes logical sense
   - Check critical path visibility across tiers
   - Confirm Phase 39 reasons display when available

## Design System Alignment

- Accent color: `#ff6b35` for critical path, interactive elements
- Font: SF Pro (system)
- Tier headers: subtle border-left accent, muted text
- Connectors: dashed `var(--border-subtle)`, solid `#ff6b35` for critical

## Design Decisions (Resolved)

| Decision | Answer |
|----------|--------|
| Replace or toggle? | **Replace** - tiered view becomes the only view |
| Collapse behavior | **Auto by count** - collapse tiers with 5+ proposals |
| Phase 39 dependency | **Wait** - build after Phase 39 completes |

## Remaining Consideration

**Drag-to-reorder within tiers:** Keep sortOrder as a tiebreaker within the same tier. This preserves user control while respecting dependency order across tiers.

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
