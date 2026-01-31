import { describe, it, expect, beforeEach } from "vitest";
import {
  useChatStore,
  selectMessagesForContext,
  selectMessageCount,
  selectQueuedMessages,
  selectIsAgentRunning,
  selectActiveConversationId,
  getContextKey,
} from "./chatStore";
import type { ChatMessage } from "@/types/ideation";
import type { ChatContext } from "@/types/chat";

// Helper to create test messages
const createTestMessage = (overrides: Partial<ChatMessage> = {}): ChatMessage => ({
  id: `msg-${Math.random().toString(36).slice(2)}`,
  sessionId: null,
  projectId: null,
  taskId: null,
  role: "user",
  content: "Test message",
  metadata: null,
  parentMessageId: null,
  createdAt: "2026-01-24T12:00:00Z",
  ...overrides,
});

// Helper to create test context
const createTestContext = (overrides: Partial<ChatContext> = {}): ChatContext => ({
  view: "kanban",
  projectId: "project-1",
  ...overrides,
});

describe("chatStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useChatStore.setState({
      messages: {},
      context: null,
      width: 320,
      isLoading: false,
      activeConversationId: null,
      queuedMessages: {},
      isAgentRunning: {},
    });
  });

  describe("initial state", () => {
    it("has empty messages", () => {
      const state = useChatStore.getState();
      expect(Object.keys(state.messages)).toHaveLength(0);
    });

    it("has null context", () => {
      const state = useChatStore.getState();
      expect(state.context).toBeNull();
    });

    it("has default width 320", () => {
      const state = useChatStore.getState();
      expect(state.width).toBe(320);
    });

    it("has isLoading false", () => {
      const state = useChatStore.getState();
      expect(state.isLoading).toBe(false);
    });

    it("has null activeConversationId", () => {
      const state = useChatStore.getState();
      expect(state.activeConversationId).toBeNull();
    });

    it("has empty queuedMessages", () => {
      const state = useChatStore.getState();
      expect(Object.keys(state.queuedMessages)).toHaveLength(0);
    });

    it("has empty isAgentRunning", () => {
      const state = useChatStore.getState();
      expect(Object.keys(state.isAgentRunning)).toHaveLength(0);
    });
  });

  describe("setContext", () => {
    it("sets context", () => {
      const context = createTestContext({ view: "ideation", ideationSessionId: "session-1" });

      useChatStore.getState().setContext(context);

      const state = useChatStore.getState();
      expect(state.context).toEqual(context);
    });

    it("replaces previous context", () => {
      const context1 = createTestContext({ view: "kanban" });
      const context2 = createTestContext({ view: "ideation", ideationSessionId: "session-1" });

      useChatStore.getState().setContext(context1);
      useChatStore.getState().setContext(context2);

      const state = useChatStore.getState();
      expect(state.context?.view).toBe("ideation");
    });

    it("sets context to null", () => {
      useChatStore.setState({ context: createTestContext() });

      useChatStore.getState().setContext(null);

      const state = useChatStore.getState();
      expect(state.context).toBeNull();
    });
  });

  describe("setWidth", () => {
    it("sets width", () => {
      useChatStore.getState().setWidth(400);

      const state = useChatStore.getState();
      expect(state.width).toBe(400);
    });

    it("clamps to minimum width 280", () => {
      useChatStore.getState().setWidth(200);

      const state = useChatStore.getState();
      expect(state.width).toBe(280);
    });

    it("clamps to maximum width 800", () => {
      useChatStore.getState().setWidth(1000);

      const state = useChatStore.getState();
      expect(state.width).toBe(800);
    });

    it("accepts width at minimum boundary", () => {
      useChatStore.getState().setWidth(280);

      const state = useChatStore.getState();
      expect(state.width).toBe(280);
    });

    it("accepts width at maximum boundary", () => {
      useChatStore.getState().setWidth(800);

      const state = useChatStore.getState();
      expect(state.width).toBe(800);
    });
  });

  describe("addMessage", () => {
    it("adds message to context", () => {
      const message = createTestMessage({ id: "msg-1", content: "Hello" });

      useChatStore.getState().addMessage("session:session-1", message);

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toHaveLength(1);
      expect(state.messages["session:session-1"]?.[0].content).toBe("Hello");
    });

    it("appends to existing messages", () => {
      const msg1 = createTestMessage({ id: "msg-1", content: "First" });
      const msg2 = createTestMessage({ id: "msg-2", content: "Second" });

      useChatStore.getState().addMessage("session:session-1", msg1);
      useChatStore.getState().addMessage("session:session-1", msg2);

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toHaveLength(2);
      expect(state.messages["session:session-1"]?.[0].content).toBe("First");
      expect(state.messages["session:session-1"]?.[1].content).toBe("Second");
    });

    it("keeps messages separate by context key", () => {
      const msg1 = createTestMessage({ id: "msg-1" });
      const msg2 = createTestMessage({ id: "msg-2" });

      useChatStore.getState().addMessage("session:session-1", msg1);
      useChatStore.getState().addMessage("session:session-2", msg2);

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toHaveLength(1);
      expect(state.messages["session:session-2"]).toHaveLength(1);
    });
  });

  describe("setMessages", () => {
    it("sets messages for context", () => {
      const messages = [
        createTestMessage({ id: "msg-1", content: "First" }),
        createTestMessage({ id: "msg-2", content: "Second" }),
      ];

      useChatStore.getState().setMessages("session:session-1", messages);

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toHaveLength(2);
    });

    it("replaces existing messages", () => {
      useChatStore.setState({
        messages: {
          "session:session-1": [createTestMessage({ id: "old", content: "Old" })],
        },
      });

      const newMessages = [createTestMessage({ id: "new", content: "New" })];
      useChatStore.getState().setMessages("session:session-1", newMessages);

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toHaveLength(1);
      expect(state.messages["session:session-1"]?.[0].content).toBe("New");
    });

    it("preserves other context messages", () => {
      useChatStore.setState({
        messages: {
          "session:session-1": [createTestMessage({ id: "s1-msg" })],
          "session:session-2": [createTestMessage({ id: "s2-msg" })],
        },
      });

      const newMessages = [createTestMessage({ id: "new" })];
      useChatStore.getState().setMessages("session:session-1", newMessages);

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toHaveLength(1);
      expect(state.messages["session:session-2"]).toHaveLength(1);
    });

    it("handles empty array", () => {
      useChatStore.getState().setMessages("session:session-1", []);

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toHaveLength(0);
    });
  });

  describe("clearMessages", () => {
    it("clears messages for context", () => {
      useChatStore.setState({
        messages: {
          "session:session-1": [createTestMessage()],
        },
      });

      useChatStore.getState().clearMessages("session:session-1");

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toBeUndefined();
    });

    it("preserves other context messages", () => {
      useChatStore.setState({
        messages: {
          "session:session-1": [createTestMessage()],
          "session:session-2": [createTestMessage()],
        },
      });

      useChatStore.getState().clearMessages("session:session-1");

      const state = useChatStore.getState();
      expect(state.messages["session:session-1"]).toBeUndefined();
      expect(state.messages["session:session-2"]).toHaveLength(1);
    });

    it("does nothing if context not found", () => {
      useChatStore.getState().clearMessages("nonexistent");

      const state = useChatStore.getState();
      expect(Object.keys(state.messages)).toHaveLength(0);
    });
  });

  describe("setLoading", () => {
    it("sets isLoading to true", () => {
      useChatStore.getState().setLoading(true);

      const state = useChatStore.getState();
      expect(state.isLoading).toBe(true);
    });

    it("sets isLoading to false", () => {
      useChatStore.setState({ isLoading: true });

      useChatStore.getState().setLoading(false);

      const state = useChatStore.getState();
      expect(state.isLoading).toBe(false);
    });
  });

  describe("setActiveConversation", () => {
    it("sets activeConversationId", () => {
      useChatStore.getState().setActiveConversation("conv-123");

      const state = useChatStore.getState();
      expect(state.activeConversationId).toBe("conv-123");
    });

    it("replaces previous conversation ID", () => {
      useChatStore.setState({ activeConversationId: "conv-old" });

      useChatStore.getState().setActiveConversation("conv-new");

      const state = useChatStore.getState();
      expect(state.activeConversationId).toBe("conv-new");
    });

    it("sets to null", () => {
      useChatStore.setState({ activeConversationId: "conv-123" });

      useChatStore.getState().setActiveConversation(null);

      const state = useChatStore.getState();
      expect(state.activeConversationId).toBeNull();
    });
  });

  describe("setAgentRunning", () => {
    const contextKey = "task:test-task";

    it("sets isAgentRunning to true for context", () => {
      useChatStore.getState().setAgentRunning(contextKey, true);

      const state = useChatStore.getState();
      expect(state.isAgentRunning[contextKey]).toBe(true);
    });

    it("removes context when set to false", () => {
      useChatStore.getState().setAgentRunning(contextKey, true);
      useChatStore.getState().setAgentRunning(contextKey, false);

      const state = useChatStore.getState();
      expect(state.isAgentRunning[contextKey]).toBeUndefined();
    });

    it("keeps separate states by context key", () => {
      const taskKey = "task:task-123";
      const execKey = "task_execution:task-123";

      useChatStore.getState().setAgentRunning(taskKey, true);
      useChatStore.getState().setAgentRunning(execKey, true);
      useChatStore.getState().setAgentRunning(taskKey, false);

      const state = useChatStore.getState();
      expect(state.isAgentRunning[taskKey]).toBeUndefined();
      expect(state.isAgentRunning[execKey]).toBe(true);
    });
  });

  describe("queueMessage", () => {
    const contextKey = "task:test-task";

    it("adds message to queue for context", () => {
      useChatStore.getState().queueMessage(contextKey, "Hello");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]).toHaveLength(1);
      expect(state.queuedMessages[contextKey]?.[0].content).toBe("Hello");
    });

    it("generates unique ID for queued message", () => {
      useChatStore.getState().queueMessage(contextKey, "First");
      useChatStore.getState().queueMessage(contextKey, "Second");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].id).not.toBe(state.queuedMessages[contextKey]?.[1].id);
    });

    it("sets createdAt timestamp", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].createdAt).toBeDefined();
      expect(new Date(state.queuedMessages[contextKey]?.[0].createdAt || "").getTime()).toBeLessThanOrEqual(
        Date.now()
      );
    });

    it("sets isEditing to false by default", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].isEditing).toBe(false);
    });

    it("appends to existing queue", () => {
      useChatStore.getState().queueMessage(contextKey, "First");
      useChatStore.getState().queueMessage(contextKey, "Second");
      useChatStore.getState().queueMessage(contextKey, "Third");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]).toHaveLength(3);
      expect(state.queuedMessages[contextKey]?.[0].content).toBe("First");
      expect(state.queuedMessages[contextKey]?.[1].content).toBe("Second");
      expect(state.queuedMessages[contextKey]?.[2].content).toBe("Third");
    });

    it("keeps queues separate by context key", () => {
      const taskKey = "task:task-123";
      const execKey = "task_execution:task-123";

      useChatStore.getState().queueMessage(taskKey, "Task message");
      useChatStore.getState().queueMessage(execKey, "Execution message");

      const state = useChatStore.getState();
      expect(state.queuedMessages[taskKey]).toHaveLength(1);
      expect(state.queuedMessages[execKey]).toHaveLength(1);
      expect(state.queuedMessages[taskKey]?.[0].content).toBe("Task message");
      expect(state.queuedMessages[execKey]?.[0].content).toBe("Execution message");
    });
  });

  describe("editQueuedMessage", () => {
    const contextKey = "task:test-task";

    it("updates message content", () => {
      useChatStore.getState().queueMessage(contextKey, "Original");
      const messageId = useChatStore.getState().queuedMessages[contextKey]?.[0].id || "";

      useChatStore.getState().editQueuedMessage(contextKey, messageId, "Updated");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].content).toBe("Updated");
    });

    it("sets isEditing to false after edit", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");
      const messageId = useChatStore.getState().queuedMessages[contextKey]?.[0].id || "";
      useChatStore.setState({
        queuedMessages: { [contextKey]: [{ ...useChatStore.getState().queuedMessages[contextKey]![0], isEditing: true }] },
      });

      useChatStore.getState().editQueuedMessage(contextKey, messageId, "Updated");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].isEditing).toBe(false);
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");

      useChatStore.getState().editQueuedMessage(contextKey, "nonexistent-id", "Updated");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].content).toBe("Test");
    });

    it("only updates specified message", () => {
      useChatStore.getState().queueMessage(contextKey, "First");
      useChatStore.getState().queueMessage(contextKey, "Second");
      useChatStore.getState().queueMessage(contextKey, "Third");
      const secondId = useChatStore.getState().queuedMessages[contextKey]?.[1].id || "";

      useChatStore.getState().editQueuedMessage(contextKey, secondId, "Updated Second");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].content).toBe("First");
      expect(state.queuedMessages[contextKey]?.[1].content).toBe("Updated Second");
      expect(state.queuedMessages[contextKey]?.[2].content).toBe("Third");
    });
  });

  describe("deleteQueuedMessage", () => {
    const contextKey = "task:test-task";

    it("removes message from queue", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");
      const messageId = useChatStore.getState().queuedMessages[contextKey]?.[0].id || "";

      useChatStore.getState().deleteQueuedMessage(contextKey, messageId);

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]).toBeUndefined();
    });

    it("cleans up empty arrays", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");
      const messageId = useChatStore.getState().queuedMessages[contextKey]?.[0].id || "";

      useChatStore.getState().deleteQueuedMessage(contextKey, messageId);

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]).toBeUndefined();
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");

      useChatStore.getState().deleteQueuedMessage(contextKey, "nonexistent-id");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]).toHaveLength(1);
    });

    it("only removes specified message", () => {
      useChatStore.getState().queueMessage(contextKey, "First");
      useChatStore.getState().queueMessage(contextKey, "Second");
      useChatStore.getState().queueMessage(contextKey, "Third");
      const secondId = useChatStore.getState().queuedMessages[contextKey]?.[1].id || "";

      useChatStore.getState().deleteQueuedMessage(contextKey, secondId);

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]).toHaveLength(2);
      expect(state.queuedMessages[contextKey]?.[0].content).toBe("First");
      expect(state.queuedMessages[contextKey]?.[1].content).toBe("Third");
    });

    it("preserves other context queues", () => {
      const taskKey = "task:task-123";
      const execKey = "task_execution:task-123";
      useChatStore.getState().queueMessage(taskKey, "First");
      useChatStore.getState().queueMessage(execKey, "Second");
      const messageId = useChatStore.getState().queuedMessages[taskKey]?.[0].id || "";

      useChatStore.getState().deleteQueuedMessage(taskKey, messageId);

      const state = useChatStore.getState();
      expect(state.queuedMessages[execKey]).toHaveLength(1);
    });
  });

  describe("startEditingQueuedMessage", () => {
    const contextKey = "task:test-task";

    it("sets isEditing to true", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");
      const messageId = useChatStore.getState().queuedMessages[contextKey]?.[0].id || "";

      useChatStore.getState().startEditingQueuedMessage(contextKey, messageId);

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].isEditing).toBe(true);
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");

      useChatStore.getState().startEditingQueuedMessage(contextKey, "nonexistent-id");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].isEditing).toBe(false);
    });
  });

  describe("stopEditingQueuedMessage", () => {
    const contextKey = "task:test-task";

    it("sets isEditing to false", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");
      const messageId = useChatStore.getState().queuedMessages[contextKey]?.[0].id || "";
      useChatStore.setState({
        queuedMessages: { [contextKey]: [{ ...useChatStore.getState().queuedMessages[contextKey]![0], isEditing: true }] },
      });

      useChatStore.getState().stopEditingQueuedMessage(contextKey, messageId);

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].isEditing).toBe(false);
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueMessage(contextKey, "Test");
      useChatStore.setState({
        queuedMessages: { [contextKey]: [{ ...useChatStore.getState().queuedMessages[contextKey]![0], isEditing: true }] },
      });

      useChatStore.getState().stopEditingQueuedMessage(contextKey, "nonexistent-id");

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]?.[0].isEditing).toBe(true);
    });
  });

  describe("processQueue", () => {
    const contextKey = "task:test-task";

    it("removes first message from queue", async () => {
      useChatStore.getState().queueMessage(contextKey, "First");
      useChatStore.getState().queueMessage(contextKey, "Second");

      await useChatStore.getState().processQueue(contextKey);

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]).toHaveLength(1);
      expect(state.queuedMessages[contextKey]?.[0].content).toBe("Second");
    });

    it("does nothing if queue is empty", async () => {
      await useChatStore.getState().processQueue(contextKey);

      const state = useChatStore.getState();
      expect(state.queuedMessages[contextKey]).toBeUndefined();
    });
  });

});

