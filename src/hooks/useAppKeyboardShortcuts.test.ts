/**
 * Tests for useAppKeyboardShortcuts — feature flag gating for ⌘4 and ⌘5
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { useAppKeyboardShortcuts } from "./useAppKeyboardShortcuts";
import type { FeatureFlags } from "@/types/feature-flags";

// Mock tauri global shortcut plugin
vi.mock("@tauri-apps/plugin-global-shortcut", () => ({
  register: vi.fn(() => Promise.resolve()),
  unregister: vi.fn(() => Promise.resolve()),
}));

function fireKeyDown(key: string, metaKey = true) {
  const event = new KeyboardEvent("keydown", { key, metaKey, bubbles: true });
  window.dispatchEvent(event);
}

function fireKeyDownWithShift(key: string) {
  const event = new KeyboardEvent("keydown", { key, metaKey: true, shiftKey: true, bubbles: true });
  window.dispatchEvent(event);
}

function makeProps(overrides: Partial<Parameters<typeof useAppKeyboardShortcuts>[0]> = {}) {
  return {
    currentView: "kanban" as const,
    setCurrentView: vi.fn(),
    toggleChatVisible: vi.fn(),
    ...overrides,
  };
}

describe("useAppKeyboardShortcuts — feature flag gating", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ── default behavior (all flags enabled) ──────────────────────────────────

  it("⌘4 navigates to extensibility when extensibilityPage flag is true", () => {
    const setCurrentView = vi.fn();
    const flags: FeatureFlags = { activityPage: true, extensibilityPage: true, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView, featureFlags: flags }))
    );

    fireKeyDown("4");
    expect(setCurrentView).toHaveBeenCalledWith("extensibility");
  });

  it("⌘5 navigates to activity when activityPage flag is true", () => {
    const setCurrentView = vi.fn();
    const flags: FeatureFlags = { activityPage: true, extensibilityPage: true, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView, featureFlags: flags }))
    );

    fireKeyDown("5");
    expect(setCurrentView).toHaveBeenCalledWith("activity");
  });

  // ── flags disabled ─────────────────────────────────────────────────────────

  it("⌘4 is a no-op when extensibilityPage flag is false", () => {
    const setCurrentView = vi.fn();
    const flags: FeatureFlags = { activityPage: true, extensibilityPage: false, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView, featureFlags: flags }))
    );

    fireKeyDown("4");
    expect(setCurrentView).not.toHaveBeenCalledWith("extensibility");
  });

  it("⌘5 is a no-op when activityPage flag is false", () => {
    const setCurrentView = vi.fn();
    const flags: FeatureFlags = { activityPage: false, extensibilityPage: true, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView, featureFlags: flags }))
    );

    fireKeyDown("5");
    expect(setCurrentView).not.toHaveBeenCalledWith("activity");
  });

  it("⌘4 blocked does not affect ⌘5 when activityPage is true", () => {
    const setCurrentView = vi.fn();
    const flags: FeatureFlags = { activityPage: true, extensibilityPage: false, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView, featureFlags: flags }))
    );

    fireKeyDown("5");
    expect(setCurrentView).toHaveBeenCalledWith("activity");
  });

  // ── default behavior when featureFlags is undefined ────────────────────────

  it("⌘4 navigates when featureFlags is not provided (defaults to all-enabled)", () => {
    const setCurrentView = vi.fn();

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView }))
    );

    fireKeyDown("4");
    expect(setCurrentView).toHaveBeenCalledWith("extensibility");
  });

  it("⌘5 navigates when featureFlags is not provided (defaults to all-enabled)", () => {
    const setCurrentView = vi.fn();

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView }))
    );

    fireKeyDown("5");
    expect(setCurrentView).toHaveBeenCalledWith("activity");
  });

  // ── other shortcuts still work ─────────────────────────────────────────────

  it("⌘1 still navigates to ideation regardless of flags", () => {
    const setCurrentView = vi.fn();
    const flags: FeatureFlags = { activityPage: false, extensibilityPage: false, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView, featureFlags: flags }))
    );

    fireKeyDown("1");
    expect(setCurrentView).toHaveBeenCalledWith("ideation");
  });

  it("⌘3 still navigates to kanban regardless of flags", () => {
    const setCurrentView = vi.fn();
    const flags: FeatureFlags = { activityPage: false, extensibilityPage: false, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ setCurrentView, featureFlags: flags }))
    );

    fireKeyDown("3");
    expect(setCurrentView).toHaveBeenCalledWith("kanban");
  });

  // ── CMD+SHIFT+B battle mode shortcut ──────────────────────────────────────

  it("⌘⇧B is a no-op when featureFlags.battleMode is false", () => {
    const onBattleModeToggle = vi.fn();
    const flags: FeatureFlags = { activityPage: true, extensibilityPage: true, battleMode: false };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ currentView: "graph", onBattleModeToggle, featureFlags: flags }))
    );

    fireKeyDownWithShift("b");
    expect(onBattleModeToggle).not.toHaveBeenCalled();
  });

  it("⌘⇧B calls onBattleModeToggle when flag is enabled and currentView is graph", () => {
    const onBattleModeToggle = vi.fn();
    const flags: FeatureFlags = { activityPage: true, extensibilityPage: true, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ currentView: "graph", onBattleModeToggle, featureFlags: flags }))
    );

    fireKeyDownWithShift("b");
    expect(onBattleModeToggle).toHaveBeenCalledOnce();
  });

  it("⌘⇧B is a no-op when flag is enabled but currentView is not graph", () => {
    const onBattleModeToggle = vi.fn();
    const flags: FeatureFlags = { activityPage: true, extensibilityPage: true, battleMode: true };

    renderHook(() =>
      useAppKeyboardShortcuts(makeProps({ currentView: "kanban", onBattleModeToggle, featureFlags: flags }))
    );

    fireKeyDownWithShift("b");
    expect(onBattleModeToggle).not.toHaveBeenCalled();
  });
});
