import { createElement } from "react";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { vi, describe, it, expect, beforeEach } from "vitest";
import { useIdeationModelSettings } from "./useIdeationModelSettings";
import { ideationModelApi } from "@/api/ideation-model";
import type { IdeationModelResponse } from "@/api/ideation-model";

vi.mock("@/api/ideation-model", () => ({
  ideationModelApi: {
    get: vi.fn(),
    update: vi.fn(),
  },
  defaultIdeationModelSettings: {
    primaryModel: "inherit",
    verifierModel: "inherit",
    effectivePrimaryModel: "",
    effectiveVerifierModel: "",
    primaryModelSource: "",
    verifierModelSource: "",
  },
}));

const globalSettings: IdeationModelResponse = {
  primaryModel: "claude-3-5-sonnet",
  verifierModel: "claude-3-5-haiku",
  effectivePrimaryModel: "claude-3-5-sonnet",
  effectiveVerifierModel: "claude-3-5-haiku",
  primaryModelSource: "global",
  verifierModelSource: "global",
};

const inheritSettings: IdeationModelResponse = {
  primaryModel: "inherit",
  verifierModel: "inherit",
  effectivePrimaryModel: "claude-3-5-sonnet",
  effectiveVerifierModel: "claude-3-5-haiku",
  primaryModelSource: "global",
  verifierModelSource: "global",
};

const explicitSettings: IdeationModelResponse = {
  primaryModel: "claude-3-opus",
  verifierModel: "claude-3-haiku",
  effectivePrimaryModel: "claude-3-opus",
  effectiveVerifierModel: "claude-3-haiku",
  primaryModelSource: "project",
  verifierModelSource: "project",
};

function createTestClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
}

