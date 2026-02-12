/**
 * BasicTaskDetail component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BasicTaskDetail } from "./BasicTaskDetail";
import type { Task } from "@/types/task";

vi.mock("@/hooks/useTaskSteps", () => ({
  useTaskSteps: vi.fn(),
}));

const mockConfirmation = {
  confirm: vi.fn(async () => true),
  confirmationDialogProps: {},
  ConfirmationDialog: () => null,
};

vi.mock("@/hooks/useConfirmation", () => ({
  useConfirmation: vi.fn(() => mockConfirmation),
}));

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      move: vi.fn(async () => ({})),
    },
  },
}));

const mockStepList = vi.fn(({ taskId, editable, hideCompletionNotes }) => (
  <div
    data-testid="mock-step-list"
    data-task-id={taskId}
    data-editable={String(editable)}
    data-hide-completion-notes={String(hideCompletionNotes)}
  />
));

vi.mock("../StepList", () => ({
  StepList: (props: unknown) => mockStepList(props),
}));

import { useTaskSteps } from "@/hooks/useTaskSteps";
import { api } from "@/lib/tauri";

const mockUseTaskSteps = vi.mocked(useTaskSteps);
const mockApiTasksMove = vi.mocked(api.tasks.move);

function createTestTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task Title",
    description: "Test task description",
    priority: 2,
    internalStatus: "ready",
    needsReviewPoint: false,
    sourceProposalId: null,
    planArtifactId: null,
    createdAt: "2026-01-28T12:00:00+00:00",
    updatedAt: "2026-01-28T12:00:00+00:00",
    startedAt: null,
    completedAt: null,
    archivedAt: null,
    ...overrides,
  };
}

function TestWrapper({ children }: { children: React.ReactNode }) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("BasicTaskDetail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseTaskSteps.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);
  });

  it("renders container and description content", () => {
    const task = createTestTask({ description: "Task description here" });
    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("basic-task-detail")).toBeInTheDocument();
    expect(screen.getByText("Description")).toBeInTheDocument();
    expect(screen.getByText("Task description here")).toBeInTheDocument();
  });

  it("renders placeholder when description is null", () => {
    const task = createTestTask({ description: null });
    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByText(/no description provided/i)).toBeInTheDocument();
  });

  it("shows loading state while fetching steps", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });
    expect(screen.getByTestId("basic-task-steps-loading")).toBeInTheDocument();
  });

  it("renders StepList section when task has steps", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: [
        {
          id: "step-1",
          taskId: task.id,
          title: "Step 1",
          status: "pending",
          order: 0,
          createdAt: "2026-01-28T12:00:00+00:00",
          updatedAt: "2026-01-28T12:00:00+00:00",
        },
      ],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

    expect(screen.getByTestId("basic-task-steps-section")).toBeInTheDocument();
    expect(screen.getByTestId("mock-step-list")).toBeInTheDocument();
    expect(screen.getByTestId("mock-step-list")).toHaveAttribute(
      "data-hide-completion-notes",
      "false"
    );
  });

  it("passes historical mode to StepList", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: [
        {
          id: "step-1",
          taskId: task.id,
          title: "Step 1",
          status: "pending",
          order: 0,
          createdAt: "2026-01-28T12:00:00+00:00",
          updatedAt: "2026-01-28T12:00:00+00:00",
        },
      ],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<BasicTaskDetail task={task} isHistorical />, { wrapper: TestWrapper });
    expect(screen.getByTestId("mock-step-list")).toHaveAttribute(
      "data-hide-completion-notes",
      "true"
    );
  });

  it("shows empty steps state when no steps exist", () => {
    const task = createTestTask();
    mockUseTaskSteps.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);

    render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });
    expect(screen.getByText("No steps defined yet")).toBeInTheDocument();
  });

  describe("failure reason display", () => {
    it("displays failure reason when task is in failed state with metadata", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Connection timeout",
        failure_details: "Task failed to connect to the server",
        is_timeout: true,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-reason-section")).toBeInTheDocument();
      expect(screen.getByText("Failure Reason")).toBeInTheDocument();
      expect(screen.getByText("Connection timeout")).toBeInTheDocument();
      expect(screen.getByText("timeout")).toBeInTheDocument();
      expect(screen.getByText("Task failed to connect to the server")).toBeInTheDocument();
    });

    it("hides failure reason section when task is not failed", () => {
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("failure-reason-section")).not.toBeInTheDocument();
    });

    it("displays failure error without timeout badge when is_timeout is false", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Invalid input",
        failure_details: null,
        is_timeout: false,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByText("Invalid input")).toBeInTheDocument();
      expect(screen.queryByText("timeout")).not.toBeInTheDocument();
    });

    it("displays error without details when failure_details is null", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Process error",
        failure_details: null,
        is_timeout: false,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByText("Process error")).toBeInTheDocument();
      expect(screen.queryByText(/Task failed/)).not.toBeInTheDocument();
    });

    it("handles malformed JSON metadata gracefully", () => {
      const task = createTestTask({
        internalStatus: "failed",
        metadata: "invalid json",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("failure-reason-section")).not.toBeInTheDocument();
    });
  });


  describe("restart action for terminal states", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      mockUseTaskSteps.mockReturnValue({
        data: [],
        isLoading: false,
        isError: false,
      } as ReturnType<typeof useTaskSteps>);
      mockConfirmation.confirm = vi.fn(async () => true);
    });

    it("renders restart button for failed state", () => {
      const task = createTestTask({ internalStatus: "failed" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("restart-button")).toBeInTheDocument();
      expect(screen.getByText("Restart")).toBeInTheDocument();
    });

    it("renders restart button for stopped state", () => {
      const task = createTestTask({ internalStatus: "stopped" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("restart-button")).toBeInTheDocument();
      expect(screen.getByText("Restart")).toBeInTheDocument();
    });

    it("renders restart button for cancelled state", () => {
      const task = createTestTask({ internalStatus: "cancelled" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("restart-button")).toBeInTheDocument();
      expect(screen.getByText("Restart")).toBeInTheDocument();
    });

    it("renders resume button for paused state", () => {
      const task = createTestTask({ internalStatus: "paused" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("restart-button")).toBeInTheDocument();
      expect(screen.getByText("Resume")).toBeInTheDocument();
    });

    it("does not render restart button for backlog state", () => {
      const task = createTestTask({ internalStatus: "backlog" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("restart-button")).not.toBeInTheDocument();
    });

    it("does not render restart button for ready state", () => {
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("restart-button")).not.toBeInTheDocument();
    });

    it("does not render restart button for blocked state", () => {
      const task = createTestTask({ internalStatus: "blocked" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("restart-button")).not.toBeInTheDocument();
    });

    it("does not render restart button when isHistorical is true", () => {
      const task = createTestTask({ internalStatus: "failed" });
      render(<BasicTaskDetail task={task} isHistorical />, {
        wrapper: TestWrapper,
      });

      expect(screen.queryByTestId("restart-button")).not.toBeInTheDocument();
    });

    it("calls api.tasks.move with correct parameters on button click", async () => {
      const user = userEvent.setup();
      const task = createTestTask({ internalStatus: "failed" });
      mockConfirmation.confirm = vi.fn(async () => true);

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      const button = screen.getByTestId("restart-button");
      await user.click(button);

      await waitFor(() => {
        expect(mockApiTasksMove).toHaveBeenCalledWith(task.id, "ready");
      });
    });
  });
});

