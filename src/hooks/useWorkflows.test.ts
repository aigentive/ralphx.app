import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import {
  useWorkflows,
  useWorkflow,
  useActiveWorkflowColumns,
  useCreateWorkflow,
  useUpdateWorkflow,
  useSetDefaultWorkflow,
  workflowKeys,
} from "./useWorkflows";
import { api } from "@/lib/tauri";
import type { WorkflowSchema, WorkflowColumn } from "@/types/workflow";

const mockColumn: WorkflowColumn = {
  id: "draft",
  name: "Backlog",
  mapsTo: "backlog",
};

const mockWorkflow: WorkflowSchema = {
  id: "workflow-1",
  name: "Test Workflow",
  description: "A test workflow",
  columns: [mockColumn],
  isDefault: false,
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  });

  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

describe("workflowKeys", () => {
  it("generates stable query keys", () => {
    expect(workflowKeys.all).toEqual(["workflows"]);
    expect(workflowKeys.lists()).toEqual(["workflows", "list"]);
    expect(workflowKeys.details()).toEqual(["workflows", "detail"]);
    expect(workflowKeys.detail("workflow-1")).toEqual(["workflows", "detail", "workflow-1"]);
    expect(workflowKeys.activeColumns()).toEqual(["workflows", "activeColumns"]);
  });
});

describe("useWorkflows", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches workflows", async () => {
    vi.spyOn(api.workflows, "list").mockResolvedValueOnce([mockWorkflow]);

    const { result } = renderHook(() => useWorkflows(), { wrapper: createWrapper() });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([mockWorkflow]);
  });

  it("surfaces fetch errors", async () => {
    vi.spyOn(api.workflows, "list").mockRejectedValueOnce(new Error("boom"));

    const { result } = renderHook(() => useWorkflows(), { wrapper: createWrapper() });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(result.current.error?.message).toBe("boom");
  });
});

describe("useWorkflow", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches one workflow by id", async () => {
    vi.spyOn(api.workflows, "get").mockResolvedValueOnce(mockWorkflow);

    const { result } = renderHook(() => useWorkflow("workflow-1"), { wrapper: createWrapper() });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual(mockWorkflow);
  });

  it("does not fetch when id is empty", () => {
    const getSpy = vi.spyOn(api.workflows, "get");

    const { result } = renderHook(() => useWorkflow(""), { wrapper: createWrapper() });

    expect(result.current.isFetching).toBe(false);
    expect(getSpy).not.toHaveBeenCalled();
  });
});

describe("useActiveWorkflowColumns", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches active columns", async () => {
    vi.spyOn(api.workflows, "getActiveColumns").mockResolvedValueOnce([mockColumn]);

    const { result } = renderHook(() => useActiveWorkflowColumns(), { wrapper: createWrapper() });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([mockColumn]);
  });
});

describe("workflow mutations", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("creates a workflow", async () => {
    const createSpy = vi.spyOn(api.workflows, "create").mockResolvedValueOnce(mockWorkflow);
    const { result } = renderHook(() => useCreateWorkflow(), { wrapper: createWrapper() });

    await act(async () => {
      await result.current.mutateAsync({
        name: "Test Workflow",
        columns: [{ id: "draft", name: "Backlog", maps_to: "backlog" }],
      });
    });

    expect(createSpy).toHaveBeenCalledTimes(1);
  });

  it("updates a workflow", async () => {
    const updateSpy = vi.spyOn(api.workflows, "update").mockResolvedValueOnce({ ...mockWorkflow, name: "Updated" });
    const { result } = renderHook(() => useUpdateWorkflow(), { wrapper: createWrapper() });

    await act(async () => {
      await result.current.mutateAsync({ id: "workflow-1", input: { name: "Updated" } });
    });

    expect(updateSpy).toHaveBeenCalledWith("workflow-1", { name: "Updated" });
  });

  it("sets default workflow", async () => {
    const setDefaultSpy = vi.spyOn(api.workflows, "setDefault").mockResolvedValueOnce({ ...mockWorkflow, isDefault: true });
    const { result } = renderHook(() => useSetDefaultWorkflow(), { wrapper: createWrapper() });

    await act(async () => {
      await result.current.mutateAsync("workflow-1");
    });

    expect(setDefaultSpy).toHaveBeenCalledWith("workflow-1");
  });
});
