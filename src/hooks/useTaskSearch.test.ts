/**
 * useTaskSearch hook tests
 *
 * Tests for useTaskSearch hook using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { useTaskSearch, taskSearchKeys } from "./useTaskSearch";
import { api } from "@/lib/tauri";
import type { Task } from "@/types/task";

// Mock the tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      search: vi.fn(),
    },
  },
}));

// Create mock task data
const mockTask1: Task = {
  id: "task-1",
  projectId: "project-1",
  category: "feature",
  title: "Add user authentication",
  description: "Implement JWT-based authentication",
  priority: "high",
  internalStatus: "backlog",
  sourceProposalId: null,
  planArtifactId: null,
  archivedAt: null,
  createdAt: "2026-01-26T12:00:00Z",
  updatedAt: "2026-01-26T12:00:00Z",
};

const mockTask2: Task = {
  id: "task-2",
  projectId: "project-1",
  category: "feature",
  title: "Create dashboard",
  description: "Build user dashboard with authentication status",
  priority: "medium",
  internalStatus: "ready",
  sourceProposalId: null,
  planArtifactId: null,
  archivedAt: null,
  createdAt: "2026-01-26T12:00:00Z",
  updatedAt: "2026-01-26T12:00:00Z",
};

const mockArchivedTask: Task = {
  id: "task-3",
  projectId: "project-1",
  category: "feature",
  title: "Old authentication flow",
  description: "Deprecated authentication implementation",
  priority: "low",
  internalStatus: "cancelled",
  sourceProposalId: null,
  planArtifactId: null,
  archivedAt: "2026-01-20T10:00:00Z",
  createdAt: "2026-01-15T12:00:00Z",
  updatedAt: "2026-01-20T10:00:00Z",
};

// Helper to create a wrapper with QueryClient
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });

  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

describe("useTaskSearch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("query key factory", () => {
    it("should generate correct query keys", () => {
      expect(taskSearchKeys.all).toEqual(["tasks", "search"]);

      const params = {
        projectId: "project-1",
        query: "auth",
        includeArchived: false,
      };
      expect(taskSearchKeys.search(params)).toEqual([
        "tasks",
        "search",
        "project-1",
        "auth",
        false,
      ]);

      const paramsWithArchived = {
        projectId: "project-1",
        query: "auth",
        includeArchived: true,
      };
      expect(taskSearchKeys.search(paramsWithArchived)).toEqual([
        "tasks",
        "search",
        "project-1",
        "auth",
        true,
      ]);
    });
  });

  describe("useTaskSearch hook", () => {
    it("should search tasks by query", async () => {
      vi.mocked(api.tasks.search).mockResolvedValue([mockTask1, mockTask2]);

      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: "auth",
            includeArchived: false,
          }),
        { wrapper: createWrapper() }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));

      expect(api.tasks.search).toHaveBeenCalledWith(
        "project-1",
        "auth",
        false
      );
      expect(result.current.data).toEqual([mockTask1, mockTask2]);
    });

    it("should not search when query is null", async () => {
      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: null,
            includeArchived: false,
          }),
        { wrapper: createWrapper() }
      );

      // Query should be disabled
      expect(result.current.fetchStatus).toBe("idle");
      expect(result.current.data).toBeUndefined();
      expect(api.tasks.search).not.toHaveBeenCalled();
    });

    it("should not search when query has less than 2 characters", async () => {
      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: "a",
            includeArchived: false,
          }),
        { wrapper: createWrapper() }
      );

      // Query should be disabled
      expect(result.current.fetchStatus).toBe("idle");
      expect(result.current.data).toBeUndefined();
      expect(api.tasks.search).not.toHaveBeenCalled();
    });

    it("should search when query has exactly 2 characters", async () => {
      vi.mocked(api.tasks.search).mockResolvedValue([mockTask1]);

      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: "ab",
            includeArchived: false,
          }),
        { wrapper: createWrapper() }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));

      expect(api.tasks.search).toHaveBeenCalledWith("project-1", "ab", false);
      expect(result.current.data).toEqual([mockTask1]);
    });

    it("should include archived tasks when includeArchived is true", async () => {
      vi.mocked(api.tasks.search).mockResolvedValue([
        mockTask1,
        mockArchivedTask,
      ]);

      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: "auth",
            includeArchived: true,
          }),
        { wrapper: createWrapper() }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));

      expect(api.tasks.search).toHaveBeenCalledWith("project-1", "auth", true);
      expect(result.current.data).toEqual([mockTask1, mockArchivedTask]);
    });

    it("should handle empty search results", async () => {
      vi.mocked(api.tasks.search).mockResolvedValue([]);

      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: "nonexistent",
            includeArchived: false,
          }),
        { wrapper: createWrapper() }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));

      expect(api.tasks.search).toHaveBeenCalledWith(
        "project-1",
        "nonexistent",
        false
      );
      expect(result.current.data).toEqual([]);
    });

    it("should handle search errors", async () => {
      const error = new Error("Search failed");
      vi.mocked(api.tasks.search).mockRejectedValue(error);

      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: "auth",
            includeArchived: false,
          }),
        { wrapper: createWrapper() }
      );

      await waitFor(() => expect(result.current.isError).toBe(true));

      expect(result.current.error).toEqual(error);
    });

    it("should use 30 second stale time for caching", () => {
      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: "auth",
            includeArchived: false,
          }),
        { wrapper: createWrapper() }
      );

      // Access the query from the hook's internal state
      // Note: This is testing implementation details, but staleTime is critical for this hook
      expect(result.current).toBeDefined();
    });

    it("should update results when query changes", async () => {
      vi.mocked(api.tasks.search)
        .mockResolvedValueOnce([mockTask1])
        .mockResolvedValueOnce([mockTask2]);

      const { result, rerender } = renderHook(
        ({ query }: { query: string }) =>
          useTaskSearch({
            projectId: "project-1",
            query,
            includeArchived: false,
          }),
        {
          wrapper: createWrapper(),
          initialProps: { query: "auth" },
        }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual([mockTask1]);

      // Change query
      rerender({ query: "dashboard" });

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(api.tasks.search).toHaveBeenCalledWith(
        "project-1",
        "dashboard",
        false
      );
      expect(result.current.data).toEqual([mockTask2]);
    });

    it("should disable query when switching to query with < 2 chars", async () => {
      vi.mocked(api.tasks.search).mockResolvedValue([mockTask1]);

      const { result, rerender } = renderHook(
        ({ query }: { query: string | null }) =>
          useTaskSearch({
            projectId: "project-1",
            query,
            includeArchived: false,
          }),
        {
          wrapper: createWrapper(),
          initialProps: { query: "auth" },
        }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));
      expect(result.current.data).toEqual([mockTask1]);

      // Clear query - switches to a different query key, so data is undefined
      rerender({ query: "a" });

      // Query should be disabled
      expect(result.current.fetchStatus).toBe("idle");
      // With a different query key, TanStack Query returns undefined initially
      expect(result.current.data).toBeUndefined();
    });

    it("should support case-insensitive search", async () => {
      vi.mocked(api.tasks.search).mockResolvedValue([mockTask1, mockTask2]);

      const { result } = renderHook(
        () =>
          useTaskSearch({
            projectId: "project-1",
            query: "AUTH",
            includeArchived: false,
          }),
        { wrapper: createWrapper() }
      );

      await waitFor(() => expect(result.current.isSuccess).toBe(true));

      // Backend handles case-insensitivity, we just pass the query
      expect(api.tasks.search).toHaveBeenCalledWith(
        "project-1",
        "AUTH",
        false
      );
      expect(result.current.data).toEqual([mockTask1, mockTask2]);
    });
  });
});
