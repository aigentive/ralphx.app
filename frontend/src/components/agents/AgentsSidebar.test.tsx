import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ComponentProps } from "react";
import userEvent from "@testing-library/user-event";

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
  total?: number;
  hasNextPage?: boolean;
  isFetchingNextPage?: boolean;
  fetchNextPage?: () => Promise<unknown>;
};

const { conversationsByProject } = vi.hoisted(() => ({
  conversationsByProject: new Map<string, ConversationsResult>(),
}));
const { projectConversationCalls } = vi.hoisted(() => ({
  projectConversationCalls: [] as Array<{
    projectId: string | null;
    includeArchived: boolean;
    options?: { search?: string; enabled?: boolean };
  }>,
}));
const { archivedConversationCounts, archivedCountCalls } = vi.hoisted(() => ({
  archivedConversationCounts: new Map<string, number>(),
  archivedCountCalls: [] as string[][],
}));

vi.mock("./useProjectAgentConversations", () => ({
  useProjectAgentConversations: (
    projectId: string | null | undefined,
    includeArchived = false,
    options?: { search?: string; enabled?: boolean }
  ) =>
    (() => {
      projectConversationCalls.push({
        projectId: projectId ?? null,
        includeArchived,
        options,
      });
      const result = conversationsByProject.get(projectId ?? "");
      if (result) {
        return {
          ...result,
          total: result.total ?? result.data.length,
        };
      }
      return {
        data: [],
        isLoading: false,
        total: 0,
        hasNextPage: false,
        isFetchingNextPage: false,
        fetchNextPage: vi.fn(),
      };
    })(),
}));

