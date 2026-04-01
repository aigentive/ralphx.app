import { describe, it, expect, beforeEach } from "vitest";
import {
  useReviewStore,
  selectPendingReviewsList,
  selectReviewById,
  selectSelectedReview,
  selectPendingReviewCount,
  selectIsReviewSelected,
} from "./reviewStore";
import type { ReviewResponse } from "@/lib/tauri";

// Helper to create mock review response
const createMockReview = (overrides: Partial<ReviewResponse> = {}): ReviewResponse => ({
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

describe("reviewStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useReviewStore.setState({
      pendingReviews: {},
      selectedReviewId: null,
      isLoading: false,
      error: null,
    });
  });

  describe("setPendingReviews", () => {
    it("sets all pending reviews and indexes by ID", () => {
      const reviews = [
        createMockReview({ id: "r1" }),
        createMockReview({ id: "r2" }),
      ];

      useReviewStore.getState().setPendingReviews(reviews);

      const state = useReviewStore.getState();
      expect(Object.keys(state.pendingReviews)).toHaveLength(2);
      expect(state.pendingReviews["r1"]?.id).toBe("r1");
      expect(state.pendingReviews["r2"]?.id).toBe("r2");
    });

    it("replaces existing reviews", () => {
      useReviewStore.setState({
        pendingReviews: { old: createMockReview({ id: "old" }) },
      });
      const reviews = [createMockReview({ id: "new" })];

      useReviewStore.getState().setPendingReviews(reviews);

      const state = useReviewStore.getState();
      expect(state.pendingReviews["old"]).toBeUndefined();
      expect(state.pendingReviews["new"]).toBeDefined();
    });

    it("clears loading state", () => {
      useReviewStore.setState({ isLoading: true });

      useReviewStore.getState().setPendingReviews([]);

      expect(useReviewStore.getState().isLoading).toBe(false);
    });
  });

  describe("setReview", () => {
    it("adds a new review", () => {
      const review = createMockReview({ id: "r1" });

      useReviewStore.getState().setReview(review);

      expect(useReviewStore.getState().pendingReviews["r1"]).toBeDefined();
    });

    it("updates an existing review", () => {
      useReviewStore.setState({
        pendingReviews: { r1: createMockReview({ id: "r1", notes: null }) },
      });
      const updated = createMockReview({ id: "r1", notes: "Updated notes" });

      useReviewStore.getState().setReview(updated);

      expect(useReviewStore.getState().pendingReviews["r1"]?.notes).toBe("Updated notes");
    });
  });

  describe("removeReview", () => {
    it("removes a review by ID", () => {
      useReviewStore.setState({
        pendingReviews: {
          r1: createMockReview({ id: "r1" }),
          r2: createMockReview({ id: "r2" }),
        },
      });

      useReviewStore.getState().removeReview("r1");

      const state = useReviewStore.getState();
      expect(state.pendingReviews["r1"]).toBeUndefined();
      expect(state.pendingReviews["r2"]).toBeDefined();
    });

    it("clears selection if removed review was selected", () => {
      useReviewStore.setState({
        pendingReviews: { r1: createMockReview({ id: "r1" }) },
        selectedReviewId: "r1",
      });

      useReviewStore.getState().removeReview("r1");

      expect(useReviewStore.getState().selectedReviewId).toBeNull();
    });

    it("does not clear selection if different review was selected", () => {
      useReviewStore.setState({
        pendingReviews: {
          r1: createMockReview({ id: "r1" }),
          r2: createMockReview({ id: "r2" }),
        },
        selectedReviewId: "r2",
      });

      useReviewStore.getState().removeReview("r1");

      expect(useReviewStore.getState().selectedReviewId).toBe("r2");
    });
  });

  describe("selectReview", () => {
    it("selects a review by ID", () => {
      useReviewStore.getState().selectReview("r1");

      expect(useReviewStore.getState().selectedReviewId).toBe("r1");
    });

    it("clears selection when null", () => {
      useReviewStore.setState({ selectedReviewId: "r1" });

      useReviewStore.getState().selectReview(null);

      expect(useReviewStore.getState().selectedReviewId).toBeNull();
    });
  });

  describe("setLoading", () => {
    it("sets loading state to true", () => {
      useReviewStore.getState().setLoading(true);

      expect(useReviewStore.getState().isLoading).toBe(true);
    });

    it("sets loading state to false", () => {
      useReviewStore.setState({ isLoading: true });

      useReviewStore.getState().setLoading(false);

      expect(useReviewStore.getState().isLoading).toBe(false);
    });
  });

  describe("setError", () => {
    it("sets error message and clears loading", () => {
      useReviewStore.setState({ isLoading: true });

      useReviewStore.getState().setError("Something went wrong");

      const state = useReviewStore.getState();
      expect(state.error).toBe("Something went wrong");
      expect(state.isLoading).toBe(false);
    });

    it("clears error when null", () => {
      useReviewStore.setState({ error: "Previous error" });

      useReviewStore.getState().setError(null);

      expect(useReviewStore.getState().error).toBeNull();
    });
  });

  describe("clearReviews", () => {
    it("clears all reviews and selection", () => {
      useReviewStore.setState({
        pendingReviews: {
          r1: createMockReview({ id: "r1" }),
          r2: createMockReview({ id: "r2" }),
        },
        selectedReviewId: "r1",
      });

      useReviewStore.getState().clearReviews();

      const state = useReviewStore.getState();
      expect(Object.keys(state.pendingReviews)).toHaveLength(0);
      expect(state.selectedReviewId).toBeNull();
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useReviewStore.setState({
      pendingReviews: {},
      selectedReviewId: null,
      isLoading: false,
      error: null,
    });
  });

  describe("selectPendingReviewsList", () => {
    it("returns all pending reviews as array", () => {
      useReviewStore.setState({
        pendingReviews: {
          r1: createMockReview({ id: "r1" }),
          r2: createMockReview({ id: "r2" }),
        },
      });

      const result = selectPendingReviewsList(useReviewStore.getState());

      expect(result).toHaveLength(2);
    });

    it("returns empty array when no reviews", () => {
      const result = selectPendingReviewsList(useReviewStore.getState());

      expect(result).toEqual([]);
    });
  });

  describe("selectReviewById", () => {
    it("returns review when exists", () => {
      const review = createMockReview({ id: "r1" });
      useReviewStore.setState({ pendingReviews: { r1: review } });

      const result = selectReviewById("r1")(useReviewStore.getState());

      expect(result?.id).toBe("r1");
    });

    it("returns null when review does not exist", () => {
      const result = selectReviewById("nonexistent")(useReviewStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectSelectedReview", () => {
    it("returns selected review", () => {
      useReviewStore.setState({
        pendingReviews: { r1: createMockReview({ id: "r1" }) },
        selectedReviewId: "r1",
      });

      const result = selectSelectedReview(useReviewStore.getState());

      expect(result?.id).toBe("r1");
    });

    it("returns null when no review selected", () => {
      const result = selectSelectedReview(useReviewStore.getState());

      expect(result).toBeNull();
    });

    it("returns null when selected review not found", () => {
      useReviewStore.setState({ selectedReviewId: "nonexistent" });

      const result = selectSelectedReview(useReviewStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectPendingReviewCount", () => {
    it("returns count of pending reviews", () => {
      useReviewStore.setState({
        pendingReviews: {
          r1: createMockReview({ id: "r1" }),
          r2: createMockReview({ id: "r2" }),
          r3: createMockReview({ id: "r3" }),
        },
      });

      const result = selectPendingReviewCount(useReviewStore.getState());

      expect(result).toBe(3);
    });

    it("returns 0 when no reviews", () => {
      const result = selectPendingReviewCount(useReviewStore.getState());

      expect(result).toBe(0);
    });
  });

  describe("selectIsReviewSelected", () => {
    it("returns true when review is selected", () => {
      useReviewStore.setState({ selectedReviewId: "r1" });

      const result = selectIsReviewSelected("r1")(useReviewStore.getState());

      expect(result).toBe(true);
    });

    it("returns false when different review is selected", () => {
      useReviewStore.setState({ selectedReviewId: "r2" });

      const result = selectIsReviewSelected("r1")(useReviewStore.getState());

      expect(result).toBe(false);
    });

    it("returns false when no review is selected", () => {
      const result = selectIsReviewSelected("r1")(useReviewStore.getState());

      expect(result).toBe(false);
    });
  });
});
