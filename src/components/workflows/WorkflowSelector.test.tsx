/**
 * WorkflowSelector component tests
 *
 * Tests for:
 * - Rendering workflow list in dropdown
 * - Current workflow display
 * - Workflow selection handling
 * - Loading state
 * - Empty state
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { WorkflowSelector } from "./WorkflowSelector";
import type { WorkflowSchema } from "@/types/workflow";

// ============================================================================
// Test Data
// ============================================================================

const createMockWorkflow = (overrides: Partial<WorkflowSchema> = {}): WorkflowSchema => ({
  id: "workflow-1",
  name: "Test Workflow",
  description: "A test workflow",
  columns: [
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "todo", name: "To Do", mapsTo: "ready" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
  isDefault: false,
  ...overrides,
});

const mockWorkflows: WorkflowSchema[] = [
  createMockWorkflow({ id: "wf-1", name: "RalphX Default", isDefault: true }),
  createMockWorkflow({ id: "wf-2", name: "Jira Compatible", isDefault: false }),
  createMockWorkflow({ id: "wf-3", name: "Custom Flow", isDefault: false }),
];

describe("WorkflowSelector", () => {
  const defaultProps = {
    workflows: mockWorkflows,
    currentWorkflowId: "wf-1",
    onSelectWorkflow: vi.fn(),
    isLoading: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<WorkflowSelector {...defaultProps} />);
      expect(screen.getByTestId("workflow-selector")).toBeInTheDocument();
    });

    it("displays current workflow name", () => {
      render(<WorkflowSelector {...defaultProps} />);
      expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("RalphX Default");
    });

    it("renders dropdown trigger button", () => {
      render(<WorkflowSelector {...defaultProps} />);
      expect(screen.getByTestId("dropdown-trigger")).toBeInTheDocument();
    });

    it("hides dropdown initially", () => {
      render(<WorkflowSelector {...defaultProps} />);
      expect(screen.queryByTestId("workflow-dropdown")).not.toBeInTheDocument();
    });

    it("shows default badge for default workflow", () => {
      render(<WorkflowSelector {...defaultProps} />);
      expect(screen.getByTestId("default-badge")).toBeInTheDocument();
    });

    it("does not show default badge for non-default workflow", () => {
      render(<WorkflowSelector {...defaultProps} currentWorkflowId="wf-2" />);
      expect(screen.queryByTestId("default-badge")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Dropdown Behavior
  // ==========================================================================

  describe("dropdown behavior", () => {
    it("opens dropdown when trigger is clicked", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getByTestId("workflow-dropdown")).toBeInTheDocument();
    });

    it("closes dropdown when trigger is clicked again", () => {
      render(<WorkflowSelector {...defaultProps} />);
      const trigger = screen.getByTestId("dropdown-trigger");
      fireEvent.click(trigger);
      fireEvent.click(trigger);
      expect(screen.queryByTestId("workflow-dropdown")).not.toBeInTheDocument();
    });

    it("displays all workflows in dropdown", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const dropdown = screen.getByTestId("workflow-dropdown");
      expect(within(dropdown).getByText("RalphX Default")).toBeInTheDocument();
      expect(within(dropdown).getByText("Jira Compatible")).toBeInTheDocument();
      expect(within(dropdown).getByText("Custom Flow")).toBeInTheDocument();
    });

    it("closes dropdown when clicking outside", () => {
      render(
        <div>
          <div data-testid="outside">Outside</div>
          <WorkflowSelector {...defaultProps} />
        </div>
      );
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getByTestId("workflow-dropdown")).toBeInTheDocument();

      fireEvent.mouseDown(screen.getByTestId("outside"));
      expect(screen.queryByTestId("workflow-dropdown")).not.toBeInTheDocument();
    });

    it("closes dropdown when Escape is pressed", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getByTestId("workflow-dropdown")).toBeInTheDocument();

      fireEvent.keyDown(document, { key: "Escape" });
      expect(screen.queryByTestId("workflow-dropdown")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Workflow Selection
  // ==========================================================================

  describe("workflow selection", () => {
    it("calls onSelectWorkflow when workflow is clicked", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const dropdown = screen.getByTestId("workflow-dropdown");
      fireEvent.click(within(dropdown).getByText("Jira Compatible"));

      expect(defaultProps.onSelectWorkflow).toHaveBeenCalledWith("wf-2");
    });

    it("closes dropdown after selection", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const dropdown = screen.getByTestId("workflow-dropdown");
      fireEvent.click(within(dropdown).getByText("Jira Compatible"));

      expect(screen.queryByTestId("workflow-dropdown")).not.toBeInTheDocument();
    });

    it("highlights current workflow in dropdown", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const workflowItems = screen.getAllByTestId("workflow-item");
      expect(workflowItems[0]).toHaveAttribute("data-selected", "true");
      expect(workflowItems[1]).toHaveAttribute("data-selected", "false");
    });
  });

  // ==========================================================================
  // Default Workflow Indicator
  // ==========================================================================

  describe("default workflow indicator", () => {
    it("shows default indicator in dropdown for default workflow", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const workflowItems = screen.getAllByTestId("workflow-item");
      expect(within(workflowItems[0]).getByTestId("workflow-default-indicator")).toBeInTheDocument();
    });

    it("does not show default indicator for non-default workflows", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const workflowItems = screen.getAllByTestId("workflow-item");
      expect(within(workflowItems[1]).queryByTestId("workflow-default-indicator")).not.toBeInTheDocument();
      expect(within(workflowItems[2]).queryByTestId("workflow-default-indicator")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Empty State
  // ==========================================================================

  describe("empty state", () => {
    it("shows message when no workflows exist", () => {
      render(<WorkflowSelector {...defaultProps} workflows={[]} currentWorkflowId={null} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      expect(screen.getByText(/no workflows/i)).toBeInTheDocument();
    });

    it("shows placeholder when no current workflow", () => {
      render(<WorkflowSelector {...defaultProps} currentWorkflowId={null} />);
      expect(screen.getByTestId("current-workflow-name")).toHaveTextContent("Select Workflow");
    });
  });

  // ==========================================================================
  // Loading State
  // ==========================================================================

  describe("loading state", () => {
    it("disables dropdown when loading", () => {
      render(<WorkflowSelector {...defaultProps} isLoading />);
      expect(screen.getByTestId("dropdown-trigger")).toBeDisabled();
    });

    it("shows loading indicator when isLoading", () => {
      render(<WorkflowSelector {...defaultProps} isLoading />);
      expect(screen.getByTestId("loading-indicator")).toBeInTheDocument();
    });

    it("does not open dropdown when loading", () => {
      render(<WorkflowSelector {...defaultProps} isLoading />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.queryByTestId("workflow-dropdown")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("has proper aria attributes on dropdown trigger", () => {
      render(<WorkflowSelector {...defaultProps} />);
      const trigger = screen.getByTestId("dropdown-trigger");
      expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
      expect(trigger).toHaveAttribute("aria-expanded", "false");
    });

    it("updates aria-expanded when dropdown is open", () => {
      render(<WorkflowSelector {...defaultProps} />);
      const trigger = screen.getByTestId("dropdown-trigger");
      fireEvent.click(trigger);
      expect(trigger).toHaveAttribute("aria-expanded", "true");
    });

    it("dropdown has listbox role", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    it("workflow items have option role", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getAllByRole("option")).toHaveLength(3);
    });

    it("current workflow option has aria-selected", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const options = screen.getAllByRole("option");
      expect(options[0]).toHaveAttribute("aria-selected", "true");
      expect(options[1]).toHaveAttribute("aria-selected", "false");
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<WorkflowSelector {...defaultProps} />);
      const selector = screen.getByTestId("workflow-selector");
      expect(selector).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses design tokens for dropdown background", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const dropdown = screen.getByTestId("workflow-dropdown");
      expect(dropdown).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("uses design tokens for text colors", () => {
      render(<WorkflowSelector {...defaultProps} />);
      const name = screen.getByTestId("current-workflow-name");
      expect(name).toHaveStyle({ color: "var(--text-primary)" });
    });

    it("uses accent color for default badge", () => {
      render(<WorkflowSelector {...defaultProps} />);
      const badge = screen.getByTestId("default-badge");
      expect(badge).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });
  });

  // ==========================================================================
  // Column Count Display
  // ==========================================================================

  describe("column count display", () => {
    it("shows column count in dropdown items", () => {
      render(<WorkflowSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const workflowItems = screen.getAllByTestId("workflow-item");
      expect(within(workflowItems[0]).getByTestId("column-count")).toHaveTextContent("3 columns");
    });
  });
});
