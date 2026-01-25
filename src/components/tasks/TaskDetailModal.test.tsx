/**
 * Tests for TaskDetailModal component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskDetailModal } from "./TaskDetailModal";
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

describe("TaskDetailModal", () => {
  const mockOnClose = vi.fn();

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

  describe("modal behavior", () => {
    it("should render when isOpen is true", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(screen.getByTestId("task-detail-modal")).toBeInTheDocument();
    });

    it("should not render when isOpen is false", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={false} onClose={mockOnClose} />
      );
      expect(screen.queryByTestId("task-detail-modal")).not.toBeInTheDocument();
    });

    it("should not render when task is null", () => {
      renderWithProviders(
        <TaskDetailModal task={null} isOpen={true} onClose={mockOnClose} />
      );
      expect(screen.queryByTestId("task-detail-modal")).not.toBeInTheDocument();
    });

    it("should call onClose when close button is clicked", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      const closeButton = screen.getByTestId("task-detail-close");
      fireEvent.click(closeButton);
      expect(mockOnClose).toHaveBeenCalledTimes(1);
    });
  });

  describe("content rendering", () => {
    it("should render task title", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(screen.getByTestId("task-detail-title")).toHaveTextContent(
        "Add user authentication"
      );
    });

    it("should render task description", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(screen.getByTestId("task-detail-description")).toHaveTextContent(
        "Implement login and registration flow"
      );
    });

    it("should render task category", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(screen.getByTestId("task-detail-category")).toHaveTextContent(
        "feature"
      );
    });

    it("should render priority badge with correct level", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(screen.getByTestId("task-detail-priority")).toHaveTextContent(
        "P1"
      );
    });

    it("should render status badge with current status", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge).toBeInTheDocument();
      expect(statusBadge).toHaveAttribute("data-status", "pending_review");
    });

    it("should show empty description message when description is null", () => {
      const taskWithNullDesc = { ...mockTask, description: null };
      renderWithProviders(
        <TaskDetailModal
          task={taskWithNullDesc}
          isOpen={true}
          onClose={mockOnClose}
        />
      );
      expect(
        screen.queryByTestId("task-detail-description")
      ).not.toBeInTheDocument();
      expect(screen.getByText("No description provided")).toBeInTheDocument();
    });
  });

  describe("reviews section", () => {
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

      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(
        screen.getByTestId("task-detail-reviews-section")
      ).toBeInTheDocument();
    });

    it("should not render reviews section when no reviews", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(
        screen.queryByTestId("task-detail-reviews-section")
      ).not.toBeInTheDocument();
    });

    it("should show loading spinner when reviews are loading", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [],
        isLoading: true,
        hasAiReview: false,
        hasHumanReview: false,
        latestReview: null,
      });

      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(screen.getByTestId("reviews-loading")).toBeInTheDocument();
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

      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
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

      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(screen.getByText("Human Review")).toBeInTheDocument();
    });
  });

  describe("fix tasks indicator", () => {
    it("should display fix task indicator when fixTaskCount is provided", () => {
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

      renderWithProviders(
        <TaskDetailModal
          task={mockTask}
          isOpen={true}
          onClose={mockOnClose}
          fixTaskCount={1}
        />
      );
      expect(screen.getByTestId("fix-task-indicator")).toBeInTheDocument();
      expect(screen.getByText("1 fix task")).toBeInTheDocument();
    });

    it("should show plural for multiple fix tasks", () => {
      mockUseReviewsByTaskId.mockReturnValue({
        data: [
          {
            id: "review-1",
            project_id: "project-1",
            task_id: "task-123",
            reviewer_type: "ai",
            status: "changes_requested",
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

      renderWithProviders(
        <TaskDetailModal
          task={mockTask}
          isOpen={true}
          onClose={mockOnClose}
          fixTaskCount={3}
        />
      );
      expect(screen.getByText("3 fix tasks")).toBeInTheDocument();
    });
  });

  describe("history section", () => {
    it("should render history section", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(
        screen.getByTestId("task-detail-history-section")
      ).toBeInTheDocument();
    });

    it("should pass taskId to StateHistoryTimeline via hook", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      expect(mockUseTaskStateHistory).toHaveBeenCalledWith("task-123");
    });
  });

  describe("priority badge colors", () => {
    it("should render P1 badge with error color", () => {
      const p1Task = { ...mockTask, priority: 1 };
      renderWithProviders(
        <TaskDetailModal task={p1Task} isOpen={true} onClose={mockOnClose} />
      );
      const badge = screen.getByTestId("task-detail-priority");
      expect(badge.style.backgroundColor).toBe("var(--status-error)");
    });

    it("should render P2 badge with accent color", () => {
      const p2Task = { ...mockTask, priority: 2 };
      renderWithProviders(
        <TaskDetailModal task={p2Task} isOpen={true} onClose={mockOnClose} />
      );
      const badge = screen.getByTestId("task-detail-priority");
      expect(badge.style.backgroundColor).toBe("var(--accent-primary)");
    });

    it("should render P3 badge with warning color", () => {
      const p3Task = { ...mockTask, priority: 3 };
      renderWithProviders(
        <TaskDetailModal task={p3Task} isOpen={true} onClose={mockOnClose} />
      );
      const badge = screen.getByTestId("task-detail-priority");
      expect(badge.style.backgroundColor).toBe("var(--status-warning)");
    });

    it("should render P4 badge with hover color", () => {
      const p4Task = { ...mockTask, priority: 4 };
      renderWithProviders(
        <TaskDetailModal task={p4Task} isOpen={true} onClose={mockOnClose} />
      );
      const badge = screen.getByTestId("task-detail-priority");
      expect(badge.style.backgroundColor).toBe("var(--bg-hover)");
    });
  });

  describe("status badge colors", () => {
    it("should apply appropriate color for approved status", () => {
      const approvedTask = { ...mockTask, internalStatus: "approved" as const };
      renderWithProviders(
        <TaskDetailModal
          task={approvedTask}
          isOpen={true}
          onClose={mockOnClose}
        />
      );
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge.style.backgroundColor).toBe(
        "rgba(16, 185, 129, 0.15)"
      );
    });

    it("should apply appropriate color for failed status", () => {
      const failedTask = { ...mockTask, internalStatus: "failed" as const };
      renderWithProviders(
        <TaskDetailModal
          task={failedTask}
          isOpen={true}
          onClose={mockOnClose}
        />
      );
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge.style.backgroundColor).toBe("rgba(239, 68, 68, 0.15)");
    });

    it("should apply appropriate color for executing status", () => {
      const executingTask = {
        ...mockTask,
        internalStatus: "executing" as const,
      };
      renderWithProviders(
        <TaskDetailModal
          task={executingTask}
          isOpen={true}
          onClose={mockOnClose}
        />
      );
      const statusBadge = screen.getByTestId("task-detail-status");
      expect(statusBadge.style.backgroundColor).toBe(
        "rgba(255, 107, 53, 0.15)"
      );
    });
  });

  describe("data attributes", () => {
    it("should have data-task-id on view container", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      const container = screen.getByTestId("task-detail-view");
      expect(container).toHaveAttribute("data-task-id", "task-123");
    });
  });

  describe("accessibility", () => {
    it("should have aria-label on close button", () => {
      renderWithProviders(
        <TaskDetailModal task={mockTask} isOpen={true} onClose={mockOnClose} />
      );
      const closeButton = screen.getByTestId("task-detail-close");
      expect(closeButton).toHaveAttribute("aria-label", "Close");
    });
  });
});
