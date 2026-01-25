/**
 * ReviewNotesModal component tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ReviewNotesModal } from "./ReviewNotesModal";

describe("ReviewNotesModal", () => {
  describe("basic rendering", () => {
    it("renders modal when open", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      expect(screen.getByTestId("review-notes-modal")).toBeInTheDocument();
    });

    it("does not render modal when closed", () => {
      render(
        <ReviewNotesModal
          isOpen={false}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      expect(screen.queryByTestId("review-notes-modal")).not.toBeInTheDocument();
    });

    it("renders title", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Request Changes"
        />
      );
      expect(screen.getByTestId("modal-title")).toHaveTextContent("Request Changes");
    });

    it("renders notes textarea", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      expect(screen.getByTestId("notes-textarea")).toBeInTheDocument();
    });
  });

  describe("fix description field", () => {
    it("shows fix description field when showFixDescription is true", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Request Changes"
          showFixDescription={true}
        />
      );
      expect(screen.getByTestId("fix-description-textarea")).toBeInTheDocument();
    });

    it("hides fix description field when showFixDescription is false", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
          showFixDescription={false}
        />
      );
      expect(screen.queryByTestId("fix-description-textarea")).not.toBeInTheDocument();
    });

    it("hides fix description field by default", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      expect(screen.queryByTestId("fix-description-textarea")).not.toBeInTheDocument();
    });
  });

  describe("form interaction", () => {
    it("allows typing in notes textarea", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      const textarea = screen.getByTestId("notes-textarea");
      fireEvent.change(textarea, { target: { value: "Good work!" } });
      expect(textarea).toHaveValue("Good work!");
    });

    it("allows typing in fix description textarea", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Request Changes"
          showFixDescription={true}
        />
      );
      const textarea = screen.getByTestId("fix-description-textarea");
      fireEvent.change(textarea, { target: { value: "Fix the login validation" } });
      expect(textarea).toHaveValue("Fix the login validation");
    });
  });

  describe("submit behavior", () => {
    it("calls onSubmit with notes when Submit clicked", () => {
      const onSubmit = vi.fn();
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={onSubmit}
          title="Add Review Notes"
        />
      );
      const textarea = screen.getByTestId("notes-textarea");
      fireEvent.change(textarea, { target: { value: "Looks good" } });
      fireEvent.click(screen.getByRole("button", { name: /submit/i }));
      expect(onSubmit).toHaveBeenCalledWith({ notes: "Looks good", fixDescription: undefined });
    });

    it("calls onSubmit with notes and fixDescription when both provided", () => {
      const onSubmit = vi.fn();
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={onSubmit}
          title="Request Changes"
          showFixDescription={true}
        />
      );
      const notesTextarea = screen.getByTestId("notes-textarea");
      const fixTextarea = screen.getByTestId("fix-description-textarea");
      fireEvent.change(notesTextarea, { target: { value: "Issues found" } });
      fireEvent.change(fixTextarea, { target: { value: "Fix validation logic" } });
      fireEvent.click(screen.getByRole("button", { name: /submit/i }));
      expect(onSubmit).toHaveBeenCalledWith({ notes: "Issues found", fixDescription: "Fix validation logic" });
    });

    it("clears form after submit", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      const textarea = screen.getByTestId("notes-textarea");
      fireEvent.change(textarea, { target: { value: "Test notes" } });
      fireEvent.click(screen.getByRole("button", { name: /submit/i }));
      expect(textarea).toHaveValue("");
    });
  });

  describe("cancel behavior", () => {
    it("calls onClose when Cancel clicked", () => {
      const onClose = vi.fn();
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={onClose}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      fireEvent.click(screen.getByRole("button", { name: /cancel/i }));
      expect(onClose).toHaveBeenCalled();
    });

    it("clears form when Cancel clicked", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      const textarea = screen.getByTestId("notes-textarea");
      fireEvent.change(textarea, { target: { value: "Unsaved notes" } });
      fireEvent.click(screen.getByRole("button", { name: /cancel/i }));
      // After closing and reopening, form should be cleared
      expect(textarea).toHaveValue("");
    });
  });

  describe("labels", () => {
    it("renders notes label", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      expect(screen.getByText("Notes")).toBeInTheDocument();
    });

    it("renders fix description label when shown", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Request Changes"
          showFixDescription={true}
        />
      );
      expect(screen.getByText("Fix Description")).toBeInTheDocument();
    });

    it("uses custom notes label when provided", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
          notesLabel="Review Feedback"
        />
      );
      expect(screen.getByText("Review Feedback")).toBeInTheDocument();
    });
  });

  describe("placeholder text", () => {
    it("shows placeholder in notes textarea", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      const textarea = screen.getByTestId("notes-textarea");
      expect(textarea).toHaveAttribute("placeholder", "Enter your review notes...");
    });

    it("shows custom placeholder when provided", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
          notesPlaceholder="Provide feedback..."
        />
      );
      const textarea = screen.getByTestId("notes-textarea");
      expect(textarea).toHaveAttribute("placeholder", "Provide feedback...");
    });
  });

  describe("data attributes", () => {
    it("sets data-testid on modal container", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      expect(screen.getByTestId("review-notes-modal")).toBeInTheDocument();
    });

    it("sets data-has-fix-description attribute", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Request Changes"
          showFixDescription={true}
        />
      );
      expect(screen.getByTestId("review-notes-modal")).toHaveAttribute("data-has-fix-description", "true");
    });
  });

  describe("styling", () => {
    it("uses shadcn Dialog with correct max-width", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      const modal = screen.getByTestId("review-notes-modal");
      expect(modal).toHaveClass("max-w-md");
    });

    it("renders modal overlay with blur effect", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
        />
      );
      const overlay = screen.getByTestId("modal-overlay");
      expect(overlay).toHaveClass("backdrop-blur-[8px]");
    });
  });

  describe("submit button state", () => {
    it("disables submit when notes are empty and required", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
          notesRequired={true}
        />
      );
      expect(screen.getByRole("button", { name: /submit/i })).toBeDisabled();
    });

    it("enables submit when notes are provided and required", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
          notesRequired={true}
        />
      );
      const textarea = screen.getByTestId("notes-textarea");
      fireEvent.change(textarea, { target: { value: "Some notes" } });
      expect(screen.getByRole("button", { name: /submit/i })).not.toBeDisabled();
    });

    it("enables submit with empty notes when not required", () => {
      render(
        <ReviewNotesModal
          isOpen={true}
          onClose={vi.fn()}
          onSubmit={vi.fn()}
          title="Add Review Notes"
          notesRequired={false}
        />
      );
      expect(screen.getByRole("button", { name: /submit/i })).not.toBeDisabled();
    });
  });
});
