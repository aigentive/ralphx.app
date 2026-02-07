/**
 * Tests for useAskUserQuestion hook
 *
 * Tests event listening for agent:ask_user_question events,
 * storing question payloads in uiStore, and submitting answers.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useAskUserQuestion } from "./useAskUserQuestion";
import { useUiStore } from "@/stores/uiStore";
import type { AskUserQuestionPayload, AskUserQuestionResponse } from "@/types/ask-user-question";

// Mock Tauri event listener
const mockListeners = new Map<string, (event: { payload: unknown }) => void>();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, callback: (event: { payload: unknown }) => void) => {
    mockListeners.set(eventName, callback);
    return Promise.resolve(() => {
      mockListeners.delete(eventName);
    });
  }),
}));

// Mock Tauri invoke for answering questions
const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Helper to emit events
function emitEvent(eventName: string, payload: unknown) {
  const listener = mockListeners.get(eventName);
  if (listener) {
    listener({ payload });
  }
}

// Valid test payload
const validPayload: AskUserQuestionPayload = {
  requestId: "req-test-123",
  taskId: "task-123",
  question: "Which authentication method should we use?",
  header: "Auth method",
  options: [
    { label: "JWT tokens", description: "Recommended for stateless APIs" },
    { label: "Session cookies", description: "Traditional server-side sessions" },
  ],
  multiSelect: false,
};

describe("useAskUserQuestion", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListeners.clear();
    mockInvoke.mockResolvedValue(undefined);
    // Reset store state
    useUiStore.setState({
      sidebarOpen: true,
      activeModal: null,
      modalContext: undefined,
      notifications: [],
      loading: {},
      confirmation: null,
      activeQuestion: null,
    });
  });

  afterEach(() => {
    mockListeners.clear();
  });

  describe("listener registration", () => {
    it("should register agent:ask_user_question listener on mount", async () => {
      renderHook(() => useAskUserQuestion());

      await waitFor(() => {
        expect(mockListeners.has("agent:ask_user_question")).toBe(true);
      });
    });

    it("should unregister listener on unmount", async () => {
      const { unmount } = renderHook(() => useAskUserQuestion());

      await waitFor(() => {
        expect(mockListeners.has("agent:ask_user_question")).toBe(true);
      });

      unmount();

      await waitFor(() => {
        expect(mockListeners.has("agent:ask_user_question")).toBe(false);
      });
    });
  });

  describe("event handling", () => {
    it("should store question payload in uiStore on valid event", async () => {
      renderHook(() => useAskUserQuestion());

      await waitFor(() => {
        expect(mockListeners.has("agent:ask_user_question")).toBe(true);
      });

      act(() => {
        emitEvent("agent:ask_user_question", validPayload);
      });

      const state = useUiStore.getState();
      expect(state.activeQuestion).toEqual(validPayload);
    });

    it("should ignore invalid events with missing fields", async () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

      renderHook(() => useAskUserQuestion());

      await waitFor(() => {
        expect(mockListeners.has("agent:ask_user_question")).toBe(true);
      });

      const invalidPayload = { taskId: "task-123" }; // Missing required fields

      act(() => {
        emitEvent("agent:ask_user_question", invalidPayload);
      });

      const state = useUiStore.getState();
      expect(state.activeQuestion).toBeNull();
      expect(consoleSpy).toHaveBeenCalledWith(
        "Invalid ask_user_question event:",
        expect.any(String)
      );

      consoleSpy.mockRestore();
    });

    it("should ignore events with less than 2 options", async () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

      renderHook(() => useAskUserQuestion());

      await waitFor(() => {
        expect(mockListeners.has("agent:ask_user_question")).toBe(true);
      });

      const invalidPayload = {
        ...validPayload,
        options: [{ label: "Only one", description: "Not enough" }],
      };

      act(() => {
        emitEvent("agent:ask_user_question", invalidPayload);
      });

      const state = useUiStore.getState();
      expect(state.activeQuestion).toBeNull();

      consoleSpy.mockRestore();
    });
  });

  describe("return values", () => {
    it("should return activeQuestion from store", async () => {
      // Pre-populate store with question
      useUiStore.getState().setActiveQuestion(validPayload);

      const { result } = renderHook(() => useAskUserQuestion());

      expect(result.current.activeQuestion).toEqual(validPayload);
    });

    it("should return null activeQuestion when no question is active", async () => {
      const { result } = renderHook(() => useAskUserQuestion());

      expect(result.current.activeQuestion).toBeNull();
    });

    it("should return submitAnswer function", async () => {
      const { result } = renderHook(() => useAskUserQuestion());

      expect(typeof result.current.submitAnswer).toBe("function");
    });

    it("should return clearQuestion function", async () => {
      const { result } = renderHook(() => useAskUserQuestion());

      expect(typeof result.current.clearQuestion).toBe("function");
    });

    it("should return isLoading state", async () => {
      const { result } = renderHook(() => useAskUserQuestion());

      expect(result.current.isLoading).toBe(false);
    });
  });

  describe("submitAnswer", () => {
    it("should call resolve_user_question when requestId is present (MCP flow)", async () => {
      useUiStore.getState().setActiveQuestion(validPayload);

      const { result } = renderHook(() => useAskUserQuestion());

      const response: AskUserQuestionResponse = {
        requestId: "req-test-123",
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      expect(mockInvoke).toHaveBeenCalledWith("resolve_user_question", {
        requestId: "req-test-123",
        selectedOptions: ["JWT tokens"],
        customResponse: undefined,
      });
    });

    it("should call answer_user_question when no requestId (legacy flow)", async () => {
      useUiStore.getState().setActiveQuestion(validPayload);

      const { result } = renderHook(() => useAskUserQuestion());

      const response: AskUserQuestionResponse = {
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      expect(mockInvoke).toHaveBeenCalledWith("answer_user_question", {
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
        customResponse: undefined,
      });
    });

    it("should include customResponse when provided", async () => {
      useUiStore.getState().setActiveQuestion(validPayload);

      const { result } = renderHook(() => useAskUserQuestion());

      const response: AskUserQuestionResponse = {
        requestId: "req-test-123",
        selectedOptions: [],
        customResponse: "Use OAuth2 instead",
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      expect(mockInvoke).toHaveBeenCalledWith("resolve_user_question", {
        requestId: "req-test-123",
        selectedOptions: [],
        customResponse: "Use OAuth2 instead",
      });
    });

    it("should clear active question after successful submission", async () => {
      useUiStore.getState().setActiveQuestion(validPayload);

      const { result } = renderHook(() => useAskUserQuestion());

      const response: AskUserQuestionResponse = {
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      const state = useUiStore.getState();
      expect(state.activeQuestion).toBeNull();
    });

    it("should set isLoading true during submission", async () => {
      useUiStore.getState().setActiveQuestion(validPayload);

      // Make invoke take some time
      let resolveInvoke: () => void;
      mockInvoke.mockImplementation(
        () =>
          new Promise((resolve) => {
            resolveInvoke = () => resolve(undefined);
          })
      );

      const { result } = renderHook(() => useAskUserQuestion());

      const response: AskUserQuestionResponse = {
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      // Start submission
      act(() => {
        result.current.submitAnswer(response);
      });

      // Check loading state
      expect(result.current.isLoading).toBe(true);

      // Complete submission
      await act(async () => {
        resolveInvoke!();
      });

      expect(result.current.isLoading).toBe(false);
    });

    it("should handle submission errors gracefully", async () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
      useUiStore.getState().setActiveQuestion(validPayload);

      mockInvoke.mockRejectedValue(new Error("Network error"));

      const { result } = renderHook(() => useAskUserQuestion());

      const response: AskUserQuestionResponse = {
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      // Should not clear question on error
      const state = useUiStore.getState();
      expect(state.activeQuestion).toEqual(validPayload);
      expect(consoleSpy).toHaveBeenCalledWith(
        "Failed to submit answer:",
        expect.any(Error)
      );

      consoleSpy.mockRestore();
    });

    it("should not call invoke if no active question", async () => {
      const { result } = renderHook(() => useAskUserQuestion());

      const response: AskUserQuestionResponse = {
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });

  describe("clearQuestion", () => {
    it("should clear active question from store", async () => {
      useUiStore.getState().setActiveQuestion(validPayload);

      const { result } = renderHook(() => useAskUserQuestion());

      act(() => {
        result.current.clearQuestion();
      });

      const state = useUiStore.getState();
      expect(state.activeQuestion).toBeNull();
    });
  });

  describe("multiple questions", () => {
    it("should replace existing question with new one", async () => {
      renderHook(() => useAskUserQuestion());

      await waitFor(() => {
        expect(mockListeners.has("agent:ask_user_question")).toBe(true);
      });

      const firstPayload = { ...validPayload, requestId: "req-1", taskId: "task-1" };
      const secondPayload = { ...validPayload, requestId: "req-2", taskId: "task-2" };

      act(() => {
        emitEvent("agent:ask_user_question", firstPayload);
      });

      expect(useUiStore.getState().activeQuestion?.taskId).toBe("task-1");

      act(() => {
        emitEvent("agent:ask_user_question", secondPayload);
      });

      expect(useUiStore.getState().activeQuestion?.taskId).toBe("task-2");
    });
  });

  describe("multi-select questions", () => {
    it("should handle multi-select question payloads", async () => {
      renderHook(() => useAskUserQuestion());

      await waitFor(() => {
        expect(mockListeners.has("agent:ask_user_question")).toBe(true);
      });

      const multiSelectPayload: AskUserQuestionPayload = {
        ...validPayload,
        multiSelect: true,
        question: "Which features do you want to enable?",
      };

      act(() => {
        emitEvent("agent:ask_user_question", multiSelectPayload);
      });

      const state = useUiStore.getState();
      expect(state.activeQuestion).toEqual(multiSelectPayload);
      expect(state.activeQuestion?.multiSelect).toBe(true);
    });

    it("should submit multiple selected options for multi-select (MCP flow)", async () => {
      const multiSelectPayload: AskUserQuestionPayload = {
        ...validPayload,
        multiSelect: true,
      };
      useUiStore.getState().setActiveQuestion(multiSelectPayload);

      const { result } = renderHook(() => useAskUserQuestion());

      const response: AskUserQuestionResponse = {
        requestId: "req-test-123",
        taskId: "task-123",
        selectedOptions: ["JWT tokens", "Session cookies"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      expect(mockInvoke).toHaveBeenCalledWith("resolve_user_question", {
        requestId: "req-test-123",
        selectedOptions: ["JWT tokens", "Session cookies"],
        customResponse: undefined,
      });
    });
  });
});
