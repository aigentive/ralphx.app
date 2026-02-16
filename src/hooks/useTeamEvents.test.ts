/**
 * useTeamEvents hook tests
 *
 * Tests the two-effect split architecture:
 *   Effect 1 (always active): team:created + team:disbanded
 *   Effect 2 (gated by isTeamActive): remaining 7 event types
 *
 * Verifies matchKey filtering, store action routing, token summing,
 * agent:chunk teammate routing, and cleanup on unmount.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useTeamStore } from "@/stores/teamStore";
import { useChatStore } from "@/stores/chatStore";

// ============================================================================
// Mock infrastructure — capture EventBus subscriptions
// ============================================================================

const subscriptions = new Map<string, ((...args: unknown[]) => void)[]>();

function fireEvent<T>(event: string, payload: T) {
  const handlers = subscriptions.get(event);
  if (handlers) {
    for (const handler of handlers) {
      handler(payload);
    }
  }
}

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: (event: string, handler: (...args: unknown[]) => void) => {
      if (!subscriptions.has(event)) subscriptions.set(event, []);
      subscriptions.get(event)!.push(handler);
      return () => {
        const handlers = subscriptions.get(event);
        if (handlers) {
          const idx = handlers.indexOf(handler);
          if (idx >= 0) handlers.splice(idx, 1);
        }
      };
    },
  }),
}));

// buildStoreKey: match the real implementation for "task_execution" prefix
vi.mock("@/lib/chat-context-registry", () => ({
  buildStoreKey: (contextType: string, contextId: string) => {
    const prefixes: Record<string, string> = {
      task_execution: "task_execution",
      ideation: "session",
      task: "task",
      project: "project",
      review: "review",
      merge: "merge",
    };
    return `${prefixes[contextType] ?? contextType}:${contextId}`;
  },
}));

// ============================================================================
// Import hook under test (after mocks)
// ============================================================================

import { useTeamEvents } from "./useTeamEvents";

// ============================================================================
// Helpers
// ============================================================================

const CONTEXT_KEY = "task_execution:task-abc";
const CONTEXT_TYPE = "task_execution";
const CONTEXT_ID = "task-abc";

function makePayload(overrides?: Record<string, unknown>) {
  return {
    context_type: CONTEXT_TYPE,
    context_id: CONTEXT_ID,
    ...overrides,
  };
}

// ============================================================================
// Tests
// ============================================================================

describe("useTeamEvents", () => {
  beforeEach(() => {
    subscriptions.clear();
    // Reset stores to initial state
    useTeamStore.setState({ activeTeams: {} });
    useChatStore.getState().setTeamActive(CONTEXT_KEY, false);
  });

  afterEach(() => {
    // Clean up any remaining teams
    useTeamStore.setState({ activeTeams: {} });
  });

  // --------------------------------------------------------------------------
  // 1. No subscriptions when contextKey is null
  // --------------------------------------------------------------------------
  it("should not subscribe to any events when contextKey is null", () => {
    renderHook(() => useTeamEvents(null));

    expect(subscriptions.size).toBe(0);
  });

  // --------------------------------------------------------------------------
  // 2. Effect 1: team:created
  // --------------------------------------------------------------------------
  describe("Effect 1: team:created", () => {
    it("should create team in teamStore and set chatStore team active on matching event", () => {
      renderHook(() => useTeamEvents(CONTEXT_KEY));

      act(() => {
        fireEvent("team:created", {
          ...makePayload(),
          team_name: "my-team",
          lead_name: "team-lead",
        });
      });

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY];
      expect(team).toBeDefined();
      expect(team!.teamName).toBe("my-team");
      expect(team!.leadName).toBe("team-lead");
      expect(useChatStore.getState().isTeamActive[CONTEXT_KEY]).toBe(true);
    });

    it("should default lead_name to team_name when lead_name is absent", () => {
      renderHook(() => useTeamEvents(CONTEXT_KEY));

      act(() => {
        fireEvent("team:created", {
          ...makePayload(),
          team_name: "solo-team",
        });
      });

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY];
      expect(team!.leadName).toBe("solo-team");
    });

    it("should ignore team:created with non-matching context", () => {
      renderHook(() => useTeamEvents(CONTEXT_KEY));

      act(() => {
        fireEvent("team:created", {
          context_type: "task_execution",
          context_id: "other-task",
          team_name: "other-team",
        });
      });

      expect(useTeamStore.getState().activeTeams[CONTEXT_KEY]).toBeUndefined();
    });
  });

  // --------------------------------------------------------------------------
  // 3. Effect 1: team:disbanded
  // --------------------------------------------------------------------------
  describe("Effect 1: team:disbanded", () => {
    it("should mark team as historical but keep data and isTeamActive", () => {
      renderHook(() => useTeamEvents(CONTEXT_KEY));

      // First create a team
      act(() => {
        fireEvent("team:created", {
          ...makePayload(),
          team_name: "my-team",
          lead_name: "lead",
        });
      });
      expect(useTeamStore.getState().activeTeams[CONTEXT_KEY]).toBeDefined();

      // Then disband — team should remain as historical, not deleted
      act(() => {
        fireEvent("team:disbanded", makePayload({ team_name: "my-team" }));
      });

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY];
      expect(team).toBeDefined();
      expect(team?.isHistorical).toBe(true);
      // isTeamActive stays true so TeamActivityPanel keeps rendering
      expect(useChatStore.getState().isTeamActive[CONTEXT_KEY]).toBeTruthy();
    });
  });

  // --------------------------------------------------------------------------
  // 4. Effect 2 gating: events only fire when team is active
  // --------------------------------------------------------------------------
  describe("Effect 2 gating", () => {
    it("should not subscribe to teammate_spawned before team is active", () => {
      renderHook(() => useTeamEvents(CONTEXT_KEY));

      // No team created yet — fire teammate_spawned
      act(() => {
        fireEvent("team:teammate_spawned", {
          ...makePayload(),
          team_name: "my-team",
          teammate_name: "worker-1",
          color: "#ff0000",
          model: "sonnet",
          role: "coder",
        });
      });

      // Team doesn't exist, so no teammates
      expect(useTeamStore.getState().activeTeams[CONTEXT_KEY]).toBeUndefined();
    });

    it("should process teammate_spawned after team is created", () => {
      renderHook(() => useTeamEvents(CONTEXT_KEY));

      // Create team first (activates Effect 2)
      act(() => {
        fireEvent("team:created", {
          ...makePayload(),
          team_name: "my-team",
          lead_name: "lead",
        });
      });

      // Now spawn teammate
      act(() => {
        fireEvent("team:teammate_spawned", {
          ...makePayload(),
          team_name: "my-team",
          teammate_name: "worker-1",
          color: "#ff0000",
          model: "sonnet",
          role: "coder",
        });
      });

      const team = useTeamStore.getState().activeTeams[CONTEXT_KEY];
      expect(team!.teammates["worker-1"]).toBeDefined();
      expect(team!.teammates["worker-1"]!.status).toBe("spawning");
      expect(team!.teammates["worker-1"]!.model).toBe("sonnet");
      expect(team!.teammates["worker-1"]!.roleDescription).toBe("coder");
    });
  });

  // --------------------------------------------------------------------------
  // 5. agent:run_started → teammate running
  // --------------------------------------------------------------------------
  it("should set teammate status to running on agent:run_started with teammate_name", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    // Setup: create team + add teammate
    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });
    act(() => {
      fireEvent("team:teammate_spawned", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        color: "#f00", model: "sonnet", role: "coder",
      });
    });

    act(() => {
      fireEvent("agent:run_started", {
        ...makePayload(),
        teammate_name: "w1",
      });
    });

    const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"];
    expect(mate!.status).toBe("running");
  });

  // --------------------------------------------------------------------------
  // 6. agent:run_completed → teammate idle + clear stream
  // --------------------------------------------------------------------------
  it("should set teammate to idle and clear stream on agent:run_completed", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });
    act(() => {
      fireEvent("team:teammate_spawned", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        color: "#f00", model: "sonnet", role: "coder",
      });
    });

    // Simulate some streaming text
    act(() => {
      fireEvent("agent:chunk", {
        ...makePayload(), teammate_name: "w1", text: "hello",
      });
    });
    expect(useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"]!.streamingText).toBe("hello");

    act(() => {
      fireEvent("agent:run_completed", {
        ...makePayload(), teammate_name: "w1",
      });
    });

    const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"];
    expect(mate!.status).toBe("idle");
    expect(mate!.streamingText).toBe("");
  });

  // --------------------------------------------------------------------------
  // 7. team:teammate_idle with last_activity
  // --------------------------------------------------------------------------
  it("should update teammate to idle with activity on team:teammate_idle", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });
    act(() => {
      fireEvent("team:teammate_spawned", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        color: "#f00", model: "sonnet", role: "coder",
      });
    });

    act(() => {
      fireEvent("team:teammate_idle", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        last_activity: "Completed code review",
      });
    });

    const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"];
    expect(mate!.status).toBe("idle");
    expect(mate!.currentActivity).toBe("Completed code review");
  });

  // --------------------------------------------------------------------------
  // 8. team:message → addTeamMessage
  // --------------------------------------------------------------------------
  it("should add message to team store on team:message", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });

    act(() => {
      fireEvent("team:message", {
        ...makePayload(),
        team_name: "t",
        message_id: "msg-1",
        sender: "worker-1",
        recipient: "lead",
        content: "Task done",
        message_type: "direct",
        timestamp: "2026-02-15T10:00:00Z",
      });
    });

    const messages = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.messages;
    expect(messages).toHaveLength(1);
    expect(messages[0]!.from).toBe("worker-1");
    expect(messages[0]!.to).toBe("lead");
    expect(messages[0]!.content).toBe("Task done");
  });

  // --------------------------------------------------------------------------
  // 9. team:cost_update → token summing
  // --------------------------------------------------------------------------
  it("should sum input_tokens + output_tokens and pass to updateTeammateCost", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });
    act(() => {
      fireEvent("team:teammate_spawned", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        color: "#f00", model: "sonnet", role: "coder",
      });
    });

    act(() => {
      fireEvent("team:cost_update", {
        ...makePayload(),
        team_name: "t",
        teammate_name: "w1",
        input_tokens: 1000,
        output_tokens: 500,
        estimated_usd: 0.05,
      });
    });

    const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"];
    expect(mate!.tokensUsed).toBe(1500); // input + output
    expect(mate!.estimatedCostUsd).toBe(0.05);
  });

  // --------------------------------------------------------------------------
  // 10. team:teammate_shutdown
  // --------------------------------------------------------------------------
  it("should set teammate status to shutdown on team:teammate_shutdown", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });
    act(() => {
      fireEvent("team:teammate_spawned", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        color: "#f00", model: "sonnet", role: "coder",
      });
    });

    act(() => {
      fireEvent("team:teammate_shutdown", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
      });
    });

    const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"];
    expect(mate!.status).toBe("shutdown");
  });

  // --------------------------------------------------------------------------
  // 11. agent:chunk → teammate streaming text
  // --------------------------------------------------------------------------
  it("should append streaming text on agent:chunk with teammate_name", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });
    act(() => {
      fireEvent("team:teammate_spawned", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        color: "#f00", model: "sonnet", role: "coder",
      });
    });

    act(() => {
      fireEvent("agent:chunk", { ...makePayload(), teammate_name: "w1", text: "Hello " });
    });
    act(() => {
      fireEvent("agent:chunk", { ...makePayload(), teammate_name: "w1", text: "world" });
    });

    const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"];
    expect(mate!.streamingText).toBe("Hello world");
  });

  // --------------------------------------------------------------------------
  // 12. agent:chunk without teammate_name is ignored
  // --------------------------------------------------------------------------
  it("should ignore agent:chunk without teammate_name", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });
    act(() => {
      fireEvent("team:teammate_spawned", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        color: "#f00", model: "sonnet", role: "coder",
      });
    });

    act(() => {
      fireEvent("agent:chunk", { ...makePayload(), text: "ignored" });
    });

    const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"];
    expect(mate!.streamingText).toBe("");
  });

  // --------------------------------------------------------------------------
  // 13. matchKey filtering — non-matching events ignored
  // --------------------------------------------------------------------------
  it("should ignore agent:run_started with non-matching context", () => {
    renderHook(() => useTeamEvents(CONTEXT_KEY));

    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });
    act(() => {
      fireEvent("team:teammate_spawned", {
        ...makePayload(), team_name: "t", teammate_name: "w1",
        color: "#f00", model: "sonnet", role: "coder",
      });
    });

    act(() => {
      fireEvent("agent:run_started", {
        context_type: "task_execution",
        context_id: "other-task",
        teammate_name: "w1",
      });
    });

    const mate = useTeamStore.getState().activeTeams[CONTEXT_KEY]!.teammates["w1"];
    expect(mate!.status).toBe("spawning"); // unchanged
  });

  // --------------------------------------------------------------------------
  // 14. Cleanup on unmount unsubscribes all handlers
  // --------------------------------------------------------------------------
  it("should unsubscribe all handlers on unmount", () => {
    const { unmount } = renderHook(() => useTeamEvents(CONTEXT_KEY));

    // Create team to activate Effect 2
    act(() => {
      fireEvent("team:created", { ...makePayload(), team_name: "t", lead_name: "l" });
    });

    // Verify subscriptions exist
    const totalBefore = Array.from(subscriptions.values()).reduce((sum, h) => sum + h.length, 0);
    expect(totalBefore).toBeGreaterThan(0);

    unmount();

    // All subscriptions should be removed
    const totalAfter = Array.from(subscriptions.values()).reduce((sum, h) => sum + h.length, 0);
    expect(totalAfter).toBe(0);
  });
});
