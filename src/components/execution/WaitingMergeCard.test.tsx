/**
 * WaitingMergeCard component tests
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { WaitingMergeCard } from "./WaitingMergeCard";
import type { MergePipelineTask } from "@/api/merge-pipeline";

function createMockTask(overrides?: Partial<MergePipelineTask>): MergePipelineTask {
  return {
    taskId: "task-123",
    title: "Test merge task",
    internalStatus: "pending_merge",
    sourceBranch: "ralphx/app/task-abc123",
    targetBranch: "main",
    isDeferred: false,
    isMainMergeDeferred: false,
    blockingBranch: null,
    conflictFiles: null,
    errorContext: null,
    ...overrides,
  };
}

describe("WaitingMergeCard", () => {
  describe("normal waiting state", () => {
    it("renders task title", () => {
      render(<WaitingMergeCard task={createMockTask({ title: "Add user auth" })} />);
      expect(screen.getByText("Add user auth")).toBeInTheDocument();
    });

    it("renders short branch name", () => {
      render(<WaitingMergeCard task={createMockTask({ targetBranch: "ralphx/app/plan-xyz" })} />);
      expect(screen.getByText("plan-xyz")).toBeInTheDocument();
    });

    it("shows clock icon for non-deferred tasks", () => {
      const { container } = render(<WaitingMergeCard task={createMockTask()} />);
      expect(container.querySelector("[data-testid='main-merge-deferred-icon']")).not.toBeInTheDocument();
    });

    it("does not show deferred badge", () => {
      render(<WaitingMergeCard task={createMockTask()} />);
      expect(screen.queryByTestId("main-merge-deferred-badge")).not.toBeInTheDocument();
    });

    it("shows tooltip with pending merge reason", () => {
      const { container } = render(<WaitingMergeCard task={createMockTask()} />);
      const row = container.firstElementChild as HTMLElement;
      expect(row.getAttribute("title")).toContain("Pending merge");
    });
  });

  describe("branch-deferred state", () => {
    it("shows tooltip with blocking branch", () => {
      const task = createMockTask({
        isDeferred: true,
        blockingBranch: "ralphx/app/task-other",
      });
      const { container } = render(<WaitingMergeCard task={task} />);
      const row = container.firstElementChild as HTMLElement;
      expect(row.getAttribute("title")).toContain("Waiting for ralphx/app/task-other to merge");
    });

    it("shows generic deferred message when no blocking branch", () => {
      const task = createMockTask({
        isDeferred: true,
        blockingBranch: null,
      });
      const { container } = render(<WaitingMergeCard task={task} />);
      const row = container.firstElementChild as HTMLElement;
      expect(row.getAttribute("title")).toContain("Waiting for active merge to complete");
    });

    it("does not show main-merge-deferred badge", () => {
      const task = createMockTask({ isDeferred: true, blockingBranch: "other" });
      render(<WaitingMergeCard task={task} />);
      expect(screen.queryByTestId("main-merge-deferred-badge")).not.toBeInTheDocument();
    });
  });

  describe("main-merge-deferred state", () => {
    it("shows Users icon instead of Clock", () => {
      const task = createMockTask({
        isDeferred: true,
        isMainMergeDeferred: true,
      });
      render(<WaitingMergeCard task={task} />);
      expect(screen.getByTestId("main-merge-deferred-icon")).toBeInTheDocument();
    });

    it("shows deferred badge with agent count", () => {
      const task = createMockTask({
        isDeferred: true,
        isMainMergeDeferred: true,
      });
      render(<WaitingMergeCard task={task} runningCount={3} />);
      expect(screen.getByTestId("main-merge-deferred-badge")).toBeInTheDocument();
      expect(screen.getByText("3 agents")).toBeInTheDocument();
    });

    it("shows singular 'agent' for count of 1", () => {
      const task = createMockTask({
        isDeferred: true,
        isMainMergeDeferred: true,
      });
      render(<WaitingMergeCard task={task} runningCount={1} />);
      expect(screen.getByText("1 agent")).toBeInTheDocument();
    });

    it("shows generic 'agents' when runningCount is 0", () => {
      const task = createMockTask({
        isDeferred: true,
        isMainMergeDeferred: true,
      });
      render(<WaitingMergeCard task={task} runningCount={0} />);
      expect(screen.getByText("agents")).toBeInTheDocument();
    });

    it("shows generic 'agents' when runningCount is undefined", () => {
      const task = createMockTask({
        isDeferred: true,
        isMainMergeDeferred: true,
      });
      render(<WaitingMergeCard task={task} />);
      expect(screen.getByText("agents")).toBeInTheDocument();
    });

    it("shows tooltip with agent-specific message", () => {
      const task = createMockTask({
        isDeferred: true,
        isMainMergeDeferred: true,
      });
      const { container } = render(<WaitingMergeCard task={task} runningCount={2} />);
      const row = container.firstElementChild as HTMLElement;
      expect(row.getAttribute("title")).toContain("Waiting for 2 agents to finish");
    });

    it("uses accent color for icon and badge", () => {
      const task = createMockTask({
        isDeferred: true,
        isMainMergeDeferred: true,
      });
      render(<WaitingMergeCard task={task} runningCount={2} />);
      const icon = screen.getByTestId("main-merge-deferred-icon");
      expect(icon).toHaveStyle({ color: "#ff6b35" });

      const badge = screen.getByTestId("main-merge-deferred-badge");
      expect(badge).toHaveStyle({ color: "#ff6b35" });
    });

    it("still renders task title and branch", () => {
      const task = createMockTask({
        title: "Deploy feature X",
        targetBranch: "main",
        isDeferred: true,
        isMainMergeDeferred: true,
      });
      render(<WaitingMergeCard task={task} runningCount={1} />);
      expect(screen.getByText("Deploy feature X")).toBeInTheDocument();
      expect(screen.getByText("main")).toBeInTheDocument();
    });
  });
});
