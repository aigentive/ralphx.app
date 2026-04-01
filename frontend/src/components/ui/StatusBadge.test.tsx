/**
 * Tests for StatusBadge component
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { StatusBadge, type QAStatus } from "./StatusBadge";

describe("StatusBadge", () => {
  describe("review status variants", () => {
    it("should render AI Approved badge", () => {
      render(<StatusBadge type="review" status="ai_approved" />);
      expect(screen.getByText("AI Approved")).toBeInTheDocument();
      expect(screen.getByTestId("status-badge")).toHaveAttribute(
        "data-status",
        "ai_approved"
      );
    });

    it("should render Human Approved badge", () => {
      render(<StatusBadge type="review" status="human_approved" />);
      expect(screen.getByText("Human Approved")).toBeInTheDocument();
    });

    it("should render Needs Changes badge", () => {
      render(<StatusBadge type="review" status="needs_changes" />);
      expect(screen.getByText("Needs Changes")).toBeInTheDocument();
    });

    it("should render check icon for AI Approved", () => {
      render(<StatusBadge type="review" status="ai_approved" />);
      expect(screen.getByTestId("icon-check")).toBeInTheDocument();
    });

    it("should render double check icon for Human Approved", () => {
      render(<StatusBadge type="review" status="human_approved" />);
      expect(screen.getByTestId("icon-double-check")).toBeInTheDocument();
    });

    it("should render warning icon for Needs Changes", () => {
      render(<StatusBadge type="review" status="needs_changes" />);
      expect(screen.getByTestId("icon-warning")).toBeInTheDocument();
    });
  });

  describe("QA status variants", () => {
    const qaStatuses: QAStatus[] = [
      "pending",
      "preparing",
      "ready",
      "testing",
      "passed",
      "failed",
    ];

    it.each(qaStatuses)("should render %s QA status", (status) => {
      render(<StatusBadge type="qa" status={status} />);
      expect(screen.getByTestId("status-badge")).toHaveAttribute(
        "data-status",
        status
      );
    });

    it("should render 'Pending' text for pending status", () => {
      render(<StatusBadge type="qa" status="pending" />);
      expect(screen.getByText("Pending")).toBeInTheDocument();
    });

    it("should render 'Preparing' text for preparing status", () => {
      render(<StatusBadge type="qa" status="preparing" />);
      expect(screen.getByText("Preparing")).toBeInTheDocument();
    });

    it("should render 'Ready' text for ready status", () => {
      render(<StatusBadge type="qa" status="ready" />);
      expect(screen.getByText("Ready")).toBeInTheDocument();
    });

    it("should render 'Testing' text for testing status", () => {
      render(<StatusBadge type="qa" status="testing" />);
      expect(screen.getByText("Testing")).toBeInTheDocument();
    });

    it("should render 'Passed' text for passed status", () => {
      render(<StatusBadge type="qa" status="passed" />);
      expect(screen.getByText("Passed")).toBeInTheDocument();
    });

    it("should render 'Failed' text for failed status", () => {
      render(<StatusBadge type="qa" status="failed" />);
      expect(screen.getByText("Failed")).toBeInTheDocument();
    });
  });

  describe("styling", () => {
    it("should apply correct color for success status (passed)", () => {
      render(<StatusBadge type="qa" status="passed" />);
      const badge = screen.getByTestId("status-badge");
      expect(badge.style.backgroundColor).toBe("var(--status-success)");
    });

    it("should apply correct color for error status (failed)", () => {
      render(<StatusBadge type="qa" status="failed" />);
      const badge = screen.getByTestId("status-badge");
      expect(badge.style.backgroundColor).toBe("var(--status-error)");
    });

    it("should apply correct color for warning status (needs_changes)", () => {
      render(<StatusBadge type="review" status="needs_changes" />);
      const badge = screen.getByTestId("status-badge");
      expect(badge.style.backgroundColor).toBe("var(--status-warning)");
    });

    it("should apply correct color for info status (human_approved)", () => {
      render(<StatusBadge type="review" status="human_approved" />);
      const badge = screen.getByTestId("status-badge");
      expect(badge.style.backgroundColor).toBe("var(--status-info)");
    });
  });
});
