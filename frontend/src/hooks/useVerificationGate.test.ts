import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { useVerificationGate } from "./useVerificationGate";
import { ideationApi } from "@/api/ideation";
import type { IdeationSessionResponse } from "@/api/ideation";

vi.mock("@/api/ideation", () => ({
  ideationApi: {
    verification: {
      getStatus: vi.fn(),
    },
  },
}));

function createWrapper(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

const baseSession: Pick<
  IdeationSessionResponse,
  "id" | "planArtifactId" | "sessionPurpose" | "verificationStatus" | "verificationInProgress"
> = {
  id: "session-1",
  planArtifactId: "plan-1",
  sessionPurpose: "general",
  verificationStatus: "unverified",
  verificationInProgress: false,
};

describe("useVerificationGate", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("allows accept when verification API reports verified even if the session cache is stale", async () => {
    vi.mocked(ideationApi.verification.getStatus).mockResolvedValueOnce({
      sessionId: "session-1",
      status: "verified",
      inProgress: false,
      gaps: [],
      rounds: [],
      roundDetails: [],
      runHistory: [],
    });

    const queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
          gcTime: 0,
        },
      },
    });

    const { result } = renderHook(() => useVerificationGate(baseSession), {
      wrapper: createWrapper(queryClient),
    });

    await waitFor(() => {
      expect(result.current.canAccept).toBe(true);
    });

    expect(result.current.status).toBe("verified");
    expect(result.current.reason).toBeUndefined();
    expect(ideationApi.verification.getStatus).toHaveBeenCalledWith("session-1");
  });

  it("falls back to unverified when no authoritative verification record exists yet", async () => {
    vi.mocked(ideationApi.verification.getStatus).mockRejectedValueOnce(
      new Error("Failed to get verification status: 404")
    );

    const queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
          gcTime: 0,
        },
      },
    });

    const { result } = renderHook(() => useVerificationGate(baseSession), {
      wrapper: createWrapper(queryClient),
    });

    await waitFor(() => {
      expect(ideationApi.verification.getStatus).toHaveBeenCalledWith("session-1");
    });

    expect(result.current.canAccept).toBe(false);
    expect(result.current.status).toBe("unverified");
    expect(result.current.reason).toBe("Plan has not been verified. Run verification or skip.");
  });
});
