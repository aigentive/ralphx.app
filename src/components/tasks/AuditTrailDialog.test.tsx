/**
 * AuditTrailDialog component tests
 *
 * Tests the dialog that renders a unified audit trail timeline
 * combining review notes and activity events for a task.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";
import { AuditTrailDialog } from "./AuditTrailDialog";
import type { AuditEntry } from "@/hooks/useAuditTrail";

// ============================================================================
// Mocks
// ============================================================================

const mockUseAuditTrail = vi.fn();

vi.mock("@/hooks/useAuditTrail", () => ({
  useAuditTrail: (...args: unknown[]) => mockUseAuditTrail(...args),
}));

// ============================================================================
// Test Helpers
// ============================================================================

function createMockAuditEntry(
  overrides: Partial<AuditEntry> = {}
): AuditEntry {
  return {
    id: "entry-1",
    source: "review",
    timestamp: "2026-02-23T10:00:00+00:00",
    type: "Approved",
    actor: "AI Reviewer",
    description: "Code review passed",
    ...overrides,
  };
}

function TestWrapper({ children }: { children: React.ReactNode }) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

function renderDialog(props: {
  isOpen: boolean;
  onClose?: () => void;
  taskId?: string;
}) {
  const { isOpen, onClose = vi.fn(), taskId = "task-123" } = props;
  return render(
    <AuditTrailDialog isOpen={isOpen} onClose={onClose} taskId={taskId} />,
    { wrapper: TestWrapper }
  );
}

// ============================================================================
// Tests
// ============================================================================

describe("AuditTrailDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseAuditTrail.mockReturnValue({
      entries: [],
      isLoading: false,
      isEmpty: true,
      error: null,
    });
  });

  it("renders nothing when isOpen=false", () => {
    renderDialog({ isOpen: false });

    // Dialog content should not be in the DOM
    expect(screen.queryByTestId("audit-trail-dialog")).not.toBeInTheDocument();
    expect(screen.queryByText("Audit Trail")).not.toBeInTheDocument();
  });

  it("shows loading spinner when data is loading", () => {
    mockUseAuditTrail.mockReturnValue({
      entries: [],
      isLoading: true,
      isEmpty: true,
      error: null,
    });

    renderDialog({ isOpen: true });

    expect(screen.getByTestId("audit-trail-loading")).toBeInTheDocument();
  });

  it("renders timeline entries when data is available", () => {
    const entries: AuditEntry[] = [
      createMockAuditEntry({
        id: "entry-1",
        source: "activity",
        type: "text",
        actor: "Agent",
        description: "Started execution",
        timestamp: "2026-02-23T09:00:00+00:00",
        status: "executing",
      }),
      createMockAuditEntry({
        id: "entry-2",
        source: "review",
        type: "Approved",
        actor: "AI Reviewer",
        description: "Review completed",
        timestamp: "2026-02-23T11:00:00+00:00",
      }),
    ];

    mockUseAuditTrail.mockReturnValue({
      entries,
      isLoading: false,
      isEmpty: false,
      error: null,
    });

    renderDialog({ isOpen: true });

    expect(screen.getByText("Started execution")).toBeInTheDocument();
    expect(screen.getByText("Review completed")).toBeInTheDocument();
  });

  it("shows empty state when no entries", () => {
    mockUseAuditTrail.mockReturnValue({
      entries: [],
      isLoading: false,
      isEmpty: true,
      error: null,
    });

    renderDialog({ isOpen: true });

    expect(screen.getByTestId("audit-trail-empty")).toBeInTheDocument();
  });

  it("closes when close button is clicked", async () => {
    const onClose = vi.fn();
    mockUseAuditTrail.mockReturnValue({
      entries: [],
      isLoading: false,
      isEmpty: true,
      error: null,
    });

    renderDialog({ isOpen: true, onClose });

    const closeButton = screen.getByTestId("dialog-close");
    await userEvent.click(closeButton);

    expect(onClose).toHaveBeenCalled();
  });

  it("displays full timestamps (not relative time)", () => {
    const entries: AuditEntry[] = [
      createMockAuditEntry({
        id: "entry-1",
        timestamp: "2026-02-23T14:30:45+00:00",
        description: "Test entry",
      }),
    ];

    mockUseAuditTrail.mockReturnValue({
      entries,
      isLoading: false,
      isEmpty: false,
      error: null,
    });

    renderDialog({ isOpen: true });

    // Full timestamp should be rendered (formatTimestamp uses toLocaleString with year/month/day/time)
    const content = document.body.textContent ?? "";
    expect(
      content.includes("2026") ||
        content.includes("14:30") ||
        content.includes("2:30")
    ).toBe(true);
  });

  it("shows source badges (Review vs Activity)", () => {
    const entries: AuditEntry[] = [
      createMockAuditEntry({
        id: "entry-1",
        source: "review",
        description: "Review entry",
      }),
      createMockAuditEntry({
        id: "entry-2",
        source: "activity",
        type: "text",
        actor: "Agent",
        description: "Activity entry",
      }),
    ];

    mockUseAuditTrail.mockReturnValue({
      entries,
      isLoading: false,
      isEmpty: false,
      error: null,
    });

    renderDialog({ isOpen: true });

    // SourceBadge renders "Review" and "Activity" labels
    expect(screen.getByText("Review")).toBeInTheDocument();
    expect(screen.getByText("Activity")).toBeInTheDocument();
  });

  it("passes taskId to useAuditTrail hook with enabled=true when open", () => {
    renderDialog({ isOpen: true, taskId: "task-456" });

    expect(mockUseAuditTrail).toHaveBeenCalledWith("task-456", { enabled: true });
  });

  it("passes enabled=false to hook when dialog is closed", () => {
    renderDialog({ isOpen: false, taskId: "task-456" });

    expect(mockUseAuditTrail).toHaveBeenCalledWith("task-456", { enabled: false });
  });

  it("renders timeline container when entries exist", () => {
    const entries: AuditEntry[] = [
      createMockAuditEntry({ id: "entry-1", description: "Something happened" }),
    ];

    mockUseAuditTrail.mockReturnValue({
      entries,
      isLoading: false,
      isEmpty: false,
      error: null,
    });

    renderDialog({ isOpen: true });

    expect(screen.getByTestId("audit-trail-timeline")).toBeInTheDocument();
  });

  it("displays actor information for entries", () => {
    const entries: AuditEntry[] = [
      createMockAuditEntry({
        id: "entry-1",
        actor: "AI Reviewer",
        description: "Reviewed code",
      }),
    ];

    mockUseAuditTrail.mockReturnValue({
      entries,
      isLoading: false,
      isEmpty: false,
      error: null,
    });

    renderDialog({ isOpen: true });

    expect(screen.getByText(/AI Reviewer/)).toBeInTheDocument();
  });

  it("shows status badge for entries with status", () => {
    const entries: AuditEntry[] = [
      createMockAuditEntry({
        id: "entry-1",
        source: "activity",
        type: "text",
        actor: "Agent",
        description: "Working on it",
        status: "executing",
      }),
    ];

    mockUseAuditTrail.mockReturnValue({
      entries,
      isLoading: false,
      isEmpty: false,
      error: null,
    });

    renderDialog({ isOpen: true });

    expect(screen.getByText("executing")).toBeInTheDocument();
  });

  it("shows metadata when present", () => {
    const entries: AuditEntry[] = [
      createMockAuditEntry({
        id: "entry-1",
        description: "Review note",
        metadata: "2 issues found",
      }),
    ];

    mockUseAuditTrail.mockReturnValue({
      entries,
      isLoading: false,
      isEmpty: false,
      error: null,
    });

    renderDialog({ isOpen: true });

    expect(screen.getByText("2 issues found")).toBeInTheDocument();
  });
});
