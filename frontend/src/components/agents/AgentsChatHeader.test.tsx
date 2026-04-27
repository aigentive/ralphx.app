import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AgentsChatHeader } from "./AgentsChatHeader";
import {
  conversationFixture as conversation,
  conversationWorkspaceFixture as conversationWorkspace,
  renderWithAgentProviders as renderWithProviders,
} from "./agentsTestFixtures";

describe("AgentsChatHeader", () => {
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

  it("shows conversation stats in the Agents chat header", () => {
    renderWithProviders(
      <AgentsChatHeader
        conversation={conversation()}
        workspace={null}
        modelDisplay={{ id: "gpt-5.4", label: "gpt-5.4" }}
        artifactOpen={false}
        activeArtifactTab="plan"
        onRenameConversation={vi.fn().mockResolvedValue(undefined)}
        onToggleArtifacts={vi.fn()}
        onSelectArtifact={vi.fn()}
      />
    );

    expect(screen.getByTestId("chat-session-chips")).toBeInTheDocument();
    expect(screen.getByTestId("chat-session-stats-button")).toBeInTheDocument();
    expect(screen.getByText("gpt-5.4")).toBeInTheDocument();
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
