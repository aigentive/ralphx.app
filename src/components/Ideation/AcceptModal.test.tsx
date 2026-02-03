/**
 * AcceptModal.test.tsx
 * Tests for the modal that accepts a plan and creates tasks in Kanban
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AcceptModal } from "./AcceptModal";
import type { TaskProposal, DependencyGraph } from "@/types/ideation";

const mockProposals: TaskProposal[] = [
  {
    id: "proposal-1",
    sessionId: "session-1",
    title: "Setup database",
    description: "Initialize SQLite database",
    category: "setup",
    steps: [],
    acceptanceCriteria: [],
    suggestedPriority: "high",
    priorityScore: 75,
    priorityReason: "Foundation task",
    estimatedComplexity: "moderate",
    userPriority: null,
    userModified: false,
    status: "pending",
    createdTaskId: null,
    sortOrder: 0,
    createdAt: "2026-01-24T00:00:00Z",
    updatedAt: "2026-01-24T00:00:00Z",
  },
  {
    id: "proposal-2",
    sessionId: "session-1",
    title: "Create user model",
    description: "Define user entity",
    category: "feature",
    steps: [],
    acceptanceCriteria: [],
    suggestedPriority: "medium",
    priorityScore: 55,
    priorityReason: "Depends on database",
    estimatedComplexity: "simple",
    userPriority: null,
    userModified: false,
    status: "pending",
    createdTaskId: null,
    sortOrder: 1,
    createdAt: "2026-01-24T00:00:00Z",
    updatedAt: "2026-01-24T00:00:00Z",
  },
  {
    id: "proposal-3",
    sessionId: "session-1",
    title: "Add authentication",
    description: "Implement login/logout",
    category: "feature",
    steps: [],
    acceptanceCriteria: [],
    suggestedPriority: "high",
    priorityScore: 70,
    priorityReason: "Core feature",
    estimatedComplexity: "complex",
    userPriority: null,
    userModified: false,
    status: "pending",
    createdTaskId: null,
    sortOrder: 2,
    createdAt: "2026-01-24T00:00:00Z",
    updatedAt: "2026-01-24T00:00:00Z",
  },
];

const mockDependencyGraph: DependencyGraph = {
  nodes: [
    { proposalId: "proposal-1", title: "Setup database", inDegree: 0, outDegree: 1 },
    { proposalId: "proposal-2", title: "Create user model", inDegree: 1, outDegree: 1 },
    { proposalId: "proposal-3", title: "Add authentication", inDegree: 1, outDegree: 0 },
  ],
  edges: [
    { from: "proposal-1", to: "proposal-2" },
    { from: "proposal-2", to: "proposal-3" },
  ],
  criticalPath: ["proposal-1", "proposal-2", "proposal-3"],
  hasCycles: false,
  cycles: null,
};

const mockDependencyGraphWithCycles: DependencyGraph = {
  ...mockDependencyGraph,
  hasCycles: true,
  cycles: [["proposal-1", "proposal-2", "proposal-3", "proposal-1"]],
};

describe("AcceptModal", () => {
  const defaultProps = {
    isOpen: true,
    proposals: mockProposals,
    dependencyGraph: mockDependencyGraph,
    sessionId: "session-1",
    onAccept: vi.fn(),
    onCancel: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Rendering", () => {
    it("renders modal when isOpen is true", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByTestId("accept-modal")).toBeInTheDocument();
    });

    it("does not render when isOpen is false", () => {
      render(<AcceptModal {...defaultProps} isOpen={false} />);
      expect(screen.queryByTestId("accept-modal")).not.toBeInTheDocument();
    });

    it("renders modal overlay", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByTestId("modal-overlay")).toBeInTheDocument();
    });

    it("renders modal content", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByTestId("modal-content")).toBeInTheDocument();
    });

    it("renders header with title", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByText("Accept Plan")).toBeInTheDocument();
    });
  });

  describe("Tasks Summary", () => {
    it("displays summary section", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByText("Tasks to Create")).toBeInTheDocument();
    });

    it("shows count of tasks to create", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByText(/3 tasks? will be created/i)).toBeInTheDocument();
    });

    it("lists all proposal titles", () => {
      render(<AcceptModal {...defaultProps} />);
      // Titles may appear in both proposals list and dependency graph
      expect(screen.getAllByText("Setup database").length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText("Create user model").length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText("Add authentication").length).toBeGreaterThanOrEqual(1);
    });

    it("shows proposal categories", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByText("setup")).toBeInTheDocument();
      expect(screen.getAllByText("feature")).toHaveLength(2);
    });

    it("shows singular 'task' when only one proposal", () => {
      render(<AcceptModal {...defaultProps} proposals={[mockProposals[0]]} />);
      expect(screen.getByText("1 task will be created")).toBeInTheDocument();
    });
  });

  describe("Dependency Graph Preview", () => {
    it("displays dependency graph section", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByText("Dependencies")).toBeInTheDocument();
    });

    it("shows dependency count", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByText(/2 dependencies/i)).toBeInTheDocument();
    });

    it("displays dependency edges", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByTestId("dependency-preview")).toBeInTheDocument();
    });

    it("shows no dependencies message when empty", () => {
      const emptyGraph: DependencyGraph = {
        nodes: [],
        edges: [],
        criticalPath: [],
        hasCycles: false,
        cycles: null,
      };
      render(<AcceptModal {...defaultProps} dependencyGraph={emptyGraph} />);
      expect(screen.getByText("No dependencies")).toBeInTheDocument();
    });

    it("shows critical path when present", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByText(/Critical path:/i)).toBeInTheDocument();
    });
  });

  describe("Target Column Selector", () => {
    it("renders column selector with label", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByLabelText("Target Column")).toBeInTheDocument();
    });

    it("shows all column options", () => {
      render(<AcceptModal {...defaultProps} />);
      const select = screen.getByLabelText("Target Column") as HTMLSelectElement;
      expect(select.querySelector('option[value="draft"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="backlog"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="todo"]')).toBeInTheDocument();
    });

    it("defaults to backlog column", () => {
      render(<AcceptModal {...defaultProps} />);
      const select = screen.getByLabelText("Target Column") as HTMLSelectElement;
      expect(select.value).toBe("backlog");
    });

    it("allows changing column selection", async () => {
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} />);
      const select = screen.getByLabelText("Target Column");
      await user.selectOptions(select, "todo");
      expect(select).toHaveValue("todo");
    });
  });

  describe("Preserve Dependencies Checkbox", () => {
    it("renders checkbox with label", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByLabelText(/Preserve dependencies/i)).toBeInTheDocument();
    });

    it("checkbox is checked by default", () => {
      render(<AcceptModal {...defaultProps} />);
      const checkbox = screen.getByLabelText(/Preserve dependencies/i) as HTMLInputElement;
      expect(checkbox.checked).toBe(true);
    });

    it("allows toggling checkbox", async () => {
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} />);
      const checkbox = screen.getByLabelText(/Preserve dependencies/i);
      await user.click(checkbox);
      expect(checkbox).not.toBeChecked();
    });

    it("shows helper text explaining the option", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByText(/Create task dependencies/i)).toBeInTheDocument();
    });
  });

  describe("Warnings Display", () => {
    it("shows warning when cycles detected", () => {
      render(<AcceptModal {...defaultProps} dependencyGraph={mockDependencyGraphWithCycles} />);
      expect(screen.getByTestId("warning-cycles")).toBeInTheDocument();
      expect(screen.getByText(/Circular dependency detected/i)).toBeInTheDocument();
    });

    it("does not show cycle warning when no cycles", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.queryByTestId("warning-cycles")).not.toBeInTheDocument();
    });

    it("shows warning for missing dependencies", () => {
      // Create a proposal that depends on a non-selected proposal
      const incompleteProposals = [mockProposals[1]]; // depends on proposal-1 which is not selected
      const incompleteGraph: DependencyGraph = {
        nodes: [{ proposalId: "proposal-2", title: "Create user model", inDegree: 1, outDegree: 0 }],
        edges: [{ from: "proposal-1", to: "proposal-2" }],
        criticalPath: [],
        hasCycles: false,
        cycles: null,
      };
      render(
        <AcceptModal
          {...defaultProps}
          proposals={incompleteProposals}
          dependencyGraph={incompleteGraph}
          warnings={["Missing dependency: Setup database"]}
        />
      );
      expect(screen.getByTestId("warning-missing")).toBeInTheDocument();
    });

    it("renders multiple warnings", () => {
      render(
        <AcceptModal
          {...defaultProps}
          dependencyGraph={mockDependencyGraphWithCycles}
          warnings={["Missing dependency: Task A", "Missing dependency: Task B"]}
        />
      );
      const warnings = screen.getAllByTestId(/warning-/);
      expect(warnings.length).toBeGreaterThanOrEqual(2);
    });

    it("uses warning color for warning messages", () => {
      render(<AcceptModal {...defaultProps} dependencyGraph={mockDependencyGraphWithCycles} />);
      const warningEl = screen.getByTestId("warning-cycles");
      expect(warningEl).toHaveStyle({ color: "var(--status-warning)" });
    });
  });

  describe("Accept and Cancel Buttons", () => {
    it("renders accept button", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByRole("button", { name: /Accept Plan/i })).toBeInTheDocument();
    });

    it("renders cancel button", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    });

    it("calls onAccept with correct options when accept clicked", async () => {
      const onAccept = vi.fn();
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} onAccept={onAccept} />);

      const acceptButton = screen.getByRole("button", { name: /Accept Plan/i });
      await user.click(acceptButton);

      expect(onAccept).toHaveBeenCalledTimes(1);
      expect(onAccept).toHaveBeenCalledWith({
        sessionId: "session-1",
        proposalIds: ["proposal-1", "proposal-2", "proposal-3"],
        targetColumn: "backlog",
        preserveDependencies: true,
      });
    });

    it("calls onAccept with updated options", async () => {
      const onAccept = vi.fn();
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} onAccept={onAccept} />);

      // Change column
      const select = screen.getByLabelText("Target Column");
      await user.selectOptions(select, "draft");

      // Uncheck preserve dependencies
      const checkbox = screen.getByLabelText(/Preserve dependencies/i);
      await user.click(checkbox);

      const acceptButton = screen.getByRole("button", { name: /Accept Plan/i });
      await user.click(acceptButton);

      expect(onAccept).toHaveBeenCalledWith({
        sessionId: "session-1",
        proposalIds: ["proposal-1", "proposal-2", "proposal-3"],
        targetColumn: "draft",
        preserveDependencies: false,
      });
    });

    it("calls onCancel when cancel clicked", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} onCancel={onCancel} />);

      const cancelButton = screen.getByRole("button", { name: "Cancel" });
      await user.click(cancelButton);

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it("accept button is disabled when no proposals", () => {
      render(<AcceptModal {...defaultProps} proposals={[]} />);
      const acceptButton = screen.getByRole("button", { name: /Accept Plan/i });
      expect(acceptButton).toBeDisabled();
    });

    it("accept button shows count in label", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByRole("button", { name: /Accept Plan \(3 tasks\)/i })).toBeInTheDocument();
    });
  });

  describe("Loading State", () => {
    it("shows loading state when isAccepting is true", () => {
      render(<AcceptModal {...defaultProps} isAccepting={true} />);
      expect(screen.getByRole("button", { name: /Accepting.../i })).toBeInTheDocument();
    });

    it("disables accept button when accepting", () => {
      render(<AcceptModal {...defaultProps} isAccepting={true} />);
      const acceptButton = screen.getByRole("button", { name: /Accepting.../i });
      expect(acceptButton).toBeDisabled();
    });

    it("disables cancel button when accepting", () => {
      render(<AcceptModal {...defaultProps} isAccepting={true} />);
      const cancelButton = screen.getByRole("button", { name: "Cancel" });
      expect(cancelButton).toBeDisabled();
    });

    it("disables column selector when accepting", () => {
      render(<AcceptModal {...defaultProps} isAccepting={true} />);
      const select = screen.getByLabelText("Target Column");
      expect(select).toBeDisabled();
    });

    it("disables checkbox when accepting", () => {
      render(<AcceptModal {...defaultProps} isAccepting={true} />);
      const checkbox = screen.getByLabelText(/Preserve dependencies/i);
      expect(checkbox).toBeDisabled();
    });
  });

  describe("Overlay Click Behavior", () => {
    it("calls onCancel when overlay clicked", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} onCancel={onCancel} />);

      const overlay = screen.getByTestId("modal-overlay");
      await user.click(overlay);

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it("does not call onCancel when content clicked", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} onCancel={onCancel} />);

      const content = screen.getByTestId("modal-content");
      await user.click(content);

      expect(onCancel).not.toHaveBeenCalled();
    });

    it("does not call onCancel on overlay click when accepting", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} onCancel={onCancel} isAccepting={true} />);

      const overlay = screen.getByTestId("modal-overlay");
      await user.click(overlay);

      expect(onCancel).not.toHaveBeenCalled();
    });
  });

  describe("Accessibility", () => {
    it("has accessible name for modal", () => {
      render(<AcceptModal {...defaultProps} />);
      const modal = screen.getByRole("dialog");
      expect(modal).toHaveAccessibleName("Accept Plan");
    });

    it("has proper labels for form controls", () => {
      render(<AcceptModal {...defaultProps} />);
      expect(screen.getByLabelText("Target Column")).toBeInTheDocument();
      expect(screen.getByLabelText(/Preserve dependencies/i)).toBeInTheDocument();
    });

    it("warnings have role=alert", () => {
      render(<AcceptModal {...defaultProps} dependencyGraph={mockDependencyGraphWithCycles} />);
      const warning = screen.getByTestId("warning-cycles");
      expect(warning).toHaveAttribute("role", "alert");
    });
  });

  describe("Keyboard Navigation", () => {
    it("closes modal on Escape key", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} onCancel={onCancel} />);

      await user.keyboard("{Escape}");

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it("does not close on Escape when accepting", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<AcceptModal {...defaultProps} onCancel={onCancel} isAccepting={true} />);

      await user.keyboard("{Escape}");

      expect(onCancel).not.toHaveBeenCalled();
    });
  });

  describe("Styling", () => {
    it("uses correct overlay styling", () => {
      render(<AcceptModal {...defaultProps} />);
      const overlay = screen.getByTestId("modal-overlay");
      expect(overlay).toHaveStyle({ backgroundColor: "rgba(0, 0, 0, 0.5)" });
    });

    it("modal is centered with fixed positioning", () => {
      render(<AcceptModal {...defaultProps} />);
      const modal = screen.getByTestId("accept-modal");
      expect(modal).toHaveClass("fixed", "inset-0", "z-50");
    });

    it("content has elevated background", () => {
      render(<AcceptModal {...defaultProps} />);
      const content = screen.getByTestId("modal-content");
      expect(content).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("accept button uses accent color", () => {
      render(<AcceptModal {...defaultProps} />);
      const acceptButton = screen.getByRole("button", { name: /Accept Plan/i });
      expect(acceptButton).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });

    it("anti-ai-slop: no purple gradients", () => {
      render(<AcceptModal {...defaultProps} />);
      const modal = screen.getByTestId("accept-modal");
      const styles = window.getComputedStyle(modal);
      expect(styles.background).not.toMatch(/purple|#800080|#a855f7/i);
    });
  });
});
