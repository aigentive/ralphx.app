/**
 * Tests for optimistic move with race condition handling
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import { createMockTask } from "@/test/mock-data";
import { useOptimisticMove } from "./useOptimisticMove";

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      move: vi.fn(),
    },
  },
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("useOptimisticMove", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should call move API on successful move", async () => {
    const task = createMockTask({ id: "t1", internalStatus: "ready" });
    vi.mocked(api.tasks.move).mockResolvedValue({ ...task, internalStatus: "backlog" });

    const { result } = renderHook(() => useOptimisticMove("project-1"), {
      wrapper: createWrapper(),
    });

    act(() => {
      result.current.move("t1", "backlog");
    });

    await waitFor(() => {
      expect(api.tasks.move).toHaveBeenCalledWith("t1", "backlog");
    });
  });

  it("should set error on move failure", async () => {
    vi.mocked(api.tasks.move).mockRejectedValue(new Error("Task already started"));

    const { result } = renderHook(() => useOptimisticMove("project-1"), {
      wrapper: createWrapper(),
    });

    act(() => {
      result.current.move("t1", "backlog");
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Task already started");
    });
  });

  it("should provide clearError function", async () => {
    vi.mocked(api.tasks.move).mockRejectedValue(new Error("Task already started"));

    const { result } = renderHook(() => useOptimisticMove("project-1"), {
      wrapper: createWrapper(),
    });

    act(() => {
      result.current.move("t1", "backlog");
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Task already started");
    });

    act(() => {
      result.current.clearError();
    });

    expect(result.current.error).toBeNull();
  });

  it("should return isMoving as false initially", () => {
    const { result } = renderHook(() => useOptimisticMove("project-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isMoving).toBe(false);
  });

  it("should return move and clearError functions", () => {
    const { result } = renderHook(() => useOptimisticMove("project-1"), {
      wrapper: createWrapper(),
    });

    expect(typeof result.current.move).toBe("function");
    expect(typeof result.current.clearError).toBe("function");
  });
});
