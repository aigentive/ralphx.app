import { describe, it, expect, beforeEach } from "vitest";
import {
  useChatStore,
  selectMessagesForContext,
  selectMessageCount,
  selectQueuedMessages,
  selectIsAgentRunning,
  selectActiveConversationId,
  selectExecutionQueuedMessages,
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
      isOpen: false,
      width: 320,
      isLoading: false,
      activeConversationId: null,
      queuedMessages: [],
      executionQueuedMessages: {},
      isAgentRunning: false,
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

    it("has isOpen false", () => {
      const state = useChatStore.getState();
      expect(state.isOpen).toBe(false);
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
      expect(state.queuedMessages).toHaveLength(0);
    });

    it("has empty executionQueuedMessages", () => {
      const state = useChatStore.getState();
      expect(Object.keys(state.executionQueuedMessages)).toHaveLength(0);
    });

    it("has isAgentRunning false", () => {
      const state = useChatStore.getState();
      expect(state.isAgentRunning).toBe(false);
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

  describe("togglePanel", () => {
    it("opens closed panel", () => {
      useChatStore.getState().togglePanel();

      const state = useChatStore.getState();
      expect(state.isOpen).toBe(true);
    });

    it("closes open panel", () => {
      useChatStore.setState({ isOpen: true });

      useChatStore.getState().togglePanel();

      const state = useChatStore.getState();
      expect(state.isOpen).toBe(false);
    });
  });

  describe("setOpen", () => {
    it("sets isOpen to true", () => {
      useChatStore.getState().setOpen(true);

      const state = useChatStore.getState();
      expect(state.isOpen).toBe(true);
    });

    it("sets isOpen to false", () => {
      useChatStore.setState({ isOpen: true });

      useChatStore.getState().setOpen(false);

      const state = useChatStore.getState();
      expect(state.isOpen).toBe(false);
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
    it("sets isAgentRunning to true", () => {
      useChatStore.getState().setAgentRunning(true);

      const state = useChatStore.getState();
      expect(state.isAgentRunning).toBe(true);
    });

    it("sets isAgentRunning to false", () => {
      useChatStore.setState({ isAgentRunning: true });

      useChatStore.getState().setAgentRunning(false);

      const state = useChatStore.getState();
      expect(state.isAgentRunning).toBe(false);
    });
  });

  describe("queueMessage", () => {
    it("adds message to queue", () => {
      useChatStore.getState().queueMessage("Hello");

      const state = useChatStore.getState();
      expect(state.queuedMessages).toHaveLength(1);
      expect(state.queuedMessages[0].content).toBe("Hello");
    });

    it("generates unique ID for queued message", () => {
      useChatStore.getState().queueMessage("First");
      useChatStore.getState().queueMessage("Second");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].id).not.toBe(state.queuedMessages[1].id);
    });

    it("sets createdAt timestamp", () => {
      useChatStore.getState().queueMessage("Test");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].createdAt).toBeDefined();
      expect(new Date(state.queuedMessages[0].createdAt).getTime()).toBeLessThanOrEqual(
        Date.now()
      );
    });

    it("sets isEditing to false by default", () => {
      useChatStore.getState().queueMessage("Test");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].isEditing).toBe(false);
    });

    it("appends to existing queue", () => {
      useChatStore.getState().queueMessage("First");
      useChatStore.getState().queueMessage("Second");
      useChatStore.getState().queueMessage("Third");

      const state = useChatStore.getState();
      expect(state.queuedMessages).toHaveLength(3);
      expect(state.queuedMessages[0].content).toBe("First");
      expect(state.queuedMessages[1].content).toBe("Second");
      expect(state.queuedMessages[2].content).toBe("Third");
    });
  });

  describe("editQueuedMessage", () => {
    it("updates message content", () => {
      useChatStore.getState().queueMessage("Original");
      const messageId = useChatStore.getState().queuedMessages[0].id;

      useChatStore.getState().editQueuedMessage(messageId, "Updated");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].content).toBe("Updated");
    });

    it("sets isEditing to false after edit", () => {
      useChatStore.getState().queueMessage("Test");
      const messageId = useChatStore.getState().queuedMessages[0].id;
      useChatStore.setState({
        queuedMessages: [{ ...useChatStore.getState().queuedMessages[0], isEditing: true }],
      });

      useChatStore.getState().editQueuedMessage(messageId, "Updated");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].isEditing).toBe(false);
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueMessage("Test");

      useChatStore.getState().editQueuedMessage("nonexistent-id", "Updated");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].content).toBe("Test");
    });

    it("only updates specified message", () => {
      useChatStore.getState().queueMessage("First");
      useChatStore.getState().queueMessage("Second");
      useChatStore.getState().queueMessage("Third");
      const secondId = useChatStore.getState().queuedMessages[1].id;

      useChatStore.getState().editQueuedMessage(secondId, "Updated Second");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].content).toBe("First");
      expect(state.queuedMessages[1].content).toBe("Updated Second");
      expect(state.queuedMessages[2].content).toBe("Third");
    });
  });

  describe("deleteQueuedMessage", () => {
    it("removes message from queue", () => {
      useChatStore.getState().queueMessage("Test");
      const messageId = useChatStore.getState().queuedMessages[0].id;

      useChatStore.getState().deleteQueuedMessage(messageId);

      const state = useChatStore.getState();
      expect(state.queuedMessages).toHaveLength(0);
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueMessage("Test");

      useChatStore.getState().deleteQueuedMessage("nonexistent-id");

      const state = useChatStore.getState();
      expect(state.queuedMessages).toHaveLength(1);
    });

    it("only removes specified message", () => {
      useChatStore.getState().queueMessage("First");
      useChatStore.getState().queueMessage("Second");
      useChatStore.getState().queueMessage("Third");
      const secondId = useChatStore.getState().queuedMessages[1].id;

      useChatStore.getState().deleteQueuedMessage(secondId);

      const state = useChatStore.getState();
      expect(state.queuedMessages).toHaveLength(2);
      expect(state.queuedMessages[0].content).toBe("First");
      expect(state.queuedMessages[1].content).toBe("Third");
    });
  });

  describe("startEditingQueuedMessage", () => {
    it("sets isEditing to true", () => {
      useChatStore.getState().queueMessage("Test");
      const messageId = useChatStore.getState().queuedMessages[0].id;

      useChatStore.getState().startEditingQueuedMessage(messageId);

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].isEditing).toBe(true);
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueMessage("Test");

      useChatStore.getState().startEditingQueuedMessage("nonexistent-id");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].isEditing).toBe(false);
    });
  });

  describe("stopEditingQueuedMessage", () => {
    it("sets isEditing to false", () => {
      useChatStore.getState().queueMessage("Test");
      const messageId = useChatStore.getState().queuedMessages[0].id;
      useChatStore.setState({
        queuedMessages: [{ ...useChatStore.getState().queuedMessages[0], isEditing: true }],
      });

      useChatStore.getState().stopEditingQueuedMessage(messageId);

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].isEditing).toBe(false);
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueMessage("Test");
      useChatStore.setState({
        queuedMessages: [{ ...useChatStore.getState().queuedMessages[0], isEditing: true }],
      });

      useChatStore.getState().stopEditingQueuedMessage("nonexistent-id");

      const state = useChatStore.getState();
      expect(state.queuedMessages[0].isEditing).toBe(true);
    });
  });

  describe("processQueue", () => {
    it("removes first message from queue", async () => {
      useChatStore.getState().queueMessage("First");
      useChatStore.getState().queueMessage("Second");

      await useChatStore.getState().processQueue();

      const state = useChatStore.getState();
      expect(state.queuedMessages).toHaveLength(1);
      expect(state.queuedMessages[0].content).toBe("Second");
    });

    it("does nothing if queue is empty", async () => {
      await useChatStore.getState().processQueue();

      const state = useChatStore.getState();
      expect(state.queuedMessages).toHaveLength(0);
    });
  });

  describe("queueExecutionMessage", () => {
    it("adds message to execution queue for task", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "Hello worker");

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]).toHaveLength(1);
      expect(state.executionQueuedMessages["task-123"]?.[0].content).toBe("Hello worker");
    });

    it("generates unique ID for queued execution message", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "First");
      useChatStore.getState().queueExecutionMessage("task-123", "Second");

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]?.[0].id).not.toBe(
        state.executionQueuedMessages["task-123"]?.[1].id
      );
    });

    it("sets createdAt timestamp", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "Test");

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]?.[0].createdAt).toBeDefined();
      expect(
        new Date(state.executionQueuedMessages["task-123"]?.[0].createdAt || "").getTime()
      ).toBeLessThanOrEqual(Date.now());
    });

    it("sets isEditing to false by default", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "Test");

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]?.[0].isEditing).toBe(false);
    });

    it("appends to existing queue for same task", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "First");
      useChatStore.getState().queueExecutionMessage("task-123", "Second");
      useChatStore.getState().queueExecutionMessage("task-123", "Third");

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]).toHaveLength(3);
      expect(state.executionQueuedMessages["task-123"]?.[0].content).toBe("First");
      expect(state.executionQueuedMessages["task-123"]?.[1].content).toBe("Second");
      expect(state.executionQueuedMessages["task-123"]?.[2].content).toBe("Third");
    });

    it("keeps queues separate by task ID", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "Message for task 123");
      useChatStore.getState().queueExecutionMessage("task-456", "Message for task 456");

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]).toHaveLength(1);
      expect(state.executionQueuedMessages["task-456"]).toHaveLength(1);
      expect(state.executionQueuedMessages["task-123"]?.[0].content).toBe(
        "Message for task 123"
      );
      expect(state.executionQueuedMessages["task-456"]?.[0].content).toBe(
        "Message for task 456"
      );
    });
  });

  describe("deleteExecutionQueuedMessage", () => {
    it("removes message from execution queue", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "Test");
      const messageId = useChatStore.getState().executionQueuedMessages["task-123"]?.[0].id || "";

      useChatStore.getState().deleteExecutionQueuedMessage("task-123", messageId);

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]).toBeUndefined();
    });

    it("cleans up empty arrays", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "Test");
      const messageId = useChatStore.getState().executionQueuedMessages["task-123"]?.[0].id || "";

      useChatStore.getState().deleteExecutionQueuedMessage("task-123", messageId);

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]).toBeUndefined();
    });

    it("does nothing if task queue not found", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "Test");

      useChatStore.getState().deleteExecutionQueuedMessage("task-456", "nonexistent-id");

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]).toHaveLength(1);
    });

    it("does nothing if message not found", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "Test");

      useChatStore.getState().deleteExecutionQueuedMessage("task-123", "nonexistent-id");

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]).toHaveLength(1);
    });

    it("only removes specified message", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "First");
      useChatStore.getState().queueExecutionMessage("task-123", "Second");
      useChatStore.getState().queueExecutionMessage("task-123", "Third");
      const secondId = useChatStore.getState().executionQueuedMessages["task-123"]?.[1].id || "";

      useChatStore.getState().deleteExecutionQueuedMessage("task-123", secondId);

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-123"]).toHaveLength(2);
      expect(state.executionQueuedMessages["task-123"]?.[0].content).toBe("First");
      expect(state.executionQueuedMessages["task-123"]?.[1].content).toBe("Third");
    });

    it("preserves other task queues", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "First");
      useChatStore.getState().queueExecutionMessage("task-456", "Second");
      const messageId = useChatStore.getState().executionQueuedMessages["task-123"]?.[0].id || "";

      useChatStore.getState().deleteExecutionQueuedMessage("task-123", messageId);

      const state = useChatStore.getState();
      expect(state.executionQueuedMessages["task-456"]).toHaveLength(1);
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
      queuedMessages: [],
      executionQueuedMessages: {},
      isAgentRunning: false,
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
    it("returns queued messages", () => {
      useChatStore.getState().queueMessage("First");
      useChatStore.getState().queueMessage("Second");

      const result = selectQueuedMessages(useChatStore.getState());

      expect(result).toHaveLength(2);
      expect(result[0].content).toBe("First");
      expect(result[1].content).toBe("Second");
    });

    it("returns empty array when no queued messages", () => {
      const result = selectQueuedMessages(useChatStore.getState());

      expect(result).toHaveLength(0);
    });
  });

  describe("selectIsAgentRunning", () => {
    it("returns true when agent is running", () => {
      useChatStore.setState({ isAgentRunning: true });

      const result = selectIsAgentRunning(useChatStore.getState());

      expect(result).toBe(true);
    });

    it("returns false when agent is not running", () => {
      const result = selectIsAgentRunning(useChatStore.getState());

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

  describe("selectExecutionQueuedMessages", () => {
    it("returns queued execution messages for task", () => {
      useChatStore.getState().queueExecutionMessage("task-123", "First");
      useChatStore.getState().queueExecutionMessage("task-123", "Second");

      const result = selectExecutionQueuedMessages("task-123")(useChatStore.getState());

      expect(result).toHaveLength(2);
      expect(result[0].content).toBe("First");
      expect(result[1].content).toBe("Second");
    });

    it("returns empty array for unknown task", () => {
      const result = selectExecutionQueuedMessages("task-unknown")(useChatStore.getState());

      expect(result).toHaveLength(0);
    });

    it("returns empty array when no execution queues exist", () => {
      const result = selectExecutionQueuedMessages("task-123")(useChatStore.getState());

      expect(result).toHaveLength(0);
    });
  });
});
