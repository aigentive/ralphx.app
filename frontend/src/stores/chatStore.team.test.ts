/**
 * chatStore team extension tests — Tests for isTeamActive state and setTeamActive action
 */

import { describe, it, expect, beforeEach } from "vitest";
import { useChatStore, selectIsTeamActive } from "./chatStore";

const CONTEXT_KEY = "task_execution:abc";

describe("chatStore — team extensions", () => {
  beforeEach(() => {
    useChatStore.setState({ isTeamActive: {} });
  });

  describe("setTeamActive", () => {
    it("sets team active for a context", () => {
      useChatStore.getState().setTeamActive(CONTEXT_KEY, true);
      expect(useChatStore.getState().isTeamActive[CONTEXT_KEY]).toBe(true);
    });

    it("removes key when set to false", () => {
      useChatStore.getState().setTeamActive(CONTEXT_KEY, true);
      useChatStore.getState().setTeamActive(CONTEXT_KEY, false);
      expect(useChatStore.getState().isTeamActive[CONTEXT_KEY]).toBeUndefined();
    });

    it("handles multiple contexts independently", () => {
      useChatStore.getState().setTeamActive("ctx-1", true);
      useChatStore.getState().setTeamActive("ctx-2", true);
      useChatStore.getState().setTeamActive("ctx-1", false);

      expect(useChatStore.getState().isTeamActive["ctx-1"]).toBeUndefined();
      expect(useChatStore.getState().isTeamActive["ctx-2"]).toBe(true);
    });
  });

  describe("selectIsTeamActive", () => {
    it("returns false when not set", () => {
      const selector = selectIsTeamActive("nonexistent");
      expect(selector(useChatStore.getState())).toBe(false);
    });

    it("returns true when team is active", () => {
      useChatStore.getState().setTeamActive(CONTEXT_KEY, true);
      const selector = selectIsTeamActive(CONTEXT_KEY);
      expect(selector(useChatStore.getState())).toBe(true);
    });

    it("returns false after team is deactivated", () => {
      useChatStore.getState().setTeamActive(CONTEXT_KEY, true);
      useChatStore.getState().setTeamActive(CONTEXT_KEY, false);
      const selector = selectIsTeamActive(CONTEXT_KEY);
      expect(selector(useChatStore.getState())).toBe(false);
    });
  });
});
