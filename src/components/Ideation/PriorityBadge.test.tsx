/**
 * PriorityBadge.test.tsx
 * Tests for the priority badge component
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { PriorityBadge } from "./PriorityBadge";

describe("PriorityBadge", () => {
  describe("Rendering", () => {
    it("renders badge with priority text", () => {
      render(<PriorityBadge priority="high" />);
      expect(screen.getByText("High")).toBeInTheDocument();
    });

    it("renders with data-testid", () => {
      render(<PriorityBadge priority="medium" />);
      expect(screen.getByTestId("priority-badge")).toBeInTheDocument();
    });

    it("includes priority level in data attribute", () => {
      render(<PriorityBadge priority="critical" />);
      expect(screen.getByTestId("priority-badge")).toHaveAttribute("data-priority", "critical");
    });
  });

  describe("Priority Colors", () => {
    it("renders critical with red background (#ef4444)", () => {
      render(<PriorityBadge priority="critical" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ef4444" });
    });

    it("renders high with orange background (#ff6b35)", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ff6b35" });
    });

    it("renders medium with amber background (#ffa94d)", () => {
      render(<PriorityBadge priority="medium" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ffa94d" });
    });

    it("renders low with gray background (#6b7280)", () => {
      render(<PriorityBadge priority="low" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#6b7280" });
    });
  });

  describe("Text Colors", () => {
    it("critical has white text for contrast", () => {
      render(<PriorityBadge priority="critical" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ color: "#ffffff" });
    });

    it("high has dark text for contrast", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ color: "#1a1a1a" });
    });

    it("medium has dark text for contrast", () => {
      render(<PriorityBadge priority="medium" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ color: "#1a1a1a" });
    });

    it("low has white text for contrast", () => {
      render(<PriorityBadge priority="low" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ color: "#ffffff" });
    });
  });

  describe("Priority Text", () => {
    it("displays 'Critical' for critical priority", () => {
      render(<PriorityBadge priority="critical" />);
      expect(screen.getByText("Critical")).toBeInTheDocument();
    });

    it("displays 'High' for high priority", () => {
      render(<PriorityBadge priority="high" />);
      expect(screen.getByText("High")).toBeInTheDocument();
    });

    it("displays 'Medium' for medium priority", () => {
      render(<PriorityBadge priority="medium" />);
      expect(screen.getByText("Medium")).toBeInTheDocument();
    });

    it("displays 'Low' for low priority", () => {
      render(<PriorityBadge priority="low" />);
      expect(screen.getByText("Low")).toBeInTheDocument();
    });
  });

  describe("Size Variants", () => {
    it("renders compact size by default", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("text-xs");
      expect(badge).toHaveClass("px-1.5");
      expect(badge).toHaveClass("py-0.5");
    });

    it("renders compact size when size='compact'", () => {
      render(<PriorityBadge priority="high" size="compact" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("text-xs");
      expect(badge).toHaveClass("px-1.5");
      expect(badge).toHaveClass("py-0.5");
    });

    it("renders full size when size='full'", () => {
      render(<PriorityBadge priority="high" size="full" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("text-sm");
      expect(badge).toHaveClass("px-2");
      expect(badge).toHaveClass("py-1");
    });
  });

  describe("Styling", () => {
    it("has rounded corners", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("rounded");
    });

    it("has medium font weight", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("font-medium");
    });

    it("uses inline-flex for alignment", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("inline-flex");
    });

    it("centers content", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("items-center");
      expect(badge).toHaveClass("justify-center");
    });
  });

  describe("Accessibility", () => {
    it("has role=status by default", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveAttribute("role", "status");
    });

    it("has accessible label", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveAttribute("aria-label", "Priority: High");
    });

    it("uses correct aria-label for each priority", () => {
      const { rerender } = render(<PriorityBadge priority="critical" />);
      expect(screen.getByTestId("priority-badge")).toHaveAttribute("aria-label", "Priority: Critical");

      rerender(<PriorityBadge priority="medium" />);
      expect(screen.getByTestId("priority-badge")).toHaveAttribute("aria-label", "Priority: Medium");

      rerender(<PriorityBadge priority="low" />);
      expect(screen.getByTestId("priority-badge")).toHaveAttribute("aria-label", "Priority: Low");
    });
  });

  describe("Custom className", () => {
    it("accepts additional className", () => {
      render(<PriorityBadge priority="high" className="custom-class" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("custom-class");
    });

    it("merges custom className with default classes", () => {
      render(<PriorityBadge priority="high" className="mt-2" />);
      const badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveClass("mt-2");
      expect(badge).toHaveClass("rounded");
    });
  });

  describe("Anti-AI-Slop", () => {
    it("uses specified colors, not purple", () => {
      const { rerender } = render(<PriorityBadge priority="critical" />);
      let badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ef4444" });

      rerender(<PriorityBadge priority="high" />);
      badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ff6b35" });

      rerender(<PriorityBadge priority="medium" />);
      badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#ffa94d" });

      rerender(<PriorityBadge priority="low" />);
      badge = screen.getByTestId("priority-badge");
      expect(badge).toHaveStyle({ backgroundColor: "#6b7280" });
    });

    it("does not use Inter font", () => {
      render(<PriorityBadge priority="high" />);
      const badge = screen.getByTestId("priority-badge");
      const styles = window.getComputedStyle(badge);
      expect(styles.fontFamily).not.toMatch(/inter/i);
    });
  });
});
