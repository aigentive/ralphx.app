# Battle Mode V1.1: Active-Task Clarity + Visual Overhaul

## Summary

Upgrade the existing Battle Mode so users can immediately identify which tasks are actively being worked, while significantly improving visual quality and game feel.
No backend contract changes are required; all improvements use existing frontend task/execution events and graph task data.

---

## Scope and Success Criteria

1. Active tasks are unmistakable within 500ms of status changes.
2. Graphics move from basic rectangles to a cohesive neon arcade style (parallax, glow, impact effects, readable enemy identity).
3. Large task sets remain readable and smooth via adaptive aggregation.
4. No regressions to existing graph controls, execution controls, or panel restore behavior.

## Product Decisions (Locked)

| Decision | Choice |
|----------|--------|
| Active clarity | Tracked lock-ons |
| Visual style | Neon vector arcade |
| Density strategy | Adaptive aggregation |
| Input/controls | Visual-only (no execution side effects) |
| Audio | Muted by default with user toggle |

---

## Important Interface and Type Changes

1. **`BattleModeOverlay` props additions:**
   - `projectId: string`
   - `activeTaskIds?: string[]` (optional optimization path if precomputed by parent)
2. **New internal engine types:**
   - `BattleEntity` (`individual` | `cluster`)
   - `BattleThreatLevel` (`idle` | `active` | `critical`)
   - `BattleRecentTransition` (`taskId`, `from`, `to`, `ts`)
3. **`useBattleModeTaskFeed` output expansion:**
   - Emit normalized events with timestamp and optional `fromStatus` when available.
4. **`statusMapping.ts` additions:**
   - `isActivelyWorkedStatus(status)` for explicit active classification.
   - `getThreatWeight(status)` for aggregation and spawn ordering.

---

## Implementation Plan

### 1. Active-Task Signal Layer

1. Define active statuses as: `executing`, `re_executing`, `qa_refining`, `qa_testing`, `pending_review`, `reviewing`, `merging`.
2. Track `recentlyActive` window (default 12s) for tasks just transitioned out of active states.
3. Render active entities with:
   - Pulsing lock-on ring.
   - Directional exhaust/trail.
   - Bright edge glow + subtle scanline sweep.
   - Always-visible short label (`taskId` short + truncated title).
4. Add top HUD **"Active Now"** strip listing up to N active tasks with status badges.

### 2. Visual Upgrade (Neon Arcade)

1. Replace flat background with layered render passes:
   - Starfield base.
   - Slow parallax nebula gradient.
   - Foreground dust streaks.
2. Enemy rendering:
   - Vector silhouette variants by group (queue, execution, review, merge, failure).
   - Multi-hit armor visuals for merge group.
3. Combat FX:
   - Impact sparks, shock-ring, hit flash, score popup.
   - Distinct elimination burst for approved/merged.
4. Player ship upgrade:
   - Glow core, engine plume, recoil flash on firing.

### 3. Adaptive Aggregation for High Task Counts

1. Add threshold-based clustering (example defaults):

   | Task Count | Rendering Strategy |
   |------------|--------------------|
   | <= 40 | Render all individuals |
   | > 40 | Cluster non-active tasks by status group + lane |

2. Cluster behavior:
   - Display as formation with count badge.
   - Split cluster into individuals when member becomes active or recently active.
3. Preserve active-task priority:
   - Active/recent tasks are **never** hidden inside clusters.
4. Keep deterministic mapping so the same task appears in stable lane/region.

### 4. Event and Data Pipeline Refinement

1. Extend task feed normalization:
   - Prefer `task:event.status_changed` with `from`/`to`.
   - Fallback to legacy `task:status_changed`.
2. Maintain per-task last transition metadata in overlay state.
3. On transition:
   - Update threat class immediately.
   - Trigger transient spotlight animation (1.2s).
   - Update HUD roster and cluster membership atomically.

### 5. Readability and UX Polishing

1. Add mini legend in overlay: **Active**, **Recent**, **Idle**, **Cluster**.
2. Add **"Focus Active"** hotkey (`F`) to dim non-active entities for 3 seconds.
3. Improve HUD typography/spacing for fast scan.
4. Add quality toggle:

   | Level | Description |
   |-------|-------------|
   | **High** | Full particles/parallax |
   | **Balanced** (default) | Moderate particles |
   | **Low** | Simplified effects for weaker devices |

### 6. Performance and Stability

1. Render loop remains Canvas 2D with capped particle pools.
2. Avoid per-frame allocations in hot paths.
3. Throttle expensive text rendering via glyph cache/pre-measured labels.
4. Pause heavy effects when tab is hidden.
5. Maintain 60fps target, graceful degrade to stable 30fps.

---

## File-Level Work Plan

| File | Changes |
|------|---------|
| `src/components/TaskGraph/battle/BattleModeOverlay.tsx` | Refactor renderer into layered passes. Add active lock-on visuals, labels, cluster rendering, quality modes. |
| `src/components/TaskGraph/battle/useBattleModeTaskFeed.ts` | Enrich normalized payload (`from`/`to`/`timestamp`/`source`). |
| `src/components/TaskGraph/battle/statusMapping.ts` | Add active classification + threat weight helpers. |
| `src/components/TaskGraph/TaskGraphView.tsx` | Pass `projectId` and optional active-task hints if needed. |
| **New:** `battle/entities.ts` | Entity + cluster transforms. |
| **New:** `battle/effects.ts` | Particles/FX pools. |
| **New:** `battle/labels.ts` | Truncation/cache. |
| **New:** `battle/constants.ts` | Quality thresholds. |

---

## Tests and Scenarios

### Unit

1. Active-status classifier correctness.
2. Aggregation logic keeps active/recent tasks unclustered.
3. Transition reducer updates entity class and HUD atomically.
4. Threat weighting and lane assignment are deterministic.

### Component

1. Active tasks display lock-on + label within one render cycle after event.
2. Focus Active dims non-active entities and auto-restores.
3. Quality mode toggles change effect density without crashing.

### Integration

1. Transition chain: `ready` → `executing` → `reviewing` → `revision_needed` → `re_executing` → `approved` → `merging` → `merged`.
2. Verify visual cues change at each step and final completion burst appears.
3. Large dataset scenario (100+ tasks): clusters appear, active tasks remain individually visible.

### Regression

1. Graph panel auto-hide/restore behavior remains intact.
2. Execution control buttons still function identically.
3. Exiting Battle Mode restores previous graph interaction state.

---

## Rollout

1. Ship behind existing Battle Mode path with internal quality default **Balanced**.
2. Add lightweight telemetry counters (fps bucket, cluster count, active count) via existing logger only.
3. If regressions appear, fallback is disabling clusters while keeping active lock-ons.

---

## Assumptions and Defaults

1. No backend API/event schema changes are introduced.
2. Canvas 2D remains rendering backend.
3. Audio default stays off.
4. Active-task statuses are defined by current frontend status taxonomy.
5. Upgrade targets desktop first (Tauri + web dev mode), with responsive containment for smaller viewports.
