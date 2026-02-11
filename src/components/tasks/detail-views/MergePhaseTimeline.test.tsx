/**
 * MergePhaseTimeline component tests
 * Verifies phase rendering, status indicators, and message display
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MergePhaseTimeline } from "./MergePhaseTimeline";
import type { MergeProgressEvent } from "@/types/events";

function makePhase(
  overrides: Partial<MergeProgressEvent> = {}
): MergeProgressEvent {
  return {
    task_id: "task-123",
    phase: "worktree_setup",
    status: "started",
    message: "",
    timestamp: "2026-02-11T10:00:00Z",
    ...overrides,
  };
}

describe("MergePhaseTimeline", () => {
  it("renders nothing when phases array is empty", () => {
    const { container } = render(<MergePhaseTimeline phases={[]} />);
    expect(container.innerHTML).toBe("");
  });

  it("renders timeline container with testid", () => {
    render(<MergePhaseTimeline phases={[makePhase()]} />);
    expect(screen.getByTestId("merge-phase-timeline")).toBeInTheDocument();
  });

  it("shows 'Merge Progress' section title", () => {
    render(<MergePhaseTimeline phases={[makePhase()]} />);
    expect(screen.getByText("Merge Progress")).toBeInTheDocument();
  });

  it("renders phase label for a single started phase", () => {
    render(
      <MergePhaseTimeline
        phases={[makePhase({ phase: "worktree_setup", status: "started" })]}
      />
    );
    expect(screen.getByText("Worktree Setup")).toBeInTheDocument();
  });

  it("renders multiple phase labels in order", () => {
    const phases = [
      makePhase({ phase: "worktree_setup", status: "passed" }),
      makePhase({ phase: "programmatic_merge", status: "passed" }),
      makePhase({ phase: "typecheck", status: "started" }),
    ];
    render(<MergePhaseTimeline phases={phases} />);

    expect(screen.getByText("Worktree Setup")).toBeInTheDocument();
    expect(screen.getByText("Merge")).toBeInTheDocument();
    expect(screen.getByText("Type Check")).toBeInTheDocument();
  });

  it("shows pending phases between received phases based on PHASE_CONFIG order", () => {
    // If we receive worktree_setup and typecheck, phases in between
    // (programmatic_merge) should appear as pending
    const phases = [
      makePhase({ phase: "worktree_setup", status: "passed" }),
      makePhase({ phase: "typecheck", status: "started" }),
    ];
    render(<MergePhaseTimeline phases={phases} />);

    // programmatic_merge is between them in PHASE_CONFIG, should show as pending
    expect(screen.getByText("Merge")).toBeInTheDocument();
  });

  it("shows message for started phase", () => {
    render(
      <MergePhaseTimeline
        phases={[
          makePhase({
            phase: "typecheck",
            status: "started",
            message: "Running type checker...",
          }),
        ]}
      />
    );
    expect(screen.getByText("Running type checker...")).toBeInTheDocument();
  });

  it("shows message for failed phase", () => {
    render(
      <MergePhaseTimeline
        phases={[
          makePhase({
            phase: "lint",
            status: "failed",
            message: "Lint errors found",
          }),
        ]}
      />
    );
    expect(screen.getByText("Lint errors found")).toBeInTheDocument();
  });

  it("does not show message for passed phase", () => {
    render(
      <MergePhaseTimeline
        phases={[
          makePhase({
            phase: "typecheck",
            status: "passed",
            message: "Typecheck passed",
          }),
        ]}
      />
    );
    // Message should NOT appear for passed phases
    expect(screen.queryByText("Typecheck passed")).not.toBeInTheDocument();
  });

  it("renders full phase sequence with correct labels", () => {
    const phases = [
      makePhase({ phase: "worktree_setup", status: "passed" }),
      makePhase({ phase: "programmatic_merge", status: "passed" }),
      makePhase({ phase: "typecheck", status: "passed" }),
      makePhase({ phase: "lint", status: "passed" }),
      makePhase({ phase: "clippy", status: "passed" }),
      makePhase({ phase: "test", status: "passed" }),
      makePhase({ phase: "finalize", status: "started" }),
    ];
    render(<MergePhaseTimeline phases={phases} />);

    expect(screen.getByText("Worktree Setup")).toBeInTheDocument();
    expect(screen.getByText("Merge")).toBeInTheDocument();
    expect(screen.getByText("Type Check")).toBeInTheDocument();
    expect(screen.getByText("Lint")).toBeInTheDocument();
    expect(screen.getByText("Clippy")).toBeInTheDocument();
    expect(screen.getByText("Test")).toBeInTheDocument();
    expect(screen.getByText("Finalize")).toBeInTheDocument();
  });

  it("displays (live) indicator", () => {
    render(<MergePhaseTimeline phases={[makePhase()]} />);
    expect(screen.getByText("(live)")).toBeInTheDocument();
  });
});
