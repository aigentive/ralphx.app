import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  TaskRerunDialog,
  type TaskRerunDialogProps,
  type RerunOption,
  type CommitInfo,
} from "./TaskRerunDialog";
import type { Task } from "@/types/task";

// ============================================================================
// Test Fixtures
// ============================================================================

const mockTask: Task = {
  id: "test-task-id",
  projectId: "test-project-id",
  category: "feature",
  title: "Add user authentication",
  description: "Implement login and registration",
  priority: 1,
  internalStatus: "approved",
  needsReviewPoint: false,
  createdAt: "2026-01-25T00:00:00Z",
  updatedAt: "2026-01-25T00:00:00Z",
  startedAt: "2026-01-25T01:00:00Z",
  completedAt: "2026-01-25T02:00:00Z",
};

const mockCommitInfo: CommitInfo = {
  sha: "a1b2c3d",
  message: "feat: Add user authentication",
  hasDependentCommits: false,
};

const mockCommitInfoWithDependents: CommitInfo = {
  sha: "a1b2c3d",
  message: "feat: Add user authentication",
  hasDependentCommits: true,
};

function createDefaultProps(
  overrides: Partial<TaskRerunDialogProps> = {}
): TaskRerunDialogProps {
  return {
    isOpen: true,
    onClose: vi.fn(),
    onConfirm: vi.fn(),
    task: mockTask,
    commitInfo: mockCommitInfo,
    ...overrides,
  };
}

// ============================================================================
// Rendering Tests
// ============================================================================

