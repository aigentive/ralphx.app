import type { TaskGraphNode } from "@/api/task-graph.types";
import type { InternalStatus } from "@/types/status";
import {
  BATTLE_V2_ACTIVE_WINDOW_MS,
  BATTLE_V2_BOSS_MIN_INTERVAL_MS,
  BATTLE_V2_BULLET_SPEED,
  BATTLE_V2_ENEMY_BULLET_SPEED,
  BATTLE_V2_SHOT_COOLDOWN,
} from "./config";
import type {
  BattleEngineState,
  BattleEntity,
  BattleTaskState,
  BattleTaskSyncEvent,
} from "./types";
import { getBattleSpecForStatus, getThreatWeight, isActivelyWorkedStatus, mapStatusToBattleGroup } from "../battle/statusMapping";

const PLAYER_WIDTH = 34;
const PLAYER_HEIGHT = 20;

function statusFromRaw(raw: string): InternalStatus {
  return raw as InternalStatus;
}

function shortLabel(taskId: string, title: string): string {
  const sid = taskId.slice(0, 8);
  const t = title.length > 16 ? `${title.slice(0, 16)}...` : title;
  return `${sid} ${t}`;
}

function laneY(group: string): number {
  switch (group) {
    case "queue":
      return -132;
    case "execution":
      return -164;
    case "review":
      return -196;
    case "merge":
      return -228;
    default:
      return -108;
  }
}

function buildTaskMap(tasks: TaskGraphNode[], now: number): Map<string, BattleTaskState> {
  const map = new Map<string, BattleTaskState>();
  for (const task of tasks) {
    map.set(task.taskId, {
      taskId: task.taskId,
      title: task.title,
      status: statusFromRaw(task.internalStatus),
      lastTransitionAt: now,
      suppressedUntil: 0,
      breachCount: 0,
    });
  }
  return map;
}

function hash(value: string): number {
  let h = 2166136261;
  for (let i = 0; i < value.length; i += 1) {
    h ^= value.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h);
}

export function buildEntities(tasks: Map<string, BattleTaskState>, width: number, now: number): Map<string, BattleEntity> {
  const entities = new Map<string, BattleEntity>();
  const total = tasks.size;

  for (const task of tasks.values()) {
    const spec = getBattleSpecForStatus(task.status);
    if (!spec || spec.group === "complete") {
      continue;
    }

    const seed = hash(task.taskId);
    const active = isActivelyWorkedStatus(task.status);
    const recent = !active && now - task.lastTransitionAt < BATTLE_V2_ACTIVE_WINDOW_MS;
    if (task.suppressedUntil > now) {
      continue;
    }

    const shouldCluster = total > 60 && !active && !recent;

    if (shouldCluster) {
      const clusterId = `cluster:${spec.group}`;
      const existing = entities.get(clusterId);
      if (existing) {
        existing.taskIds.push(task.taskId);
        existing.hp += 0.35;
        existing.maxHp += 0.35;
        existing.label = `${existing.taskIds.length} tasks`;
      } else {
        const x = 60 + (seed % Math.max(180, width - 220));
        entities.set(clusterId, {
          id: clusterId,
          kind: "cluster",
          x,
          y: laneY(spec.group) + (seed % 34),
          w: 56,
          h: 34,
          speed: Math.max(16, spec.speed - 6),
          drift: 6,
          phase: (seed % 120) / 30,
          hp: 4,
          maxHp: 4,
          color: spec.color,
          active: false,
          recent: false,
          threatWeight: getThreatWeight(task.status),
          taskIds: [task.taskId],
          label: "1 task",
          seed,
        });
      }
      continue;
    }

    const x = 40 + (seed % Math.max(210, width - 120));
    const kind = spec.group === "failure" ? "hazard" : active ? "elite" : "drone";
    entities.set(`task:${task.taskId}`, {
      id: `task:${task.taskId}`,
      kind,
      x,
      y: laneY(spec.group) + (seed % 12),
      w: active ? 32 : 28,
      h: active ? 24 : 20,
      speed: spec.speed + (active ? 8 : 0),
      drift: 8 + (seed % 10),
      phase: (seed % 200) / 100,
      hp: spec.hp,
      maxHp: spec.hp,
      color: spec.color,
      active,
      recent,
      threatWeight: getThreatWeight(task.status),
      taskIds: [task.taskId],
      label: shortLabel(task.taskId, task.title),
      seed,
    });
  }

  return entities;
}

