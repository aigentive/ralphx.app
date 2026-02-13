/**
 * useChatEvents hook tests
 *
 * Tests event subscription behavior: tool call accumulation, subagent routing,
 * streaming text, lifecycle clearing, error handling, and context filtering.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import type { ContextType } from "@/types/chat-conversation";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";
import type { StreamingTask } from "@/types/streaming-task";

// ============================================================================
// Mock infrastructure
// ============================================================================

// Capture subscriptions so tests can fire events manually
const subscriptions = new Map<string, ((...args: unknown[]) => void)[]>();

function fireEvent<T>(event: string, payload: T) {
  const handlers = subscriptions.get(event);
  if (handlers) {
    for (const handler of handlers) {
      handler(payload);
    }
  }
}

const mockInvalidateQueries = vi.fn();

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: (event: string, handler: (...args: unknown[]) => void) => {
      if (!subscriptions.has(event)) subscriptions.set(event, []);
      subscriptions.get(event)!.push(handler);
      return () => {
        const handlers = subscriptions.get(event);
        if (handlers) {
          const idx = handlers.indexOf(handler);
          if (idx >= 0) handlers.splice(idx, 1);
        }
      };
    },
  }),
}));

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({
    invalidateQueries: mockInvalidateQueries,
  }),
}));

vi.mock("@/hooks/useChat", () => ({
  chatKeys: {
    conversation: (id: string) => ["chat", "conversations", id],
  },
}));

// Dynamic mock for chat-context-registry — tests override via mockContextConfig
let mockContextConfig: {
  supportsStreamingText: boolean;
  supportsSubagentTasks: boolean;
  supportsDiffViews: boolean;
} | null = null;

vi.mock("@/lib/chat-context-registry", () => ({
  getContextConfig: () => mockContextConfig,
}));

// ============================================================================
// Import hook under test (after mocks)
// ============================================================================

import { useChatEvents } from "./useChatEvents";

// ============================================================================
// Helpers
// ============================================================================

const CONV_ID = "conv-abc";
const CTX_ID = "ctx-123";

interface DefaultProps {
  activeConversationId: string | null;
  contextId: string | null;
  contextType: ContextType | null;
  setStreamingToolCalls: ReturnType<typeof vi.fn>;
  setStreamingText: ReturnType<typeof vi.fn>;
  setStreamingTasks: ReturnType<typeof vi.fn>;
}

function makeProps(overrides?: Partial<DefaultProps>): DefaultProps {
  return {
    activeConversationId: CONV_ID,
    contextId: CTX_ID,
    contextType: "task_execution" as ContextType,
    setStreamingToolCalls: vi.fn(),
    setStreamingText: vi.fn(),
    setStreamingTasks: vi.fn(),
    ...overrides,
  };
}

/**
 * Helper to execute a state updater function captured by vi.fn().
 * setStreamingX mocks are called with updater functions (prev => next).
 * This executes the updater with a given prev value and returns the result.
 */
function executeUpdater<T>(mockFn: ReturnType<typeof vi.fn>, prev: T, callIndex = 0): T {
  const call = mockFn.mock.calls[callIndex];
  if (!call) throw new Error(`No call at index ${callIndex}`);
  const updater = call[0];
  if (typeof updater === "function") {
    return updater(prev) as T;
  }
  return updater as T;
}

// ============================================================================
// Tests
// ============================================================================

