/**
 * ExecutionControlBar component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
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
      expect(bar).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
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
  });

  describe("pause/resume icon", () => {
    it("shows pause icon when not paused", () => {
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
      expect(btn.textContent).toContain("⏸");
    });

    it("shows resume icon when paused", () => {
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
      expect(btn.textContent).toContain("▶");
    });
  });

  describe("stop button styling", () => {
    it("uses error color for stop button", () => {
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
      expect(stopBtn).toHaveStyle({ backgroundColor: "var(--status-error)" });
    });

    it("uses muted background when disabled", () => {
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
      expect(stopBtn).toHaveStyle({ backgroundColor: "var(--bg-hover)" });
    });
  });
});
