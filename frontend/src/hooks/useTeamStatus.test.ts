/**
 * useTeamStatus tests — Polling hook for team status
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { useTeamStatus } from "./useTeamStatus";

// ============================================================================
// Mocks
// ============================================================================

let mockEnabled: boolean | undefined;
let mockRefetchInterval: number | undefined;

vi.mock("@tanstack/react-query", () => ({
  useQuery: (opts: { enabled?: boolean; refetchInterval?: number; queryKey: unknown[] }) => {
    mockEnabled = opts.enabled;
    mockRefetchInterval = opts.refetchInterval;
    return { data: null, isLoading: false, error: null };
  },
}));

let isTeamActiveValue = false;
vi.mock("@/stores/chatStore", () => ({
  useChatStore: (_selector: (state: unknown) => unknown) => isTeamActiveValue,
  selectIsTeamActive: (_key: string) => (_state: unknown) => isTeamActiveValue,
}));

let teamNameValue = "";
vi.mock("@/stores/teamStore", () => ({
  useTeamStore: (_selector: (state: Record<string, unknown>) => unknown) => teamNameValue,
}));

vi.mock("@/lib/chat-context-registry", () => ({
  buildStoreKey: (contextType: string, contextId: string) => `${contextType}:${contextId}`,
}));

vi.mock("@/api/team", () => ({
  getTeamStatus: vi.fn().mockResolvedValue(null),
}));

// ============================================================================
// Tests
// ============================================================================

describe("useTeamStatus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isTeamActiveValue = false;
    teamNameValue = "";
    mockEnabled = undefined;
    mockRefetchInterval = undefined;
  });

  it("disables polling when team is not active", () => {
    isTeamActiveValue = false;
    teamNameValue = "";

    renderHook(() => useTeamStatus("task_execution", "task-1"));

    expect(mockEnabled).toBe(false);
  });

  it("disables polling when teamName is empty", () => {
    isTeamActiveValue = true;
    teamNameValue = "";

    renderHook(() => useTeamStatus("task_execution", "task-1"));

    expect(mockEnabled).toBe(false);
  });

  it("enables polling when team is active and teamName exists", () => {
    isTeamActiveValue = true;
    teamNameValue = "my-team";

    renderHook(() => useTeamStatus("task_execution", "task-1"));

    expect(mockEnabled).toBe(true);
  });

  it("uses 5s refetch interval", () => {
    isTeamActiveValue = true;
    teamNameValue = "my-team";

    renderHook(() => useTeamStatus("task_execution", "task-1"));

    expect(mockRefetchInterval).toBe(5000);
  });
});
