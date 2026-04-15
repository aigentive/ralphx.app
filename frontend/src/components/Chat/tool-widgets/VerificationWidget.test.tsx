import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { VerificationWidget } from "./VerificationWidget";
import { getToolCallWidget } from "./registry";
import type { ToolCall } from "./shared.constants";

const mockUseVerificationStatus = vi.fn(() => ({ data: undefined }));

vi.mock("@/hooks/useVerificationStatus", () => ({
  useVerificationStatus: (sessionId: string | undefined) => mockUseVerificationStatus(sessionId),
}));

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
  beforeEach(() => {
    mockUseVerificationStatus.mockReset();
    mockUseVerificationStatus.mockReturnValue({ data: undefined });
  });

  describe("backend-owned verifier flow widgets", () => {
    it("renders enrichment progress with specialist status instead of raw payload fallback", () => {
      const toolCall = makeToolCall("mcp__ralphx__run_verification_enrichment", {
        arguments: { selected_specialists: ["intent", "code-quality"] },
        result: mcpWrap({
          selected_specialists: [
            { name: "intent", label: "intent", critic: "intent" },
            { name: "code-quality", label: "code-quality", critic: "code-quality" },
          ],
          timed_out: true,
          findings_by_critic: [
            { critic: "intent", found: false, total_matches: 0 },
            { critic: "code-quality", found: true, total_matches: 1 },
          ],
          delegate_snapshots: [
            { job_id: "intent-1", status: "completed", label: "intent" },
            { job_id: "quality-1", status: "running", label: "code-quality" },
          ],
        }),
      });

      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Verification enrichment")).toBeInTheDocument();
      expect(screen.getByText("2 launched")).toBeInTheDocument();
      expect(screen.getByText("Timed out")).toBeInTheDocument();
      expect(screen.getByText("intent")).toBeInTheDocument();
      expect(screen.getByText("code-quality")).toBeInTheDocument();
      expect(screen.getByText("Completed")).toBeInTheDocument();
      expect(screen.getByText("Generating")).toBeInTheDocument();
      expect(screen.getByText("Completed with no findings published.")).toBeInTheDocument();
      expect(screen.getByText("1 finding published.")).toBeInTheDocument();
    });

    it("avoids misleading zero-specialist copy when requests were made but launches have not materialized", () => {
      const toolCall = makeToolCall("mcp__ralphx__run_verification_enrichment", {
        arguments: { selected_specialists: ["intent", "code-quality"] },
        result: mcpWrap({
          selected_specialists: [],
          timed_out: true,
          findings_by_critic: [],
        }),
      });

      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Verification enrichment")).toBeInTheDocument();
      expect(screen.getByText("2 requested")).toBeInTheDocument();
      expect(screen.queryByText("0 specialists")).not.toBeInTheDocument();
      expect(screen.getByText("Waiting for specialist launches.")).toBeInTheDocument();
    });

    it("renders round progress, classification, and severity counts for run_verification_round", () => {
      const toolCall = makeToolCall("mcp__ralphx__run_verification_round", {
        arguments: { round: 2 },
        result: mcpWrap({
          round: 2,
          classification: "complete",
          optional_timed_out: true,
          gap_counts: { critical: 0, high: 1, medium: 2, low: 0 },
          required_delegates: [
            { label: "completeness", critic: "completeness", job_id: "job-1" },
            { label: "feasibility", critic: "feasibility", job_id: "job-2" },
          ],
          delegate_snapshots: [
            { job_id: "job-1", status: "completed", label: "completeness" },
            { job_id: "job-2", status: "completed", label: "feasibility" },
          ],
          required_critic_settlement: {
            summary: "Required critics settled cleanly.",
            findings_by_critic: [
              { critic: "completeness", found: true, total_matches: 1, finding: { summary: "Completeness found one blocker." } },
              { critic: "feasibility", found: false, total_matches: 0 },
            ],
          },
          optional_specialists: [
            { label: "ux", critic: "ux", job_id: "job-3" },
          ],
          optional_delegates: [
            { label: "ux", critic: "ux", job_id: "job-3" },
          ],
          optional_findings_by_critic: [
            { critic: "ux", found: false, total_matches: 0 },
          ],
          optional_delegate_snapshots: [
            { job_id: "job-3", status: "running", label: "ux" },
          ],
        }),
      });

      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Verification round")).toBeInTheDocument();
      expect(screen.getByText("Round 2")).toBeInTheDocument();
      expect(screen.getByText("complete")).toBeInTheDocument();
      expect(screen.getByText("H 1")).toBeInTheDocument();
      expect(screen.getByText("M 2")).toBeInTheDocument();
      expect(screen.getByText("completeness")).toBeInTheDocument();
      expect(screen.getByText("feasibility")).toBeInTheDocument();
      expect(screen.getByText("Optional timed out")).toBeInTheDocument();
      expect(screen.getByText("Optional specialists")).toBeInTheDocument();
      expect(screen.getByText("Completeness found one blocker.")).toBeInTheDocument();
      expect(screen.getByText("Timed out while the delegate was still running.")).toBeInTheDocument();
    });

    it("renders backend-authoritative round report state", () => {
      const toolCall = makeToolCall("mcp__ralphx__report_verification_round", {
        arguments: { round: 1 },
        result: mcpWrap({
          status: "reviewing",
          in_progress: true,
          current_round: 1,
          current_gaps: [{ severity: "medium", category: "api", description: "gap" }],
        }),
      });

      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Round report")).toBeInTheDocument();
      expect(screen.getByText("reviewing")).toBeInTheDocument();
      expect(screen.getByText("1 gap")).toBeInTheDocument();
    });

    it("prefers authoritative live state over stale round report payloads", () => {
      mockUseVerificationStatus.mockReturnValue({
        data: {
          sessionId: "session-1",
          status: "needs_revision",
          inProgress: false,
          currentRound: 2,
          maxRounds: 5,
          convergenceReason: "max_rounds",
          gaps: [{ severity: "high", category: "api", description: "fresh gap" }],
          rounds: [],
          roundDetails: [],
        },
      });

      const toolCall = makeToolCall("mcp__ralphx__report_verification_round", {
        arguments: { round: 1 },
        result: mcpWrap({
          session_id: "session-1",
          status: "reviewing",
          in_progress: true,
          current_round: 1,
          current_gaps: [],
        }),
      });

      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("needs_revision")).toBeInTheDocument();
      expect(screen.getByText("Round 2")).toBeInTheDocument();
      expect(screen.getByText("1 gap")).toBeInTheDocument();
      expect(screen.queryByText("reviewing")).not.toBeInTheDocument();
    });

    it("renders terminal cleanup outcome including infra settlement summary", () => {
      const toolCall = makeToolCall("mcp__ralphx__complete_plan_verification", {
        arguments: { status: "needs_revision" },
        result: mcpWrap({
          status: "unverified",
          convergence_reason: "agent_error",
          settlement: {
            classification: "infra_failure",
            summary: "Required verification findings were missing.",
          },
        }),
      });

      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Final cleanup")).toBeInTheDocument();
      expect(screen.getByText("infra_failure")).toBeInTheDocument();
      expect(screen.getByText("Agent error")).toBeInTheDocument();
      expect(screen.getByText("Required verification findings were missing.")).toBeInTheDocument();
    });

    it("prefers authoritative live state over stale final cleanup payloads", () => {
      mockUseVerificationStatus.mockReturnValue({
        data: {
          sessionId: "session-1",
          status: "needs_revision",
          inProgress: false,
          currentRound: 2,
          maxRounds: 5,
          convergenceReason: "max_rounds",
          gaps: [{ severity: "high", category: "api", description: "fresh gap" }],
          rounds: [],
          roundDetails: [],
        },
      });

      const toolCall = makeToolCall("mcp__ralphx__complete_plan_verification", {
        arguments: { status: "needs_revision" },
        result: mcpWrap({
          session_id: "session-1",
          status: "unverified",
          convergence_reason: "agent_error",
          settlement: {
            classification: "infra_failure",
            summary: "Required verification findings were missing.",
          },
        }),
      });

      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("needs_revision")).toBeInTheDocument();
      expect(screen.getByText("Max rounds")).toBeInTheDocument();
      expect(screen.queryByText("Agent error")).not.toBeInTheDocument();
    });
  });

  describe("legacy verification tool cleanup", () => {
    it("does not register a widget for removed update_plan_verification calls", () => {
      expect(getToolCallWidget("mcp__ralphx__update_plan_verification")).toBeUndefined();
    });
  });

  describe("GetVerification (get_plan_verification)", () => {
    it("prefers authoritative live verification state over stale tool payloads", () => {
      mockUseVerificationStatus.mockReturnValue({
        data: {
          sessionId: "session-1",
          status: "needs_revision",
          inProgress: false,
          currentRound: 2,
          maxRounds: 5,
          convergenceReason: "max_rounds",
          gaps: [],
          rounds: [],
          roundDetails: [],
        },
      });

      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({
          session_id: "session-1",
          status: "reviewing",
          in_progress: true,
          current_round: 1,
          max_rounds: 5,
        }),
      });

      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("needs_revision")).toBeInTheDocument();
      expect(screen.getByText("Round 2/5")).toBeInTheDocument();
      expect(screen.queryByText("reviewing")).not.toBeInTheDocument();
    });

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

    it("renders continuity hint row when verification_child is present", () => {
      const sessionId = "abcdef12-0000-0000-0000-000000000000";
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({
          status: "unverified",
          verification_child: {
            latestChildSessionId: sessionId,
            agentState: "likely_generating",
            lastAssistantMessage: "Checking gap coverage for auth module.",
          },
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("unverified")).toBeInTheDocument();
      // Session ID snippet (first 8 chars + ellipsis)
      expect(screen.getByText("abcdef12…")).toBeInTheDocument();
      // Agent state badge
      expect(screen.getByText("Generating")).toBeInTheDocument();
      // Last message preview
      expect(screen.getByText("Checking gap coverage for auth module.")).toBeInTheDocument();
    });

    it("truncates last_assistant_message to 120 chars in continuity hint", () => {
      const longMessage = "A".repeat(200);
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({
          status: "reviewing",
          verification_child: {
            latestChildSessionId: "session-abc",
            agentState: "idle",
            lastAssistantMessage: longMessage,
          },
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      // Should be truncated to 120 + ellipsis
      expect(screen.getByText("A".repeat(120) + "…")).toBeInTheDocument();
    });

    it("renders without continuity hint when verification_child is null", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({
          status: "unverified",
          verification_child: null,
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("unverified")).toBeInTheDocument();
      // No agent state badge from continuity hint
      expect(screen.queryByText("Generating")).not.toBeInTheDocument();
      expect(screen.queryByText("Idle")).not.toBeInTheDocument();
    });

    it("renders without continuity hint when verification_child is absent", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({ status: "reviewing", in_progress: true, current_round: 1, max_rounds: 5 }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("reviewing")).toBeInTheDocument();
      // No agent state hints from continuity block
      expect(screen.queryByText("Generating")).not.toBeInTheDocument();
    });

    it("renders continuity hint without message when lastAssistantMessage is null", () => {
      const toolCall = makeToolCall("mcp__ralphx__get_plan_verification", {
        result: mcpWrap({
          status: "unverified",
          verification_child: {
            latestChildSessionId: "deadbeef-0000",
            agentState: "likely_waiting",
            lastAssistantMessage: null,
          },
        }),
      });
      render(<VerificationWidget toolCall={toolCall} />);
      expect(screen.getByText("Waiting")).toBeInTheDocument();
      expect(screen.getByText("deadbeef…")).toBeInTheDocument();
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
