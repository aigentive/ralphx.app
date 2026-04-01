import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PlanDisplay } from "./PlanDisplay";
import type { TeamMetadata } from "./PlanDisplay";
import type { Artifact } from "@/types/artifact";

const mockGetVersionHistory = vi.fn();
const mockGetAtVersion = vi.fn();

vi.mock("@/api/artifact", () => ({
  artifactApi: {
    getAtVersion: (...args: unknown[]) => mockGetAtVersion(...args),
    getVersionHistory: (...args: unknown[]) => mockGetVersionHistory(...args),
    get: vi.fn().mockResolvedValue(null),
    getByTask: vi.fn().mockResolvedValue([]),
    getByBucket: vi.fn().mockResolvedValue([]),
  },
}));

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
  beforeEach(() => {
    mockGetAtVersion.mockResolvedValue(null);
    mockGetVersionHistory.mockResolvedValue([]);
  });

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

  it("calls onEdit and onExport from action buttons", async () => {
    const user = userEvent.setup();
    const onEdit = vi.fn();
    const onExport = vi.fn();
    render(<PlanDisplay plan={mockPlan} onEdit={onEdit} onExport={onExport} />);

    // Actions are in a MoreHorizontal dropdown — open it first
    const buttons = screen.getAllByRole("button");
    const moreButton = buttons[buttons.length - 1];
    await user.click(moreButton);

    await user.click(screen.getByRole("menuitem", { name: /edit/i }));
    expect(onEdit).toHaveBeenCalledTimes(1);

    await user.click(moreButton);
    await user.click(screen.getByRole("menuitem", { name: /export/i }));
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
  // Version history timestamps
  // ============================================================================

  describe("version history timestamps", () => {
    const multiVersionPlan: Artifact = {
      ...mockPlan,
      metadata: {
        ...mockPlan.metadata,
        version: 3,
      },
    };

    it("does not show version dropdown for single-version plans", () => {
      render(<PlanDisplay plan={mockPlan} />);
      // version === 1, no History button rendered
      expect(screen.queryByTitle("View version history")).not.toBeInTheDocument();
    });

    it("calls getVersionHistory on mount for multi-version plans", async () => {
      mockGetVersionHistory.mockResolvedValue([
        { id: "v3-id", version: 3, name: "Plan", created_at: "2026-03-18T11:30:00Z" },
        { id: "v2-id", version: 2, name: "Plan", created_at: "2026-03-17T16:15:00Z" },
        { id: "v1-id", version: 1, name: "Plan", created_at: "2026-03-16T09:00:00Z" },
      ]);

      render(<PlanDisplay plan={multiVersionPlan} />);

      await waitFor(() => {
        expect(mockGetVersionHistory).toHaveBeenCalledWith(multiVersionPlan.id);
      });
    });

    it("does not call getVersionHistory for single-version plans", async () => {
      render(<PlanDisplay plan={mockPlan} />);

      // Give React a tick to run effects
      await act(async () => {});
      expect(mockGetVersionHistory).not.toHaveBeenCalled();
    });

    it("renders without error when version history fetch fails (graceful fallback)", async () => {
      mockGetVersionHistory.mockRejectedValue(new Error("Network error"));

      render(<PlanDisplay plan={multiVersionPlan} />);

      // Should not throw — component renders normally
      await waitFor(() => {
        expect(mockGetVersionHistory).toHaveBeenCalled();
      });

      // History button still visible
      expect(screen.getByTitle("View version history")).toBeInTheDocument();
    });

    it("opens dropdown and shows version numbers with userEvent", async () => {
      const user = userEvent.setup();
      mockGetVersionHistory.mockResolvedValue([
        { id: "v2-id", version: 2, name: "Plan", created_at: "2026-03-18T10:00:00Z" },
        { id: "v1-id", version: 1, name: "Plan", created_at: "2026-03-17T10:00:00Z" },
      ]);

      const twoVersionPlan: Artifact = {
        ...mockPlan,
        metadata: { ...mockPlan.metadata, version: 2 },
      };

      render(<PlanDisplay plan={twoVersionPlan} />);

      // Wait for version history to load
      await waitFor(() => {
        expect(mockGetVersionHistory).toHaveBeenCalled();
      });

      // Open dropdown
      await user.click(screen.getByTitle("View version history"));

      // Dropdown items should be visible
      await waitFor(() => {
        expect(screen.getByText("(latest)")).toBeInTheDocument();
      });
    });
  });
});
