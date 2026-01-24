import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  sendChatMessage,
  sendMessageWithContext,
  getSessionMessages,
  getRecentSessionMessages,
  getProjectMessages,
  getTaskMessages,
  deleteChatMessage,
  deleteSessionMessages,
  countSessionMessages,
  chatApi,
} from "./chat";
import type { ChatContext } from "../types/chat";

// Cast invoke to a mock function for testing
const mockInvoke = invoke as ReturnType<typeof vi.fn>;

// Helper to create mock chat message (snake_case - matches Rust backend)
const createMockMessageRaw = (overrides = {}) => ({
  id: "message-1",
  session_id: "session-1",
  project_id: null,
  task_id: null,
  role: "user",
  content: "Hello",
  metadata: null,
  parent_message_id: null,
  created_at: "2026-01-24T12:00:00Z",
  ...overrides,
});

describe("sendChatMessage", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should send message to session", async () => {
    const message = createMockMessageRaw();
    mockInvoke.mockResolvedValue(message);

    await sendChatMessage(
      { type: "session", sessionId: "session-1" },
      { content: "Hello" }
    );

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        session_id: "session-1",
        role: "user",
        content: "Hello",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should send message to project", async () => {
    const message = createMockMessageRaw({ project_id: "project-1", session_id: null });
    mockInvoke.mockResolvedValue(message);

    await sendChatMessage(
      { type: "project", projectId: "project-1" },
      { content: "Project message" }
    );

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        project_id: "project-1",
        role: "user",
        content: "Project message",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should send message about task", async () => {
    const message = createMockMessageRaw({ task_id: "task-1", session_id: null });
    mockInvoke.mockResolvedValue(message);

    await sendChatMessage(
      { type: "task", taskId: "task-1" },
      { content: "Task message" }
    );

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        task_id: "task-1",
        role: "user",
        content: "Task message",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should send message with custom role", async () => {
    const message = createMockMessageRaw({ role: "orchestrator" });
    mockInvoke.mockResolvedValue(message);

    await sendChatMessage(
      { type: "session", sessionId: "session-1" },
      { content: "Response", role: "orchestrator" }
    );

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        session_id: "session-1",
        role: "orchestrator",
        content: "Response",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should send message with metadata and parent", async () => {
    const message = createMockMessageRaw({
      metadata: '{"key":"value"}',
      parent_message_id: "parent-1",
    });
    mockInvoke.mockResolvedValue(message);

    await sendChatMessage(
      { type: "session", sessionId: "session-1" },
      {
        content: "Reply",
        metadata: '{"key":"value"}',
        parentMessageId: "parent-1",
      }
    );

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        session_id: "session-1",
        role: "user",
        content: "Reply",
        metadata: '{"key":"value"}',
        parent_message_id: "parent-1",
      },
    });
  });

  it("should return created message with camelCase fields", async () => {
    const message = createMockMessageRaw({
      parent_message_id: "parent-1",
      created_at: "2026-01-24T15:00:00Z",
    });
    mockInvoke.mockResolvedValue(message);

    const result = await sendChatMessage(
      { type: "session", sessionId: "session-1" },
      { content: "Hello" }
    );

    expect(result.sessionId).toBe("session-1");
    expect(result.parentMessageId).toBe("parent-1");
    expect(result.createdAt).toBe("2026-01-24T15:00:00Z");
  });

  it("should validate message schema", async () => {
    mockInvoke.mockResolvedValue({ invalid: "message" });

    await expect(
      sendChatMessage({ type: "session", sessionId: "s1" }, { content: "Test" })
    ).rejects.toThrow();
  });
});

