import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  MergeWorkflowDialog,
  type MergeWorkflowDialogProps,
  type MergeOption,
  type CompletionData,
} from "./MergeWorkflowDialog";
import type { Project } from "@/types/project";

// ============================================================================
// Test Fixtures
// ============================================================================

const mockProject: Project = {
  id: "test-project-id",
  name: "Test Project",
  workingDirectory: "/Users/dev/test-project",
  gitMode: "worktree",
  worktreePath: "~/ralphx-worktrees/test-project",
  worktreeBranch: "ralphx/feature-auth",
  baseBranch: "main",
  createdAt: "2026-01-25T00:00:00Z",
  updatedAt: "2026-01-25T00:00:00Z",
};

const mockCompletionData: CompletionData = {
  commitCount: 12,
  branchName: "ralphx/feature-auth",
};

function createDefaultProps(
  overrides: Partial<MergeWorkflowDialogProps> = {}
): MergeWorkflowDialogProps {
  return {
    isOpen: true,
    onClose: vi.fn(),
    onConfirm: vi.fn(),
    project: mockProject,
    completionData: mockCompletionData,
    ...overrides,
  };
}

// ============================================================================
// Rendering Tests
// ============================================================================

describe("MergeWorkflowDialog", () => {
  describe("Rendering", () => {
    it("should not render when isOpen is false", () => {
      render(<MergeWorkflowDialog {...createDefaultProps({ isOpen: false })} />);
      expect(screen.queryByTestId("merge-workflow-dialog")).not.toBeInTheDocument();
    });

    it("should render dialog when isOpen is true", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("merge-workflow-dialog")).toBeInTheDocument();
    });

    it("should display project name in header", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByText("Project Complete: Test Project")).toBeInTheDocument();
    });

    it("should display commit count from completionData", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("commit-count")).toHaveTextContent("12 commits");
    });

    it("should display singular 'commit' for count of 1", () => {
      render(
        <MergeWorkflowDialog
          {...createDefaultProps({
            completionData: { commitCount: 1, branchName: "test-branch" },
          })}
        />
      );
      expect(screen.getByTestId("commit-count")).toHaveTextContent("1 commit");
    });

    it("should display branch name from completionData", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("branch-name")).toHaveTextContent(
        "ralphx/feature-auth"
      );
    });

    it("should display close button (shadcn dialog has default close)", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByRole("button", { name: /close/i })).toBeInTheDocument();
    });

    it("should display cancel and confirm buttons", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("cancel-button")).toBeInTheDocument();
      expect(screen.getByTestId("confirm-button")).toBeInTheDocument();
    });

    it("should display 'Continue' text on confirm button by default", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("confirm-button")).toHaveTextContent("Continue");
    });
  });

  // ============================================================================
  // View Buttons Tests
  // ============================================================================

  describe("View Buttons", () => {
    it("should render View Diff button when onViewDiff is provided", () => {
      render(
        <MergeWorkflowDialog
          {...createDefaultProps({ onViewDiff: vi.fn() })}
        />
      );
      expect(screen.getByTestId("view-diff-button")).toBeInTheDocument();
    });

    it("should not render View Diff button when onViewDiff is not provided", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.queryByTestId("view-diff-button")).not.toBeInTheDocument();
    });

    it("should call onViewDiff when View Diff button is clicked", () => {
      const onViewDiff = vi.fn();
      render(
        <MergeWorkflowDialog {...createDefaultProps({ onViewDiff })} />
      );
      fireEvent.click(screen.getByTestId("view-diff-button"));
      expect(onViewDiff).toHaveBeenCalledTimes(1);
    });

    it("should render View Commits button when onViewCommits is provided", () => {
      render(
        <MergeWorkflowDialog
          {...createDefaultProps({ onViewCommits: vi.fn() })}
        />
      );
      expect(screen.getByTestId("view-commits-button")).toBeInTheDocument();
    });

    it("should not render View Commits button when onViewCommits is not provided", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.queryByTestId("view-commits-button")).not.toBeInTheDocument();
    });

    it("should call onViewCommits when View Commits button is clicked", () => {
      const onViewCommits = vi.fn();
      render(
        <MergeWorkflowDialog {...createDefaultProps({ onViewCommits })} />
      );
      fireEvent.click(screen.getByTestId("view-commits-button"));
      expect(onViewCommits).toHaveBeenCalledTimes(1);
    });
  });

  // ============================================================================
  // Merge Options Tests
  // ============================================================================

  describe("Merge Options", () => {
    const optionIds: MergeOption[] = [
      "merge",
      "rebase",
      "create_pr",
      "keep_worktree",
      "discard",
    ];

    it.each(optionIds)("should render %s option", (option) => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByTestId(`merge-option-${option}`)).toBeInTheDocument();
    });

    it("should have merge option selected by default", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByTestId("merge-option-merge")).toHaveAttribute(
        "data-selected",
        "true"
      );
    });

    it("should select option when clicked", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      fireEvent.click(screen.getByTestId("merge-option-rebase"));
      expect(screen.getByTestId("merge-option-rebase")).toHaveAttribute(
        "data-selected",
        "true"
      );
      expect(screen.getByTestId("merge-option-merge")).toHaveAttribute(
        "data-selected",
        "false"
      );
    });

    it("should display option labels", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(screen.getByText("Merge to main")).toBeInTheDocument();
      expect(screen.getByText("Rebase onto main")).toBeInTheDocument();
      expect(screen.getByText("Create Pull Request")).toBeInTheDocument();
      expect(screen.getByText("Keep worktree")).toBeInTheDocument();
      expect(screen.getByText("Discard changes")).toBeInTheDocument();
    });

    it("should display option descriptions", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      expect(
        screen.getByText("Creates a merge commit preserving branch history")
      ).toBeInTheDocument();
      expect(
        screen.getByText("Replays commits on top of main for linear history")
      ).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Discard Confirmation Tests
  // ============================================================================

  describe("Discard Confirmation", () => {
    it("should show warning when discard is selected", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      fireEvent.click(screen.getByTestId("merge-option-discard"));
      expect(
        screen.getByText("This cannot be undone. All commits will be lost.")
      ).toBeInTheDocument();
    });

    it("should show confirmation prompt on first confirm click when discard is selected", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      fireEvent.click(screen.getByTestId("merge-option-discard"));
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(screen.getByTestId("discard-confirmation")).toBeInTheDocument();
    });

    it("should change button text to 'Confirm Discard' after first click", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      fireEvent.click(screen.getByTestId("merge-option-discard"));
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(screen.getByTestId("confirm-button")).toHaveTextContent(
        "Confirm Discard"
      );
    });

    it("should call onConfirm with discard option on second click", () => {
      const onConfirm = vi.fn();
      render(<MergeWorkflowDialog {...createDefaultProps({ onConfirm })} />);
      fireEvent.click(screen.getByTestId("merge-option-discard"));
      fireEvent.click(screen.getByTestId("confirm-button")); // First click - show confirmation
      fireEvent.click(screen.getByTestId("confirm-button")); // Second click - confirm
      expect(onConfirm).toHaveBeenCalledWith({
        option: "discard",
        project: mockProject,
      });
    });

    it("should reset confirmation when switching away from discard", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      fireEvent.click(screen.getByTestId("merge-option-discard"));
      fireEvent.click(screen.getByTestId("confirm-button")); // Show confirmation
      expect(screen.getByTestId("discard-confirmation")).toBeInTheDocument();
      fireEvent.click(screen.getByTestId("merge-option-merge")); // Switch option
      expect(screen.queryByTestId("discard-confirmation")).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Confirm Flow Tests
  // ============================================================================

  describe("Confirm Flow", () => {
    it("should call onConfirm with merge option by default", () => {
      const onConfirm = vi.fn();
      render(<MergeWorkflowDialog {...createDefaultProps({ onConfirm })} />);
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "merge",
        project: mockProject,
      });
    });

    it("should call onConfirm with rebase option when selected", () => {
      const onConfirm = vi.fn();
      render(<MergeWorkflowDialog {...createDefaultProps({ onConfirm })} />);
      fireEvent.click(screen.getByTestId("merge-option-rebase"));
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "rebase",
        project: mockProject,
      });
    });

    it("should call onConfirm with create_pr option when selected", () => {
      const onConfirm = vi.fn();
      render(<MergeWorkflowDialog {...createDefaultProps({ onConfirm })} />);
      fireEvent.click(screen.getByTestId("merge-option-create_pr"));
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "create_pr",
        project: mockProject,
      });
    });

    it("should call onConfirm with keep_worktree option when selected", () => {
      const onConfirm = vi.fn();
      render(<MergeWorkflowDialog {...createDefaultProps({ onConfirm })} />);
      fireEvent.click(screen.getByTestId("merge-option-keep_worktree"));
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "keep_worktree",
        project: mockProject,
      });
    });
  });

  // ============================================================================
  // Close/Cancel Tests
  // ============================================================================

  describe("Close and Cancel", () => {
    it("should call onClose when close button is clicked", () => {
      const onClose = vi.fn();
      render(<MergeWorkflowDialog {...createDefaultProps({ onClose })} />);
      // shadcn Dialog close button has sr-only "Close" text
      fireEvent.click(screen.getByRole("button", { name: /close/i }));
      expect(onClose).toHaveBeenCalledTimes(1);
    });

    it("should call onClose when cancel button is clicked", () => {
      const onClose = vi.fn();
      render(<MergeWorkflowDialog {...createDefaultProps({ onClose })} />);
      fireEvent.click(screen.getByTestId("cancel-button"));
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });

  // ============================================================================
  // Processing State Tests
  // ============================================================================

  describe("Processing State", () => {
    it("should display 'Processing...' when isProcessing is true", () => {
      render(
        <MergeWorkflowDialog {...createDefaultProps({ isProcessing: true })} />
      );
      expect(screen.getByTestId("confirm-button")).toHaveTextContent(
        "Processing..."
      );
    });

    it("should disable cancel button when processing", () => {
      render(
        <MergeWorkflowDialog {...createDefaultProps({ isProcessing: true })} />
      );
      expect(screen.getByTestId("cancel-button")).toBeDisabled();
    });

    it("should disable confirm button when processing", () => {
      render(
        <MergeWorkflowDialog {...createDefaultProps({ isProcessing: true })} />
      );
      expect(screen.getByTestId("confirm-button")).toBeDisabled();
    });

    it("should disable view diff button when processing", () => {
      render(
        <MergeWorkflowDialog
          {...createDefaultProps({
            isProcessing: true,
            onViewDiff: vi.fn(),
          })}
        />
      );
      expect(screen.getByTestId("view-diff-button")).toBeDisabled();
    });

    it("should disable view commits button when processing", () => {
      render(
        <MergeWorkflowDialog
          {...createDefaultProps({
            isProcessing: true,
            onViewCommits: vi.fn(),
          })}
        />
      );
      expect(screen.getByTestId("view-commits-button")).toBeDisabled();
    });
  });

  // ============================================================================
  // Error State Tests
  // ============================================================================

  describe("Error State", () => {
    it("should not display error when error is null", () => {
      render(<MergeWorkflowDialog {...createDefaultProps({ error: null })} />);
      expect(screen.queryByTestId("dialog-error")).not.toBeInTheDocument();
    });

    it("should display error message when error is provided", () => {
      render(
        <MergeWorkflowDialog
          {...createDefaultProps({ error: "Something went wrong" })}
        />
      );
      expect(screen.getByTestId("dialog-error")).toBeInTheDocument();
      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // State Reset Tests
  // ============================================================================

  describe("State Reset", () => {
    it("should reset to merge option when dialog reopens", () => {
      const { rerender } = render(
        <MergeWorkflowDialog {...createDefaultProps()} />
      );
      // Select a different option
      fireEvent.click(screen.getByTestId("merge-option-discard"));
      // Close the dialog
      rerender(
        <MergeWorkflowDialog {...createDefaultProps({ isOpen: false })} />
      );
      // Reopen the dialog
      rerender(<MergeWorkflowDialog {...createDefaultProps()} />);
      // Should be back to merge
      expect(screen.getByTestId("merge-option-merge")).toHaveAttribute(
        "data-selected",
        "true"
      );
    });

    it("should reset discard confirmation when dialog reopens", () => {
      const { rerender } = render(
        <MergeWorkflowDialog {...createDefaultProps()} />
      );
      // Select discard and trigger confirmation
      fireEvent.click(screen.getByTestId("merge-option-discard"));
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(screen.getByTestId("discard-confirmation")).toBeInTheDocument();
      // Close the dialog
      rerender(
        <MergeWorkflowDialog {...createDefaultProps({ isOpen: false })} />
      );
      // Reopen the dialog
      rerender(<MergeWorkflowDialog {...createDefaultProps()} />);
      // Confirmation should be gone
      expect(screen.queryByTestId("discard-confirmation")).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Styling Tests
  // ============================================================================

  describe("Styling", () => {
    it("should apply accent styling for non-destructive confirm button", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      const button = screen.getByTestId("confirm-button");
      // When merge (default) is selected, button should have accent styling class
      expect(button.className).toContain("bg-[var(--accent-primary)]");
    });

    it("should apply error styling for confirm button when discard is selected", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      fireEvent.click(screen.getByTestId("merge-option-discard"));
      const button = screen.getByTestId("confirm-button");
      // When discard is selected, button should have error styling class
      expect(button.className).toContain("bg-[var(--status-error)]");
    });

    it("should apply disabled styling for confirm button when processing", () => {
      render(
        <MergeWorkflowDialog {...createDefaultProps({ isProcessing: true })} />
      );
      const button = screen.getByTestId("confirm-button");
      // When processing, button should have hover/disabled styling class
      expect(button.className).toContain("bg-[var(--bg-hover)]");
    });
  });

  // ============================================================================
  // Accessibility Tests
  // ============================================================================

  describe("Accessibility", () => {
    it("should have radio inputs with correct name attribute", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      const radios = document.querySelectorAll('input[type="radio"]');
      radios.forEach((radio) => {
        expect(radio).toHaveAttribute("name", "mergeOption");
      });
    });

    it("should have sr-only class on radio inputs for visual hiding", () => {
      render(<MergeWorkflowDialog {...createDefaultProps()} />);
      const radios = document.querySelectorAll('input[type="radio"]');
      radios.forEach((radio) => {
        expect(radio).toHaveClass("sr-only");
      });
    });
  });

  // ============================================================================
  // Different Project Types
  // ============================================================================

  describe("Different Project Types", () => {
    it("should work with local git mode project", () => {
      const localProject: Project = {
        ...mockProject,
        gitMode: "local",
        worktreePath: null,
        worktreeBranch: null,
        baseBranch: null,
      };
      const onConfirm = vi.fn();
      render(
        <MergeWorkflowDialog
          {...createDefaultProps({ project: localProject, onConfirm })}
        />
      );
      fireEvent.click(screen.getByTestId("confirm-button"));
      expect(onConfirm).toHaveBeenCalledWith({
        option: "merge",
        project: localProject,
      });
    });

    it("should display correct project name for different projects", () => {
      const differentProject: Project = {
        ...mockProject,
        name: "My Awesome App",
      };
      render(
        <MergeWorkflowDialog
          {...createDefaultProps({ project: differentProject })}
        />
      );
      expect(screen.getByText("Project Complete: My Awesome App")).toBeInTheDocument();
    });
  });
});
