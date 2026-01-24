/**
 * ReviewCard component tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ReviewCard } from "./ReviewCard";
import type { Review } from "@/types/review";

const createMockReview = (overrides: Partial<Review> = {}): Review => ({
  id: "review-1",
  projectId: "project-1",
  taskId: "task-1",
  reviewerType: "ai",
  status: "pending",
  notes: null,
  createdAt: "2026-01-24T12:00:00Z",
  completedAt: null,
  ...overrides,
});

describe("ReviewCard", () => {
  describe("basic rendering", () => {
    it("renders task title", () => {
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Add user authentication"
        />
      );
      expect(screen.getByTestId("review-task-title")).toHaveTextContent("Add user authentication");
    });

    it("renders review status badge", () => {
      render(
        <ReviewCard
          review={createMockReview({ status: "pending" })}
          taskTitle="Test task"
        />
      );
      expect(screen.getByTestId("review-status-badge")).toBeInTheDocument();
    });

    it("renders notes when provided", () => {
      render(
        <ReviewCard
          review={createMockReview({ notes: "Security-sensitive changes detected" })}
          taskTitle="Test task"
        />
      );
      expect(screen.getByTestId("review-notes")).toHaveTextContent("Security-sensitive changes detected");
    });

    it("does not render notes section when notes is null", () => {
      render(
        <ReviewCard
          review={createMockReview({ notes: null })}
          taskTitle="Test task"
        />
      );
      expect(screen.queryByTestId("review-notes")).not.toBeInTheDocument();
    });
  });

  describe("reviewer type indicator", () => {
    it("shows AI indicator for ai reviewer", () => {
      render(
        <ReviewCard
          review={createMockReview({ reviewerType: "ai" })}
          taskTitle="Test task"
        />
      );
      expect(screen.getByTestId("reviewer-type-indicator")).toHaveTextContent("AI Review");
    });

    it("shows Human indicator for human reviewer", () => {
      render(
        <ReviewCard
          review={createMockReview({ reviewerType: "human" })}
          taskTitle="Test task"
        />
      );
      expect(screen.getByTestId("reviewer-type-indicator")).toHaveTextContent("Human Review");
    });
  });

  describe("action buttons", () => {
    it("renders View Diff button", () => {
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Test task"
          onViewDiff={vi.fn()}
        />
      );
      expect(screen.getByRole("button", { name: /view diff/i })).toBeInTheDocument();
    });

    it("renders Approve button for pending review", () => {
      render(
        <ReviewCard
          review={createMockReview({ status: "pending" })}
          taskTitle="Test task"
          onApprove={vi.fn()}
        />
      );
      expect(screen.getByRole("button", { name: /approve/i })).toBeInTheDocument();
    });

    it("renders Request Changes button for pending review", () => {
      render(
        <ReviewCard
          review={createMockReview({ status: "pending" })}
          taskTitle="Test task"
          onRequestChanges={vi.fn()}
        />
      );
      expect(screen.getByRole("button", { name: /request changes/i })).toBeInTheDocument();
    });

    it("calls onViewDiff when View Diff clicked", () => {
      const onViewDiff = vi.fn();
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Test task"
          onViewDiff={onViewDiff}
        />
      );
      fireEvent.click(screen.getByRole("button", { name: /view diff/i }));
      expect(onViewDiff).toHaveBeenCalledWith("review-1");
    });

    it("calls onApprove when Approve clicked", () => {
      const onApprove = vi.fn();
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Test task"
          onApprove={onApprove}
        />
      );
      fireEvent.click(screen.getByRole("button", { name: /approve/i }));
      expect(onApprove).toHaveBeenCalledWith("review-1");
    });

    it("calls onRequestChanges when Request Changes clicked", () => {
      const onRequestChanges = vi.fn();
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Test task"
          onRequestChanges={onRequestChanges}
        />
      );
      fireEvent.click(screen.getByRole("button", { name: /request changes/i }));
      expect(onRequestChanges).toHaveBeenCalledWith("review-1");
    });

    it("hides action buttons for completed reviews", () => {
      render(
        <ReviewCard
          review={createMockReview({ status: "approved" })}
          taskTitle="Test task"
          onApprove={vi.fn()}
          onRequestChanges={vi.fn()}
        />
      );
      expect(screen.queryByRole("button", { name: /approve/i })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /request changes/i })).not.toBeInTheDocument();
    });
  });

  describe("fix task indicator", () => {
    it("shows fix attempt counter when provided", () => {
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Fix login validation"
          fixAttempt={2}
          maxFixAttempts={3}
        />
      );
      expect(screen.getByTestId("fix-attempt-counter")).toHaveTextContent("Attempt 2 of 3");
    });

    it("does not show fix attempt counter when not a fix task", () => {
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Test task"
        />
      );
      expect(screen.queryByTestId("fix-attempt-counter")).not.toBeInTheDocument();
    });

    it("shows warning style when at max attempts", () => {
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Fix login validation"
          fixAttempt={3}
          maxFixAttempts={3}
        />
      );
      const counter = screen.getByTestId("fix-attempt-counter");
      expect(counter).toHaveAttribute("data-at-max", "true");
    });
  });

  describe("data attributes", () => {
    it("sets data-testid on card container", () => {
      render(
        <ReviewCard
          review={createMockReview({ id: "review-123" })}
          taskTitle="Test task"
        />
      );
      expect(screen.getByTestId("review-card-review-123")).toBeInTheDocument();
    });

    it("sets data-status attribute", () => {
      render(
        <ReviewCard
          review={createMockReview({ status: "changes_requested" })}
          taskTitle="Test task"
        />
      );
      expect(screen.getByTestId("review-card-review-1")).toHaveAttribute("data-status", "changes_requested");
    });

    it("sets data-reviewer-type attribute", () => {
      render(
        <ReviewCard
          review={createMockReview({ reviewerType: "human" })}
          taskTitle="Test task"
        />
      );
      expect(screen.getByTestId("review-card-review-1")).toHaveAttribute("data-reviewer-type", "human");
    });
  });

  describe("styling", () => {
    it("applies design system background color", () => {
      render(
        <ReviewCard
          review={createMockReview()}
          taskTitle="Test task"
        />
      );
      const card = screen.getByTestId("review-card-review-1");
      expect(card).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });
  });
});
