import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { TaskGraphNode } from "@/api/task-graph.types";
import type { InternalStatus } from "@/types/status";
import { INTERNAL_STATUS_VALUES } from "@/types/status";
import { getBattleSpecForStatus, mapStatusToBattleGroup, type BattleStatusGroup } from "./statusMapping";
import { useBattleModeTaskFeed } from "./useBattleModeTaskFeed";

interface BattleModeOverlayProps {
  active: boolean;
  tasks: TaskGraphNode[];
  runningCount: number;
  queuedCount: number;
  onExit: () => void;
}

interface Enemy {
  taskId: string;
  x: number;
  y: number;
  w: number;
  h: number;
  hp: number;
  baseHp: number;
  speed: number;
  drift: number;
  phase: number;
  color: string;
  group: BattleStatusGroup;
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

interface EngineState {
  playerX: number;
  bullets: Bullet[];
  enemies: Map<string, Enemy>;
  particles: Particle[];
  score: number;
  lives: number;
  wave: number;
  paused: boolean;
  gameOver: boolean;
  shotCooldown: number;
}

const VALID_STATUSES = new Set<string>(INTERNAL_STATUS_VALUES);

const PLAYER_WIDTH = 28;
const PLAYER_HEIGHT = 18;
const PLAYER_SPEED = 420;
const BULLET_SPEED = 460;
const SHOT_COOLDOWN = 0.14;

function toInternalStatus(raw: string): InternalStatus {
  if (VALID_STATUSES.has(raw)) {
    return raw as InternalStatus;
  }
  return "ready";
}

function seededEnemy(task: TaskGraphNode, index: number, total: number, width: number): Enemy | null {
  const status = toInternalStatus(task.internalStatus);
  const spec = getBattleSpecForStatus(status);
  if (!spec) return null;

  const cols = Math.max(6, Math.min(12, Math.ceil(Math.sqrt(Math.max(total, 1))) + 2));
  const col = index % cols;
  const row = Math.floor(index / cols);
  const margin = 48;
  const usable = Math.max(220, width - margin * 2);
  const cell = usable / cols;

  return {
    taskId: task.taskId,
    x: margin + col * cell + cell * 0.14,
    y: 48 + row * 34,
    w: 26,
    h: 20,
    hp: spec.hp,
    baseHp: spec.hp,
    speed: spec.speed,
    drift: 9 + (index % 5) * 2,
    phase: index * 0.65,
    color: spec.color,
    group: spec.group,
  };
}

function createInitialState(tasks: TaskGraphNode[], width: number): EngineState {
  const enemies = new Map<string, Enemy>();

  tasks.forEach((task, idx) => {
    const enemy = seededEnemy(task, idx, tasks.length, width);
    if (enemy) {
      enemies.set(task.taskId, enemy);
    }
  });

  return {
    playerX: Math.max(20, width / 2 - PLAYER_WIDTH / 2),
    bullets: [],
    enemies,
    particles: [],
    score: 0,
    lives: 3,
    wave: 1,
    paused: false,
    gameOver: false,
    shotCooldown: 0,
  };
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

  const [soundEnabled, setSoundEnabled] = useState(false);
  const [hud, setHud] = useState({ score: 0, lives: 3, wave: 1, gameOver: false, paused: false });

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
    stateRef.current = createInitialState(tasks, sizeRef.current.width);
    setHud({ score: 0, lives: 3, wave: 1, gameOver: false, paused: false });
  }, [tasks]);

  const syncHud = useCallback(() => {
    const s = stateRef.current;
    setHud({ score: s.score, lives: s.lives, wave: s.wave, gameOver: s.gameOver, paused: s.paused });
  }, []);

  useBattleModeTaskFeed({
    active,
    allowedTaskIds,
    onStatusEvent: useCallback(({ taskId, status }) => {
      const state = stateRef.current;
      const spec = getBattleSpecForStatus(status);

      if (!spec) {
        if (state.enemies.delete(taskId)) {
          state.score += mapStatusToBattleGroup(status) === "complete" ? 125 : 30;
          playSfx(840, 0.06);
        }
        return;
      }

      const existing = state.enemies.get(taskId);
      if (existing) {
        existing.group = spec.group;
        existing.color = spec.color;
        existing.speed = spec.speed;
        existing.baseHp = spec.hp;
        existing.hp = Math.max(existing.hp, spec.hp);
        return;
      }

      state.enemies.set(taskId, {
        taskId,
        x: 40 + Math.random() * Math.max(180, sizeRef.current.width - 120),
        y: 20,
        w: 26,
        h: 20,
        hp: spec.hp,
        baseHp: spec.hp,
        speed: spec.speed,
        drift: 8 + Math.random() * 12,
        phase: Math.random() * Math.PI,
        color: spec.color,
        group: spec.group,
      });
    }, [playSfx]),
  });

  useEffect(() => {
    if (!active) return;
    resetGame();
  }, [active, resetGame]);

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
    };

    resizeCanvas();
    window.addEventListener("resize", resizeCanvas);

    return () => {
      window.removeEventListener("resize", resizeCanvas);
    };
  }, [active]);

  useEffect(() => {
    if (!active) {
      if (frameRef.current !== null) {
        cancelAnimationFrame(frameRef.current);
        frameRef.current = null;
      }
      return;
    }

    const draw = (ctx: CanvasRenderingContext2D, state: EngineState) => {
      const { width, height } = sizeRef.current;

      ctx.clearRect(0, 0, width, height);

      const gradient = ctx.createLinearGradient(0, 0, 0, height);
      gradient.addColorStop(0, "rgba(12, 18, 32, 0.9)");
      gradient.addColorStop(1, "rgba(6, 10, 20, 0.95)");
      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, width, height);

      // Enemies
      for (const enemy of state.enemies.values()) {
        ctx.fillStyle = enemy.color;
        ctx.globalAlpha = 0.95;
        ctx.fillRect(enemy.x, enemy.y, enemy.w, enemy.h);
        ctx.fillStyle = "rgba(255,255,255,0.45)";
        ctx.fillRect(enemy.x + 5, enemy.y + 5, 4, 4);
        ctx.fillRect(enemy.x + enemy.w - 9, enemy.y + 5, 4, 4);
        ctx.globalAlpha = 1;

        if (enemy.baseHp > 1) {
          ctx.fillStyle = "rgba(255,255,255,0.7)";
          ctx.font = "11px monospace";
          ctx.fillText(String(enemy.hp), enemy.x + enemy.w / 2 - 3, enemy.y - 4);
        }
      }

      // Bullets
      ctx.fillStyle = "#f8fafc";
      for (const bullet of state.bullets) {
        ctx.fillRect(bullet.x, bullet.y, 3, 10);
      }

      // Particles
      for (const particle of state.particles) {
        ctx.fillStyle = particle.color;
        ctx.globalAlpha = Math.max(0, particle.life);
        ctx.fillRect(particle.x, particle.y, 2, 2);
      }
      ctx.globalAlpha = 1;

      // Player ship
      const py = height - 42;
      ctx.fillStyle = state.gameOver ? "#64748b" : "#7dd3fc";
      ctx.beginPath();
      ctx.moveTo(state.playerX + PLAYER_WIDTH / 2, py);
      ctx.lineTo(state.playerX, py + PLAYER_HEIGHT);
      ctx.lineTo(state.playerX + PLAYER_WIDTH, py + PLAYER_HEIGHT);
      ctx.closePath();
      ctx.fill();

      ctx.fillStyle = "rgba(125, 211, 252, 0.3)";
      ctx.fillRect(state.playerX + 6, py + PLAYER_HEIGHT, PLAYER_WIDTH - 12, 4);
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
            y: height - 46,
            vy: -BULLET_SPEED,
          });
          state.shotCooldown = SHOT_COOLDOWN;
          playSfx(440, 0.03);
        }

        const descentMultiplier = 1 + Math.min(1.8, runningCount * 0.17 + queuedCount * 0.03);

        for (const enemy of state.enemies.values()) {
          enemy.phase += dt * 1.5;
          enemy.x += Math.sin(enemy.phase) * enemy.drift * dt;
          enemy.y += enemy.speed * descentMultiplier * dt;

          if (enemy.x < 6) enemy.x = 6;
          if (enemy.x + enemy.w > width - 6) enemy.x = width - enemy.w - 6;

          if (enemy.y + enemy.h >= height - 44) {
            state.enemies.delete(enemy.taskId);
            state.lives -= 1;
            state.score = Math.max(0, state.score - 80);
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
        state.bullets = state.bullets.filter((b) => b.y > -12);

        const toRemove = new Set<string>();
        const spentBullets = new Set<number>();

        state.bullets.forEach((bullet, bulletIdx) => {
          for (const enemy of state.enemies.values()) {
            if (bullet.x < enemy.x || bullet.x > enemy.x + enemy.w || bullet.y < enemy.y || bullet.y > enemy.y + enemy.h) {
              continue;
            }

            spentBullets.add(bulletIdx);
            enemy.hp -= 1;
            state.score += enemy.group === "merge" ? 40 : 20;
            playSfx(620, 0.04);

            for (let i = 0; i < 8; i += 1) {
              state.particles.push({
                x: bullet.x,
                y: bullet.y,
                vx: (Math.random() - 0.5) * 80,
                vy: (Math.random() - 0.5) * 80,
                life: 1,
                color: enemy.color,
              });
            }

            if (enemy.hp <= 0) {
              toRemove.add(enemy.taskId);
              state.score += enemy.group === "failure" ? 120 : 70;
            }
            break;
          }
        });

        if (spentBullets.size > 0) {
          state.bullets = state.bullets.filter((_, index) => !spentBullets.has(index));
        }

        if (toRemove.size > 0) {
          for (const id of toRemove) {
            state.enemies.delete(id);
          }
        }

        for (const particle of state.particles) {
          particle.x += particle.vx * dt;
          particle.y += particle.vy * dt;
          particle.life -= dt * 2.1;
        }
        state.particles = state.particles.filter((p) => p.life > 0);

        if (state.enemies.size === 0 && !state.gameOver) {
          state.wave += 1;
          const nextWaveTasks = tasks.filter((task) => {
            const status = toInternalStatus(task.internalStatus);
            return mapStatusToBattleGroup(status) !== "complete";
          });
          nextWaveTasks.forEach((task, idx) => {
            const enemy = seededEnemy(task, idx, nextWaveTasks.length, sizeRef.current.width);
            if (enemy) {
              enemy.y = Math.max(22, enemy.y - 18);
              enemy.speed += Math.min(14, state.wave * 1.5);
              state.enemies.set(task.taskId, enemy);
            }
          });
        }
      }

      draw(ctx, state);
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
  }, [active, playSfx, queuedCount, runningCount, syncHud, tasks]);

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

      <div className="absolute top-3 left-3 flex items-center gap-2 rounded-md border border-white/10 bg-black/55 px-3 py-2 text-xs text-white/90">
        <span>Battle Mode</span>
        <span className="text-white/55">|</span>
        <span>Score {hud.score}</span>
        <span className="text-white/55">|</span>
        <span>Lives {hud.lives}</span>
        <span className="text-white/55">|</span>
        <span>Wave {hud.wave}</span>
      </div>

      <div className="absolute top-3 right-3 flex items-center gap-2">
        <button
          type="button"
          className="rounded-md border border-white/15 bg-black/55 px-3 py-2 text-xs text-white/90 hover:bg-black/70"
          onClick={() => {
            setSoundEnabled((prev) => !prev);
          }}
          data-testid="battle-mode-sound-toggle"
        >
          {soundEnabled ? "Sound On" : "Sound Off"}
        </button>
        <button
          type="button"
          className="rounded-md border border-white/15 bg-black/55 px-3 py-2 text-xs text-white/90 hover:bg-black/70"
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

      <div className="absolute bottom-3 left-3 rounded-md border border-white/10 bg-black/55 px-3 py-2 text-xs text-white/70">
        Move: A/D or Arrows | Fire: Space/J | Pause: P | Exit: Esc
      </div>

      {hud.gameOver && (
        <div className="absolute inset-0 flex items-center justify-center">
          <div className="rounded-lg border border-red-400/30 bg-black/70 px-6 py-5 text-center text-white">
            <p className="text-lg font-semibold">System Overrun</p>
            <p className="mt-1 text-sm text-white/80">Score: {hud.score}</p>
            <p className="mt-2 text-xs text-white/70">Press R to restart or Exit Battle</p>
          </div>
        </div>
      )}
    </div>
  );
}
