import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { ideationApi } from "@/api/ideation";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useAgentSessionStore, type AgentRuntimeSelection } from "@/stores/agentSessionStore";
import type { AgentConversation } from "./agentConversations";
import { AgentsChatHeader, AgentsView } from "./AgentsView";

const {
  useProjectsMock,
  useProjectAgentConversationsMock,
  useConversationMock,
  sendAgentMessageMock,
  createConversationMock,
  spawnConversationSessionNamerMock,
  updateConversationTitleMock,
  archiveConversationMock,
  restoreConversationMock,
} = vi.hoisted(() => ({
  useProjectsMock: vi.fn(),
  useProjectAgentConversationsMock: vi.fn(),
  useConversationMock: vi.fn(),
  sendAgentMessageMock: vi.fn(),
  createConversationMock: vi.fn(),
  spawnConversationSessionNamerMock: vi.fn(),
  updateConversationTitleMock: vi.fn(),
  archiveConversationMock: vi.fn(),
  restoreConversationMock: vi.fn(),
}));

vi.mock("@/hooks/useProjects", () => ({
  useProjects: () => useProjectsMock(),
}));

vi.mock("./useProjectAgentConversations", () => ({
  agentConversationKeys: {
    all: ["agents", "project-conversations"],
    project: (projectId: string) => ["agents", "project-conversations", projectId],
    projectList: (projectId: string, includeArchived: boolean, search = "") => [
      "agents",
      "project-conversations",
      projectId,
      "archived",
      includeArchived,
      "search",
      search.trim().toLowerCase(),
    ],
  },
  useProjectAgentConversations: (
    projectId: string | null | undefined,
    includeArchived = false,
    options?: { search?: string }
  ) => useProjectAgentConversationsMock(projectId, includeArchived, options),
}));

vi.mock("@/hooks/useChat", () => ({
  chatKeys: {
    conversation: (conversationId: string) => ["chat", "conversations", conversationId],
    conversationList: (contextType: string, contextId: string) => [
      "chat",
      "conversations",
      contextType,
      contextId,
    ],
  },
  invalidateConversationDataQueries: vi.fn(),
  useConversation: (conversationId: string | null) => useConversationMock(conversationId),
}));

vi.mock("@/api/chat", () => ({
  chatApi: {
    sendAgentMessage: (...args: unknown[]) => sendAgentMessageMock(...args),
    createConversation: (...args: unknown[]) => createConversationMock(...args),
    spawnConversationSessionNamer: (...args: unknown[]) =>
      spawnConversationSessionNamerMock(...args),
    updateConversationTitle: (...args: unknown[]) => updateConversationTitleMock(...args),
    archiveConversation: (...args: unknown[]) => archiveConversationMock(...args),
    restoreConversation: (...args: unknown[]) => restoreConversationMock(...args),
  },
}));

vi.mock("@/api/ideation", () => ({
  ideationApi: {
    sessions: {
      getWithData: vi.fn(),
      updateTitle: vi.fn(),
      archive: vi.fn(),
      reopen: vi.fn(),
    },
  },
}));

vi.mock("@/components/Chat/IntegratedChatPanel", () => ({
  IntegratedChatPanel: ({
    headerContent,
    contentWidthClassName,
  }: {
    headerContent?: ReactNode;
    contentWidthClassName?: string;
  }) => (
    <div
      data-testid="integrated-chat-panel"
      data-content-width-class={contentWidthClassName ?? ""}
    >
      {headerContent}
    </div>
  ),
}));

vi.mock("./AgentsArtifactPane", () => ({
  AgentsArtifactPane: () => <div data-testid="agents-artifact-pane" />,
}));

vi.mock("./useProjectAgentBridgeEvents", () => ({
  useProjectAgentBridgeEvents: () => undefined,
}));

vi.mock("./useAgentConversationTitleEvents", () => ({
  useAgentConversationTitleEvents: () => undefined,
}));

const runtime: AgentRuntimeSelection = {
  provider: "codex",
  modelId: "gpt-5.4",
};

const project = {
  id: "project-1",
  name: "ralphx",
  workingDirectory: "/tmp/ralphx",
  gitMode: "worktree" as const,
  baseBranch: null,
  worktreeParentDirectory: null,
  useFeatureBranches: true,
  mergeValidationMode: "block" as const,
  detectedAnalysis: null,
  customAnalysis: null,
  analyzedAt: null,
  githubPrEnabled: false,
  createdAt: "2026-04-23T09:00:00Z",
  updatedAt: "2026-04-23T09:00:00Z",
};

