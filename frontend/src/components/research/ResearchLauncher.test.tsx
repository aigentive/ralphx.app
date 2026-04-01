/**
 * ResearchLauncher component tests
 *
 * Tests for:
 * - Form fields (question, context, scope, constraints)
 * - Depth preset selector
 * - Custom depth option
 * - Form submission
 * - Validation
 * - Loading state
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ResearchLauncher } from "./ResearchLauncher";

describe("ResearchLauncher", () => {
  const defaultProps = {
    onLaunch: vi.fn(),
    onCancel: vi.fn(),
    isLaunching: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("research-launcher")).toBeInTheDocument();
    });

    it("renders question input", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("question-input")).toBeInTheDocument();
    });

    it("renders context input", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("context-input")).toBeInTheDocument();
    });

    it("renders scope input", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("scope-input")).toBeInTheDocument();
    });

    it("renders depth preset selector", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("depth-preset-selector")).toBeInTheDocument();
    });

    it("renders all depth presets", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("preset-quick-scan")).toBeInTheDocument();
      expect(screen.getByTestId("preset-standard")).toBeInTheDocument();
      expect(screen.getByTestId("preset-deep-dive")).toBeInTheDocument();
      expect(screen.getByTestId("preset-exhaustive")).toBeInTheDocument();
    });

    it("renders launch and cancel buttons", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("launch-button")).toBeInTheDocument();
      expect(screen.getByTestId("cancel-button")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Depth Preset Selection
  // ==========================================================================

  describe("depth preset selection", () => {
    it("selects standard preset by default", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("preset-standard")).toHaveAttribute("data-selected", "true");
    });

    it("selects preset when clicked", async () => {
      const user = userEvent.setup();
      render(<ResearchLauncher {...defaultProps} />);

      await user.click(screen.getByTestId("preset-deep-dive"));
      expect(screen.getByTestId("preset-deep-dive")).toHaveAttribute("data-selected", "true");
      expect(screen.getByTestId("preset-standard")).toHaveAttribute("data-selected", "false");
    });

    it("shows preset description", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByText(/50 iterations/i)).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Custom Depth
  // ==========================================================================

  describe("custom depth", () => {
    it("shows custom depth option", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("preset-custom")).toBeInTheDocument();
    });

    it("shows custom depth inputs when custom is selected", async () => {
      const user = userEvent.setup();
      render(<ResearchLauncher {...defaultProps} />);

      await user.click(screen.getByTestId("preset-custom"));
      expect(screen.getByTestId("custom-iterations-input")).toBeInTheDocument();
      expect(screen.getByTestId("custom-timeout-input")).toBeInTheDocument();
    });

    it("hides custom depth inputs when preset is selected", async () => {
      const user = userEvent.setup();
      render(<ResearchLauncher {...defaultProps} />);

      await user.click(screen.getByTestId("preset-custom"));
      expect(screen.getByTestId("custom-iterations-input")).toBeInTheDocument();

      await user.click(screen.getByTestId("preset-standard"));
      expect(screen.queryByTestId("custom-iterations-input")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Form Submission
  // ==========================================================================

  describe("form submission", () => {
    it("calls onLaunch with research brief on submit", async () => {
      const user = userEvent.setup();
      render(<ResearchLauncher {...defaultProps} />);

      await user.type(screen.getByTestId("question-input"), "What is the best approach?");
      await user.type(screen.getByTestId("context-input"), "We need to decide on architecture");
      await user.click(screen.getByTestId("launch-button"));

      expect(defaultProps.onLaunch).toHaveBeenCalledWith(
        expect.objectContaining({
          brief: expect.objectContaining({
            question: "What is the best approach?",
            context: "We need to decide on architecture",
          }),
          depth: expect.objectContaining({ type: "preset", preset: "standard" }),
        })
      );
    });

    it("calls onCancel when cancel is clicked", async () => {
      const user = userEvent.setup();
      render(<ResearchLauncher {...defaultProps} />);

      await user.click(screen.getByTestId("cancel-button"));
      expect(defaultProps.onCancel).toHaveBeenCalled();
    });

    it("includes custom depth in submission when custom selected", async () => {
      const user = userEvent.setup();
      render(<ResearchLauncher {...defaultProps} />);

      await user.type(screen.getByTestId("question-input"), "Research question");
      await user.click(screen.getByTestId("preset-custom"));
      await user.clear(screen.getByTestId("custom-iterations-input"));
      await user.type(screen.getByTestId("custom-iterations-input"), "100");
      await user.clear(screen.getByTestId("custom-timeout-input"));
      await user.type(screen.getByTestId("custom-timeout-input"), "4");
      await user.click(screen.getByTestId("launch-button"));

      expect(defaultProps.onLaunch).toHaveBeenCalledWith(
        expect.objectContaining({
          depth: expect.objectContaining({
            type: "custom",
            config: expect.objectContaining({ maxIterations: 100, timeoutHours: 4 }),
          }),
        })
      );
    });
  });

  // ==========================================================================
  // Validation
  // ==========================================================================

  describe("validation", () => {
    it("disables launch button when question is empty", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByTestId("launch-button")).toBeDisabled();
    });

    it("enables launch button when question is filled", async () => {
      const user = userEvent.setup();
      render(<ResearchLauncher {...defaultProps} />);

      await user.type(screen.getByTestId("question-input"), "What to research?");
      expect(screen.getByTestId("launch-button")).not.toBeDisabled();
    });
  });

  // ==========================================================================
  // Loading State
  // ==========================================================================

  describe("loading state", () => {
    it("disables form when launching", () => {
      render(<ResearchLauncher {...defaultProps} isLaunching />);
      expect(screen.getByTestId("question-input")).toBeDisabled();
      expect(screen.getByTestId("launch-button")).toBeDisabled();
    });

    it("shows launching text on button", () => {
      render(<ResearchLauncher {...defaultProps} isLaunching />);
      expect(screen.getByTestId("launch-button")).toHaveTextContent(/launching/i);
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("labels question input", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByLabelText(/research question/i)).toBeInTheDocument();
    });

    it("labels depth presets as radio group", () => {
      render(<ResearchLauncher {...defaultProps} />);
      expect(screen.getByRole("radiogroup")).toBeInTheDocument();
    });

    it("preset buttons have radio role", () => {
      render(<ResearchLauncher {...defaultProps} />);
      const presets = screen.getAllByRole("radio");
      expect(presets.length).toBeGreaterThanOrEqual(4);
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<ResearchLauncher {...defaultProps} />);
      const launcher = screen.getByTestId("research-launcher");
      expect(launcher).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses accent color for launch button", () => {
      render(<ResearchLauncher {...defaultProps} />);
      const button = screen.getByTestId("launch-button");
      expect(button).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });

    it("uses accent color for selected preset", async () => {
      const user = userEvent.setup();
      render(<ResearchLauncher {...defaultProps} />);

      await user.type(screen.getByTestId("question-input"), "Question");
      const selectedPreset = screen.getByTestId("preset-standard");
      expect(selectedPreset.getAttribute("style")).toContain("border-color: var(--accent-primary)");
    });
  });
});
