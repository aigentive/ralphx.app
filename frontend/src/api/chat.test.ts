import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  parseContentBlocks,
  parseToolCalls,
  listConversations,
  getConversation,
  createConversation,
  getAgentRunStatus,
  sendAgentMessage,
  getQueuedAgentMessages,
  deleteQueuedAgentMessage,
  isChatServiceAvailable,
  stopAgent,
  isAgentRunning,
  chatApi,
  getConversationActiveState,
} from "./chat";
import type { ConversationActiveStateResponse } from "./chat";

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
        provider_session_id: "thread-1",
        provider_harness: "codex",
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
    expect(result[0]).toMatchObject({
      contextType: "project",
      contextId: "p1",
      providerSessionId: "thread-1",
      providerHarness: "codex",
      claudeSessionId: null,
    });
  });

  it("preserves unknown provider harness values", async () => {
    mockInvoke.mockResolvedValue([
      {
        id: "c-unknown",
        context_type: "project",
        context_id: "p-unknown",
        claude_session_id: null,
        provider_session_id: "thread-unknown",
        provider_harness: "openai",
        title: "Unknown provider row",
        message_count: 1,
        last_message_at: null,
        created_at: "2026-01-24T10:00:00Z",
        updated_at: "2026-01-24T10:00:00Z",
      },
    ]);

    const result = await listConversations("project", "p-unknown");

    expect(result[0]).toMatchObject({
      providerSessionId: "thread-unknown",
      providerHarness: "openai",
      claudeSessionId: null,
    });
  });

  it("does not infer claude harness from provider session id alone", async () => {
    mockInvoke.mockResolvedValue([
      {
        id: "c2",
        context_type: "project",
        context_id: "p2",
        claude_session_id: null,
        provider_session_id: "thread-legacy",
        provider_harness: null,
        title: "Legacy provider row",
        message_count: 1,
        last_message_at: null,
        created_at: "2026-01-24T10:00:00Z",
        updated_at: "2026-01-24T10:00:00Z",
      },
    ]);

    const result = await listConversations("project", "p2");

    expect(result[0]).toMatchObject({
      providerSessionId: "thread-legacy",
      providerHarness: null,
      claudeSessionId: null,
    });
  });

  it("infers claude harness only from the legacy claude session id", async () => {
    mockInvoke.mockResolvedValue([
      {
        id: "c3",
        context_type: "project",
        context_id: "p3",
        claude_session_id: "claude-thread-1",
        provider_session_id: null,
        provider_harness: null,
        title: "Legacy claude row",
        message_count: 1,
        last_message_at: null,
        created_at: "2026-01-24T10:00:00Z",
        updated_at: "2026-01-24T10:00:00Z",
      },
    ]);

    const result = await listConversations("project", "p3");

    expect(result[0]).toMatchObject({
      providerSessionId: "claude-thread-1",
      providerHarness: "claude",
      claudeSessionId: "claude-thread-1",
    });
  });

  it("gets conversation with transformed messages", async () => {
    mockInvoke.mockResolvedValue({
      conversation: {
        id: "c1",
        context_type: "project",
        context_id: "p1",
        claude_session_id: null,
        provider_session_id: "thread-2",
        provider_harness: "codex",
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
      provider_session_id: null,
      provider_harness: null,
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
    expect(result).toEqual({ conversationId: "c1", agentRunId: "r1", isNewConversation: true, wasQueued: false, queuedMessageId: undefined });
  });

  it("lists queued messages", async () => {
    mockInvoke.mockResolvedValueOnce([{ id: "q1", content: "queued", created_at: "2026-01-24T10:00:00Z", is_editing: false }]);

    const list = await getQueuedAgentMessages("project", "p1");

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
    expect(chatApi.getConversationActiveState).toBe(getConversationActiveState);
  });
});

describe("getConversationActiveState", () => {
  let mockFetch: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockFetch = vi.fn();
    global.fetch = mockFetch;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("fetches conversation active state with stats fields", async () => {
    const mockResponse: ConversationActiveStateResponse = {
      is_active: true,
      tool_calls: [],
      streaming_tasks: [
        {
          tool_use_id: "toolu_abc123",
          description: "Running tests",
          subagent_type: "ralphx:coder",
          model: "sonnet",
          status: "completed",
          total_tokens: 5000,
          total_tool_uses: 12,
          duration_ms: 45000,
        },
      ],
      partial_text: "",
    };

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockResponse),
    });

    const result = await getConversationActiveState("conv-123");

    expect(mockFetch).toHaveBeenCalledWith(
      "http://localhost:3847/api/conversations/conv-123/active-state"
    );
    expect(result.is_active).toBe(true);
    expect(result.streaming_tasks).toHaveLength(1);
    const task = result.streaming_tasks[0];
    expect(task.tool_use_id).toBe("toolu_abc123");
    expect(task.total_tokens).toBe(5000);
    expect(task.total_tool_uses).toBe(12);
    expect(task.duration_ms).toBe(45000);
  });

  it("handles response with no stats fields (old format)", async () => {
    const mockResponse: ConversationActiveStateResponse = {
      is_active: true,
      tool_calls: [],
      streaming_tasks: [
        {
          tool_use_id: "toolu_xyz",
          status: "running",
        },
      ],
      partial_text: "Working...",
    };

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockResponse),
    });

    const result = await getConversationActiveState("conv-456");

    expect(result.streaming_tasks[0].total_tokens).toBeUndefined();
    expect(result.streaming_tasks[0].total_tool_uses).toBeUndefined();
    expect(result.streaming_tasks[0].duration_ms).toBeUndefined();
  });

  it("throws on non-ok response", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      status: 404,
    });

    await expect(getConversationActiveState("conv-missing")).rejects.toThrow(
      "Failed to get conversation active state: 404"
    );
  });
});
