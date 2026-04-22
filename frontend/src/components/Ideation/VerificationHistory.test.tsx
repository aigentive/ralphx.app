import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { VerificationHistory } from "./VerificationHistory";
import type { RoundSummary, VerificationGap, VerificationRoundDetail } from "@/types/ideation";

const mockRounds: RoundSummary[] = [
  { round: 1, gapScore: 23, gapCount: 4 },
  { round: 2, gapScore: 10, gapCount: 2 },
  { round: 3, gapScore: 0, gapCount: 0 },
];

const mockGaps: VerificationGap[] = [
  { severity: "high", category: "correctness", description: "Missing null check", whyItMatters: "Will crash" },
  { severity: "medium", category: "performance", description: "No caching" },
  { severity: "low", category: "style", description: "Inconsistent naming" },
];

const mockRoundDetails: VerificationRoundDetail[] = [
  {
    round: 1,
    gapScore: 13,
    gapCount: 2,
    gaps: [
      { severity: "critical", category: "completeness", description: "Missing migration registration" },
      { severity: "high", category: "testing", description: "Missing register-project coverage" },
    ],
  },
  {
    round: 2,
    gapScore: 3,
    gapCount: 1,
    gaps: [
      { severity: "high", category: "testing", description: "Missing register-project coverage" },
    ],
  },
];

describe("VerificationHistory", () => {
  it("renders round score trend bars", () => {
    render(<VerificationHistory rounds={mockRounds} />);

    // Gap scores visible as labels above bars
    expect(screen.getByText("23")).toBeInTheDocument();
    expect(screen.getByText("10")).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();

    // Round labels
    expect(screen.getByText("R1")).toBeInTheDocument();
    expect(screen.getByText("R2")).toBeInTheDocument();
    expect(screen.getByText("R3")).toBeInTheDocument();
  });

  it("renders gap score by round section heading when rounds provided", () => {
    render(<VerificationHistory rounds={mockRounds} />);
    expect(screen.getByText(/Gap Score by Round/i)).toBeInTheDocument();
  });

  it("renders empty state when no rounds and no gaps", () => {
    render(<VerificationHistory rounds={[]} />);
    expect(screen.getByText(/No verification rounds recorded/i)).toBeInTheDocument();
  });

  it("renders current gaps breakdown grouped by severity", () => {
    render(<VerificationHistory rounds={[]} currentGaps={mockGaps} />);

    expect(screen.getByText(/Final Gaps \(3\)/i)).toBeInTheDocument();
    expect(screen.getByText("Missing null check")).toBeInTheDocument();
    expect(screen.getByText("No caching")).toBeInTheDocument();
    expect(screen.getByText("Inconsistent naming")).toBeInTheDocument();
  });

  it("shows gap severity labels in breakdown", () => {
    render(<VerificationHistory rounds={[]} currentGaps={mockGaps} />);

    expect(screen.getByText(/High \(1\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Medium \(1\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Low \(1\)/i)).toBeInTheDocument();
  });

  it("shows whyItMatters when provided", () => {
    render(<VerificationHistory rounds={[]} currentGaps={mockGaps} />);
    expect(screen.getByText("Will crash")).toBeInTheDocument();
  });

  it("shows convergence reason label for verified status", () => {
    render(
      <VerificationHistory
        rounds={mockRounds}
        status="verified"
        convergenceReason="zero_blocking"
        gapScore={0}
      />
    );

    expect(screen.getByText("Plan verified")).toBeInTheDocument();
    expect(screen.getByText("No blocking gaps remain")).toBeInTheDocument();
  });

  it("shows needs_revision status summary with gap score", () => {
    render(
      <VerificationHistory
        rounds={mockRounds}
        status="needs_revision"
        convergenceReason="max_rounds"
        gapScore={15}
      />
    );

    expect(screen.getByText("Gaps require attention")).toBeInTheDocument();
    expect(screen.getByText("Maximum verification rounds reached")).toBeInTheDocument();
    expect(screen.getByText(/Gap score: 15/)).toBeInTheDocument();
  });

  it("does not show status summary when reviewing", () => {
    render(
      <VerificationHistory
        rounds={mockRounds}
        status="reviewing"
      />
    );

    expect(screen.queryByText("Plan verified")).not.toBeInTheDocument();
    expect(screen.queryByText("Gaps require attention")).not.toBeInTheDocument();
  });

  it("does not show critical label when no critical gaps", () => {
    render(<VerificationHistory rounds={[]} currentGaps={mockGaps} />);
    // mockGaps has no critical severity
    expect(screen.queryByText(/Critical/)).not.toBeInTheDocument();
  });

  it("renders round lineage with addressed gaps when round details are present", () => {
    render(
      <VerificationHistory
        rounds={mockRounds}
        roundDetails={mockRoundDetails}
        currentGaps={mockRoundDetails[1]?.gaps}
      />
    );

    expect(screen.getByText(/Round Lineage/i)).toBeInTheDocument();
    expect(screen.getByText(/Remaining after round 2/i)).toBeInTheDocument();
    expect(screen.getByText(/Addressed Since Round 1/i)).toBeInTheDocument();
    expect(screen.queryByText(/Remaining after round 1/i)).not.toBeInTheDocument();
  });

  it("renders newest round first in the lineage list", () => {
    render(
      <VerificationHistory
        rounds={mockRounds}
        roundDetails={mockRoundDetails}
      />
    );

    const buttons = screen.getAllByRole("button", { name: /round \d+ summary/i });
    expect(buttons).toHaveLength(2);
    expect(buttons[0]).toHaveTextContent("Round 2");
    expect(buttons[1]).toHaveTextContent("Round 1");
  });

  it("uses progressive disclosure for round details", async () => {
    const user = userEvent.setup();

    render(
      <VerificationHistory
        rounds={mockRounds}
        roundDetails={mockRoundDetails}
      />
    );

    const round2Button = screen.getByRole("button", { name: /round 2 summary/i });
    const round1Button = screen.getByRole("button", { name: /round 1 summary/i });

    expect(round2Button).toHaveAttribute("aria-expanded", "true");
    expect(round1Button).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByText(/Remaining after round 2/i)).toBeInTheDocument();
    expect(screen.queryByText(/Remaining after round 1/i)).not.toBeInTheDocument();

    await user.click(round1Button);

    expect(round1Button).toHaveAttribute("aria-expanded", "true");
    expect(round2Button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText(/Remaining after round 2/i)).not.toBeInTheDocument();
    expect(screen.getByText(/Remaining after round 1/i)).toBeInTheDocument();
    expect(screen.getByText("Missing migration registration")).toBeInTheDocument();

    await user.click(round1Button);

    expect(round1Button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText(/Remaining after round 1/i)).not.toBeInTheDocument();
  });
});
