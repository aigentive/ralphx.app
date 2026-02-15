/**
 * splitPaneStore tests — Unit tests for all store actions and selectors
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  useSplitPaneStore,
  selectFocusedPane,
  selectCoordinatorWidth,
  selectIsPrefixKeyActive,
  selectContextKey,
  selectIsActive,
  selectPaneOrder,
  selectPanes,
  selectPaneById,
} from "./splitPaneStore";

// ============================================================================
// Helpers
// ============================================================================

/** Reset store to initial state before each test */
function resetStore() {
  useSplitPaneStore.setState({
    isActive: false,
    focusedPane: null,
    coordinatorWidth: 40,
    isPrefixKeyActive: false,
    contextKey: null,
    paneOrder: [],
    panes: {},
  });
}

function getState() {
  return useSplitPaneStore.getState();
}

// ============================================================================
// Tests
// ============================================================================

describe("splitPaneStore", () => {
  beforeEach(() => {
    resetStore();
  });

  // ── initTeam ────────────────────────────────────────────────────

  describe("initTeam", () => {
    it("activates store and sets pane order", () => {
      getState().initTeam(["coder-1", "coder-2", "reviewer"]);

      expect(getState().isActive).toBe(true);
      expect(getState().paneOrder).toEqual(["coder-1", "coder-2", "reviewer"]);
    });

    it("creates default pane state for each id", () => {
      getState().initTeam(["coder-1", "coder-2"]);

      expect(getState().panes["coder-1"]).toEqual({
        streaming: false,
        minimized: false,
        maximized: false,
        unreadCount: 0,
      });
      expect(getState().panes["coder-2"]).toBeDefined();
    });

    it("focuses the first pane", () => {
      getState().initTeam(["alpha", "beta"]);
      expect(getState().focusedPane).toBe("alpha");
    });

    it("sets focusedPane to null for empty array", () => {
      getState().initTeam([]);
      expect(getState().focusedPane).toBeNull();
      expect(getState().isActive).toBe(true);
    });
  });

  // ── addPane ─────────────────────────────────────────────────────

  describe("addPane", () => {
    it("adds a pane to order and creates state", () => {
      getState().initTeam(["a"]);
      getState().addPane("b");

      expect(getState().paneOrder).toEqual(["a", "b"]);
      expect(getState().panes["b"]).toEqual({
        streaming: false,
        minimized: false,
        maximized: false,
        unreadCount: 0,
      });
    });

    it("does not duplicate if pane already exists", () => {
      getState().initTeam(["a", "b"]);
      getState().addPane("a");

      expect(getState().paneOrder).toEqual(["a", "b"]);
    });
  });

  // ── removePane ──────────────────────────────────────────────────

  describe("removePane", () => {
    it("removes pane from order and deletes state", () => {
      getState().initTeam(["a", "b", "c"]);
      getState().removePane("b");

      expect(getState().paneOrder).toEqual(["a", "c"]);
      expect(getState().panes["b"]).toBeUndefined();
    });

    it("moves focus to first pane when focused pane is removed", () => {
      getState().initTeam(["a", "b", "c"]);
      getState().setFocusedPane("b");
      getState().removePane("b");

      expect(getState().focusedPane).toBe("a");
    });

    it("sets focus to null when last pane removed", () => {
      getState().initTeam(["only"]);
      getState().removePane("only");

      expect(getState().focusedPane).toBeNull();
    });
  });

  // ── clearTeam ───────────────────────────────────────────────────

  describe("clearTeam", () => {
    it("deactivates and clears all panes", () => {
      getState().initTeam(["a", "b"]);
      getState().clearTeam();

      expect(getState().isActive).toBe(false);
      expect(getState().paneOrder).toEqual([]);
      expect(getState().panes).toEqual({});
      expect(getState().focusedPane).toBeNull();
    });
  });

  // ── focusNext ───────────────────────────────────────────────────

  describe("focusNext", () => {
    it("moves focus to next pane", () => {
      getState().initTeam(["a", "b", "c"]);
      // starts at "a"
      getState().focusNext();
      expect(getState().focusedPane).toBe("b");
    });

    it("wraps around to first pane at the end", () => {
      getState().initTeam(["a", "b"]);
      getState().setFocusedPane("b");
      getState().focusNext();

      expect(getState().focusedPane).toBe("a");
    });

    it("clears unread on newly focused pane", () => {
      getState().initTeam(["a", "b"]);
      // generate unread on b
      getState().appendPaneChunk("b", "text");
      expect(getState().panes["b"]!.unreadCount).toBe(1);

      getState().focusNext();
      expect(getState().focusedPane).toBe("b");
      expect(getState().panes["b"]!.unreadCount).toBe(0);
    });

    it("does nothing on empty paneOrder", () => {
      getState().focusNext();
      expect(getState().focusedPane).toBeNull();
    });

    it("focuses first pane when no pane currently focused", () => {
      getState().initTeam(["a", "b"]);
      // Force null focus
      useSplitPaneStore.setState({ focusedPane: null });
      getState().focusNext();
      expect(getState().focusedPane).toBe("a");
    });
  });

  // ── focusPrev ───────────────────────────────────────────────────

  describe("focusPrev", () => {
    it("moves focus to previous pane", () => {
      getState().initTeam(["a", "b", "c"]);
      getState().setFocusedPane("c");
      getState().focusPrev();

      expect(getState().focusedPane).toBe("b");
    });

    it("wraps around to last pane at the beginning", () => {
      getState().initTeam(["a", "b", "c"]);
      // starts at "a"
      getState().focusPrev();

      expect(getState().focusedPane).toBe("c");
    });

    it("clears unread on newly focused pane", () => {
      getState().initTeam(["a", "b"]);
      getState().setFocusedPane("b");
      getState().appendPaneChunk("a", "text");
      expect(getState().panes["a"]!.unreadCount).toBe(1);

      getState().focusPrev();
      expect(getState().focusedPane).toBe("a");
      expect(getState().panes["a"]!.unreadCount).toBe(0);
    });

    it("does nothing on empty paneOrder", () => {
      getState().focusPrev();
      expect(getState().focusedPane).toBeNull();
    });
  });

  // ── setFocusedPane ──────────────────────────────────────────────

  describe("setFocusedPane", () => {
    it("sets focused pane and clears unread", () => {
      getState().initTeam(["a", "b"]);
      getState().appendPaneChunk("b", "text");
      expect(getState().panes["b"]!.unreadCount).toBe(1);

      getState().setFocusedPane("b");
      expect(getState().focusedPane).toBe("b");
      expect(getState().panes["b"]!.unreadCount).toBe(0);
    });

    it("sets to null without error", () => {
      getState().initTeam(["a"]);
      getState().setFocusedPane(null);
      expect(getState().focusedPane).toBeNull();
    });
  });

  // ── minimizePane ────────────────────────────────────────────────

  describe("minimizePane", () => {
    it("sets minimized true and maximized false", () => {
      getState().initTeam(["a"]);
      getState().maximizePane("a"); // pre-set maximized
      getState().minimizePane("a");

      expect(getState().panes["a"]!.minimized).toBe(true);
      expect(getState().panes["a"]!.maximized).toBe(false);
    });

    it("does nothing for nonexistent pane", () => {
      getState().initTeam(["a"]);
      getState().minimizePane("nonexistent");
      // no crash
      expect(getState().panes["a"]!.minimized).toBe(false);
    });
  });

  // ── maximizePane ────────────────────────────────────────────────

  describe("maximizePane", () => {
    it("maximizes target and minimizes all others", () => {
      getState().initTeam(["a", "b", "c"]);
      getState().maximizePane("b");

      expect(getState().panes["b"]!.maximized).toBe(true);
      expect(getState().panes["b"]!.minimized).toBe(false);
      expect(getState().panes["a"]!.minimized).toBe(true);
      expect(getState().panes["a"]!.maximized).toBe(false);
      expect(getState().panes["c"]!.minimized).toBe(true);
      expect(getState().panes["c"]!.maximized).toBe(false);
    });
  });

  // ── restorePane ─────────────────────────────────────────────────

  describe("restorePane", () => {
    it("clears both minimized and maximized", () => {
      getState().initTeam(["a"]);
      getState().maximizePane("a");
      getState().restorePane("a");

      expect(getState().panes["a"]!.minimized).toBe(false);
      expect(getState().panes["a"]!.maximized).toBe(false);
    });

    it("does nothing for nonexistent pane", () => {
      getState().initTeam(["a"]);
      getState().restorePane("nonexistent");
      expect(getState().panes["a"]!.minimized).toBe(false);
    });
  });

  // ── resetPaneSizes ──────────────────────────────────────────────

  describe("resetPaneSizes", () => {
    it("restores all panes and resets coordinator width", () => {
      getState().initTeam(["a", "b"]);
      getState().maximizePane("a");
      getState().setCoordinatorWidth(60);

      getState().resetPaneSizes();

      expect(getState().panes["a"]!.minimized).toBe(false);
      expect(getState().panes["a"]!.maximized).toBe(false);
      expect(getState().panes["b"]!.minimized).toBe(false);
      expect(getState().coordinatorWidth).toBe(40);
    });
  });

  // ── setCoordinatorWidth ─────────────────────────────────────────

  describe("setCoordinatorWidth", () => {
    it("sets width within range", () => {
      getState().setCoordinatorWidth(55);
      expect(getState().coordinatorWidth).toBe(55);
    });

    it("clamps to minimum 20%", () => {
      getState().setCoordinatorWidth(5);
      expect(getState().coordinatorWidth).toBe(20);
    });

    it("clamps to maximum 80%", () => {
      getState().setCoordinatorWidth(95);
      expect(getState().coordinatorWidth).toBe(80);
    });
  });

  // ── appendPaneChunk ─────────────────────────────────────────────

  describe("appendPaneChunk", () => {
    it("sets streaming true and increments unread for unfocused pane", () => {
      getState().initTeam(["a", "b"]);
      // "a" is focused
      getState().appendPaneChunk("b", "chunk text");

      expect(getState().panes["b"]!.streaming).toBe(true);
      expect(getState().panes["b"]!.unreadCount).toBe(1);
    });

    it("increments unread multiple times", () => {
      getState().initTeam(["a", "b"]);
      getState().appendPaneChunk("b", "1");
      getState().appendPaneChunk("b", "2");
      getState().appendPaneChunk("b", "3");

      expect(getState().panes["b"]!.unreadCount).toBe(3);
    });

    it("does NOT increment unread for focused pane", () => {
      getState().initTeam(["a", "b"]);
      getState().appendPaneChunk("a", "chunk");

      expect(getState().panes["a"]!.streaming).toBe(true);
      expect(getState().panes["a"]!.unreadCount).toBe(0);
    });

    it("does nothing for nonexistent pane", () => {
      getState().initTeam(["a"]);
      getState().appendPaneChunk("ghost", "data");
      expect(Object.keys(getState().panes)).toEqual(["a"]);
    });
  });

  // ── clearPaneStream ─────────────────────────────────────────────

  describe("clearPaneStream", () => {
    it("sets streaming false", () => {
      getState().initTeam(["a"]);
      getState().appendPaneChunk("a", "data");
      expect(getState().panes["a"]!.streaming).toBe(true);

      getState().clearPaneStream("a");
      expect(getState().panes["a"]!.streaming).toBe(false);
    });
  });

  // ── addPaneToolCall ─────────────────────────────────────────────

  describe("addPaneToolCall", () => {
    it("increments unread for unfocused pane", () => {
      getState().initTeam(["a", "b"]);
      getState().addPaneToolCall("b", { tool: "edit" });

      expect(getState().panes["b"]!.unreadCount).toBe(1);
    });

    it("does NOT increment unread for focused pane", () => {
      getState().initTeam(["a"]);
      getState().addPaneToolCall("a", { tool: "edit" });

      expect(getState().panes["a"]!.unreadCount).toBe(0);
    });
  });

  // ── addPaneMessage ──────────────────────────────────────────────

  describe("addPaneMessage", () => {
    it("increments unread for unfocused pane", () => {
      getState().initTeam(["a", "b"]);
      getState().addPaneMessage("b", { text: "hi" });

      expect(getState().panes["b"]!.unreadCount).toBe(1);
    });

    it("does NOT increment unread for focused pane", () => {
      getState().initTeam(["a"]);
      getState().addPaneMessage("a", { text: "hi" });

      expect(getState().panes["a"]!.unreadCount).toBe(0);
    });
  });

  // ── setPrefixKeyActive ──────────────────────────────────────────

  describe("setPrefixKeyActive", () => {
    it("sets prefix key active state", () => {
      getState().setPrefixKeyActive(true);
      expect(getState().isPrefixKeyActive).toBe(true);

      getState().setPrefixKeyActive(false);
      expect(getState().isPrefixKeyActive).toBe(false);
    });
  });

  // ── setContextKey ───────────────────────────────────────────────

  describe("setContextKey", () => {
    it("sets and clears context key", () => {
      getState().setContextKey("task_execution:abc");
      expect(getState().contextKey).toBe("task_execution:abc");

      getState().setContextKey(null);
      expect(getState().contextKey).toBeNull();
    });
  });

  // ── reset ───────────────────────────────────────────────────────

  describe("reset", () => {
    it("resets entire store to initial state", () => {
      getState().initTeam(["a", "b"]);
      getState().setCoordinatorWidth(60);
      getState().setPrefixKeyActive(true);
      getState().setContextKey("test");

      getState().reset();

      expect(getState().isActive).toBe(false);
      expect(getState().focusedPane).toBeNull();
      expect(getState().coordinatorWidth).toBe(40);
      expect(getState().isPrefixKeyActive).toBe(false);
      expect(getState().contextKey).toBeNull();
      expect(getState().paneOrder).toEqual([]);
      expect(getState().panes).toEqual({});
    });
  });

  // ── Selectors ───────────────────────────────────────────────────

  describe("selectors", () => {
    it("selectFocusedPane returns focused pane", () => {
      getState().initTeam(["a"]);
      expect(selectFocusedPane(getState())).toBe("a");
    });

    it("selectCoordinatorWidth returns width", () => {
      getState().setCoordinatorWidth(55);
      expect(selectCoordinatorWidth(getState())).toBe(55);
    });

    it("selectIsPrefixKeyActive returns prefix state", () => {
      expect(selectIsPrefixKeyActive(getState())).toBe(false);
      getState().setPrefixKeyActive(true);
      expect(selectIsPrefixKeyActive(getState())).toBe(true);
    });

    it("selectContextKey returns context key", () => {
      expect(selectContextKey(getState())).toBeNull();
      getState().setContextKey("task:1");
      expect(selectContextKey(getState())).toBe("task:1");
    });

    it("selectIsActive returns active state", () => {
      expect(selectIsActive(getState())).toBe(false);
      getState().initTeam(["a"]);
      expect(selectIsActive(getState())).toBe(true);
    });

    it("selectPaneOrder returns pane order", () => {
      getState().initTeam(["x", "y"]);
      expect(selectPaneOrder(getState())).toEqual(["x", "y"]);
    });

    it("selectPanes returns all pane states", () => {
      getState().initTeam(["a"]);
      const panes = selectPanes(getState());
      expect(panes["a"]).toBeDefined();
      expect(panes["a"]!.streaming).toBe(false);
    });

    it("selectPaneById returns pane or null", () => {
      getState().initTeam(["a"]);
      expect(selectPaneById("a")(getState())).toEqual({
        streaming: false,
        minimized: false,
        maximized: false,
        unreadCount: 0,
      });
      expect(selectPaneById("nonexistent")(getState())).toBeNull();
    });
  });
});
