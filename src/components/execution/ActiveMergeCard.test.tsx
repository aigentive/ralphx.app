/**
 * ActiveMergeCard component tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ActiveMergeCard } from "./ActiveMergeCard";
import type { MergePipelineTask } from "@/api/merge-pipeline";

function createMockMergeTask(overrides?: Partial<MergePipelineTask>): MergePipelineTask {
  return {
    taskId: "task-merge-001",
    title: "Merge Task",
    internalStatus: "merging",
    sourceBranch: "ralphx/app/task-merge-001",
    targetBranch: "main",
    isDeferred: false,
    isMainMergeDeferred: false,
    blockingBranch: null,
    conflictFiles: null,
    errorContext: null,
    ...overrides,
  };
}

describe("ActiveMergeCard", () => {
  describe("rendering", () => {
    it("renders the task title", () => {
      render(<ActiveMergeCard task={createMockMergeTask({ title: "Merge feature branch" })} onStop={vi.fn()} onViewDetails={vi.fn()} />);
      expect(screen.getByText("Merge feature branch")).toBeInTheDocument();
    });

    it("renders stop button", () => {
      const { container } = render(<ActiveMergeCard task={createMockMergeTask()} onStop={vi.fn()} onViewDetails={vi.fn()} />);
      const stopButton = container.querySelector("button[title='Stop merge']");
      expect(stopButton).toBeInTheDocument();
    });

    it("calls onStop with task ID when stop button clicked", () => {
      const onStop = vi.fn();
      const task = createMockMergeTask({ taskId: "task-stop-test" });
      const { container } = render(<ActiveMergeCard task={task} onStop={onStop} onViewDetails={vi.fn()} />);

      const stopButton = container.querySelector("button[title='Stop merge']") as HTMLButtonElement;
      fireEvent.click(stopButton);

      expect(onStop).toHaveBeenCalledWith("task-stop-test");
      expect(onStop).toHaveBeenCalledOnce();
    });
  });

  describe("click-to-navigate", () => {
    it("calls onViewDetails with task.taskId when title is clicked", () => {
      const onViewDetails = vi.fn();
      const task = createMockMergeTask({ taskId: "task-nav-merge-999", title: "Navigate merge" });
      render(<ActiveMergeCard task={task} onStop={vi.fn()} onViewDetails={onViewDetails} />);

      fireEvent.click(screen.getByText("Navigate merge"));

      expect(onViewDetails).toHaveBeenCalledWith("task-nav-merge-999");
      expect(onViewDetails).toHaveBeenCalledOnce();
    });

    it("title is rendered as a button element", () => {
      const task = createMockMergeTask({ title: "Merge Button Task" });
      render(<ActiveMergeCard task={task} onStop={vi.fn()} onViewDetails={vi.fn()} />);

      const titleEl = screen.getByText("Merge Button Task");
      expect(titleEl.tagName).toBe("BUTTON");
    });
  });
});
