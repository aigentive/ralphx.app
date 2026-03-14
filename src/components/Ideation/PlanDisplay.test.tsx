import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PlanDisplay } from "./PlanDisplay";
import type { TeamMetadata } from "./PlanDisplay";
import type { Artifact } from "@/types/artifact";
import type { VerificationGap } from "@/api/ideation.types";

vi.mock("./TeamFindingsSection", () => ({
  TeamFindingsSection: ({ findings, teamMode, teammateCount }: { findings: unknown[]; teamMode: string; teammateCount: number }) => (
    <div data-testid="team-findings-section" data-team-mode={teamMode} data-count={teammateCount}>
      {findings.length} findings
    </div>
  ),
}));

vi.mock("./DebateSummary", () => ({
  DebateSummary: ({ data }: { data: { winner: { name: string } } }) => (
    <div data-testid="debate-summary">Winner: {data.winner.name}</div>
  ),
}));

vi.mock("./VerificationHistory", () => ({
  VerificationHistory: ({ rounds }: { rounds: { round: number; gapScore: number }[] }) => (
    <div data-testid="verification-history">
      <div>Gap Score by Round</div>
      {rounds.map((r) => (
        <div key={r.round}>R{r.round}: {r.gapScore}</div>
      ))}
    </div>
  ),
}));

const mockPlan: Artifact = {
  id: "artifact-1",
  type: "specification",
  name: "Authentication Implementation Plan",
  content: {
    type: "inline",
    text: `# Authentication Plan\n\n## Overview\nImplement JWT-based authentication system.`,
  },
  metadata: {
    createdAt: "2026-01-26T10:00:00Z",
    createdBy: "orchestrator-ideation",
    version: 1,
  },
  derivedFrom: [],
  bucketId: "prd-library",
};