function createWrapper(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

describe("useIdeationModelSettings", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = createTestClient();
    // Default: global returns globalSettings, per-project returns inheritSettings
    vi.mocked(ideationModelApi.get).mockImplementation((projectId) => {
      if (projectId === null) return Promise.resolve(globalSettings);
      return Promise.resolve(inheritSettings);
    });
    vi.mocked(ideationModelApi.update).mockResolvedValue({
      ...globalSettings,
      primaryModel: "claude-3-opus",
      effectivePrimaryModel: "claude-3-opus",
    });
  });

  it("invalidates all ['ideation', 'model'] queries on successful global mutation", async () => {
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");
    const updatedSettings: IdeationModelResponse = {
      ...globalSettings,
      primaryModel: "claude-3-opus",
      effectivePrimaryModel: "claude-3-opus",
    };
    vi.mocked(ideationModelApi.update).mockResolvedValue(updatedSettings);

    const { result } = renderHook(() => useIdeationModelSettings(null), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      result.current.updateSettings({ primaryModel: "claude-3-opus" });
    });

    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalledWith(
        expect.objectContaining({ queryKey: ["ideation", "model"] }),
      );
    });
  });

  it("refreshes inherit-row effective values after global mutation succeeds", async () => {
    const wrapper = createWrapper(queryClient);
    const updatedGlobal: IdeationModelResponse = {
      ...globalSettings,
      primaryModel: "claude-3-opus",
      effectivePrimaryModel: "claude-3-opus",
    };
    const updatedInherit: IdeationModelResponse = {
      ...inheritSettings,
      effectivePrimaryModel: "claude-3-opus",
    };

    const { result: globalResult } = renderHook(
      () => useIdeationModelSettings(null),
      { wrapper },
    );
    const { result: inheritResult } = renderHook(
      () => useIdeationModelSettings("proj-1"),
      { wrapper },
    );

    // Wait for initial data to load for both hooks
    await waitFor(() => {
      expect(globalResult.current.isPlaceholderData).toBe(false);
      expect(inheritResult.current.isPlaceholderData).toBe(false);
    });

    expect(inheritResult.current.settings.effectivePrimaryModel).toBe(
      "claude-3-5-sonnet",
    );

    // After global mutation, refetch will return updated effective values for inherit row
    vi.mocked(ideationModelApi.update).mockResolvedValue(updatedGlobal);
    vi.mocked(ideationModelApi.get).mockImplementation((projectId) => {
      if (projectId === "proj-1") return Promise.resolve(updatedInherit);
      return Promise.resolve(updatedGlobal);
    });

    await act(async () => {
      globalResult.current.updateSettings({ primaryModel: "claude-3-opus" });
    });

    // After invalidation, the inherit row refetches and shows updated effective value
    await waitFor(() => {
      expect(inheritResult.current.settings.effectivePrimaryModel).toBe(
        "claude-3-opus",
      );
    });
  });

  it("does not overwrite explicit-override row; retains primaryModelSource/verifierModelSource", async () => {
    const wrapper = createWrapper(queryClient);
    vi.mocked(ideationModelApi.get).mockImplementation((projectId) => {
      if (projectId === "proj-2") return Promise.resolve(explicitSettings);
      return Promise.resolve(globalSettings);
    });

    const { result: globalResult } = renderHook(
      () => useIdeationModelSettings(null),
      { wrapper },
    );
    const { result: proj2Result } = renderHook(
      () => useIdeationModelSettings("proj-2"),
      { wrapper },
    );

    // Wait for proj-2 data to load
    await waitFor(() => {
      expect(proj2Result.current.isPlaceholderData).toBe(false);
    });

    expect(proj2Result.current.settings.primaryModelSource).toBe("project");

    const setQueryDataSpy = vi.spyOn(queryClient, "setQueryData");
    const updatedGlobal: IdeationModelResponse = {
      ...globalSettings,
      primaryModel: "claude-3-opus",
      effectivePrimaryModel: "claude-3-opus",
    };
    vi.mocked(ideationModelApi.update).mockResolvedValue(updatedGlobal);
    // Explicit override row refetches and still returns project-level values
    vi.mocked(ideationModelApi.get).mockImplementation((projectId) => {
      if (projectId === "proj-2") return Promise.resolve(explicitSettings);
      return Promise.resolve(updatedGlobal);
    });

    await act(async () => {
      globalResult.current.updateSettings({ primaryModel: "claude-3-opus" });
    });

    await waitFor(() => {
      expect(globalResult.current.isUpdating).toBe(false);
    });

    // setQueryData was only called for the mutated global key (null), not for proj-2
    const proj2SetCalls = setQueryDataSpy.mock.calls.filter(([key]) => {
      return Array.isArray(key) && key[2] === "proj-2";
    });
    expect(proj2SetCalls).toHaveLength(0);

    // Explicit override row source fields are preserved after global change
    await waitFor(() => {
      expect(proj2Result.current.settings.primaryModelSource).toBe("project");
      expect(proj2Result.current.settings.verifierModelSource).toBe("project");
    });
  });

  it("rolls back to previous state when mutation errors", async () => {
    vi.mocked(ideationModelApi.update).mockRejectedValue(
      new Error("API error"),
    );

    const { result } = renderHook(() => useIdeationModelSettings(null), {
      wrapper: createWrapper(queryClient),
    });

    // Wait for initial query data to load (not placeholder)
    await waitFor(() => {
      expect(result.current.isPlaceholderData).toBe(false);
    });

    expect(result.current.settings.primaryModel).toBe("claude-3-5-sonnet");

    await act(async () => {
      result.current.updateSettings({ primaryModel: "claude-3-opus" });
    });

    // Wait for mutation error and rollback
    await waitFor(() => {
      expect(result.current.saveError).not.toBeNull();
    });

    // Settings should be rolled back to the pre-mutation state
    expect(result.current.settings.primaryModel).toBe("claude-3-5-sonnet");
    expect(result.current.settings.effectivePrimaryModel).toBe(
      "claude-3-5-sonnet",
    );
  });
});
