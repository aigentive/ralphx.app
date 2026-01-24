/**
 * ReviewsPanel component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReviewsPanel } from "./ReviewsPanel";
import type { ReviewResponse } from "@/lib/tauri";

// Mock the hooks
vi.mock("@/hooks/useReviews", () => ({
  usePendingReviews: vi.fn(),
}));

vi.mock("@/stores/taskStore", () => ({
  useTaskStore: vi.fn(() => ({})),
}));

import { usePendingReviews } from "@/hooks/useReviews";

const createMockReview = (overrides: Partial<ReviewResponse> = {}): ReviewResponse => ({
  id: `review-${Math.random().toString(36).slice(2)}`,
  project_id: "project-1",
  task_id: "task-1",
  reviewer_type: "ai",
  status: "pending",
  notes: null,
  created_at: "2026-01-24T12:00:00Z",
  completed_at: null,
  ...overrides,
});

const mockTaskTitles: Record<string, string> = {
  "task-1": "Add user authentication",
  "task-2": "Fix login validation",
  "task-3": "Update API endpoints",
};

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

describe("ReviewsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("loading state", () => {
    it("shows loading indicator when loading", () => {
      vi.mocked(usePendingReviews).mockReturnValue({
        data: [],
        isLoading: true,
        error: null,
        isEmpty: true,
        count: 0,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByTestId("reviews-panel-loading")).toBeInTheDocument();
    });
  });

  describe("empty state", () => {
    it("shows empty state when no pending reviews", () => {
      vi.mocked(usePendingReviews).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
        isEmpty: true,
        count: 0,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByTestId("reviews-panel-empty")).toBeInTheDocument();
      expect(screen.getByText(/no pending reviews/i)).toBeInTheDocument();
    });
  });

  describe("reviews list", () => {
    it("renders review cards for each pending review", () => {
      const reviews = [
        createMockReview({ id: "review-1", task_id: "task-1" }),
        createMockReview({ id: "review-2", task_id: "task-2" }),
      ];

      vi.mocked(usePendingReviews).mockReturnValue({
        data: reviews,
        isLoading: false,
        error: null,
        isEmpty: false,
        count: 2,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByTestId("review-card-review-1")).toBeInTheDocument();
      expect(screen.getByTestId("review-card-review-2")).toBeInTheDocument();
    });

    it("displays task titles in review cards", () => {
      const reviews = [
        createMockReview({ id: "review-1", task_id: "task-1" }),
      ];

      vi.mocked(usePendingReviews).mockReturnValue({
        data: reviews,
        isLoading: false,
        error: null,
        isEmpty: false,
        count: 1,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByText("Add user authentication")).toBeInTheDocument();
    });
  });

  describe("filter tabs", () => {
    const mixedReviews = [
      createMockReview({ id: "review-1", task_id: "task-1", reviewer_type: "ai" }),
      createMockReview({ id: "review-2", task_id: "task-2", reviewer_type: "human" }),
      createMockReview({ id: "review-3", task_id: "task-3", reviewer_type: "ai" }),
    ];

    beforeEach(() => {
      vi.mocked(usePendingReviews).mockReturnValue({
        data: mixedReviews,
        isLoading: false,
        error: null,
        isEmpty: false,
        count: 3,
        refetch: vi.fn(),
      });
    });

    it("renders All, AI Review, Human Review tabs", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByRole("tab", { name: /all/i })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: /ai review/i })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: /human review/i })).toBeInTheDocument();
    });

    it("shows all reviews when All tab is active (default)", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByTestId("review-card-review-1")).toBeInTheDocument();
      expect(screen.getByTestId("review-card-review-2")).toBeInTheDocument();
      expect(screen.getByTestId("review-card-review-3")).toBeInTheDocument();
    });

    it("filters to AI reviews when AI Review tab clicked", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("tab", { name: /ai review/i }));

      expect(screen.getByTestId("review-card-review-1")).toBeInTheDocument();
      expect(screen.queryByTestId("review-card-review-2")).not.toBeInTheDocument();
      expect(screen.getByTestId("review-card-review-3")).toBeInTheDocument();
    });

    it("filters to Human reviews when Human Review tab clicked", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("tab", { name: /human review/i }));

      expect(screen.queryByTestId("review-card-review-1")).not.toBeInTheDocument();
      expect(screen.getByTestId("review-card-review-2")).toBeInTheDocument();
      expect(screen.queryByTestId("review-card-review-3")).not.toBeInTheDocument();
    });

    it("shows empty state for filter with no matching reviews", () => {
      const aiOnlyReviews = [
        createMockReview({ id: "review-1", task_id: "task-1", reviewer_type: "ai" }),
      ];

      vi.mocked(usePendingReviews).mockReturnValue({
        data: aiOnlyReviews,
        isLoading: false,
        error: null,
        isEmpty: false,
        count: 1,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("tab", { name: /human review/i }));

      expect(screen.getByTestId("reviews-panel-empty")).toBeInTheDocument();
    });

    it("highlights active tab", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      const allTab = screen.getByRole("tab", { name: /all/i });
      expect(allTab).toHaveAttribute("data-active", "true");

      fireEvent.click(screen.getByRole("tab", { name: /ai review/i }));
      const aiTab = screen.getByRole("tab", { name: /ai review/i });
      expect(aiTab).toHaveAttribute("data-active", "true");
      expect(allTab).toHaveAttribute("data-active", "false");
    });
  });

  describe("action callbacks", () => {
    it("calls onApprove when approve button clicked on card", () => {
      const onApprove = vi.fn();
      const reviews = [createMockReview({ id: "review-1", task_id: "task-1" })];

      vi.mocked(usePendingReviews).mockReturnValue({
        data: reviews,
        isLoading: false,
        error: null,
        isEmpty: false,
        count: 1,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel
          projectId="project-1"
          taskTitles={mockTaskTitles}
          onApprove={onApprove}
        />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /approve/i }));
      expect(onApprove).toHaveBeenCalledWith("review-1");
    });

    it("calls onRequestChanges when request changes button clicked", () => {
      const onRequestChanges = vi.fn();
      const reviews = [createMockReview({ id: "review-1", task_id: "task-1" })];

      vi.mocked(usePendingReviews).mockReturnValue({
        data: reviews,
        isLoading: false,
        error: null,
        isEmpty: false,
        count: 1,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel
          projectId="project-1"
          taskTitles={mockTaskTitles}
          onRequestChanges={onRequestChanges}
        />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /request changes/i }));
      expect(onRequestChanges).toHaveBeenCalledWith("review-1");
    });

    it("calls onViewDiff when view diff button clicked", () => {
      const onViewDiff = vi.fn();
      const reviews = [createMockReview({ id: "review-1", task_id: "task-1" })];

      vi.mocked(usePendingReviews).mockReturnValue({
        data: reviews,
        isLoading: false,
        error: null,
        isEmpty: false,
        count: 1,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel
          projectId="project-1"
          taskTitles={mockTaskTitles}
          onViewDiff={onViewDiff}
        />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));
      expect(onViewDiff).toHaveBeenCalledWith("review-1");
    });
  });

  describe("header", () => {
    it("shows panel title", () => {
      vi.mocked(usePendingReviews).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
        isEmpty: true,
        count: 0,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByTestId("reviews-panel-title")).toHaveTextContent("Reviews");
    });

    it("shows close button when onClose provided", () => {
      const onClose = vi.fn();

      vi.mocked(usePendingReviews).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
        isEmpty: true,
        count: 0,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} onClose={onClose} />,
        { wrapper: createWrapper() }
      );

      const closeButton = screen.getByTestId("reviews-panel-close");
      expect(closeButton).toBeInTheDocument();
      fireEvent.click(closeButton);
      expect(onClose).toHaveBeenCalled();
    });
  });

  describe("data attributes", () => {
    it("sets data-testid on panel container", () => {
      vi.mocked(usePendingReviews).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
        isEmpty: true,
        count: 0,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByTestId("reviews-panel")).toBeInTheDocument();
    });
  });

  describe("styling", () => {
    it("applies design system background color", () => {
      vi.mocked(usePendingReviews).mockReturnValue({
        data: [],
        isLoading: false,
        error: null,
        isEmpty: true,
        count: 0,
        refetch: vi.fn(),
      });

      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      const panel = screen.getByTestId("reviews-panel");
      expect(panel).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });
  });
});
