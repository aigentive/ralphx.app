import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import type { AgentConversationWorkspace } from "@/api/chat";
import { ideationApi } from "@/api/ideation";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useAgentSessionStore, type AgentRuntimeSelection } from "@/stores/agentSessionStore";
import { useAgentTerminalStore } from "./agentTerminalStore";
import type { AgentConversation } from "./agentConversations";
import { AgentsChatHeader, AgentsView } from "./AgentsView";

const {
  useProjectsMock,
  useProjectAgentConversationsMock,
  useConversationMock,
  startAgentConversationMock,
  getAgentConversationWorkspaceMock,
  getAgentConversationWorkspaceFreshnessMock,
  listAgentConversationWorkspacesByProjectMock,
  listConversationsMock,
  publishAgentConversationWorkspaceMock,
  switchAgentConversationModeMock,
  sendAgentMessageMock,
  createConversationMock,
  spawnConversationSessionNamerMock,
  updateConversationTitleMock,
  archiveConversationMock,
  restoreConversationMock,
  getPlanBranchesMock,
  listIdeationSessionsMock,
  getWorkspaceChangesMock,
  getWorkspaceDiffMock,
  toastErrorMock,
  toastSuccessMock,
  preloadAgentTerminalExperienceMock,
  terminalDrawerModuleLoadedMock,
  terminalDrawerMountMock,
  terminalDrawerUnmountMock,
} = vi.hoisted(() => ({
  useProjectsMock: vi.fn(),
  useProjectAgentConversationsMock: vi.fn(),
  useConversationMock: vi.fn(),
  startAgentConversationMock: vi.fn(),
  getAgentConversationWorkspaceMock: vi.fn(),
  getAgentConversationWorkspaceFreshnessMock: vi.fn(),
  listAgentConversationWorkspacesByProjectMock: vi.fn(),
  listConversationsMock: vi.fn(),
  publishAgentConversationWorkspaceMock: vi.fn(),
  switchAgentConversationModeMock: vi.fn(),
  sendAgentMessageMock: vi.fn(),
  createConversationMock: vi.fn(),
  spawnConversationSessionNamerMock: vi.fn(),
  updateConversationTitleMock: vi.fn(),
  archiveConversationMock: vi.fn(),
  restoreConversationMock: vi.fn(),
  getPlanBranchesMock: vi.fn(),
  listIdeationSessionsMock: vi.fn(),
  getWorkspaceChangesMock: vi.fn(),
  getWorkspaceDiffMock: vi.fn(),
  toastErrorMock: vi.fn(),
  toastSuccessMock: vi.fn(),
  preloadAgentTerminalExperienceMock: vi.fn(),
  terminalDrawerModuleLoadedMock: vi.fn(),
  terminalDrawerMountMock: vi.fn(),
  terminalDrawerUnmountMock: vi.fn(),
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

vi.mock("./AgentTerminalDrawer", async () => {
  terminalDrawerModuleLoadedMock();
  const React = await vi.importActual<typeof import("react")>("react");
  const ReactDom = await vi.importActual<typeof import("react-dom")>("react-dom");

  return {
    AgentTerminalDrawer: ({
      placement,
      onPlacementChange,
      dockElement,
    }: {
      placement: string;
      onPlacementChange: (placement: "auto" | "chat" | "panel") => void;
      dockElement: HTMLElement | null;
    }) => {
      React.useEffect(() => {
        terminalDrawerMountMock();
        return () => {
          terminalDrawerUnmountMock();
        };
      }, []);

      const drawer = (
        <div data-testid="agent-terminal-drawer" data-placement={placement}>
          <button
            type="button"
            data-testid="agent-terminal-placement"
            onClick={() =>
              onPlacementChange(
                placement === "auto" ? "panel" : placement === "panel" ? "chat" : "auto"
              )
            }
          >
            {placement}
          </button>
        </div>
      );

      return dockElement ? ReactDom.createPortal(drawer, dockElement) : drawer;
    },
  };
});

vi.mock("./agentTerminalPreload", () => ({
  preloadAgentTerminalDrawer: () => import("./AgentTerminalDrawer"),
  preloadAgentTerminalExperience: (...args: unknown[]) =>
    preloadAgentTerminalExperienceMock(...args),
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
    startAgentConversation: (...args: unknown[]) => startAgentConversationMock(...args),
    getAgentConversationWorkspace: (...args: unknown[]) =>
      getAgentConversationWorkspaceMock(...args),
    getAgentConversationWorkspaceFreshness: (...args: unknown[]) =>
      getAgentConversationWorkspaceFreshnessMock(...args),
    listAgentConversationWorkspacesByProject: (...args: unknown[]) =>
      listAgentConversationWorkspacesByProjectMock(...args),
    listConversations: (...args: unknown[]) => listConversationsMock(...args),
    publishAgentConversationWorkspace: (...args: unknown[]) =>
      publishAgentConversationWorkspaceMock(...args),
    switchAgentConversationMode: (...args: unknown[]) =>
      switchAgentConversationModeMock(...args),
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
      list: (...args: unknown[]) => listIdeationSessionsMock(...args),
      updateTitle: vi.fn(),
      archive: vi.fn(),
      reopen: vi.fn(),
    },
  },
}));

vi.mock("@/api/diff", () => ({
  diffApi: {
    getAgentConversationWorkspaceFileChanges: (...args: unknown[]) =>
      getWorkspaceChangesMock(...args),
    getAgentConversationWorkspaceFileDiff: (...args: unknown[]) =>
      getWorkspaceDiffMock(...args),
  },
}));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}));

