/**
 * TeamProcessGroup component tests
 *
 * Tests getTeammateDotColor (7 status→4 colors), active count logic,
 * wave data detection, and teammate rendering.
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TeamProcessGroup } from "./TeamProcessGroup";
import type { RunningProcess, TeammateSummary } from "@/api/running-processes";

function createMockTeammate(overrides?: Partial<TeammateSummary>): TeammateSummary {
  return {
    name: "worker-1",
    status: "active",
    ...overrides,
  };
}

function createMockProcess(overrides?: Partial<RunningProcess>): RunningProcess {
  return {
    taskId: "team-task-1",
    title: "Team Task",
    internalStatus: "executing",
    stepProgress: null,
    elapsedSeconds: 60,
    triggerOrigin: "scheduler",
    taskBranch: null,
    teammates: [
      createMockTeammate({ name: "lead", status: "active" }),
      createMockTeammate({ name: "researcher", status: "idle" }),
      createMockTeammate({ name: "coder", status: "completed" }),
    ],
    ...overrides,
  };
}

describe("TeamProcessGroup", () => {
  describe("getTeammateDotColor (via rendering)", () => {
    // The function maps 7+ statuses to 4 distinct colors. We verify
    // the dot color by checking the style on the rendered status dot.

    it("maps 'active' status to green", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "active" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      expect(dot).toHaveStyle({ backgroundColor: "var(--status-success)" });
    });

    it("maps 'executing' status to green", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "executing" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      expect(dot).toHaveStyle({ backgroundColor: "var(--status-success)" });
    });

    it("maps 'running' status to green", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "running" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      expect(dot).toHaveStyle({ backgroundColor: "var(--status-success)" });
    });

    it("maps 'completed' status to grey", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "completed" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      expect(dot).toHaveStyle({ backgroundColor: "var(--text-muted)" });
    });

    it("maps 'done' status to grey", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "done" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      expect(dot).toHaveStyle({ backgroundColor: "var(--text-muted)" });
    });

    it("maps 'failed' status to red", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "failed" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      expect(dot).toHaveStyle({ backgroundColor: "var(--status-error)" });
    });

    it("maps 'idle' status to yellow", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "idle" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      expect(dot).toHaveStyle({ backgroundColor: "var(--status-warning)" });
    });

    it("maps unknown status to grey (default)", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "unknown-status" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      expect(dot).toHaveStyle({ backgroundColor: "var(--text-muted)" });
    });

    it("uses teammate.color override when provided", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "active", color: "hsl(270 50% 50%)" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      const dot = screen.getByTestId("teammate-a").querySelector("span.rounded-full");
      // Custom color overrides getTeammateDotColor result
      expect(dot).toHaveStyle({ backgroundColor: "hsl(270 50% 50%)" });
    });
  });

  describe("active count", () => {
    it("excludes completed teammates from active count", () => {
      const process = createMockProcess({
        teammates: [
          createMockTeammate({ name: "a", status: "active" }),
          createMockTeammate({ name: "b", status: "completed" }),
          createMockTeammate({ name: "c", status: "idle" }),
        ],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      expect(screen.getByText("Team: 2/3")).toBeInTheDocument();
    });

    it("excludes done teammates from active count", () => {
      const process = createMockProcess({
        teammates: [
          createMockTeammate({ name: "a", status: "active" }),
          createMockTeammate({ name: "b", status: "done" }),
        ],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      expect(screen.getByText("Team: 1/2")).toBeInTheDocument();
    });

    it("shows 0 active when all completed", () => {
      const process = createMockProcess({
        teammates: [
          createMockTeammate({ name: "a", status: "completed" }),
          createMockTeammate({ name: "b", status: "done" }),
        ],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      expect(screen.getByText("Team: 0/2")).toBeInTheDocument();
    });
  });

  describe("wave data detection", () => {
    it("renders WaveGateIndicator when currentWave and totalWaves present", () => {
      const process = createMockProcess({
        currentWave: 2,
        totalWaves: 3,
        teammates: [createMockTeammate({ name: "a", status: "active", wave: 2 })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      expect(screen.getByText(/Wave 2\/3/)).toBeInTheDocument();
    });

    it("does not render WaveGateIndicator when wave data missing", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "a", status: "active" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);
      expect(screen.queryByText(/Wave/)).not.toBeInTheDocument();
    });
  });

  describe("expand/collapse", () => {
    it("toggles teammate visibility on chevron click", () => {
      const process = createMockProcess();
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);

      // Initially expanded — teammates visible
      expect(screen.getByTestId("teammate-lead")).toBeInTheDocument();

      // Click toggle to collapse
      fireEvent.click(screen.getByTestId("team-toggle-team-task-1"));
      expect(screen.queryByTestId("teammate-lead")).not.toBeInTheDocument();

      // Click toggle to expand again
      fireEvent.click(screen.getByTestId("team-toggle-team-task-1"));
      expect(screen.getByTestId("teammate-lead")).toBeInTheDocument();
    });
  });

  describe("pause/stop buttons", () => {
    it("calls onPause with taskId when pause clicked", () => {
      const onPause = vi.fn();
      const process = createMockProcess({ taskId: "t-1" });
      render(<TeamProcessGroup process={process} onPause={onPause} onStop={vi.fn()} />);
      fireEvent.click(screen.getByTestId("pause-button-t-1"));
      expect(onPause).toHaveBeenCalledWith("t-1");
    });

    it("calls onStop with taskId when stop clicked", () => {
      const onStop = vi.fn();
      const process = createMockProcess({ taskId: "t-1" });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={onStop} />);
      fireEvent.click(screen.getByTestId("stop-button-t-1"));
      expect(onStop).toHaveBeenCalledWith("t-1");
    });
  });

  describe("click-to-navigate", () => {
    it("clicking header row calls onNavigate with taskId", () => {
      const onNavigate = vi.fn();
      const process = createMockProcess({ taskId: "team-nav-1" });
      render(
        <TeamProcessGroup
          process={process}
          onPause={vi.fn()}
          onStop={vi.fn()}
          onNavigate={onNavigate}
        />
      );

      // Click the team header row (role=button div)
      const header = screen.getByTestId("team-group-team-nav-1").querySelector('[role="button"]');
      fireEvent.click(header!);

      expect(onNavigate).toHaveBeenCalledWith("team-nav-1");
      expect(onNavigate).toHaveBeenCalledOnce();
    });

    it("clicking chevron toggle does NOT call onNavigate (stopPropagation)", () => {
      const onNavigate = vi.fn();
      const process = createMockProcess({ taskId: "team-chevron-1" });
      render(
        <TeamProcessGroup
          process={process}
          onPause={vi.fn()}
          onStop={vi.fn()}
          onNavigate={onNavigate}
        />
      );

      fireEvent.click(screen.getByTestId("team-toggle-team-chevron-1"));
      expect(onNavigate).not.toHaveBeenCalled();
    });

    it("clicking pause button does NOT call onNavigate (stopPropagation)", () => {
      const onNavigate = vi.fn();
      const process = createMockProcess({ taskId: "team-sp-pause" });
      render(
        <TeamProcessGroup
          process={process}
          onPause={vi.fn()}
          onStop={vi.fn()}
          onNavigate={onNavigate}
        />
      );

      fireEvent.click(screen.getByTestId("pause-button-team-sp-pause"));
      expect(onNavigate).not.toHaveBeenCalled();
    });

    it("clicking stop button does NOT call onNavigate (stopPropagation)", () => {
      const onNavigate = vi.fn();
      const process = createMockProcess({ taskId: "team-sp-stop" });
      render(
        <TeamProcessGroup
          process={process}
          onPause={vi.fn()}
          onStop={vi.fn()}
          onNavigate={onNavigate}
        />
      );

      fireEvent.click(screen.getByTestId("stop-button-team-sp-stop"));
      expect(onNavigate).not.toHaveBeenCalled();
    });

    it("teammate rows have no role=button (not clickable)", () => {
      const process = createMockProcess({
        teammates: [createMockTeammate({ name: "worker-a", status: "active" })],
      });
      render(<TeamProcessGroup process={process} onPause={vi.fn()} onStop={vi.fn()} />);

      const teammateRow = screen.getByTestId("teammate-worker-a");
      expect(teammateRow).not.toHaveAttribute("role", "button");
      expect(teammateRow).not.toHaveAttribute("tabindex");
    });
  });
});
