import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { useAgentSessionStore, type AgentRuntimeSelection } from "@/stores/agentSessionStore";
import type { AgentConversation } from "./agentConversations";
import { AgentsChatHeader, AgentsView } from "./AgentsView";

const {
  useProjectsMock,
  useProjectAgentConversationsMock,
  useConversationMock,
} = vi.hoisted(() => ({
  useProjectsMock: vi.fn(),
  useProjectAgentConversationsMock: vi.fn(),
  useConversationMock: vi.fn(),
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
  useConversation: (conversationId: string | null) => useConversationMock(conversationId),
}));

vi.mock("@/components/Chat/IntegratedChatPanel", () => ({
  IntegratedChatPanel: ({ headerContent }: { headerContent?: ReactNode }) => (
    <div data-testid="integrated-chat-panel">{headerContent}</div>
  ),
}));

vi.mock("./AgentsArtifactPane", () => ({
  AgentsArtifactPane: () => <div data-testid="agents-artifact-pane" />,
}));

vi.mock("./NewAgentDialog", () => ({
  NewAgentDialog: () => null,
}));

vi.mock("./useProjectAgentBridgeEvents", () => ({
  useProjectAgentBridgeEvents: () => undefined,
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
  useConversationMock.mockReturnValue({
    data: {
      conversation: agentConversation,
      messages: [],
    },
    isLoading: false,
  });
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
    window.localStorage.clear();
    useProjectAgentConversationsMock.mockReset();
    useProjectsMock.mockReset();
    useConversationMock.mockReset();
    useAgentSessionStore.setState({
      focusedProjectId: "project-1",
      selectedProjectId: "project-1",
      selectedConversationId: "conversation-1",
      expandedProjectIds: { "project-1": true },
      artifactByConversationId: {
        "conversation-1": {
          isOpen: true,
          activeTab: "plan",
          taskMode: "graph",
        },
      },
      runtimeByConversationId: {
        "conversation-1": runtime,
      },
      lastRuntimeByProjectId: {
        "project-1": runtime,
      },
    });
  });

  it("restores persisted artifact width, enforces 320px mins, and resets to default on double click", async () => {
    window.localStorage.setItem("ralphx-agents-artifact-width", "480");
    mockAgentViewData();

    renderWithProviders(
      <AgentsView
        projectId="project-1"
        isNewAgentDialogOpen={false}
        onNewAgentDialogOpenChange={vi.fn()}
        onCreateProject={vi.fn()}
      />
    );

    const pane = screen.getByTestId("agents-artifact-resizable-pane");
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

  it("deselects the selected agent when its row is clicked again", async () => {
    mockAgentViewData();

    renderWithProviders(
      <AgentsView
        projectId="project-1"
        isNewAgentDialogOpen={false}
        onNewAgentDialogOpenChange={vi.fn()}
        onCreateProject={vi.fn()}
      />
    );

    const row = screen.getByTestId("agents-session-conversation-1");
    fireEvent.click(within(row).getAllByRole("button")[0]);

    await waitFor(() =>
      expect(
        screen.getByText("Pick a conversation from the sidebar")
      ).toBeInTheDocument()
    );
  });
});
