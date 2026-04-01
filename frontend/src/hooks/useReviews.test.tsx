import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import {
  usePendingReviews,
  useReviewsByTaskId,
  useTaskStateHistory,
  reviewKeys,
} from "./useReviews";
import { useReviewStore } from "@/stores/reviewStore";
import { api } from "@/lib/tauri";
import type { ReviewResponse, ReviewNoteResponse } from "@/lib/tauri";

// Mock the Tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    reviews: {
      getPending: vi.fn(),
      getByTaskId: vi.fn(),
      getTaskStateHistory: vi.fn(),
    },
  },
}));

const mockApi = api as {
  reviews: {
    getPending: ReturnType<typeof vi.fn>;
    getByTaskId: ReturnType<typeof vi.fn>;
    getTaskStateHistory: ReturnType<typeof vi.fn>;
  };
};

// Helper to create mock review response
const createMockReviewResponse = (
  overrides: Partial<ReviewResponse> = {}
): ReviewResponse => ({
  id: "review-1",
  project_id: "project-1",
  task_id: "task-1",
  reviewer_type: "ai",
  status: "pending",
  notes: null,
  created_at: "2026-01-24T12:00:00Z",
  completed_at: null,
  ...overrides,
});

// Helper to create mock review note response
const createMockReviewNoteResponse = (
  overrides: Partial<ReviewNoteResponse> = {}
): ReviewNoteResponse => ({
  id: "note-1",
  task_id: "task-1",
  reviewer: "ai",
  outcome: "approved",
  notes: "Looks good",
  created_at: "2026-01-24T12:00:00Z",
  ...overrides,
});

// Create wrapper with QueryClient
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("reviewKeys", () => {
  it("creates correct query keys", () => {
    expect(reviewKeys.all).toEqual(["reviews"]);
    expect(reviewKeys.pending()).toEqual(["reviews", "pending"]);
    expect(reviewKeys.pendingByProject("project-1")).toEqual([
      "reviews",
      "pending",
      "project-1",
    ]);
    expect(reviewKeys.byTask()).toEqual(["reviews", "byTask"]);
    expect(reviewKeys.byTaskId("task-1")).toEqual(["reviews", "byTask", "task-1"]);
    expect(reviewKeys.stateHistory()).toEqual(["reviews", "stateHistory"]);
    expect(reviewKeys.stateHistoryById("task-1")).toEqual([
      "reviews",
      "stateHistory",
      "task-1",
    ]);
  });
});

