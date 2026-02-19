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
      restart: vi.fn(async () => ({ type: "Success", task: {} })),
      unblock: vi.fn(async () => ({})),
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
const mockApiTasksUnblock = vi.mocked(api.tasks.unblock);

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
      expect(screen.getByText("Failure Details")).toBeInTheDocument();
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

    it("handles malformed JSON metadata gracefully (shows fallback)", () => {
      const task = createTestTask({
        internalStatus: "failed",
        metadata: "invalid json",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-reason-section")).toBeInTheDocument();
      expect(screen.getByText("Task execution failed. Error details were not recorded during the state transition.")).toBeInTheDocument();
    });

    it("displays generic fallback banner for failed task with null metadata", () => {
      const task = createTestTask({
        internalStatus: "failed",
        metadata: null,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-reason-section")).toBeInTheDocument();
      expect(screen.getByText("Task execution failed. Error details were not recorded during the state transition.")).toBeInTheDocument();
    });

    it("displays blockedReason when failed with null metadata but blockedReason exists", () => {
      const task = createTestTask({
        internalStatus: "failed",
        metadata: null,
        blockedReason: "Dependency task failed to complete",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-reason-section")).toBeInTheDocument();
      expect(screen.getByText("Dependency task failed to complete")).toBeInTheDocument();
    });

    it("displays rich failure info when metadata is valid (existing behavior)", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Build script exited with code 1",
        failure_details: "npm run build failed",
        is_timeout: false,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-reason-section")).toBeInTheDocument();
      expect(screen.getByText("Build script exited with code 1")).toBeInTheDocument();
      expect(screen.getByText("npm run build failed")).toBeInTheDocument();
    });

    it("displays generic fallback for failed task with malformed JSON metadata", () => {
      const task = createTestTask({
        internalStatus: "failed",
        metadata: "{ invalid json }",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-reason-section")).toBeInTheDocument();
      expect(screen.getByText("Task execution failed. Error details were not recorded during the state transition.")).toBeInTheDocument();
    });

    it("displays generic fallback for qa_failed task with null metadata", () => {
      const task = createTestTask({
        internalStatus: "qa_failed",
        metadata: null,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-reason-section")).toBeInTheDocument();
      expect(screen.getByText("Task execution failed. Error details were not recorded during the state transition.")).toBeInTheDocument();
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
        expect(mockApiTasksMove).toHaveBeenCalledWith(task.id, "ready", undefined, undefined);
      });
    });
  });

  describe("restart note textarea", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      mockUseTaskSteps.mockReturnValue({
        data: [],
        isLoading: false,
        isError: false,
      } as ReturnType<typeof useTaskSteps>);
      mockConfirmation.confirm = vi.fn(async () => true);
    });

    it("renders note textarea for failed state", () => {
      const task = createTestTask({ internalStatus: "failed" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("restart-note-textarea")).toBeInTheDocument();
    });

    it("renders note textarea for stopped state", () => {
      const task = createTestTask({ internalStatus: "stopped" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("restart-note-textarea")).toBeInTheDocument();
    });

    it("renders note textarea for cancelled state", () => {
      const task = createTestTask({ internalStatus: "cancelled" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("restart-note-textarea")).toBeInTheDocument();
    });

    it("renders note textarea for paused state", () => {
      const task = createTestTask({ internalStatus: "paused" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("restart-note-textarea")).toBeInTheDocument();
    });

    it("does not render note textarea for ready state", () => {
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("restart-note-textarea")).not.toBeInTheDocument();
    });

    it("passes note to api.tasks.move when restarting failed task", async () => {
      const user = userEvent.setup();
      const task = createTestTask({ internalStatus: "failed" });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      const textarea = screen.getByTestId("restart-note-textarea");
      await user.type(textarea, "Fix the broken import");

      const button = screen.getByTestId("restart-button");
      await user.click(button);

      await waitFor(() => {
        expect(mockApiTasksMove).toHaveBeenCalledWith(
          task.id,
          "ready",
          undefined,
          "Fix the broken import"
        );
      });
    });

    it("passes undefined note when textarea is empty", async () => {
      const user = userEvent.setup();
      const task = createTestTask({ internalStatus: "failed" });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      const button = screen.getByTestId("restart-button");
      await user.click(button);

      await waitFor(() => {
        expect(mockApiTasksMove).toHaveBeenCalledWith(
          task.id,
          "ready",
          undefined,
          undefined
        );
      });
    });

    it("note textarea accepts user input for stopped task", async () => {
      const user = userEvent.setup();
      // stop_metadata is a nested JSON string inside the outer metadata JSON object
      const stopMetadataInner = JSON.stringify({
        stopped_from_status: "executing",
        stopped_at: new Date().toISOString(),
        stop_reason: "User requested",
      });
      const stopMetadata = JSON.stringify({
        stop_metadata: stopMetadataInner,
      });
      const task = createTestTask({
        internalStatus: "stopped",
        metadata: stopMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      const textarea = screen.getByTestId("restart-note-textarea");
      expect(textarea).toBeInTheDocument();
      await user.type(textarea, "Try a different approach");
      expect(textarea).toHaveValue("Try a different approach");
    });
  });

  describe("execution mode selector", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      mockUseTaskSteps.mockReturnValue({
        data: [],
        isLoading: false,
        isError: false,
      } as ReturnType<typeof useTaskSteps>);
      mockConfirmation.confirm = vi.fn(async () => true);
    });

    it("renders ExecutionModeSelector for ready state", () => {
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("execution-mode-selector")).toBeInTheDocument();
      expect(screen.getByTestId("mode-solo")).toBeInTheDocument();
      expect(screen.getByTestId("mode-team")).toBeInTheDocument();
    });

    it("renders ExecutionModeSelector for failed state", () => {
      const task = createTestTask({ internalStatus: "failed" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("execution-mode-selector")).toBeInTheDocument();
    });

    it("defaults to solo mode", () => {
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      const soloBtn = screen.getByTestId("mode-solo");
      // Solo button should have non-transparent background (selected state)
      expect(soloBtn).toHaveStyle({ backgroundColor: "hsla(220 10% 100% / 0.08)" });
    });

    it("switches to team mode with orange styling on click", async () => {
      const user = userEvent.setup();
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      const teamBtn = screen.getByTestId("mode-team");
      await user.click(teamBtn);

      // Team button should have warm orange background when selected
      expect(teamBtn).toHaveStyle({ backgroundColor: "hsla(14 100% 60% / 0.15)" });
    });

    it("passes 'team' agentVariant to API when team mode selected", async () => {
      const user = userEvent.setup();
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      // Select team mode
      await user.click(screen.getByTestId("mode-team"));
      // Click start
      await user.click(screen.getByTestId("start-button"));

      await waitFor(() => {
        expect(mockApiTasksMove).toHaveBeenCalledWith(task.id, "ready", "team", undefined);
      });
    });

    it("confirmation dialog includes mode note for team mode", async () => {
      const user = userEvent.setup();
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("mode-team"));
      await user.click(screen.getByTestId("start-button"));

      await waitFor(() => {
        expect(mockConfirmation.confirm).toHaveBeenCalledWith(
          expect.objectContaining({
            description: expect.stringContaining("in team mode"),
          }),
        );
      });
    });
  });

  describe("attempt count display for failed tasks", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      mockUseTaskSteps.mockReturnValue({
        data: [],
        isLoading: false,
        isError: false,
      } as ReturnType<typeof useTaskSteps>);
    });

    it("shows attempt count when auto_retry_count_executing is present in metadata", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Agent gave up",
        is_timeout: false,
        auto_retry_count_executing: 3,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("attempt-count")).toBeInTheDocument();
      expect(screen.getByText(/Failed after 3 attempts/)).toBeInTheDocument();
    });

    it("shows singular 'attempt' for count of 1", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Agent gave up",
        is_timeout: false,
        auto_retry_count_executing: 1,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByText(/Failed after 1 attempt/)).toBeInTheDocument();
    });

    it("does not show attempt count when auto_retry_count_executing is 0", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Agent gave up",
        is_timeout: false,
        auto_retry_count_executing: 0,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("attempt-count")).not.toBeInTheDocument();
    });

    it("shows timeout badge alongside attempt count for timeout failures", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Agent timed out",
        is_timeout: true,
        auto_retry_count_executing: 5,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("attempt-count")).toBeInTheDocument();
      expect(screen.getByTestId("timeout-badge")).toBeInTheDocument();
      expect(screen.getByText("timeout")).toBeInTheDocument();
    });

    it("shows timeout badge even when attempt count is 0", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Agent timed out",
        is_timeout: true,
        auto_retry_count_executing: 0,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("timeout-badge")).toBeInTheDocument();
      expect(screen.queryByTestId("attempt-count")).not.toBeInTheDocument();
    });

    it("shows failure error message via data-testid", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Build script failed",
        is_timeout: false,
        auto_retry_count_executing: 2,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-error-message")).toBeInTheDocument();
      expect(screen.getByTestId("failure-error-message")).toHaveTextContent("Build script failed");
    });

    it("shows failure details via data-testid when present", () => {
      const failureMetadata = JSON.stringify({
        failure_error: "Build failed",
        failure_details: "npm run build exited with code 1",
        is_timeout: false,
        auto_retry_count_executing: 1,
      });
      const task = createTestTask({
        internalStatus: "failed",
        metadata: failureMetadata,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("failure-details")).toHaveTextContent(
        "npm run build exited with code 1"
      );
    });
  });

  describe("dependency-failed StatusBanner for blocked tasks", () => {
    beforeEach(() => {
      vi.clearAllMocks();
      mockUseTaskSteps.mockReturnValue({
        data: [],
        isLoading: false,
        isError: false,
      } as ReturnType<typeof useTaskSteps>);
      mockConfirmation.confirm = vi.fn(async () => true);
    });

    it("shows dependency-failed banner when blockedReason indicates failed dependency", () => {
      const task = createTestTask({
        internalStatus: "blocked",
        blockedReason: "Dependency Setup Infrastructure failed.",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("dependency-failed-banner")).toBeInTheDocument();
      expect(screen.getByText("Dependency Failed")).toBeInTheDocument();
      expect(
        screen.getByText(/"Setup Infrastructure" failed/)
      ).toBeInTheDocument();
    });

    it("does not show dependency-failed banner when blockedReason is a normal waiting reason", () => {
      const task = createTestTask({
        internalStatus: "blocked",
        blockedReason: "Waiting for: Setup Infrastructure",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("dependency-failed-banner")).not.toBeInTheDocument();
    });

    it("does not show dependency-failed banner when task is not blocked", () => {
      const task = createTestTask({
        internalStatus: "ready",
        blockedReason: null,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("dependency-failed-banner")).not.toBeInTheDocument();
    });

    it("does not show dependency-failed banner when blockedReason is null", () => {
      const task = createTestTask({
        internalStatus: "blocked",
        blockedReason: null,
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("dependency-failed-banner")).not.toBeInTheDocument();
    });

    it("shows unblock-warning-card with unblock button for failed dependency", () => {
      const task = createTestTask({
        internalStatus: "blocked",
        blockedReason: "Dependency Build Frontend failed.",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("unblock-warning-card")).toBeInTheDocument();
      expect(screen.getByTestId("unblock-button")).toBeInTheDocument();
      expect(screen.getByText("Unblock Anyway")).toBeInTheDocument();
    });

    it("hides unblock-warning-card in historical mode", () => {
      const task = createTestTask({
        internalStatus: "blocked",
        blockedReason: "Dependency Build Frontend failed.",
      });

      render(<BasicTaskDetail task={task} isHistorical />, { wrapper: TestWrapper });

      expect(screen.queryByTestId("unblock-warning-card")).not.toBeInTheDocument();
    });

    it("shows confirmation dialog with warning text on unblock click", async () => {
      const user = userEvent.setup();
      const task = createTestTask({
        internalStatus: "blocked",
        blockedReason: "Dependency Build Frontend failed.",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("unblock-button"));

      await waitFor(() => {
        expect(mockConfirmation.confirm).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Unblock despite failed dependency?",
            description: expect.stringContaining("Build Frontend"),
          })
        );
      });
    });

    it("calls api.tasks.unblock when confirmed", async () => {
      const user = userEvent.setup();
      mockConfirmation.confirm = vi.fn(async () => true);
      const task = createTestTask({
        internalStatus: "blocked",
        blockedReason: "Dependency Build Frontend failed.",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("unblock-button"));

      await waitFor(() => {
        expect(mockApiTasksUnblock).toHaveBeenCalledWith(task.id);
      });
    });

    it("does not call api.tasks.unblock when confirmation is cancelled", async () => {
      const user = userEvent.setup();
      mockConfirmation.confirm = vi.fn(async () => false);
      const task = createTestTask({
        internalStatus: "blocked",
        blockedReason: "Dependency Build Frontend failed.",
      });

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      await user.click(screen.getByTestId("unblock-button"));

      await waitFor(() => {
        expect(mockConfirmation.confirm).toHaveBeenCalled();
        expect(mockApiTasksUnblock).not.toHaveBeenCalled();
      });
    });
  });
});

