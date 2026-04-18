/**
 * Persistence helpers for settings dialog UI state.
 *
 * Wraps localStorage so the user returns to the same section, tab,
 * and collapse state across app restarts.
 */

import type { SettingsSectionId } from "./settings-registry";

const ACTIVE_SECTION_KEY = "ralphx-settings-active-section";
const HARNESS_TAB_KEY = "ralphx-settings-harness-tab";
const HARNESS_EXPANDED_KEY = "ralphx-settings-harness-expanded";

export type HarnessTabScope = "ideation" | "execution";
export type HarnessTabValue = "global" | "project";

export function loadActiveSection(): SettingsSectionId | null {
  try {
    const saved = localStorage.getItem(ACTIVE_SECTION_KEY);
    return saved ? (saved as SettingsSectionId) : null;
  } catch {
    return null;
  }
}

export function saveActiveSection(section: SettingsSectionId): void {
  try {
    localStorage.setItem(ACTIVE_SECTION_KEY, section);
  } catch {
    /* ignore write errors */
  }
}

function loadHarnessTabMap(): Record<string, HarnessTabValue> {
  try {
    const raw = localStorage.getItem(HARNESS_TAB_KEY);
    return raw ? (JSON.parse(raw) as Record<string, HarnessTabValue>) : {};
  } catch {
    return {};
  }
}

export function loadHarnessTab(scope: HarnessTabScope): HarnessTabValue {
  return loadHarnessTabMap()[scope] ?? "global";
}

export function saveHarnessTab(
  scope: HarnessTabScope,
  tab: HarnessTabValue,
): void {
  try {
    const next = { ...loadHarnessTabMap(), [scope]: tab };
    localStorage.setItem(HARNESS_TAB_KEY, JSON.stringify(next));
  } catch {
    /* ignore write errors */
  }
}

function loadHarnessExpandedMap(): Record<string, boolean> {
  try {
    const raw = localStorage.getItem(HARNESS_EXPANDED_KEY);
    return raw ? (JSON.parse(raw) as Record<string, boolean>) : {};
  } catch {
    return {};
  }
}

export function expandedKey(
  tab: HarnessTabValue,
  laneId: string,
): string {
  return `${tab}:${laneId}`;
}

export function loadHarnessExpanded(
  tab: HarnessTabValue,
  laneIds: string[],
): Record<string, boolean> {
  const map = loadHarnessExpandedMap();
  const result: Record<string, boolean> = {};
  for (const laneId of laneIds) {
    const stored = map[expandedKey(tab, laneId)];
    if (stored !== undefined) {
      result[laneId] = stored;
    }
  }
  return result;
}

export function saveHarnessExpanded(
  tab: HarnessTabValue,
  laneId: string,
  expanded: boolean,
): void {
  try {
    const map = loadHarnessExpandedMap();
    map[expandedKey(tab, laneId)] = expanded;
    localStorage.setItem(HARNESS_EXPANDED_KEY, JSON.stringify(map));
  } catch {
    /* ignore write errors */
  }
}

export function saveHarnessExpandedBulk(
  tab: HarnessTabValue,
  laneIds: string[],
  expanded: boolean,
): void {
  try {
    const map = loadHarnessExpandedMap();
    for (const laneId of laneIds) {
      map[expandedKey(tab, laneId)] = expanded;
    }
    localStorage.setItem(HARNESS_EXPANDED_KEY, JSON.stringify(map));
  } catch {
    /* ignore write errors */
  }
}
