/**
 * useWorkflows hooks tests
 *
 * Tests for useWorkflows, useWorkflow, and workflow mutation hooks
 * using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import {
  useWorkflows,
  useWorkflow,
  useActiveWorkflowColumns,
  useCreateWorkflow,
  useUpdateWorkflow,
  useDeleteWorkflow,
  useSetDefaultWorkflow,
  workflowKeys,
} from "./useWorkflows";
import * as workflowsApi from "@/lib/api/workflows";
import type { WorkflowResponse, WorkflowColumnResponse } from "@/lib/api/workflows";

// Mock the workflows API
vi.mock("@/lib/api/workflows", () => ({
  getWorkflows: vi.fn(),
  getWorkflow: vi.fn(),
  getActiveWorkflowColumns: vi.fn(),
  createWorkflow: vi.fn(),
  updateWorkflow: vi.fn(),
  deleteWorkflow: vi.fn(),
  setDefaultWorkflow: vi.fn(),
}));

// Create mock data
const mockColumn: WorkflowColumnResponse = {
  id: "col-1",
  name: "Backlog",
  maps_to: "backlog",
  color: null,
  icon: null,
  skip_review: null,
  auto_advance: null,
  agent_profile: null,
};

const mockColumn2: WorkflowColumnResponse = {
  id: "col-2",
  name: "In Progress",
  maps_to: "executing",
  color: "#ff6b35",
  icon: null,
  skip_review: false,
  auto_advance: null,
  agent_profile: "worker-1",
};

const mockWorkflow: WorkflowResponse = {
  id: "workflow-1",
  name: "Test Workflow",
  description: "A test workflow",
  columns: [mockColumn, mockColumn2],
  is_default: false,
  worker_profile: null,
  reviewer_profile: null,
};

const mockWorkflow2: WorkflowResponse = {
  id: "workflow-2",
  name: "Default Workflow",
  description: "The default workflow",
  columns: [mockColumn],
  is_default: true,
  worker_profile: "worker-1",
  reviewer_profile: "reviewer-1",
};

// Test wrapper with QueryClientProvider
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

describe("workflowKeys", () => {
  it("should generate correct key for all", () => {
    expect(workflowKeys.all).toEqual(["workflows"]);
  });

  it("should generate correct key for lists", () => {
    expect(workflowKeys.lists()).toEqual(["workflows", "list"]);
  });

  it("should generate correct key for details", () => {
    expect(workflowKeys.details()).toEqual(["workflows", "detail"]);
  });

  it("should generate correct key for detail by id", () => {
    expect(workflowKeys.detail("workflow-1")).toEqual([
      "workflows",
      "detail",
      "workflow-1",
    ]);
  });

  it("should generate correct key for active columns", () => {
    expect(workflowKeys.activeColumns()).toEqual([
      "workflows",
      "activeColumns",
    ]);
  });
});

describe("useWorkflows", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch all workflows successfully", async () => {
    const mockWorkflows = [mockWorkflow, mockWorkflow2];
    vi.mocked(workflowsApi.getWorkflows).mockResolvedValueOnce(mockWorkflows);

    const { result } = renderHook(() => useWorkflows(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockWorkflows);
    expect(workflowsApi.getWorkflows).toHaveBeenCalledTimes(1);
  });

  it("should return empty array when no workflows exist", async () => {
    vi.mocked(workflowsApi.getWorkflows).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useWorkflows(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([]);
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch workflows");
    vi.mocked(workflowsApi.getWorkflows).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useWorkflows(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useWorkflow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch a single workflow successfully", async () => {
    vi.mocked(workflowsApi.getWorkflow).mockResolvedValueOnce(mockWorkflow);

    const { result } = renderHook(() => useWorkflow("workflow-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockWorkflow);
    expect(workflowsApi.getWorkflow).toHaveBeenCalledWith("workflow-1");
  });

  it("should return null for non-existent workflow", async () => {
    vi.mocked(workflowsApi.getWorkflow).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useWorkflow("non-existent"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch workflow");
    vi.mocked(workflowsApi.getWorkflow).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useWorkflow("workflow-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });

  it("should not fetch when id is empty", async () => {
    const { result } = renderHook(() => useWorkflow(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isFetching).toBe(false);
    expect(workflowsApi.getWorkflow).not.toHaveBeenCalled();
  });
});

describe("useActiveWorkflowColumns", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch active workflow columns successfully", async () => {
    const mockColumns = [mockColumn, mockColumn2];
    vi.mocked(workflowsApi.getActiveWorkflowColumns).mockResolvedValueOnce(mockColumns);

    const { result } = renderHook(() => useActiveWorkflowColumns(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockColumns);
    expect(workflowsApi.getActiveWorkflowColumns).toHaveBeenCalledTimes(1);
  });

  it("should return empty array when no columns exist", async () => {
    vi.mocked(workflowsApi.getActiveWorkflowColumns).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useActiveWorkflowColumns(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([]);
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch columns");
    vi.mocked(workflowsApi.getActiveWorkflowColumns).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useActiveWorkflowColumns(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useCreateWorkflow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should create a workflow successfully", async () => {
    vi.mocked(workflowsApi.createWorkflow).mockResolvedValueOnce(mockWorkflow);

    const { result } = renderHook(() => useCreateWorkflow(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({
        name: "Test Workflow",
        columns: [
          { id: "col-1", name: "Backlog", maps_to: "backlog" },
        ],
      });
    });

    expect(workflowsApi.createWorkflow).toHaveBeenCalled();
    expect(vi.mocked(workflowsApi.createWorkflow).mock.calls[0][0]).toEqual({
      name: "Test Workflow",
      columns: [{ id: "col-1", name: "Backlog", maps_to: "backlog" }],
    });
  });

  it("should handle creation error", async () => {
    const error = new Error("Failed to create workflow");
    vi.mocked(workflowsApi.createWorkflow).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useCreateWorkflow(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync({
          name: "Test Workflow",
          columns: [{ id: "col-1", name: "Backlog", maps_to: "backlog" }],
        });
      })
    ).rejects.toThrow("Failed to create workflow");
  });
});

describe("useUpdateWorkflow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should update a workflow successfully", async () => {
    const updatedWorkflow = { ...mockWorkflow, name: "Updated Workflow" };
    vi.mocked(workflowsApi.updateWorkflow).mockResolvedValueOnce(updatedWorkflow);

    const { result } = renderHook(() => useUpdateWorkflow(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({
        id: "workflow-1",
        input: { name: "Updated Workflow" },
      });
    });

    expect(workflowsApi.updateWorkflow).toHaveBeenCalledWith("workflow-1", {
      name: "Updated Workflow",
    });
  });

  it("should handle update error", async () => {
    const error = new Error("Failed to update workflow");
    vi.mocked(workflowsApi.updateWorkflow).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useUpdateWorkflow(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync({
          id: "workflow-1",
          input: { name: "Updated Workflow" },
        });
      })
    ).rejects.toThrow("Failed to update workflow");
  });
});

describe("useDeleteWorkflow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should delete a workflow successfully", async () => {
    vi.mocked(workflowsApi.deleteWorkflow).mockResolvedValueOnce(undefined);

    const { result } = renderHook(() => useDeleteWorkflow(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync("workflow-1");
    });

    expect(workflowsApi.deleteWorkflow).toHaveBeenCalled();
    expect(vi.mocked(workflowsApi.deleteWorkflow).mock.calls[0][0]).toBe("workflow-1");
  });

  it("should handle delete error", async () => {
    const error = new Error("Failed to delete workflow");
    vi.mocked(workflowsApi.deleteWorkflow).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useDeleteWorkflow(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync("workflow-1");
      })
    ).rejects.toThrow("Failed to delete workflow");
  });
});

describe("useSetDefaultWorkflow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should set default workflow successfully", async () => {
    const defaultWorkflow = { ...mockWorkflow, is_default: true };
    vi.mocked(workflowsApi.setDefaultWorkflow).mockResolvedValueOnce(defaultWorkflow);

    const { result } = renderHook(() => useSetDefaultWorkflow(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync("workflow-1");
    });

    expect(workflowsApi.setDefaultWorkflow).toHaveBeenCalled();
    expect(vi.mocked(workflowsApi.setDefaultWorkflow).mock.calls[0][0]).toBe("workflow-1");
  });

  it("should handle set default error", async () => {
    const error = new Error("Failed to set default workflow");
    vi.mocked(workflowsApi.setDefaultWorkflow).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useSetDefaultWorkflow(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync("workflow-1");
      })
    ).rejects.toThrow("Failed to set default workflow");
  });
});
