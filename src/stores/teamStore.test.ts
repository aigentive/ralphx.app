/**
 * teamStore tests — Unit tests for all store actions and selectors
 */

import { describe, it, expect, beforeEach } from "vitest";
import { useTeamStore, selectTeammates, selectTeamMessages, selectTeammateByName, selectActiveTeam, selectIsTeamActive } from "./teamStore";
import type { TeammateState } from "./teamStore";

// ============================================================================
// Test Data
// ============================================================================

const CONTEXT_KEY = "task_execution:abc";

function makeTeammate(overrides: Partial<TeammateState> = {}): TeammateState {
  return {
    name: "coder-1",
    color: "#3b82f6",
    model: "sonnet",
    roleDescription: "Auth middleware",
    status: "spawning",
    currentActivity: null,
    tokensUsed: 0,
    estimatedCostUsd: 0,
    streamingText: "",
    ...overrides,
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("teamStore", () => {
  beforeEach(() => {
    // Reset store to initial state
    useTeamStore.setState({ activeTeams: {} });
  });

  // ── createTeam ──────────────────────────────────────────────────

  describe("createTeam", () => {
    it("creates a new team for a context key", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "task-abc", "lead-agent");

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY];
      expect(team).toBeDefined();
      expect(team!.teamName).toBe("task-abc");
      expect(team!.leadName).toBe("lead-agent");
      expect(team!.teammates).toEqual({});
      expect(team!.messages).toEqual([]);
      expect(team!.totalTokens).toBe(0);
      expect(team!.totalEstimatedCostUsd).toBe(0);
      expect(team!.createdAt).toBeTruthy();
    });

    it("overwrites existing team when called again", () => {
      const { createTeam, addTeammate } = useTeamStore.getState();
      createTeam(CONTEXT_KEY, "old-team", "old-lead");
      addTeammate(CONTEXT_KEY, makeTeammate());

      createTeam(CONTEXT_KEY, "new-team", "new-lead");

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY];
      expect(team!.teamName).toBe("new-team");
      expect(team!.teammates).toEqual({});
    });
  });

  // ── addTeammate ─────────────────────────────────────────────────

  describe("addTeammate", () => {
    it("adds a teammate to an existing team", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      const mate = makeTeammate({ name: "coder-1" });
      useTeamStore.getState().addTeammate(CONTEXT_KEY, mate);

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY]!;
      expect(team.teammates["coder-1"]).toBeDefined();
      expect(team.teammates["coder-1"]!.color).toBe("#3b82f6");
    });

    it("does nothing if team does not exist", () => {
      useTeamStore.getState().addTeammate("nonexistent", makeTeammate());
      expect(useTeamStore.getState().activeTeams["nonexistent"]).toBeUndefined();
    });

    it("adds multiple teammates", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-2", color: "#10b981" }));

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY]!;
      expect(Object.keys(team.teammates)).toHaveLength(2);
    });
  });

  // ── updateTeammateStatus ────────────────────────────────────────

  describe("updateTeammateStatus", () => {
    it("updates teammate status", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));

      useTeamStore.getState().updateTeammateStatus(CONTEXT_KEY, "coder-1", "running");

      const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["coder-1"]!;
      expect(mate.status).toBe("running");
    });

    it("updates teammate status with activity", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));

      useTeamStore.getState().updateTeammateStatus(CONTEXT_KEY, "coder-1", "running", "Writing auth.ts");

      const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["coder-1"]!;
      expect(mate.status).toBe("running");
      expect(mate.currentActivity).toBe("Writing auth.ts");
    });

    it("does nothing for nonexistent teammate", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      // Should not throw
      useTeamStore.getState().updateTeammateStatus(CONTEXT_KEY, "ghost", "running");
    });
  });

  // ── appendTeammateChunk ─────────────────────────────────────────

  describe("appendTeammateChunk", () => {
    it("appends streaming text", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));

      useTeamStore.getState().appendTeammateChunk(CONTEXT_KEY, "coder-1", "Hello ");
      useTeamStore.getState().appendTeammateChunk(CONTEXT_KEY, "coder-1", "World");

      const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["coder-1"]!;
      expect(mate.streamingText).toBe("Hello World");
    });
  });

  // ── clearTeammateStream ─────────────────────────────────────────

  describe("clearTeammateStream", () => {
    it("clears streaming text", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));
      useTeamStore.getState().appendTeammateChunk(CONTEXT_KEY, "coder-1", "some text");

      useTeamStore.getState().clearTeammateStream(CONTEXT_KEY, "coder-1");

      const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["coder-1"]!;
      expect(mate.streamingText).toBe("");
    });
  });

  // ── updateTeammateCost ──────────────────────────────────────────

  describe("updateTeammateCost", () => {
    it("updates teammate cost and adjusts team totals", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));

      useTeamStore.getState().updateTeammateCost(CONTEXT_KEY, "coder-1", 50000, 0.30);

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY]!;
      expect(team.teammates["coder-1"]!.tokensUsed).toBe(50000);
      expect(team.teammates["coder-1"]!.estimatedCostUsd).toBe(0.30);
      expect(team.totalTokens).toBe(50000);
      expect(team.totalEstimatedCostUsd).toBeCloseTo(0.30);
    });

    it("handles incremental cost updates correctly", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-2" }));

      // First teammate gets 50K tokens
      useTeamStore.getState().updateTeammateCost(CONTEXT_KEY, "coder-1", 50000, 0.30);
      // Second teammate gets 80K tokens
      useTeamStore.getState().updateTeammateCost(CONTEXT_KEY, "coder-2", 80000, 0.48);
      // First teammate updates to 85K (increment of 35K)
      useTeamStore.getState().updateTeammateCost(CONTEXT_KEY, "coder-1", 85000, 0.51);

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY]!;
      expect(team.totalTokens).toBe(165000); // 85K + 80K
      expect(team.totalEstimatedCostUsd).toBeCloseTo(0.99); // 0.51 + 0.48
    });
  });

  // ── addTeamMessage ──────────────────────────────────────────────

  describe("addTeamMessage", () => {
    it("adds a message to the team", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");

      useTeamStore.getState().addTeamMessage(CONTEXT_KEY, {
        id: "msg-1",
        from: "coder-1",
        to: "coder-2",
        content: "Use AppResult<T>",
        timestamp: "2026-02-15T10:00:00Z",
      });

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY]!;
      expect(team.messages).toHaveLength(1);
      expect(team.messages[0]!.from).toBe("coder-1");
    });

    it("caps messages at 200", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");

      // Add 210 messages
      for (let i = 0; i < 210; i++) {
        useTeamStore.getState().addTeamMessage(CONTEXT_KEY, {
          id: `msg-${i}`,
          from: "coder-1",
          to: "coder-2",
          content: `Message ${i}`,
          timestamp: new Date().toISOString(),
        });
      }

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY]!;
      expect(team.messages).toHaveLength(200);
      // Should keep the most recent
      expect(team.messages[199]!.id).toBe("msg-209");
    });
  });

  // ── removeTeammate ──────────────────────────────────────────────

  describe("removeTeammate", () => {
    it("removes a teammate from the team", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-2" }));

      useTeamStore.getState().removeTeammate(CONTEXT_KEY, "coder-1");

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY]!;
      expect(team.teammates["coder-1"]).toBeUndefined();
      expect(team.teammates["coder-2"]).toBeDefined();
    });
  });

  // ── disbandTeam ─────────────────────────────────────────────────

  describe("disbandTeam", () => {
    it("removes the team entirely", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));

      useTeamStore.getState().disbandTeam(CONTEXT_KEY);

      expect(useTeamStore.getState().activeTeams[CONTEXT_KEY]).toBeUndefined();
    });
  });

  // ── getTeammates ────────────────────────────────────────────────

  describe("getTeammates", () => {
    it("returns empty array when no team exists", () => {
      const result = useTeamStore.getState().getTeammates("nonexistent");
      expect(result).toEqual([]);
    });

    it("returns teammates array", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-2" }));

      const result = useTeamStore.getState().getTeammates(CONTEXT_KEY);
      expect(result).toHaveLength(2);
    });
  });

  // ── Selectors ───────────────────────────────────────────────────

  describe("selectors", () => {
    it("selectIsTeamActive returns false when no team", () => {
      const selector = selectIsTeamActive("nonexistent");
      expect(selector(useTeamStore.getState())).toBe(false);
    });

    it("selectIsTeamActive returns true when team exists", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      const selector = selectIsTeamActive(CONTEXT_KEY);
      expect(selector(useTeamStore.getState())).toBe(true);
    });

    it("selectTeammates returns empty array for nonexistent context", () => {
      const selector = selectTeammates("nonexistent");
      const result = selector(useTeamStore.getState());
      expect(result).toEqual([]);
      // Should return the same reference (stable empty array)
      expect(result).toBe(selector(useTeamStore.getState()));
    });

    it("selectTeammates returns teammates", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));
      const selector = selectTeammates(CONTEXT_KEY);
      const result = selector(useTeamStore.getState());
      expect(result).toHaveLength(1);
      expect(result[0]!.name).toBe("coder-1");
    });

    it("selectTeamMessages returns empty array for nonexistent context", () => {
      const selector = selectTeamMessages("nonexistent");
      const result = selector(useTeamStore.getState());
      expect(result).toEqual([]);
    });

    it("selectTeamMessages returns messages", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeamMessage(CONTEXT_KEY, {
        id: "msg-1", from: "a", to: "b", content: "hi", timestamp: "2026-01-01T00:00:00Z",
      });
      const selector = selectTeamMessages(CONTEXT_KEY);
      expect(selector(useTeamStore.getState())).toHaveLength(1);
    });

    it("selectTeammateByName returns null for nonexistent", () => {
      const selector = selectTeammateByName("nonexistent", "ghost");
      expect(selector(useTeamStore.getState())).toBeNull();
    });

    it("selectTeammateByName returns the teammate", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      useTeamStore.getState().addTeammate(CONTEXT_KEY, makeTeammate({ name: "coder-1" }));
      const selector = selectTeammateByName(CONTEXT_KEY, "coder-1");
      const result = selector(useTeamStore.getState());
      expect(result).not.toBeNull();
      expect(result!.name).toBe("coder-1");
    });

    it("selectActiveTeam returns null for nonexistent", () => {
      const selector = selectActiveTeam("nonexistent");
      expect(selector(useTeamStore.getState())).toBeNull();
    });

    it("selectActiveTeam returns team", () => {
      useTeamStore.getState().createTeam(CONTEXT_KEY, "team", "lead");
      const selector = selectActiveTeam(CONTEXT_KEY);
      const result = selector(useTeamStore.getState());
      expect(result).not.toBeNull();
      expect(result!.teamName).toBe("team");
    });
  });
});
