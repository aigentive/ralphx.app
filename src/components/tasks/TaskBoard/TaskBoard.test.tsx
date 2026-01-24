/**
 * Tests for TaskBoard component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import { defaultWorkflow } from "@/types/workflow";
import { createMockTask } from "@/test/mock-data";
import { TaskBoard } from "./TaskBoard";

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
      move: vi.fn(),
    },
    workflows: {
      get: vi.fn(),
    },
  },
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("TaskBoard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("loading state", () => {
    it("should show skeleton while loading", async () => {
      vi.mocked(api.tasks.list).mockImplementation(() => new Promise(() => {}));
      vi.mocked(api.workflows.get).mockImplementation(() => new Promise(() => {}));

      render(<TaskBoard projectId="p1" workflowId="w1" />, { wrapper: createWrapper() });
      expect(screen.getByTestId("task-board-skeleton")).toBeInTheDocument();
    });

    it("should hide skeleton when data is loaded", async () => {
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      render(<TaskBoard projectId="p1" workflowId="w1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.queryByTestId("task-board-skeleton")).not.toBeInTheDocument();
      });
    });
  });

  describe("rendering columns", () => {
    it("should render with data-testid", async () => {
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      render(<TaskBoard projectId="p1" workflowId="w1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("task-board")).toBeInTheDocument();
      });
    });

    it("should render 7 columns from default workflow", async () => {
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      render(<TaskBoard projectId="p1" workflowId="w1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("column-draft")).toBeInTheDocument();
        expect(screen.getByTestId("column-backlog")).toBeInTheDocument();
        expect(screen.getByTestId("column-todo")).toBeInTheDocument();
        expect(screen.getByTestId("column-planned")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_progress")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_review")).toBeInTheDocument();
        expect(screen.getByTestId("column-done")).toBeInTheDocument();
      });
    });

    it("should render tasks in their columns", async () => {
      const tasks = [
        createMockTask({ id: "t1", title: "Backlog Task", internalStatus: "backlog" }),
        createMockTask({ id: "t2", title: "Ready Task", internalStatus: "ready" }),
      ];
      vi.mocked(api.tasks.list).mockResolvedValue(tasks);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      render(<TaskBoard projectId="p1" workflowId="w1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        // Tasks appear in columns that match their status
        // Note: backlog status maps to both draft and backlog columns
        expect(screen.getAllByText("Backlog Task").length).toBeGreaterThan(0);
        expect(screen.getAllByText("Ready Task").length).toBeGreaterThan(0);
      });
    });
  });

  describe("horizontal scrolling", () => {
    it("should have horizontal scroll container", async () => {
      vi.mocked(api.tasks.list).mockResolvedValue([]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      render(<TaskBoard projectId="p1" workflowId="w1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        const board = screen.getByTestId("task-board");
        expect(board).toHaveClass("overflow-x-auto");
      });
    });
  });

  describe("error handling", () => {
    it("should show error message when fetch fails", async () => {
      vi.mocked(api.tasks.list).mockRejectedValue(new Error("Failed to fetch tasks"));
      vi.mocked(api.workflows.get).mockResolvedValue(defaultWorkflow);

      render(<TaskBoard projectId="p1" workflowId="w1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByText(/Failed to fetch tasks/)).toBeInTheDocument();
      });
    });
  });
});
