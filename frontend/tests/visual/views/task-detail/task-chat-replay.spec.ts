import { expect, test, type Page } from "@playwright/test";
import { setupTaskChatScenario } from "../../../fixtures/chat.fixtures";

const mergeContractContextId = "task-mock-merge-incomplete";
const mergeContractConversationId = "conv-merge-contract";

const mergeContractConversation = {
  id: mergeContractConversationId,
  contextType: "merge",
  contextId: mergeContractContextId,
  claudeSessionId: null,
  providerSessionId: "thread-merge-contract",
  providerHarness: "codex",
  upstreamProvider: "openai",
  providerProfile: null,
  title: "Merge Contract Session",
  messageCount: 0,
  lastMessageAt: null,
  createdAt: "2026-04-10T10:00:00.000Z",
  updatedAt: "2026-04-10T10:00:00.000Z",
} as const;

const mergeUserMessage = {
  id: "msg-merge-user-1",
  sessionId: null,
  projectId: "project-mock-1",
  taskId: mergeContractContextId,
  role: "user",
  content: "Please resolve the merge path.",
  metadata: null,
  parentMessageId: null,
  conversationId: mergeContractConversationId,
  toolCalls: null,
  contentBlocks: null,
  sender: null,
  attributionSource: "native",
  providerHarness: null,
  providerSessionId: null,
  upstreamProvider: null,
  providerProfile: null,
  logicalModel: null,
  effectiveModelId: null,
  logicalEffort: null,
  effectiveEffort: null,
  inputTokens: null,
  outputTokens: null,
  cacheCreationTokens: null,
  cacheReadTokens: null,
  estimatedUsd: null,
  createdAt: "2026-04-10T10:00:00.000Z",
} as const;

const mergeLiveProviderMessage = {
  id: "msg-merge-assistant-1",
  sessionId: null,
  projectId: "project-mock-1",
  taskId: mergeContractContextId,
  role: "assistant",
  content: "I am checking the merge target now.",
  metadata: null,
  parentMessageId: "msg-merge-user-1",
  conversationId: mergeContractConversationId,
  toolCalls: [
    {
      id: "tool-merge-target-1",
      name: "ralphx::get_merge_target",
      arguments: { task_id: mergeContractContextId },
      result: { branch: "main" },
    },
  ],
  contentBlocks: [
    { type: "text", text: "I am checking the merge target now." },
    {
      type: "tool_use",
      id: "tool-merge-target-1",
      name: "ralphx::get_merge_target",
      arguments: { task_id: mergeContractContextId },
      result: { branch: "main" },
    },
  ],
  sender: null,
  attributionSource: "native",
  providerHarness: "codex",
  providerSessionId: "thread-merge-contract",
  upstreamProvider: "openai",
  providerProfile: null,
  logicalModel: "gpt-5.4",
  effectiveModelId: "gpt-5.4",
  logicalEffort: "high",
  effectiveEffort: "xhigh",
  inputTokens: 1440,
  outputTokens: 180,
  cacheCreationTokens: 20,
  cacheReadTokens: 900,
  estimatedUsd: null,
  createdAt: "2026-04-10T10:01:00.000Z",
} as const;

const mergeFinalProviderMessage = {
  ...mergeLiveProviderMessage,
  content: "I am checking the merge target now.\n\nThe target is main, and the merge conflict is isolated to src/commands/gateway.ts.",
  contentBlocks: [
    { type: "text", text: "I am checking the merge target now." },
    {
      type: "tool_use",
      id: "tool-merge-target-1",
      name: "ralphx::get_merge_target",
      arguments: { task_id: mergeContractContextId },
      result: { branch: "main" },
    },
    {
      type: "text",
      text: "The target is main, and the merge conflict is isolated to src/commands/gateway.ts.",
    },
  ],
  outputTokens: 240,
  createdAt: "2026-04-10T10:02:00.000Z",
} as const;

async function seedTaskContractConversation(
  page: Page,
  messages: Array<typeof mergeUserMessage | typeof mergeLiveProviderMessage>,
) {
  await setupTaskChatScenario(page, "merge_db_compact");

  await page.evaluate(async ({ conversation, seededMessages }) => {
    const mockChatApi = window.__mockChatApi;
    const queryClient = window.__queryClient;
    const chatStore = window.__chatStore;

    if (!mockChatApi || !queryClient || !chatStore) {
      throw new Error("Expected mock chat app globals to be available");
    }

    mockChatApi.seedConversation(conversation, seededMessages);

    queryClient.setQueryData(["chat", "conversations", "merge", conversation.contextId], [conversation]);
    queryClient.setQueryData(["chat", "conversations", conversation.id], {
      conversation,
      messages: seededMessages,
    });

    chatStore.getState().setActiveConversation(`merge:${conversation.contextId}`, conversation.id);
  }, {
    conversation: mergeContractConversation,
    seededMessages: messages,
  });

  await expect(page.locator('[data-testid="integrated-chat-panel"]')).toBeVisible();
}

