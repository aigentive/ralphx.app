/**
 * ExecutionControlBar component tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ExecutionControlBar } from "./ExecutionControlBar";

describe("ExecutionControlBar", () => {
  describe("basic rendering", () => {
    it("renders with data-testid", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
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
          maxConcurrent={2}
          queuedCount={3}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("running-count")).toHaveTextContent("Running: 1/2");
    });

    it("displays queued tasks count", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={2}
          queuedCount={5}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("queued-count")).toHaveTextContent("Queued: 5");
    });
  });

  describe("pause button", () => {
    it("renders pause button when not paused", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
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
          maxConcurrent={2}
          queuedCount={3}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveTextContent("Resume");
    });

    it("calls onPauseToggle when clicked", () => {
      const onPauseToggle = vi.fn();
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      expect(screen.getByTestId("stop-button")).toBeDisabled();
    });

    it("enables stop button when there are running tasks", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
    it("applies design system background color", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("applies border styling from design system", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar.style.borderColor).toBe("var(--border-subtle)");
    });

    it("applies box shadow for elevation", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const bar = screen.getByTestId("execution-control-bar");
      expect(bar.style.boxShadow).toBe("0 -2px 8px rgba(0,0,0,0.15)");
    });
  });

  describe("status indicator colors", () => {
    it("shows success color when running tasks exist", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "var(--status-success)" });
    });

    it("shows warning color when paused", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={2}
          queuedCount={3}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "var(--status-warning)" });
    });

    it("shows muted color when idle with no queued", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const indicator = screen.getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "var(--text-muted)" });
    });

    it("has pulsing animation class when running", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const stopBtn = screen.getByTestId("stop-button");
      // Check for error color class (using Tailwind classes now)
      expect(stopBtn).toHaveClass("text-[var(--status-error)]");
    });

    it("has muted styling when disabled", () => {
      render(
        <ExecutionControlBar
          runningCount={0}
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const stopBtn = screen.getByTestId("stop-button");
      // Check for muted color class when disabled
      expect(stopBtn).toHaveClass("text-[var(--text-muted)]");
      expect(stopBtn).toHaveClass("opacity-50");
    });

    it("has Square icon", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
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
          maxConcurrent={2}
          queuedCount={3}
          isPaused={true}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveClass("text-[var(--accent-primary)]");
      expect(pauseBtn).toHaveClass("bg-[var(--accent-muted)]");
    });

    it("has default styling when not paused", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
          queuedCount={0}
          isPaused={false}
          onPauseToggle={vi.fn()}
          onStop={vi.fn()}
        />
      );
      const pauseBtn = screen.getByTestId("pause-toggle-button");
      expect(pauseBtn).toHaveClass("text-[var(--text-primary)]");
    });
  });

  describe("current task display", () => {
    it("shows current task name when running", () => {
      render(
        <ExecutionControlBar
          runningCount={1}
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
          maxConcurrent={2}
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
});
