/**
 * Tests for keyboard shortcuts hook
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useKeyboardShortcuts } from "./useKeyboardShortcuts";

describe("useKeyboardShortcuts", () => {
  const mockOnMove = vi.fn();
  const mockOnDelete = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    // Clean up any event listeners
  });

  it("should call onMove with 'planned' when P is pressed", () => {
    renderHook(() =>
      useKeyboardShortcuts({
        selectedTaskId: "task-1",
        onMove: mockOnMove,
        onDelete: mockOnDelete,
      })
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "p" });
      document.dispatchEvent(event);
    });

    expect(mockOnMove).toHaveBeenCalledWith("task-1", "planned");
  });

  it("should call onMove with 'backlog' when B is pressed", () => {
    renderHook(() =>
      useKeyboardShortcuts({
        selectedTaskId: "task-1",
        onMove: mockOnMove,
        onDelete: mockOnDelete,
      })
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "b" });
      document.dispatchEvent(event);
    });

    expect(mockOnMove).toHaveBeenCalledWith("task-1", "backlog");
  });

  it("should call onMove with 'todo' when T is pressed", () => {
    renderHook(() =>
      useKeyboardShortcuts({
        selectedTaskId: "task-1",
        onMove: mockOnMove,
        onDelete: mockOnDelete,
      })
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "t" });
      document.dispatchEvent(event);
    });

    expect(mockOnMove).toHaveBeenCalledWith("task-1", "todo");
  });

  it("should call onDelete when Delete is pressed", () => {
    renderHook(() =>
      useKeyboardShortcuts({
        selectedTaskId: "task-1",
        onMove: mockOnMove,
        onDelete: mockOnDelete,
      })
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "Delete" });
      document.dispatchEvent(event);
    });

    expect(mockOnDelete).toHaveBeenCalledWith("task-1");
  });

  it("should not call any callback when no task is selected", () => {
    renderHook(() =>
      useKeyboardShortcuts({
        selectedTaskId: null,
        onMove: mockOnMove,
        onDelete: mockOnDelete,
      })
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "p" });
      document.dispatchEvent(event);
    });

    expect(mockOnMove).not.toHaveBeenCalled();
    expect(mockOnDelete).not.toHaveBeenCalled();
  });

  it("should ignore shortcuts when typing in an input", () => {
    renderHook(() =>
      useKeyboardShortcuts({
        selectedTaskId: "task-1",
        onMove: mockOnMove,
        onDelete: mockOnDelete,
      })
    );

    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "p", bubbles: true });
      input.dispatchEvent(event);
    });

    expect(mockOnMove).not.toHaveBeenCalled();

    document.body.removeChild(input);
  });

  it("should support uppercase keys", () => {
    renderHook(() =>
      useKeyboardShortcuts({
        selectedTaskId: "task-1",
        onMove: mockOnMove,
        onDelete: mockOnDelete,
      })
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "P" });
      document.dispatchEvent(event);
    });

    expect(mockOnMove).toHaveBeenCalledWith("task-1", "planned");
  });
});
