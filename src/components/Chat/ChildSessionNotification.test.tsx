/**
 * ChildSessionNotification component tests
 *
 * Tests verification notification rendering, reconciliation effect (auto-clear on terminal states),
 * and dismiss button. General follow-up navigation is handled by ChildSessionWidget.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { ChildSessionNotification } from "./ChildSessionNotification";
import type { VerificationStatus } from "@/types/ideation";

// ---- Store mock state --------------------------------------------------------

interface MockSessionState {
  verificationStatus: VerificationStatus;
  verificationInProgress: boolean;
}

let mockVerificationNotifications: Record<string, string> = {};
let mockSessionState: MockSessionState | null = null;
const mockSetActiveIdeationTab = vi.fn();
const mockSetActiveVerificationChildId = vi.fn();
const mockClearVerificationNotification = vi.fn();

vi.mock("@/stores/ideationStore", () => ({
  useIdeationStore: (selector: (s: Record<string, unknown>) => unknown) =>
    selector({
      verificationNotifications: mockVerificationNotifications,
      setActiveIdeationTab: mockSetActiveIdeationTab,
      setActiveVerificationChildId: mockSetActiveVerificationChildId,
      clearVerificationNotification: mockClearVerificationNotification,
      sessions: mockSessionState
        ? { "session-1": mockSessionState }
        : {},
    }),
}));

// useShallow just wraps the selector — return identity so tests work normally
vi.mock("zustand/react/shallow", () => ({
  useShallow: (selector: (s: unknown) => unknown) => selector,
}));

// EventBus mock — no events needed for these focused tests
vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: vi.fn(() => vi.fn()),
  }),
}));

// ---- Helpers -----------------------------------------------------------------

const SESSION_ID = "session-1";
const CHILD_ID = "child-session-abc";

function renderNotification() {
  return render(
    <ChildSessionNotification
      sessionId={SESSION_ID}
    />,
  );
}

// ---- Tests -------------------------------------------------------------------

describe("ChildSessionNotification — verification notification", () => {
  beforeEach(() => {
    mockVerificationNotifications = {};
    mockSessionState = null;
    vi.clearAllMocks();
  });

  it("renders nothing when no verification notification and no child sessions", () => {
    const { container } = renderNotification();
    expect(container.firstChild).toBeNull();
  });

  it("renders notification banner when verificationChildId is set", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    renderNotification();
    expect(screen.getByTestId("verification-started-notification")).toBeInTheDocument();
    expect(screen.getByText("Verification started")).toBeInTheDocument();
    expect(screen.getByTestId("view-verification-button")).toBeInTheDocument();
  });

  it("renders dismiss button on the verification notification", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    renderNotification();
    expect(screen.getByTestId("dismiss-verification-button")).toBeInTheDocument();
  });

  it("dismiss button calls clearVerificationNotification on click", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    renderNotification();
    fireEvent.click(screen.getByTestId("dismiss-verification-button"));
    expect(mockClearVerificationNotification).toHaveBeenCalledWith(SESSION_ID);
  });

  it("View button calls setActiveIdeationTab and setActiveVerificationChildId", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    renderNotification();
    fireEvent.click(screen.getByTestId("view-verification-button"));
    expect(mockSetActiveIdeationTab).toHaveBeenCalledWith(SESSION_ID, "verification");
    expect(mockSetActiveVerificationChildId).toHaveBeenCalledWith(SESSION_ID, CHILD_ID);
  });
});

describe("ChildSessionNotification — reconciliation effect (terminal state auto-clear)", () => {
  beforeEach(() => {
    mockVerificationNotifications = {};
    mockSessionState = null;
    vi.clearAllMocks();
  });

  it("auto-clears notification when session is verified", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = { verificationStatus: "verified", verificationInProgress: false };
    renderNotification();
    expect(mockClearVerificationNotification).toHaveBeenCalledWith(SESSION_ID);
  });

  it("auto-clears notification when session is needs_revision", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = { verificationStatus: "needs_revision", verificationInProgress: false };
    renderNotification();
    expect(mockClearVerificationNotification).toHaveBeenCalledWith(SESSION_ID);
  });

  it("auto-clears notification when session is skipped", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = { verificationStatus: "skipped", verificationInProgress: false };
    renderNotification();
    expect(mockClearVerificationNotification).toHaveBeenCalledWith(SESSION_ID);
  });

  it("auto-clears notification when session is imported_verified", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = { verificationStatus: "imported_verified", verificationInProgress: false };
    renderNotification();
    expect(mockClearVerificationNotification).toHaveBeenCalledWith(SESSION_ID);
  });

  it("does NOT auto-clear when verification is in reviewing state with inProgress=true", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = { verificationStatus: "reviewing", verificationInProgress: true };
    renderNotification();
    expect(mockClearVerificationNotification).not.toHaveBeenCalled();
  });

  it("does NOT auto-clear when verification is reviewing but inProgress flag is true", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = { verificationStatus: "reviewing", verificationInProgress: true };
    renderNotification();
    expect(screen.getByTestId("verification-started-notification")).toBeInTheDocument();
    expect(mockClearVerificationNotification).not.toHaveBeenCalled();
  });

  it("does NOT auto-clear when session is unverified (should not have notification but guards correctly)", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = { verificationStatus: "unverified", verificationInProgress: false };
    renderNotification();
    // unverified is excluded from terminal condition — notification persists
    expect(mockClearVerificationNotification).not.toHaveBeenCalled();
  });

  it("skips effect when session is not in the store (null state)", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = null; // session not loaded yet
    renderNotification();
    expect(mockClearVerificationNotification).not.toHaveBeenCalled();
  });

  it("skips effect when no verificationChildId is set", () => {
    mockVerificationNotifications = {}; // no notification
    mockSessionState = { verificationStatus: "verified", verificationInProgress: false };
    renderNotification();
    expect(mockClearVerificationNotification).not.toHaveBeenCalled();
  });

  it("auto-clears when store session transitions to terminal state (re-render)", () => {
    mockVerificationNotifications = { [SESSION_ID]: CHILD_ID };
    mockSessionState = { verificationStatus: "reviewing", verificationInProgress: true };
    const { rerender } = renderNotification();
    expect(mockClearVerificationNotification).not.toHaveBeenCalled();

    // Simulate store update: verification completes
    mockSessionState = { verificationStatus: "verified", verificationInProgress: false };
    act(() => {
      rerender(
        <ChildSessionNotification
          sessionId={SESSION_ID}
        />,
      );
    });

    expect(mockClearVerificationNotification).toHaveBeenCalledWith(SESSION_ID);
  });
});
