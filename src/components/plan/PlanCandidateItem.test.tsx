import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { PlanCandidateItem } from "./PlanCandidateItem";
import type { PlanCandidate } from "@/stores/planStore";
import { useChatStore } from "@/stores/chatStore";

function createCandidate(overrides: Partial<PlanCandidate> = {}): PlanCandidate {
  return {
    sessionId: "session-1",
    title: "Test Plan",
    acceptedAt: "2026-01-24T12:00:00Z",
    taskStats: { total: 5, incomplete: 3, activeNow: 0 },
    interactionStats: { selectedCount: 0, lastSelectedAt: null },
    score: 0,
    ...overrides,
  };
}

const defaultProps = {
  plan: createCandidate(),
  isActive: false,
  isHighlighted: false,
  onMouseEnter: vi.fn(),
  onClick: vi.fn(),
};

describe("PlanCandidateItem", () => {
  beforeEach(() => {
    useChatStore.setState({ agentStatus: {} });
  });

  it("renders the plan title", () => {
    render(<PlanCandidateItem {...defaultProps} />);
    expect(screen.getByText("Test Plan")).toBeInTheDocument();
  });

  describe("agent status indicators", () => {
    it("no active work: shows no indicator text", () => {
      render(<PlanCandidateItem {...defaultProps} />);
      expect(screen.queryByText(/Active work/)).not.toBeInTheDocument();
      expect(screen.queryByText(/Session active/)).not.toBeInTheDocument();
      expect(screen.queryByText(/Awaiting input/)).not.toBeInTheDocument();
    });

    it("task execution active only: shows Active work indicator", () => {
      render(
        <PlanCandidateItem
          {...defaultProps}
          plan={createCandidate({ taskStats: { total: 5, incomplete: 2, activeNow: 2 } })}
        />
      );
      expect(screen.getByText(/Active work/)).toBeInTheDocument();
      expect(screen.queryByText(/Session active/)).not.toBeInTheDocument();
    });

    it("ideation active only: shows Session active indicator", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      render(<PlanCandidateItem {...defaultProps} />);
      expect(screen.getByText(/Session active/)).toBeInTheDocument();
      expect(screen.queryByText(/Active work/)).not.toBeInTheDocument();
    });

    it("ideation waiting only: shows Awaiting input indicator", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "waiting_for_input" } });
      render(<PlanCandidateItem {...defaultProps} />);
      expect(screen.getByText(/Awaiting input/)).toBeInTheDocument();
    });

    it("both active: shows Active work and Session active", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      render(
        <PlanCandidateItem
          {...defaultProps}
          plan={createCandidate({ taskStats: { total: 5, incomplete: 2, activeNow: 1 } })}
        />
      );
      expect(screen.getByText(/Active work/)).toBeInTheDocument();
      expect(screen.getByText(/Session active/)).toBeInTheDocument();
    });

    it("no tasks + ideation active: shows standalone Session active (no bullet)", () => {
      useChatStore.setState({ agentStatus: { "session:session-1": "generating" } });
      render(
        <PlanCandidateItem
          {...defaultProps}
          plan={createCandidate({ taskStats: { total: 0, incomplete: 0, activeNow: 0 } })}
        />
      );
      expect(screen.getByText("Session active")).toBeInTheDocument();
    });
  });
});
