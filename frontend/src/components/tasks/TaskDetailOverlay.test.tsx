import React from "react";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskDetailOverlay } from "./TaskDetailOverlay";
import { useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import type { Task } from "@/types/task";

vi.mock("./TaskDetailPanel", () => ({
  TaskDetailPanel: () => <div data-testid="mock-task-detail-panel" />,
}));

vi.mock("./TaskEditForm", () => ({
  TaskEditForm: () => <div data-testid="mock-task-edit-form" />,
}));

vi.mock("./StatusDropdown", () => ({
  StatusDropdown: () => <button data-testid="mock-status-dropdown">Status</button>,
}));

vi.mock("./StateTimelineNav", () => ({
  StateTimelineNav: () => <div data-testid="mock-state-timeline-nav" />,
}));

vi.mock("@/components/tasks/AuditTrailDialog", () => ({
  AuditTrailDialog: () => null,
}));

vi.mock("@/hooks/useTasks", () => ({
  taskKeys: {
    all: ["tasks"],
    detail: (taskId: string) => ["tasks", "detail", taskId],
  },
  useTasks: vi.fn(() => ({ data: [] })),
}));

vi.mock("@/hooks/useTaskMutation", () => ({
  useTaskMutation: vi.fn(() => ({
    updateMutation: { mutate: vi.fn(), isPending: false },
    moveMutation: { mutate: vi.fn(), isPending: false },
    archiveMutation: { mutate: vi.fn() },
    restoreMutation: { mutate: vi.fn() },
    isArchiving: false,
    isRestoring: false,
  })),
}));

vi.mock("@/hooks/useIdeation", () => ({
  useCreateIdeationSession: vi.fn(() => ({
    mutateAsync: vi.fn(),
    isPending: false,
  })),
}));

vi.mock("@/hooks/useConfirmation", () => ({
  useConfirmation: vi.fn(() => ({
    confirm: vi.fn(async () => true),
    confirmationDialogProps: {},
    ConfirmationDialog: () => null,
  })),
}));

function createTestTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task",
    description: "Test description",
    priority: 2,
    internalStatus: "ready",
    needsReviewPoint: false,
    createdAt: "2026-01-28T12:00:00+00:00",
    updatedAt: "2026-01-28T12:00:00+00:00",
    startedAt: null,
    completedAt: null,
    archivedAt: null,
    blockedReason: null,
    taskBranch: "ralphx/ralphx/task-123",
    worktreePath: null,
    mergeCommitSha: null,
    metadata: null,
    ...overrides,
  };
}

function renderOverlay(task: Task) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  useTaskStore.getState().setTasks([task]);
  useUiStore.getState().setSelectedTaskId(task.id);
  return render(
    <QueryClientProvider client={queryClient}>
      <TaskDetailOverlay projectId={task.projectId} />
    </QueryClientProvider>
  );
}

describe("TaskDetailOverlay", () => {
  beforeEach(() => {
    useTaskStore.getState().setTasks([]);
    useUiStore.getState().setSelectedTaskId(null);
    useUiStore.getState().setTaskHistoryState(null);
  });

  it("hides edit controls for managed plan merge tasks waiting on PR", () => {
    renderOverlay(
      createTestTask({
        category: "plan_merge",
        internalStatus: "waiting_on_pr",
        taskBranch: null,
      })
    );

    expect(screen.queryByTestId("task-overlay-edit-button")).not.toBeInTheDocument();
    expect(screen.queryByTestId("mock-status-dropdown")).not.toBeInTheDocument();
    expect(screen.getByTestId("task-overlay-category")).toHaveTextContent("Plan merge");
    expect(screen.queryByText("plan_merge")).not.toBeInTheDocument();
  });

  it("keeps edit controls for regular user-created tasks", () => {
    renderOverlay(createTestTask({ category: "feature", internalStatus: "ready" }));

    expect(screen.getByTestId("task-overlay-edit-button")).toBeInTheDocument();
    expect(screen.getByTestId("mock-status-dropdown")).toBeInTheDocument();
  });

  it("adds accessible names and app tooltips to header icon buttons", async () => {
    const user = userEvent.setup();
    renderOverlay(createTestTask({ category: "feature", internalStatus: "backlog" }));

    const archiveButton = screen.getByRole("button", { name: "Archive task" });
    expect(screen.getByRole("button", { name: "Start Ideation" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Edit task" })).toBeInTheDocument();
    expect(archiveButton).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Audit Trail" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close" })).toBeInTheDocument();

    await user.hover(archiveButton);
    expect((await screen.findAllByText("Archive task")).length).toBeGreaterThan(0);
  });
});
