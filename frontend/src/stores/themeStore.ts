/**
 * Theme store — controls active theme + motion + font scale.
 *
 * Persists to localStorage and mirrors state to `<html>` data attributes so CSS
 * selectors in globals.css pick up the change:
 *   - data-theme="default" | "high-contrast"
 *   - data-motion="reduce" (optional override)
 *   - data-font-scale="lg" | "xl" (optional bump)
 *
 * A tiny inline script in index.html applies these attributes BEFORE React
 * hydrates to avoid flash-of-wrong-theme (FOWT).
 *
 * Spec: specs/design/theme-architecture.md
 */

import { create } from "zustand";

export type ThemeName = "dark" | "light" | "high-contrast";
export type MotionPreference = "system" | "reduce";
export type FontScale = "default" | "lg" | "xl";

/**
 * Non-high-contrast themes — used to remember the "last selected everyday
 * theme" so toggling high contrast off can snap back to it.
 */
export type BaseThemeName = Exclude<ThemeName, "high-contrast">;

const THEME_KEY = "ralphx-theme";
const LAST_BASE_THEME_KEY = "ralphx-last-base-theme";
const MOTION_KEY = "ralphx-motion";
const FONT_SCALE_KEY = "ralphx-font-scale";

function safeGet(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function safeSet(key: string, value: string | null): void {
  try {
    if (value === null) {
      localStorage.removeItem(key);
    } else {
      localStorage.setItem(key, value);
    }
  } catch {
    /* no-op */
  }
}

function loadTheme(): ThemeName {
  const raw = safeGet(THEME_KEY);
  // Migration: previous releases stored "default" — treat as the canonical
  // dark theme. Any unrecognised value also falls back to dark.
  if (raw === "high-contrast") return "high-contrast";
  if (raw === "light") return "light";
  return "dark";
}

function loadLastBaseTheme(): BaseThemeName {
  const raw = safeGet(LAST_BASE_THEME_KEY);
  return raw === "light" ? "light" : "dark";
}

function loadMotion(): MotionPreference {
  const v = safeGet(MOTION_KEY);
  return v === "reduce" ? "reduce" : "system";
}

function loadFontScale(): FontScale {
  const v = safeGet(FONT_SCALE_KEY);
  if (v === "lg" || v === "xl") return v;
  return "default";
}

function applyThemeAttr(theme: ThemeName): void {
  if (typeof document === "undefined") return;
  // `dark` is the implicit default (matches `:root` tokens) — no attribute
  // needed. `light` and `high-contrast` get explicit attributes.
  if (theme === "dark") {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.setAttribute("data-theme", theme);
  }
}

function applyMotionAttr(motion: MotionPreference): void {
  if (typeof document === "undefined") return;
  if (motion === "reduce") {
    document.documentElement.setAttribute("data-motion", "reduce");
  } else {
    document.documentElement.removeAttribute("data-motion");
  }
}

function applyFontScaleAttr(scale: FontScale): void {
  if (typeof document === "undefined") return;
  if (scale === "default") {
    document.documentElement.removeAttribute("data-font-scale");
  } else {
    document.documentElement.setAttribute("data-font-scale", scale);
  }
}

interface ThemeState {
  theme: ThemeName;
  /**
   * Last non-HC theme the user chose — used when toggling HC off so they
   * return to their preferred base theme rather than always snapping to dark.
   */
  lastBaseTheme: BaseThemeName;
  motion: MotionPreference;
  fontScale: FontScale;
  setTheme: (theme: ThemeName) => void;
  setMotion: (motion: MotionPreference) => void;
  setFontScale: (scale: FontScale) => void;
  toggleHighContrast: () => void;
}

export const useThemeStore = create<ThemeState>((set, get) => ({
  theme: loadTheme(),
  lastBaseTheme: loadLastBaseTheme(),
  motion: loadMotion(),
  fontScale: loadFontScale(),
  setTheme: (theme) => {
    // Persist theme — `dark` is the implicit default, keep key empty for it.
    safeSet(THEME_KEY, theme === "dark" ? null : theme);
    applyThemeAttr(theme);
    // Remember the last base (non-HC) theme so toggling HC off can restore it.
    if (theme !== "high-contrast") {
      safeSet(LAST_BASE_THEME_KEY, theme === "dark" ? null : theme);
      set({ theme, lastBaseTheme: theme });
    } else {
      set({ theme });
    }
  },
  setMotion: (motion) => {
    safeSet(MOTION_KEY, motion === "system" ? null : motion);
    applyMotionAttr(motion);
    set({ motion });
  },
  setFontScale: (scale) => {
    safeSet(FONT_SCALE_KEY, scale === "default" ? null : scale);
    applyFontScaleAttr(scale);
    set({ fontScale: scale });
  },
  toggleHighContrast: () => {
    const state = get();
    const next: ThemeName =
      state.theme === "high-contrast" ? state.lastBaseTheme : "high-contrast";
    state.setTheme(next);
  },
}));

/**
 * Initialise DOM attributes from the persisted store state on app mount. Call
 * once from the app root so the React state and the DOM attributes stay in
 * sync even if the inline bootstrap script in index.html was missed.
 */
export function syncThemeAttributesFromStore(): void {
  const { theme, motion, fontScale } = useThemeStore.getState();
  applyThemeAttr(theme);
  applyMotionAttr(motion);
  applyFontScaleAttr(fontScale);
}
