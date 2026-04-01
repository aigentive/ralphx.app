/**
 * usePriorityAssessment hook tests
 *
 * Tests for usePriorityAssessment hook for priority assessment mutations
 * using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { usePriorityAssessment } from "./usePriorityAssessment";
import { ideationApi } from "@/api/ideation";
import type { PriorityAssessmentResponse } from "@/api/ideation";

// Mock the ideation API
vi.mock("@/api/ideation", () => ({
  ideationApi: {
    proposals: {
      assessPriority: vi.fn(),
      assessAllPriorities: vi.fn(),
    },
  },
}));

// Create mock data
const mockAssessment1: PriorityAssessmentResponse = {
  proposalId: "proposal-1",
  priority: "high",
  score: 75,
  reason: "Blocks 3 other tasks",
};

const mockAssessment2: PriorityAssessmentResponse = {
  proposalId: "proposal-2",
  priority: "medium",
  score: 50,
  reason: "No dependencies",
};

const mockAssessment3: PriorityAssessmentResponse = {
  proposalId: "proposal-3",
  priority: "low",
  score: 25,
  reason: "Optional enhancement",
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

describe("usePriorityAssessment", () => {
  describe("assessPriority", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    afterEach(() => {
      vi.resetAllMocks();
    });

    it("should assess priority for a single proposal", async () => {
      vi.mocked(ideationApi.proposals.assessPriority).mockResolvedValueOnce(
        mockAssessment1
      );

      const { result } = renderHook(() => usePriorityAssessment(), {
        wrapper: createWrapper(),
      });

      let assessment: PriorityAssessmentResponse | undefined;
      await act(async () => {
        assessment = await result.current.assessPriority.mutateAsync("proposal-1");
      });

      expect(assessment).toEqual(mockAssessment1);
      expect(ideationApi.proposals.assessPriority).toHaveBeenCalledWith("proposal-1");
    });

    it("should handle assessment error", async () => {
      const error = new Error("Failed to assess priority");
      vi.mocked(ideationApi.proposals.assessPriority).mockRejectedValueOnce(error);

      const { result } = renderHook(() => usePriorityAssessment(), {
        wrapper: createWrapper(),
      });

      await expect(
        act(async () => {
          await result.current.assessPriority.mutateAsync("proposal-1");
        })
      ).rejects.toThrow("Failed to assess priority");
    });

    it("should set loading state during assessment", async () => {
      let resolvePromise: (value: PriorityAssessmentResponse) => void;
      const promise = new Promise<PriorityAssessmentResponse>((resolve) => {
        resolvePromise = resolve;
      });
      vi.mocked(ideationApi.proposals.assessPriority).mockReturnValueOnce(promise);

      const { result } = renderHook(() => usePriorityAssessment(), {
        wrapper: createWrapper(),
      });

      expect(result.current.assessPriority.isPending).toBe(false);

      act(() => {
        result.current.assessPriority.mutate("proposal-1");
      });

      await waitFor(() => {
        expect(result.current.assessPriority.isPending).toBe(true);
      });

      await act(async () => {
        resolvePromise!(mockAssessment1);
        await promise;
      });

      await waitFor(() => {
        expect(result.current.assessPriority.isPending).toBe(false);
      });
    });
  });

  describe("assessAllPriorities", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    afterEach(() => {
      vi.resetAllMocks();
    });

    it("should assess priorities for all proposals in session", async () => {
      const mockAssessments = [mockAssessment1, mockAssessment2, mockAssessment3];
      vi.mocked(ideationApi.proposals.assessAllPriorities).mockResolvedValueOnce(
        mockAssessments
      );

      const { result } = renderHook(() => usePriorityAssessment(), {
        wrapper: createWrapper(),
      });

      let assessments: PriorityAssessmentResponse[] | undefined;
      await act(async () => {
        assessments = await result.current.assessAllPriorities.mutateAsync("session-1");
      });

      expect(assessments).toEqual(mockAssessments);
      expect(ideationApi.proposals.assessAllPriorities).toHaveBeenCalledWith("session-1");
    });

    it("should handle empty session gracefully", async () => {
      vi.mocked(ideationApi.proposals.assessAllPriorities).mockResolvedValueOnce([]);

      const { result } = renderHook(() => usePriorityAssessment(), {
        wrapper: createWrapper(),
      });

      let assessments: PriorityAssessmentResponse[] | undefined;
      await act(async () => {
        assessments = await result.current.assessAllPriorities.mutateAsync("session-1");
      });

      expect(assessments).toEqual([]);
    });

    it("should handle batch assessment error", async () => {
      const error = new Error("Failed to assess all priorities");
      vi.mocked(ideationApi.proposals.assessAllPriorities).mockRejectedValueOnce(error);

      const { result } = renderHook(() => usePriorityAssessment(), {
        wrapper: createWrapper(),
      });

      await expect(
        act(async () => {
          await result.current.assessAllPriorities.mutateAsync("session-1");
        })
      ).rejects.toThrow("Failed to assess all priorities");
    });

    it("should set loading state during batch assessment", async () => {
      let resolvePromise: (value: PriorityAssessmentResponse[]) => void;
      const promise = new Promise<PriorityAssessmentResponse[]>((resolve) => {
        resolvePromise = resolve;
      });
      vi.mocked(ideationApi.proposals.assessAllPriorities).mockReturnValueOnce(promise);

      const { result } = renderHook(() => usePriorityAssessment(), {
        wrapper: createWrapper(),
      });

      expect(result.current.assessAllPriorities.isPending).toBe(false);

      act(() => {
        result.current.assessAllPriorities.mutate("session-1");
      });

      await waitFor(() => {
        expect(result.current.assessAllPriorities.isPending).toBe(true);
      });

      await act(async () => {
        resolvePromise!([mockAssessment1]);
        await promise;
      });

      await waitFor(() => {
        expect(result.current.assessAllPriorities.isPending).toBe(false);
      });
    });
  });

  describe("combined usage", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    afterEach(() => {
      vi.resetAllMocks();
    });

    it("should provide both mutations independently", async () => {
      vi.mocked(ideationApi.proposals.assessPriority).mockResolvedValueOnce(
        mockAssessment1
      );
      vi.mocked(ideationApi.proposals.assessAllPriorities).mockResolvedValueOnce([
        mockAssessment2,
        mockAssessment3,
      ]);

      const { result } = renderHook(() => usePriorityAssessment(), {
        wrapper: createWrapper(),
      });

      // Both mutations should be available
      expect(result.current.assessPriority).toBeDefined();
      expect(result.current.assessAllPriorities).toBeDefined();

      // Can use them independently
      let single: PriorityAssessmentResponse | undefined;
      let batch: PriorityAssessmentResponse[] | undefined;

      await act(async () => {
        single = await result.current.assessPriority.mutateAsync("proposal-1");
        batch = await result.current.assessAllPriorities.mutateAsync("session-1");
      });

      expect(single).toEqual(mockAssessment1);
      expect(batch).toEqual([mockAssessment2, mockAssessment3]);
    });
  });
});
