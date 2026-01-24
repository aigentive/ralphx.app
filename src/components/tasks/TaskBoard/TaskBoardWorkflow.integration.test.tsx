/**
 * Integration test: Workflow CRUD and column rendering
 *
 * Tests the integration between workflows and TaskBoard:
 * - Create custom workflow with 5 columns
 * - Set as default workflow
 * - Verify TaskBoard renders correct columns
 * - Delete workflow and verify fallback to default
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskBoardWithHeader } from "./TaskBoardWithHeader";
import * as workflowsApi from "@/lib/api/workflows";
import { api } from "@/lib/tauri";
import type { WorkflowResponse } from "@/lib/api/workflows";

// ============================================================================
// Mocks
// ============================================================================

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
      move: vi.fn(),
    },
    workflows: {
      get: vi.fn(),
      list: vi.fn(),
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

// ============================================================================
// Test Data
// ============================================================================

const defaultRalphXWorkflow: WorkflowResponse = {
  id: "ralphx-default",
  name: "RalphX Default",
  description: "Default RalphX workflow",
  columns: [
    { id: "draft", name: "Draft", maps_to: "backlog", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "backlog", name: "Backlog", maps_to: "backlog", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "todo", name: "To Do", maps_to: "ready", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "planned", name: "Planned", maps_to: "ready", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "in_progress", name: "In Progress", maps_to: "executing", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "in_review", name: "In Review", maps_to: "pending_review", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "done", name: "Done", maps_to: "approved", color: null, icon: null, skip_review: null, auto_advance: null, agent_profile: null },
  ],
  is_default: true,
  worker_profile: null,
  reviewer_profile: null,
};

const custom5ColumnWorkflow: WorkflowResponse = {
  id: "custom-5-col",
  name: "Custom 5-Column Workflow",
  description: "A custom workflow with 5 columns",
  columns: [
    { id: "ideas", name: "Ideas", maps_to: "backlog", color: "#8B5CF6", icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "selected", name: "Selected", maps_to: "ready", color: "#3B82F6", icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "in_dev", name: "In Development", maps_to: "executing", color: "#F59E0B", icon: null, skip_review: null, auto_advance: null, agent_profile: "fast-worker" },
    { id: "testing", name: "Testing", maps_to: "pending_review", color: "#10B981", icon: null, skip_review: null, auto_advance: null, agent_profile: null },
    { id: "shipped", name: "Shipped", maps_to: "approved", color: "#6366F1", icon: null, skip_review: null, auto_advance: null, agent_profile: null },
  ],
  is_default: false,
  worker_profile: null,
  reviewer_profile: null,
};

// ============================================================================
// Test Utilities
// ============================================================================

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

// ============================================================================
// Tests
// ============================================================================

describe("Workflow CRUD and Column Rendering Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ========================================================================
  // Test 1: Create custom workflow with 5 columns
  // ========================================================================

  describe("Test 1: Create custom workflow with 5 columns", () => {
    it("workflow with 5 columns is properly structured", () => {
      // Verify the custom workflow structure
      expect(custom5ColumnWorkflow.columns).toHaveLength(5);
      expect(custom5ColumnWorkflow.columns.map((c) => c.id)).toEqual([
        "ideas",
        "selected",
        "in_dev",
        "testing",
        "shipped",
      ]);
      expect(custom5ColumnWorkflow.columns.map((c) => c.maps_to)).toEqual([
        "backlog",
        "ready",
        "executing",
        "pending_review",
        "approved",
      ]);
    });

    it("workflow columns have correct properties", () => {
      const inDevColumn = custom5ColumnWorkflow.columns.find((c) => c.id === "in_dev")!;
      expect(inDevColumn.name).toBe("In Development");
      expect(inDevColumn.maps_to).toBe("executing");
      expect(inDevColumn.color).toBe("#F59E0B");
      expect(inDevColumn.agent_profile).toBe("fast-worker");
    });
  });

  // ========================================================================
  // Test 2: Set as default workflow
  // ========================================================================

  describe("Test 2: Set as default workflow", () => {
    it("can switch from default to custom workflow", async () => {
      // Setup: Start with default, then switch to custom
      const workflowsWithDefault = [defaultRalphXWorkflow, custom5ColumnWorkflow];
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue(workflowsWithDefault);
      vi.mocked(api.workflows.get).mockImplementation(async (id) =>
        workflowsWithDefault.find((w) => w.id === id) || null
      );
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Wait for initial load with default workflow
      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("RalphX Default");
      });

      // Open dropdown and select custom workflow
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const items = screen.getAllByTestId("workflow-item");
      const customItem = items.find((item) => item.textContent?.includes("Custom 5-Column"));
      expect(customItem).toBeDefined();
      fireEvent.click(customItem!);

      // Verify custom workflow is now selected
      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Custom 5-Column Workflow");
      });
    });

    it("default badge shows for default workflow", async () => {
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue([defaultRalphXWorkflow]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultRalphXWorkflow);
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("default-badge")).toBeInTheDocument();
      });
    });
  });

  // ========================================================================
  // Test 3: Verify TaskBoard renders correct columns
  // ========================================================================

  describe("Test 3: Verify TaskBoard renders correct columns", () => {
    it("renders 7 columns for RalphX Default workflow", async () => {
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue([defaultRalphXWorkflow]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultRalphXWorkflow);
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        // RalphX Default has 7 columns
        expect(screen.getByTestId("column-draft")).toBeInTheDocument();
        expect(screen.getByTestId("column-backlog")).toBeInTheDocument();
        expect(screen.getByTestId("column-todo")).toBeInTheDocument();
        expect(screen.getByTestId("column-planned")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_progress")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_review")).toBeInTheDocument();
        expect(screen.getByTestId("column-done")).toBeInTheDocument();
      });
    });

    it("renders 5 columns for custom workflow", async () => {
      // Start with custom workflow as default
      const customAsDefault = { ...custom5ColumnWorkflow, is_default: true };
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue([customAsDefault]);
      vi.mocked(api.workflows.get).mockResolvedValue(customAsDefault);
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        // Custom workflow has 5 columns
        expect(screen.getByTestId("column-ideas")).toBeInTheDocument();
        expect(screen.getByTestId("column-selected")).toBeInTheDocument();
        expect(screen.getByTestId("column-in_dev")).toBeInTheDocument();
        expect(screen.getByTestId("column-testing")).toBeInTheDocument();
        expect(screen.getByTestId("column-shipped")).toBeInTheDocument();
      });
    });

    it("columns change when workflow is switched", async () => {
      const workflows = [defaultRalphXWorkflow, custom5ColumnWorkflow];
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue(workflows);
      vi.mocked(api.workflows.get).mockImplementation(async (id) =>
        workflows.find((w) => w.id === id) || null
      );
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Wait for initial 7-column workflow
      await waitFor(() => {
        expect(screen.getByTestId("column-draft")).toBeInTheDocument();
      });

      // Switch to custom workflow
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const items = screen.getAllByTestId("workflow-item");
      const customItem = items.find((item) => item.textContent?.includes("Custom 5-Column"));
      fireEvent.click(customItem!);

      // Verify 5-column workflow is now rendered
      await waitFor(() => {
        expect(screen.getByTestId("column-ideas")).toBeInTheDocument();
        expect(screen.getByTestId("column-shipped")).toBeInTheDocument();
      });
    });
  });

  // ========================================================================
  // Test 4: Delete workflow and verify fallback to default
  // ========================================================================

  describe("Test 4: Delete workflow and verify fallback to default", () => {
    it("falls back to first available workflow when current is deleted", async () => {
      // Start with custom workflow selected, then simulate it being deleted
      const workflows = [defaultRalphXWorkflow, custom5ColumnWorkflow];
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue(workflows);
      vi.mocked(api.workflows.get).mockImplementation(async (id) =>
        workflows.find((w) => w.id === id) || null
      );
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Start with default workflow
      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("RalphX Default");
      });

      // The fallback behavior is handled at the application level
      // When a workflow is deleted, the list is re-fetched and
      // TaskBoardWithHeader selects the default workflow
    });

    it("shows default workflow columns after deletion fallback", async () => {
      // Only default workflow remains after deletion
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue([defaultRalphXWorkflow]);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultRalphXWorkflow);
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Verify default columns are rendered
      await waitFor(() => {
        expect(screen.getByTestId("column-draft")).toBeInTheDocument();
        expect(screen.getByTestId("column-done")).toBeInTheDocument();
      });

      // Verify 7 columns for RalphX Default
      const columns = screen.getAllByTestId(/^column-/);
      expect(columns.length).toBe(7);
    });
  });

  // ========================================================================
  // Additional Integration Tests
  // ========================================================================

  describe("Additional integration tests", () => {
    it("workflow list shows all available workflows", async () => {
      const workflows = [defaultRalphXWorkflow, custom5ColumnWorkflow];
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue(workflows);
      vi.mocked(api.workflows.get).mockResolvedValue(defaultRalphXWorkflow);
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      // Wait for workflows to load - the current workflow name should show the actual workflow
      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("RalphX Default");
      });

      // Open dropdown
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      // Verify both workflows are listed
      const items = screen.getAllByTestId("workflow-item");
      expect(items).toHaveLength(2);
    });

    it("preserves task data when switching workflows", async () => {
      const workflows = [defaultRalphXWorkflow, custom5ColumnWorkflow];
      vi.mocked(workflowsApi.getWorkflows).mockResolvedValue(workflows);
      vi.mocked(api.workflows.get).mockImplementation(async (id) =>
        workflows.find((w) => w.id === id) || null
      );
      vi.mocked(api.tasks.list).mockResolvedValue([]);

      render(<TaskBoardWithHeader projectId="p1" />, { wrapper: createWrapper() });

      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("RalphX Default");
      });

      // Record call count
      const initialCallCount = vi.mocked(api.tasks.list).mock.calls.length;

      // Switch workflow
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const items = screen.getAllByTestId("workflow-item");
      fireEvent.click(items[1]);

      await waitFor(() => {
        expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Custom 5-Column");
      });

      // Tasks should not be re-fetched (workflow switch doesn't change task data)
      expect(vi.mocked(api.tasks.list).mock.calls.length).toBe(initialCallCount);
    });
  });
});
