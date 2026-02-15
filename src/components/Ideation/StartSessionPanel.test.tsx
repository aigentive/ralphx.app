/**
 * StartSessionPanel component tests — team flow only
 *
 * Tests mode selector, TeamConfigPanel reveal, start handlers
 * (solo vs team paths), seed-from-task with team params, and error handling.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { StartSessionPanel } from "./StartSessionPanel";

// Mock dependencies
const mockMutateAsync = vi.fn();
vi.mock("@/hooks/useIdeation", () => ({
  useCreateIdeationSession: () => ({
    mutateAsync: mockMutateAsync,
    isPending: false,
  }),
}));

const mockAddSession = vi.fn();
const mockSetActiveSession = vi.fn();
vi.mock("@/stores/ideationStore", () => ({
  useIdeationStore: (selector: (s: Record<string, unknown>) => unknown) =>
    selector({
      addSession: mockAddSession,
      setActiveSession: mockSetActiveSession,
    }),
}));

let mockActiveProjectId: string | null = "project-1";
vi.mock("@/stores/projectStore", () => ({
  useProjectStore: (selector: (s: Record<string, unknown>) => unknown) =>
    selector({ activeProjectId: mockActiveProjectId }),
}));

// Mock sonner toast
const mockToastError = vi.fn();
vi.mock("sonner", () => ({
  toast: { error: (...args: unknown[]) => mockToastError(...args) },
}));

// Mock TaskPickerDialog to avoid complex rendering
vi.mock("./TaskPickerDialog", () => ({
  TaskPickerDialog: ({ isOpen, onSelect }: { isOpen: boolean; onClose: () => void; onSelect: (task: unknown) => void }) =>
    isOpen ? (
      <div data-testid="task-picker">
        <button data-testid="pick-task" onClick={() => onSelect({ id: "task-1", projectId: "project-1", title: "Mock Task" })}>
          Pick
        </button>
      </div>
    ) : null,
}));

describe("StartSessionPanel", () => {
  const onNewSession = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockActiveProjectId = "project-1";
    mockMutateAsync.mockResolvedValue({ id: "session-1", projectId: "project-1" });
  });

  describe("mode selector", () => {
    it("renders Solo, Research Team, and Debate Team buttons", () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);
      expect(screen.getByText("Solo")).toBeInTheDocument();
      expect(screen.getByText(/Research Team/)).toBeInTheDocument();
      expect(screen.getByText(/Debate Team/)).toBeInTheDocument();
    });

    it("defaults to solo mode", () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);
      expect(screen.getByText("Start New Session")).toBeInTheDocument();
    });

    it("switches button text when Research Team selected", () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);
      fireEvent.click(screen.getByText(/Research Team/));
      expect(screen.getByText("Start Research Session")).toBeInTheDocument();
    });

    it("switches button text when Debate Team selected", () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);
      fireEvent.click(screen.getByText(/Debate Team/));
      expect(screen.getByText("Start Debate Session")).toBeInTheDocument();
    });
  });

  describe("TeamConfigPanel animated reveal", () => {
    it("hides config panel in solo mode (maxHeight 0)", () => {
      const { container } = render(<StartSessionPanel onNewSession={onNewSession} />);
      const wrapper = container.querySelector(".overflow-hidden.transition-all");
      expect(wrapper).toHaveStyle({ maxHeight: "0px", opacity: "0" });
    });

    it("reveals config panel in team mode (maxHeight 280px)", () => {
      const { container } = render(<StartSessionPanel onNewSession={onNewSession} />);
      fireEvent.click(screen.getByText(/Research Team/));
      const wrapper = container.querySelector(".overflow-hidden.transition-all");
      expect(wrapper).toHaveStyle({ maxHeight: "280px", opacity: "1" });
    });
  });

  describe("handleStartSession", () => {
    it("calls onNewSession in solo mode (via ⌘N shortcut)", () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);
      // ⌘N keyboard shortcut triggers onNewSession when in solo mode
      fireEvent.keyDown(window, { key: "n", metaKey: true });
      expect(onNewSession).toHaveBeenCalledOnce();
      expect(mockMutateAsync).not.toHaveBeenCalled();
    });

    it("calls createSession.mutateAsync in team mode", async () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);
      fireEvent.click(screen.getByText(/Research Team/));

      await act(async () => {
        fireEvent.click(screen.getByText("Start Research Session"));
      });

      expect(mockMutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          projectId: "project-1",
          teamMode: "research",
          teamConfig: expect.objectContaining({ maxTeammates: 5 }),
        }),
      );
    });

    it("adds session to store and sets active on success", async () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);
      fireEvent.click(screen.getByText(/Research Team/));

      await act(async () => {
        fireEvent.click(screen.getByText("Start Research Session"));
      });

      expect(mockAddSession).toHaveBeenCalledWith({ id: "session-1", projectId: "project-1" });
      expect(mockSetActiveSession).toHaveBeenCalledWith("session-1");
    });

    it("shows toast error when no active project in team mode", async () => {
      mockActiveProjectId = null;
      render(<StartSessionPanel onNewSession={onNewSession} />);
      fireEvent.click(screen.getByText(/Debate Team/));

      await act(async () => {
        fireEvent.click(screen.getByText("Start Debate Session"));
      });

      expect(mockToastError).toHaveBeenCalledWith("No active project selected");
      expect(mockMutateAsync).not.toHaveBeenCalled();
    });

    it("shows toast error on createSession failure", async () => {
      mockMutateAsync.mockRejectedValueOnce(new Error("API error"));
      render(<StartSessionPanel onNewSession={onNewSession} />);
      fireEvent.click(screen.getByText(/Research Team/));

      await act(async () => {
        fireEvent.click(screen.getByText("Start Research Session"));
      });

      expect(mockToastError).toHaveBeenCalledWith("Failed to create session");
    });
  });

  describe("handleSeedFromTask", () => {
    it("includes team params when in team mode", async () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);
      fireEvent.click(screen.getByText(/Debate Team/));

      // Open task picker via secondary button
      fireEvent.click(screen.getByText("Seed from Draft Task"));

      await waitFor(() => {
        expect(screen.getByTestId("task-picker")).toBeInTheDocument();
      });

      await act(async () => {
        fireEvent.click(screen.getByTestId("pick-task"));
      });

      expect(mockMutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          projectId: "project-1",
          title: "Ideation: Mock Task",
          seedTaskId: "task-1",
          teamMode: "debate",
          teamConfig: expect.objectContaining({ compositionMode: "dynamic" }),
        }),
      );
    });

    it("omits team params when in solo mode", async () => {
      render(<StartSessionPanel onNewSession={onNewSession} />);

      fireEvent.click(screen.getByText("Seed from Draft Task"));

      await waitFor(() => {
        expect(screen.getByTestId("task-picker")).toBeInTheDocument();
      });

      await act(async () => {
        fireEvent.click(screen.getByTestId("pick-task"));
      });

      expect(mockMutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          projectId: "project-1",
          seedTaskId: "task-1",
        }),
      );
      // Should NOT have team params
      const callArgs = mockMutateAsync.mock.calls[0]![0];
      expect(callArgs.teamMode).toBeUndefined();
      expect(callArgs.teamConfig).toBeUndefined();
    });

    it("shows toast error on seed failure", async () => {
      mockMutateAsync.mockRejectedValueOnce(new Error("Seed fail"));
      render(<StartSessionPanel onNewSession={onNewSession} />);

      fireEvent.click(screen.getByText("Seed from Draft Task"));

      await waitFor(() => {
        expect(screen.getByTestId("task-picker")).toBeInTheDocument();
      });

      await act(async () => {
        fireEvent.click(screen.getByTestId("pick-task"));
      });

      expect(mockToastError).toHaveBeenCalledWith("Failed to start ideation session");
    });
  });
});
