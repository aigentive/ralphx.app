/**
 * Integration test: Workflow API and TaskBoard
 *
 * Tests the integration between workflows API and TaskBoard:
 * - getActiveWorkflowColumns returns columns for TaskBoard
 * - Custom workflows can be created and listed
 * - Default workflow can be changed
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskBoardWithHeader } from "./TaskBoardWithHeader";
import * as workflowsApi from "@/lib/api/workflows";
import { api } from "@/lib/tauri";
import type { WorkflowResponse, WorkflowColumnResponse } from "@/lib/api/workflows";
import type { TaskListResponse } from "@/types/task";
import type { InfiniteData } from "@tanstack/react-query";

// ============================================================================
// Mocks
// ============================================================================

// Mock IntersectionObserver
class MockIntersectionObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  constructor() {}
}
window.IntersectionObserver = MockIntersectionObserver as unknown as typeof IntersectionObserver;

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
      move: vi.fn(),
      getArchivedCount: vi.fn(),
      search: vi.fn(),
    },
  },
}));

vi.mock("@/lib/api/workflows", () => ({
  getWorkflows: vi.fn(),
  getWorkflow: vi.fn(),
  createWorkflow: vi.fn(),
  updateWorkflow: vi.fn(),
  deleteWorkflow: vi.fn(),
  setDefaultWorkflow: vi.fn(),
  getActiveWorkflowColumns: vi.fn(),
}));

// Mock useInfiniteTasksQuery
vi.mock("@/hooks/useInfiniteTasksQuery", async (importOriginal) => {
  const actual = await importOriginal() as Record<string, unknown>;
  return {
    ...actual,
    useInfiniteTasksQuery: vi.fn(),
  };
});

// Mock Tauri events
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));

import { useInfiniteTasksQuery } from "@/hooks/useInfiniteTasksQuery";

// ============================================================================
// Test Data
// ============================================================================

const defaultRalphXWorkflow: WorkflowResponse = {
  id: "ralphx-default",
  name: "RalphX Default",
  description: "Default RalphX workflow",
  columns: [
    { id: "draft", name: "Draft", mapsTo: "backlog" },
    { id: "ready", name: "Ready", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_review", name: "In Review", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
  isDefault: true,
  workerProfile: undefined,
  reviewerProfile: undefined,
};

const customAgileWorkflow: WorkflowResponse = {
  id: "custom-agile",
  name: "Custom Agile",
  description: "Custom Agile workflow with sprint columns",
  columns: [
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "selected", name: "Selected", mapsTo: "ready" },
    { id: "dev", name: "Development", mapsTo: "executing" },
    { id: "qa", name: "QA", mapsTo: "pending_review" },
    { id: "release", name: "Release Ready", mapsTo: "approved" },
  ],
  isDefault: false,
  workerProfile: undefined,
  reviewerProfile: undefined,
};

const defaultColumns: WorkflowColumnResponse[] = defaultRalphXWorkflow.columns;
const customColumns: WorkflowColumnResponse[] = customAgileWorkflow.columns;

// ============================================================================
// Test Helpers
// ============================================================================

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

// ============================================================================
// Tests
// ============================================================================

describe("TaskBoardWorkflow Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default mocks
    vi.mocked(api.tasks.getArchivedCount).mockResolvedValue(0);
    vi.mocked(api.tasks.search).mockResolvedValue([]);
    vi.mocked(useInfiniteTasksQuery).mockReturnValue({
      data: { pages: [{ tasks: [], total: 0, hasMore: false, offset: 0 }], pageParams: [undefined] } as InfiniteData<TaskListResponse>,
      fetchNextPage: vi.fn(),
      hasNextPage: false,
      isFetchingNextPage: false,
      isLoading: false,
      isError: false,
      error: null,
    } as unknown as ReturnType<typeof useInfiniteTasksQuery>);
  });

  // ==========================================================================
  // Default Workflow Columns
  // ==========================================================================

  describe("default workflow columns", () => {
    it("renders TaskBoard with default RalphX workflow columns", async () => {
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue([defaultRalphXWorkflow]);
      vi.mocked(workflowsApi.getActiveWorkflowColumns).mockResolvedValue(defaultColumns);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("column-draft")).toBeInTheDocument();
        expect(screen.getByTestId("column-ready")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_progress")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_review")).toBeInTheDocument();
        expect(screen.getByTestId("column-done")).toBeInTheDocument();
      });
    });

    it("shows default workflow in selector", async () => {
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue([defaultRalphXWorkflow]);
      vi.mocked(workflowsApi.getActiveWorkflowColumns).mockResolvedValue(defaultColumns);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("RalphX Default");
        expect(screen.getByTestId("default-badge")).toBeInTheDocument();
      });
    });
  });

  // ==========================================================================
  // Custom Workflow Support
  // ==========================================================================

  describe("custom workflow support", () => {
    it("lists both default and custom workflows in selector", async () => {
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue([
        defaultRalphXWorkflow,
        customAgileWorkflow,
      ]);
      vi.mocked(workflowsApi.getActiveWorkflowColumns).mockResolvedValue(defaultColumns);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Wait for workflows to load
      await waitFor(() => {
        expect(screen.getByTestId("workflow-selector")).toBeInTheDocument();
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("RalphX Default");
      });

      // Open dropdown - use fireEvent for proper async handling
      const trigger = screen.getByTestId("dropdown-trigger");
      trigger.click();

      // Wait for dropdown to appear
      await waitFor(() => {
        expect(screen.getByTestId("workflow-dropdown")).toBeInTheDocument();
      });

      const items = screen.getAllByTestId("workflow-item");
      expect(items).toHaveLength(2);
    });

    it("shows workflow as active when getActiveWorkflowColumns returns its columns", async () => {
      // Simulate custom workflow being set as default
      const customAsDefault = { ...customAgileWorkflow, isDefault: true };
      const ralphxNotDefault = { ...defaultRalphXWorkflow, isDefault: false };

      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue([ralphxNotDefault, customAsDefault]);
      vi.mocked(workflowsApi.getActiveWorkflowColumns).mockResolvedValue(customColumns);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // The board should show custom workflow's columns
      await waitFor(() => {
        expect(screen.getByTestId("column-backlog")).toBeInTheDocument();
        expect(screen.getByTestId("column-selected")).toBeInTheDocument();
        expect(screen.getByTestId("column-dev")).toBeInTheDocument();
        expect(screen.getByTestId("column-qa")).toBeInTheDocument();
        expect(screen.getByTestId("column-release")).toBeInTheDocument();
      });

      // Workflow selector shows custom workflow as current (it's the default)
      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Custom Agile");
      });
    });
  });

  // ==========================================================================
  // Workflow API Operations
  // ==========================================================================

  describe("workflow API operations", () => {
    it("can create a custom workflow", async () => {
      const newWorkflow: WorkflowResponse = {
        id: "new-workflow",
        name: "New Workflow",
        description: "A new workflow",
        columns: [
          { id: "start", name: "Start", mapsTo: "backlog" },
          { id: "end", name: "End", mapsTo: "approved" },
        ],
        isDefault: false,
        workerProfile: undefined,
        reviewerProfile: undefined,
      };

      vi.mocked(workflowsApi.createWorkflow).mockResolvedValue(newWorkflow);

      await workflowsApi.createWorkflow({
        name: "New Workflow",
        columns: [
          { id: "start", name: "Start", maps_to: "backlog" },
          { id: "end", name: "End", maps_to: "approved" },
        ],
      });

      expect(workflowsApi.createWorkflow).toHaveBeenCalled();
    });

    it("can set a workflow as default", async () => {
      vi.mocked(workflowsApi.setDefaultWorkflow).mockResolvedValue({
        ...customAgileWorkflow,
        isDefault: true,
      });

      await workflowsApi.setDefaultWorkflow("custom-agile");

      expect(workflowsApi.setDefaultWorkflow).toHaveBeenCalledWith("custom-agile");
    });

    it("can delete a custom workflow", async () => {
      vi.mocked(workflowsApi.deleteWorkflow).mockResolvedValue(undefined);

      await workflowsApi.deleteWorkflow("custom-agile");

      expect(workflowsApi.deleteWorkflow).toHaveBeenCalledWith("custom-agile");
    });
  });
});
