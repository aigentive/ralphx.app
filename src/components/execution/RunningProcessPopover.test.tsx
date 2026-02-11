/**
 * RunningProcessPopover component tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { RunningProcessPopover } from "./RunningProcessPopover";
import type { RunningProcess } from "@/api/running-processes";

// Mock process data helper
function createMockProcess(overrides?: Partial<RunningProcess>): RunningProcess {
  return {
    taskId: "task-123",
    title: "Test Task",
    internalStatus: "executing",
    stepProgress: {
      taskId: "task-123",
      total: 7,
      completed: 2,
      inProgress: 1,
      pending: 4,
      skipped: 0,
      failed: 0,
      currentStep: {
        id: "step-3",
        taskId: "task-123",
        title: "Step 3",
        description: null,
        status: "in_progress",
        sortOrder: 2,
        dependsOn: null,
        createdBy: "user",
        completionNote: null,
        createdAt: "2026-02-11T12:00:00Z",
        updatedAt: "2026-02-11T12:00:00Z",
        startedAt: "2026-02-11T12:00:00Z",
        completedAt: null,
      },
      nextStep: null,
      percentComplete: 28.57,
    },
    elapsedSeconds: 134,
    triggerOrigin: "scheduler",
    taskBranch: "ralphx/app/task-123",
    ...overrides,
  };
}

describe("RunningProcessPopover", () => {
  describe("basic rendering", () => {
    it("renders trigger element", () => {
      render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={3}
          open={false}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );
      expect(screen.getByText("Trigger")).toBeInTheDocument();
    });

    it("renders popover content when open", () => {
      render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={3}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );
      expect(screen.getByTestId("running-process-popover")).toBeInTheDocument();
    });

    it("does not render popover content when closed", () => {
      render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={3}
          open={false}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );
      expect(screen.queryByTestId("running-process-popover")).not.toBeInTheDocument();
    });
  });

  describe("header", () => {
    it("displays correct title with process count", () => {
      const processes = [
        createMockProcess({ taskId: "task-1" }),
        createMockProcess({ taskId: "task-2" }),
      ];
      render(
        <RunningProcessPopover
          processes={processes}
          maxConcurrent={3}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );
      expect(screen.getByText("Running Processes (2/3)")).toBeInTheDocument();
    });

    it("displays max concurrency in settings button", () => {
      render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={5}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );
      expect(screen.getByText(/Max: 5/)).toBeInTheDocument();
    });

    it("calls onOpenSettings when settings button clicked", () => {
      const onOpenSettings = vi.fn();
      render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={3}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={onOpenSettings}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );

      fireEvent.click(screen.getByTestId("open-settings-button"));
      expect(onOpenSettings).toHaveBeenCalledOnce();
    });
  });

  describe("process list", () => {
    it("displays empty state when no processes", () => {
      render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={3}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );
      expect(screen.getByText("No running processes")).toBeInTheDocument();
    });

    it("renders all processes as ProcessCard components", () => {
      const processes = [
        createMockProcess({ taskId: "task-1", title: "Task 1" }),
        createMockProcess({ taskId: "task-2", title: "Task 2" }),
        createMockProcess({ taskId: "task-3", title: "Task 3" }),
      ];
      render(
        <RunningProcessPopover
          processes={processes}
          maxConcurrent={3}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );

      expect(screen.getByTestId("process-card-task-1")).toBeInTheDocument();
      expect(screen.getByTestId("process-card-task-2")).toBeInTheDocument();
      expect(screen.getByTestId("process-card-task-3")).toBeInTheDocument();
    });

    it("passes onPauseProcess callback to ProcessCard", () => {
      const onPauseProcess = vi.fn();
      const processes = [createMockProcess({ taskId: "task-1" })];
      render(
        <RunningProcessPopover
          processes={processes}
          maxConcurrent={3}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={onPauseProcess}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );

      fireEvent.click(screen.getByTestId("pause-button-task-1"));
      expect(onPauseProcess).toHaveBeenCalledWith("task-1");
    });

    it("passes onStopProcess callback to ProcessCard", () => {
      const onStopProcess = vi.fn();
      const processes = [createMockProcess({ taskId: "task-1" })];
      render(
        <RunningProcessPopover
          processes={processes}
          maxConcurrent={3}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={onStopProcess}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );

      fireEvent.click(screen.getByTestId("stop-button-task-1"));
      expect(onStopProcess).toHaveBeenCalledWith("task-1");
    });
  });

  describe("footer", () => {
    it("displays info text with max concurrent count", () => {
      render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={5}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );
      expect(screen.getByText(/runs up to 5 tasks in parallel/)).toBeInTheDocument();
    });

    it("calls onOpenSettings when footer link clicked", () => {
      const onOpenSettings = vi.fn();
      render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={3}
          open={true}
          onOpenChange={vi.fn()}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={onOpenSettings}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );

      // Find the footer button by text content "Open Settings"
      const footerButton = screen.getByText("Open Settings");
      fireEvent.click(footerButton);
      expect(onOpenSettings).toHaveBeenCalled();
    });
  });

  describe("open/close behavior", () => {
    it("calls onOpenChange when popover state changes", () => {
      const onOpenChange = vi.fn();
      const { rerender } = render(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={3}
          open={false}
          onOpenChange={onOpenChange}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );

      // Simulate opening
      rerender(
        <RunningProcessPopover
          processes={[]}
          maxConcurrent={3}
          open={true}
          onOpenChange={onOpenChange}
          onPauseProcess={vi.fn()}
          onStopProcess={vi.fn()}
          onOpenSettings={vi.fn()}
        >
          <button>Trigger</button>
        </RunningProcessPopover>
      );

      expect(screen.getByTestId("running-process-popover")).toBeInTheDocument();
    });
  });
});
