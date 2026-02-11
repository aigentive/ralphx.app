# Battle Mode V2: Cyber-Neon Warzone (Hard Replace)

## Summary

Build a full V2 replacement of Battle Mode focused on cinematic visuals, high-impact gameplay, and clear task-state readability, using PixiJS/WebGL at a desktop 60fps target.
V2 will hard-replace current Battle Mode (no V1 toggle), include adaptive soundtrack + SFX, and keep progression session-only (no leaderboard/meta backend).

---

## Product Goals and Success Criteria

1. Visual quality is "showpiece" level: layered atmosphere, high-end particles, bloom/glow, cinematic transitions, and polished HUD.
2. Active tasks are instantly identifiable via lock-on visuals, callout labels, and threat prioritization.
3. Gameplay is genuinely fun: escalation, mini-bosses, combo loop, and pressure-driven pacing tied to real task execution.
4. Performance is stable at 60fps on desktop-class hardware in normal project sizes.
5. Existing execution controls and graph workflows remain safe and unchanged functionally (game remains visual-only).

## Locked Decisions

| Decision | Choice |
|----------|--------|
| Renderer | PixiJS/WebGL |
| Dependencies | New major deps allowed |
| Performance target | Desktop 60fps |
| Art direction | Cyber-neon warzone |
| Gameplay scope | Full arcade escalation (not just visual reskin) |
| Audio | Adaptive soundtrack + SFX |
| Assets | Procedural + in-repo authored |
| Rollout | Hard replace current Battle Mode |
| Meta/progression backend | Out of scope for V2 |

---

## Architecture and Interfaces

### New Core Modules

| Module | Path | Responsibility |
|--------|------|----------------|
| **V2 Overlay** | `battle-v2/BattleModeV2Overlay.tsx` | Main container, Pixi stage host, HUD/controls bridge |
| **Engine** | `battle-v2/engine/BattleEngine.ts` | Fixed-timestep simulation, entity systems, wave orchestration |
| **Renderer Bridge** | `battle-v2/render/RendererBridge.ts` | Pixi app lifecycle, scene graph setup, post-processing chain |
| **Systems** | `battle-v2/systems/*` | `EntitySystem`, `CombatSystem`, `SpawnSystem`, `TaskSyncSystem`, `EffectsSystem`, `AudioSystem` |
| **Config** | `battle-v2/config/*` | Tunables for quality levels, spawn pacing, threat curves, FX budgets |

All paths relative to `src/components/TaskGraph/`.

### Public/Internal Type Additions

| Type | Shape |
|------|-------|
| `BattleTaskFeedEventV2` | `{ taskId, fromStatus, toStatus, timestamp, source }` |
| `BattleEntityV2` | Discriminated union: `drone` \| `elite` \| `cluster` \| `miniBoss` \| `hazard` |
| `BattleThreatProfile` | Derived from internal status and transition recency |
| `BattleQualityPreset` | `high` \| `balanced` \| `low` with deterministic effect budgets |
| `BattleAudioState` | Intensity level + mute + soundtrack segment |

### Existing Interface Changes

1. **`TaskGraphView`** swaps current overlay mount to V2 overlay component.
2. **`ExecutionControlBar`** keeps same toggle surface/UX copy ("Battle Mode"), but launches V2 pipeline.
3. **`useBattleModeTaskFeed`** evolves to V2 event contract (backward-compatible adapter retained until full migration complete).

---

## Data Flow (Decision Complete)

```
Backend/IPC events
  → Feed adapter (normalizes to BattleTaskFeedEventV2)
    → TaskSyncSystem (threat models, entity assignment)
      → SpawnSystem (lane pressure, wave events)
        → CombatSystem (bullets, abilities, combos, score)
          → EffectsSystem (GPU-friendly particles, screen FX)
            → HUD (score, combo, wave, active strip, ability charge, danger level)
```

**Entity assignment rules:**
- Active tasks → always individual
- Non-active overflow → can aggregate into visual formations
- Completed tasks → burst/score resolution effects

---

## Gameplay Spec (V2)

### Controls