const conversation = (
  overrides: Partial<AgentConversation> = {}
): AgentConversation => ({
  id: "conversation-1",
  contextType: "project",
  contextId: "project-1",
  claudeSessionId: null,
  providerSessionId: "thread-1",
  providerHarness: "codex",
  upstreamProvider: null,
  providerProfile: null,
  title: "Untitled agent",
  messageCount: 1,
  lastMessageAt: "2026-04-23T09:00:00Z",
  createdAt: "2026-04-23T09:00:00Z",
  updatedAt: "2026-04-23T09:00:00Z",
  archivedAt: null,
  projectId: "project-1",
  ideationSessionId: null,
  ...overrides,
});

function renderWithProviders(ui: ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>{ui}</TooltipProvider>
    </QueryClientProvider>
  );
}

function mockSidebarBreakpoint({ isLarge, isMedium }: { isLarge: boolean; isMedium: boolean }) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: vi.fn((query: string) => ({
      matches:
        query === "(min-width: 1440px)"
          ? isLarge
          : query === "(min-width: 1280px)"
            ? isMedium
            : false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

function mockAgentViewData(agentConversation: AgentConversation = conversation()) {
  useProjectsMock.mockReturnValue({
    data: [project],
    isLoading: false,
  });
  useProjectAgentConversationsMock.mockReturnValue({
    data: [agentConversation],
    conversations: [agentConversation],
    isLoading: false,
    isSuccess: true,
    hasNextPage: false,
    isFetchingNextPage: false,
    fetchNextPage: vi.fn(),
  });
  useConversationMock.mockImplementation((conversationId: string | null) => ({
    data:
      conversationId === agentConversation.id
        ? {
            conversation: agentConversation,
            messages: [],
          }
        : null,
    isLoading: false,
  }));
}

function mockSessionWithData(
  overrides?: Partial<Awaited<ReturnType<typeof ideationApi.sessions.getWithData>>["session"]>,
  proposals: Awaited<ReturnType<typeof ideationApi.sessions.getWithData>>["proposals"] = []
) {
  vi.mocked(ideationApi.sessions.getWithData).mockResolvedValue({
    session: {
      id: "session-1",
      projectId: "project-1",
      title: "Agent Plan",
      titleSource: "auto",
      status: "active",
      planArtifactId: null,
      seedTaskId: null,
      parentSessionId: null,
      teamMode: null,
      teamConfig: null,
      createdAt: "2026-04-23T09:00:00Z",
      updatedAt: "2026-04-23T09:00:00Z",
      archivedAt: null,
      convertedAt: null,
      verificationStatus: "unverified",
      verificationInProgress: false,
      gapScore: null,
      inheritedPlanArtifactId: null,
      sessionPurpose: "general",
      acceptanceStatus: null,
      ...overrides,
    },
    proposals,
    messages: [],
  });
}

function resetAgentSessionState(
  overrides: Partial<ReturnType<typeof useAgentSessionStore.getState>> = {}
) {
  useAgentSessionStore.setState({
    focusedProjectId: "project-1",
    selectedProjectId: null,
    selectedConversationId: null,
    expandedProjectIds: { "project-1": true },
    artifactByConversationId: {},
    runtimeByConversationId: {},
    lastRuntimeByProjectId: {
      "project-1": runtime,
    },
    ...overrides,
  });
}

function renderAgentsView() {
  return renderWithProviders(
    <AgentsView projectId="project-1" onCreateProject={vi.fn()} />
  );
}

function selectSidebarConversationRow() {
  const row = screen.getByTestId("agents-session-conversation-1");
  fireEvent.click(within(row).getAllByRole("button")[0] ?? row);
  return row;
}

describe("AgentsChatHeader", () => {
  beforeEach(() => {
    useProjectAgentConversationsMock.mockReset();
    useProjectsMock.mockReset();
    useConversationMock.mockReset();
  });

  it("opts the title button out of the high-contrast default button border", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation()}
        runtime={runtime}
        artifactOpen={false}
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.getByTestId("agents-chat-title-button")).toHaveAttribute(
      "data-theme-button-skip",
      "true"
    );
  });

  it("hides artifact shortcut buttons while the artifact pane is open", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation()}
        runtime={runtime}
        artifactOpen
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.queryByLabelText("Plan")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Close artifacts")).toBeInTheDocument();
  });
});

