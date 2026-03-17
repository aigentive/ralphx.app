/**
 * VerificationPanel.test.tsx
 *
 * Covers the page-load hydration fix:
 * - Tab populates when session.verificationStatus is "unverified" but backend has data
 * - Empty state renders correctly when no verification has been run (404)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import type { IdeationSession } from "@/types/ideation";

// ── Mock React Query (avoid two-React-copies issue from mixed node_modules) ──

const mockSetQueryData = vi.fn();
const mockUseQueryClient = vi.fn(() => ({ setQueryData: mockSetQueryData }));
let mockQueryResult: { data: unknown; isLoading: boolean } = { data: undefined, isLoading: false };

vi.mock("@tanstack/react-query", () => ({
  useQuery: vi.fn(() => mockQueryResult),
  useQueryClient: mockUseQueryClient,
}));

// ── Mock API ─────────────────────────────────────────────────────────────────

vi.mock("@/api/ideation", () => ({
  ideationApi: {
    verification: {
      getStatus: vi.fn(),
      skip: vi.fn(),
    },
    sessions: {
      getChildren: vi.fn().mockResolvedValue([]),
    },
  },
}));

vi.mock("@/hooks/useIdeation", () => ({
  ideationKeys: {
    sessions: () => ["sessions"],
    sessionWithData: (id: string) => ["session", id],
  },
}));

vi.mock("@/api/chat", () => ({
  chatApi: {
    sendAgentMessage: vi.fn(),
  },
}));

vi.mock("@/stores/ideationStore", () => ({
  useIdeationStore: vi.fn((selector: (s: object) => unknown) =>
    selector({
      activeVerificationChildId: {},
      setActiveVerificationChildId: vi.fn(),
      setVerificationNotification: vi.fn(),
    })
  ),
}));

// Sub-components that require their own data fetching — mock to simple divs
vi.mock("./VerificationBadge", () => ({
  VerificationBadge: ({ status }: { status: string }) => (
    <div data-testid="verification-badge">{status}</div>
  ),
}));

vi.mock("./VerificationGapList", () => ({
  VerificationGapList: () => <div data-testid="verification-gap-list" />,
}));

vi.mock("./VerificationHistory", () => ({
  VerificationHistory: () => <div data-testid="verification-history" />,
}));

// ── Session fixture ───────────────────────────────────────────────────────────

const baseSession: IdeationSession = {
  id: "session-1",
  projectId: "proj-1",
  title: "My Plan",
  status: "active",
  sessionPurpose: "ideation",
  verificationStatus: "unverified",
  verificationInProgress: false,
  planArtifactId: "artifact-1",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("VerificationPanel — page-load hydration", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    mockQueryResult = { data: undefined, isLoading: false };
    const { useQuery } = await import("@tanstack/react-query");
    vi.mocked(useQuery).mockReturnValue(mockQueryResult as ReturnType<typeof useQuery>);
  });

  it("shows verification content (rounds) when query returns data even if session.verificationStatus is 'unverified'", async () => {
    const { useQuery } = await import("@tanstack/react-query");
    const verificationData = {
      sessionId: "session-1",
      status: "verified",
      inProgress: false,
      gaps: [],
      rounds: [{ round: 1, gapScore: 5, gapCount: 1 }],
    };
    // First call = verification data, second = childSessions (empty)
    vi.mocked(useQuery)
      .mockReturnValueOnce({ data: verificationData } as ReturnType<typeof useQuery>)
      .mockReturnValueOnce({ data: [] } as unknown as ReturnType<typeof useQuery>);

    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={baseSession} />);

    await waitFor(() => {
      // Should NOT show the empty state
      expect(screen.queryByTestId("verification-empty-state")).not.toBeInTheDocument();
    });
    // Should show main content
    expect(screen.getByTestId("verification-panel-content")).toBeInTheDocument();
  });

  it("shows empty state when query returns null (404 — no verification ever started)", async () => {
    const { useQuery } = await import("@tanstack/react-query");
    // Both queries return null/empty
    vi.mocked(useQuery)
      .mockReturnValueOnce({ data: null } as unknown as ReturnType<typeof useQuery>)
      .mockReturnValueOnce({ data: [] } as unknown as ReturnType<typeof useQuery>);

    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={baseSession} />);

    await waitFor(() => {
      expect(screen.getByTestId("verification-empty-state")).toBeInTheDocument();
    });
    // Verify First CTA shows since plan exists
    expect(screen.getByTestId("verify-first-button")).toBeInTheDocument();
  });

  it("shows empty state for session with no plan artifact and does not show Verify First button", async () => {
    const { useQuery } = await import("@tanstack/react-query");
    vi.mocked(useQuery)
      .mockReturnValueOnce({ data: null } as unknown as ReturnType<typeof useQuery>)
      .mockReturnValueOnce({ data: [] } as unknown as ReturnType<typeof useQuery>);

    const sessionNoPlan: IdeationSession = { ...baseSession, planArtifactId: undefined };
    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={sessionNoPlan} />);

    await waitFor(() => {
      expect(screen.getByTestId("verification-empty-state")).toBeInTheDocument();
    });
    // No Verify First CTA without a plan artifact
    expect(screen.queryByTestId("verify-first-button")).not.toBeInTheDocument();
  });

  it("hydrates session query cache when query returns non-unverified status and session still shows unverified", async () => {
    const { useQuery } = await import("@tanstack/react-query");
    const verificationData = {
      sessionId: "session-1",
      status: "verified",
      inProgress: false,
      gaps: [],
      rounds: [{ round: 1, gapScore: 0, gapCount: 0 }],
    };
    // First call = verificationData query, second = childSessions query
    vi.mocked(useQuery)
      .mockReturnValueOnce({ data: verificationData } as ReturnType<typeof useQuery>)
      .mockReturnValueOnce({ data: [] } as unknown as ReturnType<typeof useQuery>);

    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={baseSession} />);

    // The useEffect should call queryClient.setQueryData to hydrate the session cache
    await waitFor(() => {
      expect(mockSetQueryData).toHaveBeenCalledWith(
        ["session", "session-1"],
        expect.any(Function)
      );
    });
  });

  it("does NOT hydrate session cache when session already has a non-unverified status", async () => {
    const { useQuery } = await import("@tanstack/react-query");
    const verificationData = {
      sessionId: "session-1",
      status: "verified",
      inProgress: false,
      gaps: [],
      rounds: [{ round: 1, gapScore: 0, gapCount: 0 }],
    };
    vi.mocked(useQuery)
      .mockReturnValueOnce({ data: verificationData } as ReturnType<typeof useQuery>)
      .mockReturnValueOnce({ data: [] } as unknown as ReturnType<typeof useQuery>);

    // Session already has a non-unverified status — hydration should be skipped
    const sessionAlreadyVerified: IdeationSession = {
      ...baseSession,
      verificationStatus: "verified",
    };
    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={sessionAlreadyVerified} />);

    await new Promise((r) => setTimeout(r, 50));
    expect(mockSetQueryData).not.toHaveBeenCalled();
  });
});
