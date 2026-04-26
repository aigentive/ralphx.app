import {
  getAgentsViewTestMocks,
  mockAgentViewData,
  renderAgentsView,
  resetAgentSessionState,
  setupAgentsViewTest,
} from "./AgentsView.testSetup";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import { useAgentTerminalStore } from "./agentTerminalStore";
import {
  conversationFixture as conversation,
  conversationWorkspaceFixture as conversationWorkspace,
} from "./agentsTestFixtures";

const {
  getAgentConversationWorkspaceMock,
  integratedChatPanelRenderMock,
  terminalDrawerMountMock,
  terminalDrawerUnmountMock,
} = getAgentsViewTestMocks();

describe("AgentsView terminal docks", () => {
  beforeEach(setupAgentsViewTest);

  it("opens the artifact panel and moves an open auto terminal in the same click", async () => {
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
    expect(drawer).toHaveAttribute("data-placement", "auto");
    await waitFor(() => expect(terminalDrawerMountMock).toHaveBeenCalledTimes(1));
    integratedChatPanelRenderMock.mockClear();

    fireEvent.click(await screen.findByTestId("agents-publish-workspace"));

    expect(screen.getByTestId("agents-artifact-resizable-pane")).toHaveStyle({
      width: "66.666667%",
      opacity: "1",
      pointerEvents: "auto",
    });
    expect(screen.getByTestId("agents-artifact-resizable-pane")).toContainElement(
      screen.getByTestId("agent-terminal-drawer")
    );
    expect(integratedChatPanelRenderMock).not.toHaveBeenCalled();
    expect(terminalDrawerMountMock).toHaveBeenCalledTimes(1);
    expect(terminalDrawerUnmountMock).not.toHaveBeenCalled();
  });

  it("closes the artifact panel and moves an open auto terminal in the same click", async () => {
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
    useAgentTerminalStore.setState({
      openByConversationId: { "conversation-1": true },
      heightByConversationId: {},
      activeTerminalByConversationId: {},
      placement: "auto",
    });

    renderAgentsView();

    const drawer = await screen.findByTestId("agent-terminal-drawer");
    await waitFor(() =>
      expect(screen.getByTestId("agents-artifact-resizable-pane")).toContainElement(drawer)
    );
    await waitFor(() => expect(terminalDrawerMountMock).toHaveBeenCalledTimes(1));
    integratedChatPanelRenderMock.mockClear();

    fireEvent.click(screen.getByLabelText("Close panel"));

    expect(screen.getByTestId("agents-artifact-resizable-pane")).toHaveStyle({
      width: "0px",
      minWidth: "0px",
      maxWidth: "0px",
      opacity: "0",
      pointerEvents: "none",
    });
    expect(screen.getByTestId("agent-terminal-host-chat")).toContainElement(
      screen.getByTestId("agent-terminal-drawer")
    );
    expect(integratedChatPanelRenderMock).not.toHaveBeenCalled();
    expect(terminalDrawerMountMock).toHaveBeenCalledTimes(1);
    expect(terminalDrawerUnmountMock).not.toHaveBeenCalled();
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
    const closedPane = await screen.findByTestId("agents-artifact-resizable-pane");
    expect(closedPane).toHaveStyle({
      width: "0px",
      minWidth: "0px",
      maxWidth: "0px",
      opacity: "0",
      pointerEvents: "none",
    });

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

});
