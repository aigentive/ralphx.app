import type { InternalStatus } from "@/types/status";
import { BATTLE_CLUSTER_THRESHOLD, BATTLE_RECENT_ACTIVE_MS } from "./constants";
import { getBattleSpecForStatus, isActivelyWorkedStatus, type BattleStatusGroup } from "./statusMapping";

export interface BattleTaskState {
  taskId: string;
  title: string;
  status: InternalStatus;
  lastTransitionAt: number;
}

interface BaseEntity {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  speed: number;
  drift: number;
  phase: number;
  color: string;
  group: Exclude<BattleStatusGroup, "complete">;
  active: boolean;
  recent: boolean;
  kind: "individual" | "cluster";
}

export interface BattleIndividualEntity extends BaseEntity {
  kind: "individual";
  taskId: string;
  title: string;
  hp: number;
  baseHp: number;
}

export interface BattleClusterEntity extends BaseEntity {
  kind: "cluster";
  taskIds: string[];
  count: number;
  hp: number;
}

export type BattleEntity = BattleIndividualEntity | BattleClusterEntity;

function hashString(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

function shortTitle(title: string): string {
  if (title.length <= 20) return title;
  return `${title.slice(0, 19)}...`;
}

function shortTaskLabel(taskId: string, title: string): string {
  const compactId = taskId.length > 8 ? taskId.slice(0, 8) : taskId;
  return `${compactId} ${shortTitle(title)}`;
}

function laneY(group: Exclude<BattleStatusGroup, "complete">): number {
  switch (group) {
    case "queue":
      return 20;
    case "execution":
      return 44;
    case "review":
      return 68;
    case "merge":
      return 94;
    case "failure":
      return 118;
  }
}

function createIndividual(task: BattleTaskState, width: number, now: number): BattleIndividualEntity | null {
  const spec = getBattleSpecForStatus(task.status);
  if (!spec) return null;
  if (spec.group === "complete") return null;

  const seed = hashString(task.taskId);
  const span = Math.max(240, width - 120);
  const x = 40 + (seed % span);
  const active = isActivelyWorkedStatus(task.status);
  const recent = !active && now - task.lastTransitionAt < BATTLE_RECENT_ACTIVE_MS;

  return {
    id: `task:${task.taskId}`,
    kind: "individual",
    taskId: task.taskId,
    title: shortTaskLabel(task.taskId, task.title),
    x,
    y: laneY(spec.group) + (seed % 16),
    w: 30,
    h: 22,
    hp: spec.hp,
    baseHp: spec.hp,
    speed: spec.speed,
    drift: 8 + (seed % 10),
    phase: (seed % 200) / 100,
    color: spec.color,
    group: spec.group,
    active,
    recent,
  };
}

function createCluster(group: Exclude<BattleStatusGroup, "complete">, tasks: BattleTaskState[], width: number): BattleClusterEntity | null {
  if (tasks.length === 0) return null;
  const first = tasks[0];
  if (!first) return null;

  const spec = getBattleSpecForStatus(first.status);
  if (!spec) return null;
  if (spec.group === "complete") return null;

  const seed = hashString(`${group}:${tasks.length}`);
  const span = Math.max(180, width - 180);

  return {
    id: `cluster:${group}`,
    kind: "cluster",
    taskIds: tasks.map((task) => task.taskId),
    count: tasks.length,
    hp: Math.max(2, Math.min(9, Math.ceil(tasks.length / 8))),
    x: 70 + (seed % span),
    y: laneY(group),
    w: 42,
    h: 30,
    speed: Math.max(16, spec.speed - 6),
    drift: 5 + (seed % 6),
    phase: (seed % 300) / 120,
    color: spec.color,
    group,
    active: false,
    recent: false,
  };
}

export function buildBattleEntities(taskStates: Map<string, BattleTaskState>, width: number, now: number): Map<string, BattleEntity> {
  const allIndividuals = Array.from(taskStates.values())
    .map((task) => createIndividual(task, width, now))
    .filter((entity): entity is BattleIndividualEntity => entity !== null);

  const activeOrRecent = allIndividuals.filter((entity) => entity.active || entity.recent);

  if (allIndividuals.length <= BATTLE_CLUSTER_THRESHOLD) {
    return new Map(allIndividuals.map((entity) => [entity.id, entity]));
  }

  const map = new Map<string, BattleEntity>();
  for (const entity of activeOrRecent) {
    map.set(entity.id, entity);
  }

  const grouped = new Map<Exclude<BattleStatusGroup, "complete">, BattleTaskState[]>();
  for (const state of taskStates.values()) {
    const spec = getBattleSpecForStatus(state.status);
    if (!spec) continue;
    if (spec.group === "complete") continue;

    const key = spec.group;
    const shouldSkip = activeOrRecent.some((entity) => entity.taskId === state.taskId);
    if (shouldSkip) continue;

    const existing = grouped.get(key);
    if (existing) {
      existing.push(state);
    } else {
      grouped.set(key, [state]);
    }
  }

  for (const [group, tasks] of grouped) {
    const cluster = createCluster(group, tasks, width);
    if (cluster) {
      map.set(cluster.id, cluster);
    }
  }

  return map;
}
