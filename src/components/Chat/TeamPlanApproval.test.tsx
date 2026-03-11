/**
 * TeamPlanApproval component tests
 *
 * Verifies approve/reject handlers, expired plan error handling,
 * and clearPendingPlan calls.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TeamPlanApproval } from "./TeamPlanApproval";
import { useTeamStore } from "@/stores/teamStore";

// ============================================================================
// Mocks
// ============================================================================

const mockToastError = vi.fn();
vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), {
    error: (...args: unknown[]) => mockToastError(...args),
  }),
}));

const mockApproveTeamPlan = vi.fn();
const mockRejectTeamPlan = vi.fn();
vi.mock("@/api/team", () => ({
  approveTeamPlan: (...args: unknown[]) => mockApproveTeamPlan(...args),
  rejectTeamPlan: (...args: unknown[]) => mockRejectTeamPlan(...args),
}));

// ============================================================================
// Test data
// ============================================================================

const CONTEXT_KEY = "session:session-test";

const basePlan = {
  planId: "plan-test-123",
  process: "Feature Implementation",
  teammates: [
    { role: "Backend Engineer", model: "sonnet", tools: [], mcp_tools: [] },
    { role: "Frontend Engineer", model: "sonnet", tools: [], mcp_tools: [] },
  ],
  originContextType: "ideation",
  originContextId: "session-test",
  createdAt: Date.now(),
};

// ============================================================================
// Tests
// ============================================================================

describe("TeamPlanApproval", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockApproveTeamPlan.mockResolvedValue(undefined);
    mockRejectTeamPlan.mockResolvedValue(undefined);
    useTeamStore.setState({ pendingPlans: { [CONTEXT_KEY]: basePlan }, activeTeams: {} });
  });

  it("should render plan process name and teammate count", () => {
    render(<TeamPlanApproval plan={basePlan} contextKey={CONTEXT_KEY} />);
    expect(screen.getByText(/Feature Implementation/)).toBeDefined();
    expect(screen.getByText(/2 teammates/)).toBeDefined();
  });

  it("should show expired toast and clear pending plan when approving an already-expired backend plan", async () => {
    const user = userEvent.setup();
    mockApproveTeamPlan.mockRejectedValue(new Error("plan expired: channel closed"));

    render(<TeamPlanApproval plan={basePlan} contextKey={CONTEXT_KEY} />);

    const approveBtn = screen.getByRole("button", { name: /approve/i });
    await user.click(approveBtn);

    expect(mockToastError).toHaveBeenCalledWith(
      "Plan already expired — agent already received timeout response",
    );

    // clearPendingPlan should be called after expired approval
    expect(useTeamStore.getState().pendingPlans[CONTEXT_KEY]).toBeUndefined();
  });

  it("should show error message (not toast) for non-expired approval failures", async () => {
    const user = userEvent.setup();
    mockApproveTeamPlan.mockRejectedValue(new Error("Network error"));

    render(<TeamPlanApproval plan={basePlan} contextKey={CONTEXT_KEY} />);

    const approveBtn = screen.getByRole("button", { name: /approve/i });
    await user.click(approveBtn);

    // Generic error — shown inline, not as toast, plan NOT cleared
    expect(mockToastError).not.toHaveBeenCalled();
    expect(screen.getByText("Network error")).toBeDefined();
    expect(useTeamStore.getState().pendingPlans[CONTEXT_KEY]).toBeDefined();
  });

  it("should clear pending plan and call rejectTeamPlan on reject", async () => {
    const user = userEvent.setup();

    render(<TeamPlanApproval plan={basePlan} contextKey={CONTEXT_KEY} />);

    const rejectBtn = screen.getByRole("button", { name: /reject/i });
    await user.click(rejectBtn);

    expect(mockRejectTeamPlan).toHaveBeenCalledWith("plan-test-123");
    expect(useTeamStore.getState().pendingPlans[CONTEXT_KEY]).toBeUndefined();
  });
});
