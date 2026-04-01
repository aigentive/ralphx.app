export type BattleV2Quality = "high" | "balanced" | "low";

export interface BattleQualityPreset {
  maxSparks: number;
  starFar: number;
  starNear: number;
  bgStreakAlpha: number;
  screenShake: number;
}

export const BATTLE_V2_QUALITY_PRESETS: Record<BattleV2Quality, BattleQualityPreset> = {
  high: {
    maxSparks: 500,
    starFar: 150,
    starNear: 100,
    bgStreakAlpha: 0.22,
    screenShake: 1,
  },
  balanced: {
    maxSparks: 320,
    starFar: 100,
    starNear: 70,
    bgStreakAlpha: 0.16,
    screenShake: 0.7,
  },
  low: {
    maxSparks: 160,
    starFar: 60,
    starNear: 36,
    bgStreakAlpha: 0.08,
    screenShake: 0.35,
  },
};

export const BATTLE_V2_ACTIVE_WINDOW_MS = 12_000;
export const BATTLE_V2_ROSTER_LIMIT = 6;
export const BATTLE_V2_PLAYER_SPEED = 470;
export const BATTLE_V2_BULLET_SPEED = 560;
export const BATTLE_V2_ENEMY_BULLET_SPEED = 260;
export const BATTLE_V2_SHOT_COOLDOWN = 0.11;
export const BATTLE_V2_BOSS_MIN_INTERVAL_MS = 18_000;