vi.mock("@/api/plan-branch", () => ({
  planBranchApi: {
    getByProject: (...args: unknown[]) => getPlanBranchesMock(...args),
  },
}));

vi.mock("@/components/Chat/IntegratedChatPanel", () => ({
  IntegratedChatPanel: ({
    headerContent,
    contentWidthClassName,
    renderComposer,
  }: {
    headerContent?: ReactNode;
    contentWidthClassName?: string;
    renderComposer?: (props: Record<string, unknown>) => ReactNode;
  }) => (
    <div
      data-testid="integrated-chat-panel"
      data-content-width-class={contentWidthClassName ?? ""}
    >
      {headerContent}
      {renderComposer?.({
        onSend: vi.fn(),
        onStop: vi.fn(),
        agentStatus: "idle",
        isSending: false,
        isReadOnly: false,
        autoFocus: false,
        hasQueuedMessages: false,
        onEditLastQueued: vi.fn(),
        attachments: [],
        enableAttachments: false,
        onFilesSelected: vi.fn(),
        onRemoveAttachment: vi.fn(),
        attachmentsUploading: false,
      })}
    </div>
  ),
}));

vi.mock("./AgentsArtifactPane", () => ({
  AgentsArtifactPane: ({
    conversation,
    activeTab,
    onClose,
    onPublishWorkspace,
  }: {
    conversation: AgentConversation | null;
    activeTab?: string;
    onClose?: () => void;
    onPublishWorkspace?: (conversationId: string) => Promise<void>;
  }) => (
    <div data-testid="agents-artifact-pane" data-active-tab={activeTab ?? ""}>
      {onClose ? (
        <button type="button" data-testid="agents-artifact-pane-close" onClick={onClose}>
          Close
        </button>
      ) : null}
      {conversation && onPublishWorkspace ? (
        <button
          type="button"
          data-testid="agents-publish-confirm"
          onClick={() => void onPublishWorkspace(conversation.id)}
        >
          Publish
        </button>
      ) : null}
    </div>
  ),
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

const conversationWorkspace = (
  overrides: Partial<AgentConversationWorkspace> = {}
): AgentConversationWorkspace => ({
  conversationId: "conversation-1",
  projectId: "project-1",
  mode: "edit",
  baseRefKind: "project_default",
  baseRef: "main",
  baseDisplayName: "Project default (main)",
  baseCommit: null,
  branchName: "ralphx/ralphx/agent-abcdef12",
  worktreePath: "/tmp/ralphx/conversation-1",
  linkedIdeationSessionId: null,
  linkedPlanBranchId: null,
  publicationPrNumber: null,
  publicationPrUrl: null,
  publicationPrStatus: null,
  publicationPushStatus: null,
  status: "active",
  createdAt: "2026-04-23T09:00:00Z",
  updatedAt: "2026-04-23T09:00:00Z",
  ...overrides,
});

function renderWithProviders(ui: ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return {
    ...render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>{ui}</TooltipProvider>
    </QueryClientProvider>
    ),
    queryClient,
  };
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
    lastSelectedConversationByProjectId: {},
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
    preloadAgentTerminalExperienceMock.mockReset();
  });

  it("opts the title button out of the high-contrast default button border", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation()}
        workspace={null}
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
        conversation={conversation({ agentMode: "ideation" })}
        workspace={conversationWorkspace({ mode: "ideation" })}
        artifactOpen
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.queryByLabelText("Plan")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Close panel")).toBeInTheDocument();
  });

  it("does not render redundant runtime metadata in the title area", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation()}
        workspace={null}
        artifactOpen={false}
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.queryByText("Provider")).not.toBeInTheDocument();
    expect(screen.queryByText("Model")).not.toBeInTheDocument();
    expect(screen.queryByText("Mode")).not.toBeInTheDocument();
    expect(screen.queryByText("Default")).not.toBeInTheDocument();
  });

  it("shows the workspace branch status when available", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation()}
        workspace={{
          conversationId: "conversation-1",
          projectId: "project-1",
          mode: "edit",
          baseRefKind: "project_default",
          baseRef: "main",
          baseDisplayName: "Project default (main)",
          baseCommit: null,
          branchName: "ralphx/ralphx/agent-abcdef12",
          worktreePath: "/tmp/ralphx/conversation-1",
          linkedIdeationSessionId: null,
          linkedPlanBranchId: null,
          publicationPrNumber: null,
          publicationPrUrl: null,
          publicationPrStatus: null,
          publicationPushStatus: null,
          status: "active",
          createdAt: "2026-04-23T09:00:00Z",
          updatedAt: "2026-04-23T09:00:00Z",
        }}
        artifactOpen={false}
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.getByTestId("agents-workspace-status")).toHaveTextContent(
      "agent-abcdef12"
    );
    expect(screen.getByTestId("agents-workspace-status")).toHaveTextContent("active");
  });

  it("shows a commit and publish shortcut for editable workspaces", () => {
    const publish = vi.fn().mockResolvedValue(undefined);
    const openPublishPane = vi.fn();
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation({ id: "conversation-1" })}
        workspace={{
          conversationId: "conversation-1",
          projectId: "project-1",
          mode: "edit",
          baseRefKind: "project_default",
          baseRef: "main",
          baseDisplayName: "Project default (main)",
          baseCommit: null,
          branchName: "ralphx/ralphx/agent-abcdef12",
          worktreePath: "/tmp/ralphx/conversation-1",
          linkedIdeationSessionId: null,
          linkedPlanBranchId: null,
          publicationPrNumber: null,
          publicationPrUrl: null,
          publicationPrStatus: null,
          publicationPushStatus: null,
          status: "active",
          createdAt: "2026-04-23T09:00:00Z",
          updatedAt: "2026-04-23T09:00:00Z",
        }}
        artifactOpen={false}
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onPublishWorkspace={publish}
        onOpenPublishPane={openPublishPane}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    fireEvent.click(screen.getByTestId("agents-publish-workspace"));

    expect(openPublishPane).toHaveBeenCalledTimes(1);
    expect(publish).not.toHaveBeenCalled();
  });

  it("uses the publish action as a pane shortcut instead of immediately publishing", () => {
    const openPublishPane = vi.fn();
    const publish = vi.fn().mockResolvedValue(undefined);
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation({ id: "conversation-1" })}
        workspace={conversationWorkspace()}
        artifactOpen={false}
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onPublishWorkspace={publish}
        onOpenPublishPane={openPublishPane}
        onToggleTerminal={vi.fn()}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    fireEvent.click(screen.getByTestId("agents-publish-workspace"));

    expect(openPublishPane).toHaveBeenCalledTimes(1);
    expect(publish).not.toHaveBeenCalled();
  });

  it("labels the publish shortcut as a base update when the branch is stale", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation({ id: "conversation-1", agentMode: "edit" })}
        workspace={conversationWorkspace({
          mode: "edit",
          baseRef: "feature/agent-screen",
        })}
        artifactOpen={false}
        activeArtifactTab="plan"
        publishShortcutLabel="Update from feature/agent-screen"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onPublishWorkspace={vi.fn().mockResolvedValue(undefined)}
        onOpenPublishPane={vi.fn()}
        onToggleTerminal={vi.fn()}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.getByTestId("agents-publish-workspace")).toHaveTextContent(
      "Update from feature/agent-screen"
    );
  });

  it("hides the publish header shortcut while the publish pane is open", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation({ id: "conversation-1", agentMode: "edit" })}
        workspace={conversationWorkspace({ mode: "edit" })}
        artifactOpen
        activeArtifactTab="publish"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onPublishWorkspace={vi.fn().mockResolvedValue(undefined)}
        onOpenPublishPane={vi.fn()}
        onToggleTerminal={vi.fn()}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.queryByTestId("agents-publish-workspace")).not.toBeInTheDocument();
    expect(screen.queryByTestId("agents-workspace-status")).not.toBeInTheDocument();
  });

  it("hides ideation artifact shortcuts for edit-mode conversations", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation({ agentMode: "edit" })}
        workspace={conversationWorkspace({ mode: "edit" })}
        artifactOpen={false}
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleTerminal={vi.fn()}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.queryByLabelText("Plan")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Verification")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Proposals")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Tasks")).not.toBeInTheDocument();
  });

  it("shows ideation artifact shortcuts for ideation-mode conversations", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation({ agentMode: "ideation" })}
        workspace={conversationWorkspace({ mode: "ideation" })}
        artifactOpen={false}
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleTerminal={vi.fn()}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.getByLabelText("Plan")).toBeInTheDocument();
    expect(screen.getByLabelText("Verification")).toBeInTheDocument();
    expect(screen.getByLabelText("Proposals")).toBeInTheDocument();
    expect(screen.getByLabelText("Tasks")).toBeInTheDocument();
  });

  it("toggles the terminal from the header when a workspace is available", () => {
    const toggleTerminal = vi.fn();
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation()}
        workspace={conversationWorkspace()}
        artifactOpen={false}
        activeArtifactTab="plan"
        terminalOpen={false}
        terminalUnavailableReason={null}
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleTerminal={toggleTerminal}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    fireEvent.click(screen.getByTestId("agents-terminal-toggle"));

    expect(toggleTerminal).toHaveBeenCalledTimes(1);
  });

  it("preloads terminal code when the terminal header action receives intent", () => {
    const preloadTerminal = vi.fn();
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation()}
        workspace={conversationWorkspace()}
        artifactOpen={false}
        activeArtifactTab="plan"
        terminalOpen={false}
        terminalUnavailableReason={null}
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleTerminal={vi.fn()}
        onPreloadTerminal={preloadTerminal}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    const toggle = screen.getByTestId("agents-terminal-toggle");
    fireEvent.pointerEnter(toggle);
    fireEvent.focus(toggle);

    expect(preloadTerminal).toHaveBeenCalledTimes(2);
  });

  it("disables the terminal header action for branchless conversations", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation({ agentMode: "chat" })}
        workspace={null}
        artifactOpen={false}
        activeArtifactTab="plan"
        terminalOpen={false}
        terminalUnavailableReason="Terminal requires a workspace-backed conversation"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleTerminal={vi.fn()}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.getByTestId("agents-terminal-toggle")).toBeDisabled();
  });
});

