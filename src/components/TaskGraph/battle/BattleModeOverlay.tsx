import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { TaskGraphNode } from "@/api/task-graph.types";
import type { InternalStatus } from "@/types/status";
import { INTERNAL_STATUS_VALUES } from "@/types/status";
import {
  BATTLE_ACTIVE_ROSTER_LIMIT,
  BATTLE_MAX_PARTICLES,
  type BattleQuality,
} from "./constants";
import {
  buildBattleEntities,
  type BattleClusterEntity,
  type BattleEntity,
  type BattleIndividualEntity,
  type BattleTaskState,
} from "./entities";
import {
  getThreatWeight,
  isActivelyWorkedStatus,
  mapStatusToBattleGroup,
} from "./statusMapping";
import { useBattleModeTaskFeed } from "./useBattleModeTaskFeed";

interface BattleModeOverlayProps {
  active: boolean;
  tasks: TaskGraphNode[];
  runningCount: number;
  queuedCount: number;
  onExit: () => void;
}

interface Bullet {
  x: number;
  y: number;
  vy: number;
}

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  color: string;
}

interface Star {
  x: number;
  y: number;
  speed: number;
  size: number;
  alpha: number;
}

interface EngineState {
  playerX: number;
  bullets: Bullet[];
  entities: Map<string, BattleEntity>;
  taskStates: Map<string, BattleTaskState>;
  particles: Particle[];
  score: number;
  lives: number;
  wave: number;
  paused: boolean;
  gameOver: boolean;
  shotCooldown: number;
  focusActiveUntil: number;
  recentTransitionPulseUntil: number;
}

function reconcileEntities(
  previous: Map<string, BattleEntity>,
  next: Map<string, BattleEntity>
): Map<string, BattleEntity> {
  const reconciled = new Map<string, BattleEntity>();

  for (const [id, entity] of next) {
    const prev = previous.get(id);
    if (!prev || prev.kind !== entity.kind) {
      reconciled.set(id, entity);
      continue;
    }

    if (entity.kind === "cluster" && prev.kind === "cluster") {
      const merged: BattleClusterEntity = {
        ...entity,
        x: prev.x,
        y: prev.y,
        phase: prev.phase,
        hp: Math.min(prev.hp, entity.hp),
      };
      reconciled.set(id, merged);
      continue;
    }

    if (entity.kind === "individual" && prev.kind === "individual") {
      const merged: BattleIndividualEntity = {
        ...entity,
        x: prev.x,
        y: prev.y,
        phase: prev.phase,
        hp: Math.min(prev.hp, entity.baseHp),
      };
      reconciled.set(id, merged);
      continue;
    }

    reconciled.set(id, entity);
  }

  return reconciled;
}

const VALID_STATUSES = new Set<string>(INTERNAL_STATUS_VALUES);

const PLAYER_WIDTH = 30;
const PLAYER_HEIGHT = 19;
const PLAYER_SPEED = 430;
const BULLET_SPEED = 500;
const SHOT_COOLDOWN = 0.13;

function toInternalStatus(raw: string): InternalStatus {
  if (VALID_STATUSES.has(raw)) {
    return raw as InternalStatus;
  }
  return "ready";
}

function qualityFactor(quality: BattleQuality): number {
  switch (quality) {
    case "high":
      return 1;
    case "balanced":
      return 0.66;
    case "low":
      return 0.35;
  }
}

function qualityLabel(quality: BattleQuality): string {
  switch (quality) {
    case "high":
      return "High";
    case "balanced":
      return "Balanced";
    case "low":
      return "Low";
  }
}

function nextQuality(quality: BattleQuality): BattleQuality {
  if (quality === "balanced") return "high";
  if (quality === "high") return "low";
  return "balanced";
}

function createTaskStates(tasks: TaskGraphNode[], now: number): Map<string, BattleTaskState> {
  const map = new Map<string, BattleTaskState>();
  for (const task of tasks) {
    map.set(task.taskId, {
      taskId: task.taskId,
      title: task.title,
      status: toInternalStatus(task.internalStatus),
      lastTransitionAt: now,
    });
  }
  return map;
}

