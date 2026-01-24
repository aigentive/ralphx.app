import { describe, it, expect, beforeEach } from "vitest";
import {
  useChatStore,
  selectMessagesForContext,
  selectMessageCount,
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
});
