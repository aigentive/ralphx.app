/**
 * useChildSessionStatus hook tests
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { useChildSessionStatus } from "./useChildSessionStatus";
import * as chatApi from "@/api/chat";
import type { ChildSessionStatusResponse } from "@/api/chat";

vi.mock("@/api/chat", () => ({
  getChildSessionStatus: vi.fn(),
}));

const mockedGetStatus = vi.mocked(chatApi.getChildSessionStatus);

function makeResponse(
  estimatedStatus: "idle" | "likely_generating" | "likely_waiting",
  messages: { role: string; content: string; created_at: string | null }[] = []
): ChildSessionStatusResponse {
  return {
    session_id: "session-123",
    title: "Test Session",
    agent_state: { estimated_status: estimatedStatus },
    recent_messages: messages,
  };
}

function makeWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });
  return ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useChildSessionStatus", () => {
  it("returns loading state before fetch completes", () => {
    mockedGetStatus.mockReturnValue(new Promise(() => {})); // never resolves
    const { result } = renderHook(
      () => useChildSessionStatus("session-123"),
      { wrapper: makeWrapper() }
    );
    expect(result.current.isLoading).toBe(true);
    expect(result.current.data).toBeUndefined();
  });

  it("returns data on successful fetch with messages", async () => {
    const messages = [
      { role: "user", content: "Hello", created_at: "2026-03-01T10:00:00Z" },
      { role: "assistant", content: "Hi there", created_at: "2026-03-01T10:01:00Z" },
    ];
    mockedGetStatus.mockResolvedValue(makeResponse("likely_generating", messages));

    const { result } = renderHook(
      () => useChildSessionStatus("session-123"),
      { wrapper: makeWrapper() }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.recent_messages).toHaveLength(2);
    expect(result.current.data?.agent_state.estimated_status).toBe("likely_generating");
  });

  it("returns data on successful fetch with empty messages", async () => {
    mockedGetStatus.mockResolvedValue(makeResponse("likely_generating", []));

    const { result } = renderHook(
      () => useChildSessionStatus("session-123"),
      { wrapper: makeWrapper() }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.recent_messages).toHaveLength(0);
  });

  it("does not fetch when sessionId is null", () => {
    const { result } = renderHook(
      () => useChildSessionStatus(null),
      { wrapper: makeWrapper() }
    );
    expect(result.current.isLoading).toBe(false);
    expect(result.current.fetchStatus).toBe("idle");
    expect(mockedGetStatus).not.toHaveBeenCalled();
  });

  it("does not fetch when sessionId is undefined", () => {
    const { result } = renderHook(
      () => useChildSessionStatus(undefined),
      { wrapper: makeWrapper() }
    );
    expect(result.current.isLoading).toBe(false);
    expect(result.current.fetchStatus).toBe("idle");
    expect(mockedGetStatus).not.toHaveBeenCalled();
  });

  it("does not fetch when enabled=false", () => {
    const { result } = renderHook(
      () => useChildSessionStatus("session-123", false),
      { wrapper: makeWrapper() }
    );
    expect(result.current.isLoading).toBe(false);
    expect(result.current.fetchStatus).toBe("idle");
    expect(mockedGetStatus).not.toHaveBeenCalled();
  });

  it("disables polling when agent status is idle", async () => {
    mockedGetStatus.mockResolvedValue(
      makeResponse("idle", [{ role: "user", content: "Hi", created_at: null }])
    );

    const { result } = renderHook(
      () => useChildSessionStatus("session-123"),
      { wrapper: makeWrapper() }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    // With refetchInterval returning false when idle, no additional calls after initial fetch
    expect(mockedGetStatus).toHaveBeenCalledTimes(1);
  });

  it("permanently disables polling when first fetch returns idle + empty messages (history mode)", async () => {
    // First fetch: idle + no messages → history mode → permanent disable
    mockedGetStatus.mockResolvedValueOnce(makeResponse("idle", []));

    const { result } = renderHook(
      () => useChildSessionStatus("session-123"),
      { wrapper: makeWrapper() }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    // After the first fetch, the hook should have disabled permanently
    expect(result.current.data?.agent_state.estimated_status).toBe("idle");
    expect(result.current.data?.recent_messages).toHaveLength(0);
    expect(mockedGetStatus).toHaveBeenCalledTimes(1);
  });
});
