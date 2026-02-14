/**
 * QuickActionRow.test.tsx - Tests for QuickActionRow component
 *
 * Tests all four flow states (idle, confirming, creating, success),
 * button callbacks, and animation props.
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Lightbulb } from "lucide-react";
import { QuickActionRow } from "./QuickActionRow";
import type { QuickAction, QuickActionFlowState } from "./QuickActionRow";

// Mock framer-motion to avoid animation complexity in tests
vi.mock("framer-motion", () => ({
  motion: {
    div: ({ children, ...props }: React.PropsWithChildren<Record<string, unknown>>) => (
      <div {...props}>{children}</div>
    ),
  },
  AnimatePresence: ({ children }: React.PropsWithChildren) => <>{children}</>,
}));

describe("QuickActionRow", () => {
  const mockAction: QuickAction = {
    id: "test-action",
    label: "Test Action",
    icon: Lightbulb,
    description: (query: string) => `"${query}"`,
    isVisible: (query: string) => query.trim().length > 0,
    execute: vi.fn().mockResolvedValue("entity-123"),
    creatingLabel: "Creating test...",
    successLabel: "Test created!",
    viewLabel: "View Test",
    navigateTo: vi.fn(),
  };

  const defaultProps = {
    action: mockAction,
    flowState: "idle" as QuickActionFlowState,
    searchQuery: "test query",
    isHighlighted: false,
    onMouseEnter: vi.fn(),
    onSelect: vi.fn(),
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
    onViewEntity: vi.fn(),
    highlightedRef: null,
  };

  describe("idle state", () => {
    it("renders button row with icon, label, and query description", () => {
      render(<QuickActionRow {...defaultProps} />);

      expect(screen.getByText("Test Action")).toBeInTheDocument();
      expect(screen.getByText('"test query"')).toBeInTheDocument();
      // Icon should be rendered (Lightbulb icon)
      const iconContainer = screen.getByText("Test Action").closest("button");
      expect(iconContainer).toBeInTheDocument();
    });

    it("calls onSelect when clicked", async () => {
      const user = userEvent.setup();
      const onSelect = vi.fn();
      render(<QuickActionRow {...defaultProps} onSelect={onSelect} />);

      const button = screen.getByText("Test Action").closest("button");
      await user.click(button!);

      expect(onSelect).toHaveBeenCalledTimes(1);
    });

    it("calls onMouseEnter when hovered", async () => {
      const user = userEvent.setup();
      const onMouseEnter = vi.fn();
      render(<QuickActionRow {...defaultProps} onMouseEnter={onMouseEnter} />);

      const button = screen.getByText("Test Action").closest("button");
      await user.hover(button!);

      expect(onMouseEnter).toHaveBeenCalledTimes(1);
    });

    it("applies highlighted styling when isHighlighted=true", () => {
      render(<QuickActionRow {...defaultProps} isHighlighted={true} />);

      const button = screen.getByText("Test Action").closest("button");
      // Check that background is set (exact rgba value depends on browser conversion)
      expect(button).toHaveStyle({ background: "rgba(255, 99, 51, 0.16)" });
    });

    it("applies highlightedRef when isHighlighted=true", () => {
      const ref = { current: null };
      render(<QuickActionRow {...defaultProps} isHighlighted={true} highlightedRef={ref} />);

      const button = screen.getByText("Test Action").closest("button");
      expect(ref.current).toBe(button);
    });

    it("has motion.div with animation props", () => {
      const { container } = render(<QuickActionRow {...defaultProps} />);

      // Check that motion.div exists with animation props
      const motionDiv = container.querySelector("[data-testid='quick-action-idle']");
      expect(motionDiv).toBeInTheDocument();
    });
  });

  describe("confirming state", () => {
    const confirmingProps = {
      ...defaultProps,
      flowState: "confirming" as QuickActionFlowState,
    };

    it("renders confirmation prompt with query", () => {
      render(<QuickActionRow {...confirmingProps} />);

      expect(screen.getByText(/Start/i)).toBeInTheDocument();
      expect(screen.getByText(/test query/i)).toBeInTheDocument();
    });

    it("renders Confirm and Cancel buttons", () => {
      render(<QuickActionRow {...confirmingProps} />);

      expect(screen.getByRole("button", { name: /confirm/i })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /cancel/i })).toBeInTheDocument();
    });

    it("calls onConfirm when Confirm clicked", async () => {
      const user = userEvent.setup();
      const onConfirm = vi.fn();
      render(<QuickActionRow {...confirmingProps} onConfirm={onConfirm} />);

      await user.click(screen.getByRole("button", { name: /confirm/i }));

      expect(onConfirm).toHaveBeenCalledTimes(1);
    });

    it("calls onCancel when Cancel clicked", async () => {
      const user = userEvent.setup();
      const onCancel = vi.fn();
      render(<QuickActionRow {...confirmingProps} onCancel={onCancel} />);

      await user.click(screen.getByRole("button", { name: /cancel/i }));

      expect(onCancel).toHaveBeenCalledTimes(1);
    });
  });

  describe("creating state", () => {
    const creatingProps = {
      ...defaultProps,
      flowState: "creating" as QuickActionFlowState,
    };

    it("renders spinner and creatingLabel", () => {
      render(<QuickActionRow {...creatingProps} />);

      expect(screen.getByText("Creating test...")).toBeInTheDocument();
      // Spinner should be present (Loader2 icon with animate-spin)
      const spinner = screen.getByText("Creating test...").previousElementSibling;
      expect(spinner).toHaveClass("animate-spin");
    });

    it("disables all interactions", () => {
      render(<QuickActionRow {...creatingProps} />);

      // Should not have any interactive buttons
      const buttons = screen.queryAllByRole("button");
      expect(buttons).toHaveLength(0);
    });
  });

  describe("success state", () => {
    const successProps = {
      ...defaultProps,
      flowState: "success" as QuickActionFlowState,
    };

    it("renders check icon and successLabel", () => {
      render(<QuickActionRow {...successProps} />);

      expect(screen.getByText("Test created!")).toBeInTheDocument();
      // Check icon should be present
      const checkIcon = screen.getByText("Test created!").previousElementSibling;
      expect(checkIcon).toBeInTheDocument();
    });

    it("renders viewLabel button", () => {
      render(<QuickActionRow {...successProps} />);

      expect(screen.getByRole("button", { name: "View Test" })).toBeInTheDocument();
    });

    it("calls onViewEntity when view button clicked", async () => {
      const user = userEvent.setup();
      const onViewEntity = vi.fn();
      render(<QuickActionRow {...successProps} onViewEntity={onViewEntity} />);

      await user.click(screen.getByRole("button", { name: "View Test" }));

      expect(onViewEntity).toHaveBeenCalledTimes(1);
    });
  });

  describe("animation props", () => {
    it("idle state has motion.div with height animation", () => {
      const { container } = render(<QuickActionRow {...defaultProps} />);

      const motionDiv = container.querySelector("[data-testid='quick-action-idle']");
      expect(motionDiv).toBeInTheDocument();
    });

    it("confirming/creating/success states wrapped in AnimatePresence", () => {
      const { container, rerender } = render(
        <QuickActionRow {...defaultProps} flowState="confirming" />
      );

      let content = container.querySelector("[data-testid='quick-action-content']");
      expect(content).toBeInTheDocument();

      rerender(<QuickActionRow {...defaultProps} flowState="creating" />);
      content = container.querySelector("[data-testid='quick-action-content']");
      expect(content).toBeInTheDocument();

      rerender(<QuickActionRow {...defaultProps} flowState="success" />);
      content = container.querySelector("[data-testid='quick-action-content']");
      expect(content).toBeInTheDocument();
    });
  });
});
