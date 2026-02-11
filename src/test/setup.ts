/**
 * Vitest test setup file
 * This file runs before each test file
 */

import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// Mock __UI_DEBUG__ global used by logger
(globalThis as unknown as { __UI_DEBUG__: boolean }).__UI_DEBUG__ = false;

// Mock ResizeObserver for Radix UI components (ScrollArea, etc.)
class ResizeObserverMock implements ResizeObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
global.ResizeObserver = ResizeObserverMock;

// Cleanup after each test case (e.g., clearing jsdom)
afterEach(() => {
  cleanup();
});

// Mock Tauri's invoke function for testing
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock Tauri's event module
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));

// Reset all mocks between tests
afterEach(() => {
  vi.clearAllMocks();
});
