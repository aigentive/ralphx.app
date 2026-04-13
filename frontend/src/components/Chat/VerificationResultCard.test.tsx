import { describe, it, expect } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { VerificationResultCard } from "./VerificationResultCard";

describe("VerificationResultCard", () => {
  const defaultProps = {
    summary: "1 gap remains: 1 critical.",
    convergenceReason: "agent_error",
    currentRound: 1,
    maxRounds: 5,
    recommendedNextAction: "rerun_verification",
    blockers: [
      {
        severity: "critical",
        description: "Delegated critic startup failed before any plan analysis.",
      },
    ],
    actionableForParent: false,
  };

  it("renders collapsed by default", () => {
    render(<VerificationResultCard {...defaultProps} />);
    expect(screen.getByText("Verification result")).toBeInTheDocument();
    expect(screen.queryByText(defaultProps.summary)).not.toBeInTheDocument();
  });

  it("expands to show summary and blocker details", () => {
    render(<VerificationResultCard {...defaultProps} />);
    fireEvent.click(screen.getByRole("button"));
    expect(screen.getByText(defaultProps.summary)).toBeInTheDocument();
    expect(screen.getByText(/Infra\/runtime issue/)).toBeInTheDocument();
    expect(screen.getByText(/Delegated critic startup failed/)).toBeInTheDocument();
    expect(screen.getByText(/Recommended next action: Re-run verification/)).toBeInTheDocument();
  });

  it("shows actionable label for plan-fixable outcomes", () => {
    render(
      <VerificationResultCard
        {...defaultProps}
        convergenceReason="max_rounds"
        recommendedNextAction="revise_plan"
        actionableForParent
      />,
    );
    fireEvent.click(screen.getByRole("button"));
    expect(screen.getByText(/Actionable for plan/)).toBeInTheDocument();
    expect(screen.getByText(/Recommended next action: Revise plan/)).toBeInTheDocument();
  });
});
