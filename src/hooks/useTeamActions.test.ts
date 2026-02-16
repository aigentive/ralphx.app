/**
 * useTeamActions tests — Mutation hooks for team operations
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useTeamActions } from "./useTeamActions";

// ============================================================================
// Mocks
// ============================================================================

const mockInvalidateQueries = vi.fn();
vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({ invalidateQueries: mockInvalidateQueries }),
  useMutation: (opts: { mutationFn: (...args: unknown[]) => Promise<unknown>; onSuccess?: () => void }) => ({
    mutate: vi.fn((...args: unknown[]) => {
      const result = opts.mutationFn(...args);
      result.then(() => opts.onSuccess?.());
      return result;
    }),
    mutateAsync: vi.fn((...args: unknown[]) => {
      const result = opts.mutationFn(...args);
      result.then(() => opts.onSuccess?.());
      return result;
    }),
    isPending: false,
  }),
}));

vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      activeTeams: {
        "task_execution:task-1": { teamName: "test-team-abc" },
      },
    }),
}));

vi.mock("@/lib/chat-context-registry", () => ({
  buildStoreKey: (contextType: string, contextId: string) => `${contextType}:${contextId}`,
}));

vi.mock("@/hooks/useTeamStatus", () => ({
  teamKeys: {
    all: ["teams"] as const,
    status: (ct: string, ci: string) => ["teams", "status", ct, ci] as const,
  },
}));

const mockSendTeamMessage = vi.fn().mockResolvedValue({ id: "msg-1", sender: "lead", recipient: "coder", content: "hi", message_type: "text", timestamp: "2026-01-01T00:00:00Z" });
const mockSendTeammateMessage = vi.fn().mockResolvedValue(undefined);
const mockStopTeammate = vi.fn().mockResolvedValue(undefined);
const mockStopTeam = vi.fn().mockResolvedValue(undefined);

vi.mock("@/api/team", () => ({
  sendTeamMessage: (...args: unknown[]) => mockSendTeamMessage(...args),
  sendTeammateMessage: (...args: unknown[]) => mockSendTeammateMessage(...args),
  stopTeammate: (...args: unknown[]) => mockStopTeammate(...args),
  stopTeam: (...args: unknown[]) => mockStopTeam(...args),
}));

// ============================================================================
// Tests
// ============================================================================

describe("useTeamActions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns sendTeamMessage, messageTeammate, stopTeammate, stopTeam mutations", () => {
    const { result } = renderHook(() => useTeamActions("task_execution", "task-1"));

    expect(result.current.sendTeamMessage).toBeDefined();
    expect(result.current.sendTeamMessage.mutateAsync).toBeDefined();
    expect(result.current.messageTeammate).toBeDefined();
    expect(result.current.messageTeammate.mutateAsync).toBeDefined();
    expect(result.current.stopTeammate).toBeDefined();
    expect(result.current.stopTeam).toBeDefined();
  });

  it("sendTeamMessage calls API with resolved teamName", async () => {
    const { result } = renderHook(() => useTeamActions("task_execution", "task-1"));

    await act(async () => {
      await result.current.sendTeamMessage.mutateAsync({ content: "hello", target: "coder-1" });
    });

    expect(mockSendTeamMessage).toHaveBeenCalledWith("test-team-abc", "coder-1", "hello");
  });

  it("messageTeammate calls API with resolved teamName", async () => {
    const { result } = renderHook(() => useTeamActions("task_execution", "task-1"));

    await act(async () => {
      await result.current.messageTeammate.mutateAsync({ teammateName: "coder-1", content: "hello via stdin" });
    });

    expect(mockSendTeammateMessage).toHaveBeenCalledWith("test-team-abc", "coder-1", "hello via stdin");
  });

  it("stopTeammate calls API with resolved teamName", async () => {
    const { result } = renderHook(() => useTeamActions("task_execution", "task-1"));

    await act(async () => {
      await result.current.stopTeammate.mutateAsync("coder-1");
    });

    expect(mockStopTeammate).toHaveBeenCalledWith("test-team-abc", "coder-1");
  });

  it("stopTeam calls API with resolved teamName", async () => {
    const { result } = renderHook(() => useTeamActions("task_execution", "task-1"));

    await act(async () => {
      await result.current.stopTeam.mutateAsync();
    });

    expect(mockStopTeam).toHaveBeenCalledWith("test-team-abc");
  });

  it("invalidates team status on successful mutation", async () => {
    const { result } = renderHook(() => useTeamActions("task_execution", "task-1"));

    await act(async () => {
      await result.current.sendTeamMessage.mutateAsync({ content: "hi", target: "coder" });
    });

    expect(mockInvalidateQueries).toHaveBeenCalledWith({
      queryKey: ["teams", "status", "task_execution", "task-1"],
    });
  });
});
