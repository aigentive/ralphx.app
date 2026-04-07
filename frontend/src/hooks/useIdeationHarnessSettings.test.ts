import { createElement } from "react";
import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ideationHarnessApi } from "@/api/ideation-harness";
import type { IdeationHarnessLaneView } from "@/api/ideation-harness";
import { useIdeationHarnessSettings } from "./useIdeationHarnessSettings";

vi.mock("@/api/ideation-harness", () => ({
  ideationHarnessApi: {
    get: vi.fn(),
    update: vi.fn(),
  },
  defaultIdeationHarnessLanes: [
    {
      lane: "ideation_primary",
      row: null,
      configuredHarness: null,
      effectiveHarness: "claude",
      fallbackHarness: null,
      fallbackActivated: false,
      binaryPath: null,
      binaryFound: false,
      probeSucceeded: false,
      available: false,
      missingCoreExecFeatures: [],
      error: null,
    },
  ],
}));

const globalLanes: IdeationHarnessLaneView[] = [
  {
    lane: "ideation_primary",
    row: null,
    configuredHarness: null,
    effectiveHarness: "claude",
    fallbackHarness: null,
    fallbackActivated: false,
    binaryPath: "/usr/local/bin/claude",
    binaryFound: true,
    probeSucceeded: true,
    available: true,
    missingCoreExecFeatures: [],
    error: null,
  },
  {
    lane: "ideation_verifier",
    row: null,
    configuredHarness: null,
    effectiveHarness: "claude",
    fallbackHarness: null,
    fallbackActivated: false,
    binaryPath: "/usr/local/bin/claude",
    binaryFound: true,
    probeSucceeded: true,
    available: true,
    missingCoreExecFeatures: [],
    error: null,
  },
];

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

describe("useIdeationHarnessSettings", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = createTestClient();
    vi.mocked(ideationHarnessApi.get).mockResolvedValue(globalLanes);
    vi.mocked(ideationHarnessApi.update).mockResolvedValue({
      lane: "ideation_primary",
      harness: "codex",
      model: "gpt-5.4",
      effort: "xhigh",
      approvalPolicy: "on-request",
      sandboxMode: "workspace-write",
      fallbackHarness: "claude",
      updatedAt: new Date().toISOString(),
      projectId: null,
    });
  });

  it("loads merged lane data", async () => {
    const { result } = renderHook(() => useIdeationHarnessSettings(null), {
      wrapper: createWrapper(queryClient),
    });

    await waitFor(() => {
      expect(result.current.isPlaceholderData).toBe(false);
    });

    expect(result.current.lanes).toEqual(globalLanes);
  });

  it("invalidates ideation harness queries after a successful update", async () => {
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");
    const { result } = renderHook(() => useIdeationHarnessSettings(null), {
      wrapper: createWrapper(queryClient),
    });

    await act(async () => {
      result.current.updateLane({
        lane: "ideation_primary",
        harness: "codex",
        model: "gpt-5.4",
        effort: "xhigh",
        approvalPolicy: "on-request",
        sandboxMode: "workspace-write",
        fallbackHarness: "claude",
      });
    });

    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalledWith(
        expect.objectContaining({ queryKey: ["ideation", "harness"] }),
      );
    });
  });
});
