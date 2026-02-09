/**
 * QuestionInputBanner component tests
 * Tests rendering, chip interaction, dismiss handlers, and dimming logic
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QuestionInputBanner } from "./QuestionInputBanner";
import type { AskUserQuestionPayload } from "@/types/ask-user-question";

// ============================================================================
// Test Data
// ============================================================================

const singleSelectQuestion: AskUserQuestionPayload = {
  requestId: "req-1",
  question: "Which framework should we use?",
  header: "Architecture Decision",
  options: [
    { label: "React" },
    { label: "Vue", value: "vue" },
    { label: "Svelte", value: "svelte" },
  ],
  multiSelect: false,
};

const multiSelectQuestion: AskUserQuestionPayload = {
  requestId: "req-2",
  question: "Select the features to include:",
  header: "Feature Selection",
  options: [
    { label: "Auth", value: "auth" },
    { label: "Logging", value: "logging" },
    { label: "Caching", value: "caching" },
  ],
  multiSelect: true,
};

const noHeaderQuestion: AskUserQuestionPayload = {
  requestId: "req-3",
  question: "Continue with this approach?",
  options: [
    { label: "Yes", value: "yes" },
    { label: "No", value: "no" },
  ],
  multiSelect: false,
};

const defaultProps = {
  question: singleSelectQuestion,
  selectedIndices: new Set<number>(),
  onChipClick: vi.fn(),
  onDismiss: vi.fn(),
};

// ============================================================================
// Tests
// ============================================================================

describe("QuestionInputBanner", () => {
  describe("Active Question State", () => {
    it("renders the question header", () => {
      render(<QuestionInputBanner {...defaultProps} />);

      expect(screen.getByText("Architecture Decision")).toBeInTheDocument();
    });

    it("renders fallback header when header is null", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          question={noHeaderQuestion}
        />
      );

      expect(screen.getByText("Question from agent")).toBeInTheDocument();
    });

    it("renders the question text", () => {
      render(<QuestionInputBanner {...defaultProps} />);

      expect(
        screen.getByText("Which framework should we use?")
      ).toBeInTheDocument();
    });

    it("renders all option chips", () => {
      render(<QuestionInputBanner {...defaultProps} />);

      expect(screen.getByText("React")).toBeInTheDocument();
      expect(screen.getByText("Vue")).toBeInTheDocument();
      expect(screen.getByText("Svelte")).toBeInTheDocument();
    });

    it("renders numbered chips starting at 1", () => {
      render(<QuestionInputBanner {...defaultProps} />);

      expect(screen.getByText("1")).toBeInTheDocument();
      expect(screen.getByText("2")).toBeInTheDocument();
      expect(screen.getByText("3")).toBeInTheDocument();
    });

    it("renders the dismiss button", () => {
      render(<QuestionInputBanner {...defaultProps} />);

      expect(
        screen.getByRole("button", { name: "Dismiss question" })
      ).toBeInTheDocument();
    });

    it("renders the data-testid on the outer wrapper", () => {
      render(<QuestionInputBanner {...defaultProps} />);

      expect(screen.getByTestId("question-input-banner")).toBeInTheDocument();
    });
  });

  describe("Answered/Collapsed State", () => {
    it("renders answered state when answeredValue is set", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          answeredValue="React"
        />
      );

      expect(
        screen.getByTestId("question-input-banner-answered")
      ).toBeInTheDocument();
    });

    it("shows the answered value text", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          answeredValue="React"
        />
      );

      expect(screen.getByText("React")).toBeInTheDocument();
      expect(screen.getByText("Answered:")).toBeInTheDocument();
    });

    it("does not render question text or chips in answered state", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          answeredValue="React"
        />
      );

      expect(
        screen.queryByText("Which framework should we use?")
      ).not.toBeInTheDocument();
      expect(screen.queryByText("Vue")).not.toBeInTheDocument();
    });

    it("renders dismiss button in answered state when onDismissAnswered provided", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          answeredValue="React"
          onDismissAnswered={vi.fn()}
        />
      );

      expect(
        screen.getByRole("button", { name: "Dismiss answered summary" })
      ).toBeInTheDocument();
    });

    it("does not render dismiss button in answered state when onDismissAnswered not provided", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          answeredValue="React"
        />
      );

      expect(
        screen.queryByRole("button", { name: "Dismiss answered summary" })
      ).not.toBeInTheDocument();
    });
  });

  describe("Chip Click Handler", () => {
    it("calls onChipClick with correct index when chip is clicked", async () => {
      const user = userEvent.setup();
      const onChipClick = vi.fn();

      render(
        <QuestionInputBanner
          {...defaultProps}
          onChipClick={onChipClick}
        />
      );

      await user.click(screen.getByText("Vue"));
      expect(onChipClick).toHaveBeenCalledWith(1);
    });

    it("calls onChipClick with index 0 for first chip", async () => {
      const user = userEvent.setup();
      const onChipClick = vi.fn();

      render(
        <QuestionInputBanner
          {...defaultProps}
          onChipClick={onChipClick}
        />
      );

      await user.click(screen.getByText("React"));
      expect(onChipClick).toHaveBeenCalledWith(0);
    });

    it("calls onChipClick with last index for last chip", async () => {
      const user = userEvent.setup();
      const onChipClick = vi.fn();

      render(
        <QuestionInputBanner
          {...defaultProps}
          onChipClick={onChipClick}
        />
      );

      await user.click(screen.getByText("Svelte"));
      expect(onChipClick).toHaveBeenCalledWith(2);
    });
  });

  describe("Dismiss Handlers", () => {
    it("calls onDismiss after animation delay when dismiss button is clicked", async () => {
      const user = userEvent.setup();
      const onDismiss = vi.fn();

      render(
        <QuestionInputBanner
          {...defaultProps}
          onDismiss={onDismiss}
        />
      );

      await user.click(
        screen.getByRole("button", { name: "Dismiss question" })
      );

      // onDismiss is called after 350ms animation delay via setTimeout
      await vi.waitFor(() => {
        expect(onDismiss).toHaveBeenCalledTimes(1);
      }, { timeout: 500 });
    });

    it("calls onDismissAnswered after animation delay when answered dismiss is clicked", async () => {
      const user = userEvent.setup();
      const onDismissAnswered = vi.fn();

      render(
        <QuestionInputBanner
          {...defaultProps}
          answeredValue="React"
          onDismissAnswered={onDismissAnswered}
        />
      );

      await user.click(
        screen.getByRole("button", { name: "Dismiss answered summary" })
      );

      await vi.waitFor(() => {
        expect(onDismissAnswered).toHaveBeenCalledTimes(1);
      }, { timeout: 500 });
    });
  });

  describe("Single-Select Dimming Logic", () => {
    it("does not dim chips when nothing is selected", () => {
      render(<QuestionInputBanner {...defaultProps} />);

      const buttons = screen.getAllByRole("button").filter(
        (b) => !b.getAttribute("aria-label")
      );

      // All chips should have opacity 1 (not dimmed)
      for (const button of buttons) {
        expect(button.style.opacity).toBe("1");
      }
    });

    it("dims unselected chips when one is selected in single-select mode", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          selectedIndices={new Set([1])}
        />
      );

      const buttons = screen.getAllByRole("button").filter(
        (b) => !b.getAttribute("aria-label")
      );

      // First chip (React, index 0) - dimmed
      expect(buttons[0].style.opacity).toBe("0.45");
      // Second chip (Vue, index 1) - selected, not dimmed
      expect(buttons[1].style.opacity).toBe("1");
      // Third chip (Svelte, index 2) - dimmed
      expect(buttons[2].style.opacity).toBe("0.45");
    });

    it("does not dim chips in multi-select mode even with selections", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          question={multiSelectQuestion}
          selectedIndices={new Set([0])}
        />
      );

      const buttons = screen.getAllByRole("button").filter(
        (b) => !b.getAttribute("aria-label")
      );

      // In multi-select, no chips should be dimmed
      for (const button of buttons) {
        expect(button.style.opacity).toBe("1");
      }
    });
  });

  describe("Multi-Select Checkmarks", () => {
    it("renders checkmark icons in multi-select mode", () => {
      render(
        <QuestionInputBanner
          {...defaultProps}
          question={multiSelectQuestion}
        />
      );

      // Check SVG elements exist (lucide Check icons)
      const banner = screen.getByTestId("question-input-banner");
      const svgs = banner.querySelectorAll("svg");
      // Should have checkmarks for each option (3 options in multiSelectQuestion)
      // plus the dismiss X button
      expect(svgs.length).toBeGreaterThanOrEqual(3);
    });

    it("does not render checkmark icons in single-select mode", () => {
      render(<QuestionInputBanner {...defaultProps} />);

      const banner = screen.getByTestId("question-input-banner");
      // In single-select, only the dismiss X button should have an SVG
      const svgs = banner.querySelectorAll("svg");
      expect(svgs.length).toBe(1); // Only the X dismiss button
    });
  });
});
