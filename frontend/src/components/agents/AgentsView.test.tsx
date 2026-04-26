import {
  getAgentsViewTestMocks,
  mockAgentViewData,
  mockSidebarBreakpoint,
  renderAgentsView,
  selectSidebarConversationRow,
  setupAgentsViewTest,
} from "./AgentsView.testSetup";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ideationApi } from "@/api/ideation";
import {
  conversationFixture as conversation,
  conversationWorkspaceFixture as conversationWorkspace,
} from "./agentsTestFixtures";
const {
  getAgentConversationWorkspaceMock,
  useConversationMock,
} = getAgentsViewTestMocks();

describe("AgentsView", () => {
  beforeEach(setupAgentsViewTest);

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