function reconcileEntities(previous: Map<string, BattleEntity>, next: Map<string, BattleEntity>): Map<string, BattleEntity> {
  const map = new Map<string, BattleEntity>();
  for (const [id, entity] of next) {
    const prev = previous.get(id);
    if (!prev) {
      map.set(id, entity);
      continue;
    }
    map.set(id, {
      ...entity,
      x: prev.x,
      y: prev.y,
      phase: prev.phase,
      hp: Math.min(prev.hp, entity.maxHp),
    });
  }
  return map;
}

export function createEngineState(tasks: TaskGraphNode[], width: number): BattleEngineState {
  const now = Date.now();
  const taskMap = buildTaskMap(tasks, now);
  return {
    score: 0,
    combo: 0,
    comboDecayAt: 0,
    lives: 3,
    overload: 0,
    wave: 1,
    paused: false,
    gameOver: false,
    playerX: Math.max(20, width / 2 - PLAYER_WIDTH / 2),
    abilityCharge: 0,
    entities: buildEntities(taskMap, width, now),
    tasks: taskMap,
    bullets: [],
    sparks: [],
    focusActiveUntil: 0,
    pulseUntil: 0,
    bossCooldownUntil: now + BATTLE_V2_BOSS_MIN_INTERVAL_MS,
    lastDirectorTick: now,
    shotCooldown: 0,
  };
}

export function applyTaskSyncEvent(state: BattleEngineState, event: BattleTaskSyncEvent, width: number): void {
  const existing = state.tasks.get(event.taskId);
  if (existing) {
    existing.status = event.toStatus;
    existing.lastTransitionAt = event.timestamp;
    existing.suppressedUntil = 0;
  }
  state.pulseUntil = Date.now() + 900;

  if (mapStatusToBattleGroup(event.toStatus) === "complete") {
    state.score += 140;
    state.combo += 1;
    state.comboDecayAt = Date.now() + 3000;
    state.tasks.delete(event.taskId);
  }

  state.entities = reconcileEntities(state.entities, buildEntities(state.tasks, width, Date.now()));
}

export function syncTasksSnapshot(state: BattleEngineState, tasks: TaskGraphNode[], width: number): void {
  const now = Date.now();
  const next = buildTaskMap(tasks, now);
  for (const [taskId, task] of state.tasks) {
    const incoming = next.get(taskId);
    if (incoming) {
      incoming.lastTransitionAt = task.lastTransitionAt;
      incoming.suppressedUntil = task.suppressedUntil;
      incoming.breachCount = task.breachCount;
    }
  }
  state.tasks = next;
  state.entities = reconcileEntities(state.entities, buildEntities(state.tasks, width, now));
}

function spawnMiniBoss(state: BattleEngineState, width: number): void {
  const id = "boss:review-storm";
  if (state.entities.has(id)) return;
  state.entities.set(id, {
    id,
    kind: "miniBoss",
    x: width * 0.5 - 56,
    y: 14,
    w: 112,
    h: 54,
    speed: 24,
    drift: 22,
    phase: 0,
    hp: 24,
    maxHp: 24,
    color: "#ff5fd2",
    active: true,
    recent: true,
    threatWeight: 8,
    taskIds: [],
    label: "Review Storm",
    seed: hash(id),
  });
}

