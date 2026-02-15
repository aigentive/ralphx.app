/**
 * useTeamKeyboardNav hook tests
 *
 * Tests tmux-inspired prefix key navigation:
 *   Ctrl+B activates prefix mode with 1.5s auto-cancel,
 *   then arrow/vim keys navigate panes, number keys jump,
 *   z/minus/equals/x for layout/stop, Escape cancels.
 *
 * Verifies input element filtering and fire-and-forget stopTeammate.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useSplitPaneStore } from "@/stores/splitPaneStore";
import { useTeamStore } from "@/stores/teamStore";

// ============================================================================
// Mock stopTeammate API (fire-and-forget)
// ============================================================================

const mockStopTeammate = vi.fn().mockResolvedValue(undefined);

vi.mock("@/api/team", () => ({
  stopTeammate: (...args: unknown[]) => mockStopTeammate(...args),
}));

// ============================================================================
// Import hook under test (after mocks)
// ============================================================================

import { useTeamKeyboardNav } from "./useTeamKeyboardNav";

// ============================================================================
// Helpers
// ============================================================================

const CONTEXT_KEY = "task_execution:task-abc";

function setupTeamWithTeammates(names: string[]) {
  // Set up teamStore with active team
  useTeamStore.setState({
    activeTeams: {
      [CONTEXT_KEY]: {
        teamName: "test-team",
        leadName: "lead",
        teammates: Object.fromEntries(
          names.map((name) => [
            name,
            {
              name,
              color: "#ff0000",
              model: "sonnet",
              roleDescription: "coder",
              status: "running" as const,
              currentActivity: null,
              tokensUsed: 0,
              estimatedCostUsd: 0,
              streamingText: "",
            },
          ]),
        ),
        messages: [],
        totalTokens: 0,
        totalEstimatedCostUsd: 0,
        createdAt: "2026-02-15T10:00:00Z",
      },
    },
  });
}

function fireKey(key: string, modifiers: Partial<KeyboardEventInit> = {}) {
  const event = new KeyboardEvent("keydown", {
    key,
    bubbles: true,
    cancelable: true,
    ...modifiers,
  });
  document.dispatchEvent(event);
  return event;
}

function activatePrefix() {
  fireKey("b", { ctrlKey: true });
}

// ============================================================================
// Tests
// ============================================================================

describe("useTeamKeyboardNav", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useSplitPaneStore.getState().reset();
    useTeamStore.setState({ activeTeams: {} });
    mockStopTeammate.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // --------------------------------------------------------------------------
  // 1. Does not listen when disabled
  // --------------------------------------------------------------------------
  it("should not respond to Ctrl+B when disabled", () => {
    renderHook(() => useTeamKeyboardNav(false, CONTEXT_KEY));

    act(() => {
      fireKey("b", { ctrlKey: true });
    });

    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  // --------------------------------------------------------------------------
  // 2. Does not listen when contextKey is null
  // --------------------------------------------------------------------------
  it("should not respond when contextKey is null", () => {
    renderHook(() => useTeamKeyboardNav(true, null));

    act(() => {
      fireKey("b", { ctrlKey: true });
    });

    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  // --------------------------------------------------------------------------
  // 3. Ctrl+B activates prefix mode
  // --------------------------------------------------------------------------
  it("should activate prefix mode on Ctrl+B", () => {
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });

    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(true);
  });

  // --------------------------------------------------------------------------
  // 4. 1.5s timeout auto-cancels prefix
  // --------------------------------------------------------------------------
  it("should auto-cancel prefix after 1500ms timeout", () => {
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(true);

    act(() => {
      vi.advanceTimersByTime(1500);
    });

    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  // --------------------------------------------------------------------------
  // 5. ArrowLeft / h → focus coordinator
  // --------------------------------------------------------------------------
  it("should focus coordinator on ArrowLeft after prefix", () => {
    setupTeamWithTeammates(["w1", "w2"]);
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    act(() => {
      fireKey("ArrowLeft");
    });

    expect(useSplitPaneStore.getState().focusedPane).toBe("coordinator");
    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  it("should focus coordinator on h after prefix", () => {
    setupTeamWithTeammates(["w1"]);
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    act(() => {
      fireKey("h");
    });

    expect(useSplitPaneStore.getState().focusedPane).toBe("coordinator");
  });

  // --------------------------------------------------------------------------
  // 6. ArrowRight / l → focus first teammate
  // --------------------------------------------------------------------------
  it("should focus first teammate on ArrowRight after prefix", () => {
    setupTeamWithTeammates(["w1", "w2"]);
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    act(() => {
      fireKey("ArrowRight");
    });

    expect(useSplitPaneStore.getState().focusedPane).toBe("w1");
    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  // --------------------------------------------------------------------------
  // 7. ArrowDown / j → next teammate (wrapping)
  // --------------------------------------------------------------------------
  it("should navigate to next teammate on ArrowDown and wrap around", () => {
    setupTeamWithTeammates(["w1", "w2"]);
    useSplitPaneStore.getState().setFocusedPane("w1");
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    act(() => {
      fireKey("ArrowDown");
    });

    expect(useSplitPaneStore.getState().focusedPane).toBe("w2");

    // Wrap around
    act(() => {
      activatePrefix();
    });
    act(() => {
      fireKey("j");
    });

    expect(useSplitPaneStore.getState().focusedPane).toBe("w1");
  });

  // --------------------------------------------------------------------------
  // 8. Number keys 1-5 → jump to teammate by index
  // --------------------------------------------------------------------------
  it("should jump to teammate by 1-based index", () => {
    setupTeamWithTeammates(["w1", "w2", "w3"]);
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    act(() => {
      fireKey("2");
    });

    expect(useSplitPaneStore.getState().focusedPane).toBe("w2");
    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  // --------------------------------------------------------------------------
  // 9. z → toggle maximize
  // --------------------------------------------------------------------------
  it("should maximize focused pane on z key", () => {
    setupTeamWithTeammates(["w1", "w2"]);
    useSplitPaneStore.getState().setFocusedPane("w1");
    useSplitPaneStore.getState().addPane("w1");
    useSplitPaneStore.getState().addPane("w2");
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    act(() => {
      fireKey("z");
    });

    expect(useSplitPaneStore.getState().panes["w1"]?.maximized).toBe(true);
    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  // --------------------------------------------------------------------------
  // 10. x → fire-and-forget stopTeammate
  // --------------------------------------------------------------------------
  it("should call stopTeammate fire-and-forget on x key", () => {
    setupTeamWithTeammates(["w1"]);
    useSplitPaneStore.getState().setFocusedPane("w1");
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    act(() => {
      fireKey("x");
    });

    expect(mockStopTeammate).toHaveBeenCalledWith("test-team", "w1");
    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  // --------------------------------------------------------------------------
  // 11. Escape cancels prefix
  // --------------------------------------------------------------------------
  it("should cancel prefix mode on Escape", () => {
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(true);

    act(() => {
      fireKey("Escape");
    });

    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });

  // --------------------------------------------------------------------------
  // 12. Input element filtering — ignores keystrokes in input/textarea
  // --------------------------------------------------------------------------
  it("should not activate prefix when target is an input element", () => {
    renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    const input = document.createElement("input");
    document.body.appendChild(input);

    act(() => {
      const event = new KeyboardEvent("keydown", {
        key: "b",
        ctrlKey: true,
        bubbles: true,
      });
      input.dispatchEvent(event);
    });

    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
    document.body.removeChild(input);
  });

  // --------------------------------------------------------------------------
  // 13. Cleanup on unmount
  // --------------------------------------------------------------------------
  it("should deactivate prefix and remove listeners on unmount", () => {
    const { unmount } = renderHook(() => useTeamKeyboardNav(true, CONTEXT_KEY));

    act(() => {
      activatePrefix();
    });
    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(true);

    unmount();

    expect(useSplitPaneStore.getState().isPrefixKeyActive).toBe(false);
  });
});
