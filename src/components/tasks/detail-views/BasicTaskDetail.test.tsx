/**
 * BasicTaskDetail component tests
 *
 * Tests for the basic task detail view used for backlog, ready, blocked states.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BasicTaskDetail } from "./BasicTaskDetail";
import type { Task } from "@/types/task";

// Mock hooks
vi.mock("@/hooks/useTaskSteps", () => ({
  useTaskSteps: vi.fn(),
}));

import { useTaskSteps } from "@/hooks/useTaskSteps";
const mockUseTaskSteps = vi.mocked(useTaskSteps);

// Helper to create test task
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

// Test wrapper with QueryClient
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
    // Default mock: no steps
    mockUseTaskSteps.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskSteps>);
  });

  describe("rendering", () => {
    it("renders status badge with correct status", () => {
      const task = createTestTask({ internalStatus: "ready" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      const badge = screen.getByTestId("basic-task-status");
      expect(badge).toHaveTextContent("Ready");
    });

    it("renders task title", () => {
      const task = createTestTask({ title: "My Task Title" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("basic-task-title")).toHaveTextContent(
        "My Task Title"
      );
    });

    it("renders priority badge", () => {
      const task = createTestTask({ priority: 1 });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("basic-task-priority")).toHaveTextContent("P1");
    });

    it("renders category", () => {
      const task = createTestTask({ category: "bug" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("basic-task-category")).toHaveTextContent("bug");
    });

    it("renders description when provided", () => {
      const task = createTestTask({ description: "Task description here" });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("basic-task-description")).toHaveTextContent(
        "Task description here"
      );
    });

    it("renders placeholder when description is null", () => {
      const task = createTestTask({ description: null });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByText(/no description/i)).toBeInTheDocument();
    });
  });

  describe("steps section", () => {
    it("renders StepList when task has steps", () => {
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
    });

    it("does not render steps section when no steps", () => {
      const task = createTestTask();
      mockUseTaskSteps.mockReturnValue({
        data: [],
        isLoading: false,
        isError: false,
      } as ReturnType<typeof useTaskSteps>);

      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(
        screen.queryByTestId("basic-task-steps-section")
      ).not.toBeInTheDocument();
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
  });

  describe("different statuses", () => {
    it.each([
      ["backlog", "Backlog"],
      ["ready", "Ready"],
      ["blocked", "Blocked"],
    ])("renders correct badge for %s status", (status, label) => {
      const task = createTestTask({
        internalStatus: status as Task["internalStatus"],
      });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("basic-task-status")).toHaveTextContent(label);
    });
  });

  describe("priority colors", () => {
    it.each([1, 2, 3, 4])("renders priority P%i correctly", (priority) => {
      const task = createTestTask({ priority });
      render(<BasicTaskDetail task={task} />, { wrapper: TestWrapper });

      expect(screen.getByTestId("basic-task-priority")).toHaveTextContent(
        `P${priority}`
      );
    });
  });
});
