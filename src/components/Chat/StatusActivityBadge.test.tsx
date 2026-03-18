/**
 * StatusActivityBadge tests
 *
 * Tests for the "Last activity X ago" indicator with progressive color coding.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { StatusActivityBadge } from "./StatusActivityBadge";

// ============================================================================
// Store mocks
// ============================================================================

const mockLastAgentEventTimestamp: Record<string, number> = {};

vi.mock("@/stores/chatStore", () => ({
  useChatStore: vi.fn((selector: (state: { lastAgentEventTimestamp: Record<string, number> }) => unknown) => {
    return selector({ lastAgentEventTimestamp: mockLastAgentEventTimestamp });
  }),
}));

vi.mock("@/stores/uiStore", () => ({
  useUiStore: vi.fn((selector: (state: { setActivityFilter: () => void; setCurrentView: () => void }) => unknown) =>
    selector({ setActivityFilter: vi.fn(), setCurrentView: vi.fn() })
  ),
}));

// ============================================================================
// Helpers
// ============================================================================

const baseProps = {
  isAgentActive: true,
  agentType: "agent" as const,
  contextType: "task_execution" as const,
  contextId: "task-123",
  agentStatus: "generating" as const,
};

function setLastEvent(storeKey: string, msAgo: number) {
  mockLastAgentEventTimestamp[storeKey] = Date.now() - msAgo;
}

// ============================================================================
// Tests
// ============================================================================

describe("StatusActivityBadge", () => {
  beforeEach(() => {
    // Clear timestamps between tests
    for (const key of Object.keys(mockLastAgentEventTimestamp)) {
      delete mockLastAgentEventTimestamp[key];
    }
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders without 'Last activity' when no storeKey provided", () => {
    render(<StatusActivityBadge {...baseProps} />);
    expect(screen.queryByText(/Last:/)).toBeNull();
  });

  it("renders without 'Last activity' when idle (not generating)", () => {
    render(
      <StatusActivityBadge
        {...baseProps}
        agentStatus="idle"
        isAgentActive={false}
        hasActivity={true}
        storeKey="task_execution:task-123"
      />
    );
    // Should show the muted activity icon (not the badge with Last:)
    expect(screen.queryByText(/Last:/)).toBeNull();
  });

  it("renders without 'Last activity' when storeKey provided but timestamp is 0", () => {
    render(<StatusActivityBadge {...baseProps} storeKey="task_execution:task-123" />);
    // lastAgentEventTimestamp["task_execution:task-123"] is undefined → 0 → no display
    expect(screen.queryByText(/Last:/)).toBeNull();
  });

  it("renders 'Last: Xs ago' with green color when activity is less than 1 minute ago", () => {
    const storeKey = "task_execution:task-123";
    setLastEvent(storeKey, 30_000); // 30 seconds ago

    render(<StatusActivityBadge {...baseProps} storeKey={storeKey} />);

    const lastActivity = screen.getByText(/Last: 30s ago/);
    expect(lastActivity).toBeDefined();
    expect(lastActivity.className).toContain("text-green-400");
  });

  it("renders 'Last: Xm ago' with yellow color at 1-3 minutes", () => {
    const storeKey = "task_execution:task-123";
    setLastEvent(storeKey, 90_000); // 90 seconds = 1m 30s ago

    render(<StatusActivityBadge {...baseProps} storeKey={storeKey} />);

    const lastActivity = screen.getByText(/Last: 1m 30s ago/);
    expect(lastActivity).toBeDefined();
    expect(lastActivity.className).toContain("text-yellow-400");
  });

  it("renders with yellow color at exactly 1 minute (60 seconds)", () => {
    const storeKey = "task_execution:task-123";
    setLastEvent(storeKey, 60_000); // exactly 60 seconds ago

    render(<StatusActivityBadge {...baseProps} storeKey={storeKey} />);

    const lastActivity = screen.getByText(/Last: 1m ago/);
    expect(lastActivity).toBeDefined();
    expect(lastActivity.className).toContain("text-yellow-400");
  });

  it("renders with red color at 3-5 minutes", () => {
    const storeKey = "task_execution:task-123";
    setLastEvent(storeKey, 240_000); // 240 seconds = 4 minutes ago

    render(<StatusActivityBadge {...baseProps} storeKey={storeKey} />);

    const lastActivity = screen.getByText(/Last: 4m ago/);
    expect(lastActivity).toBeDefined();
    expect(lastActivity.className).toContain("text-red-400");
  });

  it("renders with red color at exactly 3 minutes (180 seconds)", () => {
    const storeKey = "task_execution:task-123";
    setLastEvent(storeKey, 180_000); // exactly 180 seconds ago

    render(<StatusActivityBadge {...baseProps} storeKey={storeKey} />);

    const lastActivity = screen.getByText(/Last: 3m ago/);
    expect(lastActivity).toBeDefined();
    expect(lastActivity.className).toContain("text-red-400");
  });

  it("updates elapsed time via setInterval every second", () => {
    const storeKey = "task_execution:task-123";
    setLastEvent(storeKey, 30_000); // 30 seconds ago

    render(<StatusActivityBadge {...baseProps} storeKey={storeKey} />);

    // Initially shows 30s
    expect(screen.getByText(/Last: 30s ago/)).toBeDefined();

    // Advance time by 10 seconds
    act(() => {
      // Move lastEvent back by 10 more seconds and advance fake timers
      mockLastAgentEventTimestamp[storeKey] -= 10_000;
      vi.advanceTimersByTime(10_000);
    });

    // Should now show ~40s ago
    expect(screen.getByText(/Last: 40s ago/)).toBeDefined();
  });

  it("returns null when idle with no activity", () => {
    const { container } = render(
      <StatusActivityBadge
        {...baseProps}
        isAgentActive={false}
        agentStatus="idle"
        hasActivity={false}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it("shows status badge text during generating state", () => {
    const storeKey = "task_execution:task-123";
    setLastEvent(storeKey, 5_000); // 5 seconds ago

    render(<StatusActivityBadge {...baseProps} storeKey={storeKey} />);

    expect(screen.getByText("Agent responding...")).toBeDefined();
  });

  it("shows worker text for AGENT_WORKER type", () => {
    const storeKey = "task_execution:task-123";
    setLastEvent(storeKey, 5_000);

    render(<StatusActivityBadge {...baseProps} agentType="worker" storeKey={storeKey} />);

    expect(screen.getByText("Worker running...")).toBeDefined();
  });
});
