/**
 * Tests for ReviewStatusBadge component
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ReviewStatusBadge } from "./ReviewStatusBadge";
import type { ReviewStatus } from "@/types/review";

describe("ReviewStatusBadge", () => {
  describe("status display", () => {
    it("should render Pending badge for pending status", () => {
      render(<ReviewStatusBadge status="pending" />);
      expect(screen.getByText("Pending")).toBeInTheDocument();
      expect(screen.getByTestId("review-status-badge")).toHaveAttribute(
        "data-status",
        "pending"
      );
    });

    it("should render Approved badge for approved status", () => {
      render(<ReviewStatusBadge status="approved" />);
      expect(screen.getByText("Approved")).toBeInTheDocument();
      expect(screen.getByTestId("review-status-badge")).toHaveAttribute(
        "data-status",
        "approved"
      );
    });

    it("should render Changes Requested badge for changes_requested status", () => {
      render(<ReviewStatusBadge status="changes_requested" />);
      expect(screen.getByText("Changes Requested")).toBeInTheDocument();
      expect(screen.getByTestId("review-status-badge")).toHaveAttribute(
        "data-status",
        "changes_requested"
      );
    });

    it("should render Rejected badge for rejected status", () => {
      render(<ReviewStatusBadge status="rejected" />);
      expect(screen.getByText("Rejected")).toBeInTheDocument();
      expect(screen.getByTestId("review-status-badge")).toHaveAttribute(
        "data-status",
        "rejected"
      );
    });
  });

  describe("icons", () => {
    it("should render clock icon for pending status", () => {
      render(<ReviewStatusBadge status="pending" />);
      expect(screen.getByTestId("icon-clock")).toBeInTheDocument();
    });

    it("should render check icon for approved status", () => {
      render(<ReviewStatusBadge status="approved" />);
      expect(screen.getByTestId("icon-check")).toBeInTheDocument();
    });

    it("should render warning icon for changes_requested status", () => {
      render(<ReviewStatusBadge status="changes_requested" />);
      expect(screen.getByTestId("icon-warning")).toBeInTheDocument();
    });

    it("should render x icon for rejected status", () => {
      render(<ReviewStatusBadge status="rejected" />);
      expect(screen.getByTestId("icon-x")).toBeInTheDocument();
    });
  });

  describe("colors", () => {
    it("should apply orange/warning color for pending status", () => {
      render(<ReviewStatusBadge status="pending" />);
      const badge = screen.getByTestId("review-status-badge");
      expect(badge.style.backgroundColor).toBe("var(--status-warning)");
    });

    it("should apply green/success color for approved status", () => {
      render(<ReviewStatusBadge status="approved" />);
      const badge = screen.getByTestId("review-status-badge");
      expect(badge.style.backgroundColor).toBe("var(--status-success)");
    });

    it("should apply orange/warning color for changes_requested status", () => {
      render(<ReviewStatusBadge status="changes_requested" />);
      const badge = screen.getByTestId("review-status-badge");
      expect(badge.style.backgroundColor).toBe("var(--status-warning)");
    });

    it("should apply red/error color for rejected status", () => {
      render(<ReviewStatusBadge status="rejected" />);
      const badge = screen.getByTestId("review-status-badge");
      expect(badge.style.backgroundColor).toBe("var(--status-error)");
    });

    it("should use dark text color for contrast", () => {
      render(<ReviewStatusBadge status="approved" />);
      const badge = screen.getByTestId("review-status-badge");
      expect(badge.style.color).toBe("var(--bg-base)");
    });
  });

  describe("all status types", () => {
    const allStatuses: ReviewStatus[] = [
      "pending",
      "approved",
      "changes_requested",
      "rejected",
    ];

    it.each(allStatuses)("should render %s status correctly", (status) => {
      render(<ReviewStatusBadge status={status} />);
      const badge = screen.getByTestId("review-status-badge");
      expect(badge).toBeInTheDocument();
      expect(badge).toHaveAttribute("data-status", status);
    });
  });
});
