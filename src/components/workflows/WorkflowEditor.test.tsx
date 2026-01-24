/**
 * WorkflowEditor component tests
 *
 * Tests for:
 * - Rendering workflow form fields
 * - Column list display
 * - Add/remove columns
 * - Column configuration (name, mapsTo status)
 * - Form submission
 * - Loading and disabled states
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WorkflowEditor } from "./WorkflowEditor";
import type { WorkflowSchema, WorkflowColumn } from "@/types/workflow";

// ============================================================================
// Test Data
// ============================================================================

const createMockColumn = (overrides: Partial<WorkflowColumn> = {}): WorkflowColumn => ({
  id: "col-1",
  name: "Backlog",
  mapsTo: "backlog",
  ...overrides,
});

const createMockWorkflow = (overrides: Partial<WorkflowSchema> = {}): WorkflowSchema => ({
  id: "workflow-1",
  name: "Test Workflow",
  description: "A test workflow",
  columns: [
    createMockColumn({ id: "col-1", name: "Backlog", mapsTo: "backlog" }),
    createMockColumn({ id: "col-2", name: "In Progress", mapsTo: "executing" }),
    createMockColumn({ id: "col-3", name: "Done", mapsTo: "approved" }),
  ],
  isDefault: false,
  ...overrides,
});

describe("WorkflowEditor", () => {
  const defaultProps = {
    workflow: createMockWorkflow(),
    onSave: vi.fn(),
    onCancel: vi.fn(),
    isSaving: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<WorkflowEditor {...defaultProps} />);
      expect(screen.getByTestId("workflow-editor")).toBeInTheDocument();
    });

    it("displays workflow name in input", () => {
      render(<WorkflowEditor {...defaultProps} />);
      expect(screen.getByTestId("workflow-name-input")).toHaveValue("Test Workflow");
    });

    it("displays workflow description in input", () => {
      render(<WorkflowEditor {...defaultProps} />);
      expect(screen.getByTestId("workflow-description-input")).toHaveValue("A test workflow");
    });

    it("renders all columns", () => {
      render(<WorkflowEditor {...defaultProps} />);
      const columnItems = screen.getAllByTestId("column-item");
      expect(columnItems).toHaveLength(3);
    });

    it("shows column names in inputs", () => {
      render(<WorkflowEditor {...defaultProps} />);
      const nameInputs = screen.getAllByTestId("column-name-input");
      expect(nameInputs[0]).toHaveValue("Backlog");
      expect(nameInputs[1]).toHaveValue("In Progress");
      expect(nameInputs[2]).toHaveValue("Done");
    });

    it("renders add column button", () => {
      render(<WorkflowEditor {...defaultProps} />);
      expect(screen.getByTestId("add-column-button")).toBeInTheDocument();
    });

    it("renders save and cancel buttons", () => {
      render(<WorkflowEditor {...defaultProps} />);
      expect(screen.getByTestId("save-button")).toBeInTheDocument();
      expect(screen.getByTestId("cancel-button")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Create Mode
  // ==========================================================================

  describe("create mode", () => {
    it("shows empty fields when no workflow provided", () => {
      render(<WorkflowEditor {...defaultProps} workflow={undefined} />);
      expect(screen.getByTestId("workflow-name-input")).toHaveValue("");
    });

    it("shows one default column when no workflow provided", () => {
      render(<WorkflowEditor {...defaultProps} workflow={undefined} />);
      const columnItems = screen.getAllByTestId("column-item");
      expect(columnItems).toHaveLength(1);
    });
  });

  // ==========================================================================
  // Column Management
  // ==========================================================================

  describe("column management", () => {
    it("adds new column when add button is clicked", async () => {
      const user = userEvent.setup();
      render(<WorkflowEditor {...defaultProps} />);

      await user.click(screen.getByTestId("add-column-button"));

      const columnItems = screen.getAllByTestId("column-item");
      expect(columnItems).toHaveLength(4);
    });

    it("removes column when remove button is clicked", async () => {
      const user = userEvent.setup();
      render(<WorkflowEditor {...defaultProps} />);

      const columnItems = screen.getAllByTestId("column-item");
      const removeButton = within(columnItems[1]).getByTestId("remove-column-button");
      await user.click(removeButton);

      expect(screen.getAllByTestId("column-item")).toHaveLength(2);
    });

    it("does not show remove button when only one column", () => {
      render(<WorkflowEditor {...defaultProps} workflow={undefined} />);

      const columnItems = screen.getAllByTestId("column-item");
      expect(within(columnItems[0]).queryByTestId("remove-column-button")).not.toBeInTheDocument();
    });

    it("updates column name when input changes", async () => {
      const user = userEvent.setup();
      render(<WorkflowEditor {...defaultProps} />);

      const nameInputs = screen.getAllByTestId("column-name-input");
      await user.clear(nameInputs[0]);
      await user.type(nameInputs[0], "New Name");

      expect(nameInputs[0]).toHaveValue("New Name");
    });

    it("updates column mapsTo when status changes", async () => {
      const user = userEvent.setup();
      render(<WorkflowEditor {...defaultProps} />);

      const statusSelects = screen.getAllByTestId("column-status-select");
      await user.selectOptions(statusSelects[0], "ready");

      expect(statusSelects[0]).toHaveValue("ready");
    });
  });

  // ==========================================================================
  // Form Submission
  // ==========================================================================

  describe("form submission", () => {
    it("calls onSave with updated workflow when save is clicked", async () => {
      const user = userEvent.setup();
      render(<WorkflowEditor {...defaultProps} />);

      await user.click(screen.getByTestId("save-button"));

      expect(defaultProps.onSave).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "Test Workflow",
          columns: expect.arrayContaining([
            expect.objectContaining({ name: "Backlog" }),
          ]),
        })
      );
    });

    it("calls onCancel when cancel is clicked", async () => {
      const user = userEvent.setup();
      render(<WorkflowEditor {...defaultProps} />);

      await user.click(screen.getByTestId("cancel-button"));

      expect(defaultProps.onCancel).toHaveBeenCalled();
    });

    it("generates new IDs for new columns", async () => {
      const user = userEvent.setup();
      render(<WorkflowEditor {...defaultProps} workflow={undefined} />);

      const nameInput = screen.getByTestId("workflow-name-input");
      await user.type(nameInput, "New Workflow");
      await user.click(screen.getByTestId("save-button"));

      expect(defaultProps.onSave).toHaveBeenCalledWith(
        expect.objectContaining({
          columns: expect.arrayContaining([
            expect.objectContaining({ id: expect.stringMatching(/^col-/) }),
          ]),
        })
      );
    });
  });

  // ==========================================================================
  // Loading State
  // ==========================================================================

  describe("loading state", () => {
    it("disables save button when saving", () => {
      render(<WorkflowEditor {...defaultProps} isSaving />);
      expect(screen.getByTestId("save-button")).toBeDisabled();
    });

    it("disables cancel button when saving", () => {
      render(<WorkflowEditor {...defaultProps} isSaving />);
      expect(screen.getByTestId("cancel-button")).toBeDisabled();
    });

    it("shows saving text on save button", () => {
      render(<WorkflowEditor {...defaultProps} isSaving />);
      expect(screen.getByTestId("save-button")).toHaveTextContent("Saving...");
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("labels workflow name input", () => {
      render(<WorkflowEditor {...defaultProps} />);
      expect(screen.getByLabelText(/workflow name/i)).toBeInTheDocument();
    });

    it("labels column name inputs", () => {
      render(<WorkflowEditor {...defaultProps} />);
      const nameInputs = screen.getAllByTestId("column-name-input");
      nameInputs.forEach((input) => {
        expect(input).toHaveAccessibleName();
      });
    });

    it("labels status selects", () => {
      render(<WorkflowEditor {...defaultProps} />);
      const statusSelects = screen.getAllByTestId("column-status-select");
      statusSelects.forEach((select) => {
        expect(select).toHaveAccessibleName();
      });
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<WorkflowEditor {...defaultProps} />);
      const editor = screen.getByTestId("workflow-editor");
      expect(editor).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses accent color for save button", () => {
      render(<WorkflowEditor {...defaultProps} />);
      const saveButton = screen.getByTestId("save-button");
      expect(saveButton).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });
  });

  // ==========================================================================
  // Status Options
  // ==========================================================================

  describe("status options", () => {
    it("shows all internal statuses in select", () => {
      render(<WorkflowEditor {...defaultProps} />);
      const statusSelects = screen.getAllByTestId("column-status-select");
      const options = within(statusSelects[0]).getAllByRole("option");

      // Should have all 14 internal statuses
      expect(options.length).toBeGreaterThanOrEqual(14);
    });
  });
});