| Action | Keys |
|--------|------|
| Move | `A`/`D` or arrows |
| Fire | `Space`/`J` |
| Focus Active | `F` |
| Ability | `Shift` (screen-line disruptor; cooldown based) |
| Pause | `P` |
| Exit | `Esc` |

### Escalation

- Wave director uses active task count, queue pressure, and threat weights.
- Mini-boss spawns at defined pressure thresholds (e.g., sustained review/merge congestion).

### Scoring

- Per-hit + per-elimination + status-resolution bonuses.
- Combo multiplier decays if no action window.

### Fail State

- Lives + overload meter; game over overlays with restart.

### Task Readability

- Lock-on rings for active tasks.
- Recent transition pulse effect.
- Short callout labels with status glyph.

---

## Visual Direction (Cyber-Neon Warzone)

| Layer | Description |
|-------|-------------|
| **Background** | Deep-space gradient, volumetric haze, parallax starfields, energy streaks |
| **Enemies** | Unique silhouettes, emissive edges, animated shaders |
| **FX stack** | Bloom glow, chromatic accents, shockwaves, directional hit sparks |
| **UI/HUD** | Glassy tactical overlay, animated danger arcs, high-contrast typography |
| **Camera feel** | Subtle reactive shake, impulse flashes on heavy collisions |

---

## Audio Plan

1. **Adaptive soundtrack layers:** ambient, tension, critical intensity.
2. **Event-driven SFX:** shot, hit, shield break, elimination, boss telegraph, ability discharge.
3. **Audio safety:** muted default remains configurable per user preference.
4. **Runtime controls:** master/music/sfx sliders in Battle HUD quick menu.

---

## Performance Plan

| Strategy | Detail |
|----------|--------|
| Simulation | Fixed tick + decoupled render loop |
| Particles | Pooling and capped emitter budgets by quality preset |
| Rendering | Batched sprites/meshes in Pixi; no per-frame allocations |
| Degradation | Auto-drop from high → balanced if frame time exceeds threshold for sustained window |
| Visibility | Pause heavy FX when window hidden |

---

## Migration and Rollout

1. Implement V2 in parallel path (`battle-v2/`) while preserving existing behavior temporarily.
2. Replace mount point in `TaskGraphView` once parity + quality gates pass.
3. Remove old V1 battle files after replacement stabilizes.
4. Retain one internal emergency kill-switch env/config for rapid disable.

---

## Test Cases and Scenarios

### Unit

1. Threat mapping by status and transition recency.
2. Spawn director output under varying execution pressure.
3. Combo and score math.
4. Ability cooldown/charge behavior.
5. Quality preset budget enforcement.

### Integration

1. Full task path simulation:
   `ready` → `executing` → `reviewing` → `revision_needed` → `re_executing` → `approved` → `merging` → `merged`
   — verify entity morphing + FX + scoring hooks.
2. High-load scenario (100+ tasks):
   active tasks remain individual and readable.
3. Boss trigger scenario:
   sustained merge/review pressure produces mini-boss event.

### UI/E2E

1. Toggle Battle Mode from graph execution bar.
2. Exit and re-enter while preserving graph panel behavior.
3. Audio toggles and quality toggles function without stage restart.
4. Focus Active temporarily dims non-active threats.

### Performance/Regression

1. 60fps target validation on reference desktop hardware profile.
2. No regressions in execution pause/stop controls.
3. No mutation of actual task state from gameplay actions.

---

## Implementation Phases

| Phase | Scope |
|-------|-------|
| **A** | Pixi foundation + renderer bridge + HUD shell |
| **B** | Task sync + entity model + readable active-task visuals |
| **C** | Combat loop + abilities + combo system |
| **D** | Cinematic FX + audio system + adaptive quality |
| **E** | Boss events + final balancing + migration/hard replacement |
| **F** | Full test suite + perf pass + cleanup old V1 code |

---

## Assumptions and Defaults

1. Desktop-first optimization is acceptable for V2.
2. No backend schema changes are required for this release.
3. Battle Mode remains visual-only regarding task workflow side effects.
4. Session-only score/progression is sufficient for V2 launch.
5. Any licensed third-party assets are out of scope; all assets are procedural or in-repo authored.
