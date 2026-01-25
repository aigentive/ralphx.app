/**
 * ReviewsPanel component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReviewsPanel } from "./ReviewsPanel";
import type { ReviewResponse } from "@/lib/tauri";

// Mock the hooks
vi.mock("@/hooks/useReviews", () => ({
  usePendingReviews: vi.fn(),
}));

vi.mock("@/hooks/useGitDiff", () => ({
  useGitDiff: vi.fn(() => ({
    changes: [],
    commits: [],
    isLoadingChanges: false,
    isLoadingHistory: false,
    error: null,
    fetchDiff: vi.fn(),
    refresh: vi.fn(),
  })),
}));

vi.mock("@/stores/taskStore", () => ({
  useTaskStore: vi.fn(() => ({})),
}));

// Mock DiffViewer to avoid complex rendering
vi.mock("@/components/diff", () => ({
  DiffViewer: ({ onFetchDiff: _onFetchDiff, ...props }: { onFetchDiff: unknown }) => (
    <div data-testid="mock-diff-viewer" data-props={JSON.stringify(props)}>
      DiffViewer
    </div>
  ),
}));

import { usePendingReviews } from "@/hooks/useReviews";
import { useGitDiff } from "@/hooks/useGitDiff";

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

    it("renders All, AI, Human tabs with counts", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      expect(screen.getByRole("tab", { name: /all.*\(3\)/i })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: /ai.*\(2\)/i })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: /human.*\(1\)/i })).toBeInTheDocument();
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

    it("filters to AI reviews when AI tab clicked", async () => {
      const user = userEvent.setup();
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      await user.click(screen.getByRole("tab", { name: /ai.*\(2\)/i }));

      await waitFor(() => {
        expect(screen.getByTestId("review-card-review-1")).toBeInTheDocument();
        expect(screen.queryByTestId("review-card-review-2")).not.toBeInTheDocument();
        expect(screen.getByTestId("review-card-review-3")).toBeInTheDocument();
      });
    });

    it("filters to Human reviews when Human tab clicked", async () => {
      const user = userEvent.setup();
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      await user.click(screen.getByRole("tab", { name: /human.*\(1\)/i }));

      await waitFor(() => {
        expect(screen.queryByTestId("review-card-review-1")).not.toBeInTheDocument();
        expect(screen.getByTestId("review-card-review-2")).toBeInTheDocument();
        expect(screen.queryByTestId("review-card-review-3")).not.toBeInTheDocument();
      });
    });

    it("shows empty state for filter with no matching reviews", async () => {
      const user = userEvent.setup();
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

      await user.click(screen.getByRole("tab", { name: /human.*\(0\)/i }));

      await waitFor(() => {
        expect(screen.getByTestId("reviews-panel-empty")).toBeInTheDocument();
      });
    });

    it("highlights active tab using data-state attribute", async () => {
      const user = userEvent.setup();
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      const allTab = screen.getByRole("tab", { name: /all.*\(3\)/i });
      expect(allTab).toHaveAttribute("data-state", "active");

      await user.click(screen.getByRole("tab", { name: /ai.*\(2\)/i }));

      await waitFor(() => {
        const aiTab = screen.getByRole("tab", { name: /ai.*\(2\)/i });
        expect(aiTab).toHaveAttribute("data-state", "active");
        expect(allTab).toHaveAttribute("data-state", "inactive");
      });
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
    it("applies design system background color via class", () => {
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
      expect(panel).toHaveClass("bg-[var(--bg-surface)]");
    });
  });

  describe("DiffViewer integration", () => {
    const reviewWithDiff = createMockReview({ id: "review-1", task_id: "task-1" });

    beforeEach(() => {
      vi.mocked(usePendingReviews).mockReturnValue({
        data: [reviewWithDiff],
        isLoading: false,
        error: null,
        isEmpty: false,
        count: 1,
        refetch: vi.fn(),
      });

      vi.mocked(useGitDiff).mockReturnValue({
        changes: [
          { path: "src/auth.ts", status: "modified", additions: 10, deletions: 5 },
        ],
        commits: [
          {
            sha: "abc123",
            shortSha: "abc123",
            message: "feat: add auth",
            author: "Claude",
            date: new Date(),
          },
        ],
        isLoadingChanges: false,
        isLoadingHistory: false,
        error: null,
        fetchDiff: vi.fn(),
        refresh: vi.fn(),
      });
    });

    it("switches to detail view when View Diff is clicked", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      // Click View Diff button
      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));

      // Should show detail view with DiffViewer
      expect(screen.getByTestId("review-detail-view")).toBeInTheDocument();
      expect(screen.getByTestId("mock-diff-viewer")).toBeInTheDocument();
    });

    it("shows task title in detail view header", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));

      expect(screen.getByTestId("review-detail-title")).toHaveTextContent(
        "Add user authentication"
      );
    });

    it("shows back button in detail view", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));

      expect(screen.getByTestId("review-detail-back")).toBeInTheDocument();
    });

    it("returns to list view when back button is clicked", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      // Go to detail view
      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));
      expect(screen.getByTestId("review-detail-view")).toBeInTheDocument();

      // Click back
      fireEvent.click(screen.getByTestId("review-detail-back"));

      // Should be back to list view
      expect(screen.queryByTestId("review-detail-view")).not.toBeInTheDocument();
      expect(screen.getByTestId("review-card-review-1")).toBeInTheDocument();
    });

    it("shows approve and request changes buttons in detail view for pending reviews", () => {
      const onApprove = vi.fn();
      const onRequestChanges = vi.fn();

      render(
        <ReviewsPanel
          projectId="project-1"
          taskTitles={mockTaskTitles}
          onApprove={onApprove}
          onRequestChanges={onRequestChanges}
        />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));

      expect(screen.getByTestId("review-detail-approve")).toBeInTheDocument();
      expect(screen.getByTestId("review-detail-request-changes")).toBeInTheDocument();
    });

    it("calls onApprove from detail view", () => {
      const onApprove = vi.fn();

      render(
        <ReviewsPanel
          projectId="project-1"
          taskTitles={mockTaskTitles}
          onApprove={onApprove}
        />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));
      fireEvent.click(screen.getByTestId("review-detail-approve"));

      expect(onApprove).toHaveBeenCalledWith("review-1");
    });

    it("calls onRequestChanges from detail view", () => {
      const onRequestChanges = vi.fn();

      render(
        <ReviewsPanel
          projectId="project-1"
          taskTitles={mockTaskTitles}
          onRequestChanges={onRequestChanges}
        />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));
      fireEvent.click(screen.getByTestId("review-detail-request-changes"));

      expect(onRequestChanges).toHaveBeenCalledWith("review-1");
    });

    it("calls external onViewDiff callback when provided", () => {
      const onViewDiff = vi.fn();

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

    it("uses useGitDiff hook with correct task ID", () => {
      render(
        <ReviewsPanel projectId="project-1" taskTitles={mockTaskTitles} />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));

      expect(useGitDiff).toHaveBeenCalledWith({
        taskId: "task-1",
        enabled: true,
      });
    });

    it("hides action buttons for non-pending reviews in detail view", () => {
      const approvedReview = createMockReview({
        id: "review-1",
        task_id: "task-1",
        status: "approved",
      });

      vi.mocked(usePendingReviews).mockReturnValue({
        data: [approvedReview],
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
          onApprove={vi.fn()}
          onRequestChanges={vi.fn()}
        />,
        { wrapper: createWrapper() }
      );

      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));

      expect(screen.queryByTestId("review-detail-approve")).not.toBeInTheDocument();
      expect(screen.queryByTestId("review-detail-request-changes")).not.toBeInTheDocument();
    });
  });
});