describe("usePendingReviews", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useReviewStore.setState({
      pendingReviews: {},
      selectedReviewId: null,
      isLoading: false,
      error: null,
    });
  });

  it("fetches pending reviews on mount", async () => {
    const reviews = [
      createMockReviewResponse({ id: "review-1" }),
      createMockReviewResponse({ id: "review-2" }),
    ];
    mockApi.reviews.getPending.mockResolvedValue(reviews);

    const { result } = renderHook(() => usePendingReviews("project-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(mockApi.reviews.getPending).toHaveBeenCalledWith("project-1");
    expect(result.current.data).toHaveLength(2);
  });

  it("returns empty array when no pending reviews", async () => {
    mockApi.reviews.getPending.mockResolvedValue([]);

    const { result } = renderHook(() => usePendingReviews("project-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual([]);
    expect(result.current.isEmpty).toBe(true);
  });

  it("computes isEmpty correctly", async () => {
    const reviews = [createMockReviewResponse()];
    mockApi.reviews.getPending.mockResolvedValue(reviews);

    const { result } = renderHook(() => usePendingReviews("project-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.isEmpty).toBe(false);
  });

  it("computes count correctly", async () => {
    const reviews = [
      createMockReviewResponse({ id: "review-1" }),
      createMockReviewResponse({ id: "review-2" }),
      createMockReviewResponse({ id: "review-3" }),
    ];
    mockApi.reviews.getPending.mockResolvedValue(reviews);

    const { result } = renderHook(() => usePendingReviews("project-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.count).toBe(3);
  });

  it("handles fetch errors", async () => {
    mockApi.reviews.getPending.mockRejectedValue(new Error("Fetch failed"));

    const { result } = renderHook(() => usePendingReviews("project-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Fetch failed");
    });
  });

  it("does not fetch when disabled", async () => {
    const { result } = renderHook(
      () => usePendingReviews("project-1", { enabled: false }),
      {
        wrapper: createWrapper(),
      }
    );

    expect(result.current.isLoading).toBe(false);
    expect(mockApi.reviews.getPending).not.toHaveBeenCalled();
  });

  it("does not fetch without projectId", async () => {
    const { result } = renderHook(() => usePendingReviews(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(mockApi.reviews.getPending).not.toHaveBeenCalled();
  });

  it("syncs data to store", async () => {
    const reviews = [createMockReviewResponse({ id: "review-1" })];
    mockApi.reviews.getPending.mockResolvedValue(reviews);

    renderHook(() => usePendingReviews("project-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      const state = useReviewStore.getState();
      expect(state.pendingReviews["review-1"]).toBeDefined();
    });
  });
});

describe("useReviewsByTaskId", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useReviewStore.setState({
      pendingReviews: {},
      selectedReviewId: null,
      isLoading: false,
      error: null,
    });
  });

  it("fetches reviews for a task", async () => {
    const reviews = [
      createMockReviewResponse({ id: "review-1", task_id: "task-1" }),
      createMockReviewResponse({
        id: "review-2",
        task_id: "task-1",
        reviewer_type: "human",
      }),
    ];
    mockApi.reviews.getByTaskId.mockResolvedValue(reviews);

    const { result } = renderHook(() => useReviewsByTaskId("task-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(mockApi.reviews.getByTaskId).toHaveBeenCalledWith("task-1");
    expect(result.current.data).toHaveLength(2);
  });

  it("returns empty array when no reviews exist", async () => {
    mockApi.reviews.getByTaskId.mockResolvedValue([]);

    const { result } = renderHook(() => useReviewsByTaskId("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual([]);
  });

  it("handles fetch errors", async () => {
    mockApi.reviews.getByTaskId.mockRejectedValue(new Error("Fetch failed"));

    const { result } = renderHook(() => useReviewsByTaskId("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Fetch failed");
    });
  });

  it("does not fetch when disabled", async () => {
    const { result } = renderHook(
      () => useReviewsByTaskId("task-1", { enabled: false }),
      {
        wrapper: createWrapper(),
      }
    );

    expect(result.current.isLoading).toBe(false);
    expect(mockApi.reviews.getByTaskId).not.toHaveBeenCalled();
  });

  it("does not fetch without taskId", async () => {
    const { result } = renderHook(() => useReviewsByTaskId(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(mockApi.reviews.getByTaskId).not.toHaveBeenCalled();
  });

  it("computes hasAiReview correctly", async () => {
    const reviews = [
      createMockReviewResponse({ id: "review-1", reviewer_type: "ai" }),
    ];
    mockApi.reviews.getByTaskId.mockResolvedValue(reviews);

    const { result } = renderHook(() => useReviewsByTaskId("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.hasAiReview).toBe(true);
    expect(result.current.hasHumanReview).toBe(false);
  });

  it("computes hasHumanReview correctly", async () => {
    const reviews = [
      createMockReviewResponse({ id: "review-1", reviewer_type: "human" }),
    ];
    mockApi.reviews.getByTaskId.mockResolvedValue(reviews);

    const { result } = renderHook(() => useReviewsByTaskId("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.hasHumanReview).toBe(true);
    expect(result.current.hasAiReview).toBe(false);
  });

  it("gets latest review correctly", async () => {
    const reviews = [
      createMockReviewResponse({
        id: "review-1",
        created_at: "2026-01-24T10:00:00Z",
      }),
      createMockReviewResponse({
        id: "review-2",
        created_at: "2026-01-24T12:00:00Z",
      }),
    ];
    mockApi.reviews.getByTaskId.mockResolvedValue(reviews);

    const { result } = renderHook(() => useReviewsByTaskId("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    // Latest review should be review-2 (later timestamp)
    expect(result.current.latestReview?.id).toBe("review-2");
  });
});

describe("useTaskStateHistory", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useReviewStore.setState({
      pendingReviews: {},
      selectedReviewId: null,
      isLoading: false,
      error: null,
    });
  });

  it("fetches state history for a task", async () => {
    const history = [
      createMockReviewNoteResponse({
        id: "note-1",
        created_at: "2026-01-24T10:00:00Z",
      }),
      createMockReviewNoteResponse({
        id: "note-2",
        created_at: "2026-01-24T12:00:00Z",
      }),
    ];
    mockApi.reviews.getTaskStateHistory.mockResolvedValue(history);

    const { result } = renderHook(() => useTaskStateHistory("task-1"), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(mockApi.reviews.getTaskStateHistory).toHaveBeenCalledWith("task-1");
    expect(result.current.data).toHaveLength(2);
  });

  it("returns empty array when no history", async () => {
    mockApi.reviews.getTaskStateHistory.mockResolvedValue([]);

    const { result } = renderHook(() => useTaskStateHistory("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.data).toEqual([]);
    expect(result.current.isEmpty).toBe(true);
  });

  it("computes isEmpty correctly", async () => {
    const history = [createMockReviewNoteResponse()];
    mockApi.reviews.getTaskStateHistory.mockResolvedValue(history);

    const { result } = renderHook(() => useTaskStateHistory("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.isEmpty).toBe(false);
  });

  it("handles fetch errors", async () => {
    mockApi.reviews.getTaskStateHistory.mockRejectedValue(
      new Error("Fetch failed")
    );

    const { result } = renderHook(() => useTaskStateHistory("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.error).toBe("Fetch failed");
    });
  });

  it("does not fetch when disabled", async () => {
    const { result } = renderHook(
      () => useTaskStateHistory("task-1", { enabled: false }),
      {
        wrapper: createWrapper(),
      }
    );

    expect(result.current.isLoading).toBe(false);
    expect(mockApi.reviews.getTaskStateHistory).not.toHaveBeenCalled();
  });

  it("does not fetch without taskId", async () => {
    const { result } = renderHook(() => useTaskStateHistory(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(false);
    expect(mockApi.reviews.getTaskStateHistory).not.toHaveBeenCalled();
  });

  it("sorts history by created_at descending", async () => {
    const history = [
      createMockReviewNoteResponse({
        id: "note-1",
        created_at: "2026-01-24T10:00:00Z",
      }),
      createMockReviewNoteResponse({
        id: "note-3",
        created_at: "2026-01-24T14:00:00Z",
      }),
      createMockReviewNoteResponse({
        id: "note-2",
        created_at: "2026-01-24T12:00:00Z",
      }),
    ];
    mockApi.reviews.getTaskStateHistory.mockResolvedValue(history);

    const { result } = renderHook(() => useTaskStateHistory("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    // Should be sorted newest first
    expect(result.current.data[0]?.id).toBe("note-3");
    expect(result.current.data[1]?.id).toBe("note-2");
    expect(result.current.data[2]?.id).toBe("note-1");
  });

  it("gets latest entry correctly", async () => {
    const history = [
      createMockReviewNoteResponse({
        id: "note-1",
        outcome: "changes_requested",
        created_at: "2026-01-24T10:00:00Z",
      }),
      createMockReviewNoteResponse({
        id: "note-2",
        outcome: "approved",
        created_at: "2026-01-24T12:00:00Z",
      }),
    ];
    mockApi.reviews.getTaskStateHistory.mockResolvedValue(history);

    const { result } = renderHook(() => useTaskStateHistory("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.latestEntry?.outcome).toBe("approved");
  });
});
