/**
 * Tests for TaskDetailView component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskDetailView } from "./TaskDetailView";
import type { Task } from "@/types/task";

// Mock the hooks
const mockUseReviewsByTaskId = vi.fn();
const mockUseTaskStateHistory = vi.fn();
vi.mock("@/hooks/useReviews", () => ({
  useReviewsByTaskId: (...args: unknown[]) => mockUseReviewsByTaskId(...args),
  useTaskStateHistory: (...args: unknown[]) => mockUseTaskStateHistory(...args),
}));

function createQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
}

function renderWithProviders(ui: React.ReactElement) {
  return render(
    <QueryClientProvider client={createQueryClient()}>{ui}</QueryClientProvider>
  );
}

const mockTask: Task = {
  id: "task-123",
  projectId: "project-1",
  category: "feature",
  title: "Add user authentication",
  description: "Implement login and registration flow",
  priority: 1,
  internalStatus: "pending_review",
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:30:00Z",
  startedAt: "2026-01-24T12:05:00Z",
  completedAt: null,
};

describe("TaskDetailView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseReviewsByTaskId.mockReturnValue({
      data: [],
      isLoading: false,
      hasAiReview: false,
      hasHumanReview: false,
      latestReview: null,
    });
    mockUseTaskStateHistory.mockReturnValue({
      data: [],
      isLoading: false,
      isEmpty: true,
    });
  });

  describe("basic rendering", () => {
    it("should render task title", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByTestId("task-detail-title")).toHaveTextContent("Add user authentication");
    });

    it("should render task description", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByTestId("task-detail-description")).toHaveTextContent("Implement login and registration flow");
    });

    it("should render task category", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByTestId("task-detail-category")).toHaveTextContent("feature");
    });

    it("should render priority indicator", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByTestId("task-detail-priority")).toHaveTextContent("P1");
    });

    it("should render status badge with current status", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge).toBeInTheDocument();
      expect(statusBadge).toHaveAttribute("data-status", "pending_review");
    });
  });

  describe("null description", () => {
    it("should handle null description gracefully", () => {
      const taskWithNullDesc = { ...mockTask, description: null };
      renderWithProviders(<TaskDetailView task={taskWithNullDesc} />);
      expect(screen.queryByTestId("task-detail-description")).not.toBeInTheDocument();
    });
  });

  describe("state history timeline", () => {
    it("should render StateHistoryTimeline component", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "ai" as const,
            outcome: "approved" as const,
            notes: "Looks good",
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByTestId("task-detail-history-section")).toBeInTheDocument();
    });

    it("should pass taskId to StateHistoryTimeline", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(mockUseTaskStateHistory).toHaveBeenCalledWith("task-123");
    });
  });

  describe("associated reviews", () => {
    it("should render reviews section when reviews exist", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [
          {
            id: "review-1",
            project_id: "project-1",
            task_id: "task-123",
            reviewer_type: "ai",
            status: "approved",
            notes: "Verified by AI",
            created_at: new Date().toISOString(),
            completed_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        hasAiReview: true,
        hasHumanReview: false,
        latestReview: null,
      });

      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByTestId("task-detail-reviews-section")).toBeInTheDocument();
    });

    it("should not render reviews section when no reviews", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [],
        isLoading: false,
        hasAiReview: false,
        hasHumanReview: false,
        latestReview: null,
      });

      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.queryByTestId("task-detail-reviews-section")).not.toBeInTheDocument();
    });

    it("should display AI review indicator", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [
          {
            id: "review-1",
            project_id: "project-1",
            task_id: "task-123",
            reviewer_type: "ai",
            status: "approved",
            notes: null,
            created_at: new Date().toISOString(),
            completed_at: null,
          },
        ],
        isLoading: false,
        hasAiReview: true,
        hasHumanReview: false,
        latestReview: null,
      });

      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByText("AI Review")).toBeInTheDocument();
    });

    it("should display human review indicator", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [
          {
            id: "review-1",
            project_id: "project-1",
            task_id: "task-123",
            reviewer_type: "human",
            status: "pending",
            notes: null,
            created_at: new Date().toISOString(),
            completed_at: null,
          },
        ],
        isLoading: false,
        hasAiReview: false,
        hasHumanReview: true,
        latestReview: null,
      });

      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByText("Human Review")).toBeInTheDocument();
    });
  });

  describe("loading states", () => {
    it("should show loading state for reviews", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [],
        isLoading: true,
        hasAiReview: false,
        hasHumanReview: false,
        latestReview: null,
      });

      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByTestId("reviews-loading")).toBeInTheDocument();
    });
  });

  describe("related fix tasks", () => {
    it("should display fix task indicator when review has target task", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [
          {
            id: "review-1",
            project_id: "project-1",
            task_id: "task-123",
            reviewer_type: "ai",
            status: "changes_requested",
            notes: "Found issues",
            created_at: new Date().toISOString(),
            completed_at: null,
          },
        ],
        isLoading: false,
        hasAiReview: true,
        hasHumanReview: false,
        latestReview: null,
      });

      renderWithProviders(<TaskDetailView task={mockTask} fixTaskCount={1} />);
      expect(screen.getByTestId("fix-task-indicator")).toBeInTheDocument();
      expect(screen.getByText("1 fix task")).toBeInTheDocument();
    });

    it("should show plural for multiple fix tasks", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [],
        isLoading: false,
        hasAiReview: false,
        hasHumanReview: false,
        latestReview: null,
      });

      renderWithProviders(<TaskDetailView task={mockTask} fixTaskCount={3} />);
      expect(screen.getByText("3 fix tasks")).toBeInTheDocument();
    });
  });

  describe("data attributes", () => {
    it("should have data-testid on container", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(screen.getByTestId("task-detail-view")).toBeInTheDocument();
    });

    it("should include task id in data attribute", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      const container = screen.getByTestId("task-detail-view");
      expect(container).toHaveAttribute("data-task-id", "task-123");
    });
  });

  describe("styling", () => {
    it("should use design system tokens", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      const container = screen.getByTestId("task-detail-view");
      expect(container.style.backgroundColor).toBe("var(--bg-surface)");
    });

    it("should apply section styling", () => {
      mockUseTaskStateHistory.mockReturnValue({
        data: [
          {
            id: "note-1",
            task_id: "task-123",
            reviewer: "ai" as const,
            outcome: "approved" as const,
            notes: null,
            created_at: new Date().toISOString(),
          },
        ],
        isLoading: false,
        isEmpty: false,
      });

      renderWithProviders(<TaskDetailView task={mockTask} />);
      const historySection = screen.getByTestId("task-detail-history-section");
      expect(historySection).toHaveClass("mt-6");
    });
  });

  describe("hook integration", () => {
    it("should pass taskId to useReviewsByTaskId", () => {
      renderWithProviders(<TaskDetailView task={mockTask} />);
      expect(mockUseReviewsByTaskId).toHaveBeenCalledWith("task-123");
    });
  });

  describe("status display", () => {
    it("should format status label correctly", () => {
      const taskWithExecDone = { ...mockTask, internalStatus: "execution_done" as const };
      renderWithProviders(<TaskDetailView task={taskWithExecDone} />);
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge).toHaveAttribute("data-status", "execution_done");
    });

    it("should apply appropriate color for approved status", () => {
      const approvedTask = { ...mockTask, internalStatus: "approved" as const };
      renderWithProviders(<TaskDetailView task={approvedTask} />);
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge.style.backgroundColor).toBe("var(--status-success)");
    });

    it("should apply appropriate color for failed status", () => {
      const failedTask = { ...mockTask, internalStatus: "failed" as const };
      renderWithProviders(<TaskDetailView task={failedTask} />);
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge.style.backgroundColor).toBe("var(--status-error)");
    });

    it("should apply appropriate color for blocked status", () => {
      const blockedTask = { ...mockTask, internalStatus: "blocked" as const };
      renderWithProviders(<TaskDetailView task={blockedTask} />);
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge.style.backgroundColor).toBe("var(--status-warning)");
    });
  });
});