vi.mock("./useArchivedConversationCounts", () => ({
  useArchivedConversationCounts: (projectIds: string[]) => {
    archivedCountCalls.push(projectIds);
    const byProjectId = Object.fromEntries(
      projectIds.map((projectId) => [projectId, archivedConversationCounts.get(projectId) ?? 0])
    );
    const totalArchivedCount = Object.values(byProjectId).reduce(
      (sum, count) => sum + count,
      0
    );

    return {
      byProjectId,
      totalArchivedCount,
      isLoading: false,
    };
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

function renderSidebar(
  projects: Project[] = [project()],
  props?: Partial<ComponentProps<typeof AgentsSidebar>>
) {
  return render(
    <TooltipProvider delayDuration={0}>
      <AgentsSidebar
        projects={projects}
        focusedProjectId="project-1"
        selectedConversationId={null}
        onFocusProject={vi.fn()}
        onSelectConversation={vi.fn()}
        onCreateAgent={vi.fn()}
        onCreateProject={vi.fn()}
        onArchiveProject={vi.fn()}
        onRenameConversation={vi.fn()}
        onArchiveConversation={vi.fn()}
        onRestoreConversation={vi.fn()}
        showArchived={false}
        onShowArchivedChange={vi.fn()}
        {...props}
      />
    </TooltipProvider>
  );
}

function getProjectRowOrder() {
  return screen
    .getAllByTestId((testId) => testId.startsWith("agents-project-project-"))
    .map((row) => row.getAttribute("data-testid"));
}

describe("AgentsSidebar", () => {
  beforeEach(() => {
    conversationsByProject.clear();
    projectConversationCalls.length = 0;
    archivedConversationCounts.clear();
    archivedCountCalls.length = 0;
    useChatStore.setState({ activeConversationIds: {}, agentStatus: {} });
    useAgentSessionStore.setState({
      expandedProjectIds: { "project-1": true, "project-2": true },
      showAllProjects: false,
      projectSort: "latest",
    });
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

  it("shows the backend total session count rather than the loaded page size", () => {
    conversationsByProject.set("project-1", {
      data: [conversation({ id: "conversation-1" }), conversation({ id: "conversation-2" })],
      total: 11,
      isLoading: false,
      hasNextPage: true,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar();

    expect(screen.getByText("11")).toBeInTheDocument();
  });

  it("shows an archived pill with the total archived count when archived sessions exist", () => {
    const onShowArchivedChange = vi.fn();
    archivedConversationCounts.set("project-1", 4);
    conversationsByProject.set("project-1", {
      data: [conversation()],
      total: 6,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([project()], { onShowArchivedChange });

    fireEvent.click(screen.getByTestId("agents-show-archived-pill"));

    expect(onShowArchivedChange).toHaveBeenCalledWith(true);
    expect(screen.getByText("4")).toBeInTheDocument();
  });

  it("hides the archived pill when there are no archived sessions", () => {
    conversationsByProject.set("project-1", {
      data: [conversation()],
      total: 6,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar();

    expect(screen.queryByTestId("agents-show-archived-pill")).not.toBeInTheDocument();
  });

  it("hides empty projects by default", () => {
    conversationsByProject.set("project-1", {
      data: [],
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar();

    expect(screen.queryByTestId("agents-project-project-1")).not.toBeInTheDocument();
    expect(screen.queryByText("No chats yet.")).not.toBeInTheDocument();
    expect(screen.queryByText("Start")).not.toBeInTheDocument();
  });

  it("only enables sidebar conversation and archived-count queries for the focused project by default", () => {
    const focused = project({ id: "project-1", name: "alpha" });
    const idle = project({ id: "project-2", name: "beta" });
    const anotherIdle = project({ id: "project-3", name: "gamma" });
    conversationsByProject.set("project-1", {
      data: [conversation({ id: "conversation-1" })],
      total: 1,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });
    conversationsByProject.set("project-2", {
      data: [conversation({ id: "conversation-2", projectId: "project-2", contextId: "project-2" })],
      total: 1,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([focused, idle, anotherIdle]);

    expect(projectConversationCalls).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          projectId: "project-1",
          options: expect.objectContaining({ enabled: true }),
        }),
        expect.objectContaining({
          projectId: "project-2",
          options: expect.objectContaining({ enabled: false }),
        }),
        expect.objectContaining({
          projectId: "project-3",
          options: expect.objectContaining({ enabled: false }),
        }),
      ])
    );
    expect(archivedCountCalls.at(-1)).toEqual(["project-1"]);
  });

  it("searches conversations on the backend across projects without matching project names", async () => {
    const focused = project({ id: "project-1", name: "alpha" });
    const idle = project({ id: "project-2", name: "beta" });
    conversationsByProject.set("project-1", {
      data: [],
      total: 0,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });
    conversationsByProject.set("project-2", {
      data: [
        conversation({
          id: "conversation-search",
          title: "Fix sidebar search",
          projectId: "project-2",
          contextId: "project-2",
        }),
      ],
      total: 1,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([focused, idle]);

    fireEvent.click(screen.getByTestId("agents-search-toggle"));
    fireEvent.change(screen.getByTestId("agents-search-input"), {
      target: { value: "sidebar" },
    });

    await waitFor(() =>
      expect(projectConversationCalls).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            projectId: "project-2",
            options: expect.objectContaining({
              enabled: true,
              search: "sidebar",
            }),
          }),
        ])
      )
    );
    expect(screen.getByTestId("agents-session-conversation-search")).toHaveTextContent(
      "Fix sidebar search"
    );
  });

  it("can reveal empty projects from the filter pill", () => {
    conversationsByProject.set("project-1", {
      data: [],
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar();

    fireEvent.click(screen.getByTestId("agents-show-all-projects-pill"));

    expect(screen.getByTestId("agents-project-project-1")).toBeInTheDocument();
    expect(screen.queryByText("No chats yet.")).not.toBeInTheDocument();
    expect(screen.queryByText("Start")).not.toBeInTheDocument();
  });

  it("shows an add project footer action instead of the archived switch", () => {
    const onCreateProject = vi.fn();
    conversationsByProject.set("project-1", {
      data: [conversation()],
      total: 6,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([project()], { onCreateProject });

    fireEvent.click(screen.getByTestId("agents-add-project"));

    expect(onCreateProject).toHaveBeenCalledTimes(1);
    expect(screen.queryByLabelText("Show archived sessions")).not.toBeInTheDocument();
  });

  it("supports alphabetical sorting from the sort pill", () => {
    const alpha = project({ id: "project-1", name: "alpha" });
    const beta = project({ id: "project-2", name: "beta" });
    useAgentSessionStore.setState({ showAllProjects: true });

    conversationsByProject.set("project-1", {
      data: [conversation({ id: "conversation-1", projectId: "project-1", contextId: "project-1" })],
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });
    conversationsByProject.set("project-2", {
      data: [conversation({ id: "conversation-2", projectId: "project-2", contextId: "project-2" })],
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([beta, alpha]);

    expect(getProjectRowOrder()).toEqual([
      "agents-project-project-2",
      "agents-project-project-1",
    ]);

    fireEvent.pointerDown(screen.getByTestId("agents-project-sort-pill"));
    fireEvent.click(screen.getByText("A-Z"));

    expect(getProjectRowOrder()).toEqual([
      "agents-project-project-1",
      "agents-project-project-2",
    ]);
  });

  it("keeps project actions visible while open and confirms before archiving", () => {
    const onArchiveProject = vi.fn();
    conversationsByProject.set("project-1", {
      data: [conversation()],
      total: 6,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([project()], { onArchiveProject });

    const actions = screen.getByTestId("agents-project-actions-project-1");
    const trigger = within(actions).getByRole("button", { name: "Project actions" });

    fireEvent.pointerDown(trigger);

    expect(actions.className).toContain("opacity-100");

    fireEvent.click(screen.getByText("Archive project"));

    expect(screen.getByText("Archive project?")).toBeInTheDocument();
    expect(onArchiveProject).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Archive project" }));

    expect(onArchiveProject).toHaveBeenCalledWith("project-1");
  });

  it("does not show a tooltip for project actions", async () => {
    const user = userEvent.setup();
    conversationsByProject.set("project-1", {
      data: [conversation()],
      total: 6,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([project()]);

    const actions = screen.getByTestId("agents-project-actions-project-1");
    const trigger = within(actions).getByRole("button", { name: "Project actions" });

    await user.hover(trigger);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("opens a rename dialog from session actions and saves the new title", async () => {
    const user = userEvent.setup();
    const onRenameConversation = vi.fn().mockResolvedValue(undefined);
    const activeConversation = conversation({ id: "conversation-rename", title: "Untitled agent" });
    conversationsByProject.set("project-1", {
      data: [activeConversation],
      total: 1,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([project()], { onRenameConversation });

    await user.click(screen.getByRole("button", { name: "Session actions" }));
    await user.click(screen.getByText("Rename session"));

    const input = screen.getByLabelText("Session title");
    await user.clear(input);
    await user.type(input, "Review follow-up");
    await user.click(screen.getByRole("button", { name: "Rename session" }));

    await waitFor(() =>
      expect(onRenameConversation).toHaveBeenCalledWith("conversation-rename", "Review follow-up")
    );
    expect(screen.queryByText("Rename session")).not.toBeInTheDocument();
  });

  it("confirms before archiving a session", async () => {
    const user = userEvent.setup();
    const onArchiveConversation = vi.fn();
    const activeConversation = conversation({ id: "conversation-archive", title: "Untitled agent" });
    conversationsByProject.set("project-1", {
      data: [activeConversation],
      total: 1,
      isLoading: false,
      hasNextPage: false,
      isFetchingNextPage: false,
      fetchNextPage: vi.fn(),
    });

    renderSidebar([project()], { onArchiveConversation });

    await user.click(screen.getByRole("button", { name: "Session actions" }));
    await user.click(screen.getByText("Archive session"));

    expect(screen.getByText("Archive session?")).toBeInTheDocument();
    expect(onArchiveConversation).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Archive session" }));

    expect(onArchiveConversation).toHaveBeenCalledWith(activeConversation);
  });
});
