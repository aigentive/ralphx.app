/**
 * ExecutionControlBar component tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import type { ReactNode } from "react";
import { ExecutionControlBar } from "./ExecutionControlBar";

vi.mock("./RunningProcessPopover", () => ({
  RunningProcessPopover: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

vi.mock("./QueuedTasksPopover", () => ({
  QueuedTasksPopover: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

vi.mock("./MergePipelinePopover", () => ({
  MergePipelinePopover: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

describe("ExecutionControlBar", () => {
  describe("basic rendering", () => {
    it("renders with data-testid", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("execution-control-bar")).toBeInTheDocument();
    });

    it("displays running tasks count", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("running-count")).toHaveTextContent(/(Running|R): 1\/10/);
    });

    it("displays queued tasks count", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={5}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("queued-count")).toHaveTextContent(/(Queued|Q): 5/);
    });

    it("includes queued agent messages in the status region label", () => {
      render(
        <ExecutionControlBar
          runningCount={2}
          maxConcurrent={10}
          queuedCount={5}
          queuedMessageCount={3}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByLabelText(/3 queued messages/)).toBeInTheDocument();
    });

    it("shows an inline queued-message warning badge when pressure exists", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={2}
          queuedMessageCount={4}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );

      expect(screen.getByTestId("queued-message-count")).toHaveTextContent(/Msg[s]?:\s*4/);
    });

    it("hides the queued-message warning badge when no messages are held", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={2}
          queuedMessageCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );

      expect(screen.queryByTestId("queued-message-count")).not.toBeInTheDocument();
    });

  });

  describe("pause button", () => {
    it("renders pause button when not paused", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveTextContent("Pause");
    });

    it("renders resume button when paused", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={true}
          haltMode="paused"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveTextContent("Resume");
    });

    it("renders disabled stopped button after stop", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={true}
          haltMode="stopped"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveTextContent("Stopped");
      expect(pauseBtn).toBeDisabled();
    });

    it("calls onPauseToggle when clicked", () => {
      const onPauseToggle = vi.fn();
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={false}
          onPauseToggle={onPauseToggle}
          onStop={vi.fn()}
        />
      );
      fireEvent.click(screen.getByTestId("pause-toggle-button"));
      expect(onPauseToggle).toHaveBeenCalledOnce();
    });

    it("disables pause button when isLoading", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={false}
          isLoading={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("pause-toggle-button")).toBeDisabled();
    });
  });

  describe("stop button", () => {
    it("renders stop button", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("stop-button")).toHaveTextContent("Stop");
    });

    it("calls onStop when clicked", () => {
      const onStop = vi.fn();
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={onStop}
        />
      );
      fireEvent.click(screen.getByTestId("stop-button"));
      expect(onStop).toHaveBeenCalledOnce();
    });

    it("disables stop button when no running tasks", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("stop-button")).toBeDisabled();
    });

    it("uses stopped aria label after a global stop", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={true}
          haltMode="stopped"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("stop-button")).toHaveAttribute(
        "aria-label",
        "Execution already stopped"
      );
    });

    it("enables stop button when there are running tasks", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("stop-button")).not.toBeDisabled();
    });

    it("disables stop button when isLoading", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={false}
          isLoading={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("stop-button")).toBeDisabled();
    });
  });

  describe("data attributes", () => {
    it("sets data-paused attribute", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-paused", "true");
    });

    it("sets data-running attribute", () => {
      render(
        <ExecutionControlBar
          runningCount={2}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-running", "2");
    });

    it("sets data-loading attribute when loading", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          isLoading={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-loading", "true");
    });

    it("sets data-status attribute", () => {
      const { rerender } = render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-status", "running");

      rerender(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("execution-control-bar")).toHaveAttribute("data-status", "paused");

      rerender(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
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
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar).toHaveStyle({ background: "hsla(220 10% 10% / 0.92)" });
    });

    it("applies subtle border styling", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar.style.borderWidth).toBe("1px");
      expect(bar.style.borderStyle).toBe("solid");
      expect(bar.style.borderColor).toBe("rgba(255, 255, 255, 0.08)");
    });

    it("applies box shadow for elevation", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar.style.boxShadow).toContain("0 4px 16px");
      expect(bar.style.boxShadow).toContain("0 12px 32px");
    });
  });

  describe("status indicator colors", () => {
    it("shows success color when running tasks exist", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "hsl(14 100% 55%)" });
    });

    it("shows warning color when paused", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "hsl(45 90% 55%)" });
    });

    it("shows stopped color when execution is globally stopped", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={true}
          haltMode="stopped"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "hsl(0 70% 55%)" });
    });

    it("shows muted color when idle with no queued", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "hsl(220 10% 55%)" });
    });

    it("has pulsing animation class when running", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).toHaveClass("status-indicator-running");
    });

    it("does not have pulsing animation when paused", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).not.toHaveClass("status-indicator-running");
    });
  });

  describe("pause/resume button icons", () => {
    it("shows Pause icon when not paused", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const btn = screen.getByTestId("pause-toggle-button");
      // Check for Lucide Pause icon (SVG)
      const svg = btn.querySelector("svg");
      expect(svg).toBeInTheDocument();
    });

    it("shows Play icon when paused", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const btn = screen.getByTestId("pause-toggle-button");
      // Check for Lucide Play icon (SVG)
      const svg = btn.querySelector("svg");
      expect(svg).toBeInTheDocument();
    });

    it("shows Loader2 spinner when loading", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          isLoading={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const btn = screen.getByTestId("pause-toggle-button");
      const svg = btn.querySelector("svg");
      expect(svg).toBeInTheDocument();
      expect(svg).toHaveClass("animate-spin");
    });
  });

  describe("stop button styling", () => {
    it("has error styling when can stop", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const stopBtn = screen.getByTestId("stop-button");
      expect(stopBtn).toHaveStyle({ backgroundColor: "hsla(0 70% 55% / 0.15)" });
      expect(stopBtn).toHaveStyle({ color: "hsl(0 70% 55%)" });
      expect(stopBtn).toHaveStyle({ opacity: "1" });
    });

    it("has muted styling when disabled", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const stopBtn = screen.getByTestId("stop-button");
      expect(stopBtn).toHaveStyle({ backgroundColor: "hsl(220 10% 18%)" });
      expect(stopBtn).toHaveStyle({ color: "hsl(220 10% 45%)" });
      expect(stopBtn).toHaveStyle({ opacity: "0.5" });
    });

    it("has Square icon", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const stopBtn = screen.getByTestId("stop-button");
      const svg = stopBtn.querySelector("svg");
      expect(svg).toBeInTheDocument();
      expect(svg).toHaveClass("fill-current");
    });
  });

  describe("pause button styling", () => {
    it("has accent styling when paused", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveStyle({ backgroundColor: "hsla(45 90% 55% / 0.15)" });
      expect(pauseBtn).toHaveStyle({ color: "hsl(45 90% 55%)" });
    });

    it("has default styling when not paused", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveStyle({ color: "hsl(220 10% 90%)" });
    });
  });

  describe("current task display", () => {
    it("shows current task name when running", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          currentTaskName="Implementing auth feature"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("current-task")).toBeInTheDocument();
      expect(screen.getByTestId("current-task")).toHaveTextContent("Implementing auth feature");
    });

    it("does not show current task when paused", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={3}
          isPaused={true}
          currentTaskName="Implementing auth feature"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.queryByTestId("current-task")).not.toBeInTheDocument();
    });

    it("does not show current task when no tasks running", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          currentTaskName="Implementing auth feature"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.queryByTestId("current-task")).not.toBeInTheDocument();
    });

    it("does not show current task when no task name provided", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.queryByTestId("current-task")).not.toBeInTheDocument();
    });

    it("has spinner icon with current task", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          currentTaskName="Building components"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const taskDisplay = screen.getByTestId("current-task");
      const svg = taskDisplay.querySelector("svg");
      expect(svg).toBeInTheDocument();
      expect(svg).toHaveClass("animate-spin");
    });

    it("has slide-in animation class", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          currentTaskName="Building components"
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const taskDisplay = screen.getByTestId("current-task");
      expect(taskDisplay).toHaveClass("task-name-enter");
    });
  });

  describe("accessibility", () => {
    it("has role region", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar).toHaveAttribute("role", "region");
    });

    it("has aria-live for status updates", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar).toHaveAttribute("aria-live", "polite");
    });

    it("pause button has aria-label", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveAttribute("aria-label", "Pause execution");
    });

    it("pause button has aria-pressed when paused", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveAttribute("aria-pressed", "true");
    });

    it("stop button has aria-label", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const stopBtn = screen.getByTestId("stop-button");
      expect(stopBtn).toHaveAttribute("aria-label", "Stop all running tasks");
    });
  });

  describe("ideation capacity indicator", () => {
    it("shows ideation indicator when ideationMax > 0", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          ideationActive={1}
          ideationMax={2}
          ideationWaiting={0}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("ideation-count")).toBeInTheDocument();
      expect(screen.getByTestId("ideation-count")).toHaveTextContent(/1\/2/);
    });

    it("hides ideation indicator when ideationMax is 0", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          ideationActive={0}
          ideationMax={0}
          ideationWaiting={0}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.queryByTestId("ideation-count")).not.toBeInTheDocument();
    });

    it("hides ideation indicator when ideationMax is not provided", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.queryByTestId("ideation-count")).not.toBeInTheDocument();
    });

    it("shows waiting badge when ideationWaiting > 0", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          ideationActive={2}
          ideationMax={2}
          ideationWaiting={3}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("ideation-waiting-badge")).toBeInTheDocument();
      expect(screen.getByTestId("ideation-waiting-badge")).toHaveTextContent("+3");
    });

    it("hides waiting badge when ideationWaiting is 0", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          ideationActive={1}
          ideationMax={2}
          ideationWaiting={0}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.queryByTestId("ideation-waiting-badge")).not.toBeInTheDocument();
    });

    it("shows 0/N when no active ideation sessions", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={10}
          queuedCount={0}
          isPaused={false}
          ideationActive={0}
          ideationMax={4}
          ideationWaiting={0}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("ideation-count")).toHaveTextContent(/0\/4/);
    });
  });
});
