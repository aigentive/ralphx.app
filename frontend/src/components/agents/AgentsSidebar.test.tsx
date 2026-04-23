import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import { useChatStore } from "@/stores/chatStore";
import { useAgentSessionStore } from "@/stores/agentSessionStore";
import type { Project } from "@/types/project";
import type { AgentConversation } from "./agentConversations";
import { formatAgentConversationCreatedAt } from "./agentConversations";
import { AgentsSidebar } from "./AgentsSidebar";

type ConversationsResult = {
  data: AgentConversation[];
  isLoading: boolean;
  hasNextPage?: boolean;
  isFetchingNextPage?: boolean;
  fetchNextPage?: () => Promise<unknown>;
};

const { conversationsByProject } = vi.hoisted(() => ({
  conversationsByProject: new Map<string, ConversationsResult>(),
}));

vi.mock("./useProjectAgentConversations", () => ({
  useProjectAgentConversations: (projectId: string | null | undefined) =>
    conversationsByProject.get(projectId ?? "") ?? {
      data: [],
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    },
}));

const project = (overrides: Partial<Project> = {}): Project => ({
  id: "project-1",
  name: "ralphx",
  workingDirectory: "/tmp/ralphx",
  gitMode: "worktree",
  baseBranch: null,
  worktreeParentDirectory: null,
  useFeatureBranches: true,
  mergeValidationMode: "block",
  detectedAnalysis: null,
  customAnalysis: null,
  analyzedAt: null,
  githubPrEnabled: false,
  createdAt: "2026-04-22T09:00:00Z",
  updatedAt: "2026-04-22T09:00:00Z",
  ...overrides,
});

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
  title: "Fix font scaling",
  messageCount: 1,
  lastMessageAt: "2026-04-22T12:00:00Z",
  createdAt: "2026-04-22T10:00:00Z",
  updatedAt: "2026-04-22T12:00:00Z",
  archivedAt: null,
  projectId: "project-1",
  ideationSessionId: null,
  ...overrides,
});

function renderSidebar() {
  return render(
    <TooltipProvider>
      <AgentsSidebar
        projects={[project()]}
        focusedProjectId="project-1"
        selectedConversationId={null}
        onFocusProject={vi.fn()}
        onSelectConversation={vi.fn()}
        onCreateAgent={vi.fn()}
        onCreateProject={vi.fn()}
        onQuickCreateAgent={vi.fn()}
        onRemoveProject={vi.fn()}
        onArchiveConversation={vi.fn()}
        onRestoreConversation={vi.fn()}
        isCreatingAgent={false}
        showArchived={false}
        onShowArchivedChange={vi.fn()}
      />
    </TooltipProvider>
  );
}

describe("AgentsSidebar", () => {
  beforeEach(() => {
    conversationsByProject.clear();
    useChatStore.setState({ activeConversationIds: {}, agentStatus: {} });
    useAgentSessionStore.setState({ expandedProjectIds: { "project-1": true } });
  });

  it("orders sessions by created time and shows created time instead of provider", () => {
    const older = conversation({
      id: "older",
      title: "Older agent",
      createdAt: "2026-04-22T10:00:00Z",
      lastMessageAt: "2026-04-22T12:00:00Z",
    });
    const newer = conversation({
      id: "newer",
      title: "Newer agent",
      createdAt: "2026-04-22T11:00:00Z",
      lastMessageAt: "2026-04-22T11:01:00Z",
    });
    conversationsByProject.set("project-1", {
      data: [newer, older],
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar();

    const rows = screen.getAllByTestId(/agents-session-/);
    expect(rows.map((row) => row.getAttribute("data-testid"))).toEqual([
      "agents-session-newer",
      "agents-session-older",
    ]);

    const firstRow = within(rows[0]);
    expect(firstRow.getByText("Newer agent")).toBeInTheDocument();
    expect(
      firstRow.getByText(formatAgentConversationCreatedAt(newer.createdAt))
    ).toBeInTheDocument();
    expect(firstRow.queryByText("codex")).not.toBeInTheDocument();
  });

  it("shows load more per project and calls the paginated fetch when pressed", () => {
    const fetchNextPage = vi.fn().mockResolvedValue(undefined);
    conversationsByProject.set("project-1", {
      data: [conversation()],
      isLoading: false,
      hasNextPage: true,
      isFetchingNextPage: false,
      fetchNextPage,
    });

    renderSidebar();

    fireEvent.click(screen.getByTestId("agents-load-more-project-1"));
    expect(fetchNextPage).toHaveBeenCalledTimes(1);
  });
});