function createInitialState(tasks: TaskGraphNode[], width: number): EngineState {
  const now = Date.now();
  const taskStates = createTaskStates(tasks, now);
  return {
    playerX: Math.max(20, width / 2 - PLAYER_WIDTH / 2),
    bullets: [],
    entities: buildBattleEntities(taskStates, width, now),
    taskStates,
    particles: [],
    score: 0,
    lives: 3,
    wave: 1,
    paused: false,
    gameOver: false,
    shotCooldown: 0,
    focusActiveUntil: 0,
    recentTransitionPulseUntil: 0,
  };
}

function deriveActiveRoster(taskStates: Map<string, BattleTaskState>): Array<{ taskId: string; title: string; status: InternalStatus }> {
  const roster = Array.from(taskStates.values())
    .filter((task) => isActivelyWorkedStatus(task.status))
    .sort((a, b) => getThreatWeight(b.status) - getThreatWeight(a.status));

  return roster.slice(0, BATTLE_ACTIVE_ROSTER_LIMIT).map((task) => ({
    taskId: task.taskId,
    title: task.title.length > 20 ? `${task.title.slice(0, 20)}...` : task.title,
    status: task.status,
  }));
}

export function BattleModeOverlay({
  active,
  tasks,
  runningCount,
  queuedCount,
  onExit,
}: BattleModeOverlayProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const frameRef = useRef<number | null>(null);
  const lastTsRef = useRef<number>(0);
  const sizeRef = useRef({ width: 800, height: 600 });
  const keysRef = useRef({ left: false, right: false, firing: false });
  const audioRef = useRef<AudioContext | null>(null);
  const starsNearRef = useRef<Star[]>([]);
  const starsFarRef = useRef<Star[]>([]);
  const lastEntityRefreshRef = useRef<number>(0);

  const [soundEnabled, setSoundEnabled] = useState(false);
  const [quality, setQuality] = useState<BattleQuality>("balanced");
  const [hud, setHud] = useState({
    score: 0,
    lives: 3,
    wave: 1,
    gameOver: false,
    paused: false,
    activeRoster: [] as Array<{ taskId: string; title: string; status: InternalStatus }>,
  });

  const stateRef = useRef<EngineState>(createInitialState(tasks, 800));

  const allowedTaskIds = useMemo(() => new Set(tasks.map((task) => task.taskId)), [tasks]);

  const playSfx = useCallback((freq: number, durationSec: number) => {
    if (!soundEnabled) return;

    const context = audioRef.current ?? new AudioContext();
    audioRef.current = context;

    const oscillator = context.createOscillator();
    const gainNode = context.createGain();
    oscillator.type = "square";
    oscillator.frequency.value = freq;

    gainNode.gain.value = 0.03;
    gainNode.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + durationSec);

    oscillator.connect(gainNode);
    gainNode.connect(context.destination);
    oscillator.start();
    oscillator.stop(context.currentTime + durationSec);
  }, [soundEnabled]);

  const resetGame = useCallback(() => {
    const next = createInitialState(tasks, sizeRef.current.width);
    stateRef.current = next;
    setHud({
      score: next.score,
      lives: next.lives,
      wave: next.wave,
      gameOver: next.gameOver,
      paused: next.paused,
      activeRoster: deriveActiveRoster(next.taskStates),
    });
  }, [tasks]);

  const syncHud = useCallback(() => {
    const s = stateRef.current;
    setHud({
      score: s.score,
      lives: s.lives,
      wave: s.wave,
      gameOver: s.gameOver,
      paused: s.paused,
      activeRoster: deriveActiveRoster(s.taskStates),
    });
  }, []);

  useBattleModeTaskFeed({
    active,
    allowedTaskIds,
    onStatusEvent: useCallback(({ taskId, toStatus, timestamp }) => {
      const state = stateRef.current;
      const existing = state.taskStates.get(taskId);
      if (existing) {
        existing.status = toStatus;
        existing.lastTransitionAt = timestamp;
      } else {
        const matchedTask = tasks.find((task) => task.taskId === taskId);
        state.taskStates.set(taskId, {
          taskId,
          title: matchedTask?.title ?? taskId,
          status: toStatus,
          lastTransitionAt: timestamp,
        });
      }

      const group = mapStatusToBattleGroup(toStatus);
      if (group === "complete") {
        state.score += 120;
        playSfx(820, 0.06);
      }

      state.recentTransitionPulseUntil = Date.now() + 900;
      const rebuilt = buildBattleEntities(state.taskStates, sizeRef.current.width, Date.now());
      state.entities = reconcileEntities(state.entities, rebuilt);
    }, [playSfx, tasks]),
  });

  useEffect(() => {
    if (!active) return;
    resetGame();
  }, [active, resetGame]);

  useEffect(() => {
    if (!active) return;

    const now = Date.now();
    const state = stateRef.current;
    const nextTasks = createTaskStates(tasks, now);

    for (const [taskId, current] of state.taskStates) {
      const incoming = nextTasks.get(taskId);
      if (incoming) {
        incoming.lastTransitionAt = current.lastTransitionAt;
      }
    }

    state.taskStates = nextTasks;
    state.entities = reconcileEntities(
      state.entities,
      buildBattleEntities(state.taskStates, sizeRef.current.width, now)
    );
    syncHud();
  }, [active, tasks, syncHud]);

  useEffect(() => {
    if (!active) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "ArrowLeft" || event.key.toLowerCase() === "a") {
        keysRef.current.left = true;
      }
      if (event.key === "ArrowRight" || event.key.toLowerCase() === "d") {
        keysRef.current.right = true;
      }
      if (event.key === " " || event.key.toLowerCase() === "j") {
        keysRef.current.firing = true;
      }
      if (event.key.toLowerCase() === "p") {
        stateRef.current.paused = !stateRef.current.paused;
        syncHud();
      }
      if (event.key.toLowerCase() === "f") {
        stateRef.current.focusActiveUntil = Date.now() + 3000;
      }
      if (event.key === "Escape") {
        onExit();
      }
      if (event.key.toLowerCase() === "r" && stateRef.current.gameOver) {
        resetGame();
      }
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      if (event.key === "ArrowLeft" || event.key.toLowerCase() === "a") {
        keysRef.current.left = false;
      }
      if (event.key === "ArrowRight" || event.key.toLowerCase() === "d") {
        keysRef.current.right = false;
      }
      if (event.key === " " || event.key.toLowerCase() === "j") {
        keysRef.current.firing = false;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
    };
  }, [active, onExit, resetGame, syncHud]);

  useEffect(() => {
    if (!active) return;

    const randomStar = (width: number, height: number, near: boolean): Star => ({
      x: Math.random() * width,
      y: Math.random() * height,
      speed: near ? 12 + Math.random() * 16 : 4 + Math.random() * 8,
      size: near ? 1.1 + Math.random() * 1.9 : 0.5 + Math.random() * 1,
      alpha: near ? 0.18 + Math.random() * 0.4 : 0.1 + Math.random() * 0.25,
    });

    const resizeCanvas = () => {
      const canvas = canvasRef.current;
      if (!canvas || !canvas.parentElement) return;

      const rect = canvas.parentElement.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;

      sizeRef.current.width = Math.max(420, rect.width);
      sizeRef.current.height = Math.max(320, rect.height);

      canvas.width = Math.floor(sizeRef.current.width * dpr);
      canvas.height = Math.floor(sizeRef.current.height * dpr);
      canvas.style.width = `${sizeRef.current.width}px`;
      canvas.style.height = `${sizeRef.current.height}px`;

      const ctx = canvas.getContext("2d");
      if (ctx) {
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      }

      const qualityScale = qualityFactor(quality);
      const farCount = Math.floor(70 * qualityScale + 40);
      const nearCount = Math.floor(55 * qualityScale + 28);
      starsFarRef.current = Array.from({ length: farCount }, () => randomStar(sizeRef.current.width, sizeRef.current.height, false));
      starsNearRef.current = Array.from({ length: nearCount }, () => randomStar(sizeRef.current.width, sizeRef.current.height, true));

      stateRef.current.entities = buildBattleEntities(
        stateRef.current.taskStates,
        sizeRef.current.width,
        Date.now()
      );
    };

    resizeCanvas();
    window.addEventListener("resize", resizeCanvas);

    return () => {
      window.removeEventListener("resize", resizeCanvas);
    };
  }, [active, quality]);

  useEffect(() => {
    if (!active) {
      if (frameRef.current !== null) {
        cancelAnimationFrame(frameRef.current);
        frameRef.current = null;
      }
      return;
    }

    const drawBackground = (ctx: CanvasRenderingContext2D, dt: number) => {
      const { width, height } = sizeRef.current;
      const pulse = 0.04 * Math.sin(Date.now() / 400);

      const gradient = ctx.createLinearGradient(0, 0, width, height);
      gradient.addColorStop(0, "rgba(6, 12, 26, 0.98)");
      gradient.addColorStop(0.5, `rgba(18, 10, 44, ${0.93 + pulse})`);
      gradient.addColorStop(1, "rgba(5, 8, 20, 0.98)");
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, width, height);

      const nebula = ctx.createRadialGradient(width * 0.2, height * 0.1, 10, width * 0.2, height * 0.1, width * 0.7);
      nebula.addColorStop(0, "rgba(87, 172, 255, 0.18)");
      nebula.addColorStop(0.5, "rgba(145, 72, 255, 0.12)");
      nebula.addColorStop(1, "rgba(0, 0, 0, 0)");
      ctx.fillStyle = nebula;
      ctx.fillRect(0, 0, width, height);

      for (const star of starsFarRef.current) {
        star.y += star.speed * dt;
        if (star.y > height + 2) star.y = -3;
        ctx.fillStyle = `rgba(170, 210, 255, ${star.alpha})`;
        ctx.fillRect(star.x, star.y, star.size, star.size);
      }

      for (const star of starsNearRef.current) {
        star.y += star.speed * dt;
        if (star.y > height + 2) star.y = -3;
        ctx.fillStyle = `rgba(255, 255, 255, ${star.alpha})`;
        ctx.fillRect(star.x, star.y, star.size, star.size);
      }
    };

    const drawEntity = (ctx: CanvasRenderingContext2D, entity: BattleEntity, now: number, focusActive: boolean) => {
      const dimmed = focusActive && !(entity.active || entity.recent);
      const alpha = dimmed ? 0.22 : 1;

      ctx.save();
      ctx.globalAlpha = alpha;

      if (entity.kind === "cluster") {
        const cluster = entity as BattleClusterEntity;
        ctx.fillStyle = `${cluster.color}44`;
        ctx.strokeStyle = `${cluster.color}`;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.roundRect(cluster.x, cluster.y, cluster.w, cluster.h, 8);
        ctx.fill();
        ctx.stroke();

        ctx.fillStyle = "rgba(248,250,252,0.95)";
        ctx.font = "12px ui-monospace, SFMono-Regular, Menlo, monospace";
        ctx.fillText(`x${cluster.count}`, cluster.x + 10, cluster.y + 19);
      } else {
        const individual = entity as BattleIndividualEntity;

        // Neon glow
        ctx.shadowBlur = individual.active ? 18 : 10;
        ctx.shadowColor = individual.color;
        ctx.fillStyle = `${individual.color}dd`;
        ctx.fillRect(individual.x, individual.y, individual.w, individual.h);

        ctx.shadowBlur = 0;
        ctx.fillStyle = "rgba(255,255,255,0.62)";
        ctx.fillRect(individual.x + 6, individual.y + 5, 5, 5);
        ctx.fillRect(individual.x + individual.w - 11, individual.y + 5, 5, 5);

        if (individual.baseHp > 1) {
          ctx.fillStyle = "rgba(255,255,255,0.9)";
          ctx.font = "11px ui-monospace, SFMono-Regular, Menlo, monospace";
          ctx.fillText(String(individual.hp), individual.x + individual.w / 2 - 3, individual.y - 5);
        }

        if (individual.active || individual.recent) {
          const ringPulse = 1 + 0.15 * Math.sin(now / 180);
          const ringW = individual.w * ringPulse;
          const ringH = individual.h * ringPulse;
          ctx.strokeStyle = individual.active ? "rgba(255,255,255,0.95)" : "rgba(255,230,160,0.8)";
          ctx.lineWidth = 1.6;
          ctx.beginPath();
          ctx.roundRect(
            individual.x - (ringW - individual.w) / 2 - 2,
            individual.y - (ringH - individual.h) / 2 - 2,
            ringW + 4,
            ringH + 4,
            8
          );
          ctx.stroke();

          // Trail
          ctx.strokeStyle = `${individual.color}88`;
          ctx.lineWidth = 1.2;
          ctx.beginPath();
          ctx.moveTo(individual.x + individual.w / 2, individual.y + individual.h + 2);
          ctx.lineTo(individual.x + individual.w / 2, individual.y + individual.h + (individual.active ? 18 : 10));
          ctx.stroke();

          ctx.fillStyle = "rgba(241,245,249,0.95)";
          ctx.font = "10px ui-monospace, SFMono-Regular, Menlo, monospace";
          ctx.fillText(individual.title, individual.x - 14, individual.y - 8);
        }
      }

      ctx.restore();
    };

    const drawPlayer = (ctx: CanvasRenderingContext2D, state: EngineState, now: number) => {
      const { height } = sizeRef.current;
      const py = height - 46;
      const glow = state.gameOver ? "rgba(100,116,139,0.7)" : "rgba(125,211,252,0.85)";
      const engine = 1 + 0.2 * Math.sin(now / 95);

      ctx.shadowBlur = 16;
      ctx.shadowColor = glow;
      ctx.fillStyle = state.gameOver ? "#64748b" : "#7dd3fc";
      ctx.beginPath();
      ctx.moveTo(state.playerX + PLAYER_WIDTH / 2, py);
      ctx.lineTo(state.playerX, py + PLAYER_HEIGHT);
      ctx.lineTo(state.playerX + PLAYER_WIDTH, py + PLAYER_HEIGHT);
      ctx.closePath();
      ctx.fill();

      ctx.shadowBlur = 0;
      ctx.fillStyle = "rgba(56,189,248,0.55)";
      ctx.fillRect(state.playerX + 8, py + PLAYER_HEIGHT, PLAYER_WIDTH - 16, 4);

      ctx.fillStyle = "rgba(147,197,253,0.65)";
      ctx.fillRect(state.playerX + PLAYER_WIDTH / 2 - 2, py + PLAYER_HEIGHT + 2, 4, 8 * engine);
    };

    const tick = (ts: number) => {
      const canvas = canvasRef.current;
      const ctx = canvas?.getContext("2d");
      if (!canvas || !ctx) {
        frameRef.current = requestAnimationFrame(tick);
        return;
      }

      const last = lastTsRef.current || ts;
      const dt = Math.min(0.05, (ts - last) / 1000);
      lastTsRef.current = ts;
      const now = Date.now();

      const state = stateRef.current;

      if (!state.paused && !state.gameOver) {
        const { width, height } = sizeRef.current;

        const moveLeft = keysRef.current.left ? -1 : 0;
        const moveRight = keysRef.current.right ? 1 : 0;
        state.playerX += (moveLeft + moveRight) * PLAYER_SPEED * dt;
        state.playerX = Math.max(8, Math.min(width - PLAYER_WIDTH - 8, state.playerX));

        state.shotCooldown = Math.max(0, state.shotCooldown - dt);
        if (keysRef.current.firing && state.shotCooldown <= 0) {
          state.bullets.push({
            x: state.playerX + PLAYER_WIDTH / 2,
            y: height - 50,
            vy: -BULLET_SPEED,
          });
          state.shotCooldown = SHOT_COOLDOWN;
          playSfx(420, 0.028);
        }

        const descentMultiplier = 1 + Math.min(1.9, runningCount * 0.16 + queuedCount * 0.02);

        for (const entity of state.entities.values()) {
          entity.phase += dt * 1.6;
          entity.x += Math.sin(entity.phase) * entity.drift * dt;
          entity.y += entity.speed * descentMultiplier * dt;

          if (entity.x < 6) entity.x = 6;
          if (entity.x + entity.w > width - 6) entity.x = width - entity.w - 6;

          if (entity.y + entity.h >= height - 48) {
            state.entities.delete(entity.id);
            state.lives -= 1;
            state.score = Math.max(0, state.score - 90);
            playSfx(160, 0.1);

            if (state.lives <= 0) {
              state.gameOver = true;
              keysRef.current.firing = false;
            }
          }
        }

        for (const bullet of state.bullets) {
          bullet.y += bullet.vy * dt;
        }
        state.bullets = state.bullets.filter((bullet) => bullet.y > -16);

        const spentBullets = new Set<number>();

        state.bullets.forEach((bullet, index) => {
          for (const entity of state.entities.values()) {
            if (
              bullet.x < entity.x ||
              bullet.x > entity.x + entity.w ||
              bullet.y < entity.y ||
              bullet.y > entity.y + entity.h
            ) {
              continue;
            }

            spentBullets.add(index);
            playSfx(620, 0.035);

            const particleBudget = Math.max(2, Math.floor(8 * qualityFactor(quality)));
            if (state.particles.length < BATTLE_MAX_PARTICLES) {
              for (let i = 0; i < particleBudget; i += 1) {
                state.particles.push({
                  x: bullet.x,
                  y: bullet.y,
                  vx: (Math.random() - 0.5) * 95,
                  vy: (Math.random() - 0.5) * 95,
                  life: 1,
                  color: entity.color,
                });
              }
            }

            if (entity.kind === "cluster") {
              const cluster = entity as BattleClusterEntity;
              cluster.hp -= 1;
              state.score += 28;
              if (cluster.hp <= 0) {
                state.entities.delete(cluster.id);
                for (const taskId of cluster.taskIds) {
                  state.taskStates.delete(taskId);
                }
                state.score += cluster.count * 10;
              }
            } else {
              const individual = entity as BattleIndividualEntity;
              individual.hp -= 1;
              state.score += individual.group === "merge" ? 40 : 24;
              if (individual.hp <= 0) {
                state.entities.delete(individual.id);
                state.taskStates.delete(individual.taskId);
                state.score += individual.group === "failure" ? 130 : 72;
              }
            }

            break;
          }
        });

        if (spentBullets.size > 0) {
          state.bullets = state.bullets.filter((_, index) => !spentBullets.has(index));
        }

        for (const particle of state.particles) {
          particle.x += particle.vx * dt;
          particle.y += particle.vy * dt;
          particle.life -= dt * 2;
        }
        state.particles = state.particles.filter((particle) => particle.life > 0);

        if (now - lastEntityRefreshRef.current > 900) {
          state.entities = reconcileEntities(
            state.entities,
            buildBattleEntities(state.taskStates, sizeRef.current.width, now)
          );
          lastEntityRefreshRef.current = now;
        }

        if (state.entities.size === 0 && !state.gameOver) {
          state.wave += 1;
          state.entities = reconcileEntities(
            state.entities,
            buildBattleEntities(state.taskStates, sizeRef.current.width, now)
          );
        }
      }

      const focusActive = state.focusActiveUntil > now;

      drawBackground(ctx, dt);

      if (state.recentTransitionPulseUntil > now) {
        const pulseProgress = 1 - (state.recentTransitionPulseUntil - now) / 900;
        ctx.strokeStyle = `rgba(255, 161, 102, ${0.35 - pulseProgress * 0.3})`;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.roundRect(8, 8, sizeRef.current.width - 16, sizeRef.current.height - 16, 12);
        ctx.stroke();
      }

      for (const entity of state.entities.values()) {
        drawEntity(ctx, entity, now, focusActive);
      }

      ctx.fillStyle = "rgba(241,245,249,0.95)";
      for (const bullet of state.bullets) {
        ctx.fillRect(bullet.x, bullet.y, 3, 10);
      }

      for (const particle of state.particles) {
        ctx.fillStyle = particle.color;
        ctx.globalAlpha = Math.max(0, particle.life);
        ctx.fillRect(particle.x, particle.y, 2, 2);
      }
      ctx.globalAlpha = 1;

      drawPlayer(ctx, state, now);
      syncHud();
      frameRef.current = requestAnimationFrame(tick);
    };

    frameRef.current = requestAnimationFrame(tick);

    return () => {
      if (frameRef.current !== null) {
        cancelAnimationFrame(frameRef.current);
        frameRef.current = null;
      }
      lastTsRef.current = 0;
    };
  }, [active, playSfx, quality, queuedCount, runningCount, syncHud]);

  useEffect(() => {
    return () => {
      if (audioRef.current) {
        audioRef.current.close().catch(() => undefined);
      }
    };
  }, []);

  if (!active) return null;

  return (
    <div className="absolute inset-0 z-20" data-testid="battle-mode-overlay">
      <canvas ref={canvasRef} className="absolute inset-0" />

      <div className="absolute top-3 left-3 flex items-center gap-2 rounded-md border border-cyan-200/20 bg-slate-950/65 px-3 py-2 text-xs text-slate-100">
        <span className="font-semibold text-cyan-300">Battle Mode</span>
        <span className="text-slate-500">|</span>
        <span>Score {hud.score}</span>
        <span className="text-slate-500">|</span>
        <span>Lives {hud.lives}</span>
        <span className="text-slate-500">|</span>
        <span>Wave {hud.wave}</span>
      </div>

      <div className="absolute top-3 left-1/2 -translate-x-1/2 rounded-md border border-white/10 bg-slate-950/60 px-3 py-2 text-xs text-slate-100">
        <span className="text-amber-200">Active Now</span>
        <span className="mx-2 text-slate-500">|</span>
        {hud.activeRoster.length === 0 ? (
          <span className="text-slate-400">No active tasks</span>
        ) : (
          <span className="text-slate-200">
            {hud.activeRoster.map((item) => `${item.taskId.slice(0, 8)}:${item.status}`).join("  •  ")}
          </span>
        )}
      </div>

      <div className="absolute top-3 right-3 flex items-center gap-2">
        <button
          type="button"
          className="rounded-md border border-white/15 bg-slate-950/65 px-3 py-2 text-xs text-slate-100 hover:bg-slate-900/80"
          onClick={() => {
            setQuality((prev) => nextQuality(prev));
          }}
          data-testid="battle-mode-quality-toggle"
        >
          Quality: {qualityLabel(quality)}
        </button>
        <button
          type="button"
          className="rounded-md border border-white/15 bg-slate-950/65 px-3 py-2 text-xs text-slate-100 hover:bg-slate-900/80"
          onClick={() => {
            setSoundEnabled((prev) => !prev);
          }}
          data-testid="battle-mode-sound-toggle"
        >
          {soundEnabled ? "Sound On" : "Sound Off"}
        </button>
        <button
          type="button"
          className="rounded-md border border-white/15 bg-slate-950/65 px-3 py-2 text-xs text-slate-100 hover:bg-slate-900/80"
          onClick={() => {
            stateRef.current.focusActiveUntil = Date.now() + 3000;
          }}
          data-testid="battle-mode-focus-active"
        >
          Focus Active
        </button>
        <button
          type="button"
          className="rounded-md border border-white/15 bg-slate-950/65 px-3 py-2 text-xs text-slate-100 hover:bg-slate-900/80"
          onClick={() => {
            stateRef.current.paused = !stateRef.current.paused;
            syncHud();
          }}
          data-testid="battle-mode-pause-toggle"
        >
          {hud.paused ? "Resume" : "Pause"}
        </button>
        <button
          type="button"
          className="rounded-md border border-orange-400/35 bg-orange-500/20 px-3 py-2 text-xs text-orange-100 hover:bg-orange-500/35"
          onClick={onExit}
          data-testid="battle-mode-exit"
        >
          Exit Battle
        </button>
      </div>

      <div className="absolute bottom-3 left-3 rounded-md border border-white/10 bg-slate-950/65 px-3 py-2 text-xs text-slate-200">
        Move: A/D or Arrows | Fire: Space/J | Pause: P | Focus Active: F | Exit: Esc
      </div>

      {hud.gameOver && (
        <div className="absolute inset-0 flex items-center justify-center">
          <div className="rounded-lg border border-red-400/35 bg-slate-950/80 px-6 py-5 text-center text-white">
            <p className="text-lg font-semibold text-red-200">System Overrun</p>
            <p className="mt-1 text-sm text-slate-200">Score: {hud.score}</p>
            <p className="mt-2 text-xs text-slate-300">Press R to restart or Exit Battle</p>
          </div>
        </div>
      )}
    </div>
  );
}
