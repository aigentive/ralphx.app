import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Circle,
  Folder,
  MessageSquare,
  MoreHorizontal,
  Plus,
  Search,
  X,
  XCircle,
} from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { buildStoreKey } from "@/lib/chat-context-registry";
import { selectAgentStatus, useChatStore } from "@/stores/chatStore";
import { useAgentSessionStore } from "@/stores/agentSessionStore";
import type { Project } from "@/types/project";
import type { ChatConversation } from "@/types/chat-conversation";
import { useProjectAgentConversations } from "./useProjectAgentConversations";

interface AgentsSidebarProps {
  projects: Project[];
  focusedProjectId: string | null;
  selectedConversationId: string | null;
  onFocusProject: (projectId: string) => void;
  onSelectConversation: (projectId: string, conversation: ChatConversation) => void;
  onCreateAgent: () => void;
  onCreateProject: () => void;
  onQuickCreateAgent: (projectId?: string) => void;
  isCreatingAgent: boolean;
}

export function AgentsSidebar({
  projects,
  focusedProjectId,
  selectedConversationId,
  onFocusProject,
  onSelectConversation,
  onCreateAgent,
  onCreateProject,
  onQuickCreateAgent,
  isCreatingAgent,
}: AgentsSidebarProps) {
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const normalizedSearch = searchQuery.trim().toLowerCase();

  return (
    <aside
      className="w-[280px] min-w-[240px] max-w-[360px] h-full flex flex-col border-r overflow-hidden resize-x"
      style={{
        background: "var(--bg-surface)",
        borderColor: "var(--border-subtle)",
      }}
      data-testid="agents-sidebar"
    >
      <div
        className="h-11 px-3 flex items-center gap-2 border-b shrink-0"
        style={{
          backgroundColor: "color-mix(in srgb, var(--text-primary) 2%, transparent)",
          borderColor: "var(--border-subtle)",
        }}
      >
        <Bot className="w-4 h-4 shrink-0" style={{ color: "var(--accent-primary)" }} />
        <span className="text-sm font-semibold truncate" style={{ color: "var(--text-primary)" }}>
          Projects
        </span>
        <div className="ml-auto flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-8 w-8 p-0"
                onClick={onCreateProject}
                aria-label="New project"
                data-testid="agents-new-project"
              >
                <Plus className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              New project
            </TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-8 w-8 p-0"
                onClick={() => {
                  setIsSearchOpen((open) => {
                    if (open) {
                      setSearchQuery("");
                    }
                    return !open;
                  });
                }}
                aria-label={isSearchOpen ? "Close search" : "Search"}
                data-testid="agents-search-toggle"
              >
                {isSearchOpen ? <X className="w-4 h-4" /> : <Search className="w-4 h-4" />}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              {isSearchOpen ? "Close search" : "Search"}
            </TooltipContent>
          </Tooltip>
        </div>
      </div>

      {isSearchOpen && (
        <div
          className="px-3 py-2 border-b shrink-0"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <Input
            value={searchQuery}
            onChange={(event) => setSearchQuery(event.target.value)}
            placeholder="Search"
            className="h-8 text-xs"
            autoFocus
            data-testid="agents-search-input"
          />
        </div>
      )}

      <div
        className="p-2 border-b shrink-0"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="w-full justify-start gap-2 h-8"
          onClick={onCreateAgent}
          disabled={projects.length === 0}
          data-testid="agents-new-agent"
        >
          <Plus className="w-4 h-4" />
          <span className="text-xs font-medium">New agent</span>
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto py-2">
        {projects.length === 0 ? (
          <div className="h-full px-5 flex flex-col items-center justify-center text-center gap-3">
            <div className="space-y-1">
              <div className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
                No projects yet.
              </div>
              <div className="text-xs leading-5" style={{ color: "var(--text-muted)" }}>
                A project groups your chats, tasks, and repo.
              </div>
            </div>
            <Button type="button" size="sm" onClick={onCreateProject} className="gap-2">
              <Plus className="w-4 h-4" />
              Create first project
            </Button>
          </div>
        ) : (
          projects.map((project) => (
            <ProjectSessionGroup
              key={project.id}
              project={project}
              isFocused={focusedProjectId === project.id}
              selectedConversationId={selectedConversationId}
              searchQuery={normalizedSearch}
              onFocusProject={onFocusProject}
              onSelectConversation={onSelectConversation}
              onQuickCreateAgent={onQuickCreateAgent}
              isCreatingAgent={isCreatingAgent}
            />
          ))
        )}
      </div>
    </aside>
  );
}

