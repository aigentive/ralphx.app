/**
 * useMethodologies hooks tests
 *
 * Tests for useMethodologies, useActiveMethodology, and methodology mutation hooks
 * using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import {
  useMethodologies,
  useActiveMethodology,
  useActivateMethodology,
  useDeactivateMethodology,
  methodologyKeys,
} from "./useMethodologies";
import * as methodologiesApi from "@/lib/api/methodologies";
import type {
  MethodologyResponse,
  MethodologyActivationResponse,
} from "@/lib/api/methodologies";

// Mock the methodologies API
vi.mock("@/lib/api/methodologies", () => ({
  getMethodologies: vi.fn(),
  getActiveMethodology: vi.fn(),
  activateMethodology: vi.fn(),
  deactivateMethodology: vi.fn(),
}));

// Create mock data
const mockMethodology: MethodologyResponse = {
  id: "bmad-method",
  name: "BMAD Method",
  description: "Breakthrough Method for Agile AI-Driven Development",
  agent_profiles: ["bmad-analyst", "bmad-pm", "bmad-architect"],
  skills: ["skills/prd-creation", "skills/architecture-design"],
  workflow_id: "bmad-method",
  workflow_name: "BMAD Method",
  phases: [
    {
      id: "analysis",
      name: "Analysis",
      order: 0,
      description: "Analyze requirements",
      agent_profiles: ["bmad-analyst"],
      column_ids: ["brainstorm", "research"],
    },
    {
      id: "planning",
      name: "Planning",
      order: 1,
      description: "Create PRD",
      agent_profiles: ["bmad-pm", "bmad-ux"],
      column_ids: ["prd-draft", "prd-review"],
    },
  ],
  templates: [
    {
      artifact_type: "prd",
      template_path: "templates/bmad/prd.md",
      name: "PRD Template",
      description: "Product Requirements Document",
    },
  ],
  is_active: false,
  phase_count: 4,
  agent_count: 8,
  created_at: "2026-01-24T10:00:00Z",
};

const mockMethodology2: MethodologyResponse = {
  id: "gsd-method",
  name: "GSD (Get Shit Done)",
  description: "Spec-driven development with wave-based parallelization",
  agent_profiles: ["gsd-planner", "gsd-executor", "gsd-verifier"],
  skills: ["skills/wave-planning", "skills/verification"],
  workflow_id: "gsd-method",
  workflow_name: "GSD (Get Shit Done)",
  phases: [
    {
      id: "plan",
      name: "Plan",
      order: 0,
      description: "Planning phase",
      agent_profiles: ["gsd-planner"],
      column_ids: ["research", "planning"],
    },
  ],
  templates: [],
  is_active: true,
  phase_count: 4,
  agent_count: 11,
  created_at: "2026-01-24T08:00:00Z",
};

const mockActivationResponse: MethodologyActivationResponse = {
  methodology: { ...mockMethodology, is_active: true },
  workflow: {
    id: "bmad-method",
    name: "BMAD Method",
    description: "Breakthrough Method for Agile AI-Driven Development",
    column_count: 10,
  },
  agent_profiles: ["bmad-analyst", "bmad-pm", "bmad-architect"],
  skills: ["skills/prd-creation", "skills/architecture-design"],
  previous_methodology_id: "gsd-method",
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

describe("methodologyKeys", () => {
  it("should generate correct key for all", () => {
    expect(methodologyKeys.all).toEqual(["methodologies"]);
  });

  it("should generate correct key for lists", () => {
    expect(methodologyKeys.lists()).toEqual(["methodologies", "list"]);
  });

  it("should generate correct key for active", () => {
    expect(methodologyKeys.active()).toEqual(["methodologies", "active"]);
  });
});

describe("useMethodologies", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch all methodologies successfully", async () => {
    const mockMethodologies = [mockMethodology, mockMethodology2];
    vi.mocked(methodologiesApi.getMethodologies).mockResolvedValueOnce(mockMethodologies);

    const { result } = renderHook(() => useMethodologies(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockMethodologies);
    expect(methodologiesApi.getMethodologies).toHaveBeenCalledTimes(1);
  });

  it("should return empty array when no methodologies exist", async () => {
    vi.mocked(methodologiesApi.getMethodologies).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useMethodologies(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([]);
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch methodologies");
    vi.mocked(methodologiesApi.getMethodologies).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useMethodologies(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useActiveMethodology", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch active methodology successfully", async () => {
    vi.mocked(methodologiesApi.getActiveMethodology).mockResolvedValueOnce(mockMethodology2);

    const { result } = renderHook(() => useActiveMethodology(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockMethodology2);
    expect(methodologiesApi.getActiveMethodology).toHaveBeenCalledTimes(1);
  });

  it("should return null when no methodology is active", async () => {
    vi.mocked(methodologiesApi.getActiveMethodology).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useActiveMethodology(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch active methodology");
    vi.mocked(methodologiesApi.getActiveMethodology).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useActiveMethodology(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useActivateMethodology", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should activate a methodology successfully", async () => {
    vi.mocked(methodologiesApi.activateMethodology).mockResolvedValueOnce(
      mockActivationResponse
    );

    const { result } = renderHook(() => useActivateMethodology(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync("bmad-method");
    });

    expect(methodologiesApi.activateMethodology).toHaveBeenCalled();
    expect(vi.mocked(methodologiesApi.activateMethodology).mock.calls[0][0]).toBe(
      "bmad-method"
    );
  });

  it("should return activation response with workflow info", async () => {
    vi.mocked(methodologiesApi.activateMethodology).mockResolvedValueOnce(
      mockActivationResponse
    );

    const { result } = renderHook(() => useActivateMethodology(), {
      wrapper: createWrapper(),
    });

    let response: MethodologyActivationResponse | undefined;
    await act(async () => {
      response = await result.current.mutateAsync("bmad-method");
    });

    expect(response?.methodology.id).toBe("bmad-method");
    expect(response?.workflow.name).toBe("BMAD Method");
    expect(response?.previous_methodology_id).toBe("gsd-method");
  });

  it("should handle activation error", async () => {
    const error = new Error("Failed to activate methodology");
    vi.mocked(methodologiesApi.activateMethodology).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useActivateMethodology(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync("bmad-method");
      })
    ).rejects.toThrow("Failed to activate methodology");
  });
});

describe("useDeactivateMethodology", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should deactivate a methodology successfully", async () => {
    const deactivatedMethodology = { ...mockMethodology2, is_active: false };
    vi.mocked(methodologiesApi.deactivateMethodology).mockResolvedValueOnce(
      deactivatedMethodology
    );

    const { result } = renderHook(() => useDeactivateMethodology(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync("gsd-method");
    });

    expect(methodologiesApi.deactivateMethodology).toHaveBeenCalled();
    expect(vi.mocked(methodologiesApi.deactivateMethodology).mock.calls[0][0]).toBe(
      "gsd-method"
    );
  });

  it("should handle deactivation error", async () => {
    const error = new Error("Failed to deactivate methodology");
    vi.mocked(methodologiesApi.deactivateMethodology).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useDeactivateMethodology(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync("gsd-method");
      })
    ).rejects.toThrow("Failed to deactivate methodology");
  });
});
