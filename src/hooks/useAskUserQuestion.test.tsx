/**
 * Tests for useAskUserQuestion hook
 *
 * Tests event listening for agent:ask_user_question events,
 * storing per-session question payloads in uiStore, and submitting/dismissing answers.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useAskUserQuestion } from "./useAskUserQuestion";
import { useUiStore } from "@/stores/uiStore";
import type { AskUserQuestionPayload, AskUserQuestionResponse } from "@/types/ask-user-question";

// Mock EventBus
const mockSubscribers = new Map<string, (payload: unknown) => void>();

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: (event: string, handler: (payload: unknown) => void) => {
      mockSubscribers.set(event, handler);
      return () => {
        mockSubscribers.delete(event);
      };
    },
  }),
}));

// Mock Tauri invoke for answering questions
const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Mock the api module
vi.mock("@/lib/tauri", () => ({
  api: {
    askUserQuestion: {
      resolveQuestion: vi.fn(),
      answerQuestion: vi.fn(),
    },
  },
}));

import { api } from "@/lib/tauri";
const mockResolve = vi.mocked(api.askUserQuestion.resolveQuestion);
const mockAnswer = vi.mocked(api.askUserQuestion.answerQuestion);

// Helper to emit events
function emitEvent(eventName: string, payload: unknown) {
  const handler = mockSubscribers.get(eventName);
  if (handler) {
    handler(payload);
  }
}

const TEST_SESSION = "session-abc";

// Valid test payload
const validPayload: AskUserQuestionPayload = {
  requestId: "req-test-123",
  taskId: "task-123",
  sessionId: TEST_SESSION,
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
    mockSubscribers.clear();
    mockResolve.mockResolvedValue(undefined);
    mockAnswer.mockResolvedValue(undefined);
    // Reset store state
    useUiStore.setState({
      activeQuestions: {},
      answeredQuestions: {},
    });
  });

  afterEach(() => {
    mockSubscribers.clear();
  });

  describe("listener registration", () => {
    it("should register agent:ask_user_question listener on mount", () => {
      renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(mockSubscribers.has("agent:ask_user_question")).toBe(true);
    });

    it("should unregister listener on unmount", () => {
      const { unmount } = renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(mockSubscribers.has("agent:ask_user_question")).toBe(true);

      unmount();
      expect(mockSubscribers.has("agent:ask_user_question")).toBe(false);
    });
  });

  describe("session scoping", () => {
    it("should store question payload keyed by sessionId", () => {
      renderHook(() => useAskUserQuestion(TEST_SESSION));

      act(() => {
        emitEvent("agent:ask_user_question", validPayload);
      });

      const state = useUiStore.getState();
      expect(state.activeQuestions[TEST_SESSION]).toEqual(validPayload);
    });

    it("should only return question for current session", () => {
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      // Store question for a DIFFERENT session
      act(() => {
        emitEvent("agent:ask_user_question", {
          ...validPayload,
          sessionId: "other-session",
        });
      });

      expect(result.current.activeQuestion).toBeNull();
    });

    it("should return question when sessionId matches", () => {
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      act(() => {
        emitEvent("agent:ask_user_question", validPayload);
      });

      expect(result.current.activeQuestion).toEqual(validPayload);
    });

    it("should return null when currentSessionId is undefined", () => {
      const { result } = renderHook(() => useAskUserQuestion(undefined));

      // Store a question
      act(() => {
        emitEvent("agent:ask_user_question", validPayload);
      });

      expect(result.current.activeQuestion).toBeNull();
    });

    it("should ignore events without sessionId", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      renderHook(() => useAskUserQuestion(TEST_SESSION));

      act(() => {
        emitEvent("agent:ask_user_question", {
          ...validPayload,
          sessionId: undefined,
        });
      });

      const state = useUiStore.getState();
      expect(Object.keys(state.activeQuestions)).toHaveLength(0);
      consoleSpy.mockRestore();
    });
  });

  describe("event handling", () => {
    it("should ignore invalid events with missing fields", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      renderHook(() => useAskUserQuestion(TEST_SESSION));

      const invalidPayload = { taskId: "task-123" }; // Missing required fields

      act(() => {
        emitEvent("agent:ask_user_question", invalidPayload);
      });

      const state = useUiStore.getState();
      expect(Object.keys(state.activeQuestions)).toHaveLength(0);
      consoleSpy.mockRestore();
    });
  });

  describe("return values", () => {
    it("should return activeQuestion from store for current session", () => {
      useUiStore.getState().setActiveQuestion(TEST_SESSION, validPayload);
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(result.current.activeQuestion).toEqual(validPayload);
    });

    it("should return null activeQuestion when no question is active", () => {
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(result.current.activeQuestion).toBeNull();
    });

    it("should return submitAnswer function", () => {
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(typeof result.current.submitAnswer).toBe("function");
    });

    it("should return dismissQuestion function", () => {
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(typeof result.current.dismissQuestion).toBe("function");
    });

    it("should return isLoading state", () => {
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(result.current.isLoading).toBe(false);
    });
  });

  describe("submitAnswer", () => {
    it("should call resolveQuestion when requestId is present (MCP flow)", async () => {
      useUiStore.getState().setActiveQuestion(TEST_SESSION, validPayload);
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      const response: AskUserQuestionResponse = {
        requestId: "req-test-123",
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      expect(mockResolve).toHaveBeenCalledWith({
        requestId: "req-test-123",
        selectedOptions: ["JWT tokens"],
      });
    });

    it("should call answerQuestion when no requestId (legacy flow)", async () => {
      useUiStore.getState().setActiveQuestion(TEST_SESSION, validPayload);
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      const response: AskUserQuestionResponse = {
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      expect(mockAnswer).toHaveBeenCalledWith(response);
    });

    it("should clear active question and set answered after successful submission", async () => {
      useUiStore.getState().setActiveQuestion(TEST_SESSION, validPayload);
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      const response: AskUserQuestionResponse = {
        requestId: "req-test-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      const state = useUiStore.getState();
      expect(state.activeQuestions[TEST_SESSION]).toBeUndefined();
      expect(state.answeredQuestions[TEST_SESSION]).toBe("JWT tokens");
    });

    it("should not call api if no active question", async () => {
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      const response: AskUserQuestionResponse = {
        taskId: "task-123",
        selectedOptions: ["JWT tokens"],
      };

      await act(async () => {
        await result.current.submitAnswer(response);
      });

      expect(mockResolve).not.toHaveBeenCalled();
      expect(mockAnswer).not.toHaveBeenCalled();
    });
  });

  describe("dismissQuestion", () => {
    it("should clear both question and answered state for session", async () => {
      useUiStore.getState().setActiveQuestion(TEST_SESSION, validPayload);
      useUiStore.getState().setAnsweredQuestion(TEST_SESSION, "prev answer");

      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      await act(async () => {
        await result.current.dismissQuestion();
      });

      const state = useUiStore.getState();
      expect(state.activeQuestions[TEST_SESSION]).toBeUndefined();
      expect(state.answeredQuestions[TEST_SESSION]).toBeUndefined();
    });

    it("should send dismiss to backend when question has requestId", async () => {
      useUiStore.getState().setActiveQuestion(TEST_SESSION, validPayload);
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      await act(async () => {
        await result.current.dismissQuestion();
      });

      expect(mockResolve).toHaveBeenCalledWith({
        requestId: "req-test-123",
        selectedOptions: [],
        customResponse: "[dismissed]",
      });
    });

    it("should not send to backend when no active question", async () => {
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      await act(async () => {
        await result.current.dismissQuestion();
      });

      expect(mockResolve).not.toHaveBeenCalled();
    });
  });

  describe("clearAnswered", () => {
    it("should clear answered summary for session", () => {
      useUiStore.getState().setAnsweredQuestion(TEST_SESSION, "some answer");
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));

      expect(result.current.answeredQuestion).toBe("some answer");

      act(() => {
        result.current.clearAnswered();
      });

      expect(result.current.answeredQuestion).toBeUndefined();
    });
  });

  describe("answeredQuestion", () => {
    it("should return answered summary for current session", () => {
      useUiStore.getState().setAnsweredQuestion(TEST_SESSION, "JWT tokens");
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(result.current.answeredQuestion).toBe("JWT tokens");
    });

    it("should not return answered summary from other sessions", () => {
      useUiStore.getState().setAnsweredQuestion("other-session", "other answer");
      const { result } = renderHook(() => useAskUserQuestion(TEST_SESSION));
      expect(result.current.answeredQuestion).toBeUndefined();
    });
  });

  describe("multiple sessions", () => {
    it("should store questions for different sessions independently", () => {
      renderHook(() => useAskUserQuestion(TEST_SESSION));

      const payload1 = { ...validPayload, sessionId: "session-1", requestId: "req-1" };
      const payload2 = { ...validPayload, sessionId: "session-2", requestId: "req-2" };

      act(() => {
        emitEvent("agent:ask_user_question", payload1);
        emitEvent("agent:ask_user_question", payload2);
      });

      const state = useUiStore.getState();
      expect(state.activeQuestions["session-1"]?.requestId).toBe("req-1");
      expect(state.activeQuestions["session-2"]?.requestId).toBe("req-2");
    });
  });
});