function spawnBossVolley(state: BattleEngineState, entity: BattleEntity, width: number): void {
  const centerX = entity.x + entity.w / 2;
  const baseY = entity.y + entity.h;
  const phaseRatio = entity.hp / entity.maxHp;

  // Phase 1: narrow triple volley
  const spread = phaseRatio > 0.66 ? [-16, 0, 16] : phaseRatio > 0.33 ? [-28, -10, 10, 28] : [-40, -20, 0, 20, 40];
  for (const offset of spread) {
    state.bullets.push({
      x: centerX + offset,
      y: baseY,
      vx: offset * 0.3,
      vy: BATTLE_V2_ENEMY_BULLET_SPEED + (1 - phaseRatio) * 90,
      damage: phaseRatio > 0.33 ? 1 : 2,
      color: phaseRatio > 0.33 ? "#fb7185" : "#f43f5e",
      fromEnemy: true,
    });
  }

  // Phase 3: arena edge punish beams
  if (phaseRatio <= 0.33) {
    state.bullets.push({
      x: 18,
      y: baseY + 8,
      vx: 32,
      vy: BATTLE_V2_ENEMY_BULLET_SPEED + 40,
      damage: 1,
      color: "#fda4af",
      fromEnemy: true,
    });
    state.bullets.push({
      x: Math.max(28, width - 18),
      y: baseY + 8,
      vx: -32,
      vy: BATTLE_V2_ENEMY_BULLET_SPEED + 40,
      damage: 1,
      color: "#fda4af",
      fromEnemy: true,
    });
  }
}

function director(state: BattleEngineState, width: number, now: number, runningCount: number, queuedCount: number): void {
  if (now - state.lastDirectorTick < 1500) return;
  state.lastDirectorTick = now;

  state.entities = reconcileEntities(state.entities, buildEntities(state.tasks, width, now));

  const pressure = runningCount * 2 + queuedCount;
  if (pressure > 10 && now > state.bossCooldownUntil) {
    spawnMiniBoss(state, width);
    state.bossCooldownUntil = now + BATTLE_V2_BOSS_MIN_INTERVAL_MS;
    state.pulseUntil = now + 1200;
  }
}

