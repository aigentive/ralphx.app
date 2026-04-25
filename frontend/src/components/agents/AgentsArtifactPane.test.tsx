import { QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import type { AgentConversationWorkspace } from "@/api/chat";
import type { AgentArtifactTab } from "@/stores/agentSessionStore";
import { createTestQueryClient } from "@/test/store-utils";
import { AgentsArtifactPane } from "./AgentsArtifactPane";

const { getWorkspaceChangesMock, getWorkspaceDiffMock } = vi.hoisted(() => ({
  getWorkspaceChangesMock: vi.fn(),
  getWorkspaceDiffMock: vi.fn(),
}));

vi.mock("@/api/diff", () => ({
  diffApi: {
    getAgentConversationWorkspaceFileChanges: (...args: unknown[]) =>
      getWorkspaceChangesMock(...args),
    getAgentConversationWorkspaceFileDiff: (...args: unknown[]) =>
      getWorkspaceDiffMock(...args),
  },
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

  it("loads workspace changes for review before publishing", async () => {
    renderPane("publish", workspace({ mode: "edit" }));

    await waitFor(() => expect(screen.getByTestId("agents-review-changes")).toBeEnabled());
    expect(getWorkspaceChangesMock).toHaveBeenCalledWith("conversation-1");
  });
});
