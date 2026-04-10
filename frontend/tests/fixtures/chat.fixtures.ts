import { expect, Page } from "@playwright/test";
import { setupApp } from "./setup.fixtures";
import {
  IDEATION_REPLAY_CONTEXT,
  TASK_REPLAY_CONTEXTS,
  type MockChatScenarioName,
} from "@/api-mock/chat-scenarios";

export type ChatScenarioName = MockChatScenarioName;

type TaskChatScenarioName = Exclude<ChatScenarioName, "ideation_db_widget_mix">;

export async function seedChatScenario(page: Page, scenario: ChatScenarioName) {
  await page.evaluate(({ scenarioName }) => {
    const mockChatApi = (window as Window).__mockChatApi;
    const queryClient = (window as Window).__queryClient;

    if (!mockChatApi) {
      throw new Error("Mock chat API not available");
    }

    mockChatApi.reset();
    mockChatApi.seedScenario(scenarioName);
    queryClient?.invalidateQueries();
  }, { scenarioName: scenario });
}

export async function setupIdeationChatScenario(page: Page, scenario: Extract<ChatScenarioName, "ideation_db_widget_mix">) {
  await setupApp(page);
  await page.click('[data-testid="nav-ideation"]');
  await page.waitForSelector('[data-testid="ideation-view"]', { timeout: 10000 });
  await page.waitForTimeout(250);
  await seedChatScenario(page, scenario);
  await page.evaluate(async (replayContext) => {
    const chatStore = (window as Window).__chatStore;
    const ideationStore = (window as Window).__ideationStore;
    const mockChatApi = (window as Window).__mockChatApi;
    const queryClient = (window as Window).__queryClient;
    const { conversationId, contextType, contextId } = replayContext;

    if (!ideationStore) {
      throw new Error("Ideation store not available");
    }
    if (!mockChatApi) {
      throw new Error("Mock chat API not available");
    }
    if (!queryClient) {
      throw new Error("Query client not available");
    }

    ideationStore.getState().selectSession({
      id: "session-mock-1",
      projectId: "project-mock-1",
      title: "Demo Ideation Session",
      titleSource: null,
      status: "active",
      planArtifactId: null,
      seedTaskId: null,
      parentSessionId: null,
      teamMode: null,
      teamConfig: null,
      createdAt: "2026-03-11T21:51:34.589194Z",
      updatedAt: "2026-03-11T21:58:43.308888Z",
      archivedAt: null,
      convertedAt: null,
      verificationStatus: "unverified",
      verificationInProgress: false,
      gapScore: null,
      sessionPurpose: "general",
      acceptanceStatus: null,
    });

    const conversations = await mockChatApi.listConversations(contextType, contextId);
    const conversation = await mockChatApi.getConversation(conversationId);

    queryClient.setQueryData(
      ["chat", "conversations", contextType, contextId],
      conversations,
    );
    queryClient.setQueryData(
      ["chat", "conversations", conversationId],
      conversation,
    );

    chatStore?.getState().setActiveConversation(`session:${contextId}`, conversationId);
  }, IDEATION_REPLAY_CONTEXT);
  await page.waitForFunction((replayContext) => {
    const chatStore = (window as Window).__chatStore;
    return (
      chatStore?.getState().activeConversationIds?.[`session:${replayContext.contextId}`] ===
      replayContext.conversationId
    );
  }, IDEATION_REPLAY_CONTEXT);
  await expect(page.locator('[data-testid="conversation-panel"]')).toBeVisible();
  await expect(page.locator('[data-testid="integrated-chat-messages"]')).toBeVisible();
  await expect(page.locator('[data-testid="chat-session-provider-context"]')).toBeVisible();
}

export async function setupTaskChatScenario(page: Page, scenario: TaskChatScenarioName) {
  const { contextType, contextId, conversationId } = TASK_REPLAY_CONTEXTS[scenario];

  await setupApp(page);
  await seedChatScenario(page, scenario);
  await page.evaluate(({ taskId }) => {
    const uiStore = (window as Window).__uiStore as {
      getState(): {
        setCurrentView(view: string): void;
        setSelectedTaskId(taskId: string | null): void;
      };
    } | undefined;

    if (!uiStore) {
      throw new Error("UI store not available");
    }

    uiStore.getState().setCurrentView("kanban");
    uiStore.getState().setSelectedTaskId(taskId);
  }, { taskId: contextId });
  await page.waitForSelector('[data-testid="task-detail-overlay"]', { timeout: 10000 });
  await page.waitForSelector('[data-testid="integrated-chat-panel"]', { timeout: 10000 });

  await page.evaluate(async ({ currentContextType, currentContextId, currentConversationId }) => {
    const chatStore = (window as Window).__chatStore;
    const mockChatApi = (window as Window).__mockChatApi;
    const queryClient = (window as Window).__queryClient;

    if (!mockChatApi) {
      throw new Error("Mock chat API not available");
    }
    if (!queryClient) {
      throw new Error("Query client not available");
    }

    const conversations = await mockChatApi.listConversations(
      currentContextType,
      currentContextId,
    );
    const conversation = await mockChatApi.getConversation(currentConversationId);

    queryClient.setQueryData(
      ["chat", "conversations", currentContextType, currentContextId],
      conversations,
    );
    queryClient.setQueryData(
      ["chat", "conversations", currentConversationId],
      conversation,
    );

    chatStore?.getState().setActiveConversation(
      `${currentContextType}:${currentContextId}`,
      currentConversationId,
    );
  }, {
    currentContextType: contextType,
    currentContextId: contextId,
    currentConversationId: conversationId,
  });

  await page.waitForFunction(({ storeKey, currentConversationId }) => {
    const chatStore = (window as Window).__chatStore;
    return chatStore?.getState().activeConversationIds?.[storeKey] === currentConversationId;
  }, {
    storeKey: `${contextType}:${contextId}`,
    currentConversationId: conversationId,
  });

  await expect(page.locator('[data-testid="integrated-chat-panel"]')).toBeVisible();
  await expect(page.locator('[data-testid="chat-session-provider-context"]')).toBeVisible();
}