describe("AgentsView", () => {
  beforeEach(() => {
    mockSidebarBreakpoint({ isLarge: true, isMedium: true });
    window.localStorage.clear();
    useProjectAgentConversationsMock.mockReset();
    useProjectsMock.mockReset();
    useConversationMock.mockReset();
    sendAgentMessageMock.mockReset();
    createConversationMock.mockReset();
    spawnConversationSessionNamerMock.mockReset();
    updateConversationTitleMock.mockReset();
    archiveConversationMock.mockReset();
    restoreConversationMock.mockReset();

    sendAgentMessageMock.mockResolvedValue({
      conversationId: "conversation-2",
      agentRunId: "run-2",
      isNewConversation: true,
      wasQueued: false,
      queuedAsPending: false,
      queuedMessageId: null,
    });
    createConversationMock.mockResolvedValue({ id: "conversation-seeded" });
    spawnConversationSessionNamerMock.mockResolvedValue(undefined);
    updateConversationTitleMock.mockResolvedValue({
      ...conversation(),
      id: "conversation-2",
      title: "Fix agent landing flow",
    });
    vi.mocked(ideationApi.sessions.getWithData).mockReset();
    mockSessionWithData();
    archiveConversationMock.mockResolvedValue(undefined);
    restoreConversationMock.mockResolvedValue(undefined);
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockResolvedValue(undefined);

    resetAgentSessionState();
  });

  it("defaults to the starter composer even when a conversation was previously selected", async () => {
    mockAgentViewData();

    renderAgentsView();

    await waitFor(() =>
      expect(screen.getByTestId("agents-start-composer")).toBeInTheDocument()
    );
    expect(screen.getByText("Start an agent conversation")).toBeInTheDocument();
    expect(screen.getByTestId("agents-start-project")).toBeInTheDocument();
    expect(screen.getByTestId("agents-start-provider")).toBeInTheDocument();
    expect(screen.getByTestId("agents-start-model")).toBeInTheDocument();
    expect(screen.getByTestId("agents-start-new-project")).toBeInTheDocument();
    expect(screen.queryByTestId("integrated-chat-panel")).not.toBeInTheDocument();
  });

  it("starts a new conversation directly from the starter composer and triggers the session namer", async () => {
    mockAgentViewData();

    renderAgentsView();

    fireEvent.change(screen.getByTestId("agents-start-textarea"), {
      target: { value: "fix agent landing flow" },
    });
    fireEvent.click(screen.getByTestId("agents-start-submit"));

    await waitFor(() =>
      expect(sendAgentMessageMock).toHaveBeenCalledWith(
        "project",
        "project-1",
        "fix agent landing flow",
        undefined,
        undefined,
        {
          providerHarness: "codex",
          modelId: "gpt-5.4",
        }
      )
    );
    await waitFor(() =>
      expect(spawnConversationSessionNamerMock).toHaveBeenCalledWith(
        "conversation-2",
        "fix agent landing flow"
      )
    );
  });

  it("uploads starter attachments against a seeded conversation before sending the first message", async () => {
    mockAgentViewData();
    sendAgentMessageMock.mockResolvedValue({
      conversationId: "conversation-seeded",
      agentRunId: "run-2",
      isNewConversation: false,
      wasQueued: false,
      queuedAsPending: false,
      queuedMessageId: null,
    });
    vi.mocked(invoke).mockResolvedValue({ id: "attachment-1" });

    renderAgentsView();

    const fileInput = screen.getByTestId("attachment-file-input");
    const file = new File(["draft"], "notes.md", { type: "text/markdown" });

    fireEvent.change(fileInput, {
      target: { files: [file] },
    });
    fireEvent.change(screen.getByTestId("agents-start-textarea"), {
      target: { value: "review this note" },
    });
    fireEvent.click(screen.getByTestId("agents-start-submit"));

    await waitFor(() =>
      expect(createConversationMock).toHaveBeenCalledWith("project", "project-1")
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("upload_chat_attachment", {
        input: expect.objectContaining({
          conversationId: "conversation-seeded",
          fileName: "notes.md",
          mimeType: "text/markdown",
        }),
      })
    );
    await waitFor(() =>
      expect(sendAgentMessageMock).toHaveBeenCalledWith(
        "project",
        "project-1",
        "review this note",
        ["attachment-1"],
        undefined,
        {
          conversationId: "conversation-seeded",
          providerHarness: "codex",
          modelId: "gpt-5.4",
        }
      )
    );
  });

  it("restores persisted artifact width, enforces 320px mins, and resets to default on double click", async () => {
    window.localStorage.setItem("ralphx-agents-artifact-width", "480");
    mockAgentViewData(
      conversation({
        contextType: "ideation",
        contextId: "session-1",
        ideationSessionId: "session-1",
      })
    );
    mockSessionWithData({ planArtifactId: "plan-1" });
    resetAgentSessionState({
      artifactByConversationId: {
        "conversation-1": {
          isOpen: true,
          activeTab: "plan",
          taskMode: "graph",
        },
      },
    });

    renderAgentsView();
    selectSidebarConversationRow();

    const pane = await screen.findByTestId("agents-artifact-resizable-pane");
    expect(pane).toHaveStyle({
      width: "480px",
      minWidth: "320px",
      maxWidth: "calc(100% - 320px)",
    });

    fireEvent.doubleClick(screen.getByTestId("agents-artifact-resize-handle"));

    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-resizable-pane")).toHaveStyle({
        width: "66.666667%",
      })
    );
    expect(window.localStorage.getItem("ralphx-agents-artifact-width")).toBeNull();
  });

  it("resizes the artifact pane when the handle is dragged", async () => {
    mockAgentViewData(
      conversation({
        contextType: "ideation",
        contextId: "session-1",
        ideationSessionId: "session-1",
      })
    );
    mockSessionWithData({ planArtifactId: "plan-1" });
    resetAgentSessionState({
      artifactByConversationId: {
        "conversation-1": {
          isOpen: true,
          activeTab: "plan",
          taskMode: "graph",
        },
      },
    });

    renderAgentsView();
    selectSidebarConversationRow();

    await screen.findByTestId("agents-artifact-resizable-pane");

    const splitContainer = screen.getByTestId("agents-split-container");
    vi.spyOn(splitContainer, "getBoundingClientRect").mockReturnValue({
      x: 100,
      y: 0,
      width: 1200,
      height: 800,
      top: 0,
      right: 1300,
      bottom: 800,
      left: 100,
      toJSON: () => ({}),
    });

    fireEvent.mouseDown(screen.getByTestId("agents-artifact-resize-handle"));
    fireEvent.mouseMove(document, { clientX: 900 });
    fireEvent.mouseUp(document);

    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-resizable-pane")).toHaveStyle({
        width: "400px",
      })
    );
    expect(window.localStorage.getItem("ralphx-agents-artifact-width")).toBe("400");
  });

  it("deselects the selected agent when its row is clicked again", async () => {
    mockAgentViewData();

    renderAgentsView();
    const row = selectSidebarConversationRow();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );

    fireEvent.click(within(row).getAllByRole("button")[0] ?? row);

    await waitFor(() =>
      expect(screen.getByTestId("agents-start-composer")).toBeInTheDocument()
    );
    expect(screen.queryByTestId("integrated-chat-panel")).not.toBeInTheDocument();
  });

  it("keeps the artifact pane closed by default when the conversation has nothing to show", async () => {
    mockAgentViewData(
      conversation({
        contextType: "ideation",
        contextId: "session-1",
        ideationSessionId: "session-1",
      })
    );
    mockSessionWithData();

    renderAgentsView();
    selectSidebarConversationRow();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    expect(screen.getByTestId("integrated-chat-panel")).toHaveAttribute(
      "data-content-width-class",
      "max-w-[980px]"
    );
    expect(screen.queryByTestId("agents-artifact-pane")).not.toBeInTheDocument();
  });

  it("does not auto-restore a persisted artifact pane when the conversation still has nothing to show", async () => {
    mockAgentViewData();
    resetAgentSessionState({
      artifactByConversationId: {
        "conversation-1": {
          isOpen: true,
          activeTab: "plan",
          taskMode: "graph",
        },
      },
    });

    renderAgentsView();
    selectSidebarConversationRow();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    expect(screen.queryByTestId("agents-artifact-pane")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Open artifacts")).toBeInTheDocument();
  });

  it("still allows manually opening the artifact pane when the conversation has nothing to show", async () => {
    mockAgentViewData();

    renderAgentsView();
    selectSidebarConversationRow();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    expect(screen.queryByTestId("agents-artifact-pane")).not.toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("Open artifacts"));

    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-pane")).toBeInTheDocument()
    );
  });

  it("auto-opens the artifact pane when the conversation already has plan data", async () => {
    mockAgentViewData(
      conversation({
        contextType: "ideation",
        contextId: "session-1",
        ideationSessionId: "session-1",
      })
    );
    mockSessionWithData({ planArtifactId: "plan-1" });

    renderAgentsView();
    selectSidebarConversationRow();

    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-pane")).toBeInTheDocument()
    );
  });

  it("uses a collapsed sidebar strip on small screens and opens the overlay on demand", async () => {
    mockSidebarBreakpoint({ isLarge: false, isMedium: false });
    mockAgentViewData();

    renderAgentsView();

    expect(screen.getByTestId("agents-sidebar-toggle-strip")).toBeInTheDocument();
    expect(screen.getByTestId("agents-sidebar")).not.toBeVisible();

    fireEvent.click(screen.getByTestId("agents-sidebar-toggle-strip"));

    await waitFor(() =>
      expect(screen.getByTestId("agents-sidebar")).toBeInTheDocument()
    );
    expect(screen.getByTestId("agents-sidebar-overlay-backdrop")).toBeInTheDocument();
  });
});
