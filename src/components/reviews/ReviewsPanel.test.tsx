import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ReviewsPanel } from "./ReviewsPanel";
import type { Task } from "@/types/task";

vi.mock("@/hooks/useReviews", () => ({
  useTasksAwaitingReview: vi.fn(),
}));

vi.mock("./ReviewDetailModal", () => ({
  ReviewDetailModal: ({ taskId, onClose }: { taskId: string; onClose: () => void }) => (
    <div data-testid="review-detail-modal">
      <span data-testid="review-detail-task-id">{taskId}</span>
      <button onClick={onClose}>Close modal</button>
    </div>
  ),
}));

import { useTasksAwaitingReview } from "@/hooks/useReviews";

function createTask(overrides: Partial<Task> = {}): Task {
  return {
    id: "task-1",
    projectId: "project-1",
    category: "feature",
    title: "Add authentication",
    description: "Implement auth flow",
    priority: 1,
    internalStatus: "pending_review",
    needsReviewPoint: false,
    createdAt: "2026-01-24T12:00:00Z",
    updatedAt: "2026-01-24T12:00:00Z",
    startedAt: null,
    completedAt: null,
    archivedAt: null,
    blockedReason: null,
    taskBranch: null,
    worktreePath: null,
    mergeCommitSha: null,
    metadata: null,
    ...overrides,
  };
}

describe("ReviewsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows loading state", () => {
    vi.mocked(useTasksAwaitingReview).mockReturnValue({
      allTasks: [],
      aiTasks: [],
      humanTasks: [],
      isLoading: true,
      isEmpty: true,
      aiCount: 0,
      humanCount: 0,
      totalCount: 0,
      refetch: vi.fn(),
    });

    render(<ReviewsPanel projectId="project-1" />);

    expect(screen.getByTestId("reviews-panel-loading")).toBeInTheDocument();
  });

  it("shows empty state when no tasks await review", () => {
    vi.mocked(useTasksAwaitingReview).mockReturnValue({
      allTasks: [],
      aiTasks: [],
      humanTasks: [],
      isLoading: false,
      isEmpty: true,
      aiCount: 0,
      humanCount: 0,
      totalCount: 0,
      refetch: vi.fn(),
    });

    render(<ReviewsPanel projectId="project-1" />);

    expect(screen.getByTestId("reviews-panel-empty")).toBeInTheDocument();
    expect(screen.getByText(/no pending reviews/i)).toBeInTheDocument();
  });

  it("renders review cards from tasks awaiting review", () => {
    const aiTask = createTask({ id: "task-ai", title: "AI review task", internalStatus: "pending_review" });
    const humanTask = createTask({ id: "task-human", title: "Human review task", internalStatus: "review_passed" });

    vi.mocked(useTasksAwaitingReview).mockReturnValue({
      allTasks: [aiTask, humanTask],
      aiTasks: [aiTask],
      humanTasks: [humanTask],
      isLoading: false,
      isEmpty: false,
      aiCount: 1,
      humanCount: 1,
      totalCount: 2,
      refetch: vi.fn(),
    });

    render(<ReviewsPanel projectId="project-1" />);

    expect(screen.getByTestId("task-review-card-task-ai")).toBeInTheDocument();
    expect(screen.getByTestId("task-review-card-task-human")).toBeInTheDocument();
    expect(screen.getByText("AI review task")).toBeInTheDocument();
    expect(screen.getByText("Human review task")).toBeInTheDocument();
  });

  it("filters cards by tab", async () => {
    const user = userEvent.setup();
    const aiTask = createTask({ id: "task-ai", title: "AI review task", internalStatus: "reviewing" });
    const humanTask = createTask({ id: "task-human", title: "Human review task", internalStatus: "escalated" });

    vi.mocked(useTasksAwaitingReview).mockReturnValue({
      allTasks: [aiTask, humanTask],
      aiTasks: [aiTask],
      humanTasks: [humanTask],
      isLoading: false,
      isEmpty: false,
      aiCount: 1,
      humanCount: 1,
      totalCount: 2,
      refetch: vi.fn(),
    });

    render(<ReviewsPanel projectId="project-1" />);

    await user.click(screen.getByRole("tab", { name: /human/i }));

    expect(screen.queryByTestId("task-review-card-task-ai")).not.toBeInTheDocument();
    expect(screen.getByTestId("task-review-card-task-human")).toBeInTheDocument();
  });

  it("opens detail modal when review button is clicked", async () => {
    const user = userEvent.setup();
    const aiTask = createTask({ id: "task-ai", title: "AI review task", internalStatus: "pending_review" });

    vi.mocked(useTasksAwaitingReview).mockReturnValue({
      allTasks: [aiTask],
      aiTasks: [aiTask],
      humanTasks: [],
      isLoading: false,
      isEmpty: false,
      aiCount: 1,
      humanCount: 0,
      totalCount: 1,
      refetch: vi.fn(),
    });

    render(<ReviewsPanel projectId="project-1" />);

    await user.click(screen.getByTestId("review-button-task-ai"));

    expect(screen.getByTestId("review-detail-modal")).toBeInTheDocument();
    expect(screen.getByTestId("review-detail-task-id")).toHaveTextContent("task-ai");
  });

  it("calls onClose from close button and Escape key", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();

    vi.mocked(useTasksAwaitingReview).mockReturnValue({
      allTasks: [],
      aiTasks: [],
      humanTasks: [],
      isLoading: false,
      isEmpty: true,
      aiCount: 0,
      humanCount: 0,
      totalCount: 0,
      refetch: vi.fn(),
    });

    render(<ReviewsPanel projectId="project-1" onClose={onClose} />);

    await user.click(screen.getByTestId("reviews-panel-close"));
    expect(onClose).toHaveBeenCalledTimes(1);

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
