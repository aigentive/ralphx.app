# Battle Mode for Graph Execution (Polished v1)

## Summary

- **Feasibility:** High. This fits your current React + Tauri architecture without backend schema changes.
- **Goal:** Add a playable Space-Invaders-style overlay in Graph view that reflects live task execution status, launched from a new Battle Mode button in the execution bar.
- **Scope locked:** Graph-only, visual-only (no execution control via gameplay), grouped status mapping, session-only score, audio muted by default, auto-hide right panel while active.

---

## Implementation Design

### 1. UX Entry/Exit and Layout

1. Add **Battle Mode** toggle button to `ExecutionControlBar`.
2. In Graph view, activating Battle Mode:
   - Mounts a game overlay over the graph canvas.
   - Auto-hides right panel (timeline/chat) and restores prior panel state on exit.
   - Keeps existing Pause/Stop controls visible and functional.
3. Exiting Battle Mode:
   - Tears down game loop cleanly.
   - Restores right panel mode and normal graph interactions.

### 2. Rendering and Engine

1. Use a dedicated **Canvas 2D** overlay component (`BattleModeOverlay`) for better runtime perf and simpler object updates.
2. Keep orchestration in React state/hooks, but run frame updates in a `requestAnimationFrame` game loop.
3. Engine modules:
   - **game-state** — entities, score, lives, wave.
   - **spawner** — task-status-driven invader creation/mutation.
   - **physics** — movement/collision/projectiles.
   - **renderer** — single canvas draw pass.
   - **input** — keyboard + pointer; fire/move/pause-game.

### 3. Status-to-Gameplay Mapping (Grouped)

| Group | Statuses | Behavior |
|-------|----------|----------|
| **Queue** | `backlog`, `ready`, `blocked` | Slow upper-row invaders |
| **Execution** | `executing`, `re_executing`, `qa_refining`, `qa_testing`, `qa_passed`, `qa_failed` | Medium-speed descending invaders |
| **Review** | `pending_review`, `reviewing`, `review_passed`, `escalated`, `revision_needed` | Faster zig-zag invaders |
| **Merge** | `pending_merge`, `merging`, `merge_incomplete`, `merge_conflict` | Armored invaders (2-hit) |
| **Complete** | `approved`, `merged` | Score burst + remove from battlefield |
| **Failure** | `failed`, `cancelled`, `stopped` | Hazard meteors (penalty if reaching player) |

Task identity is stable per invader (`taskId`) so transitions update existing enemies instead of re-spawning duplicates.

### 4. Event and Data Wiring

1. Consume existing frontend task/execution event flow:
   - `task:event` + legacy `task:status_changed` for per-task transitions.
   - `execution:status_changed` / `execution:queue_changed` for HUD counters and pacing modifiers.
2. Build a lightweight adapter hook (`useBattleModeTaskFeed`) that normalizes events into engine commands:
   - `spawn(taskId, statusGroup)`
   - `mutate(taskId, statusGroup)`
   - `despawn(taskId, reason)`
3. Seed initial invader state from current graph/task snapshot when mode starts.

### 5. State and Interface Changes (Public/Internal)

1. **uiStore additions:**
   - `battleModeActive: boolean`
   - `battleModePanelRestoreState: { userOpen: boolean; compactOpen: boolean } | null`
   - Actions: `enterBattleMode()`, `exitBattleMode()`
2. **ExecutionControlBar props:**
   - `battleModeActive: boolean`
   - `onBattleModeToggle: () => void`
3. **New internal types:**
   - `BattleEnemy`, `BattleStatusGroup`, `BattleEvent`, `BattleScoreState`
4. No Rust/backend API contract changes required in v1.

### 6. Performance and Safety Constraints

| Constraint | Target |
|------------|--------|
| Frame budget | 60fps ideal, degrade gracefully to 30fps under load |
| Entity cap | 250 active entities per frame with deterministic shedding |
| Background | Pause rendering when window/tab not visible |
| Safety | Visual-only guarantee — game interactions never invoke pause/resume/stop APIs |

---

## Test Plan

### Unit

1. Status-group mapper correctness for all internal statuses.
2. Event adapter dedupe and ordered transition handling.
3. Collision/score/life rules.
4. Store enter/exit behavior and panel state restoration.

### Component/Integration

1. `ExecutionControlBar` shows Battle Mode toggle and active state.
2. Graph view mounts overlay and auto-hides right panel on activation.
3. Live status updates mutate invaders without full reset.
4. Exit destroys RAF loop and restores panel + graph interactivity.

### E2E / Visual

1. Toggle Battle Mode in Graph view during active execution stream.
2. Simulate sequence `ready` → `executing` → `review` → `revision_needed` → `reviewing` → `approved` → `merging` → `merged` and verify enemy transformation chain.
3. Verify no regressions in existing graph controls and execution bar actions.

---

## Acceptance Criteria

1. Battle Mode launches from execution bar in Graph view with no layout breakage.
2. Live task status transitions visibly drive invader behavior in real time.
3. Existing execution controls remain unchanged and reliable.
4. Exiting Battle Mode returns UI to exact pre-game panel state.
5. No backend changes and no regressions in Graph/Kanban execution controls.

---

## Delivery Phases

| Phase | Scope |
|-------|-------|
| **A** | UI/store scaffolding + toggle + layout behavior |
| **B** | Canvas engine core + player/input/projectiles |
| **C** | Event adapter + status mapping + enemy lifecycle |
| **D** | Effects/HUD/audio toggle + polish |
| **E** | Tests + perf tuning + regression pass |

---

## Assumptions and Defaults

1. Feature name is **Battle Mode**.
2. Initial release is Graph view only.
3. Gameplay is visual-only (no execution control side effects).
4. Progress is session-only (no persisted leaderboard in v1).
5. Audio is muted by default with optional in-mode toggle.
6. Renderer default is Canvas 2D for stronger performance under event-heavy task graphs.