describe("getContextKey", () => {
  it("returns session key for ideation context", () => {
    const context = createTestContext({
      view: "ideation",
      ideationSessionId: "session-123",
    });

    const key = getContextKey(context);

    expect(key).toBe("session:session-123");
  });

  it("returns task key for task_detail context", () => {
    const context = createTestContext({
      view: "task_detail",
      selectedTaskId: "task-456",
    });

    const key = getContextKey(context);

    expect(key).toBe("task:task-456");
  });

  it("returns project key for kanban context", () => {
    const context = createTestContext({
      view: "kanban",
      projectId: "project-789",
    });

    const key = getContextKey(context);

    expect(key).toBe("project:project-789");
  });

  it("returns project key for activity context", () => {
    const context = createTestContext({
      view: "activity",
      projectId: "project-abc",
    });

    const key = getContextKey(context);

    expect(key).toBe("project:project-abc");
  });

  it("returns project key for settings context", () => {
    const context = createTestContext({
      view: "settings",
      projectId: "project-def",
    });

    const key = getContextKey(context);

    expect(key).toBe("project:project-def");
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useChatStore.setState({
      messages: {},
      context: null,
      isOpen: false,
      width: 320,
      isLoading: false,
      activeConversationId: null,
      queuedMessages: {},
      isAgentRunning: {},
    });
  });

  describe("selectMessagesForContext", () => {
    it("returns messages for context", () => {
      const messages = [
        createTestMessage({ id: "msg-1", content: "Hello" }),
        createTestMessage({ id: "msg-2", content: "World" }),
      ];
      useChatStore.setState({
        messages: { "session:session-1": messages },
      });

      const result = selectMessagesForContext("session:session-1")(useChatStore.getState());

      expect(result).toHaveLength(2);
      expect(result[0].content).toBe("Hello");
      expect(result[1].content).toBe("World");
    });

    it("returns empty array for unknown context", () => {
      const result = selectMessagesForContext("session:unknown")(useChatStore.getState());

      expect(result).toHaveLength(0);
    });
  });

  describe("selectMessageCount", () => {
    it("returns message count for context", () => {
      useChatStore.setState({
        messages: {
          "session:session-1": [createTestMessage(), createTestMessage(), createTestMessage()],
        },
      });

      const result = selectMessageCount("session:session-1")(useChatStore.getState());

      expect(result).toBe(3);
    });

    it("returns 0 for unknown context", () => {
      const result = selectMessageCount("session:unknown")(useChatStore.getState());

      expect(result).toBe(0);
    });
  });

  describe("selectQueuedMessages", () => {
    const contextKey = "task:test-task";

    it("returns queued messages for context", () => {
      useChatStore.getState().queueMessage(contextKey, "First");
      useChatStore.getState().queueMessage(contextKey, "Second");

      const result = selectQueuedMessages(contextKey)(useChatStore.getState());

      expect(result).toHaveLength(2);
      expect(result[0].content).toBe("First");
      expect(result[1].content).toBe("Second");
    });

    it("returns empty array for unknown context", () => {
      const result = selectQueuedMessages("unknown-context")(useChatStore.getState());

      expect(result).toHaveLength(0);
    });

    it("returns empty array when no queued messages exist", () => {
      const result = selectQueuedMessages(contextKey)(useChatStore.getState());

      expect(result).toHaveLength(0);
    });
  });

  describe("selectIsAgentRunning", () => {
    const contextKey = "task:test-task";

    it("returns true when agent is running for context", () => {
      useChatStore.getState().setAgentRunning(contextKey, true);

      const result = selectIsAgentRunning(contextKey)(useChatStore.getState());

      expect(result).toBe(true);
    });

    it("returns false when agent is not running for context", () => {
      const result = selectIsAgentRunning(contextKey)(useChatStore.getState());

      expect(result).toBe(false);
    });

    it("returns false for unknown context", () => {
      useChatStore.getState().setAgentRunning("other-context", true);

      const result = selectIsAgentRunning(contextKey)(useChatStore.getState());

      expect(result).toBe(false);
    });
  });

  describe("selectActiveConversationId", () => {
    it("returns active conversation ID", () => {
      useChatStore.setState({ activeConversationId: "conv-123" });

      const result = selectActiveConversationId(useChatStore.getState());

      expect(result).toBe("conv-123");
    });

    it("returns null when no active conversation", () => {
      const result = selectActiveConversationId(useChatStore.getState());

      expect(result).toBeNull();
    });
  });

});