async function replaceTaskContractMessages(
  page: Page,
  messages: Array<typeof mergeUserMessage | typeof mergeLiveProviderMessage>,
) {
  await page.evaluate(async ({ conversationId, seededMessages }) => {
    const mockChatApi = window.__mockChatApi;
    const queryClient = window.__queryClient;

    if (!mockChatApi || !queryClient) {
      throw new Error("Expected mock chat app globals to be available");
    }

    mockChatApi.replaceMessages(conversationId, seededMessages);
    queryClient.setQueryData(["chat", "conversations", conversationId], {
      conversation: await mockChatApi.getConversation(conversationId).then((payload) => payload.conversation),
      messages: seededMessages,
    });
  }, {
    conversationId: mergeContractConversationId,
    seededMessages: messages,
  });
}

async function emitChatEvent(page: Page, event: string, payload: unknown) {
  await page.evaluate(async ({ eventName, eventPayload }) => {
    const eventBus = window.__eventBus;
    if (!eventBus) {
      throw new Error("Expected event bus to be available");
    }

    await eventBus.emit(eventName, eventPayload);
  }, { eventName: event, eventPayload: payload });
}

test.describe("Task Chat Replay", () => {
  test("renders DB-derived execution replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "execution_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(page.getByTestId("chat-session-provider-badge")).toHaveText(/Claude/i);
    await expect(
      panel.getByText("Execution replay sampled from a compact two-message worker conversation."),
    ).toBeVisible();
    await expect(
      panel.getByText("frontend/src/components/Chat/MessageItem.tsx"),
    ).toBeVisible();

    await page.getByTestId("chat-session-stats-button").click();
    await expect(page.getByText("980")).toBeVisible();
    await expect(page.getByText("164")).toBeVisible();
  });

  test("renders DB-derived review replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "review_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(page.getByTestId("chat-session-provider-badge")).toHaveText(/Claude/i);
    await expect(
      panel.getByText("Reviewer replay sampled from a compact two-message real conversation."),
    ).toBeVisible();
    await expect(panel.getByText(/Changes Requested/i)).toBeVisible();

    await page.getByTestId("chat-session-stats-button").click();
    await expect(page.getByText("1,506")).toBeVisible();
    await expect(page.getByText("203")).toBeVisible();
  });

  test("renders DB-derived merge replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "merge_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(page.getByTestId("chat-session-provider-badge")).toHaveText(/Codex/i);
    await expect(
      panel.getByText("Merge replay sampled from a compact two-message merger conversation."),
    ).toBeVisible();

    await page.getByTestId("chat-session-stats-button").click();
    await expect(page.getByText("1,244")).toBeVisible();
    await expect(page.getByText("188")).toBeVisible();
    await expect(page.getByText("gpt-5.4", { exact: true })).toBeVisible();
  });

  test("keeps live and finalized merge turns deduplicated with stable widget order", async ({ page }) => {
    await seedTaskContractConversation(page, [mergeUserMessage, mergeLiveProviderMessage]);

    await emitChatEvent(page, "agent:tool_call", {
      tool_name: "ralphx::get_merge_target",
      tool_id: "tool-merge-target-1",
      arguments: { task_id: mergeContractContextId },
      result: { branch: "main" },
      conversation_id: mergeContractConversationId,
      context_id: mergeContractContextId,
      context_type: "merge",
    });
    await emitChatEvent(page, "agent:chunk", {
      text: "I am checking the merge target now.",
      conversation_id: mergeContractConversationId,
      context_id: mergeContractContextId,
    });

    await expect(page.getByText("I am checking the merge target now.")).toHaveCount(1);
    await expect(page.locator('[data-testid="merge-widget-target"]')).toHaveCount(1);

    await emitChatEvent(page, "agent:message_created", {
      conversation_id: mergeContractConversationId,
      context_id: mergeContractContextId,
      context_type: "merge",
      role: "assistant",
      message_id: mergeFinalProviderMessage.id,
    });
    await replaceTaskContractMessages(page, [mergeUserMessage, mergeFinalProviderMessage]);

    await expect(page.getByText("I am checking the merge target now.")).toHaveCount(1);
    await expect(
      page.getByText("The target is main, and the merge conflict is isolated to src/commands/gateway.ts."),
    ).toHaveCount(1);
    await expect(page.locator('[data-testid="merge-widget-target"]')).toHaveCount(1);

    const firstText = page.getByText("I am checking the merge target now.");
    const widget = page.locator('[data-testid="merge-widget-target"]');
    const secondText = page.getByText(
      "The target is main, and the merge conflict is isolated to src/commands/gateway.ts.",
    );
    const firstBox = await firstText.boundingBox();
    const widgetBox = await widget.boundingBox();
    const secondBox = await secondText.boundingBox();

    expect(firstBox?.y).toBeLessThan(widgetBox?.y ?? Number.POSITIVE_INFINITY);
    expect(widgetBox?.y).toBeLessThan(secondBox?.y ?? Number.POSITIVE_INFINITY);
  });
});
