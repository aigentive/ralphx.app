/**
 * Tests for useMethodologyActivation hook
 *
 * Tests the integration between methodology activation and app state:
 * - Workflow store updates on activation
 * - Toast notifications on success/error
 * - Loading states during activation
 * - Deactivation restores default workflow
 */

import { describe, it, expect, vi, beforeEach, type Mock } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { useMethodologyActivation } from "./useMethodologyActivation";
import { useWorkflowStore } from "@/stores/workflowStore";
import { useMethodologyStore } from "@/stores/methodologyStore";
import { useUiStore } from "@/stores/uiStore";
import * as methodologiesApi from "@/lib/api/methodologies";
import type { MethodologyActivationResponse, MethodologyResponse } from "@/lib/api/methodologies";

// ============================================================================
// Mocks
// ============================================================================

vi.mock("@/lib/api/methodologies", () => ({
  activateMethodology: vi.fn(),
  deactivateMethodology: vi.fn(),
  getMethodologies: vi.fn(),
  getActiveMethodology: vi.fn(),
}));

// ============================================================================
// Test Utilities
// ============================================================================

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: 0 },
      mutations: { retry: false },
    },
  });
}

function createWrapper(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

// Reset all stores before each test
function resetStores() {
  useWorkflowStore.setState({
    workflows: {},
    activeWorkflowId: null,
    isLoading: false,
    error: null,
  });
  useMethodologyStore.setState({
    methodologies: {},
    activeMethodologyId: null,
    isLoading: false,
    isActivating: false,
    error: null,
  });
  useUiStore.setState({
    notifications: [],
    loading: {},
  });
}

// ============================================================================
// Test Data
// ============================================================================

const mockActivationResponse: MethodologyActivationResponse = {
  methodology: {
    id: "bmad-method",
    name: "BMAD Method",
    description: "Breakthrough Method for Agile AI-Driven Development",
    agent_profiles: ["bmad-analyst", "bmad-pm", "bmad-architect"],
    skills: ["/skills/bmad-analysis", "/skills/bmad-planning"],
    workflow_id: "bmad-workflow",
    workflow_name: "BMAD Workflow",
    phases: [
      { id: "analysis", name: "Analysis", order: 1, description: null, agent_profiles: ["bmad-analyst"], column_ids: ["brainstorm", "research"] },
      { id: "planning", name: "Planning", order: 2, description: null, agent_profiles: ["bmad-pm"], column_ids: ["prd-draft", "prd-review"] },
    ],
    templates: [],
    is_active: true,
    phase_count: 2,
    agent_count: 3,
    created_at: "2026-01-24T00:00:00Z",
  },
  workflow: {
    id: "bmad-workflow",
    name: "BMAD Workflow",
    description: "BMAD methodology workflow",
    column_count: 10,
  },
  agent_profiles: ["bmad-analyst", "bmad-pm", "bmad-architect"],
  skills: ["/skills/bmad-analysis", "/skills/bmad-planning"],
  previous_methodology_id: null,
};

const mockDeactivatedMethodology: MethodologyResponse = {
  id: "bmad-method",
  name: "BMAD Method",
  description: "Breakthrough Method for Agile AI-Driven Development",
  agent_profiles: ["bmad-analyst", "bmad-pm", "bmad-architect"],
  skills: ["/skills/bmad-analysis", "/skills/bmad-planning"],
  workflow_id: "bmad-workflow",
  workflow_name: "BMAD Workflow",
  phases: [],
  templates: [],
  is_active: false,
  phase_count: 2,
  agent_count: 3,
  created_at: "2026-01-24T00:00:00Z",
};

// ============================================================================
// Tests
// ============================================================================

describe("useMethodologyActivation", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    vi.clearAllMocks();
    resetStores();
    queryClient = createTestQueryClient();
  });

  describe("activate", () => {
    it("should call activateMethodology API on activate", async () => {
      (methodologiesApi.activateMethodology as Mock).mockResolvedValueOnce(mockActivationResponse);

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      await act(async () => {
        await result.current.activate("bmad-method");
      });

      expect(methodologiesApi.activateMethodology).toHaveBeenCalledWith("bmad-method");
    });

    it("should update methodology store with activated methodology", async () => {
      (methodologiesApi.activateMethodology as Mock).mockResolvedValueOnce(mockActivationResponse);

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      await act(async () => {
        await result.current.activate("bmad-method");
      });

      const methodologyState = useMethodologyStore.getState();
      expect(methodologyState.activeMethodologyId).toBe("bmad-method");
    });

    it("should show success notification on successful activation", async () => {
      (methodologiesApi.activateMethodology as Mock).mockResolvedValueOnce(mockActivationResponse);

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      await act(async () => {
        await result.current.activate("bmad-method");
      });

      const uiState = useUiStore.getState();
      expect(uiState.notifications).toHaveLength(1);
      expect(uiState.notifications[0]).toMatchObject({
        type: "success",
        message: expect.stringContaining("BMAD Method"),
      });
    });

    it("should show error notification on failed activation", async () => {
      (methodologiesApi.activateMethodology as Mock).mockRejectedValueOnce(new Error("Activation failed"));

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      await act(async () => {
        try {
          await result.current.activate("bmad-method");
        } catch {
          // Expected to throw
        }
      });

      const uiState = useUiStore.getState();
      expect(uiState.notifications).toHaveLength(1);
      expect(uiState.notifications[0]).toMatchObject({
        type: "error",
        message: expect.stringContaining("Activation failed"),
      });
    });

    it("should set isActivating to true during activation", async () => {
      let resolveActivation: (value: MethodologyActivationResponse) => void;
      const activationPromise = new Promise<MethodologyActivationResponse>((resolve) => {
        resolveActivation = resolve;
      });
      (methodologiesApi.activateMethodology as Mock).mockReturnValueOnce(activationPromise);

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      // Start activation (don't await)
      act(() => {
        result.current.activate("bmad-method");
      });

      // Check isActivating is true while pending
      await waitFor(() => {
        expect(result.current.isActivating).toBe(true);
      });

      // Resolve the promise
      await act(async () => {
        resolveActivation!(mockActivationResponse);
      });

      // isActivating should be false after completion
      await waitFor(() => {
        expect(result.current.isActivating).toBe(false);
      });
    });

    it("should return activation response on success", async () => {
      (methodologiesApi.activateMethodology as Mock).mockResolvedValueOnce(mockActivationResponse);

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      let response: MethodologyActivationResponse | undefined;
      await act(async () => {
        response = await result.current.activate("bmad-method");
      });

      expect(response).toEqual(mockActivationResponse);
    });
  });

  describe("deactivate", () => {
    it("should call deactivateMethodology API on deactivate", async () => {
      (methodologiesApi.deactivateMethodology as Mock).mockResolvedValueOnce(mockDeactivatedMethodology);

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      await act(async () => {
        await result.current.deactivate("bmad-method");
      });

      expect(methodologiesApi.deactivateMethodology).toHaveBeenCalledWith("bmad-method");
    });

    it("should update methodology store to clear active methodology", async () => {
      // Set up initial active methodology
      useMethodologyStore.setState({
        methodologies: { "bmad-method": { id: "bmad-method", isActive: true } as unknown as import("@/types/methodology").MethodologyTemplate },
        activeMethodologyId: "bmad-method",
      });

      (methodologiesApi.deactivateMethodology as Mock).mockResolvedValueOnce(mockDeactivatedMethodology);

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      await act(async () => {
        await result.current.deactivate("bmad-method");
      });

      const methodologyState = useMethodologyStore.getState();
      expect(methodologyState.activeMethodologyId).toBeNull();
    });

    it("should show success notification on successful deactivation", async () => {
      (methodologiesApi.deactivateMethodology as Mock).mockResolvedValueOnce(mockDeactivatedMethodology);

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      await act(async () => {
        await result.current.deactivate("bmad-method");
      });

      const uiState = useUiStore.getState();
      expect(uiState.notifications).toHaveLength(1);
      expect(uiState.notifications[0]).toMatchObject({
        type: "success",
        message: expect.stringContaining("default workflow"),
      });
    });

    it("should show error notification on failed deactivation", async () => {
      (methodologiesApi.deactivateMethodology as Mock).mockRejectedValueOnce(new Error("Deactivation failed"));

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      await act(async () => {
        try {
          await result.current.deactivate("bmad-method");
        } catch {
          // Expected to throw
        }
      });

      const uiState = useUiStore.getState();
      expect(uiState.notifications).toHaveLength(1);
      expect(uiState.notifications[0]).toMatchObject({
        type: "error",
        message: expect.stringContaining("Deactivation failed"),
      });
    });
  });

  describe("activeMethodology", () => {
    it("should return null when no methodology is active", () => {
      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      expect(result.current.activeMethodology).toBeNull();
    });

    it("should return active methodology from store", () => {
      useMethodologyStore.setState({
        methodologies: { "bmad-method": mockActivationResponse.methodology as unknown as import("@/types/methodology").MethodologyTemplate },
        activeMethodologyId: "bmad-method",
      });

      const { result } = renderHook(() => useMethodologyActivation(), {
        wrapper: createWrapper(queryClient),
      });

      expect(result.current.activeMethodology).toMatchObject({
        id: "bmad-method",
        name: "BMAD Method",
      });
    });
  });
});