describe("sendMessageWithContext", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should send message to ideation session", async () => {
    const message = createMockMessageRaw();
    mockInvoke.mockResolvedValue(message);

    const context: ChatContext = {
      view: "ideation",
      projectId: "project-1",
      ideationSessionId: "session-1",
    };

    await sendMessageWithContext(context, "Hello");

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        session_id: "session-1",
        role: "user",
        content: "Hello",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should throw error for ideation without sessionId", async () => {
    const context: ChatContext = {
      view: "ideation",
      projectId: "project-1",
    };

    await expect(sendMessageWithContext(context, "Hello")).rejects.toThrow(
      "Ideation context requires sessionId"
    );
  });

  it("should send message to task in task_detail view", async () => {
    const message = createMockMessageRaw({ task_id: "task-1", session_id: null });
    mockInvoke.mockResolvedValue(message);

    const context: ChatContext = {
      view: "task_detail",
      projectId: "project-1",
      selectedTaskId: "task-1",
    };

    await sendMessageWithContext(context, "Task comment");

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        task_id: "task-1",
        role: "user",
        content: "Task comment",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should throw error for task_detail without taskId", async () => {
    const context: ChatContext = {
      view: "task_detail",
      projectId: "project-1",
    };

    await expect(sendMessageWithContext(context, "Hello")).rejects.toThrow(
      "Task detail context requires selectedTaskId"
    );
  });

  it("should send message to task in kanban with selected task", async () => {
    const message = createMockMessageRaw({ task_id: "task-1", session_id: null });
    mockInvoke.mockResolvedValue(message);

    const context: ChatContext = {
      view: "kanban",
      projectId: "project-1",
      selectedTaskId: "task-1",
    };

    await sendMessageWithContext(context, "About this task");

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        task_id: "task-1",
        role: "user",
        content: "About this task",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should send message to project in kanban without selected task", async () => {
    const message = createMockMessageRaw({ project_id: "project-1", session_id: null });
    mockInvoke.mockResolvedValue(message);

    const context: ChatContext = {
      view: "kanban",
      projectId: "project-1",
    };

    await sendMessageWithContext(context, "General question");

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        project_id: "project-1",
        role: "user",
        content: "General question",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should send message to project for activity view", async () => {
    const message = createMockMessageRaw({ project_id: "project-1", session_id: null });
    mockInvoke.mockResolvedValue(message);

    const context: ChatContext = {
      view: "activity",
      projectId: "project-1",
    };

    await sendMessageWithContext(context, "Question");

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        project_id: "project-1",
        role: "user",
        content: "Question",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should send message to project for settings view", async () => {
    const message = createMockMessageRaw({ project_id: "project-1", session_id: null });
    mockInvoke.mockResolvedValue(message);

    const context: ChatContext = {
      view: "settings",
      projectId: "project-1",
    };

    await sendMessageWithContext(context, "Settings help");

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        project_id: "project-1",
        role: "user",
        content: "Settings help",
        metadata: undefined,
        parent_message_id: undefined,
      },
    });
  });

  it("should pass additional options", async () => {
    const message = createMockMessageRaw({ role: "system" });
    mockInvoke.mockResolvedValue(message);

    const context: ChatContext = {
      view: "ideation",
      projectId: "project-1",
      ideationSessionId: "session-1",
    };

    await sendMessageWithContext(context, "System message", {
      role: "system",
      metadata: '{"type":"notification"}',
    });

    expect(mockInvoke).toHaveBeenCalledWith("send_chat_message", {
      input: {
        session_id: "session-1",
        role: "system",
        content: "System message",
        metadata: '{"type":"notification"}',
        parent_message_id: undefined,
      },
    });
  });
});

describe("getSessionMessages", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_session_messages with session_id", async () => {
    mockInvoke.mockResolvedValue([createMockMessageRaw()]);

    await getSessionMessages("session-1");

    expect(mockInvoke).toHaveBeenCalledWith("get_session_messages", {
      session_id: "session-1",
    });
  });

  it("should return array of messages", async () => {
    const messages = [
      createMockMessageRaw({ id: "m1", content: "First" }),
      createMockMessageRaw({ id: "m2", content: "Second" }),
    ];
    mockInvoke.mockResolvedValue(messages);

    const result = await getSessionMessages("session-1");

    expect(result).toHaveLength(2);
    expect(result[0]?.content).toBe("First");
    expect(result[1]?.content).toBe("Second");
  });

  it("should return empty array when no messages", async () => {
    mockInvoke.mockResolvedValue([]);

    const result = await getSessionMessages("session-1");

    expect(result).toEqual([]);
  });
});

