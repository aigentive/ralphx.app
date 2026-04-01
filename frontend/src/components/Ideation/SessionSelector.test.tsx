/**
 * SessionSelector component tests
 *
 * Tests for:
 * - Rendering current session info
 * - Dropdown with session list
 * - Session status indicators
 * - New session button
 * - Archive action per session
 * - Empty state handling
 * - Accessibility
 * - Styling
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { SessionSelector } from "./SessionSelector";
import type { IdeationSession } from "@/types/ideation";

// ============================================================================
// Test Data
// ============================================================================

const createMockSession = (overrides: Partial<IdeationSession> = {}): IdeationSession => ({
  id: "session-1",
  projectId: "project-1",
  title: "Test Session",
  status: "active",
  createdAt: "2026-01-24T10:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  archivedAt: null,
  acceptedAt: null,
  ...overrides,
});

const mockSessions: IdeationSession[] = [
  createMockSession({ id: "session-1", title: "Session 1", status: "active" }),
  createMockSession({ id: "session-2", title: "Session 2", status: "archived" }),
  createMockSession({ id: "session-3", title: "Session 3", status: "accepted" }),
  createMockSession({ id: "session-4", title: null, status: "active" }),
];

describe("SessionSelector", () => {
  const defaultProps = {
    sessions: mockSessions,
    currentSession: mockSessions[0],
    onSelectSession: vi.fn(),
    onNewSession: vi.fn(),
    onArchiveSession: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<SessionSelector {...defaultProps} />);
      expect(screen.getByTestId("session-selector")).toBeInTheDocument();
    });

    it("displays current session title", () => {
      render(<SessionSelector {...defaultProps} />);
      expect(screen.getByText("Session 1")).toBeInTheDocument();
    });

    it("displays 'New Session' for session with null title", () => {
      render(
        <SessionSelector {...defaultProps} currentSession={mockSessions[3]} />
      );
      expect(screen.getByTestId("current-session-title")).toHaveTextContent("New Session");
    });

    it("renders dropdown trigger button", () => {
      render(<SessionSelector {...defaultProps} />);
      expect(screen.getByTestId("dropdown-trigger")).toBeInTheDocument();
    });

    it("renders new session button", () => {
      render(<SessionSelector {...defaultProps} />);
      expect(screen.getByRole("button", { name: /new session/i })).toBeInTheDocument();
    });

    it("hides dropdown initially", () => {
      render(<SessionSelector {...defaultProps} />);
      expect(screen.queryByTestId("session-dropdown")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Dropdown Behavior
  // ==========================================================================

  describe("dropdown behavior", () => {
    it("opens dropdown when trigger is clicked", () => {
      render(<SessionSelector {...defaultProps} />);
      const trigger = screen.getByTestId("dropdown-trigger");
      fireEvent.click(trigger);
      expect(screen.getByTestId("session-dropdown")).toBeInTheDocument();
    });

    it("closes dropdown when trigger is clicked again", () => {
      render(<SessionSelector {...defaultProps} />);
      const trigger = screen.getByTestId("dropdown-trigger");
      fireEvent.click(trigger);
      fireEvent.click(trigger);
      expect(screen.queryByTestId("session-dropdown")).not.toBeInTheDocument();
    });

    it("displays all sessions in dropdown", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const dropdown = screen.getByTestId("session-dropdown");
      expect(within(dropdown).getByText("Session 1")).toBeInTheDocument();
      expect(within(dropdown).getByText("Session 2")).toBeInTheDocument();
      expect(within(dropdown).getByText("Session 3")).toBeInTheDocument();
    });

    it("displays 'New Session' for untitled sessions in dropdown", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const dropdown = screen.getByTestId("session-dropdown");
      // Session 4 has null title
      const sessionItems = within(dropdown).getAllByTestId("session-item");
      expect(sessionItems[3]).toHaveTextContent("New Session");
    });

    it("closes dropdown when clicking outside", () => {
      render(
        <div>
          <div data-testid="outside">Outside</div>
          <SessionSelector {...defaultProps} />
        </div>
      );
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getByTestId("session-dropdown")).toBeInTheDocument();

      fireEvent.mouseDown(screen.getByTestId("outside"));
      expect(screen.queryByTestId("session-dropdown")).not.toBeInTheDocument();
    });

    it("closes dropdown when Escape is pressed", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getByTestId("session-dropdown")).toBeInTheDocument();

      fireEvent.keyDown(document, { key: "Escape" });
      expect(screen.queryByTestId("session-dropdown")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Session Selection
  // ==========================================================================

  describe("session selection", () => {
    it("calls onSelectSession when session is clicked", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const dropdown = screen.getByTestId("session-dropdown");
      const session2 = within(dropdown).getByText("Session 2");
      fireEvent.click(session2);

      expect(defaultProps.onSelectSession).toHaveBeenCalledWith("session-2");
    });

    it("closes dropdown after selection", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const dropdown = screen.getByTestId("session-dropdown");
      fireEvent.click(within(dropdown).getByText("Session 2"));

      expect(screen.queryByTestId("session-dropdown")).not.toBeInTheDocument();
    });

    it("highlights current session in dropdown", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      expect(sessionItems[0]).toHaveAttribute("data-current", "true");
      expect(sessionItems[1]).toHaveAttribute("data-current", "false");
    });
  });

  // ==========================================================================
  // Session Status Indicators
  // ==========================================================================

  describe("session status indicators", () => {
    it("shows active status indicator", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      expect(within(sessionItems[0]).getByTestId("status-indicator")).toHaveAttribute(
        "data-status",
        "active"
      );
    });

    it("shows archived status indicator", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      expect(within(sessionItems[1]).getByTestId("status-indicator")).toHaveAttribute(
        "data-status",
        "archived"
      );
    });

    it("shows accepted status indicator", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      expect(within(sessionItems[2]).getByTestId("status-indicator")).toHaveAttribute(
        "data-status",
        "accepted"
      );
    });

    it("uses correct color for active status", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      const indicator = within(sessionItems[0]).getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "var(--status-success)" });
    });

    it("uses correct color for archived status", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      const indicator = within(sessionItems[1]).getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "var(--text-muted)" });
    });

    it("uses correct color for accepted status", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      const indicator = within(sessionItems[2]).getByTestId("status-indicator");
      expect(indicator).toHaveStyle({ backgroundColor: "var(--status-info)" });
    });
  });

  // ==========================================================================
  // New Session Button
  // ==========================================================================

  describe("new session button", () => {
    it("calls onNewSession when clicked", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByRole("button", { name: /new session/i }));
      expect(defaultProps.onNewSession).toHaveBeenCalled();
    });

    it("is disabled when isLoading is true", () => {
      render(<SessionSelector {...defaultProps} isLoading />);
      const button = screen.getByRole("button", { name: /new session/i });
      expect(button).toBeDisabled();
    });
  });

  // ==========================================================================
  // Archive Action
  // ==========================================================================

  describe("archive action", () => {
    it("shows archive button for non-archived sessions", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      // Session 1 is active, should have archive button
      expect(within(sessionItems[0]).getByRole("button", { name: /archive/i })).toBeInTheDocument();
    });

    it("does not show archive button for archived sessions", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      // Session 2 is archived, should not have archive button
      expect(within(sessionItems[1]).queryByRole("button", { name: /archive/i })).not.toBeInTheDocument();
    });

    it("does not show archive button for accepted sessions", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      // Session 3 is accepted, should not have archive button
      expect(within(sessionItems[2]).queryByRole("button", { name: /archive/i })).not.toBeInTheDocument();
    });

    it("calls onArchiveSession when archive button is clicked", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      const archiveButton = within(sessionItems[0]).getByRole("button", { name: /archive/i });
      fireEvent.click(archiveButton);

      expect(defaultProps.onArchiveSession).toHaveBeenCalledWith("session-1");
    });

    it("does not close dropdown when archive button is clicked", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      const archiveButton = within(sessionItems[0]).getByRole("button", { name: /archive/i });
      fireEvent.click(archiveButton);

      // Dropdown should still be open
      expect(screen.getByTestId("session-dropdown")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Empty State
  // ==========================================================================

  describe("empty state", () => {
    it("shows message when no sessions exist", () => {
      render(<SessionSelector {...defaultProps} sessions={[]} currentSession={null} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      expect(screen.getByText(/no sessions/i)).toBeInTheDocument();
    });

    it("shows placeholder when no current session", () => {
      render(<SessionSelector {...defaultProps} currentSession={null} />);
      expect(screen.getByTestId("current-session-title")).toHaveTextContent("Select Session");
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("has proper aria attributes on dropdown trigger", () => {
      render(<SessionSelector {...defaultProps} />);
      const trigger = screen.getByTestId("dropdown-trigger");
      expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
      expect(trigger).toHaveAttribute("aria-expanded", "false");
    });

    it("updates aria-expanded when dropdown is open", () => {
      render(<SessionSelector {...defaultProps} />);
      const trigger = screen.getByTestId("dropdown-trigger");
      fireEvent.click(trigger);
      expect(trigger).toHaveAttribute("aria-expanded", "true");
    });

    it("dropdown has listbox role", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    it("session items have option role", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      expect(screen.getAllByRole("option")).toHaveLength(4);
    });

    it("current session option has aria-selected", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const options = screen.getAllByRole("option");
      expect(options[0]).toHaveAttribute("aria-selected", "true");
      expect(options[1]).toHaveAttribute("aria-selected", "false");
    });

    it("archive buttons have descriptive aria-label", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));

      const sessionItems = screen.getAllByTestId("session-item");
      const archiveButton = within(sessionItems[0]).getByRole("button", { name: /archive/i });
      expect(archiveButton).toHaveAttribute("aria-label", "Archive Session 1");
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<SessionSelector {...defaultProps} />);
      const selector = screen.getByTestId("session-selector");
      expect(selector).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses design tokens for dropdown background", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const dropdown = screen.getByTestId("session-dropdown");
      expect(dropdown).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("uses design tokens for text colors", () => {
      render(<SessionSelector {...defaultProps} />);
      const title = screen.getByTestId("current-session-title");
      expect(title).toHaveStyle({ color: "var(--text-primary)" });
    });

    it("uses design tokens for border", () => {
      render(<SessionSelector {...defaultProps} />);
      fireEvent.click(screen.getByTestId("dropdown-trigger"));
      const dropdown = screen.getByTestId("session-dropdown");
      // Check border is defined via style attribute
      expect(dropdown.getAttribute("style")).toContain("border-color: var(--border-subtle)");
    });
  });

  // ==========================================================================
  // Loading State
  // ==========================================================================

  describe("loading state", () => {
    it("disables dropdown when loading", () => {
      render(<SessionSelector {...defaultProps} isLoading />);
      const trigger = screen.getByTestId("dropdown-trigger");
      expect(trigger).toBeDisabled();
    });

    it("shows loading indicator when isLoading", () => {
      render(<SessionSelector {...defaultProps} isLoading />);
      expect(screen.getByTestId("loading-indicator")).toBeInTheDocument();
    });
  });
});
