/**
 * Tests for useStepMutations hook
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useStepMutations } from "./useStepMutations";
import { stepKeys } from "./useTaskSteps";
import { api } from "@/lib/tauri";
import type { TaskStep } from "@/types/task-step";
import type { ReactNode } from "react";

// Mock the Tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    steps: {
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
      reorder: vi.fn(),
    },
  },
}));

// Mock sonner toast
vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe("useStepMutations", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
        mutations: {
          retry: false,
        },
      },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  const mockTaskId = "task-1";

  const mockStep: TaskStep = {
    id: "step-1",
    taskId: mockTaskId,
    title: "Test Step",
    description: "Test description",
    status: "pending",
    sortOrder: 0,
    dependsOn: null,
    createdBy: "user",
    completionNote: null,
    createdAt: "2025-01-26T00:00:00+00:00",
    updatedAt: "2025-01-26T00:00:00+00:00",
    startedAt: null,
    completedAt: null,
  };

  describe("create", () => {
    it("should create a new step", async () => {
      vi.mocked(api.steps.create).mockResolvedValue(mockStep);

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.create.mutate({
        title: "New Step",
        description: "Step description",
      });

      await waitFor(() => {
        expect(result.current.create.isSuccess).toBe(true);
      });

      expect(api.steps.create).toHaveBeenCalledWith(mockTaskId, {
        title: "New Step",
        description: "Step description",
      });
    });

    it("should invalidate queries on success", async () => {
      vi.mocked(api.steps.create).mockResolvedValue(mockStep);

      const invalidateQueriesSpy = vi.spyOn(queryClient, "invalidateQueries");

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.create.mutate({ title: "New Step" });

      await waitFor(() => {
        expect(result.current.create.isSuccess).toBe(true);
      });

      expect(invalidateQueriesSpy).toHaveBeenCalledWith({
        queryKey: stepKeys.byTask(mockTaskId),
      });
      expect(invalidateQueriesSpy).toHaveBeenCalledWith({
        queryKey: stepKeys.progress(mockTaskId),
      });
    });

    it("should handle errors", async () => {
      const mockError = new Error("Failed to create step");
      vi.mocked(api.steps.create).mockRejectedValue(mockError);

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.create.mutate({ title: "New Step" });

      await waitFor(() => {
        expect(result.current.create.isError).toBe(true);
      });

      expect(result.current.create.error).toEqual(mockError);
    });
  });

  describe("update", () => {
    it("should update an existing step", async () => {
      const updatedStep = { ...mockStep, title: "Updated Step" };
      vi.mocked(api.steps.update).mockResolvedValue(updatedStep);

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.update.mutate({
        stepId: "step-1",
        data: { title: "Updated Step" },
      });

      await waitFor(() => {
        expect(result.current.update.isSuccess).toBe(true);
      });

      expect(api.steps.update).toHaveBeenCalledWith("step-1", {
        title: "Updated Step",
      });
    });

    it("should invalidate queries on success", async () => {
      vi.mocked(api.steps.update).mockResolvedValue(mockStep);

      const invalidateQueriesSpy = vi.spyOn(queryClient, "invalidateQueries");

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.update.mutate({
        stepId: "step-1",
        data: { title: "Updated" },
      });

      await waitFor(() => {
        expect(result.current.update.isSuccess).toBe(true);
      });

      expect(invalidateQueriesSpy).toHaveBeenCalledWith({
        queryKey: stepKeys.byTask(mockTaskId),
      });
      expect(invalidateQueriesSpy).toHaveBeenCalledWith({
        queryKey: stepKeys.progress(mockTaskId),
      });
    });
  });

  describe("delete", () => {
    it("should delete a step", async () => {
      vi.mocked(api.steps.delete).mockResolvedValue(undefined);

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.delete.mutate("step-1");

      await waitFor(() => {
        expect(result.current.delete.isSuccess).toBe(true);
      });

      expect(api.steps.delete).toHaveBeenCalledWith("step-1");
    });

    it("should invalidate queries on success", async () => {
      vi.mocked(api.steps.delete).mockResolvedValue(undefined);

      const invalidateQueriesSpy = vi.spyOn(queryClient, "invalidateQueries");

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.delete.mutate("step-1");

      await waitFor(() => {
        expect(result.current.delete.isSuccess).toBe(true);
      });

      expect(invalidateQueriesSpy).toHaveBeenCalledWith({
        queryKey: stepKeys.byTask(mockTaskId),
      });
      expect(invalidateQueriesSpy).toHaveBeenCalledWith({
        queryKey: stepKeys.progress(mockTaskId),
      });
    });
  });

  describe("reorder", () => {
    it("should reorder steps", async () => {
      const reorderedSteps: TaskStep[] = [
        { ...mockStep, id: "step-3", sortOrder: 0 },
        { ...mockStep, id: "step-1", sortOrder: 1 },
        { ...mockStep, id: "step-2", sortOrder: 2 },
      ];
      vi.mocked(api.steps.reorder).mockResolvedValue(reorderedSteps);

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      const stepIds = ["step-3", "step-1", "step-2"];
      result.current.reorder.mutate(stepIds);

      await waitFor(() => {
        expect(result.current.reorder.isSuccess).toBe(true);
      });

      expect(api.steps.reorder).toHaveBeenCalledWith(mockTaskId, stepIds);
    });

    it("should invalidate queries on success", async () => {
      vi.mocked(api.steps.reorder).mockResolvedValue([]);

      const invalidateQueriesSpy = vi.spyOn(queryClient, "invalidateQueries");

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.reorder.mutate(["step-1", "step-2"]);

      await waitFor(() => {
        expect(result.current.reorder.isSuccess).toBe(true);
      });

      expect(invalidateQueriesSpy).toHaveBeenCalledWith({
        queryKey: stepKeys.byTask(mockTaskId),
      });
      expect(invalidateQueriesSpy).toHaveBeenCalledWith({
        queryKey: stepKeys.progress(mockTaskId),
      });
    });
  });

  describe("pending states", () => {
    it("should expose isPending states", () => {
      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      expect(result.current.isCreating).toBe(false);
      expect(result.current.isUpdating).toBe(false);
      expect(result.current.isDeleting).toBe(false);
      expect(result.current.isReordering).toBe(false);
    });

    it("should set isCreating to true during create mutation", async () => {
      vi.mocked(api.steps.create).mockImplementation(
        () =>
          new Promise((resolve) => {
            setTimeout(() => resolve(mockStep), 100);
          })
      );

      const { result } = renderHook(() => useStepMutations(mockTaskId), {
        wrapper,
      });

      result.current.create.mutate({ title: "New Step" });

      // Wait for mutation to enter pending state
      await waitFor(() => {
        expect(result.current.isCreating).toBe(true);
      });

      await waitFor(() => {
        expect(result.current.create.isSuccess).toBe(true);
      });

      expect(result.current.isCreating).toBe(false);
    });
  });
});
