/**
 * Tests for useIdeationQuickAction hook
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useIdeationQuickAction } from "./useIdeationQuickAction";
import { useCreateIdeationSession } from "./useIdeation";
import { useIdeationStore } from "@/stores/ideationStore";
import { useUiStore } from "@/stores/uiStore";
import { chatApi } from "@/api/chat";
import { ideationApi } from "@/api/ideation";

// Mock dependencies
vi.mock("./useIdeation");
vi.mock("@/stores/ideationStore");
vi.mock("@/stores/uiStore");
vi.mock("@/api/chat");
vi.mock("@/api/ideation");

describe("useIdeationQuickAction", () => {
  const projectId = "test-project-123";
  const mockSessionId = "test-session-456";

  let mockMutateAsync: ReturnType<typeof vi.fn>;
  let mockSelectSession: ReturnType<typeof vi.fn>;
  let mockSetActiveSession: ReturnType<typeof vi.fn>;
  let mockSetCurrentView: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();

    // Mock mutation
    mockMutateAsync = vi.fn().mockResolvedValue({
      id: mockSessionId,
      projectId,
      title: "Test Session",
      status: "active",
      createdAt: "2024-01-01T00:00:00Z",
      updatedAt: "2024-01-01T00:00:00Z",
    });

    vi.mocked(useCreateIdeationSession).mockReturnValue({
      mutateAsync: mockMutateAsync,
      isPending: false,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);

    // Mock stores
    mockSelectSession = vi.fn();
    mockSetActiveSession = vi.fn();
    mockSetCurrentView = vi.fn();

    vi.mocked(useIdeationStore).mockImplementation(<T,>(selector: (state: { selectSession: typeof mockSelectSession; setActiveSession: typeof mockSetActiveSession }) => T): T => {
      const store = {
        selectSession: mockSelectSession,
        setActiveSession: mockSetActiveSession,
      };
      return selector(store);
    });

    vi.mocked(useUiStore).mockImplementation(<T,>(selector: (state: { setCurrentView: typeof mockSetCurrentView }) => T): T => {
      const store = {
        setCurrentView: mockSetCurrentView,
      };
      return selector(store);
    });

    // Mock APIs
    vi.mocked(chatApi.sendAgentMessage).mockResolvedValue({
      conversationId: "conv-123",
      agentRunId: "run-123",
      isNewConversation: true,
    });

    vi.mocked(ideationApi.sessions.spawnSessionNamer).mockResolvedValue(undefined);
  });

  describe("action properties", () => {
    it("should have correct id", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));
      expect(result.current.id).toBe("ideation");
    });

    it("should have Lightbulb icon", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));
      expect(result.current.icon).toBeDefined();
      // Lucide icons are components, just verify it's defined and truthy
      expect(result.current.icon).toBeTruthy();
    });

    it("should have correct label", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));
      expect(result.current.label).toBe("Start new ideation session");
    });

    it("should have correct labels for creating/success/view", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));
      expect(result.current.creatingLabel).toBe("Creating your ideation session...");
      expect(result.current.successLabel).toBe("Session created!");
      expect(result.current.viewLabel).toBe("View Session");
    });
  });

  describe("isVisible", () => {
    it("should return true when query is not empty", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));
      expect(result.current.isVisible("test query")).toBe(true);
      expect(result.current.isVisible("a")).toBe(true);
    });

    it("should return false when query is empty", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));
      expect(result.current.isVisible("")).toBe(false);
      expect(result.current.isVisible("   ")).toBe(false);
    });

    it("should return false when query is only whitespace", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));
      expect(result.current.isVisible("  \t  \n  ")).toBe(false);
    });
  });

  describe("description", () => {
    it("should return query wrapped in quotes", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));
      expect(result.current.description("Build a user dashboard")).toBe('"Build a user dashboard"');
      expect(result.current.description("test")).toBe('"test"');
    });
  });

  describe("execute", () => {
    it("should create session and add to store", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));

      await result.current.execute("Build a user dashboard");

      expect(mockMutateAsync).toHaveBeenCalledWith({
        projectId,
      });

      await waitFor(() => {
        expect(mockSelectSession).toHaveBeenCalledWith({
          id: mockSessionId,
          projectId,
          title: "Test Session",
          status: "active",
          createdAt: "2024-01-01T00:00:00Z",
          updatedAt: "2024-01-01T00:00:00Z",
        });
      });
    });

    it("should send message in background (fire-and-forget)", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));

      await result.current.execute("Build a user dashboard");

      await waitFor(() => {
        expect(chatApi.sendAgentMessage).toHaveBeenCalledWith(
          "ideation",
          mockSessionId,
          "Build a user dashboard"
        );
      });
    });

    it("should spawn session namer in background (fire-and-forget)", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));

      await result.current.execute("Build a user dashboard");

      await waitFor(() => {
        expect(ideationApi.sessions.spawnSessionNamer).toHaveBeenCalledWith(
          mockSessionId,
          "Build a user dashboard"
        );
      });
    });

    it("should not throw if background operations fail", async () => {
      vi.mocked(chatApi.sendAgentMessage).mockRejectedValue(new Error("Network error"));
      vi.mocked(ideationApi.sessions.spawnSessionNamer).mockRejectedValue(new Error("Network error"));

      const { result } = renderHook(() => useIdeationQuickAction(projectId));

      await expect(result.current.execute("Build a user dashboard")).resolves.toEqual(mockSessionId);
    });

    it("should return session ID", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));

      const sessionId = await result.current.execute("Build a user dashboard");

      expect(sessionId).toBe(mockSessionId);
    });
  });

  describe("navigateTo", () => {
    it("should set active session and switch to ideation view", () => {
      const { result } = renderHook(() => useIdeationQuickAction(projectId));

      result.current.navigateTo(mockSessionId);

      expect(mockSetActiveSession).toHaveBeenCalledWith(mockSessionId);
      expect(mockSetCurrentView).toHaveBeenCalledWith("ideation");
    });
  });

  describe("memoization", () => {
    it("should return same action object on re-render when deps don't change", () => {
      const { result, rerender } = renderHook(() => useIdeationQuickAction(projectId));

      const firstResult = result.current;
      rerender();
      const secondResult = result.current;

      expect(firstResult).toBe(secondResult);
    });
  });
});