export function stepState(
  state: BattleEngineState,
  dt: number,
  now: number,
  width: number,
  height: number,
  input: { left: boolean; right: boolean; firing: boolean; ability: boolean },
  runningCount: number,
  queuedCount: number
): void {
  if (state.paused || state.gameOver) return;

  const moveX = (input.right ? 1 : 0) - (input.left ? 1 : 0);
  state.playerX += moveX * 470 * dt;
  state.playerX = Math.max(8, Math.min(width - PLAYER_WIDTH - 8, state.playerX));

  state.shotCooldown = Math.max(0, state.shotCooldown - dt);
  if (input.firing && state.shotCooldown <= 0) {
    state.bullets.push({
      x: state.playerX + PLAYER_WIDTH / 2,
      y: height - 54,
      vx: 0,
      vy: -BATTLE_V2_BULLET_SPEED,
      damage: 1,
      color: "#e2e8f0",
      fromEnemy: false,
    });
    state.shotCooldown = BATTLE_V2_SHOT_COOLDOWN;
  }

  state.abilityCharge = Math.min(100, state.abilityCharge + dt * 6);
  if (input.ability && state.abilityCharge >= 100) {
    state.abilityCharge = 0;
    state.focusActiveUntil = now + 2500;
    for (const entity of state.entities.values()) {
      entity.hp -= entity.kind === "miniBoss" ? 4 : 2;
    }
  }

  const descentMultiplier = 1 + Math.min(2.2, runningCount * 0.17 + queuedCount * 0.03);

  for (const entity of state.entities.values()) {
    entity.phase += dt * 1.6;
    entity.x += Math.sin(entity.phase) * entity.drift * dt;
    entity.y += entity.speed * descentMultiplier * dt;

    if (entity.x < 6) entity.x = 6;
    if (entity.x + entity.w > width - 6) entity.x = width - entity.w - 6;

    if (entity.kind === "miniBoss") {
      const phaseRatio = entity.hp / entity.maxHp;
      const fireChance = phaseRatio > 0.66 ? 0.016 : phaseRatio > 0.33 ? 0.026 : 0.038;
      if (Math.random() < fireChance) {
        spawnBossVolley(state, entity, width);
      }
      // Slightly stronger drift as boss enrages
      entity.drift = phaseRatio > 0.66 ? 22 : phaseRatio > 0.33 ? 27 : 34;
    }

    if (entity.y + entity.h >= height - 52) {
      state.entities.delete(entity.id);
      for (const taskId of entity.taskIds) {
        const task = state.tasks.get(taskId);
        if (task) {
          task.breachCount += 1;
          task.suppressedUntil = now + Math.min(11_000, 4_200 + task.breachCount * 850);
        }
      }
      state.lives -= entity.kind === "miniBoss" ? 2 : 1;
      state.overload = Math.min(100, state.overload + 14);
    }
  }

  for (const bullet of state.bullets) {
    bullet.x += bullet.vx * dt;
    bullet.y += bullet.vy * dt;
  }
  state.bullets = state.bullets.filter(
    (bullet) => bullet.y > -30 && bullet.y < height + 30 && bullet.x > -30 && bullet.x < width + 30
  );

  const spent = new Set<number>();
  for (let i = 0; i < state.bullets.length; i += 1) {
    const bullet = state.bullets[i];
    if (!bullet) continue;

    if (bullet.fromEnemy) {
      const py = height - 46;
      const hitPlayer =
        bullet.x >= state.playerX &&
        bullet.x <= state.playerX + PLAYER_WIDTH &&
        bullet.y >= py &&
        bullet.y <= py + PLAYER_HEIGHT;
      if (hitPlayer) {
        spent.add(i);
        state.lives -= bullet.damage;
        state.overload = Math.min(100, state.overload + 8);
      }
      continue;
    }

    for (const entity of state.entities.values()) {
      const hit =
        bullet.x >= entity.x &&
        bullet.x <= entity.x + entity.w &&
        bullet.y >= entity.y &&
        bullet.y <= entity.y + entity.h;

      if (!hit) continue;
      spent.add(i);
      entity.hp -= bullet.damage;
      state.score += entity.kind === "miniBoss" ? 30 : 20;
      state.combo += 1;
      state.comboDecayAt = now + 2600;
      state.sparks.push({
        x: bullet.x,
        y: bullet.y,
        vx: (Math.random() - 0.5) * 120,
        vy: (Math.random() - 0.5) * 120,
        life: 1,
        color: entity.color,
      });

      if (entity.hp <= 0) {
        state.entities.delete(entity.id);
        for (const taskId of entity.taskIds) {
          const task = state.tasks.get(taskId);
          if (task) {
            task.suppressedUntil = now + 2_600;
          }
        }
        state.score += entity.kind === "miniBoss" ? 800 : entity.kind === "cluster" ? 220 : 80;
      }
      break;
    }
  }

  if (spent.size > 0) {
    state.bullets = state.bullets.filter((_, index) => !spent.has(index));
  }

  for (const spark of state.sparks) {
    spark.x += spark.vx * dt;
    spark.y += spark.vy * dt;
    spark.life -= dt * 2.2;
  }
  state.sparks = state.sparks.filter((spark) => spark.life > 0);

  if (state.combo > 0 && now > state.comboDecayAt) {
    state.combo = Math.max(0, state.combo - 1);
    state.comboDecayAt = now + 1200;
  }

  if (state.entities.size === 0 && state.tasks.size > 0) {
    state.wave += 1;
    state.entities = buildEntities(state.tasks, width, now);
  }

  director(state, width, now, runningCount, queuedCount);

  if (state.sparks.length > 400) {
    state.sparks = state.sparks.slice(state.sparks.length - 400);
  }

  if (state.overload > 0) {
    state.overload = Math.max(0, state.overload - dt * 4);
  }

  if (state.lives <= 0 || state.overload >= 100) {
    state.gameOver = true;
  }
}

export function getActiveRoster(state: BattleEngineState, limit: number): Array<{ taskId: string; title: string; status: InternalStatus }> {
  return Array.from(state.tasks.values())
    .filter((task) => isActivelyWorkedStatus(task.status))
    .sort((a, b) => getThreatWeight(b.status) - getThreatWeight(a.status))
    .slice(0, limit)
    .map((task) => ({
      taskId: task.taskId,
      title: task.title.length > 22 ? `${task.title.slice(0, 22)}...` : task.title,
      status: task.status,
    }));
}