describe("AgentsView", () => {
  beforeEach(() => {
    mockSidebarBreakpoint({ isLarge: true, isMedium: true });
    window.localStorage.clear();
    useProjectAgentConversationsMock.mockReset();
    useProjectsMock.mockReset();
    useConversationMock.mockReset();
    startAgentConversationMock.mockReset();
    getAgentConversationWorkspaceMock.mockReset();
    getAgentConversationWorkspaceFreshnessMock.mockReset();
    listAgentConversationWorkspacesByProjectMock.mockReset();
    listConversationsMock.mockReset();
    publishAgentConversationWorkspaceMock.mockReset();
    switchAgentConversationModeMock.mockReset();
    sendAgentMessageMock.mockReset();
    createConversationMock.mockReset();
    spawnConversationSessionNamerMock.mockReset();
    updateConversationTitleMock.mockReset();
    archiveConversationMock.mockReset();
    restoreConversationMock.mockReset();
    getPlanBranchesMock.mockReset();
    listIdeationSessionsMock.mockReset();
    getWorkspaceChangesMock.mockReset();
    getWorkspaceDiffMock.mockReset();
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    preloadAgentTerminalExperienceMock.mockReset();
    terminalDrawerMountMock.mockReset();
    terminalDrawerUnmountMock.mockReset();

    sendAgentMessageMock.mockResolvedValue({
      conversationId: "conversation-2",
      agentRunId: "run-2",
      isNewConversation: true,
      wasQueued: false,
      queuedAsPending: false,
      queuedMessageId: null,
    });
    getAgentConversationWorkspaceMock.mockResolvedValue(null);
    getAgentConversationWorkspaceFreshnessMock.mockResolvedValue({
      conversationId: "conversation-1",
      baseRef: "main",
      baseDisplayName: "Project default (main)",
      targetRef: "origin/main",
      capturedBaseCommit: "base-sha",
      targetBaseCommit: "base-sha",
      isBaseAhead: false,
    });
    listAgentConversationWorkspacesByProjectMock.mockResolvedValue([]);
    listConversationsMock.mockResolvedValue([]);
    getPlanBranchesMock.mockResolvedValue([]);
    listIdeationSessionsMock.mockResolvedValue([]);
    getWorkspaceChangesMock.mockResolvedValue([]);
    getWorkspaceDiffMock.mockResolvedValue("");
    publishAgentConversationWorkspaceMock.mockResolvedValue({
      workspace: {
        conversationId: "conversation-2",
        projectId: "project-1",
        mode: "edit",
        baseRefKind: "project_default",
        baseRef: "main",
        baseDisplayName: "Project default (main)",
        baseCommit: null,
        branchName: "ralphx/demo/agent-conversation-2",
        worktreePath: "/tmp/ralphx/conversation-2",
        linkedIdeationSessionId: null,
        linkedPlanBranchId: null,
        publicationPrNumber: 42,
        publicationPrUrl: "https://github.com/mock/project/pull/42",
        publicationPrStatus: "draft",
        publicationPushStatus: "pushed",
        status: "active",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
      commitSha: "mockcommit",
      pushed: true,
      createdPr: true,
      prNumber: 42,
      prUrl: "https://github.com/mock/project/pull/42",
    });
    switchAgentConversationModeMock.mockResolvedValue({
      conversation: conversation({
        id: "conversation-1",
        contextId: "project-1",
        agentMode: "edit",
      }),
      workspace: {
        conversationId: "conversation-1",
        projectId: "project-1",
        mode: "edit",
        baseRefKind: "project_default",
        baseRef: "main",
        baseDisplayName: "Project default (main)",
        baseCommit: null,
        branchName: "ralphx/demo/agent-conversation-1",
        worktreePath: "/tmp/ralphx/conversation-1",
        linkedIdeationSessionId: null,
        linkedPlanBranchId: null,
        publicationPrNumber: null,
        publicationPrUrl: null,
        publicationPrStatus: null,
        publicationPushStatus: null,
        status: "active",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
    });
    startAgentConversationMock.mockResolvedValue({
      conversation: conversation({ id: "conversation-2", contextId: "project-1" }),
      workspace: {
        conversationId: "conversation-2",
        projectId: "project-1",
        mode: "edit",
        baseRefKind: "project_default",
        baseRef: "main",
        baseDisplayName: "Project default (main)",
        baseCommit: null,
        branchName: "ralphx/demo/agent-conversation-2",
        worktreePath: "/tmp/ralphx/conversation-2",
        linkedIdeationSessionId: null,
        linkedPlanBranchId: null,
        publicationPrNumber: null,
        publicationPrUrl: null,
        publicationPrStatus: null,
        publicationPushStatus: null,
        status: "active",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
      sendResult: {
        conversationId: "conversation-2",
        agentRunId: "run-2",
        isNewConversation: true,
        wasQueued: false,
        queuedAsPending: false,
        queuedMessageId: null,
      },
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
    useAgentTerminalStore.setState({
      openByConversationId: {},
      heightByConversationId: {},
      activeTerminalByConversationId: {},
      placement: "auto",
    });
  });

  it("does not load the terminal drawer module until the terminal opens", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(
      conversationWorkspace({ mode: "edit" })
    );
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
    });

    renderAgentsView();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    expect(terminalDrawerModuleLoadedMock).not.toHaveBeenCalled();
  });

  it("defaults to the starter composer when no conversation is selected", async () => {
    mockAgentViewData();

    renderAgentsView();

    await waitFor(() =>
      expect(screen.getByTestId("agents-start-composer")).toBeInTheDocument()
    );
    expect(screen.getByTestId("agents-start-heading")).toHaveTextContent("Start your agent");
    expect(screen.getByTestId("agents-start-heading-word")).toHaveTextContent("agent");
    expect(screen.getByTestId("agents-start-project")).toBeInTheDocument();
    expect(screen.getByTestId("agents-start-base")).toBeInTheDocument();
    expect(screen.getByTestId("agents-start-provider")).toBeInTheDocument();
    expect(screen.getByTestId("agents-start-model")).toBeInTheDocument();
    expect(screen.queryByTestId("agents-start-new-project")).not.toBeInTheDocument();
    await userEvent.click(screen.getByTestId("agent-composer-actions-menu"));
    expect(screen.getByTestId("agents-start-mode-edit")).toBeInTheDocument();
    expect(screen.getByTestId("agents-start-new-project")).toBeInTheDocument();
    expect(screen.queryByTestId("integrated-chat-panel")).not.toBeInTheDocument();
  });

  it("restores a persisted selected conversation even when it is outside the first sidebar page", async () => {
    const restoredConversation = conversation({
      id: "conversation-restored",
      title: "Older restored agent",
      contextId: "project-1",
    });
    useProjectsMock.mockReturnValue({
      data: [project],
      isLoading: false,
    });
    useProjectAgentConversationsMock.mockReturnValue({
      data: [],
      conversations: [],
      isLoading: false,
      isSuccess: true,
      hasNextPage: true,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });
    useConversationMock.mockImplementation((conversationId: string | null) => ({
      data:
        conversationId === "conversation-restored"
          ? {
              conversation: restoredConversation,
              messages: [],
            }
          : null,
      isLoading: false,
    }));
    resetAgentSessionState({
      selectedProjectId: null,
      selectedConversationId: null,
      lastSelectedConversationByProjectId: {
        "project-1": "conversation-restored",
      },
    });

    renderAgentsView();

    expect(await screen.findByTestId("integrated-chat-panel")).toBeInTheDocument();
    expect(screen.getByTestId("agents-session-conversation-restored")).toHaveTextContent(
      "Older restored agent"
    );
  });

  it("starts a new conversation directly from the starter composer and triggers the session namer", async () => {
    const invalidateSpy = vi.spyOn(QueryClient.prototype, "invalidateQueries");
    mockAgentViewData();

    const { queryClient } = renderAgentsView();

    fireEvent.change(screen.getByTestId("agents-start-textarea"), {
      target: { value: "fix agent landing flow" },
    });
    fireEvent.click(screen.getByTestId("agents-start-submit"));

    await waitFor(() =>
      expect(startAgentConversationMock).toHaveBeenCalledWith(
        expect.objectContaining({
          projectId: "project-1",
          content: "fix agent landing flow",
          providerHarness: "codex",
          modelId: "gpt-5.4",
          mode: "edit",
          base: expect.objectContaining({
            kind: "project_default",
            ref: "main",
          }),
        })
      )
    );
    await waitFor(() =>
      expect(spawnConversationSessionNamerMock).toHaveBeenCalledWith(
        "conversation-2",
        "fix agent landing flow"
      )
    );
    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    expect(screen.queryByTestId("agents-start-composer")).not.toBeInTheDocument();
    expect(screen.getByTestId("agents-workspace-status")).toHaveTextContent(
      "agent-conversation-2"
    );
    expect(useAgentSessionStore.getState().selectedConversationId).toBe("conversation-2");
    expect(queryClient.getQueryData(["chat", "conversations", "conversation-2"])).toEqual({
      conversation: expect.objectContaining({ id: "conversation-2" }),
      messages: [],
    });
    expect(
      queryClient.getQueryData(["agents", "conversation-workspace", "conversation-2"])
    ).toEqual(expect.objectContaining({ conversationId: "conversation-2" }));
    expect(invalidateSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        queryKey: ["agents", "project-conversations", "project-1"],
      })
    );
    invalidateSpy.mockRestore();
  });

  it("starts a chat-mode conversation from the selected base and shows its workspace", async () => {
    mockAgentViewData();
    startAgentConversationMock.mockResolvedValue({
      conversation: conversation({
        id: "conversation-chat",
        contextId: "project-1",
        title: "Branch question",
        agentMode: "chat",
      }),
      workspace: {
        conversationId: "conversation-chat",
        projectId: "project-1",
        mode: "chat",
        baseRefKind: "project_default",
        baseRef: "main",
        baseDisplayName: "Project default (main)",
        baseCommit: null,
        branchName: "ralphx/demo/agent-conversation-chat",
        worktreePath: "/tmp/ralphx/conversation-chat",
        linkedIdeationSessionId: null,
        linkedPlanBranchId: null,
        publicationPrNumber: null,
        publicationPrUrl: null,
        publicationPrStatus: null,
        publicationPushStatus: null,
        status: "active",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
      sendResult: {
        conversationId: "conversation-chat",
        agentRunId: "run-chat",
        isNewConversation: true,
        wasQueued: false,
        queuedAsPending: false,
        queuedMessageId: null,
      },
    });

    renderAgentsView();

    await userEvent.click(screen.getByTestId("agent-composer-actions-menu"));
    await userEvent.click(screen.getByTestId("agents-start-mode-chat"));
    expect(screen.getByTestId("agents-start-base")).toBeInTheDocument();

    fireEvent.change(screen.getByTestId("agents-start-textarea"), {
      target: { value: "what branch am I on?" },
    });
    fireEvent.click(screen.getByTestId("agents-start-submit"));

    await waitFor(() =>
      expect(startAgentConversationMock).toHaveBeenCalledWith(
        expect.objectContaining({
          projectId: "project-1",
          content: "what branch am I on?",
          mode: "chat",
          base: expect.objectContaining({
            kind: "project_default",
            ref: "main",
          }),
        })
      )
    );
    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    expect(screen.getByTestId("agents-workspace-status")).toHaveTextContent(
      "agent-conversation-chat"
    );
  });

  it("archives the selected conversation, clears the active view, and refreshes archived counts", async () => {
    const user = userEvent.setup();
    const invalidateSpy = vi.spyOn(QueryClient.prototype, "invalidateQueries");
    mockAgentViewData();
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
    });

    renderAgentsView();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );

    await user.click(screen.getByRole("button", { name: "Session actions" }));
    await user.click(await screen.findByText("Archive session"));
    await user.click(screen.getByRole("button", { name: "Archive session" }));

    await waitFor(() =>
      expect(archiveConversationMock).toHaveBeenCalledWith("conversation-1")
    );
    await waitFor(() =>
      expect(screen.getByTestId("agents-start-composer")).toBeInTheDocument()
    );
    expect(screen.queryByTestId("integrated-chat-panel")).not.toBeInTheDocument();
    expect(invalidateSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        queryKey: ["agents", "project-conversations", "project-1", "archived-count"],
        refetchType: "active",
      })
    );

    invalidateSpy.mockRestore();
  });

  it("uploads starter attachments against a seeded conversation before sending the first message", async () => {
    mockAgentViewData();
    startAgentConversationMock.mockResolvedValue({
      conversation: conversation({ id: "conversation-seeded", contextId: "project-1" }),
      workspace: {
        conversationId: "conversation-seeded",
        projectId: "project-1",
        mode: "edit",
        baseRefKind: "project_default",
        baseRef: "main",
        baseDisplayName: "Project default (main)",
        baseCommit: null,
        branchName: "ralphx/demo/agent-conversation-seeded",
        worktreePath: "/tmp/ralphx/conversation-seeded",
        linkedIdeationSessionId: null,
        linkedPlanBranchId: null,
        publicationPrNumber: null,
        publicationPrUrl: null,
        publicationPrStatus: null,
        publicationPushStatus: null,
        status: "active",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
      sendResult: {
        conversationId: "conversation-seeded",
        agentRunId: "run-2",
        isNewConversation: false,
        wasQueued: false,
        queuedAsPending: false,
        queuedMessageId: null,
      },
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
      expect(startAgentConversationMock).toHaveBeenCalledWith(
        expect.objectContaining({
          projectId: "project-1",
          content: "review this note",
          conversationId: "conversation-seeded",
          providerHarness: "codex",
          modelId: "gpt-5.4",
          mode: "edit",
        })
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
      selectedConversationId: "conversation-1",
      artifactByConversationId: {
        "conversation-1": {
          isOpen: true,
          activeTab: "plan",
          taskMode: "graph",
        },
      },
    });

    renderAgentsView();

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
    const rectSpy = vi.spyOn(splitContainer, "getBoundingClientRect").mockReturnValue({
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
    fireEvent.mouseMove(document, { clientX: 940 });
    fireEvent.mouseMove(document, { clientX: 920 });
    fireEvent.mouseMove(document, { clientX: 900 });
    fireEvent.mouseUp(document);

    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-resizable-pane")).toHaveStyle({
        width: "400px",
      })
    );
    expect(window.localStorage.getItem("ralphx-agents-artifact-width")).toBe("400");
    expect(rectSpy).toHaveBeenCalledTimes(1);
  });

  it("docks the terminal under the artifact panel in auto mode when the panel is open", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(conversationWorkspace({ mode: "edit" }));
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
    });
    useAgentTerminalStore.setState({
      openByConversationId: { "conversation-1": true },
      heightByConversationId: {},
      activeTerminalByConversationId: {},
      placement: "auto",
    });

    renderAgentsView();
    fireEvent.click(await screen.findByTestId("agents-publish-workspace"));

    const drawer = await screen.findByTestId("agent-terminal-drawer");
    expect(drawer).toHaveAttribute("data-placement", "auto");
    expect(screen.getByTestId("agents-artifact-resizable-pane")).toContainElement(drawer);
  });

  it("persists terminal dock placement from the terminal control", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(conversationWorkspace({ mode: "edit" }));
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
    });
    useAgentTerminalStore.setState({
      openByConversationId: { "conversation-1": true },
      heightByConversationId: {},
      activeTerminalByConversationId: {},
      placement: "panel",
    });

    renderAgentsView();
    fireEvent.click(await screen.findByTestId("agents-publish-workspace"));
    fireEvent.click(await screen.findByTestId("agent-terminal-placement"));

    expect(useAgentTerminalStore.getState().placement).toBe("chat");
  });

  it("opens the publish pane when terminal placement changes to panel", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(conversationWorkspace({ mode: "edit" }));
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
    });
    useAgentTerminalStore.setState({
      openByConversationId: { "conversation-1": true },
      heightByConversationId: {},
      activeTerminalByConversationId: {},
      placement: "auto",
    });

    renderAgentsView();
    expect(screen.queryByTestId("agents-artifact-resizable-pane")).not.toBeInTheDocument();

    fireEvent.click(await screen.findByTestId("agent-terminal-placement"));

    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-resizable-pane")).toBeInTheDocument()
    );
    expect(useAgentTerminalStore.getState().placement).toBe("panel");
    expect(screen.getByTestId("agents-artifact-resizable-pane")).toContainElement(
      screen.getByTestId("agent-terminal-drawer")
    );
  });

  it("keeps the terminal drawer mounted while moving it between chat and panel docks", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(conversationWorkspace({ mode: "edit" }));
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
    });
    useAgentTerminalStore.setState({
      openByConversationId: { "conversation-1": true },
      heightByConversationId: {},
      activeTerminalByConversationId: {},
      placement: "auto",
    });

    renderAgentsView();

    const drawer = await screen.findByTestId("agent-terminal-drawer");
    await waitFor(() => expect(terminalDrawerMountMock).toHaveBeenCalledTimes(1));
    expect(screen.getByTestId("agent-terminal-host-chat")).toContainElement(drawer);

    fireEvent.click(screen.getByTestId("agent-terminal-placement"));

    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-resizable-pane")).toBeInTheDocument()
    );
    await waitFor(() =>
      expect(screen.getByTestId("agent-terminal-host-panel")).toContainElement(
        screen.getByTestId("agent-terminal-drawer")
      )
    );
    expect(terminalDrawerMountMock).toHaveBeenCalledTimes(1);
    expect(terminalDrawerUnmountMock).not.toHaveBeenCalled();
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

  it("shows the conversation base branch as a read-only start-from line", async () => {
    mockAgentViewData();
    getAgentConversationWorkspaceMock.mockResolvedValue({
      conversationId: "conversation-1",
      projectId: "project-1",
      mode: "edit",
      baseRefKind: "project_default",
      baseRef: "main",
      baseDisplayName: "Project default (main)",
      baseCommit: null,
      branchName: "ralphx/demo/agent-conversation-1",
      worktreePath: "/tmp/ralphx/conversation-1",
      linkedIdeationSessionId: null,
      linkedPlanBranchId: null,
      publicationPrNumber: null,
      publicationPrUrl: null,
      publicationPrStatus: null,
      publicationPushStatus: null,
      status: "active",
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    });

    renderAgentsView();
    selectSidebarConversationRow();

    const baseLine = await screen.findByTestId("agents-conversation-base");
    expect(baseLine).toHaveTextContent("Project default (main)");
    expect(within(baseLine).getByRole("button", { name: "Start from" })).toBeDisabled();
  });

  it("opens the right-side publish pane from the Commit & Publish header shortcut", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(conversationWorkspace({ mode: "edit" }));

    renderAgentsView();
    selectSidebarConversationRow();

    await screen.findByTestId("agents-publish-workspace");
    expect(screen.queryByTestId("agents-artifact-pane")).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId("agents-publish-workspace"));

    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-pane")).toBeInTheDocument()
    );
    expect(publishAgentConversationWorkspaceMock).not.toHaveBeenCalled();
  });

  it("does not hydrate attached ideation session data for edit conversations", async () => {
    const agentConversation = conversation({ agentMode: "edit" });
    mockAgentViewData(agentConversation);
    useConversationMock.mockImplementation((conversationId: string | null) => ({
      data:
        conversationId === agentConversation.id
          ? {
              conversation: agentConversation,
              messages: [
                {
                  id: "message-1",
                  conversationId: agentConversation.id,
                  role: "assistant",
                  content: "",
                  toolCalls: [
                    {
                      id: "tool-1",
                      name: "v1_start_ideation",
                      arguments: {},
                      result: { session_id: "session-1" },
                    },
                  ],
                  contentBlocks: [],
                  createdAt: "2026-04-23T09:00:00Z",
                },
              ],
            }
          : null,
      isLoading: false,
    }));
    getAgentConversationWorkspaceMock.mockResolvedValue(
      conversationWorkspace({ mode: "edit" })
    );

    renderAgentsView();
    selectSidebarConversationRow();

    await waitFor(() =>
      expect(getAgentConversationWorkspaceMock).toHaveBeenCalledWith("conversation-1")
    );
    expect(vi.mocked(ideationApi.sessions.getWithData)).not.toHaveBeenCalled();
  });

  it("shows Update from base in the header shortcut when the workspace base moved", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(
      conversationWorkspace({
        mode: "edit",
        baseRef: "feature/agent-screen",
        baseDisplayName: "Current branch (feature/agent-screen)",
      })
    );
    getAgentConversationWorkspaceFreshnessMock.mockResolvedValue({
      conversationId: "conversation-1",
      baseRef: "feature/agent-screen",
      baseDisplayName: "Current branch (feature/agent-screen)",
      targetRef: "origin/feature/agent-screen",
      capturedBaseCommit: "old-base",
      targetBaseCommit: "new-base",
      isBaseAhead: true,
    });

    renderAgentsView();
    selectSidebarConversationRow();

    await waitFor(() =>
      expect(screen.getByTestId("agents-publish-workspace")).toHaveTextContent(
        "Update from feature/agent-screen"
      )
    );
  });

  it("relies on the backend to route fixable publish failures into the workspace agent conversation", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock
      .mockResolvedValueOnce(conversationWorkspace({ mode: "edit" }))
      .mockResolvedValueOnce(
        conversationWorkspace({ mode: "edit", publicationPushStatus: "needs_agent" })
      );
    publishAgentConversationWorkspaceMock.mockRejectedValue(
      "Failed to commit: typecheck failed"
    );
    renderAgentsView();
    selectSidebarConversationRow();

    await screen.findByTestId("agents-publish-workspace");
    fireEvent.click(screen.getByTestId("agents-publish-workspace"));

    await screen.findByTestId("agents-publish-confirm");
    fireEvent.click(screen.getByTestId("agents-publish-confirm"));

    await waitFor(() => expect(getAgentConversationWorkspaceMock).toHaveBeenCalledTimes(2));
    expect(sendAgentMessageMock).not.toHaveBeenCalled();
    expect(toastErrorMock).toHaveBeenCalledWith(
      "Publish failed. Sent the error to the agent to fix."
    );
  });

  it("does not send operational publish failures to the workspace agent", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock
      .mockResolvedValueOnce(conversationWorkspace({ mode: "edit" }))
      .mockResolvedValueOnce(
        conversationWorkspace({ mode: "edit", publicationPushStatus: "failed" })
      );
    publishAgentConversationWorkspaceMock.mockRejectedValue(
      "GitHub integration is not available"
    );
    renderAgentsView();
    selectSidebarConversationRow();

    await screen.findByTestId("agents-publish-workspace");
    fireEvent.click(screen.getByTestId("agents-publish-workspace"));

    await screen.findByTestId("agents-publish-confirm");
    fireEvent.click(screen.getByTestId("agents-publish-confirm"));

    await waitFor(() =>
      expect(toastErrorMock).toHaveBeenCalledWith(
        "GitHub integration is not available"
      )
    );
    expect(sendAgentMessageMock).not.toHaveBeenCalled();
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
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
    });

    renderAgentsView();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    expect(screen.getByTestId("integrated-chat-panel")).toHaveAttribute(
      "data-content-width-class",
      "max-w-[980px]"
    );
    expect(screen.queryByTestId("agents-artifact-pane")).not.toBeInTheDocument();
  });

  it("restores a persisted artifact pane and active tab on conversation load", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(conversationWorkspace({ mode: "edit" }));
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
      artifactByConversationId: {
        "conversation-1": {
          isOpen: true,
          activeTab: "publish",
          taskMode: "graph",
        },
      },
    });

    renderAgentsView();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    const pane = await screen.findByTestId("agents-artifact-pane");
    expect(pane).toHaveAttribute("data-active-tab", "publish");
    expect(screen.getByTestId("agents-artifact-resizable-pane")).toBeInTheDocument();
  });

  it("still allows manually opening the artifact pane when the conversation has nothing to show", async () => {
    mockAgentViewData(
      conversation({
        contextType: "ideation",
        contextId: "session-1",
        ideationSessionId: "session-1",
        agentMode: "ideation",
      })
    );
    mockSessionWithData();

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

  it("opens the artifact pane before persisting the panel state", async () => {
    mockAgentViewData(
      conversation({
        contextType: "ideation",
        contextId: "session-1",
        ideationSessionId: "session-1",
        agentMode: "ideation",
      })
    );
    mockSessionWithData();
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
    });

    renderAgentsView();

    await waitFor(() =>
      expect(screen.getByTestId("integrated-chat-panel")).toBeInTheDocument()
    );
    const setItemSpy = vi.spyOn(Storage.prototype, "setItem");
    setItemSpy.mockClear();

    fireEvent.click(screen.getByLabelText("Open artifacts"));

    expect(screen.getByTestId("agents-artifact-resizable-pane")).toBeInTheDocument();
    expect(setItemSpy).not.toHaveBeenCalled();

    setItemSpy.mockRestore();
  });

  it("closes the artifact pane before persisting the panel state", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(conversationWorkspace({ mode: "edit" }));
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
      artifactByConversationId: {
        "conversation-1": {
          isOpen: true,
          activeTab: "publish",
          taskMode: "graph",
        },
      },
    });

    renderAgentsView();

    await screen.findByTestId("agents-artifact-pane");
    const setItemSpy = vi.spyOn(Storage.prototype, "setItem");
    setItemSpy.mockClear();

    fireEvent.click(screen.getByLabelText("Close panel"));

    expect(screen.queryByTestId("agents-artifact-resizable-pane")).not.toBeInTheDocument();
    expect(setItemSpy).not.toHaveBeenCalled();

    setItemSpy.mockRestore();
  });

  it("closes the artifact pane from the pane close action before persisting state", async () => {
    mockAgentViewData(conversation({ agentMode: "edit" }));
    getAgentConversationWorkspaceMock.mockResolvedValue(conversationWorkspace({ mode: "edit" }));
    resetAgentSessionState({
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
      artifactByConversationId: {
        "conversation-1": {
          isOpen: true,
          activeTab: "publish",
          taskMode: "graph",
        },
      },
    });

    renderAgentsView();

    await screen.findByTestId("agents-artifact-pane");
    const setItemSpy = vi.spyOn(Storage.prototype, "setItem");
    setItemSpy.mockClear();

    fireEvent.click(screen.getByTestId("agents-artifact-pane-close"));

    expect(screen.queryByTestId("agents-artifact-resizable-pane")).not.toBeInTheDocument();
    expect(setItemSpy).not.toHaveBeenCalled();

    setItemSpy.mockRestore();
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
