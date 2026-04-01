import { createElement } from "react";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { vi, describe, it, expect, beforeEach } from "vitest";
import { useIdeationEffortSettings } from "./useIdeationEffortSettings";
import { ideationEffortApi } from "@/api/ideation-effort";
import type { IdeationEffortResponse } from "@/api/ideation-effort";

vi.mock("@/api/ideation-effort", () => ({
  ideationEffortApi: {
    get: vi.fn(),
    update: vi.fn(),
  },
  defaultIdeationEffortSettings: {
    primaryEffort: "inherit",
    verifierEffort: "inherit",
    effectivePrimary: "",
    effectiveVerifier: "",
    primarySource: "",
    verifierSource: "",
  },
}));

const globalSettings: IdeationEffortResponse = {
  primaryEffort: "normal",
  verifierEffort: "normal",
  effectivePrimary: "normal",
  effectiveVerifier: "normal",
  primarySource: "global",
  verifierSource: "global",
};

const inheritSettings: IdeationEffortResponse = {
  primaryEffort: "inherit",
  verifierEffort: "inherit",
  effectivePrimary: "normal",
  effectiveVerifier: "normal",
  primarySource: "global",
  verifierSource: "global",
};

const explicitSettings: IdeationEffortResponse = {
  primaryEffort: "high",
  verifierEffort: "low",
  effectivePrimary: "high",
  effectiveVerifier: "low",
  primarySource: "project",
  verifierSource: "project",
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

describe("useIdeationEffortSettings", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = createTestClient();
    // Default: global returns globalSettings, per-project returns inheritSettings
    vi.mocked(ideationEffortApi.get).mockImplementation((projectId) => {
      if (projectId === null) return Promise.resolve(globalSettings);
      return Promise.resolve(inheritSettings);
    });
    vi.mocked(ideationEffortApi.update).mockResolvedValue({
      ...globalSettings,
      primaryEffort: "high",
      effectivePrimary: "high",
    });
  });

  it("invalidates all ['ideation', 'effort'] queries on successful global mutation", async () => {
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");
    const updatedSettings: IdeationEffortResponse = {
      ...globalSettings,
      primaryEffort: "high",
      effectivePrimary: "high",
    };
    vi.mocked(ideationEffortApi.update).mockResolvedValue(updatedSettings);

    const { result } = renderHook(() => useIdeationEffortSettings(null), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      result.current.updateSettings({ primaryEffort: "high" });
    });

    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalledWith(
        expect.objectContaining({ queryKey: ["ideation", "effort"] }),
      );
    });
  });

  it("refreshes inherit-row effective values after global mutation succeeds", async () => {
    const wrapper = createWrapper(queryClient);
    const updatedGlobal: IdeationEffortResponse = {
      ...globalSettings,
      primaryEffort: "high",
      effectivePrimary: "high",
    };
    const updatedInherit: IdeationEffortResponse = {
      ...inheritSettings,
      effectivePrimary: "high",
    };

    const { result: globalResult } = renderHook(
      () => useIdeationEffortSettings(null),
      { wrapper },
    );
    const { result: inheritResult } = renderHook(
      () => useIdeationEffortSettings("proj-1"),
      { wrapper },
    );

    // Wait for initial data to load for both hooks
    await waitFor(() => {
      expect(globalResult.current.isPlaceholderData).toBe(false);
      expect(inheritResult.current.isPlaceholderData).toBe(false);
    });

    expect(inheritResult.current.settings.effectivePrimary).toBe("normal");

    // After global mutation, refetch will return updated effective values for inherit row
    vi.mocked(ideationEffortApi.update).mockResolvedValue(updatedGlobal);
    vi.mocked(ideationEffortApi.get).mockImplementation((projectId) => {
      if (projectId === "proj-1") return Promise.resolve(updatedInherit);
      return Promise.resolve(updatedGlobal);
    });

    await act(async () => {
      globalResult.current.updateSettings({ primaryEffort: "high" });
    });

    // After invalidation, the inherit row refetches and shows updated effective value
    await waitFor(() => {
      expect(inheritResult.current.settings.effectivePrimary).toBe("high");
    });
  });

  it("does not overwrite explicit-override row; retains primarySource/verifierSource", async () => {
    const wrapper = createWrapper(queryClient);
    vi.mocked(ideationEffortApi.get).mockImplementation((projectId) => {
      if (projectId === "proj-2") return Promise.resolve(explicitSettings);
      return Promise.resolve(globalSettings);
    });

    const { result: globalResult } = renderHook(
      () => useIdeationEffortSettings(null),
      { wrapper },
    );
    const { result: proj2Result } = renderHook(
      () => useIdeationEffortSettings("proj-2"),
      { wrapper },
    );

    // Wait for proj-2 data to load
    await waitFor(() => {
      expect(proj2Result.current.isPlaceholderData).toBe(false);
    });

    expect(proj2Result.current.settings.primarySource).toBe("project");

    const setQueryDataSpy = vi.spyOn(queryClient, "setQueryData");
    const updatedGlobal: IdeationEffortResponse = {
      ...globalSettings,
      primaryEffort: "high",
      effectivePrimary: "high",
    };
    vi.mocked(ideationEffortApi.update).mockResolvedValue(updatedGlobal);
    // Explicit override row refetches and still returns project-level values
    vi.mocked(ideationEffortApi.get).mockImplementation((projectId) => {
      if (projectId === "proj-2") return Promise.resolve(explicitSettings);
      return Promise.resolve(updatedGlobal);
    });

    await act(async () => {
      globalResult.current.updateSettings({ primaryEffort: "high" });
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
      expect(proj2Result.current.settings.primarySource).toBe("project");
      expect(proj2Result.current.settings.verifierSource).toBe("project");
    });
  });

  it("rolls back to previous state when mutation errors", async () => {
    vi.mocked(ideationEffortApi.update).mockRejectedValue(
      new Error("API error"),
    );

    const { result } = renderHook(() => useIdeationEffortSettings(null), {
      wrapper: createWrapper(queryClient),
    });

    // Wait for initial query data to load (not placeholder)
    await waitFor(() => {
      expect(result.current.isPlaceholderData).toBe(false);
    });

    expect(result.current.settings.primaryEffort).toBe("normal");

    await act(async () => {
      result.current.updateSettings({ primaryEffort: "high" });
    });

    // Wait for mutation error and rollback
    await waitFor(() => {
      expect(result.current.saveError).not.toBeNull();
    });

    // Settings should be rolled back to the pre-mutation state
    expect(result.current.settings.primaryEffort).toBe("normal");
    expect(result.current.settings.effectivePrimary).toBe("normal");
  });
});
