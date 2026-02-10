import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  parseContentBlocks,
  parseToolCalls,
  listConversations,
  getConversation,
  createConversation,
  getAgentRunStatus,
  sendAgentMessage,
  queueAgentMessage,
  getQueuedAgentMessages,
  deleteQueuedAgentMessage,
  isChatServiceAvailable,
  stopAgent,
  isAgentRunning,
  chatApi,
} from "./chat";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("chat api", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("parses tool calls", () => {
    const parsed = parseToolCalls('[{"id":"t1","name":"bash","arguments":{"command":"ls"}}]');
    expect(parsed).toHaveLength(1);
    expect(parsed[0]).toMatchObject({ id: "t1", name: "bash" });
  });

  it("parses content blocks", () => {
    const parsed = parseContentBlocks('[{"type":"text","text":"hello"}]');
    expect(parsed).toHaveLength(1);
    expect(parsed[0]).toMatchObject({ type: "text", text: "hello" });
  });

  it("lists conversations", async () => {
    mockInvoke.mockResolvedValue([
      {
        id: "c1",
        context_type: "project",
        context_id: "p1",
        claude_session_id: null,
        title: "Title",
        message_count: 2,
        last_message_at: null,
        created_at: "2026-01-24T10:00:00Z",
        updated_at: "2026-01-24T10:00:00Z",
      },
    ]);

    const result = await listConversations("project", "p1");

    expect(mockInvoke).toHaveBeenCalledWith("list_agent_conversations", {
      contextType: "project",
      contextId: "p1",
    });
    expect(result[0]).toMatchObject({ contextType: "project", contextId: "p1" });
  });

  it("gets conversation with transformed messages", async () => {
    mockInvoke.mockResolvedValue({
      conversation: {
        id: "c1",
        context_type: "project",
        context_id: "p1",
        claude_session_id: null,
        title: null,
        message_count: 1,
        last_message_at: null,
        created_at: "2026-01-24T10:00:00Z",
        updated_at: "2026-01-24T10:00:00Z",
      },
      messages: [
        {
          id: "m1",
          role: "user",
          content: "Hello",
          tool_calls: null,
          content_blocks: null,
          created_at: "2026-01-24T10:00:00Z",
        },
      ],
    });

    const result = await getConversation("c1");

    expect(mockInvoke).toHaveBeenCalledWith("get_agent_conversation", { conversationId: "c1" });
    expect(result.messages[0]).toMatchObject({ id: "m1", createdAt: "2026-01-24T10:00:00Z" });
  });

  it("creates conversation", async () => {
    mockInvoke.mockResolvedValue({
      id: "c1",
      context_type: "task",
      context_id: "t1",
      claude_session_id: null,
      title: null,
      message_count: 0,
      last_message_at: null,
      created_at: "2026-01-24T10:00:00Z",
      updated_at: "2026-01-24T10:00:00Z",
    });

    await createConversation("task", "t1");

    expect(mockInvoke).toHaveBeenCalledWith("create_agent_conversation", {
      input: { contextType: "task", contextId: "t1" },
    });
  });

  it("gets nullable agent run status", async () => {
    mockInvoke.mockResolvedValue(null);
    const result = await getAgentRunStatus("c1");
    expect(result).toBeNull();
  });

  it("sends unified agent message", async () => {
    mockInvoke.mockResolvedValue({
      conversation_id: "c1",
      agent_run_id: "r1",
      is_new_conversation: true,
    });

    const result = await sendAgentMessage("project", "p1", "Hello");

    expect(mockInvoke).toHaveBeenCalledWith("send_agent_message", {
      input: { contextType: "project", contextId: "p1", content: "Hello" },
    });
    expect(result).toEqual({ conversationId: "c1", agentRunId: "r1", isNewConversation: true });
  });

  it("queues and lists queued messages", async () => {
    mockInvoke
      .mockResolvedValueOnce({ id: "q1", content: "queued", created_at: "2026-01-24T10:00:00Z", is_editing: false })
      .mockResolvedValueOnce([{ id: "q1", content: "queued", created_at: "2026-01-24T10:00:00Z", is_editing: false }]);

    const queued = await queueAgentMessage("project", "p1", "queued", "client-1");
    const list = await getQueuedAgentMessages("project", "p1");

    expect(queued).toMatchObject({ id: "q1", isEditing: false });
    expect(list).toHaveLength(1);
  });

  it("deletes queued message", async () => {
    mockInvoke.mockResolvedValue(true);
    const result = await deleteQueuedAgentMessage("project", "p1", "q1");
    expect(result).toBe(true);
  });

  it("checks service and running state and stops agent", async () => {
    mockInvoke
      .mockResolvedValueOnce(true)
      .mockResolvedValueOnce(true)
      .mockResolvedValueOnce(false);

    expect(await isChatServiceAvailable()).toBe(true);
    expect(await isAgentRunning("project", "p1")).toBe(true);
    expect(await stopAgent("project", "p1")).toBe(false);
  });

  it("exports chatApi namespace", () => {
    expect(chatApi.sendAgentMessage).toBe(sendAgentMessage);
    expect(chatApi.listConversations).toBe(listConversations);
  });
});
