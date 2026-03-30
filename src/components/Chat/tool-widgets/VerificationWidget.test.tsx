import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { VerificationWidget } from "./VerificationWidget";
import type { ToolCall } from "./shared.constants";

function makeToolCall(name: string, overrides: Partial<ToolCall> = {}): ToolCall {
  return {
    id: "verification-1",
    name,
    arguments: {},
    ...overrides,
  };
}

function mcpWrap(obj: unknown): unknown {
  return [{ type: "text", text: JSON.stringify(obj) }];
}

describe("VerificationWidget", () => {
  describe("UpdateVerification (update_plan_verification)", () => {
    it("shows loading state when result has no status", () => {
      const toolCall = makeToolCall("mcp__ralphx__update_plan_verification", {
        result: mcpWrap({}),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Updating verification...")).toBeInTheDocument();
    });

    it("renders status badge and round info", () => {
      const toolCall = makeToolCall("mcp__ralphx__update_plan_verification", {
        result: mcpWrap({
          status: "reviewing",
          current_round: 2,
          max_rounds: 5,
          current_gaps: [],
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("reviewing")).toBeInTheDocument();
      expect(screen.getByText("Round 2/5")).toBeInTheDocument();
    });

    it("renders gap count badge when gaps present", () => {
      const toolCall = makeToolCall("mcp__ralphx__update_plan_verification", {
        result: mcpWrap({
          status: "needs_revision",
          current_round: 1,
          max_rounds: 5,
          current_gaps: [
            { severity: "high", category: "auth", description: "Missing check" },
            { severity: "low", category: "perf", description: "Slow query" },
          ],
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("2 gaps")).toBeInTheDocument();
    });

    it("renders convergence badge when reason present", () => {
      const toolCall = makeToolCall("mcp__ralphx__update_plan_verification", {
        result: mcpWrap({
          status: "verified",
          current_round: 3,
          max_rounds: 5,
          current_gaps: [],
          convergence_reason: "zero_blocking",
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("All gaps resolved")).toBeInTheDocument();
    });
  });

  describe("GetVerification (get_plan_verification)", () => {
    it("shows loading state when result has no status", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({}),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Loading verification...")).toBeInTheDocument();
    });

    it("renders status badge for unverified", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({ status: "unverified" }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("unverified")).toBeInTheDocument();
    });

    it("renders round info when in_progress", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({
          status: "reviewing",
          in_progress: true,
          current_round: 2,
          max_rounds: 5,
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("reviewing")).toBeInTheDocument();
      expect(screen.getByText("Round 2/5")).toBeInTheDocument();
    });

    it("renders convergence label when verified", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({
          status: "verified",
          convergence_reason: "jaccard_converged",
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("verified")).toBeInTheDocument();
      expect(screen.getByText("Gaps converged")).toBeInTheDocument();
    });
  });

  describe("ChildSessionStatus (get_child_session_status)", () => {
    it("shows loading state when no session or agent state", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_child_session_status", {
        result: mcpWrap({}),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Loading session status...")).toBeInTheDocument();
    });

    it("renders session title and agent status", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_child_session_status", {
        result: mcpWrap({
          session: { id: "uuid", title: "My Verification Session", status: "active" },
          agent_state: { is_running: true, estimated_status: "likely_generating" },
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("My Verification Session")).toBeInTheDocument();
      expect(screen.getByText("Generating")).toBeInTheDocument();
    });

    it("renders verification round badge when verification present", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_child_session_status", {
        result: mcpWrap({
          session: { id: "uuid", title: "Verif Session", status: "active" },
          agent_state: { is_running: true, estimated_status: "likely_waiting" },
          verification: { status: "reviewing", current_round: 3, gap_score: 200 },
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Waiting")).toBeInTheDocument();
      expect(screen.getByText("Round 3")).toBeInTheDocument();
    });

    it("renders idle agent status", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_child_session_status", {
        result: mcpWrap({
          session: { id: "uuid", title: "Idle Session", status: "active" },
          agent_state: { is_running: false, estimated_status: "idle" },
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Idle")).toBeInTheDocument();
    });
  });

  describe("VerificationConfirmationStatus (get_verification_confirmation_status)", () => {
    it("shows loading state when result has no status", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_verification_confirmation_status", {
        result: mcpWrap({}),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Checking confirmation status...")).toBeInTheDocument();
    });

    it("renders pending status badge", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_verification_confirmation_status", {
        result: mcpWrap({ status: "pending" }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Pending")).toBeInTheDocument();
    });

    it("renders not_applicable status badge as N/A", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_verification_confirmation_status", {
        result: mcpWrap({ status: "not_applicable" }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("N/A")).toBeInTheDocument();
    });

    it("renders unknown status fallback with raw status text", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_verification_confirmation_status", {
        result: mcpWrap({ status: "some_future_status" }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("some_future_status")).toBeInTheDocument();
    });

    it("shows loading state when result is undefined", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_verification_confirmation_status");
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Checking confirmation status...")).toBeInTheDocument();
    });
  });

  describe("PendingConfirmations (get_pending_confirmations)", () => {
    it("shows loading state when result has no sessions", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_pending_confirmations", {
        result: mcpWrap({}),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Checking pending confirmations...")).toBeInTheDocument();
    });

    it("renders 'No pending' badge when sessions is empty", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_pending_confirmations", {
        result: mcpWrap({ sessions: [] }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("No pending")).toBeInTheDocument();
    });

    it("renders count badge when sessions are present", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_pending_confirmations", {
        result: mcpWrap({
          sessions: [
            { session_id: "s1", session_title: "Session A" },
            { session_id: "s2", session_title: "Session B" },
            { session_id: "s3", session_title: null },
          ],
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("3 pending")).toBeInTheDocument();
    });

    it("shows loading state when result is undefined", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_pending_confirmations");
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Checking pending confirmations...")).toBeInTheDocument();
    });
  });
});
