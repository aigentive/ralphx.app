import type { InternalStatus } from "@/types/status";

export type EnemyKind = "drone" | "elite" | "cluster" | "miniBoss" | "hazard";

export interface BattleTaskSyncEvent {
  taskId: string;
  fromStatus: InternalStatus | null;
  toStatus: InternalStatus;
  timestamp: number;
  source: "task:event" | "task:status_changed";
}

export interface BattleTaskState {
  taskId: string;
  title: string;
  status: InternalStatus;
  lastTransitionAt: number;
  suppressedUntil: number;
  breachCount: number;
}

export interface BattleEntity {
  id: string;
  kind: EnemyKind;
  x: number;
  y: number;
  w: number;
  h: number;
  speed: number;
  drift: number;
  phase: number;
  hp: number;
  maxHp: number;
  color: string;
  active: boolean;
  recent: boolean;
  threatWeight: number;
  taskIds: string[];
  label: string;
  seed: number;
}

export interface Bullet {
  x: number;
  y: number;
  vx: number;
  vy: number;
  damage: number;
  color: string;
  fromEnemy: boolean;
}

export interface Spark {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  color: string;
}

export interface BattleEngineState {
  score: number;
  combo: number;
  comboDecayAt: number;
  lives: number;
  overload: number;
  wave: number;
  paused: boolean;
  gameOver: boolean;
  playerX: number;
  abilityCharge: number;
  entities: Map<string, BattleEntity>;
  tasks: Map<string, BattleTaskState>;
  bullets: Bullet[];
  sparks: Spark[];
  focusActiveUntil: number;
  pulseUntil: number;
  bossCooldownUntil: number;
  lastDirectorTick: number;
  shotCooldown: number;
}
