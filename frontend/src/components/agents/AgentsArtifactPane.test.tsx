import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import type { AgentConversationWorkspace } from "@/api/chat";
import type { AgentArtifactTab } from "@/stores/agentSessionStore";
import { createTestQueryClient } from "@/test/store-utils";
import { AgentsArtifactPane } from "./AgentsArtifactPane";

const {
  getWorkspaceChangesMock,
  getWorkspaceDiffMock,
  listPublicationEventsMock,
  getWorkspaceFreshnessMock,
  updateWorkspaceFromBaseMock,
  openUrlMock,
} = vi.hoisted(() => ({
  getWorkspaceChangesMock: vi.fn(),
  getWorkspaceDiffMock: vi.fn(),
  listPublicationEventsMock: vi.fn(),
  getWorkspaceFreshnessMock: vi.fn(),
  updateWorkspaceFromBaseMock: vi.fn(),
  openUrlMock: vi.fn(),
}));

vi.mock("@/api/chat", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/api/chat")>();
  return {
    ...actual,
    chatApi: {
      ...actual.chatApi,
      listAgentConversationWorkspacePublicationEvents: (...args: unknown[]) =>
        listPublicationEventsMock(...args),
      getAgentConversationWorkspaceFreshness: (...args: unknown[]) =>
        getWorkspaceFreshnessMock(...args),
      updateAgentConversationWorkspaceFromBase: (...args: unknown[]) =>
        updateWorkspaceFromBaseMock(...args),
    },
  };
});

vi.mock("@/api/diff", () => ({
  diffApi: {
    getAgentConversationWorkspaceFileChanges: (...args: unknown[]) =>
      getWorkspaceChangesMock(...args),
    getAgentConversationWorkspaceFileDiff: (...args: unknown[]) =>
      getWorkspaceDiffMock(...args),
  },
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: (...args: unknown[]) => openUrlMock(...args),
}));

const workspace = (
  overrides: Partial<AgentConversationWorkspace> = {}
): AgentConversationWorkspace => ({
  conversationId: "conversation-1",
  projectId: "project-1",
  mode: "ideation",
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
  createdAt: "2026-04-23T09:00:00Z",
  updatedAt: "2026-04-23T09:00:00Z",
  ...overrides,
});

function renderPane(
  activeTab: AgentArtifactTab = "tasks",
  paneWorkspace = workspace(),
  onPublishWorkspace = vi.fn(),
  isPublishingWorkspace = false,
) {
  const queryClient = createTestQueryClient();

  return render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <div className="h-[480px]">
          <AgentsArtifactPane
            conversation={null}
            workspace={paneWorkspace}
            activeTab={activeTab}
            taskMode="graph"
            onTabChange={() => {}}
            onTaskModeChange={() => {}}
            onPublishWorkspace={onPublishWorkspace}
            isPublishingWorkspace={isPublishingWorkspace}
            onClose={() => {}}
          />
        </div>
      </TooltipProvider>
    </QueryClientProvider>
  );
}

