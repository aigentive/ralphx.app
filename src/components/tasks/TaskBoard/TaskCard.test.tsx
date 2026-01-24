/**
 * Tests for TaskCard component
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { DndContext } from "@dnd-kit/core";
import { createMockTask } from "@/test/mock-data";
import { TaskCard } from "./TaskCard";
import type { QAPrepStatus } from "@/types/qa-config";
import type { QAOverallStatus } from "@/types/qa";

// Wrapper component for dnd-kit context
function DndWrapper({ children }: { children: React.ReactNode }) {
  return <DndContext>{children}</DndContext>;
}

describe("TaskCard", () => {
  describe("rendering", () => {
    it("should render with data-testid", () => {
      const task = createMockTask({ id: "task-123" });
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      expect(screen.getByTestId("task-card-task-123")).toBeInTheDocument();
    });

    it("should render task title", () => {
      const task = createMockTask({ title: "My Test Task" });
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      expect(screen.getByText("My Test Task")).toBeInTheDocument();
    });

    it("should render category badge", () => {
      const task = createMockTask({ category: "feature" });
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      expect(screen.getByText("feature")).toBeInTheDocument();
    });

    it("should render priority indicator", () => {
      const task = createMockTask({ priority: 2 });
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      expect(screen.getByTestId("priority-indicator")).toBeInTheDocument();
    });

    it("should truncate long titles", () => {
      const longTitle = "This is a very long task title that should be truncated";
      const task = createMockTask({ title: longTitle });
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      const titleElement = screen.getByTestId("task-title");
      expect(titleElement).toHaveClass("truncate");
    });
  });

  describe("status badges", () => {
    it("should render review status badge when reviewStatus is provided", () => {
      const task = createMockTask();
      render(<TaskCard task={task} reviewStatus="ai_approved" />, {
        wrapper: DndWrapper,
      });
      expect(screen.getByText("AI Approved")).toBeInTheDocument();
    });

    it("should not render review badge when reviewStatus is not provided", () => {
      const task = createMockTask();
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      expect(screen.queryByText("AI Approved")).not.toBeInTheDocument();
    });

    it("should render checkpoint indicator when hasCheckpoint is true", () => {
      const task = createMockTask();
      render(<TaskCard task={task} hasCheckpoint />, { wrapper: DndWrapper });
      expect(screen.getByTestId("checkpoint-indicator")).toBeInTheDocument();
    });
  });

  describe("QA badge", () => {
    it("should render QA badge when needsQA is true", () => {
      const task = createMockTask();
      render(<TaskCard task={task} needsQA />, { wrapper: DndWrapper });
      expect(screen.getByTestId("task-qa-badge")).toBeInTheDocument();
    });

    it("should not render QA badge when needsQA is false", () => {
      const task = createMockTask();
      render(<TaskCard task={task} needsQA={false} />, { wrapper: DndWrapper });
      expect(screen.queryByTestId("task-qa-badge")).not.toBeInTheDocument();
    });

    it("should not render QA badge when needsQA is not provided", () => {
      const task = createMockTask();
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      expect(screen.queryByTestId("task-qa-badge")).not.toBeInTheDocument();
    });

    it("should show pending status when no prep or test status", () => {
      const task = createMockTask();
      render(<TaskCard task={task} needsQA />, { wrapper: DndWrapper });
      expect(screen.getByText("QA Pending")).toBeInTheDocument();
    });

    it("should show preparing status when prep is running", () => {
      const task = createMockTask();
      const prepStatus: QAPrepStatus = "running";
      render(<TaskCard task={task} needsQA prepStatus={prepStatus} />, {
        wrapper: DndWrapper,
      });
      expect(screen.getByText("Preparing")).toBeInTheDocument();
    });

    it("should show ready status when prep is completed", () => {
      const task = createMockTask();
      const prepStatus: QAPrepStatus = "completed";
      render(<TaskCard task={task} needsQA prepStatus={prepStatus} />, {
        wrapper: DndWrapper,
      });
      expect(screen.getByText("QA Ready")).toBeInTheDocument();
    });

    it("should show testing status when test is running", () => {
      const task = createMockTask();
      const testStatus: QAOverallStatus = "running";
      render(<TaskCard task={task} needsQA testStatus={testStatus} />, {
        wrapper: DndWrapper,
      });
      expect(screen.getByText("Testing")).toBeInTheDocument();
    });

    it("should show passed status when test is passed", () => {
      const task = createMockTask();
      const testStatus: QAOverallStatus = "passed";
      render(<TaskCard task={task} needsQA testStatus={testStatus} />, {
        wrapper: DndWrapper,
      });
      expect(screen.getByText("Passed")).toBeInTheDocument();
    });

    it("should show failed status when test is failed", () => {
      const task = createMockTask();
      const testStatus: QAOverallStatus = "failed";
      render(<TaskCard task={task} needsQA testStatus={testStatus} />, {
        wrapper: DndWrapper,
      });
      expect(screen.getByText("Failed")).toBeInTheDocument();
    });

    it("should prioritize test status over prep status", () => {
      const task = createMockTask();
      const prepStatus: QAPrepStatus = "running";
      const testStatus: QAOverallStatus = "passed";
      render(
        <TaskCard task={task} needsQA prepStatus={prepStatus} testStatus={testStatus} />,
        { wrapper: DndWrapper }
      );
      // Test status should take precedence
      expect(screen.getByText("Passed")).toBeInTheDocument();
      expect(screen.queryByText("Preparing")).not.toBeInTheDocument();
    });
  });

  describe("click handler", () => {
    it("should call onSelect when card is clicked", () => {
      const task = createMockTask({ id: "task-1" });
      const onSelect = vi.fn();
      render(<TaskCard task={task} onSelect={onSelect} />, {
        wrapper: DndWrapper,
      });

      fireEvent.click(screen.getByTestId("task-card-task-1"));
      expect(onSelect).toHaveBeenCalledWith("task-1");
    });

    it("should not crash when onSelect is not provided", () => {
      const task = createMockTask();
      render(<TaskCard task={task} />, { wrapper: DndWrapper });

      expect(() =>
        fireEvent.click(screen.getByTestId(`task-card-${task.id}`))
      ).not.toThrow();
    });
  });

  describe("dragging state", () => {
    it("should apply opacity-50 class when isDragging is true", () => {
      const task = createMockTask();
      render(<TaskCard task={task} isDragging />, { wrapper: DndWrapper });

      const card = screen.getByTestId(`task-card-${task.id}`);
      expect(card).toHaveClass("opacity-50");
    });

    it("should not apply opacity-50 class when isDragging is false", () => {
      const task = createMockTask();
      render(<TaskCard task={task} isDragging={false} />, {
        wrapper: DndWrapper,
      });

      const card = screen.getByTestId(`task-card-${task.id}`);
      expect(card).not.toHaveClass("opacity-50");
    });
  });

  describe("drag handle", () => {
    it("should render drag handle", () => {
      const task = createMockTask();
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      expect(screen.getByTestId("drag-handle")).toBeInTheDocument();
    });

    it("should have proper cursor style on drag handle", () => {
      const task = createMockTask();
      render(<TaskCard task={task} />, { wrapper: DndWrapper });
      const handle = screen.getByTestId("drag-handle");
      expect(handle).toHaveClass("cursor-grab");
    });
  });
});
