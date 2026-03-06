/**
 * ReviewingTaskDetail component tests
 *
 * Covers: review progress steps rendering, action buttons (stop/escalate),
 * historical mode hiding actions, confirmation dialogs, and loading states.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReviewingTaskDetail } from "./ReviewingTaskDetail";
import type { Task } from "@/types/task";

vi.mock("@/hooks/useReviews", () => ({
  useTaskStateHistory: vi.fn(() => ({ data: [] })),
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
      stop: vi.fn(async () => ({})),
      move: vi.fn(async () => ({})),
    },
  },
}));

// Mock EventBus for useValidationEvents hook
const mockListeners = new Map<string, Set<(payload: unknown) => void>>();
const stableBus = {
  subscribe: (eventName: string, callback: (payload: unknown) => void) => {
    if (!mockListeners.has(eventName)) {
      mockListeners.set(eventName, new Set());
    }
    mockListeners.get(eventName)!.add(callback);
    return () => {
      mockListeners.get(eventName)?.delete(callback);
    };
  },
};

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => stableBus,
}));

import { api } from "@/lib/tauri";

const mockApiTasksStop = vi.mocked(api.tasks.stop);
const mockApiTasksMove = vi.mocked(api.tasks.move);

function createTestTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task",
    description: "Test description",
    priority: 2,
    internalStatus: "reviewing",
    needsReviewPoint: false,
    createdAt: "2026-01-28T12:00:00+00:00",
    updatedAt: "2026-01-28T12:00:00+00:00",
    startedAt: "2026-01-28T12:00:00+00:00",
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

function renderWithProviders(ui: React.ReactElement) {
  return render(ui, { wrapper: TestWrapper });
}

describe("ReviewingTaskDetail", () => {
  beforeEach(() => {
    mockListeners.clear();
    mockConfirmation.confirm = vi.fn(async () => true);
    mockApiTasksStop.mockReset();
    mockApiTasksMove.mockReset();
    mockApiTasksStop.mockResolvedValue({} as never);
    mockApiTasksMove.mockResolvedValue({} as never);
  });

  describe("review progress steps", () => {
    it("renders reviewing-task-detail test id", () => {
      const task = createTestTask();
      renderWithProviders(<ReviewingTaskDetail task={task} />);
      expect(screen.getByTestId("reviewing-task-detail")).toBeInTheDocument();
    });

    it("renders review progress steps", () => {
      const task = createTestTask();
      renderWithProviders(<ReviewingTaskDetail task={task} />);

      expect(screen.getByText("Gathering context")).toBeInTheDocument();
      expect(screen.getByText("Examining changes")).toBeInTheDocument();
      expect(screen.getByText("Running checks")).toBeInTheDocument();
      expect(screen.getByText("Generating feedback")).toBeInTheDocument();
    });

    it("shows 'AI Review in Progress' title for active review", () => {
      const task = createTestTask();
      renderWithProviders(<ReviewingTaskDetail task={task} />);
      expect(screen.getByText("AI Review in Progress")).toBeInTheDocument();
    });

    it("shows reviewing steps section", () => {
      const task = createTestTask();
      renderWithProviders(<ReviewingTaskDetail task={task} />);
      expect(screen.getByTestId("reviewing-steps-section")).toBeInTheDocument();
      expect(screen.getByText("Review Progress")).toBeInTheDocument();
    });
  });

  describe("action buttons", () => {
    it("shows Stop Review button when not historical", () => {
      const task = createTestTask();
      renderWithProviders(<ReviewingTaskDetail task={task} />);
      expect(screen.getByTestId("stop-review-action")).toBeInTheDocument();
      expect(screen.getByText("Stop Review")).toBeInTheDocument();
    });

    it("shows Escalate button when not historical", () => {
      const task = createTestTask();
      renderWithProviders(<ReviewingTaskDetail task={task} />);
      expect(screen.getByTestId("escalate-review-action")).toBeInTheDocument();
      expect(screen.getByText("Escalate")).toBeInTheDocument();
    });

    it("hides action buttons when isHistorical is true", () => {
      const task = createTestTask();
      renderWithProviders(<ReviewingTaskDetail task={task} isHistorical />);
      expect(screen.queryByTestId("reviewing-actions-section")).not.toBeInTheDocument();
      expect(screen.queryByTestId("stop-review-action")).not.toBeInTheDocument();
      expect(screen.queryByTestId("escalate-review-action")).not.toBeInTheDocument();
    });

    it("Stop Review button shows confirmation dialog on click", async () => {
      const user = userEvent.setup();
      const task = createTestTask();
      mockConfirmation.confirm = vi.fn(async () => false);
      renderWithProviders(<ReviewingTaskDetail task={task} />);

      await user.click(screen.getByTestId("stop-review-action"));

      expect(mockConfirmation.confirm).toHaveBeenCalledWith(
        expect.objectContaining({
          title: "Stop review?",
          variant: "destructive",
        })
      );
    });

    it("Escalate button shows confirmation dialog on click", async () => {
      const user = userEvent.setup();
      const task = createTestTask();
      mockConfirmation.confirm = vi.fn(async () => false);
      renderWithProviders(<ReviewingTaskDetail task={task} />);

      await user.click(screen.getByTestId("escalate-review-action"));

      expect(mockConfirmation.confirm).toHaveBeenCalledWith(
        expect.objectContaining({
          title: "Escalate to human review?",
          variant: "destructive",
        })
      );
    });

    it("calls api.tasks.stop when Stop Review is confirmed", async () => {
      const user = userEvent.setup();
      const task = createTestTask();
      mockConfirmation.confirm = vi.fn(async () => true);
      renderWithProviders(<ReviewingTaskDetail task={task} />);

      await user.click(screen.getByTestId("stop-review-action"));

      expect(mockApiTasksStop).toHaveBeenCalledWith("task-123");
    });

    it("calls api.tasks.move when Escalate is confirmed", async () => {
      const user = userEvent.setup();
      const task = createTestTask();
      mockConfirmation.confirm = vi.fn(async () => true);
      renderWithProviders(<ReviewingTaskDetail task={task} />);

      await user.click(screen.getByTestId("escalate-review-action"));

      expect(mockApiTasksMove).toHaveBeenCalledWith("task-123", "escalated");
    });

    it("does not call api when confirmation is cancelled", async () => {
      const user = userEvent.setup();
      const task = createTestTask();
      mockConfirmation.confirm = vi.fn(async () => false);
      renderWithProviders(<ReviewingTaskDetail task={task} />);

      await user.click(screen.getByTestId("stop-review-action"));

      expect(mockApiTasksStop).not.toHaveBeenCalled();
    });
  });
});
