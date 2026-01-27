/**
 * Tests for TaskCreationForm component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TaskCreationForm } from "./TaskCreationForm";

// Mock the useTaskMutation hook
const mockMutate = vi.fn();
const mockReset = vi.fn();

vi.mock("@/hooks/useTaskMutation", () => ({
  useTaskMutation: () => ({
    createMutation: {
      mutate: mockMutate,
      isPending: false,
      isError: false,
      error: null,
      reset: mockReset,
    },
  }),
}));

describe("TaskCreationForm", () => {
  const defaultProps = {
    projectId: "project-123",
    onSuccess: vi.fn(),
    onCancel: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Rendering", () => {
    it("should render the form with all required fields", () => {
      render(<TaskCreationForm {...defaultProps} />);

      expect(screen.getByLabelText(/title/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/category/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/priority/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/description/i)).toBeInTheDocument();
    });

    it("should render submit and cancel buttons", () => {
      render(<TaskCreationForm {...defaultProps} />);

      expect(screen.getByRole("button", { name: /create task/i })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /cancel/i })).toBeInTheDocument();
    });

    it("should pre-fill title when defaultTitle is provided", () => {
      render(<TaskCreationForm {...defaultProps} defaultTitle="Pre-filled Title" />);

      expect(screen.getByLabelText(/title/i)).toHaveValue("Pre-filled Title");
    });
  });

  describe("Form Validation", () => {
    it("should not submit when title is empty", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const submitButton = screen.getByRole("button", { name: /create task/i });
      await userEvent.click(submitButton);

      expect(mockMutate).not.toHaveBeenCalled();
    });

    it("should submit form with valid data", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const titleInput = screen.getByLabelText(/title/i);
      await userEvent.type(titleInput, "Test Task");

      const submitButton = screen.getByRole("button", { name: /create task/i });
      await userEvent.click(submitButton);

      expect(mockMutate).toHaveBeenCalledWith(
        expect.objectContaining({
          projectId: "project-123",
          title: "Test Task",
          category: "feature", // default
          priority: 3, // default (P3 - Medium)
        }),
        expect.anything()
      );
    });
  });

  describe("Category Selection", () => {
    it("should default to feature category", () => {
      render(<TaskCreationForm {...defaultProps} />);

      const categorySelect = screen.getByLabelText(/category/i);
      expect(categorySelect).toHaveValue("feature");
    });

    it("should allow changing category", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const categorySelect = screen.getByLabelText(/category/i);
      await userEvent.selectOptions(categorySelect, "bug");

      expect(categorySelect).toHaveValue("bug");
    });

    it("should submit with selected category", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const titleInput = screen.getByLabelText(/title/i);
      await userEvent.type(titleInput, "Bug Fix");

      const categorySelect = screen.getByLabelText(/category/i);
      await userEvent.selectOptions(categorySelect, "bug");

      const submitButton = screen.getByRole("button", { name: /create task/i });
      await userEvent.click(submitButton);

      expect(mockMutate).toHaveBeenCalledWith(
        expect.objectContaining({
          category: "bug",
        }),
        expect.anything()
      );
    });
  });

  describe("Priority Selection", () => {
    it("should default to P3 (Medium) priority", () => {
      render(<TaskCreationForm {...defaultProps} />);

      const prioritySelect = screen.getByLabelText(/priority/i);
      expect(prioritySelect).toHaveValue("3");
    });

    it("should allow changing priority", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const prioritySelect = screen.getByLabelText(/priority/i);
      await userEvent.selectOptions(prioritySelect, "1");

      expect(prioritySelect).toHaveValue("1");
    });

    it("should submit with selected priority", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const titleInput = screen.getByLabelText(/title/i);
      await userEvent.type(titleInput, "Critical Task");

      const prioritySelect = screen.getByLabelText(/priority/i);
      await userEvent.selectOptions(prioritySelect, "1");

      const submitButton = screen.getByRole("button", { name: /create task/i });
      await userEvent.click(submitButton);

      expect(mockMutate).toHaveBeenCalledWith(
        expect.objectContaining({
          priority: 1,
        }),
        expect.anything()
      );
    });
  });

  describe("Description Field", () => {
    it("should allow entering description", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const descriptionInput = screen.getByLabelText(/description/i);
      await userEvent.type(descriptionInput, "This is a test description");

      expect(descriptionInput).toHaveValue("This is a test description");
    });

    it("should submit with description", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const titleInput = screen.getByLabelText(/title/i);
      await userEvent.type(titleInput, "Task with Description");

      const descriptionInput = screen.getByLabelText(/description/i);
      await userEvent.type(descriptionInput, "Detailed description here");

      const submitButton = screen.getByRole("button", { name: /create task/i });
      await userEvent.click(submitButton);

      expect(mockMutate).toHaveBeenCalledWith(
        expect.objectContaining({
          description: "Detailed description here",
        }),
        expect.anything()
      );
    });

    it("should submit without description when empty", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const titleInput = screen.getByLabelText(/title/i);
      await userEvent.type(titleInput, "Task without Description");

      const submitButton = screen.getByRole("button", { name: /create task/i });
      await userEvent.click(submitButton);

      const callArgs = mockMutate.mock.calls[0]?.[0];
      expect(callArgs?.description).toBeUndefined();
    });
  });

  describe("Cancel Button", () => {
    it("should call onCancel when cancel button clicked", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const cancelButton = screen.getByRole("button", { name: /cancel/i });
      await userEvent.click(cancelButton);

      expect(defaultProps.onCancel).toHaveBeenCalled();
    });

    it("should not submit form when cancel clicked", async () => {
      render(<TaskCreationForm {...defaultProps} />);

      const cancelButton = screen.getByRole("button", { name: /cancel/i });
      await userEvent.click(cancelButton);

      expect(mockMutate).not.toHaveBeenCalled();
    });
  });

  describe("Form Reset", () => {
    it("should call onSuccess after successful submission", async () => {
      mockMutate.mockImplementation((_, { onSuccess }) => {
        onSuccess?.();
      });

      render(<TaskCreationForm {...defaultProps} />);

      const titleInput = screen.getByLabelText(/title/i);
      await userEvent.type(titleInput, "Test Task");

      const submitButton = screen.getByRole("button", { name: /create task/i });
      await userEvent.click(submitButton);

      expect(defaultProps.onSuccess).toHaveBeenCalled();
    });
  });

  describe("Accessibility", () => {
    it("should have proper form labels", () => {
      render(<TaskCreationForm {...defaultProps} />);

      // All inputs should be labeled
      expect(screen.getByLabelText(/title/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/category/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/priority/i)).toBeInTheDocument();
      expect(screen.getByLabelText(/description/i)).toBeInTheDocument();
    });
  });
});