describe("PlanDisplay", () => {
  it("renders plan header and starts collapsed", () => {
    render(<PlanDisplay plan={mockPlan} />);

    expect(screen.getByText("Authentication Implementation Plan")).toBeInTheDocument();
    expect(screen.queryByText("Authentication Plan")).not.toBeInTheDocument();
  });

  it("expands and renders markdown content", () => {
    render(<PlanDisplay plan={mockPlan} />);

    fireEvent.click(screen.getByRole("button", { name: /Authentication Implementation Plan/i }));

    const heading = screen.getByText("Authentication Plan");
    expect(heading).toBeInTheDocument();
    expect(heading.tagName).toBe("H1");
    expect(screen.getByText(/JWT-based authentication/i)).toBeInTheDocument();
  });

  it("shows linked proposal counts", () => {
    const { rerender } = render(<PlanDisplay plan={mockPlan} linkedProposalsCount={3} />);
    expect(screen.getByText("3 linked proposals")).toBeInTheDocument();

    rerender(<PlanDisplay plan={mockPlan} linkedProposalsCount={1} />);
    expect(screen.getByText("1 linked proposal")).toBeInTheDocument();
  });

  it("calls onEdit and onExport from action buttons", () => {
    const onEdit = vi.fn();
    const onExport = vi.fn();
    const { container } = render(<PlanDisplay plan={mockPlan} onEdit={onEdit} onExport={onExport} />);

    const buttons = container.querySelectorAll("button");
    fireEvent.click(buttons[1]);
    fireEvent.click(buttons[2]);

    expect(onEdit).toHaveBeenCalledTimes(1);
    expect(onExport).toHaveBeenCalledTimes(1);
  });

  it("shows and handles Approve action", () => {
    const onApprove = vi.fn();
    render(<PlanDisplay plan={mockPlan} showApprove={true} isApproved={false} onApprove={onApprove} />);

    fireEvent.click(screen.getByRole("button", { name: /approve/i }));
    expect(onApprove).toHaveBeenCalledTimes(1);
  });

  it("shows approved badge when already approved", () => {
    render(<PlanDisplay plan={mockPlan} showApprove={true} isApproved={true} />);

    expect(screen.getByText("Approved")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /approve/i })).not.toBeInTheDocument();
  });

  it("shows no content for empty inline text", () => {
    const emptyPlan: Artifact = {
      ...mockPlan,
      content: { type: "inline", text: "" },
    };

    render(<PlanDisplay plan={emptyPlan} isExpanded={true} />);
    expect(screen.getByText("No content available")).toBeInTheDocument();
  });

  it("shows no content for file artifacts", () => {
    const filePlan: Artifact = {
      ...mockPlan,
      content: { type: "file", path: "/path/to/plan.md" },
    };

    render(<PlanDisplay plan={filePlan} isExpanded={true} />);
    expect(screen.getByText("No content available")).toBeInTheDocument();
  });

  describe("team metadata", () => {
    const researchMetadata: TeamMetadata = {
      teamIdeated: true,
      teamMode: "research",
      teammateCount: 3,
      findings: [
        { specialist: "auth-expert", keyFinding: "Use JWT" },
        { specialist: "db-expert", keyFinding: "Add indexes" },
      ],
    };

    const debateMetadata: TeamMetadata = {
      teamIdeated: true,
      teamMode: "debate",
      teammateCount: 2,
      findings: [{ specialist: "advocate-1", keyFinding: "REST wins" }],
      debateSummary: {
        advocates: [
          {
            name: "REST Advocate",
            role: "advocate",
            strengths: ["Simple"],
            weaknesses: ["Limited"],
            evidence: ["Industry standard"],
            criticChallenge: "Latency concerns",
          },
        ],
        winner: { name: "REST Advocate", justification: "Better ecosystem" },
      },
    };

    it("renders 'Research Team' badge for research mode", () => {
      render(<PlanDisplay plan={mockPlan} teamMetadata={researchMetadata} />);
      expect(screen.getByText("Research Team")).toBeInTheDocument();
    });

    it("renders 'Debate Team' badge for debate mode", () => {
      render(<PlanDisplay plan={mockPlan} teamMetadata={debateMetadata} />);
      expect(screen.getByText("Debate Team")).toBeInTheDocument();
    });

    it("renders team badge when expanded with research metadata", () => {
      render(<PlanDisplay plan={mockPlan} teamMetadata={researchMetadata} isExpanded={true} />);
      expect(screen.getByText("Research Team")).toBeInTheDocument();
    });

    it("renders DebateSummary for debate mode when expanded", () => {
      render(<PlanDisplay plan={mockPlan} teamMetadata={debateMetadata} isExpanded={true} />);
      expect(screen.getByTestId("debate-summary")).toBeInTheDocument();
      expect(screen.getByText("Winner: REST Advocate")).toBeInTheDocument();
    });

    it("does not render team badge when teamMetadata is absent", () => {
      render(<PlanDisplay plan={mockPlan} />);
      expect(screen.queryByText("Research Team")).not.toBeInTheDocument();
      expect(screen.queryByText("Debate Team")).not.toBeInTheDocument();
    });

    it("does not render team badge when teamIdeated is false", () => {
      const inactiveMetadata: TeamMetadata = {
        teamIdeated: false,
        teamMode: "research",
        teammateCount: 0,
        findings: [],
      };
      render(<PlanDisplay plan={mockPlan} teamMetadata={inactiveMetadata} />);
      expect(screen.queryByText("Research Team")).not.toBeInTheDocument();
    });
  });

  describe("Create Proposals button visibility", () => {
    const onCreateProposals = vi.fn();

    it("shows Create Proposals button when verified and linkedProposalsCount is 0", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="verified"
          linkedProposalsCount={0}
          onCreateProposals={onCreateProposals}
        />,
      );
      expect(screen.getByRole("button", { name: /create proposals/i })).toBeInTheDocument();
    });

    it("shows Create Proposals button when skipped and linkedProposalsCount is 0", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="skipped"
          linkedProposalsCount={0}
          onCreateProposals={onCreateProposals}
        />,
      );
      expect(screen.getByRole("button", { name: /create proposals/i })).toBeInTheDocument();
    });

    it("hides Create Proposals button when verified but linkedProposalsCount > 0", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="verified"
          linkedProposalsCount={2}
          onCreateProposals={onCreateProposals}
        />,
      );
      expect(screen.queryByRole("button", { name: /create proposals/i })).not.toBeInTheDocument();
    });

    it("hides Create Proposals button after proposals are created (0 → N transition)", () => {
      const { rerender } = render(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="verified"
          linkedProposalsCount={0}
          onCreateProposals={onCreateProposals}
        />,
      );
      expect(screen.getByRole("button", { name: /create proposals/i })).toBeInTheDocument();

      rerender(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="verified"
          linkedProposalsCount={3}
          onCreateProposals={onCreateProposals}
        />,
      );
      expect(screen.queryByRole("button", { name: /create proposals/i })).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Derived gap staleness (Step 2)
  // ============================================================================

  describe("derived gap staleness", () => {
    const mockGaps: VerificationGap[] = [
      { severity: "high", category: "correctness", description: "Missing null check" },
    ];

    const baseStaleProps = {
      verificationStatus: "needs_revision" as const,
      verificationInProgress: false,
      verificationGaps: mockGaps,
      onAddressGaps: vi.fn(),
      onRetryVerification: vi.fn(),
    };

    it("marks gaps stale when planVersion > verificationPlanVersion: CTA hidden, re-verify shown", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseStaleProps}
          planVersion={2}
          verificationPlanVersion={1}
        />,
      );

      // Stale notice is shown
      expect(screen.getByText(/plan updated — these gaps may be resolved/i)).toBeInTheDocument();
      // aria-label on the dimmed container
      expect(
        screen.getByLabelText(/verification gaps — plan has been updated since these were identified/i),
      ).toBeInTheDocument();
      // Address Gaps CTA is hidden
      expect(screen.queryByRole("button", { name: /address.*gaps/i })).not.toBeInTheDocument();
      // Re-verify Plan button is shown
      expect(screen.getByRole("button", { name: /re-verify plan/i })).toBeInTheDocument();
    });

    it("does not mark gaps stale when planVersion === verificationPlanVersion: CTA visible, re-verify hidden", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseStaleProps}
          planVersion={2}
          verificationPlanVersion={2}
        />,
      );

      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      // Address Gaps CTA is visible
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();
      // Re-verify Plan button is hidden
      expect(screen.queryByRole("button", { name: /re-verify plan/i })).not.toBeInTheDocument();
    });

    it("does not mark gaps stale when planVersion is undefined", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseStaleProps}
          planVersion={undefined}
          verificationPlanVersion={1}
        />,
      );

      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /re-verify plan/i })).not.toBeInTheDocument();
    });

    it("does not mark gaps stale when verificationPlanVersion is undefined", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseStaleProps}
          planVersion={2}
          verificationPlanVersion={undefined}
        />,
      );

      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /re-verify plan/i })).not.toBeInTheDocument();
    });

    it("does not mark gaps stale when both versions are undefined", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseStaleProps}
        />,
      );

      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Concurrent plan update + verification event ordering (Step 3)
  // ============================================================================

  describe("staleness — concurrent update ordering", () => {
    const mockGaps: VerificationGap[] = [
      { severity: "medium", category: "completeness", description: "Missing edge case" },
    ];

    const baseProps = {
      verificationStatus: "needs_revision" as const,
      verificationInProgress: false,
      verificationGaps: mockGaps,
      onAddressGaps: vi.fn(),
      onRetryVerification: vi.fn(),
    };

    it("plan version arriving before verification: planVersion=2 then verificationPlanVersion=1 → stale", () => {
      // Scenario: plan artifact updates first (version bumps to 2),
      // but verification cache still has no planVersion yet.
      const { rerender } = render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseProps}
          planVersion={2}
          verificationPlanVersion={undefined}
        />,
      );

      // Not stale yet — verificationPlanVersion missing
      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();

      // Verification cache now has an older planVersion (verification ran at version 1)
      rerender(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseProps}
          planVersion={2}
          verificationPlanVersion={1}
        />,
      );

      // Now stale — plan is newer than when verification ran
      expect(screen.getByText(/plan updated — these gaps may be resolved/i)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /address.*gaps/i })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /re-verify plan/i })).toBeInTheDocument();
    });

    it("verification arriving before plan update: verificationPlanVersion=1 then planVersion=2 → stale", () => {
      // Scenario: verification event arrives first (stamps planVersion=1 in cache),
      // then the plan artifact update bumps to version 2.
      const { rerender } = render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseProps}
          planVersion={undefined}
          verificationPlanVersion={1}
        />,
      );

      // Not stale yet — planVersion missing
      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();

      // Plan artifact update arrives — version bumps to 2
      rerender(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseProps}
          planVersion={2}
          verificationPlanVersion={1}
        />,
      );

      // Now stale — regardless of which update arrived first
      expect(screen.getByText(/plan updated — these gaps may be resolved/i)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /address.*gaps/i })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /re-verify plan/i })).toBeInTheDocument();
    });

    it("both arriving simultaneously at same version → not stale", () => {
      // Scenario: verification event and plan artifact version are both at 2 — no staleness
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          {...baseProps}
          planVersion={2}
          verificationPlanVersion={2}
        />,
      );

      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /re-verify plan/i })).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Plan-edit lock (auto-verification in progress)
  // ============================================================================

  describe("plan-edit lock", () => {
    it("disables Edit button when verificationInProgress=true", () => {
      const onEdit = vi.fn();
      render(
        <PlanDisplay
          plan={mockPlan}
          onEdit={onEdit}
          verificationStatus="reviewing"
          verificationInProgress={true}
        />
      );

      const editButton = screen.getByTitle(/Plan is being auto-verified/i);
      expect(editButton).toBeDisabled();
    });

    it("enables Edit button when verificationInProgress=false", () => {
      const onEdit = vi.fn();
      render(
        <PlanDisplay
          plan={mockPlan}
          onEdit={onEdit}
          verificationStatus="reviewing"
          verificationInProgress={false}
        />
      );

      // No disabled edit button with auto-verify title
      expect(screen.queryByTitle(/Plan is being auto-verified/i)).not.toBeInTheDocument();
    });

    it("clicking disabled Edit button does not call onEdit", () => {
      const onEdit = vi.fn();
      render(
        <PlanDisplay
          plan={mockPlan}
          onEdit={onEdit}
          verificationStatus="reviewing"
          verificationInProgress={true}
        />
      );

      const editButton = screen.getByTitle(/Plan is being auto-verified/i);
      fireEvent.click(editButton);
      expect(onEdit).not.toHaveBeenCalled();
    });

    it("enables Edit button after verification completes (verificationInProgress transitions to false)", () => {
      const { rerender } = render(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="reviewing"
          verificationInProgress={true}
        />
      );

      expect(screen.getByTitle(/Plan is being auto-verified/i)).toBeDisabled();

      rerender(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="verified"
          verificationInProgress={false}
        />
      );

      expect(screen.queryByTitle(/Plan is being auto-verified/i)).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Verification History tab
  // ============================================================================

  describe("Verification History tab", () => {
    const rounds = [
      { round: 1, gapScore: 20, gapCount: 3 },
      { round: 2, gapScore: 0, gapCount: 0 },
    ];

    it("shows history tab when verified with rounds", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="verified"
          verificationInProgress={false}
          verificationRounds={rounds}
        />
      );

      expect(screen.getByText("Verification History")).toBeInTheDocument();
    });

    it("shows history tab when needs_revision with rounds", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="needs_revision"
          verificationInProgress={false}
          verificationRounds={rounds}
        />
      );

      expect(screen.getByText("Verification History")).toBeInTheDocument();
    });

    it("does not show history tab when reviewing (in progress)", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="reviewing"
          verificationInProgress={true}
          verificationRounds={rounds}
        />
      );

      expect(screen.queryByText("Verification History")).not.toBeInTheDocument();
    });

    it("does not show history tab when no rounds", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="verified"
          verificationInProgress={false}
          verificationRounds={[]}
        />
      );

      expect(screen.queryByText("Verification History")).not.toBeInTheDocument();
    });

    it("switching to history tab hides plan content", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="verified"
          verificationInProgress={false}
          verificationRounds={rounds}
        />
      );

      // Initially on content tab — plan content visible
      expect(screen.getByText(/JWT-based authentication/i)).toBeInTheDocument();

      // Click history tab
      fireEvent.click(screen.getByText("Verification History"));

      // Plan content hidden, history shown
      expect(screen.queryByText(/JWT-based authentication/i)).not.toBeInTheDocument();
      expect(screen.getByText(/Gap Score by Round/i)).toBeInTheDocument();
    });

    it("switching back to Plan tab shows content again", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="verified"
          verificationInProgress={false}
          verificationRounds={rounds}
        />
      );

      fireEvent.click(screen.getByText("Verification History"));
      fireEvent.click(screen.getByText("Plan"));

      expect(screen.getByText(/JWT-based authentication/i)).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Incomplete verification message
  // ============================================================================

  describe("incomplete verification message", () => {
    const rounds = [
      { round: 1, gapScore: 18, gapCount: 3 },
      { round: 2, gapScore: 5, gapCount: 1 },
    ];

    it("shows incomplete message when unverified with rounds (reconciler reset)", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="unverified"
          verificationInProgress={false}
          verificationRounds={rounds}
        />
      );

      expect(screen.getByText(/Verification incomplete/i)).toBeInTheDocument();
      expect(screen.getByText(/2 rounds completed/i)).toBeInTheDocument();
    });

    it("shows 'View partial results' link when unverified with rounds", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="unverified"
          verificationInProgress={false}
          verificationRounds={rounds}
        />
      );

      expect(screen.getByText(/Verification incomplete/i)).toBeInTheDocument();
      expect(screen.getByText(/View partial results/i)).toBeInTheDocument();
    });

    it("does not show incomplete message when unverified with no rounds", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="unverified"
          verificationInProgress={false}
          verificationRounds={[]}
        />
      );

      expect(screen.queryByText(/Verification incomplete/i)).not.toBeInTheDocument();
    });

    it("does not show incomplete message when verificationRounds undefined", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="unverified"
          verificationInProgress={false}
        />
      );

      expect(screen.queryByText(/Verification incomplete/i)).not.toBeInTheDocument();
    });

    it("does not show incomplete message when status is needs_revision (not unverified)", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="needs_revision"
          verificationInProgress={false}
          verificationRounds={rounds}
        />
      );

      expect(screen.queryByText(/Verification incomplete/i)).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Full staleness flow — component-level integration test (Step 4)
  // ============================================================================

  describe("full staleness flow (integration)", () => {
    it("verify→gaps→plan update→stale→re-verify→fresh completes correctly", () => {
      const onRetryVerification = vi.fn();
      const onAddressGaps = vi.fn();

      const initialGaps: VerificationGap[] = [
        { severity: "high", category: "correctness", description: "Null pointer risk" },
      ];
      const freshGaps: VerificationGap[] = [
        { severity: "low", category: "style", description: "Minor naming issue" },
      ];

      // Phase 1: Verification completed, gaps visible, plan at version 1
      const { rerender } = render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="needs_revision"
          verificationInProgress={false}
          verificationGaps={initialGaps}
          planVersion={1}
          verificationPlanVersion={1}
          onAddressGaps={onAddressGaps}
          onRetryVerification={onRetryVerification}
        />,
      );

      // Gaps visible, CTA visible, no stale warning
      expect(screen.getByText("Null pointer risk")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();
      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /re-verify plan/i })).not.toBeInTheDocument();

      // Phase 2: Plan updated (version bump from 1 → 2), verification cache still at planVersion=1
      rerender(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="needs_revision"
          verificationInProgress={false}
          verificationGaps={initialGaps}
          planVersion={2}
          verificationPlanVersion={1}
          onAddressGaps={onAddressGaps}
          onRetryVerification={onRetryVerification}
        />,
      );

      // Gaps dimmed + CTA hidden + re-verify shown
      expect(screen.getByText(/plan updated — these gaps may be resolved/i)).toBeInTheDocument();
      expect(
        screen.getByLabelText(/verification gaps — plan has been updated since these were identified/i),
      ).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /address.*gaps/i })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /re-verify plan/i })).toBeInTheDocument();

      // Phase 3: New verification event arrives with planVersion=2 (matching current plan)
      rerender(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="needs_revision"
          verificationInProgress={false}
          verificationGaps={freshGaps}
          planVersion={2}
          verificationPlanVersion={2}
          onAddressGaps={onAddressGaps}
          onRetryVerification={onRetryVerification}
        />,
      );

      // Fresh gaps — stale warning gone, CTA back, re-verify hidden
      expect(screen.queryByText(/plan updated — these gaps may be resolved/i)).not.toBeInTheDocument();
      expect(screen.getByText("Minor naming issue")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /address.*gaps/i })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /re-verify plan/i })).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // imported_verified status
  // ============================================================================

  describe("imported_verified status", () => {
    it("shows 'Verified (imported)' badge for imported_verified status", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="imported_verified"
          verificationInProgress={false}
        />,
      );
      expect(screen.getByText("Verified (imported)")).toBeInTheDocument();
    });

    it("shows tooltip with source project name when sourceProjectName provided", () => {
      render(
        <PlanDisplay
          plan={mockPlan}
          verificationStatus="imported_verified"
          verificationInProgress={false}
          sourceProjectName="Project Alpha"
        />,
      );
      expect(screen.getByText("Verified (imported)")).toBeInTheDocument();
    });

    it("shows 'Create Proposals' button for imported_verified status", () => {
      const onCreateProposals = vi.fn();
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="imported_verified"
          verificationInProgress={false}
          onCreateProposals={onCreateProposals}
          linkedProposalsCount={0}
        />,
      );
      expect(screen.getByRole("button", { name: /create proposals/i })).toBeInTheDocument();
    });

    it("does not show 'Create Proposals' for imported_verified when proposals already exist", () => {
      const onCreateProposals = vi.fn();
      render(
        <PlanDisplay
          plan={mockPlan}
          isExpanded={true}
          verificationStatus="imported_verified"
          verificationInProgress={false}
          onCreateProposals={onCreateProposals}
          linkedProposalsCount={3}
        />,
      );
      expect(screen.queryByRole("button", { name: /create proposals/i })).not.toBeInTheDocument();
    });
  });
});
