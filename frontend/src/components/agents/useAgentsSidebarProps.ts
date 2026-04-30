import { useMemo, type ComponentProps } from "react";

import { AgentsSidebar } from "./AgentsSidebar";

type AgentsSidebarShellProps = Omit<ComponentProps<typeof AgentsSidebar>, "onCollapse">;

interface UseAgentsSidebarPropsParams {
  defaultProjectId: string | null;
  focusedProjectId: string | null;
  onArchiveConversation: AgentsSidebarShellProps["onArchiveConversation"];
  onArchiveProject: AgentsSidebarShellProps["onArchiveProject"];
  onCreateAgent: AgentsSidebarShellProps["onCreateAgent"];
  onCreateProject: AgentsSidebarShellProps["onCreateProject"];
  onFocusProject: AgentsSidebarShellProps["onFocusProject"];
  onRenameConversation: AgentsSidebarShellProps["onRenameConversation"];
  onRestoreConversation: AgentsSidebarShellProps["onRestoreConversation"];
  onSelectConversation: AgentsSidebarShellProps["onSelectConversation"];
  onShowArchivedChange: AgentsSidebarShellProps["onShowArchivedChange"];
  pinnedConversation: AgentsSidebarShellProps["pinnedConversation"];
  projects: AgentsSidebarShellProps["projects"];
  selectedConversationId: AgentsSidebarShellProps["selectedConversationId"];
  showArchived: AgentsSidebarShellProps["showArchived"];
}

export function useAgentsSidebarProps({
  defaultProjectId,
  focusedProjectId,
  onArchiveConversation,
  onArchiveProject,
  onCreateAgent,
  onCreateProject,
  onFocusProject,
  onRenameConversation,
  onRestoreConversation,
  onSelectConversation,
  onShowArchivedChange,
  pinnedConversation,
  projects,
  selectedConversationId,
  showArchived,
}: UseAgentsSidebarPropsParams): AgentsSidebarShellProps {
  return useMemo(
    () => ({
      projects,
      focusedProjectId: focusedProjectId ?? defaultProjectId,
      selectedConversationId,
      pinnedConversation: pinnedConversation ?? null,
      onFocusProject,
      onSelectConversation,
      onCreateAgent,
      onCreateProject,
      onArchiveProject,
      onRenameConversation,
      onArchiveConversation,
      onRestoreConversation,
      showArchived,
      onShowArchivedChange,
    } as const),
    [
      defaultProjectId,
      focusedProjectId,
      onArchiveConversation,
      onArchiveProject,
      onCreateAgent,
      onCreateProject,
      onFocusProject,
      onRenameConversation,
      onRestoreConversation,
      onSelectConversation,
      onShowArchivedChange,
      pinnedConversation,
      projects,
      selectedConversationId,
      showArchived,
    ],
  );
}
