/**
 * ExecutionControlBar component tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import type { ComponentProps, ReactNode } from "react";
import { ExecutionControlBar } from "./ExecutionControlBar";

vi.mock("./RunningProcessPopover", () => ({
  RunningProcessPopover: ({
    children,
    initialTab,
    open,
    showIdeation,
    ideationMax,
  }: {
    children: ReactNode;
    initialTab?: string;
    open?: boolean;
    showIdeation?: boolean;
    ideationMax?: number;
    [key: string]: unknown;
  }) => (
    <div
      data-testid="mock-running-popover"
      data-initial-tab={initialTab}
      data-open={String(open ?? false)}
      data-show-ideation={String(showIdeation ?? false)}
      data-ideation-max={ideationMax}
    >
      {children}
    </div>
  ),
}));

vi.mock("./QueuedTasksPopover", () => ({
  QueuedTasksPopover: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

vi.mock("./MergePipelinePopover", () => ({
  MergePipelinePopover: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

// Helper: renders ExecutionControlBar with all required props, accepting overrides
function renderBar(
  overrides: Partial<ComponentProps<typeof ExecutionControlBar>> = {}
) {
  return render(
    <ExecutionControlBar
      projectId="proj-1"
      runningCount={0}
      maxConcurrent={10}
      queuedCount={0}
      mergingCount={0}
      hasAttentionMerges={false}
      mergePipelineData={null}
      isPaused={false}
      onPauseToggle={vi.fn()}
      onStop={vi.fn()}
      {...overrides}
    />
  );
}

describe("ExecutionControlBar", () => {
  describe("basic rendering", () => {
    it("renders with data-testid", () => {
      renderBar({ runningCount: 1, queuedCount: 3 });
      expect(screen.getByTestId("execution-control-bar")).toBeInTheDocument();
    });

    it("displays running tasks count", () => {
      renderBar({ runningCount: 1, queuedCount: 3 });
      expect(screen.getByTestId("running-count")).toHaveTextContent(/(Running|R): 1\/10/);
    });

    it("displays queued tasks count", () => {
      renderBar({ queuedCount: 5 });
      expect(screen.getByTestId("queued-count")).toHaveTextContent(/(Queued|Q): 5/);
    });

    it("includes queued agent messages in the status region label", () => {
      renderBar({ runningCount: 2, queuedCount: 5, queuedMessageCount: 3 });
      expect(screen.getByLabelText(/3 queued messages/)).toBeInTheDocument();
    });

    it("shows an inline queued-message warning badge when pressure exists", () => {
      renderBar({ runningCount: 1, queuedCount: 2, queuedMessageCount: 4 });
      expect(screen.getByTestId("queued-message-count")).toHaveTextContent(/Msg[s]?:\s*4/);
    });

    it("hides the queued-message warning badge when no messages are held", () => {
      renderBar({ runningCount: 1, queuedCount: 2, queuedMessageCount: 0 });
      expect(screen.queryByTestId("queued-message-count")).not.toBeInTheDocument();
    });
  });

  describe("pause button", () => {
    it("renders pause button when not paused", () => {
      renderBar({ runningCount: 1, queuedCount: 3 });
      expect(screen.getByTestId("pause-toggle-button")).toHaveTextContent("Pause");
    });

    it("renders resume button when paused", () => {
      renderBar({ queuedCount: 3, isPaused: true, haltMode: "paused" });
      expect(screen.getByTestId("pause-toggle-button")).toHaveTextContent("Resume");
    });

    it("renders disabled stopped button after stop", () => {
      renderBar({ isPaused: true, haltMode: "stopped" });
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveTextContent("Stopped");
      expect(pauseBtn).toBeDisabled();
    });

    it("calls onPauseToggle when clicked", () => {
      const onPauseToggle = vi.fn();
      renderBar({ runningCount: 1, queuedCount: 3, onPauseToggle });
      fireEvent.click(screen.getByTestId("pause-toggle-button"));
      expect(onPauseToggle).toHaveBeenCalledOnce();
    });

    it("disables pause button when isLoading", () => {
      renderBar({ runningCount: 1, queuedCount: 3, isLoading: true });
      expect(screen.getByTestId("pause-toggle-button")).toBeDisabled();
    });
  });

  describe("stop button", () => {
    it("renders stop button", () => {
      renderBar({ runningCount: 1, queuedCount: 3 });
      expect(screen.getByTestId("stop-button")).toHaveTextContent("Stop");
    });

    it("calls onStop when clicked", () => {
      const onStop = vi.fn();
      renderBar({ runningCount: 1, queuedCount: 3, onStop });
      fireEvent.click(screen.getByTestId("stop-button"));
      expect(onStop).toHaveBeenCalledOnce();
    });

    it("disables stop button when no running tasks", () => {
      renderBar();
      expect(screen.getByTestId("stop-button")).toBeDisabled();
    });

    it("uses stopped aria label after a global stop", () => {
      renderBar({ isPaused: true, haltMode: "stopped" });
      expect(screen.getByTestId("stop-button")).toHaveAttribute(
        "aria-label",
        "Execution already stopped"
      );
    });

    it("enables stop button when there are running tasks", () => {
      renderBar({ runningCount: 1 });
      expect(screen.getByTestId("stop-button")).not.toBeDisabled();
    });

    it("disables stop button when isLoading", () => {
      renderBar({ runningCount: 1, queuedCount: 3, isLoading: true });
      expect(screen.getByTestId("stop-button")).toBeDisabled();
    });
  });

  describe("data attributes", () => {
    it("sets data-paused attribute", () => {
      renderBar({ isPaused: true });
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-paused", "true");
    });

    it("sets data-running attribute", () => {
      renderBar({ runningCount: 2 });
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-running", "2");
    });

    it("sets data-loading attribute when loading", () => {
      renderBar({ isLoading: true });
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-loading", "true");
    });

    it("sets data-status attribute", () => {
      const { rerender } = renderBar({ runningCount: 1 });
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-status", "running");

      rerender(
        <ExecutionControlBar
          projectId="proj-1"
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          mergingCount={0}
          hasAttentionMerges={false}
          mergePipelineData={null}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-status", "paused");

      rerender(
        <ExecutionControlBar
          projectId="proj-1"
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          mergingCount={0}
          hasAttentionMerges={false}
          mergePipelineData={null}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-status", "idle");
    });
  });

  describe("styling", () => {
    it("applies floating glass background style", () => {
      renderBar();
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar).toHaveStyle({ background: "hsla(220 10% 10% / 0.92)" });
    });

    it("applies subtle border styling", () => {
      renderBar();
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar.style.borderWidth).toBe("1px");
      expect(bar.style.borderStyle).toBe("solid");
      expect(bar.style.borderColor).toBe("rgba(255, 255, 255, 0.08)");
    });

    it("applies box shadow for elevation", () => {
      renderBar();
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar.style.boxShadow).toContain("0 4px 16px");
      expect(bar.style.boxShadow).toContain("0 12px 32px");
    });
  });

  describe("status indicator colors", () => {
    it("shows success color when running tasks exist", () => {
      renderBar({ runningCount: 1 });
      expect(screen.getByTestId("status-indicator")).toHaveStyle({ backgroundColor: "hsl(14 100% 55%)" });
    });

    it("shows warning color when paused", () => {
      renderBar({ queuedCount: 3, isPaused: true });
      expect(screen.getByTestId("status-indicator")).toHaveStyle({ backgroundColor: "hsl(45 90% 55%)" });
    });

    it("shows stopped color when execution is globally stopped", () => {
      renderBar({ isPaused: true, haltMode: "stopped" });
      expect(screen.getByTestId("status-indicator")).toHaveStyle({ backgroundColor: "hsl(0 70% 55%)" });
    });

    it("shows muted color when idle with no queued", () => {
      renderBar();
      expect(screen.getByTestId("status-indicator")).toHaveStyle({ backgroundColor: "hsl(220 10% 55%)" });
    });

    it("has pulsing animation class when running", () => {
      renderBar({ runningCount: 1 });
      expect(screen.getByTestId("status-indicator")).toHaveClass("status-indicator-running");
    });

    it("does not have pulsing animation when paused", () => {
      renderBar({ isPaused: true });
      expect(screen.getByTestId("status-indicator")).not.toHaveClass("status-indicator-running");
    });
  });

  describe("pause/resume button icons", () => {
    it("shows Pause icon when not paused", () => {
      renderBar({ runningCount: 1 });
      const btn = screen.getByTestId("pause-toggle-button");
      expect(btn.querySelector("svg")).toBeInTheDocument();
    });

    it("shows Play icon when paused", () => {
      renderBar({ isPaused: true });
      const btn = screen.getByTestId("pause-toggle-button");
      expect(btn.querySelector("svg")).toBeInTheDocument();
    });

    it("shows Loader2 spinner when loading", () => {
      renderBar({ runningCount: 1, isLoading: true });
      const btn = screen.getByTestId("pause-toggle-button");
      const svg = btn.querySelector("svg");
      expect(svg).toBeInTheDocument();
      expect(svg).toHaveClass("animate-spin");
    });
  });

  describe("stop button styling", () => {
    it("has error styling when can stop", () => {
      renderBar({ runningCount: 1 });
      const stopBtn = screen.getByTestId("stop-button");
      expect(stopBtn).toHaveStyle({ backgroundColor: "hsla(0 70% 55% / 0.15)" });
      expect(stopBtn).toHaveStyle({ color: "hsl(0 70% 55%)" });
      expect(stopBtn).toHaveStyle({ opacity: "1" });
    });

    it("has muted styling when disabled", () => {
      renderBar();
      const stopBtn = screen.getByTestId("stop-button");
      expect(stopBtn).toHaveStyle({ backgroundColor: "hsl(220 10% 18%)" });
      expect(stopBtn).toHaveStyle({ color: "hsl(220 10% 45%)" });
      expect(stopBtn).toHaveStyle({ opacity: "0.5" });
    });

    it("has Square icon", () => {
      renderBar({ runningCount: 1 });
      const stopBtn = screen.getByTestId("stop-button");
      const svg = stopBtn.querySelector("svg");
      expect(svg).toBeInTheDocument();
      expect(svg).toHaveClass("fill-current");
    });
  });

  describe("pause button styling", () => {
    it("has accent styling when paused", () => {
      renderBar({ queuedCount: 3, isPaused: true });
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveStyle({ backgroundColor: "hsla(45 90% 55% / 0.15)" });
      expect(pauseBtn).toHaveStyle({ color: "hsl(45 90% 55%)" });
    });

    it("has default styling when not paused", () => {
      renderBar({ runningCount: 1 });
      expect(screen.getByTestId("pause-toggle-button")).toHaveStyle({ color: "hsl(220 10% 90%)" });
    });
  });

  describe("current task display", () => {
    it("shows current task name when running", () => {
      renderBar({ runningCount: 1, currentTaskName: "Implementing auth feature" });
      expect(screen.getByTestId("current-task")).toBeInTheDocument();
      expect(screen.getByTestId("current-task")).toHaveTextContent("Implementing auth feature");
    });

    it("does not show current task when paused", () => {
      renderBar({ queuedCount: 3, isPaused: true, currentTaskName: "Implementing auth feature" });
      expect(screen.queryByTestId("current-task")).not.toBeInTheDocument();
    });

    it("does not show current task when no tasks running", () => {
      renderBar({ currentTaskName: "Implementing auth feature" });
      expect(screen.queryByTestId("current-task")).not.toBeInTheDocument();
    });

    it("does not show current task when no task name provided", () => {
      renderBar({ runningCount: 1 });
      expect(screen.queryByTestId("current-task")).not.toBeInTheDocument();
    });

    it("has spinner icon with current task", () => {
      renderBar({ runningCount: 1, currentTaskName: "Building components" });
      const taskDisplay = screen.getByTestId("current-task");
      const svg = taskDisplay.querySelector("svg");
      expect(svg).toBeInTheDocument();
      expect(svg).toHaveClass("animate-spin");
    });

    it("has slide-in animation class", () => {
      renderBar({ runningCount: 1, currentTaskName: "Building components" });
      expect(screen.getByTestId("current-task")).toHaveClass("task-name-enter");
    });
  });

  describe("accessibility", () => {
    it("has role region", () => {
      renderBar();
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("role", "region");
    });

    it("has aria-live for status updates", () => {
      renderBar();
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("aria-live", "polite");
    });

    it("pause button has aria-label", () => {
      renderBar({ runningCount: 1 });
      expect(screen.getByTestId("pause-toggle-button")).toHaveAttribute("aria-label", "Pause execution");
    });

    it("pause button has aria-pressed when paused", () => {
      renderBar({ isPaused: true });
      expect(screen.getByTestId("pause-toggle-button")).toHaveAttribute("aria-pressed", "true");
    });

    it("stop button has aria-label", () => {
      renderBar({ runningCount: 1 });
      expect(screen.getByTestId("stop-button")).toHaveAttribute("aria-label", "Stop all running tasks");
    });
  });

  describe("ideation capacity indicator", () => {
    it("shows ideation indicator when ideationMax > 0", () => {
      renderBar({ ideationActive: 1, ideationMax: 2, ideationWaiting: 0 });
      expect(screen.getByTestId("ideation-count")).toBeInTheDocument();
      expect(screen.getByTestId("ideation-count")).toHaveTextContent(/1\/2/);
    });

    it("hides ideation indicator when ideationMax is 0", () => {
      renderBar({ ideationActive: 0, ideationMax: 0, ideationWaiting: 0 });
      expect(screen.queryByTestId("ideation-count")).not.toBeInTheDocument();
    });

    it("hides ideation indicator when ideationMax is not provided", () => {
      renderBar();
      expect(screen.queryByTestId("ideation-count")).not.toBeInTheDocument();
    });

    it("shows waiting badge when ideationWaiting > 0", () => {
      renderBar({ ideationActive: 2, ideationMax: 2, ideationWaiting: 3 });
      expect(screen.getByTestId("ideation-waiting-badge")).toBeInTheDocument();
      expect(screen.getByTestId("ideation-waiting-badge")).toHaveTextContent("+3");
    });

    it("hides waiting badge when ideationWaiting is 0", () => {
      renderBar({ ideationActive: 1, ideationMax: 2, ideationWaiting: 0 });
      expect(screen.queryByTestId("ideation-waiting-badge")).not.toBeInTheDocument();
    });

    it("shows 0/N when no active ideation sessions", () => {
      renderBar({ ideationActive: 0, ideationMax: 4, ideationWaiting: 0 });
      expect(screen.getByTestId("ideation-count")).toHaveTextContent(/0\/4/);
    });
  });

  describe("tab selection", () => {
    it("clicking running-count button passes initialTab='execution' to RunningProcessPopover", () => {
      renderBar({ runningCount: 2, ideationMax: 2 });
      fireEvent.click(screen.getByTestId("running-count"));
      const popover = screen.getByTestId("mock-running-popover");
      expect(popover).toHaveAttribute("data-initial-tab", "execution");
      expect(popover).toHaveAttribute("data-open", "true");
    });

    it("clicking ideation-count button passes initialTab='ideation' to RunningProcessPopover", () => {
      renderBar({ ideationActive: 1, ideationMax: 2 });
      fireEvent.click(screen.getByTestId("ideation-count"));
      const popover = screen.getByTestId("mock-running-popover");
      expect(popover).toHaveAttribute("data-initial-tab", "ideation");
      expect(popover).toHaveAttribute("data-open", "true");
    });

    it("RunningProcessPopover receives showIdeation=true when ideationMax > 0", () => {
      renderBar({ ideationMax: 3 });
      expect(screen.getByTestId("mock-running-popover")).toHaveAttribute("data-show-ideation", "true");
    });

    it("RunningProcessPopover receives showIdeation=false when ideationMax is 0", () => {
      renderBar();
      expect(screen.getByTestId("mock-running-popover")).toHaveAttribute("data-show-ideation", "false");
    });
  });
});