describe("getRecentSessionMessages", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_recent_session_messages with session_id and limit", async () => {
    mockInvoke.mockResolvedValue([createMockMessageRaw()]);

    await getRecentSessionMessages("session-1", 10);

    expect(mockInvoke).toHaveBeenCalledWith("get_recent_session_messages", {
      session_id: "session-1",
      limit: 10,
    });
  });

  it("should return limited messages", async () => {
    const messages = [
      createMockMessageRaw({ id: "m1" }),
      createMockMessageRaw({ id: "m2" }),
    ];
    mockInvoke.mockResolvedValue(messages);

    const result = await getRecentSessionMessages("session-1", 2);

    expect(result).toHaveLength(2);
  });
});

describe("getProjectMessages", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_project_messages with project_id", async () => {
    mockInvoke.mockResolvedValue([]);

    await getProjectMessages("project-1");

    expect(mockInvoke).toHaveBeenCalledWith("get_project_messages", {
      project_id: "project-1",
    });
  });

  it("should return project messages", async () => {
    const messages = [
      createMockMessageRaw({
        id: "m1",
        project_id: "project-1",
        session_id: null,
      }),
    ];
    mockInvoke.mockResolvedValue(messages);

    const result = await getProjectMessages("project-1");

    expect(result).toHaveLength(1);
    expect(result[0]?.projectId).toBe("project-1");
  });
});

describe("getTaskMessages", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_task_messages with task_id", async () => {
    mockInvoke.mockResolvedValue([]);

    await getTaskMessages("task-1");

    expect(mockInvoke).toHaveBeenCalledWith("get_task_messages", {
      task_id: "task-1",
    });
  });

  it("should return task messages", async () => {
    const messages = [
      createMockMessageRaw({ id: "m1", task_id: "task-1", session_id: null }),
    ];
    mockInvoke.mockResolvedValue(messages);

    const result = await getTaskMessages("task-1");

    expect(result).toHaveLength(1);
    expect(result[0]?.taskId).toBe("task-1");
  });
});

describe("deleteChatMessage", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call delete_chat_message with id", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await deleteChatMessage("message-1");

    expect(mockInvoke).toHaveBeenCalledWith("delete_chat_message", {
      id: "message-1",
    });
  });

  it("should propagate errors", async () => {
    mockInvoke.mockRejectedValue(new Error("Message not found"));

    await expect(deleteChatMessage("nonexistent")).rejects.toThrow(
      "Message not found"
    );
  });
});

describe("deleteSessionMessages", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call delete_session_messages with session_id", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await deleteSessionMessages("session-1");

    expect(mockInvoke).toHaveBeenCalledWith("delete_session_messages", {
      session_id: "session-1",
    });
  });
});

describe("countSessionMessages", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call count_session_messages with session_id", async () => {
    mockInvoke.mockResolvedValue(5);

    const result = await countSessionMessages("session-1");

    expect(mockInvoke).toHaveBeenCalledWith("count_session_messages", {
      session_id: "session-1",
    });
    expect(result).toBe(5);
  });

  it("should return 0 for empty session", async () => {
    mockInvoke.mockResolvedValue(0);

    const result = await countSessionMessages("session-1");

    expect(result).toBe(0);
  });
});

describe("chatApi namespace", () => {
  it("should export all functions", () => {
    expect(chatApi.sendMessage).toBe(sendChatMessage);
    expect(chatApi.sendMessageWithContext).toBe(sendMessageWithContext);
    expect(chatApi.getSessionMessages).toBe(getSessionMessages);
    expect(chatApi.getRecentSessionMessages).toBe(getRecentSessionMessages);
    expect(chatApi.getProjectMessages).toBe(getProjectMessages);
    expect(chatApi.getTaskMessages).toBe(getTaskMessages);
    expect(chatApi.deleteMessage).toBe(deleteChatMessage);
    expect(chatApi.deleteSessionMessages).toBe(deleteSessionMessages);
    expect(chatApi.countSessionMessages).toBe(countSessionMessages);
  });

  it("should have 9 functions", () => {
    expect(Object.keys(chatApi)).toHaveLength(9);
  });
});
