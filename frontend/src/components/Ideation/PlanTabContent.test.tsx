import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { PlanTabContent } from "./PlanTabContent";
import { useIdeationStore } from "@/stores/ideationStore";
import type { IdeationSession, TaskProposal } from "@/types/ideation";

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn(), info: vi.fn() },
}));

vi.mock("@/api/chat", () => ({
  chatApi: { sendAgentMessage: vi.fn().mockResolvedValue(undefined) },
}));

vi.mock("./PlanDisplay", () => ({
  PlanDisplay: () => <div data-testid="plan-display" />,
}));

vi.mock("./AcceptedSessionBanner", () => ({
  AcceptedSessionBanner: () => <div data-testid="accepted-session-banner" />,
}));

vi.mock("./ExportPlanDialog", () => ({
  ExportPlanDialog: () => null,
}));

// ============================================================================
// Fixtures
// ============================================================================

const mockSession: IdeationSession = {
  id: "session-1",
  projectId: "project-1",
  title: "Test Session",
  status: "active",
  planArtifactId: null,
  parentSessionId: null,
  createdAt: "2026-01-01T00:00:00.000Z",
  updatedAt: "2026-01-01T00:00:00.000Z",
  archivedAt: null,
  convertedAt: null,
  verificationStatus: "unverified",
  sessionPurpose: "general",
};

const defaultProps = {
  session: mockSession,
  proposals: [] as TaskProposal[],
  importStatus: null,
  onImportStatusChange: vi.fn(),
  onImportPlan: vi.fn(),
  onViewWork: vi.fn(),
  isPlanExpanded: false,
  onExpandedChange: vi.fn(),
  requestedHistoricalVersion: null,
  onHistoricalVersionViewed: vi.fn(),
};

function resetStore() {
  useIdeationStore.setState({ planArtifact: null, ideationSettings: null });
}

// ============================================================================
// Tests
// ============================================================================

describe("PlanTabContent — PlanEmptyState integration", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  it("renders PlanEmptyState when no plan and no proposals", () => {
    useIdeationStore.setState({ planArtifact: null, ideationSettings: null });
    render(<PlanTabContent {...defaultProps} />);
    expect(screen.getByTestId("plan-empty-state")).toBeInTheDocument();
  });

  it("does NOT render PlanEmptyState when proposals exist (Import button shown instead)", () => {
    const proposals = [
      { id: "p1", title: "Proposal 1" } as TaskProposal,
    ];
    useIdeationStore.setState({ planArtifact: null, ideationSettings: null });
    render(<PlanTabContent {...defaultProps} proposals={proposals} />);
    expect(screen.queryByTestId("plan-empty-state")).toBeNull();
    // Import button is shown instead
    expect(screen.getByTestId("import-plan-button")).toBeInTheDocument();
  });

  it("calls onImportPlan when the browse button inside PlanEmptyState is clicked", async () => {
    const onImportPlan = vi.fn();
    useIdeationStore.setState({ planArtifact: null, ideationSettings: null });
    render(<PlanTabContent {...defaultProps} onImportPlan={onImportPlan} />);
    await userEvent.click(screen.getByTestId("drop-hint"));
    expect(onImportPlan).toHaveBeenCalledTimes(1);
  });
});