describe("TaskRerunDialog", () => {
  describe("Rendering", () => {
    it("should not render when isOpen is false", () => {
      render(<TaskRerunDialog {...createDefaultProps({ isOpen: false })} />);
      expect(screen.queryByTestId("task-rerun-dialog")).not.toBeInTheDocument();
    });

    it("should render dialog when isOpen is true", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("task-rerun-dialog")).toBeInTheDocument();
    });

    it("should render overlay and modal", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("dialog-overlay")).toBeInTheDocument();
      expect(screen.getByTestId("dialog-modal")).toBeInTheDocument();
    });

    it("should display 'Re-run Task' in header", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("dialog-title")).toHaveTextContent("Re-run Task");
    });

    it("should display task title", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("task-title")).toHaveTextContent(
        '"Add user authentication"'
      );
    });

    it("should display commit SHA", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("commit-sha")).toHaveTextContent("a1b2c3d");
    });

    it("should display commit message", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("commit-message")).toHaveTextContent(
        '"feat: Add user authentication"'
      );
    });

    it("should display close button", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("dialog-close")).toBeInTheDocument();
    });

    it("should display cancel and confirm buttons", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("cancel-button")).toBeInTheDocument();
      expect(screen.getByTestId("confirm-button")).toBeInTheDocument();
    });

    it("should display 'Confirm Re-run' text on confirm button by default", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("confirm-button")).toHaveTextContent(
        "Confirm Re-run"
      );
    });

    it("should display question about handling previous work", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(
        screen.getByText("How should we handle the previous work?")
      ).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Re-run Options Tests
  // ============================================================================

  describe("Re-run Options", () => {
    const optionIds: RerunOption[] = ["keep_changes", "revert_commit", "create_new"];

    it.each(optionIds)("should render %s option", (option) => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId(`rerun-option-${option}`)).toBeInTheDocument();
    });

    it("should have keep_changes option selected by default", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("rerun-option-keep_changes")).toHaveAttribute(
        "data-selected",
        "true"
      );
    });

    it("should select option when clicked", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      fireEvent.click(screen.getByTestId("rerun-option-revert_commit"));
      expect(screen.getByTestId("rerun-option-revert_commit")).toHaveAttribute(
        "data-selected",
        "true"
      );
      expect(screen.getByTestId("rerun-option-keep_changes")).toHaveAttribute(
        "data-selected",
        "false"
      );
    });

    it("should display option labels", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(
        screen.getByText("Keep changes, run task again")
      ).toBeInTheDocument();
      expect(
        screen.getByText("Revert commit, then run task")
      ).toBeInTheDocument();
      expect(screen.getByText("Create new task instead")).toBeInTheDocument();
    });

    it("should display option descriptions", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(
        screen.getByText(
          "AI will see current code state and make additional changes if needed"
        )
      ).toBeInTheDocument();
      expect(
        screen.getByText("Undo the previous work before re-executing")
      ).toBeInTheDocument();
      expect(
        screen.getByText("Original task stays completed, new task created")
      ).toBeInTheDocument();
    });

    it("should display 'Recommended' badge on keep_changes option", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      expect(screen.getByText("Recommended")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Dependent Commits Warning Tests
  // ============================================================================

  describe("Dependent Commits Warning", () => {
    it("should not show warning when revert is selected but no dependent commits", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      fireEvent.click(screen.getByTestId("rerun-option-revert_commit"));
      expect(
        screen.queryByTestId("dependent-commits-warning")
      ).not.toBeInTheDocument();
    });

    it("should show warning when revert is selected and has dependent commits", () => {
      render(
        <TaskRerunDialog
          {...createDefaultProps({ commitInfo: mockCommitInfoWithDependents })}
        />
      );
      fireEvent.click(screen.getByTestId("rerun-option-revert_commit"));
      expect(
        screen.getByTestId("dependent-commits-warning")
      ).toBeInTheDocument();
    });

    it("should display warning message about dependent commits", () => {
      render(
        <TaskRerunDialog
          {...createDefaultProps({ commitInfo: mockCommitInfoWithDependents })}
        />
      );
      fireEvent.click(screen.getByTestId("rerun-option-revert_commit"));
      expect(
        screen.getByText(/other commits depend on this one/i)
      ).toBeInTheDocument();
    });

    it("should hide warning when switching away from revert option", () => {
      render(
        <TaskRerunDialog
          {...createDefaultProps({ commitInfo: mockCommitInfoWithDependents })}
        />
      );
      fireEvent.click(screen.getByTestId("rerun-option-revert_commit"));
      expect(
        screen.getByTestId("dependent-commits-warning")
      ).toBeInTheDocument();
      fireEvent.click(screen.getByTestId("rerun-option-keep_changes"));
      expect(
        screen.queryByTestId("dependent-commits-warning")
      ).not.toBeInTheDocument();
    });

    it("should apply warning styling to revert option when dependent commits exist", () => {
      render(
        <TaskRerunDialog
          {...createDefaultProps({ commitInfo: mockCommitInfoWithDependents })}
        />
      );
      fireEvent.click(screen.getByTestId("rerun-option-revert_commit"));
      // The option should have warning border color - check that style attribute contains the warning color
      const option = screen.getByTestId("rerun-option-revert_commit");
      const styleAttr = option.getAttribute("style");
      expect(styleAttr).toContain("--status-warning");
    });
  });

  // ============================================================================
  // Confirm Flow Tests
  // ============================================================================

  describe("Confirm Flow", () => {
    it("should call onConfirm with keep_changes option by default", () => {
      const onConfirm = vi.fn();
      render(<TaskRerunDialog {...createDefaultProps({ onConfirm })} />);
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "keep_changes",
        task: mockTask,
      });
    });

    it("should call onConfirm with revert_commit option when selected", () => {
      const onConfirm = vi.fn();
      render(<TaskRerunDialog {...createDefaultProps({ onConfirm })} />);
      fireEvent.click(screen.getByTestId("rerun-option-revert_commit"));
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "revert_commit",
        task: mockTask,
      });
    });

    it("should call onConfirm with create_new option when selected", () => {
      const onConfirm = vi.fn();
      render(<TaskRerunDialog {...createDefaultProps({ onConfirm })} />);
      fireEvent.click(screen.getByTestId("rerun-option-create_new"));
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "create_new",
        task: mockTask,
      });
    });
  });

  // ============================================================================
  // Close/Cancel Tests
  // ============================================================================

  describe("Close and Cancel", () => {
    it("should call onClose when close button is clicked", () => {
      const onClose = vi.fn();
      render(<TaskRerunDialog {...createDefaultProps({ onClose })} />);
      fireEvent.click(screen.getByTestId("dialog-close"));
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("should call onClose when cancel button is clicked", () => {
      const onClose = vi.fn();
      render(<TaskRerunDialog {...createDefaultProps({ onClose })} />);
      fireEvent.click(screen.getByTestId("cancel-button"));
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("should call onClose when overlay is clicked", () => {
      const onClose = vi.fn();
      render(<TaskRerunDialog {...createDefaultProps({ onClose })} />);
      fireEvent.click(screen.getByTestId("dialog-overlay"));
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  // ============================================================================
  // Processing State Tests
  // ============================================================================

  describe("Processing State", () => {
    it("should display 'Processing...' when isProcessing is true", () => {
      render(
        <TaskRerunDialog {...createDefaultProps({ isProcessing: true })} />
      );
      expect(screen.getByTestId("confirm-button")).toHaveTextContent(
        "Processing..."
      );
    });

    it("should disable close button when processing", () => {
      render(
        <TaskRerunDialog {...createDefaultProps({ isProcessing: true })} />
      );
      expect(screen.getByTestId("dialog-close")).toBeDisabled();
    });

    it("should disable cancel button when processing", () => {
      render(
        <TaskRerunDialog {...createDefaultProps({ isProcessing: true })} />
      );
      expect(screen.getByTestId("cancel-button")).toBeDisabled();
    });

    it("should disable confirm button when processing", () => {
      render(
        <TaskRerunDialog {...createDefaultProps({ isProcessing: true })} />
      );
      expect(screen.getByTestId("confirm-button")).toBeDisabled();
    });

    it("should add opacity to options when processing", () => {
      render(
        <TaskRerunDialog {...createDefaultProps({ isProcessing: true })} />
      );
      const option = screen.getByTestId("rerun-option-keep_changes");
      expect(option).toHaveClass("opacity-50");
    });
  });

  // ============================================================================
  // Error State Tests
  // ============================================================================

  describe("Error State", () => {
    it("should not display error when error is null", () => {
      render(<TaskRerunDialog {...createDefaultProps({ error: null })} />);
      expect(screen.queryByTestId("dialog-error")).not.toBeInTheDocument();
    });

    it("should display error message when error is provided", () => {
      render(
        <TaskRerunDialog
          {...createDefaultProps({ error: "Revert failed due to conflicts" })}
        />
      );
      expect(screen.getByTestId("dialog-error")).toBeInTheDocument();
      expect(
        screen.getByText("Revert failed due to conflicts")
      ).toBeInTheDocument();
    });

    it("should display error with error styling", () => {
      render(
        <TaskRerunDialog {...createDefaultProps({ error: "Test error" })} />
      );
      const errorElement = screen.getByTestId("dialog-error");
      expect(errorElement).toHaveStyle({
        color: "var(--status-error)",
      });
    });
  });

  // ============================================================================
  // State Reset Tests
  // ============================================================================

  describe("State Reset", () => {
    it("should reset to keep_changes option when dialog reopens", () => {
      const { rerender } = render(
        <TaskRerunDialog {...createDefaultProps()} />
      );
      // Select a different option
      fireEvent.click(screen.getByTestId("rerun-option-revert_commit"));
      expect(screen.getByTestId("rerun-option-revert_commit")).toHaveAttribute(
        "data-selected",
        "true"
      );
      // Close the dialog
      rerender(
        <TaskRerunDialog {...createDefaultProps({ isOpen: false })} />
      );
      // Reopen the dialog
      rerender(<TaskRerunDialog {...createDefaultProps()} />);
      // Should be back to keep_changes
      expect(screen.getByTestId("rerun-option-keep_changes")).toHaveAttribute(
        "data-selected",
        "true"
      );
    });
  });

  // ============================================================================
  // Styling Tests
  // ============================================================================

  describe("Styling", () => {
    it("should use warm orange accent for confirm button", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const button = screen.getByTestId("confirm-button");
      expect(button).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });

    it("should use hover color for confirm button when processing", () => {
      render(
        <TaskRerunDialog {...createDefaultProps({ isProcessing: true })} />
      );
      const button = screen.getByTestId("confirm-button");
      expect(button).toHaveStyle({ backgroundColor: "var(--bg-hover)" });
    });

    it("should display commit SHA with monospace font", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const sha = screen.getByTestId("commit-sha");
      expect(sha).toHaveClass("font-mono");
    });

    it("should display commit SHA with accent color", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const sha = screen.getByTestId("commit-sha");
      expect(sha).toHaveStyle({ color: "var(--accent-primary)" });
    });
  });

  // ============================================================================
  // Accessibility Tests
  // ============================================================================

  describe("Accessibility", () => {
    it("should have radio inputs with correct name attribute", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const radios = document.querySelectorAll('input[type="radio"]');
      radios.forEach((radio) => {
        expect(radio).toHaveAttribute("name", "rerunOption");
      });
    });

    it("should have sr-only class on radio inputs for visual hiding", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const radios = document.querySelectorAll('input[type="radio"]');
      radios.forEach((radio) => {
        expect(radio).toHaveClass("sr-only");
      });
    });

    it("should have all three options as radio inputs", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const radios = document.querySelectorAll('input[type="radio"]');
      expect(radios.length).toBe(3);
    });
  });

  // ============================================================================
  // Different Task Types
  // ============================================================================

  describe("Different Task Types", () => {
    it("should work with task that has null description", () => {
      const taskWithNullDesc: Task = {
        ...mockTask,
        description: null,
      };
      const onConfirm = vi.fn();
      render(
        <TaskRerunDialog
          {...createDefaultProps({ task: taskWithNullDesc, onConfirm })}
        />
      );
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "keep_changes",
        task: taskWithNullDesc,
      });
    });

    it("should display correct task title for different tasks", () => {
      const differentTask: Task = {
        ...mockTask,
        title: "Refactor database layer",
      };
      render(
        <TaskRerunDialog {...createDefaultProps({ task: differentTask })} />
      );
      expect(screen.getByTestId("task-title")).toHaveTextContent(
        '"Refactor database layer"'
      );
    });

    it("should display correct commit info for different commits", () => {
      const differentCommit: CommitInfo = {
        sha: "xyz9876",
        message: "fix: Resolve memory leak",
        hasDependentCommits: false,
      };
      render(
        <TaskRerunDialog
          {...createDefaultProps({ commitInfo: differentCommit })}
        />
      );
      expect(screen.getByTestId("commit-sha")).toHaveTextContent("xyz9876");
      expect(screen.getByTestId("commit-message")).toHaveTextContent(
        '"fix: Resolve memory leak"'
      );
    });
  });

  // ============================================================================
  // Icon Rendering Tests
  // ============================================================================

  describe("Icon Rendering", () => {
    it("should render refresh icon in header", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      // The header should contain the RefreshIcon SVG
      const header = screen.getByTestId("dialog-title").parentElement;
      expect(header?.querySelector("svg")).toBeInTheDocument();
    });

    it("should render check icon for keep_changes option", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const option = screen.getByTestId("rerun-option-keep_changes");
      expect(option.querySelector("svg")).toBeInTheDocument();
    });

    it("should render revert icon for revert_commit option", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const option = screen.getByTestId("rerun-option-revert_commit");
      expect(option.querySelector("svg")).toBeInTheDocument();
    });

    it("should render plus icon for create_new option", () => {
      render(<TaskRerunDialog {...createDefaultProps()} />);
      const option = screen.getByTestId("rerun-option-create_new");
      expect(option.querySelector("svg")).toBeInTheDocument();
    });
  });
});