describe("useChatEvents", () => {
  beforeEach(() => {
    subscriptions.clear();
    mockInvalidateQueries.mockClear();
    // Default: full feature flags (task_execution context)
    mockContextConfig = {
      supportsStreamingText: true,
      supportsSubagentTasks: true,
      supportsDiffViews: true,
    };
  });

  // --------------------------------------------------------------------------
  // 1. Tool call accumulation
  // --------------------------------------------------------------------------
  describe("tool call accumulation", () => {
    it("should accumulate tool calls via setStreamingToolCalls on agent:tool_call", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:tool_call", {
          tool_name: "Read",
          tool_id: "toolu_001",
          arguments: { file_path: "/src/main.ts" },
          conversation_id: CONV_ID,
          context_id: CTX_ID,
        });
      });

      expect(props.setStreamingToolCalls).toHaveBeenCalledTimes(1);

      // Execute the updater to verify the appended tool call
      const result = executeUpdater<ToolCall[]>(props.setStreamingToolCalls, []);
      expect(result).toHaveLength(1);
      expect(result[0]).toMatchObject({
        id: "toolu_001",
        name: "Read",
        arguments: { file_path: "/src/main.ts" },
      });
    });

    it("should update existing tool call when same tool_id arrives with result", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      // First event: tool call started
      act(() => {
        fireEvent("agent:tool_call", {
          tool_name: "Read",
          tool_id: "toolu_002",
          arguments: { file_path: "/src/main.ts" },
          conversation_id: CONV_ID,
          context_id: CTX_ID,
        });
      });

      // Second event: same tool_id with result
      act(() => {
        fireEvent("agent:tool_call", {
          tool_name: "Read",
          tool_id: "toolu_002",
          arguments: { file_path: "/src/main.ts" },
          result: "file content here",
          conversation_id: CONV_ID,
          context_id: CTX_ID,
        });
      });

      expect(props.setStreamingToolCalls).toHaveBeenCalledTimes(2);

      // The second call should update the existing entry, not append
      const existing: ToolCall[] = [
        { id: "toolu_002", name: "Read", arguments: { file_path: "/src/main.ts" } },
      ];
      const result = executeUpdater<ToolCall[]>(props.setStreamingToolCalls, existing, 1);
      expect(result).toHaveLength(1);
      expect(result[0]!.result).toBe("file content here");
    });

    it("should skip result: prefixed tool names", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:tool_call", {
          tool_name: "result:toolu_001",
          tool_id: "toolu_001",
          arguments: {},
          conversation_id: CONV_ID,
          context_id: CTX_ID,
        });
      });

      expect(props.setStreamingToolCalls).not.toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // 2. Child tool call routing to subagent tasks
  // --------------------------------------------------------------------------
  describe("child tool call routing", () => {
    it("should route tool calls with parent_tool_use_id to setStreamingTasks", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      const parentId = "toolu_parent";

      act(() => {
        fireEvent("agent:tool_call", {
          tool_name: "Bash",
          tool_id: "toolu_child_001",
          arguments: { command: "ls" },
          conversation_id: CONV_ID,
          context_id: CTX_ID,
          parent_tool_use_id: parentId,
        });
      });

      // Should route to setStreamingTasks, not setStreamingToolCalls
      expect(props.setStreamingTasks).toHaveBeenCalledTimes(1);
      expect(props.setStreamingToolCalls).not.toHaveBeenCalled();

      // Execute the updater with a parent task already in the map
      const parentTask: StreamingTask = {
        toolUseId: parentId,
        description: "Test task",
        subagentType: "Bash",
        model: "sonnet",
        status: "running",
        startedAt: Date.now(),
        childToolCalls: [],
      };
      const prevMap = new Map<string, StreamingTask>([[parentId, parentTask]]);
      const nextMap = executeUpdater<Map<string, StreamingTask>>(props.setStreamingTasks, prevMap);

      const updated = nextMap.get(parentId)!;
      expect(updated.childToolCalls).toHaveLength(1);
      expect(updated.childToolCalls[0]).toMatchObject({
        id: "toolu_child_001",
        name: "Bash",
      });
    });

    it("should NOT route to tasks when supportsSubagentTasks is false", () => {
      mockContextConfig = {
        supportsStreamingText: false,
        supportsSubagentTasks: false,
        supportsDiffViews: false,
      };

      const props = makeProps({ contextType: "task" as ContextType });
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:tool_call", {
          tool_name: "Read",
          tool_id: "toolu_010",
          arguments: {},
          conversation_id: CONV_ID,
          context_id: CTX_ID,
          parent_tool_use_id: "toolu_parent",
        });
      });

      // Should fall through to setStreamingToolCalls since subagent routing is off
      expect(props.setStreamingToolCalls).toHaveBeenCalledTimes(1);
      expect(props.setStreamingTasks).not.toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // 3. Streaming text
  // --------------------------------------------------------------------------
  describe("streaming text", () => {
    it("should append text chunks via setStreamingText when supportsStreamingText", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:chunk", {
          text: "Hello ",
          conversation_id: CONV_ID,
          context_id: CTX_ID,
        });
      });

      expect(props.setStreamingText).toHaveBeenCalledTimes(1);

      // Execute the updater — appends to previous text
      const result = executeUpdater<string>(props.setStreamingText, "Previous: ");
      expect(result).toBe("Previous: Hello ");
    });
  });

  // --------------------------------------------------------------------------
  // 4. Message created clears streaming state
  // --------------------------------------------------------------------------
  describe("agent:message_created", () => {
    it("should clear streaming text, tool calls, and tasks on assistant message", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:message_created", {
          conversation_id: CONV_ID,
          context_id: CTX_ID,
          role: "assistant",
        });
      });

      expect(props.setStreamingText).toHaveBeenCalledWith("");
      expect(props.setStreamingToolCalls).toHaveBeenCalledWith([]);
      // setStreamingTasks is called with a new Map
      expect(props.setStreamingTasks).toHaveBeenCalledTimes(1);
      const taskArg = props.setStreamingTasks.mock.calls[0][0];
      expect(taskArg).toBeInstanceOf(Map);
      expect(taskArg.size).toBe(0);
    });

    it("should NOT clear streaming state on user message", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:message_created", {
          conversation_id: CONV_ID,
          context_id: CTX_ID,
          role: "user",
        });
      });

      // setStreamingText should not be called with "" for user messages
      // But invalidateQueries should still be called
      const textCalls = props.setStreamingText.mock.calls.filter(
        (call: [string | ((...args: unknown[]) => unknown)]) => call[0] === ""
      );
      expect(textCalls).toHaveLength(0);
      expect(mockInvalidateQueries).toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // 5. Run completed clears streaming state
  // --------------------------------------------------------------------------
  describe("agent:run_completed", () => {
    it("should clear all streaming state on run completion", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:run_completed", {
          conversation_id: CONV_ID,
          context_id: CTX_ID,
        });
      });

      expect(props.setStreamingToolCalls).toHaveBeenCalledWith([]);
      expect(props.setStreamingText).toHaveBeenCalledWith("");
      expect(props.setStreamingTasks).toHaveBeenCalledTimes(1);
      const taskArg = props.setStreamingTasks.mock.calls[0][0];
      expect(taskArg).toBeInstanceOf(Map);
      expect(taskArg.size).toBe(0);
      expect(mockInvalidateQueries).toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // 6. Error clears streaming tool calls
  // --------------------------------------------------------------------------
  describe("agent:error", () => {
    it("should clear streaming tool calls on error", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:error", {
          conversation_id: CONV_ID,
          context_id: CTX_ID,
          error: "Something went wrong",
        });
      });

      expect(props.setStreamingToolCalls).toHaveBeenCalledWith([]);
      expect(mockInvalidateQueries).toHaveBeenCalled();
      // Note: agent:error only clears tool calls, not text or tasks
      expect(props.setStreamingText).not.toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // 7. Context relevance filtering
  // --------------------------------------------------------------------------
  describe("context relevance filtering", () => {
    it("should ignore events with a different conversation_id", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:tool_call", {
          tool_name: "Read",
          tool_id: "toolu_wrong",
          arguments: {},
          conversation_id: "other-conv-id",
          context_id: CTX_ID,
        });
      });

      expect(props.setStreamingToolCalls).not.toHaveBeenCalled();
    });

    it("should ignore events with a different context_id", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:chunk", {
          text: "ignored",
          conversation_id: CONV_ID,
          context_id: "other-ctx-id",
        });
      });

      expect(props.setStreamingText).not.toHaveBeenCalled();
    });
  });

  // --------------------------------------------------------------------------
  // 8. Feature flags — agent:chunk only subscribed when supportsStreamingText
  // --------------------------------------------------------------------------
  describe("feature flags", () => {
    it("should NOT subscribe to agent:chunk when supportsStreamingText is false", () => {
      mockContextConfig = {
        supportsStreamingText: false,
        supportsSubagentTasks: false,
        supportsDiffViews: false,
      };

      const props = makeProps({ contextType: "task" as ContextType });
      renderHook(() => useChatEvents(props));

      // agent:chunk should have no handlers
      const chunkHandlers = subscriptions.get("agent:chunk") ?? [];
      expect(chunkHandlers).toHaveLength(0);
    });

    it("should subscribe to agent:chunk when supportsStreamingText is true", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      const chunkHandlers = subscriptions.get("agent:chunk") ?? [];
      expect(chunkHandlers.length).toBeGreaterThan(0);
    });
  });

  // --------------------------------------------------------------------------
  // 9. Task started/completed lifecycle for subagent tasks
  // --------------------------------------------------------------------------
  describe("subagent task lifecycle", () => {
    it("should create a streaming task on agent:task_started", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:task_started", {
          tool_use_id: "toolu_task_001",
          description: "Analyze file structure",
          subagent_type: "Explore",
          model: "sonnet",
          conversation_id: CONV_ID,
          context_id: CTX_ID,
        });
      });

      expect(props.setStreamingTasks).toHaveBeenCalledTimes(1);

      const nextMap = executeUpdater<Map<string, StreamingTask>>(
        props.setStreamingTasks,
        new Map(),
      );
      const task = nextMap.get("toolu_task_001");
      expect(task).toBeDefined();
      expect(task!.toolUseId).toBe("toolu_task_001");
      expect(task!.description).toBe("Analyze file structure");
      expect(task!.subagentType).toBe("Explore");
      expect(task!.model).toBe("sonnet");
      expect(task!.status).toBe("running");
      expect(task!.childToolCalls).toEqual([]);
    });

    it("should mark a streaming task as completed on agent:task_completed", () => {
      const props = makeProps();
      renderHook(() => useChatEvents(props));

      act(() => {
        fireEvent("agent:task_completed", {
          tool_use_id: "toolu_task_002",
          agent_id: "agent-xyz",
          total_duration_ms: 5000,
          total_tokens: 1200,
          total_tool_use_count: 3,
          conversation_id: CONV_ID,
          context_id: CTX_ID,
        });
      });

      expect(props.setStreamingTasks).toHaveBeenCalledTimes(1);

      // Run updater with existing running task
      const existingTask: StreamingTask = {
        toolUseId: "toolu_task_002",
        description: "Some task",
        subagentType: "Plan",
        model: "opus",
        status: "running",
        startedAt: Date.now() - 5000,
        childToolCalls: [{ id: "tc1", name: "Read", arguments: {} }],
      };
      const prevMap = new Map<string, StreamingTask>([
        ["toolu_task_002", existingTask],
      ]);
      const nextMap = executeUpdater<Map<string, StreamingTask>>(
        props.setStreamingTasks,
        prevMap,
      );

      const completed = nextMap.get("toolu_task_002")!;
      expect(completed.status).toBe("completed");
      expect(completed.completedAt).toBeDefined();
      expect(completed.agentId).toBe("agent-xyz");
      expect(completed.totalDurationMs).toBe(5000);
      expect(completed.totalTokens).toBe(1200);
      expect(completed.totalToolUseCount).toBe(3);
      // Child tool calls should be preserved
      expect(completed.childToolCalls).toHaveLength(1);
    });

    it("should NOT subscribe to task events when supportsSubagentTasks is false", () => {
      mockContextConfig = {
        supportsStreamingText: false,
        supportsSubagentTasks: false,
        supportsDiffViews: false,
      };

      const props = makeProps({ contextType: "task" as ContextType });
      renderHook(() => useChatEvents(props));

      const startedHandlers = subscriptions.get("agent:task_started") ?? [];
      const completedHandlers = subscriptions.get("agent:task_completed") ?? [];
      expect(startedHandlers).toHaveLength(0);
      expect(completedHandlers).toHaveLength(0);
    });
  });

  // --------------------------------------------------------------------------
  // 10. Cleanup on unmount
  // --------------------------------------------------------------------------
  describe("cleanup", () => {
    it("should clear streaming state and unsubscribe on unmount", () => {
      const props = makeProps();
      const { unmount } = renderHook(() => useChatEvents(props));

      // Verify subscriptions exist
      expect(subscriptions.get("agent:tool_call")?.length).toBeGreaterThan(0);

      unmount();

      // Cleanup sets values directly (not updater functions)
      expect(props.setStreamingToolCalls).toHaveBeenCalledWith([]);
      expect(props.setStreamingText).toHaveBeenCalledWith("");
      expect(props.setStreamingTasks).toHaveBeenCalledTimes(1);

      // All subscriptions should be removed
      for (const [, handlers] of subscriptions) {
        expect(handlers).toHaveLength(0);
      }
    });
  });
});
