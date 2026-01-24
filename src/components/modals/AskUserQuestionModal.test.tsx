/**
 * AskUserQuestionModal component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { AskUserQuestionModal } from "./AskUserQuestionModal";
import type { AskUserQuestionPayload } from "@/types/ask-user-question";

const mockSingleSelectQuestion: AskUserQuestionPayload = {
  taskId: "task-123",
  header: "Authentication Method",
  question: "Which authentication method should we use?",
  options: [
    { label: "JWT tokens", description: "Recommended for APIs" },
    { label: "Session cookies", description: "Traditional web sessions" },
    { label: "OAuth only", description: "Third-party auth providers" },
  ],
  multiSelect: false,
};

const mockMultiSelectQuestion: AskUserQuestionPayload = {
  taskId: "task-456",
  header: "Features",
  question: "Which features do you want to enable?",
  options: [
    { label: "Dark mode", description: "Enable dark theme support" },
    { label: "Analytics", description: "Track user behavior" },
  ],
  multiSelect: true,
};

describe("AskUserQuestionModal", () => {
  const mockSubmitAnswer = vi.fn();
  const mockClearQuestion = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("basic rendering", () => {
    it("renders nothing when no question is provided", () => {
      render(
        <AskUserQuestionModal
          question={null}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.queryByTestId("ask-user-question-modal")).not.toBeInTheDocument();
    });

    it("renders modal when question is provided", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByTestId("ask-user-question-modal")).toBeInTheDocument();
    });

    it("renders question header", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByTestId("question-header")).toHaveTextContent("Authentication Method");
    });

    it("renders question text", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByTestId("question-text")).toHaveTextContent(
        "Which authentication method should we use?"
      );
    });
  });

  describe("single select options", () => {
    it("renders options as radio buttons for single select", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const radios = screen.getAllByRole("radio");
      // 3 options + 1 "Other" option
      expect(radios).toHaveLength(4);
    });

    it("renders option labels", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByText("JWT tokens")).toBeInTheDocument();
      expect(screen.getByText("Session cookies")).toBeInTheDocument();
      expect(screen.getByText("OAuth only")).toBeInTheDocument();
    });

    it("renders option descriptions", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByText("Recommended for APIs")).toBeInTheDocument();
      expect(screen.getByText("Traditional web sessions")).toBeInTheDocument();
    });

    it("allows selecting a single option", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const jwtRadio = screen.getByRole("radio", { name: /JWT tokens/i });
      fireEvent.click(jwtRadio);
      expect(jwtRadio).toBeChecked();
    });

    it("deselects previous option when new option selected", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const jwtRadio = screen.getByRole("radio", { name: /JWT tokens/i });
      const sessionRadio = screen.getByRole("radio", { name: /Session cookies/i });
      fireEvent.click(jwtRadio);
      fireEvent.click(sessionRadio);
      expect(jwtRadio).not.toBeChecked();
      expect(sessionRadio).toBeChecked();
    });
  });

  describe("multi-select options", () => {
    it("renders options as checkboxes for multi-select", () => {
      render(
        <AskUserQuestionModal
          question={mockMultiSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const checkboxes = screen.getAllByRole("checkbox");
      // 2 options + 1 "Other" option
      expect(checkboxes).toHaveLength(3);
    });

    it("allows selecting multiple options", () => {
      render(
        <AskUserQuestionModal
          question={mockMultiSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const darkModeCheckbox = screen.getByRole("checkbox", { name: /Dark mode/i });
      const analyticsCheckbox = screen.getByRole("checkbox", { name: /Analytics/i });
      fireEvent.click(darkModeCheckbox);
      fireEvent.click(analyticsCheckbox);
      expect(darkModeCheckbox).toBeChecked();
      expect(analyticsCheckbox).toBeChecked();
    });

    it("allows toggling checkboxes", () => {
      render(
        <AskUserQuestionModal
          question={mockMultiSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const darkModeCheckbox = screen.getByRole("checkbox", { name: /Dark mode/i });
      fireEvent.click(darkModeCheckbox);
      expect(darkModeCheckbox).toBeChecked();
      fireEvent.click(darkModeCheckbox);
      expect(darkModeCheckbox).not.toBeChecked();
    });
  });

  describe("Other option", () => {
    it("renders Other option for single select", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByRole("radio", { name: /Other/i })).toBeInTheDocument();
    });

    it("renders Other option for multi-select", () => {
      render(
        <AskUserQuestionModal
          question={mockMultiSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByRole("checkbox", { name: /Other/i })).toBeInTheDocument();
    });

    it("shows text input when Other is selected", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const otherRadio = screen.getByRole("radio", { name: /Other/i });
      fireEvent.click(otherRadio);
      expect(screen.getByTestId("other-input")).toBeInTheDocument();
    });

    it("hides text input when Other is not selected", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.queryByTestId("other-input")).not.toBeInTheDocument();
    });

    it("allows typing in Other text input", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const otherRadio = screen.getByRole("radio", { name: /Other/i });
      fireEvent.click(otherRadio);
      const textInput = screen.getByTestId("other-input");
      fireEvent.change(textInput, { target: { value: "Custom auth method" } });
      expect(textInput).toHaveValue("Custom auth method");
    });
  });

  describe("submit behavior", () => {
    it("calls onSubmit with selected option for single select", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const jwtRadio = screen.getByRole("radio", { name: /JWT tokens/i });
      fireEvent.click(jwtRadio);
      fireEvent.click(screen.getByRole("button", { name: /submit/i }));
      expect(mockSubmitAnswer).toHaveBeenCalledWith({
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      });
    });

    it("calls onSubmit with multiple selected options for multi-select", () => {
      render(
        <AskUserQuestionModal
          question={mockMultiSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      fireEvent.click(screen.getByRole("checkbox", { name: /Dark mode/i }));
      fireEvent.click(screen.getByRole("checkbox", { name: /Analytics/i }));
      fireEvent.click(screen.getByRole("button", { name: /submit/i }));
      expect(mockSubmitAnswer).toHaveBeenCalledWith({
        taskId: "task-456",
        selectedOptions: ["Dark mode", "Analytics"],
      });
    });

    it("calls onSubmit with custom response when Other is selected", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const otherRadio = screen.getByRole("radio", { name: /Other/i });
      fireEvent.click(otherRadio);
      const textInput = screen.getByTestId("other-input");
      fireEvent.change(textInput, { target: { value: "Custom auth method" } });
      fireEvent.click(screen.getByRole("button", { name: /submit/i }));
      expect(mockSubmitAnswer).toHaveBeenCalledWith({
        taskId: "task-123",
        selectedOptions: [],
        customResponse: "Custom auth method",
      });
    });

    it("disables submit button when no option is selected", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByRole("button", { name: /submit/i })).toBeDisabled();
    });

    it("disables submit when Other is selected but input is empty", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const otherRadio = screen.getByRole("radio", { name: /Other/i });
      fireEvent.click(otherRadio);
      expect(screen.getByRole("button", { name: /submit/i })).toBeDisabled();
    });

    it("enables submit when Other is selected and input has text", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const otherRadio = screen.getByRole("radio", { name: /Other/i });
      fireEvent.click(otherRadio);
      fireEvent.change(screen.getByTestId("other-input"), {
        target: { value: "Custom value" },
      });
      expect(screen.getByRole("button", { name: /submit/i })).not.toBeDisabled();
    });
  });

  describe("loading state", () => {
    it("disables submit button when loading", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={true}
        />
      );
      const jwtRadio = screen.getByRole("radio", { name: /JWT tokens/i });
      fireEvent.click(jwtRadio);
      expect(screen.getByRole("button", { name: /submit/i })).toBeDisabled();
    });

    it("shows loading text on submit button when loading", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={true}
        />
      );
      expect(screen.getByRole("button", { name: /submitting/i })).toBeInTheDocument();
    });

    it("disables options when loading", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={true}
        />
      );
      const radios = screen.getAllByRole("radio");
      radios.forEach((radio) => {
        expect(radio).toBeDisabled();
      });
    });
  });

  describe("close/cancel behavior", () => {
    it("calls onClose when clicking overlay", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      fireEvent.click(screen.getByTestId("modal-overlay"));
      expect(mockClearQuestion).toHaveBeenCalled();
    });

    it("does not close when clicking modal content", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      fireEvent.click(screen.getByTestId("modal-content"));
      expect(mockClearQuestion).not.toHaveBeenCalled();
    });
  });

  describe("data attributes", () => {
    it("sets data-testid on modal container", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByTestId("ask-user-question-modal")).toBeInTheDocument();
    });

    it("sets data-task-id attribute", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByTestId("ask-user-question-modal")).toHaveAttribute(
        "data-task-id",
        "task-123"
      );
    });

    it("sets data-multi-select attribute", () => {
      render(
        <AskUserQuestionModal
          question={mockMultiSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByTestId("ask-user-question-modal")).toHaveAttribute(
        "data-multi-select",
        "true"
      );
    });
  });

  describe("styling", () => {
    it("applies design system background color to modal", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const modal = screen.getByTestId("modal-content");
      expect(modal).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("applies design system text colors", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const header = screen.getByTestId("question-header");
      expect(header).toHaveStyle({ color: "var(--text-primary)" });
    });
  });

  describe("accessibility", () => {
    it("options have accessible labels", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      expect(screen.getByRole("radio", { name: /JWT tokens/i })).toBeInTheDocument();
      expect(screen.getByRole("radio", { name: /Session cookies/i })).toBeInTheDocument();
    });

    it("input fields have proper labels", () => {
      render(
        <AskUserQuestionModal
          question={mockSingleSelectQuestion}
          onSubmit={mockSubmitAnswer}
          onClose={mockClearQuestion}
          isLoading={false}
        />
      );
      const otherRadio = screen.getByRole("radio", { name: /Other/i });
      fireEvent.click(otherRadio);
      expect(screen.getByTestId("other-input")).toHaveAttribute("placeholder", "Enter your response...");
    });
  });
});