describe("AgentsArtifactPane", () => {
  beforeEach(() => {
    getWorkspaceChangesMock.mockResolvedValue([
      { path: "frontend/src/App.tsx", status: "modified", additions: 4, deletions: 1 },
    ]);
    getWorkspaceDiffMock.mockResolvedValue({
      filePath: "frontend/src/App.tsx",
      oldContent: "old",
      newContent: "new",
      language: "typescript",
    });
    listPublicationEventsMock.mockResolvedValue([]);
    getWorkspaceFreshnessMock.mockResolvedValue({
      conversationId: "conversation-1",
      baseRef: "main",
      baseDisplayName: "Project default (main)",
      targetRef: "origin/main",
      capturedBaseCommit: "base-sha",
      targetBaseCommit: "base-sha",
      isBaseAhead: false,
    });
    updateWorkspaceFromBaseMock.mockResolvedValue({
      workspace: workspace({ mode: "edit", baseCommit: "base-sha" }),
      updated: false,
      targetRef: "origin/main",
      baseCommit: "base-sha",
    });
    openUrlMock.mockResolvedValue(undefined);
  });

  it("anchors the active tab border to the bottom edge of the tab bar", () => {
    renderPane();

    const tabRow = screen.getByTestId("agents-artifact-tab-row");
    const activeTab = screen.getByTestId("agents-artifact-tab-tasks");
    const inactiveTab = screen.getByTestId("agents-artifact-tab-plan");

    expect(tabRow.getAttribute("style")).toContain(
      "border-color: var(--border-subtle);"
    );
    expect(activeTab.parentElement?.className).toContain("self-stretch");
    expect(activeTab.className).toContain("self-stretch");
    expect(activeTab.getAttribute("data-theme-button-skip")).toBe("true");
    expect(inactiveTab.getAttribute("data-theme-button-skip")).toBe("true");
    expect(activeTab.className).not.toContain("border-b-2");
    expect(activeTab.querySelector("span[style='background: var(--accent-primary);']")).not.toBeNull();
    expect(inactiveTab.querySelector("span[style='background: var(--accent-primary);']")).toBeNull();
  });

  it("renders only the publish tab for edit workspaces", () => {
    renderPane("publish", workspace({ mode: "edit" }));

    expect(screen.getByTestId("agents-artifact-tab-publish")).toBeInTheDocument();
    expect(screen.getByTestId("agents-publish-pane")).toBeInTheDocument();
    expect(screen.queryByTestId("agents-artifact-tab-plan")).not.toBeInTheDocument();
    expect(screen.queryByTestId("agents-artifact-tab-verification")).not.toBeInTheDocument();
    expect(screen.queryByTestId("agents-artifact-tab-proposal")).not.toBeInTheDocument();
    expect(screen.queryByTestId("agents-artifact-tab-tasks")).not.toBeInTheDocument();
  });

  it("confirms publish from the publish pane", () => {
    const publish = vi.fn().mockResolvedValue(undefined);
    renderPane("publish", workspace({ mode: "edit" }), publish);

    fireEvent.click(screen.getByTestId("agents-publish-confirm"));

    expect(publish).toHaveBeenCalledWith("conversation-1");
  });

  it("opens the published PR from the publish pane", async () => {
    renderPane(
      "publish",
      workspace({
        mode: "edit",
        publicationPrNumber: 78,
        publicationPrUrl: "https://github.com/mock/project/pull/78",
      }),
    );

    fireEvent.click(await screen.findByTestId("agents-open-pr"));

    expect(openUrlMock).toHaveBeenCalledWith("https://github.com/mock/project/pull/78");
  });

  it("uses Update from base as the primary action when the base branch moved", async () => {
    const publish = vi.fn().mockResolvedValue(undefined);
    getWorkspaceFreshnessMock.mockResolvedValue({
      conversationId: "conversation-1",
      baseRef: "feature/agent-screen",
      baseDisplayName: "Current branch (feature/agent-screen)",
      targetRef: "origin/feature/agent-screen",
      capturedBaseCommit: "old-base",
      targetBaseCommit: "new-base",
      isBaseAhead: true,
    });
    updateWorkspaceFromBaseMock.mockResolvedValue({
      workspace: workspace({
        mode: "edit",
        baseRef: "feature/agent-screen",
        baseDisplayName: "Current branch (feature/agent-screen)",
        baseCommit: "new-base",
      }),
      updated: true,
      targetRef: "origin/feature/agent-screen",
      baseCommit: "new-base",
    });

    renderPane(
      "publish",
      workspace({
        mode: "edit",
        baseRef: "feature/agent-screen",
        baseDisplayName: "Current branch (feature/agent-screen)",
        baseCommit: "old-base",
      }),
      publish,
    );

    expect(await screen.findByTestId("agents-base-stale")).toHaveTextContent(
      "feature/agent-screen"
    );
    fireEvent.click(screen.getByTestId("agents-update-from-base"));

    await waitFor(() =>
      expect(updateWorkspaceFromBaseMock).toHaveBeenCalledWith("conversation-1")
    );
    expect(publish).not.toHaveBeenCalled();
  });

  it("loads workspace changes for review before publishing", async () => {
    renderPane("publish", workspace({ mode: "edit" }));

    await waitFor(() => expect(screen.getByTestId("agents-review-changes")).toBeEnabled());
    expect(getWorkspaceChangesMock).toHaveBeenCalledWith("conversation-1");
  });

  it("shows workspace publish pipeline status in the publish pane", () => {
    renderPane("publish", workspace({ mode: "edit", publicationPushStatus: "failed" }));

    expect(screen.getByTestId("agents-publish-pipeline")).toBeInTheDocument();
    expect(screen.getByTestId("agents-publish-step-checking")).toHaveTextContent(
      "Check workspace"
    );
    expect(screen.getByTestId("agents-publish-step-refreshing")).toHaveTextContent(
      "Refresh branch"
    );
    expect(screen.getByText(/Fixable errors are sent back to the workspace agent/i))
      .toBeInTheDocument();
  });

  it("shows agent repair state in the publish pipeline", () => {
    renderPane("publish", workspace({ mode: "edit", publicationPushStatus: "needs_agent" }));

    expect(screen.getByTestId("agents-publish-pipeline")).toBeInTheDocument();
    expect(screen.getByText(/sent it back to the workspace agent/i)).toBeInTheDocument();
  });

  it("renders durable publish history in the publish pane", async () => {
    listPublicationEventsMock.mockResolvedValue([
      {
        id: "event-1",
        conversationId: "conversation-1",
        step: "refreshing",
        status: "started",
        summary: "Refreshing branch from base",
        classification: null,
        createdAt: "2026-04-26T09:01:00Z",
      },
      {
        id: "event-2",
        conversationId: "conversation-1",
        step: "needs_agent",
        status: "failed",
        summary: "Pre-commit hook failed",
        classification: "agent_fixable",
        createdAt: "2026-04-26T09:02:00Z",
      },
    ]);

    renderPane("publish", workspace({ mode: "edit", publicationPushStatus: "needs_agent" }));

    expect(await screen.findByTestId("agents-publish-events")).toBeInTheDocument();
    expect(screen.getByText("Pre-commit hook failed")).toBeInTheDocument();
    expect(screen.getByText(/agent fixable/i)).toBeInTheDocument();
  });

  it("hides old started publish history rows after publish completes", async () => {
    listPublicationEventsMock.mockResolvedValue([
      {
        id: "event-checking",
        conversationId: "conversation-1",
        step: "checking",
        status: "started",
        summary: "Checking workspace changes",
        classification: null,
        createdAt: "2026-04-26T09:01:00Z",
      },
      {
        id: "event-pushing",
        conversationId: "conversation-1",
        step: "pushing",
        status: "started",
        summary: "Pushing agent branch",
        classification: null,
        createdAt: "2026-04-26T09:02:00Z",
      },
      {
        id: "event-published",
        conversationId: "conversation-1",
        step: "published",
        status: "succeeded",
        summary: "Draft pull request is ready",
        classification: null,
        createdAt: "2026-04-26T09:03:00Z",
      },
    ]);

    renderPane(
      "publish",
      workspace({
        mode: "edit",
        publicationPushStatus: "pushed",
        publicationPrNumber: 78,
      }),
    );

    expect(await screen.findByTestId("agents-publish-events")).toBeInTheDocument();
    expect(screen.queryByText("Checking workspace changes")).not.toBeInTheDocument();
    expect(screen.queryByText("Pushing agent branch")).not.toBeInTheDocument();
    expect(screen.getByText("Draft pull request is ready")).toBeInTheDocument();
    expect(screen.getByTestId("agents-publish-event-icon-event-published"))
      .toHaveAttribute("data-state", "succeeded");
  });

  it("shows only the latest started publish history row while publishing", async () => {
    listPublicationEventsMock.mockResolvedValue([
      {
        id: "event-checking",
        conversationId: "conversation-1",
        step: "checking",
        status: "started",
        summary: "Checking workspace changes",
        classification: null,
        createdAt: "2026-04-26T09:01:00Z",
      },
      {
        id: "event-pushing",
        conversationId: "conversation-1",
        step: "pushing",
        status: "started",
        summary: "Pushing agent branch",
        classification: null,
        createdAt: "2026-04-26T09:02:00Z",
      },
    ]);

    renderPane(
      "publish",
      workspace({
        mode: "edit",
        publicationPushStatus: "pushing",
      }),
      vi.fn(),
      true,
    );

    expect(await screen.findByTestId("agents-publish-events")).toBeInTheDocument();
    expect(screen.queryByText("Checking workspace changes")).not.toBeInTheDocument();
    expect(screen.getByText("Pushing agent branch")).toBeInTheDocument();
    expect(screen.getByTestId("agents-publish-event-icon-event-pushing"))
      .toHaveAttribute("data-state", "active");
  });
});
