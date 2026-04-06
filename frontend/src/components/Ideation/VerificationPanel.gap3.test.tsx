/**
 * VerificationPanel.gap3.test.tsx
 *
 * Gap 3: Verify the backfill useEffect in VerificationPanel correctly hydrates
 * the chatStore effectiveModel for the verification child session on page-load/reopen.
 *
 * Test cases:
 *   (a) model set → store populated at child storeKey
 *   (b) store already set → no overwrite (live event wins)
 *   (c) lastEffectiveModel: null → no write
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, waitFor } from "@testing-library/react";
import { useChatStore } from "@/stores/chatStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import type { IdeationSession } from "@/types/ideation";

// ── Mock @tanstack/react-query ────────────────────────────────────────────────

vi.mock("@tanstack/react-query", () => ({
  useQuery: vi.fn(),
  useQueryClient: vi.fn(() => ({ setQueryData: vi.fn() })),
}));

// ── Mock API ──────────────────────────────────────────────────────────────────

vi.mock("@/api/ideation", () => ({
  ideationApi: {
    verification: { getStatus: vi.fn(), skip: vi.fn() },
    sessions: { getChildren: vi.fn().mockResolvedValue([]) },
  },
}));

vi.mock("@/hooks/useIdeation", () => ({
  ideationKeys: {
    sessions: () => ["sessions"],
    sessionWithData: (id: string) => ["session", id],
  },
}));

vi.mock("@/api/chat", () => ({
  chatApi: { sendAgentMessage: vi.fn() },
}));

// ── Mock useChildSessionStatus ────────────────────────────────────────────────

let mockLastEffectiveModel: string | null = null;

vi.mock("@/hooks/useChildSessionStatus", () => ({
  useChildSessionStatus: vi.fn(() => ({
    data: undefined,
    isLoading: false,
    lastEffectiveModel: mockLastEffectiveModel,
  })),
}));

// ── Mock ideationStore ────────────────────────────────────────────────────────

const CHILD_ID = "child-session-abc";

const mockSetActiveVerificationChildId = vi.fn();
const mockSetLastVerificationChildId = vi.fn();

let mockStoreState: Record<string, unknown> = {};

vi.mock("@/stores/ideationStore", () => ({
  useIdeationStore: vi.fn((selector: (s: object) => unknown) =>
    selector(mockStoreState)
  ),
}));

// ── Mock uiStore ──────────────────────────────────────────────────────────────

vi.mock("@/stores/uiStore", () => ({
  useUiStore: vi.fn((selector: (s: object) => unknown) =>
    selector({ enqueuePendingVerification: vi.fn() })
  ),
}));

// ── Mock sub-components ───────────────────────────────────────────────────────

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
  id: "session-parent-1",
  projectId: "proj-1",
  title: "My Plan",
  status: "active",
  sessionPurpose: "ideation",
  verificationStatus: "verified",
  verificationInProgress: false,
  planArtifactId: "artifact-1",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const CHILD_STORE_KEY = buildStoreKey("ideation", CHILD_ID);

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("VerificationPanel — Gap 3: effectiveModel backfill for verification child", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    mockLastEffectiveModel = null;
    // Reset effectiveModel slice of real chatStore between tests
    useChatStore.setState({ effectiveModel: {} });
    mockStoreState = {
      activeVerificationChildId: { [baseSession.id]: CHILD_ID },
      setActiveVerificationChildId: mockSetActiveVerificationChildId,
      lastVerificationChildId: { [baseSession.id]: CHILD_ID },
      setLastVerificationChildId: mockSetLastVerificationChildId,
      setVerificationNotification: vi.fn(),
    };
    // VerificationPanel calls useQuery twice: verification data + child sessions.
    // Return null for verification data and [] for child sessions (to keep component renderable).
    const { useQuery } = await import("@tanstack/react-query");
    vi.mocked(useQuery)
      .mockReturnValueOnce({ data: null, isLoading: false } as ReturnType<typeof useQuery>)
      .mockReturnValue({ data: [], isLoading: false } as unknown as ReturnType<typeof useQuery>);
  });

  it("(a) populates store at child storeKey when lastEffectiveModel is set", async () => {
    mockLastEffectiveModel = "claude-sonnet-4-6";

    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={baseSession} />);

    await waitFor(() => {
      const stored = useChatStore.getState().effectiveModel[CHILD_STORE_KEY];
      expect(stored).toBeDefined();
      expect(stored?.id).toBe("claude-sonnet-4-6");
    });
  });

  it("(b) does not overwrite when store key is already populated (live event wins)", async () => {
    mockLastEffectiveModel = "claude-haiku-4-5-20251001";

    // Pre-populate the store with a live-event model before rendering
    useChatStore.setState({
      effectiveModel: {
        [CHILD_STORE_KEY]: { id: "claude-opus-4-6", label: "Opus 4.6" },
      },
    });

    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={baseSession} />);

    // Wait a tick for any effects to flush
    await waitFor(() => {
      const stored = useChatStore.getState().effectiveModel[CHILD_STORE_KEY];
      // Must still be the live-event value, not the backfill value
      expect(stored?.id).toBe("claude-opus-4-6");
    });
  });

  it("(c) does not write to store when lastEffectiveModel is null", async () => {
    mockLastEffectiveModel = null;

    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={baseSession} />);

    await waitFor(() => {
      const stored = useChatStore.getState().effectiveModel[CHILD_STORE_KEY];
      expect(stored).toBeUndefined();
    });
  });

  it("does not write to parent session storeKey", async () => {
    mockLastEffectiveModel = "claude-sonnet-4-6";

    const { VerificationPanel } = await import("./VerificationPanel");
    render(<VerificationPanel session={baseSession} />);

    const parentStoreKey = buildStoreKey("ideation", baseSession.id);

    await waitFor(() => {
      // Child key should be set
      expect(useChatStore.getState().effectiveModel[CHILD_STORE_KEY]).toBeDefined();
    });
    // Parent key must NOT be populated by the backfill effect
    expect(useChatStore.getState().effectiveModel[parentStoreKey]).toBeUndefined();
  });
});