interface ProjectSessionGroupProps {
  project: Project;
  isFocused: boolean;
  selectedConversationId: string | null;
  searchQuery: string;
  onFocusProject: (projectId: string) => void;
  onSelectConversation: (projectId: string, conversation: ChatConversation) => void;
  onQuickCreateAgent: (projectId?: string) => void;
  isCreatingAgent: boolean;
}

function ProjectSessionGroup({
  project,
  isFocused,
  selectedConversationId,
  searchQuery,
  onFocusProject,
  onSelectConversation,
  onQuickCreateAgent,
  isCreatingAgent,
}: ProjectSessionGroupProps) {
  const expanded = useAgentSessionStore((s) => s.expandedProjectIds[project.id] ?? true);
  const toggleProjectExpanded = useAgentSessionStore((s) => s.toggleProjectExpanded);
  const conversations = useProjectAgentConversations(project.id);
  const contextKey = buildStoreKey("project", project.id);
  const activeConversationId = useChatStore((s) => s.activeConversationIds[contextKey] ?? null);
  const agentStatus = useChatStore(selectAgentStatus(contextKey));

  const sortedConversations = [...(conversations.data ?? [])].sort((a, b) => {
    const aTime = a.lastMessageAt ?? a.createdAt;
    const bTime = b.lastMessageAt ?? b.createdAt;
    return new Date(bTime).getTime() - new Date(aTime).getTime();
  });
  const projectMatchesSearch = project.name.toLowerCase().includes(searchQuery);
  const visibleConversations = useMemo(() => {
    if (!searchQuery || projectMatchesSearch) {
      return sortedConversations;
    }
    return sortedConversations.filter((conversation) => {
      const title = conversation.title || "Untitled agent";
      const provider = conversation.providerHarness ?? "agent";
      return `${title} ${provider}`.toLowerCase().includes(searchQuery);
    });
  }, [projectMatchesSearch, searchQuery, sortedConversations]);
  const activeRuntimeCount = activeConversationId && agentStatus !== "idle" ? 1 : 0;

  if (
    searchQuery &&
    !projectMatchesSearch &&
    visibleConversations.length === 0 &&
    !conversations.isLoading
  ) {
    return null;
  }

  return (
    <div className="px-2 py-1" data-testid={`agents-project-${project.id}`}>
      <div
        className="w-full h-8 px-2 flex items-center gap-2 rounded-md"
        style={{
          color: isFocused ? "var(--text-primary)" : "var(--text-secondary)",
          background: isFocused ? "var(--bg-hover)" : "transparent",
        }}
      >
        <button
          type="button"
          className="h-6 w-6 flex items-center justify-center rounded"
          onClick={() => toggleProjectExpanded(project.id)}
          aria-label={expanded ? "Collapse project" : "Expand project"}
        >
          {expanded ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </button>
        <button
          type="button"
          className="min-w-0 flex-1 flex items-center gap-2 text-left"
          onClick={() => onFocusProject(project.id)}
          onDoubleClick={() => toggleProjectExpanded(project.id)}
        >
          <Folder className="w-4 h-4 shrink-0" />
          <span className="text-xs font-semibold truncate">{project.name}</span>
        </button>
        {!expanded && activeRuntimeCount > 0 && (
          <span
            className="min-w-5 h-5 px-1.5 inline-flex items-center justify-center rounded-full text-[10px] font-semibold"
            style={{
              color: "var(--accent-primary)",
              background: "var(--accent-muted)",
              border: "1px solid var(--accent-border)",
            }}
          >
            {activeRuntimeCount}
          </span>
        )}
        <div className="flex items-center gap-0.5 opacity-80">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0"
                onClick={() => onQuickCreateAgent(project.id)}
                disabled={isCreatingAgent}
                aria-label={`New agent in ${project.name}`}
              >
                <Plus className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="right" className="text-xs">
              New agent
            </TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0"
                disabled
                aria-label="Project actions"
              >
                <MoreHorizontal className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="right" className="text-xs">
              Project actions
            </TooltipContent>
          </Tooltip>
        </div>
      </div>

      {expanded && (
        <div className="mt-1 ml-4 space-y-0.5">
          {visibleConversations.map((conversation) => {
            const isSelected = selectedConversationId === conversation.id;
            const isActiveRuntime = activeConversationId === conversation.id;
            const title = conversation.title || "Untitled agent";
            const provider = conversation.providerHarness ?? "agent";
            const statusLabel =
              isActiveRuntime && agentStatus !== "idle" ? agentStatus.replace(/_/g, " ") : provider;

            return (
              <button
                key={conversation.id}
                type="button"
                className="w-full min-h-9 px-2 py-1.5 flex items-start gap-2 rounded-md text-left"
                onClick={() => onSelectConversation(project.id, conversation)}
                style={{
                  color: isSelected ? "var(--text-primary)" : "var(--text-secondary)",
                  background: isSelected ? "var(--accent-muted)" : "transparent",
                  border: isSelected
                    ? "1px solid var(--accent-border)"
                    : "1px solid transparent",
                }}
                data-testid={`agents-session-${conversation.id}`}
              >
                <SessionStateGlyph isSelected={isSelected} isActiveRuntime={isActiveRuntime} status={agentStatus} />
                <span className="min-w-0 flex-1">
                  <span className="block text-xs font-medium truncate">{title}</span>
                  <span className="block text-[11px] truncate" style={{ color: "var(--text-muted)" }}>
                    {statusLabel}
                  </span>
                </span>
              </button>
            );
          })}

          {!conversations.isLoading && visibleConversations.length === 0 && !searchQuery && (
            <div className="px-2 py-1.5 flex items-center gap-2">
              <span className="text-[11px] min-w-0 flex-1" style={{ color: "var(--text-muted)" }}>
                No chats yet.
              </span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-[11px]"
                onClick={() => onQuickCreateAgent(project.id)}
                disabled={isCreatingAgent}
              >
                Start
              </Button>
            </div>
          )}

          {conversations.isLoading && (
            <div className="px-2 py-2 text-[11px]" style={{ color: "var(--text-muted)" }}>
              Loading...
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function SessionStateGlyph({
  isSelected,
  isActiveRuntime,
  status,
}: {
  isSelected: boolean;
  isActiveRuntime: boolean;
  status: string;
}) {
  if (isActiveRuntime) {
    if (status === "needs_approval") {
      return (
        <AlertTriangle
          className="w-3.5 h-3.5 shrink-0 mt-0.5"
          style={{ color: "var(--status-warning)" }}
        />
      );
    }

    if (status === "failed" || status === "error") {
      return (
        <XCircle
          className="w-3.5 h-3.5 shrink-0 mt-0.5"
          style={{ color: "var(--status-error)" }}
        />
      );
    }

    if (status === "completed") {
      return (
        <CheckCircle2
          className="w-3.5 h-3.5 shrink-0 mt-0.5"
          style={{ color: "var(--status-success)" }}
        />
      );
    }

    if (status !== "idle") {
      return (
        <Circle
          className="w-3 h-3 shrink-0 mt-1 fill-current"
          style={{ color: "var(--status-info)" }}
        />
      );
    }
  }

  return isSelected ? (
    <Circle
      className="w-3 h-3 shrink-0 mt-1 fill-current"
      style={{ color: "var(--accent-primary)" }}
    />
  ) : (
    <MessageSquare className="w-3.5 h-3.5 shrink-0 mt-0.5" />
  );
}
