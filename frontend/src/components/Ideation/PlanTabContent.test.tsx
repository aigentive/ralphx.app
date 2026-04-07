import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { PlanTabContent } from "./PlanTabContent";
import { useIdeationStore } from "@/stores/ideationStore";
import type { IdeationSession, TaskProposal } from "@/types/ideation";
import type { IdeationSettings } from "@/types/ideation-config";
import type { Artifact } from "@/types/artifact";

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn(), info: vi.fn() },
}));

vi.mock("@/api/chat", () => ({
  chatApi: { sendAgentMessage: vi.fn().mockResolvedValue(undefined) },
}));

vi.mock("./PlanDisplay", () => ({
  PlanDisplay: ({ onEdit }: { onEdit: () => void }) => (
    <div data-testid="plan-display">
      <button data-testid="edit-button" onClick={onEdit}>Edit</button>
    </div>
  ),
}));

vi.mock("./PlanEditor", () => ({
  PlanEditor: ({ onSave, onCancel }: { plan: Artifact; onSave: (a: Artifact) => void; onCancel: () => void }) => (
    <div data-testid="plan-editor">
      <button data-testid="save-button" onClick={() => onSave({ id: "art-1" } as Artifact)}>Save</button>
      <button data-testid="cancel-button" onClick={onCancel}>Cancel</button>
    </div>
  ),
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

const mockPlanArtifact: Artifact = {
  id: "art-1",
  type: "specification",
  name: "Test Plan",
  content: { type: "inline", text: "# Plan content" },
  metadata: { createdAt: "2026-01-01T00:00:00Z", createdBy: "agent", version: 1 },
};

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

const optionalSettings: IdeationSettings = {
  planMode: "optional",
  requirePlanApproval: false,
  suggestPlansForComplex: true,
  autoLinkProposals: true,
};
const requiredSettings: IdeationSettings = {
  planMode: "required",
  requirePlanApproval: false,
  suggestPlansForComplex: true,
  autoLinkProposals: true,
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

  it("renders PlanEmptyState when no plan, settings loaded with optional planMode, and no proposals", () => {
    useIdeationStore.setState({ planArtifact: null, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} />);
    expect(screen.getByTestId("plan-empty-state")).toBeInTheDocument();
  });

  it("renders PlanEmptyState when ideationSettings is null (null treated as non-required mode)", () => {
    useIdeationStore.setState({ planArtifact: null, ideationSettings: null });
    render(<PlanTabContent {...defaultProps} />);
    expect(screen.getByTestId("plan-empty-state")).toBeInTheDocument();
  });

  it("does NOT render PlanEmptyState when planMode is 'required' (spinner shown instead)", () => {
    useIdeationStore.setState({ planArtifact: null, ideationSettings: requiredSettings });
    render(<PlanTabContent {...defaultProps} />);
    expect(screen.queryByTestId("plan-empty-state")).toBeNull();
  });

  it("does NOT render PlanEmptyState when proposals exist (Import button shown instead)", () => {
    const proposals = [
      { id: "p1", title: "Proposal 1" } as TaskProposal,
    ];
    useIdeationStore.setState({ planArtifact: null, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} proposals={proposals} />);
    expect(screen.queryByTestId("plan-empty-state")).toBeNull();
    // Import button is shown instead
    expect(screen.getByTestId("import-plan-button")).toBeInTheDocument();
  });

  it("calls onImportPlan when the browse button inside PlanEmptyState is clicked", async () => {
    const onImportPlan = vi.fn();
    useIdeationStore.setState({ planArtifact: null, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} onImportPlan={onImportPlan} />);
    await userEvent.click(screen.getByTestId("drop-hint"));
    expect(onImportPlan).toHaveBeenCalledTimes(1);
  });
});

describe("PlanTabContent — PlanEditor wiring", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  it("shows PlanDisplay when plan artifact exists and not editing", () => {
    useIdeationStore.setState({ planArtifact: mockPlanArtifact, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} />);
    expect(screen.getByTestId("plan-display")).toBeInTheDocument();
    expect(screen.queryByTestId("plan-editor")).toBeNull();
  });

  it("switches to PlanEditor when Edit button is clicked", async () => {
    useIdeationStore.setState({ planArtifact: mockPlanArtifact, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} />);
    await userEvent.click(screen.getByTestId("edit-button"));
    expect(screen.getByTestId("plan-editor")).toBeInTheDocument();
    expect(screen.queryByTestId("plan-display")).toBeNull();
  });

  it("calls onHistoricalVersionViewed when Edit button is clicked", async () => {
    const onHistoricalVersionViewed = vi.fn();
    useIdeationStore.setState({ planArtifact: mockPlanArtifact, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} onHistoricalVersionViewed={onHistoricalVersionViewed} />);
    await userEvent.click(screen.getByTestId("edit-button"));
    expect(onHistoricalVersionViewed).toHaveBeenCalledTimes(1);
  });

  it("returns to PlanDisplay on Cancel", async () => {
    useIdeationStore.setState({ planArtifact: mockPlanArtifact, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} />);
    await userEvent.click(screen.getByTestId("edit-button"));
    await userEvent.click(screen.getByTestId("cancel-button"));
    expect(screen.getByTestId("plan-display")).toBeInTheDocument();
    expect(screen.queryByTestId("plan-editor")).toBeNull();
  });

  it("calls setPlanArtifact and returns to PlanDisplay on Save", async () => {
    useIdeationStore.setState({ planArtifact: mockPlanArtifact, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} />);
    await userEvent.click(screen.getByTestId("edit-button"));
    await userEvent.click(screen.getByTestId("save-button"));
    expect(screen.getByTestId("plan-display")).toBeInTheDocument();
    expect(screen.queryByTestId("plan-editor")).toBeNull();
  });

  it("exits edit mode with toast when plan version changes externally while editing", async () => {
    const { toast } = await import("sonner");
    useIdeationStore.setState({ planArtifact: mockPlanArtifact, ideationSettings: optionalSettings });
    render(<PlanTabContent {...defaultProps} />);
    await userEvent.click(screen.getByTestId("edit-button"));
    expect(screen.getByTestId("plan-editor")).toBeInTheDocument();

    act(() => {
      useIdeationStore.setState({
        planArtifact: { ...mockPlanArtifact, metadata: { ...mockPlanArtifact.metadata, version: 2 } },
      });
    });

    expect(screen.getByTestId("plan-display")).toBeInTheDocument();
    expect(toast.info).toHaveBeenCalledWith("Plan was updated externally. Exiting editor.");
  });
});
