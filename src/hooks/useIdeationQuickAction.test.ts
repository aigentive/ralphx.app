/**
 * Tests for useIdeationQuickAction hook
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { Lightbulb } from "lucide-react";
import { useIdeationQuickAction } from "./useIdeationQuickAction";
import * as useIdeationModule from "./useIdeation";
import * as ideationStoreModule from "@/stores/ideationStore";
import * as uiStoreModule from "@/stores/uiStore";
import * as chatApiModule from "@/api/chat";
import * as ideationApiModule from "@/api/ideation";
import type { IdeationSession } from "@/types/ideation";

// Mock modules
vi.mock("./useIdeation");
vi.mock("@/stores/ideationStore");
vi.mock("@/stores/uiStore");
vi.mock("@/api/chat");
vi.mock("@/api/ideation");

describe("useIdeationQuickAction", () => {
  const mockProjectId = "project-123";
  const mockSessionId = "session-456";
  const mockSession: Partial<IdeationSession> = {
    id: mockSessionId,
    projectId: mockProjectId,
    title: null,
    status: "active",
  };

  let mockMutateAsync: ReturnType<typeof vi.fn>;
  let mockAddSession: ReturnType<typeof vi.fn>;
  let mockSetActiveSession: ReturnType<typeof vi.fn>;
  let mockSetCurrentView: ReturnType<typeof vi.fn>;
  let mockSendAgentMessage: ReturnType<typeof vi.fn>;
  let mockSpawnSessionNamer: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();

    // Mock mutation
    mockMutateAsync = vi.fn().mockResolvedValue(mockSession);
    vi.spyOn(useIdeationModule, "useCreateIdeationSession").mockReturnValue({
      mutateAsync: mockMutateAsync,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);

    // Mock store actions
    mockAddSession = vi.fn();
    mockSetActiveSession = vi.fn();
    mockSetCurrentView = vi.fn();

    vi.spyOn(ideationStoreModule, "useIdeationStore").mockImplementation(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (selector: any) => {
        if (selector.name === "addSession" || selector.toString().includes("addSession")) {
          return mockAddSession;
        }
        if (selector.name === "setActiveSession" || selector.toString().includes("setActiveSession")) {
          return mockSetActiveSession;
        }
        return vi.fn();
      }
    );

    vi.spyOn(uiStoreModule, "useUiStore").mockImplementation(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (selector: any) => {
      if (selector.name === "setCurrentView" || selector.toString().includes("setCurrentView")) {
        return mockSetCurrentView;
      }
      return vi.fn();
    });

    // Mock API calls
    mockSendAgentMessage = vi.fn().mockResolvedValue({ id: "msg-1" });
    mockSpawnSessionNamer = vi.fn().mockResolvedValue(undefined);

    vi.spyOn(chatApiModule, "sendAgentMessage").mockImplementation(mockSendAgentMessage);
    vi.spyOn(ideationApiModule.ideationApi.sessions, "spawnSessionNamer").mockImplementation(
      mockSpawnSessionNamer
    );
  });

  describe("QuickAction properties", () => {
    it("should return a QuickAction with correct id and label", () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      expect(result.current.id).toBe("ideation");
      expect(result.current.label).toBe("Start new ideation session");
    });

    it("should use Lightbulb icon", () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      expect(result.current.icon).toBe(Lightbulb);
    });

    it("should have correct flow labels", () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      expect(result.current.creatingLabel).toBe("Creating your ideation session...");
      expect(result.current.successLabel).toBe("Session created!");
      expect(result.current.viewLabel).toBe("View Session");
    });
  });

  describe("description", () => {
    it("should wrap query in quotes", () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      expect(result.current.description("test query")).toBe('"test query"');
      expect(result.current.description("another")).toBe('"another"');
    });
  });

  describe("isVisible", () => {
    it("should return true when query is non-empty", () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      expect(result.current.isVisible("hello")).toBe(true);
      expect(result.current.isVisible("a")).toBe(true);
      expect(result.current.isVisible("  text  ")).toBe(true);
    });

    it("should return false when query is empty or whitespace", () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      expect(result.current.isVisible("")).toBe(false);
      expect(result.current.isVisible("   ")).toBe(false);
      expect(result.current.isVisible("\t\n")).toBe(false);
    });
  });

  describe("execute", () => {
    it("should create session via mutation", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      await result.current.execute("my ideation query");

      expect(mockMutateAsync).toHaveBeenCalledWith({ projectId: mockProjectId });
    });

    it("should add session to store", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      await result.current.execute("my ideation query");

      await waitFor(() => {
        expect(mockAddSession).toHaveBeenCalledWith(mockSession);
      });
    });

    it("should set active session", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      await result.current.execute("my ideation query");

      await waitFor(() => {
        expect(mockSetActiveSession).toHaveBeenCalledWith(mockSessionId);
      });
    });

    it("should send agent message (fire-and-forget)", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      await result.current.execute("my ideation query");

      await waitFor(() => {
        expect(mockSendAgentMessage).toHaveBeenCalledWith(
          "ideation",
          mockSessionId,
          "my ideation query"
        );
      });
    });

    it("should spawn session namer (fire-and-forget)", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      await result.current.execute("my ideation query");

      await waitFor(() => {
        expect(mockSpawnSessionNamer).toHaveBeenCalledWith(mockSessionId, "my ideation query");
      });
    });

    it("should return session ID on success", async () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      const sessionId = await result.current.execute("my ideation query");

      expect(sessionId).toBe(mockSessionId);
    });

    it("should not throw if fire-and-forget operations fail", async () => {
      mockSendAgentMessage.mockRejectedValue(new Error("Network error"));
      mockSpawnSessionNamer.mockRejectedValue(new Error("Spawn error"));

      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      await expect(result.current.execute("query")).resolves.toBe(mockSessionId);
    });
  });

  describe("navigateTo", () => {
    it("should set active session", () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      result.current.navigateTo(mockSessionId);

      expect(mockSetActiveSession).toHaveBeenCalledWith(mockSessionId);
    });

    it("should switch view to ideation", () => {
      const { result } = renderHook(() => useIdeationQuickAction(mockProjectId));

      result.current.navigateTo(mockSessionId);

      expect(mockSetCurrentView).toHaveBeenCalledWith("ideation");
    });
  });

  describe("memoization", () => {
    it("should return stable reference when projectId unchanged", () => {
      const { result, rerender } = renderHook(
        ({ projectId }) => useIdeationQuickAction(projectId),
        { initialProps: { projectId: mockProjectId } }
      );

      const firstAction = result.current;

      rerender({ projectId: mockProjectId });

      expect(result.current).toBe(firstAction);
    });

    it("should return new reference when projectId changes", () => {
      const { result, rerender } = renderHook(
        ({ projectId }) => useIdeationQuickAction(projectId),
        { initialProps: { projectId: "project-1" } }
      );

      const firstAction = result.current;

      rerender({ projectId: "project-2" });

      expect(result.current).not.toBe(firstAction);
      expect(result.current.id).toBe("ideation"); // Still valid action
    });
  });
});
