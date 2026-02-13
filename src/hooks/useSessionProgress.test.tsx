import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useSessionProgress } from "./useSessionProgress";
import { api } from "@/lib/tauri";
import type { Task } from "@/types/task";
import type { IdeationSession } from "@/types/ideation";

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
    },
  },
}));

const createMockTask = (overrides: Partial<Task> = {}): Task => ({
  id: "task-1",
  projectId: "project-1",
  category: "feature",
  title: "Test Task",
  description: null,
  priority: 0,
  internalStatus: "backlog",
  needsReviewPoint: false,
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  startedAt: null,
  completedAt: null,
  archivedAt: null,
  blockedReason: null,
  ...overrides,
});

const createMockSession = (overrides: Partial<IdeationSession> = {}): IdeationSession => ({
  id: "session-1",
  projectId: "project-1",
  title: "Test Session",
  status: "accepted",
  planArtifactId: null,
  seedTaskId: null,
  parentSessionId: null,
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  archivedAt: null,
  convertedAt: null,
  ...overrides,
});

describe("useSessionProgress", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("should return empty map when no sessions provided", async () => {
    vi.mocked(api.tasks.list).mockResolvedValue({
      tasks: [],
      total: 0,
      hasMore: false,
      offset: 0,
    });

    const { result } = renderHook(
      () => useSessionProgress("project-1", []),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.progressMap.size).toBe(0);
    });
  });

  it("should return empty map when tasks are loading", () => {
    let resolvePromise: (value: unknown) => void;
    const pendingPromise = new Promise((resolve) => {
      resolvePromise = resolve;
    });
    vi.mocked(api.tasks.list).mockReturnValue(pendingPromise as never);

    const sessions = [createMockSession({ id: "session-1" })];
    const { result } = renderHook(
      () => useSessionProgress("project-1", sessions),
      { wrapper }
    );

    expect(result.current.progressMap.size).toBe(0);
    expect(result.current.isLoading).toBe(true);

    // Clean up
    resolvePromise!({ tasks: [], total: 0, hasMore: false, offset: 0 });
  });

  it("should compute progress for a session with mixed task statuses", async () => {
    const tasks = [
      createMockTask({ id: "t1", ideationSessionId: "session-1", internalStatus: "backlog" }),
      createMockTask({ id: "t2", ideationSessionId: "session-1", internalStatus: "executing" }),
      createMockTask({ id: "t3", ideationSessionId: "session-1", internalStatus: "merged" }),
      createMockTask({ id: "t4", ideationSessionId: "session-1", internalStatus: "ready" }),
    ];

    vi.mocked(api.tasks.list).mockResolvedValue({
      tasks,
      total: 4,
      hasMore: false,
      offset: 0,
    });

    const sessions = [createMockSession({ id: "session-1", status: "accepted" })];
    const { result } = renderHook(
      () => useSessionProgress("project-1", sessions),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.progressMap.size).toBe(1);
    });

    const progress = result.current.progressMap.get("session-1");
    expect(progress).toBeDefined();
    expect(progress!.idle).toBe(2);    // backlog + ready
    expect(progress!.active).toBe(1);  // executing
    expect(progress!.done).toBe(1);    // merged
    expect(progress!.total).toBe(4);
  });

  it("should compute progress for multiple sessions independently", async () => {
    const tasks = [
      createMockTask({ id: "t1", ideationSessionId: "session-1", internalStatus: "merged" }),
      createMockTask({ id: "t2", ideationSessionId: "session-1", internalStatus: "merged" }),
      createMockTask({ id: "t3", ideationSessionId: "session-2", internalStatus: "backlog" }),
      createMockTask({ id: "t4", ideationSessionId: "session-2", internalStatus: "executing" }),
    ];

    vi.mocked(api.tasks.list).mockResolvedValue({
      tasks,
      total: 4,
      hasMore: false,
      offset: 0,
    });

    const sessions = [
      createMockSession({ id: "session-1", status: "accepted" }),
      createMockSession({ id: "session-2", status: "accepted" }),
    ];
    const { result } = renderHook(
      () => useSessionProgress("project-1", sessions),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.progressMap.size).toBe(2);
    });

    const s1 = result.current.progressMap.get("session-1")!;
    expect(s1.idle).toBe(0);
    expect(s1.active).toBe(0);
    expect(s1.done).toBe(2);
    expect(s1.total).toBe(2);

    const s2 = result.current.progressMap.get("session-2")!;
    expect(s2.idle).toBe(1);
    expect(s2.active).toBe(1);
    expect(s2.done).toBe(0);
    expect(s2.total).toBe(2);
  });

  it("should return zero counts for accepted sessions with no matching tasks", async () => {
    const tasks = [
      createMockTask({ id: "t1", ideationSessionId: "other-session", internalStatus: "backlog" }),
    ];

    vi.mocked(api.tasks.list).mockResolvedValue({
      tasks,
      total: 1,
      hasMore: false,
      offset: 0,
    });

    const sessions = [createMockSession({ id: "session-1", status: "accepted" })];
    const { result } = renderHook(
      () => useSessionProgress("project-1", sessions),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.progressMap.size).toBe(1);
    });

    const progress = result.current.progressMap.get("session-1")!;
    expect(progress.idle).toBe(0);
    expect(progress.active).toBe(0);
    expect(progress.done).toBe(0);
    expect(progress.total).toBe(0);
  });

  it("should only compute progress for accepted sessions", async () => {
    const tasks = [
      createMockTask({ id: "t1", ideationSessionId: "draft-session", internalStatus: "backlog" }),
      createMockTask({ id: "t2", ideationSessionId: "accepted-session", internalStatus: "executing" }),
    ];

    vi.mocked(api.tasks.list).mockResolvedValue({
      tasks,
      total: 2,
      hasMore: false,
      offset: 0,
    });

    const sessions = [
      createMockSession({ id: "draft-session", status: "active" }),
      createMockSession({ id: "accepted-session", status: "accepted" }),
      createMockSession({ id: "archived-session", status: "archived" }),
    ];
    const { result } = renderHook(
      () => useSessionProgress("project-1", sessions),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.progressMap.size).toBe(1);
    });

    // Only the accepted session should have progress computed
    expect(result.current.progressMap.has("accepted-session")).toBe(true);
    expect(result.current.progressMap.has("draft-session")).toBe(false);
    expect(result.current.progressMap.has("archived-session")).toBe(false);
  });

  it("should ignore tasks without ideationSessionId", async () => {
    const tasks = [
      createMockTask({ id: "t1", ideationSessionId: "session-1", internalStatus: "backlog" }),
      createMockTask({ id: "t2", ideationSessionId: undefined, internalStatus: "executing" }),
    ];

    vi.mocked(api.tasks.list).mockResolvedValue({
      tasks,
      total: 2,
      hasMore: false,
      offset: 0,
    });

    const sessions = [createMockSession({ id: "session-1", status: "accepted" })];
    const { result } = renderHook(
      () => useSessionProgress("project-1", sessions),
      { wrapper }
    );

    await waitFor(() => {
      expect(result.current.progressMap.size).toBe(1);
    });

    const progress = result.current.progressMap.get("session-1")!;
    expect(progress.total).toBe(1); // only t1
  });
});
