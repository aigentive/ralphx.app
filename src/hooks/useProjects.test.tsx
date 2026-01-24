import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useProjects, useProject } from "./useProjects";
import { api } from "@/lib/tauri";
import type { Project } from "@/types/project";

// Mock the tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    projects: {
      list: vi.fn(),
      get: vi.fn(),
    },
  },
}));

// Helper to create a mock project
const createMockProject = (overrides: Partial<Project> = {}): Project => ({
  id: "project-1",
  name: "Test Project",
  workingDirectory: "/path/to/project",
  gitMode: "local",
  worktreePath: null,
  worktreeBranch: null,
  baseBranch: null,
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  ...overrides,
});

describe("useProjects", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("should fetch all projects", async () => {
    const mockProjects = [
      createMockProject({ id: "project-1", name: "Project 1" }),
      createMockProject({ id: "project-2", name: "Project 2" }),
    ];
    vi.mocked(api.projects.list).mockResolvedValue(mockProjects);

    const { result } = renderHook(() => useProjects(), { wrapper });

    // Initially loading
    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(api.projects.list).toHaveBeenCalledTimes(1);
    expect(result.current.data).toEqual(mockProjects);
    expect(result.current.data).toHaveLength(2);
  });

  it("should handle loading state", async () => {
    let resolvePromise: (value: Project[]) => void;
    const pendingPromise = new Promise<Project[]>((resolve) => {
      resolvePromise = resolve;
    });
    vi.mocked(api.projects.list).mockReturnValue(pendingPromise);

    const { result } = renderHook(() => useProjects(), { wrapper });

    expect(result.current.isLoading).toBe(true);
    expect(result.current.data).toBeUndefined();

    resolvePromise!([createMockProject()]);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.isSuccess).toBe(true);
  });

  it("should handle error state", async () => {
    const error = new Error("Failed to fetch projects");
    vi.mocked(api.projects.list).mockRejectedValue(error);

    const { result } = renderHook(() => useProjects(), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toBe(error);
  });

  it("should return empty array when no projects exist", async () => {
    vi.mocked(api.projects.list).mockResolvedValue([]);

    const { result } = renderHook(() => useProjects(), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual([]);
  });

  it("should not refetch on every render", async () => {
    const mockProjects = [createMockProject()];
    vi.mocked(api.projects.list).mockResolvedValue(mockProjects);

    const { result, rerender } = renderHook(() => useProjects(), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    rerender();
    rerender();

    expect(api.projects.list).toHaveBeenCalledTimes(1);
  });
});

describe("useProject", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("should fetch a single project by ID", async () => {
    const mockProject = createMockProject({ id: "project-123", name: "My Project" });
    vi.mocked(api.projects.get).mockResolvedValue(mockProject);

    const { result } = renderHook(() => useProject("project-123"), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(api.projects.get).toHaveBeenCalledWith("project-123");
    expect(result.current.data).toEqual(mockProject);
  });

  it("should handle error for non-existent project", async () => {
    const error = new Error("Project not found");
    vi.mocked(api.projects.get).mockRejectedValue(error);

    const { result } = renderHook(() => useProject("nonexistent"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toBe(error);
  });

  it("should use different cache for different project IDs", async () => {
    const project1 = createMockProject({ id: "project-1", name: "Project 1" });
    const project2 = createMockProject({ id: "project-2", name: "Project 2" });

    vi.mocked(api.projects.get)
      .mockResolvedValueOnce(project1)
      .mockResolvedValueOnce(project2);

    const { result: result1 } = renderHook(() => useProject("project-1"), { wrapper });
    const { result: result2 } = renderHook(() => useProject("project-2"), { wrapper });

    await waitFor(() => {
      expect(result1.current.isSuccess).toBe(true);
      expect(result2.current.isSuccess).toBe(true);
    });

    expect(api.projects.get).toHaveBeenCalledWith("project-1");
    expect(api.projects.get).toHaveBeenCalledWith("project-2");
    expect(api.projects.get).toHaveBeenCalledTimes(2);
  });
});
