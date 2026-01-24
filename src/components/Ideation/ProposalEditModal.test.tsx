/**
 * ProposalEditModal.test.tsx
 * Tests for the proposal editing modal component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ProposalEditModal } from "./ProposalEditModal";
import type { TaskProposal } from "@/types/ideation";

const mockProposal: TaskProposal = {
  id: "proposal-1",
  sessionId: "session-1",
  title: "Implement user authentication",
  description: "Add login and registration functionality",
  category: "feature",
  steps: ["Create login form", "Implement JWT validation", "Add logout button"],
  acceptanceCriteria: ["Users can log in", "Users can register"],
  suggestedPriority: "high",
  priorityScore: 75,
  priorityReason: "Core feature",
  estimatedComplexity: "moderate",
  userPriority: null,
  userModified: false,
  status: "pending",
  selected: false,
  createdTaskId: null,
  sortOrder: 0,
  createdAt: "2026-01-24T00:00:00Z",
  updatedAt: "2026-01-24T00:00:00Z",
};

describe("ProposalEditModal", () => {
  const defaultProps = {
    proposal: mockProposal,
    onSave: vi.fn(),
    onCancel: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Rendering", () => {
    it("renders modal with overlay", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByTestId("proposal-edit-modal")).toBeInTheDocument();
      expect(screen.getByTestId("modal-overlay")).toBeInTheDocument();
    });

    it("renders modal content container", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByTestId("modal-content")).toBeInTheDocument();
    });

    it("renders header with title", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByText("Edit Proposal")).toBeInTheDocument();
    });

    it("does not render when proposal is null", () => {
      render(<ProposalEditModal {...defaultProps} proposal={null} />);
      expect(screen.queryByTestId("proposal-edit-modal")).not.toBeInTheDocument();
    });
  });

  describe("Title Input", () => {
    it("renders title input with label", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByLabelText("Title")).toBeInTheDocument();
    });

    it("shows proposal title in input", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const input = screen.getByLabelText("Title") as HTMLInputElement;
      expect(input.value).toBe("Implement user authentication");
    });

    it("allows editing title", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const input = screen.getByLabelText("Title");
      await user.clear(input);
      await user.type(input, "New title");
      expect(input).toHaveValue("New title");
    });
  });

  describe("Description Textarea", () => {
    it("renders description textarea with label", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByLabelText("Description")).toBeInTheDocument();
    });

    it("shows proposal description in textarea", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const textarea = screen.getByLabelText("Description") as HTMLTextAreaElement;
      expect(textarea.value).toBe("Add login and registration functionality");
    });

    it("allows editing description", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const textarea = screen.getByLabelText("Description");
      await user.clear(textarea);
      await user.type(textarea, "New description");
      expect(textarea).toHaveValue("New description");
    });

    it("handles null description", () => {
      const proposalWithNullDesc = { ...mockProposal, description: null };
      render(<ProposalEditModal {...defaultProps} proposal={proposalWithNullDesc} />);
      const textarea = screen.getByLabelText("Description") as HTMLTextAreaElement;
      expect(textarea.value).toBe("");
    });
  });

  describe("Category Selector", () => {
    it("renders category selector with label", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByLabelText("Category")).toBeInTheDocument();
    });

    it("shows all category options", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Category") as HTMLSelectElement;
      expect(select.querySelector('option[value="setup"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="feature"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="integration"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="styling"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="testing"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="documentation"]')).toBeInTheDocument();
    });

    it("shows current category selected", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Category") as HTMLSelectElement;
      expect(select.value).toBe("feature");
    });

    it("allows changing category", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Category");
      await user.selectOptions(select, "testing");
      expect(select).toHaveValue("testing");
    });
  });

  describe("Steps Editor", () => {
    it("renders steps section with label", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByText("Steps")).toBeInTheDocument();
    });

    it("shows all steps from proposal", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByDisplayValue("Create login form")).toBeInTheDocument();
      expect(screen.getByDisplayValue("Implement JWT validation")).toBeInTheDocument();
      expect(screen.getByDisplayValue("Add logout button")).toBeInTheDocument();
    });

    it("allows editing a step", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const stepInput = screen.getByDisplayValue("Create login form");
      await user.clear(stepInput);
      await user.type(stepInput, "Updated step");
      expect(stepInput).toHaveValue("Updated step");
    });

    it("renders add step button", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByLabelText("Add step")).toBeInTheDocument();
    });

    it("adds new step when add button clicked", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const addButton = screen.getByLabelText("Add step");
      await user.click(addButton);
      const inputs = screen.getAllByTestId("step-input");
      expect(inputs).toHaveLength(4);
    });

    it("renders remove button for each step", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const removeButtons = screen.getAllByLabelText(/Remove step/);
      expect(removeButtons).toHaveLength(3);
    });

    it("removes step when remove button clicked", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const removeButtons = screen.getAllByLabelText(/Remove step/);
      await user.click(removeButtons[0]);
      expect(screen.queryByDisplayValue("Create login form")).not.toBeInTheDocument();
    });

    it("shows empty state when no steps", () => {
      const proposalNoSteps = { ...mockProposal, steps: [] };
      render(<ProposalEditModal {...defaultProps} proposal={proposalNoSteps} />);
      expect(screen.getByText("No steps added")).toBeInTheDocument();
    });
  });

  describe("Acceptance Criteria Editor", () => {
    it("renders acceptance criteria section with label", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByText("Acceptance Criteria")).toBeInTheDocument();
    });

    it("shows all acceptance criteria from proposal", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByDisplayValue("Users can log in")).toBeInTheDocument();
      expect(screen.getByDisplayValue("Users can register")).toBeInTheDocument();
    });

    it("allows editing a criterion", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const criterionInput = screen.getByDisplayValue("Users can log in");
      await user.clear(criterionInput);
      await user.type(criterionInput, "Updated criterion");
      expect(criterionInput).toHaveValue("Updated criterion");
    });

    it("renders add criterion button", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByLabelText("Add criterion")).toBeInTheDocument();
    });

    it("adds new criterion when add button clicked", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const addButton = screen.getByLabelText("Add criterion");
      await user.click(addButton);
      const inputs = screen.getAllByTestId("criterion-input");
      expect(inputs).toHaveLength(3);
    });

    it("removes criterion when remove button clicked", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const removeButtons = screen.getAllByLabelText(/Remove criterion/);
      await user.click(removeButtons[0]);
      expect(screen.queryByDisplayValue("Users can log in")).not.toBeInTheDocument();
    });

    it("shows empty state when no acceptance criteria", () => {
      const proposalNoCriteria = { ...mockProposal, acceptanceCriteria: [] };
      render(<ProposalEditModal {...defaultProps} proposal={proposalNoCriteria} />);
      expect(screen.getByText("No acceptance criteria added")).toBeInTheDocument();
    });
  });

  describe("Priority Override Selector", () => {
    it("renders priority selector with label", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByLabelText("Priority Override")).toBeInTheDocument();
    });

    it("shows all priority options including auto", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Priority Override") as HTMLSelectElement;
      expect(select.querySelector('option[value=""]')).toBeInTheDocument();
      expect(select.querySelector('option[value="critical"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="high"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="medium"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="low"]')).toBeInTheDocument();
    });

    it("shows auto (suggested) option when no user override", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Priority Override") as HTMLSelectElement;
      expect(select.value).toBe("");
    });

    it("shows user priority when set", () => {
      const proposalWithUserPriority = { ...mockProposal, userPriority: "critical" as const };
      render(<ProposalEditModal {...defaultProps} proposal={proposalWithUserPriority} />);
      const select = screen.getByLabelText("Priority Override") as HTMLSelectElement;
      expect(select.value).toBe("critical");
    });

    it("allows changing priority override", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Priority Override");
      await user.selectOptions(select, "critical");
      expect(select).toHaveValue("critical");
    });

    it("shows suggested priority in auto option text", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const autoOption = screen.getByRole("option", { name: /Auto \(high\)/ });
      expect(autoOption).toBeInTheDocument();
    });
  });

  describe("Complexity Selector", () => {
    it("renders complexity selector with label", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByLabelText("Complexity")).toBeInTheDocument();
    });

    it("shows all complexity options", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Complexity") as HTMLSelectElement;
      expect(select.querySelector('option[value="trivial"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="simple"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="moderate"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="complex"]')).toBeInTheDocument();
      expect(select.querySelector('option[value="very_complex"]')).toBeInTheDocument();
    });

    it("shows current complexity selected", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Complexity") as HTMLSelectElement;
      expect(select.value).toBe("moderate");
    });

    it("allows changing complexity", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);
      const select = screen.getByLabelText("Complexity");
      await user.selectOptions(select, "complex");
      expect(select).toHaveValue("complex");
    });
  });

  describe("Save and Cancel Buttons", () => {
    it("renders save button", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();
    });

    it("renders cancel button", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
    });

    it("calls onSave with updated data when save clicked", async () => {
      const onSave = vi.fn();
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} onSave={onSave} />);

      const titleInput = screen.getByLabelText("Title");
      await user.clear(titleInput);
      await user.type(titleInput, "Updated title");

      const saveButton = screen.getByRole("button", { name: "Save" });
      await user.click(saveButton);

      expect(onSave).toHaveBeenCalledTimes(1);
      expect(onSave).toHaveBeenCalledWith(
        "proposal-1",
        expect.objectContaining({
          title: "Updated title",
        })
      );
    });

    it("calls onCancel when cancel clicked", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} onCancel={onCancel} />);

      const cancelButton = screen.getByRole("button", { name: "Cancel" });
      await user.click(cancelButton);

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it("save button is disabled when title is empty", async () => {
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} />);

      const titleInput = screen.getByLabelText("Title");
      await user.clear(titleInput);

      const saveButton = screen.getByRole("button", { name: "Save" });
      expect(saveButton).toBeDisabled();
    });

    it("save button is enabled when title is not empty", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const saveButton = screen.getByRole("button", { name: "Save" });
      expect(saveButton).not.toBeDisabled();
    });

    it("shows loading state when isSaving is true", () => {
      render(<ProposalEditModal {...defaultProps} isSaving={true} />);
      expect(screen.getByRole("button", { name: "Saving..." })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Saving..." })).toBeDisabled();
    });
  });

  describe("Overlay Click Behavior", () => {
    it("calls onCancel when overlay clicked", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} onCancel={onCancel} />);

      const overlay = screen.getByTestId("modal-overlay");
      await user.click(overlay);

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it("does not call onCancel when modal content clicked", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} onCancel={onCancel} />);

      const content = screen.getByTestId("modal-content");
      await user.click(content);

      expect(onCancel).not.toHaveBeenCalled();
    });
  });

  describe("Accessibility", () => {
    it("has accessible name for modal", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const modal = screen.getByRole("dialog");
      expect(modal).toHaveAccessibleName("Edit Proposal");
    });

    it("focuses title input when modal opens", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const titleInput = screen.getByLabelText("Title");
      expect(titleInput).toHaveFocus();
    });

    it("has proper input labels", () => {
      render(<ProposalEditModal {...defaultProps} />);
      expect(screen.getByLabelText("Title")).toBeInTheDocument();
      expect(screen.getByLabelText("Description")).toBeInTheDocument();
      expect(screen.getByLabelText("Category")).toBeInTheDocument();
      expect(screen.getByLabelText("Priority Override")).toBeInTheDocument();
      expect(screen.getByLabelText("Complexity")).toBeInTheDocument();
    });

    it("step inputs have proper aria-labels", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const stepInputs = screen.getAllByTestId("step-input");
      stepInputs.forEach((input, index) => {
        expect(input).toHaveAttribute("aria-label", `Step ${index + 1}`);
      });
    });

    it("criterion inputs have proper aria-labels", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const criterionInputs = screen.getAllByTestId("criterion-input");
      criterionInputs.forEach((input, index) => {
        expect(input).toHaveAttribute("aria-label", `Acceptance criterion ${index + 1}`);
      });
    });
  });

  describe("Styling", () => {
    it("uses correct modal overlay styling", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const overlay = screen.getByTestId("modal-overlay");
      expect(overlay).toHaveStyle({ backgroundColor: "rgba(0, 0, 0, 0.5)" });
    });

    it("modal is centered with fixed positioning", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const modal = screen.getByTestId("proposal-edit-modal");
      expect(modal).toHaveClass("fixed", "inset-0", "z-50");
    });

    it("content has elevated background", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const content = screen.getByTestId("modal-content");
      expect(content).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("save button uses accent color when enabled", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const saveButton = screen.getByRole("button", { name: "Save" });
      expect(saveButton).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });

    it("anti-ai-slop: no purple gradients in styling", () => {
      render(<ProposalEditModal {...defaultProps} />);
      const modal = screen.getByTestId("proposal-edit-modal");
      const styles = window.getComputedStyle(modal);
      expect(styles.background).not.toMatch(/purple|#800080|#a855f7/i);
    });
  });

  describe("Form Data Handling", () => {
    it("includes all fields in save callback", async () => {
      const onSave = vi.fn();
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} onSave={onSave} />);

      const saveButton = screen.getByRole("button", { name: "Save" });
      await user.click(saveButton);

      expect(onSave).toHaveBeenCalledWith(
        "proposal-1",
        expect.objectContaining({
          title: "Implement user authentication",
          description: "Add login and registration functionality",
          category: "feature",
          steps: ["Create login form", "Implement JWT validation", "Add logout button"],
          acceptanceCriteria: ["Users can log in", "Users can register"],
          userPriority: undefined,
          complexity: "moderate",
        })
      );
    });

    it("filters out empty steps", async () => {
      const onSave = vi.fn();
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} onSave={onSave} />);

      const addButton = screen.getByLabelText("Add step");
      await user.click(addButton);

      const saveButton = screen.getByRole("button", { name: "Save" });
      await user.click(saveButton);

      expect(onSave).toHaveBeenCalledWith(
        "proposal-1",
        expect.objectContaining({
          steps: ["Create login form", "Implement JWT validation", "Add logout button"],
        })
      );
    });

    it("filters out empty acceptance criteria", async () => {
      const onSave = vi.fn();
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} onSave={onSave} />);

      const addButton = screen.getByLabelText("Add criterion");
      await user.click(addButton);

      const saveButton = screen.getByRole("button", { name: "Save" });
      await user.click(saveButton);

      expect(onSave).toHaveBeenCalledWith(
        "proposal-1",
        expect.objectContaining({
          acceptanceCriteria: ["Users can log in", "Users can register"],
        })
      );
    });

    it("converts empty priority override to undefined", async () => {
      const onSave = vi.fn();
      const user = userEvent.setup();
      const proposalWithPriority = { ...mockProposal, userPriority: "critical" as const };
      render(<ProposalEditModal {...defaultProps} proposal={proposalWithPriority} onSave={onSave} />);

      const prioritySelect = screen.getByLabelText("Priority Override");
      await user.selectOptions(prioritySelect, "");

      const saveButton = screen.getByRole("button", { name: "Save" });
      await user.click(saveButton);

      expect(onSave).toHaveBeenCalledWith(
        "proposal-1",
        expect.objectContaining({
          userPriority: undefined,
        })
      );
    });
  });

  describe("Keyboard Navigation", () => {
    it("closes modal on Escape key", async () => {
      const onCancel = vi.fn();
      const user = userEvent.setup();
      render(<ProposalEditModal {...defaultProps} onCancel={onCancel} />);

      await user.keyboard("{Escape}");

      expect(onCancel).toHaveBeenCalledTimes(1);
    });
  });
});
