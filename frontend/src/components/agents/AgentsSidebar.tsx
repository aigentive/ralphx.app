import {
  AlertTriangle,
  Archive,
  Bot,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Circle,
  Folder,
  MessageSquare,
  MoreHorizontal,
  Plus,
  RotateCcw,
  Search,
  Trash2,
  X,
  XCircle,
} from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useChatStore } from "@/stores/chatStore";
import { useAgentSessionStore } from "@/stores/agentSessionStore";
import { withAlpha } from "@/lib/theme-colors";
import type { Project } from "@/types/project";
import {
  formatAgentConversationCreatedAt,
  getAgentConversationStoreKey,
  type AgentConversation,
} from "./agentConversations";
import { useProjectAgentConversations } from "./useProjectAgentConversations";

interface AgentsSidebarProps {
  projects: Project[];
  focusedProjectId: string | null;
  selectedConversationId: string | null;
  onFocusProject: (projectId: string) => void;
  onSelectConversation: (projectId: string, conversation: AgentConversation) => void;
  onCreateAgent: () => void;
  onCreateProject: () => void;
  onQuickCreateAgent: (projectId?: string) => void;
  onRemoveProject: (projectId: string) => void;
  onArchiveConversation: (conversation: AgentConversation) => void;
  onRestoreConversation: (conversation: AgentConversation) => void;
  isCreatingAgent: boolean;
  showArchived: boolean;
  onShowArchivedChange: (showArchived: boolean) => void;
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
  onRemoveProject,
  onArchiveConversation,
  onRestoreConversation,
  isCreatingAgent,
  showArchived,
  onShowArchivedChange,
}: AgentsSidebarProps) {
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const normalizedSearch = searchQuery.trim().toLowerCase();

  return (
    <aside
      className="w-[264px] min-w-[228px] max-w-[340px] h-full flex flex-col border-r overflow-hidden resize-x"
      style={{
        background: "color-mix(in srgb, var(--bg-surface) 92%, transparent)",
        backdropFilter: "blur(20px) saturate(180%)",
        WebkitBackdropFilter: "blur(20px) saturate(180%)",
        borderColor: "var(--overlay-faint)",
      }}
      data-testid="agents-sidebar"
    >
      <div
        className="px-3.5 pt-3.5 pb-2.5 flex items-center gap-2 shrink-0"
        style={{
          borderColor: "var(--overlay-faint)",
        }}
      >
        <Bot className="w-4 h-4 shrink-0" style={{ color: "var(--accent-primary)" }} />
        <span className="text-[14px] font-semibold tracking-[-0.01em] truncate" style={{ color: "var(--text-primary)" }}>
          Projects
        </span>
        <div className="ml-auto flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 w-7 p-0 rounded-md border-0 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
                onClick={onCreateProject}
                aria-label="New project"
                data-testid="agents-new-project"
                style={{ boxShadow: "none" }}
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
                className="h-7 w-7 p-0 rounded-md border-0 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
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
                style={{ boxShadow: "none" }}
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
        <div className="px-3.5 pb-2 shrink-0">
          <div
            className="relative flex items-center"
            style={{
              background: "var(--overlay-faint)",
              border: "1px solid var(--overlay-weak)",
              borderRadius: "6px",
            }}
          >
            <Search
              className="absolute left-2.5 w-3.5 h-3.5 pointer-events-none"
              style={{ color: "var(--text-muted)" }}
            />
            <input
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="Search"
              className="w-full h-7 pl-8 pr-8 text-[12px] bg-transparent outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
              style={{
                color: "var(--text-primary)",
                caretColor: "var(--accent-primary)",
              }}
              autoFocus
              data-testid="agents-search-input"
            />
            {searchQuery !== "" && (
              <button
                type="button"
                aria-label="Clear search"
                onClick={() => setSearchQuery("")}
                className="absolute right-2 w-4 h-4 flex items-center justify-center rounded-sm transition-colors duration-100 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none"
                style={{ color: "var(--text-muted)" }}
              >
                <X className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </div>
      )}

      <div className="px-3.5 pb-2.5 shrink-0">
        <Button
          type="button"
          className="w-full justify-center gap-2 h-8 text-[12px] font-medium tracking-[-0.01em] border-0 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0 transition-colors duration-150"
          onClick={onCreateAgent}
          disabled={projects.length === 0}
          data-testid="agents-new-agent"
          style={{
            background: "var(--accent-primary)",
            color: "var(--text-on-accent)",
            boxShadow: "none",
          }}
          onMouseEnter={(event) => {
            event.currentTarget.style.background = withAlpha("var(--accent-primary)", 90);
          }}
          onMouseLeave={(event) => {
            event.currentTarget.style.background = "var(--accent-primary)";
          }}
        >
          <Plus className="w-4 h-4" strokeWidth={2.5} />
          <span>New agent</span>
        </Button>
      </div>

      <div className="flex-1 overflow-y-auto py-1.5 border-t" style={{ borderColor: "var(--overlay-faint)" }}>
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
              onRemoveProject={onRemoveProject}
              onArchiveConversation={onArchiveConversation}
              onRestoreConversation={onRestoreConversation}
              isCreatingAgent={isCreatingAgent}
              showArchived={showArchived}
            />
          ))
        )}
      </div>

      <div
        className="px-3.5 py-2.5 border-t shrink-0"
        style={{ borderColor: "var(--overlay-faint)" }}
      >
        <label className="h-7 flex items-center justify-between gap-3">
          <span className="text-xs" style={{ color: "var(--text-muted)" }}>
            Archived
          </span>
          <Switch
            checked={showArchived}
            onCheckedChange={onShowArchivedChange}
            aria-label="Show archived sessions"
          />
        </label>
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
  onSelectConversation: (projectId: string, conversation: AgentConversation) => void;
  onQuickCreateAgent: (projectId?: string) => void;
  onRemoveProject: (projectId: string) => void;
  onArchiveConversation: (conversation: AgentConversation) => void;
  onRestoreConversation: (conversation: AgentConversation) => void;
  isCreatingAgent: boolean;
  showArchived: boolean;
}

function ProjectSessionGroup({
  project,
  isFocused,
  selectedConversationId,
  searchQuery,
  onFocusProject,
  onSelectConversation,
  onQuickCreateAgent,
  onRemoveProject,
  onArchiveConversation,
  onRestoreConversation,
  isCreatingAgent,
  showArchived,
}: ProjectSessionGroupProps) {
  const expanded = useAgentSessionStore((s) => s.expandedProjectIds[project.id] ?? true);
  const toggleProjectExpanded = useAgentSessionStore((s) => s.toggleProjectExpanded);
  const conversations = useProjectAgentConversations(project.id, showArchived, {
    search: searchQuery,
  });
  const activeConversationIds = useChatStore((s) => s.activeConversationIds);
  const agentStatuses = useChatStore((s) => s.agentStatus);
  const projectMatchesSearch = project.name.toLowerCase().includes(searchQuery);
  const visibleConversations = useMemo(
    () => conversations.data ?? [],
    [conversations.data]
  );
  const activeRuntimeCount = visibleConversations.filter((conversation) => {
    const rowKey = getAgentConversationStoreKey(conversation);
    return (
      activeConversationIds[rowKey] === conversation.id &&
      (agentStatuses[rowKey] ?? "idle") !== "idle"
    );
  }).length;

  if (
    searchQuery &&
    !projectMatchesSearch &&
    visibleConversations.length === 0 &&
    !conversations.isLoading
  ) {
    return null;
  }

  return (
    <div className="mt-1.5" data-testid={`agents-project-${project.id}`}>
      <div className="px-3.5">
      <div
        className="group/project w-full min-h-8 px-2.5 py-1.5 flex items-center gap-2 rounded-md transition-colors duration-150"
        style={{
          color: isFocused ? "var(--text-primary)" : "var(--text-muted)",
          background: isFocused ? "var(--overlay-faint)" : "var(--bg-base)",
          border: "1px solid var(--overlay-faint)",
        }}
        onMouseEnter={(event) => {
          if (!isFocused) {
            event.currentTarget.style.background = "var(--overlay-faint)";
          }
        }}
        onMouseLeave={(event) => {
          if (!isFocused) {
            event.currentTarget.style.background = "transparent";
          }
        }}
      >
        <button
          type="button"
          className="min-w-0 flex-1 flex items-center gap-2 bg-transparent border-0 p-0 text-left shadow-none outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
          onClick={() => onFocusProject(project.id)}
          style={{ boxShadow: "none" }}
        >
          <Folder className="w-4 h-4 shrink-0" />
          <span className="text-[11.5px] font-semibold tracking-[-0.01em] truncate">{project.name}</span>
        </button>
        {!expanded && activeRuntimeCount > 0 && (
          <span
            className="text-[10px] px-1.5 rounded-full font-medium leading-[16px]"
            style={{
              color: "var(--accent-primary)",
              background: withAlpha("var(--accent-primary)", 15),
            }}
          >
            {activeRuntimeCount}
          </span>
        )}
        <div
          className="hidden items-center gap-0.5 opacity-0 transition-opacity duration-150 group-hover/project:opacity-100"
          style={isFocused ? { opacity: 1 } : undefined}
        >
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-5.5 w-5.5 p-0 rounded border-0 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
                onClick={() => onQuickCreateAgent(project.id)}
                disabled={isCreatingAgent}
                aria-label={`New agent in ${project.name}`}
                style={{ boxShadow: "none" }}
              >
                <Plus className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="right" className="text-xs">
              New agent
            </TooltipContent>
          </Tooltip>
          <DropdownMenu>
            <Tooltip>
              <TooltipTrigger asChild>
                <DropdownMenuTrigger asChild>
                  <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-5.5 w-5.5 p-0 rounded border-0 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
                  aria-label="Project actions"
                  style={{ boxShadow: "none" }}
                >
                    <MoreHorizontal className="w-3.5 h-3.5" />
                  </Button>
                </DropdownMenuTrigger>
              </TooltipTrigger>
              <TooltipContent side="right" className="text-xs">
                Project actions
              </TooltipContent>
            </Tooltip>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                className="gap-2 text-xs"
                onClick={() => onRemoveProject(project.id)}
              >
                <Trash2 className="w-3.5 h-3.5" />
                Remove project
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
        <button
          type="button"
          className="h-4.5 w-4.5 flex items-center justify-center rounded outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none shrink-0"
          onClick={() => toggleProjectExpanded(project.id)}
          aria-label={expanded ? "Collapse project" : "Expand project"}
        >
          {expanded ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </button>
      </div>
      </div>

      {expanded && (
        <div className="mt-0.5 space-y-0.5">
          {visibleConversations.map((conversation) => {
            const rowKey = getAgentConversationStoreKey(conversation);
            const activeConversationId = activeConversationIds[rowKey] ?? null;
            const agentStatus = agentStatuses[rowKey] ?? "idle";
            const isSelected = selectedConversationId === conversation.id;
            const isActiveRuntime = activeConversationId === conversation.id;
            const title = conversation.title || "Untitled agent";
            const createdLabel = formatAgentConversationCreatedAt(conversation.createdAt);
            const statusLabel = conversation.archivedAt
              ? `Archived * ${createdLabel}`
              : createdLabel;

            return (
              <div
                key={conversation.id}
                className="group/session relative w-full min-h-[38px] px-3.5 py-1.5 flex items-center gap-2 cursor-pointer transition-all duration-150 ease-out"
                style={{
                  color: isSelected ? "var(--text-primary)" : "var(--text-secondary)",
                  background: isSelected
                    ? withAlpha("var(--accent-primary)", 12)
                    : "transparent",
                  borderTop: "1px solid transparent",
                  borderBottom: "1px solid transparent",
                  borderLeft: isSelected ? "2px solid var(--accent-primary)" : "2px solid transparent",
                  borderRight: "none",
                  opacity: conversation.archivedAt ? 0.58 : 1,
                }}
                onMouseEnter={(event) => {
                  if (!isSelected) {
                    event.currentTarget.style.background = "var(--overlay-faint)";
                  }
                }}
                onMouseLeave={(event) => {
                  if (!isSelected) {
                    event.currentTarget.style.background = "transparent";
                  }
                }}
                data-testid={`agents-session-${conversation.id}`}
              >
                <button
                  type="button"
                  className="min-w-0 flex-1 flex items-center gap-2 bg-transparent border-0 p-0 text-left shadow-none outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
                  onClick={() => onSelectConversation(project.id, conversation)}
                  style={{ boxShadow: "none" }}
                >
                  <span
                    className="w-5 h-5 rounded-md flex items-center justify-center shrink-0 transition-colors duration-150"
                    style={{
                      background: isSelected
                        ? withAlpha("var(--accent-primary)", 15)
                        : "var(--overlay-faint)",
                      border: isSelected
                        ? "1px solid var(--accent-border)"
                        : "1px solid var(--overlay-faint)",
                    }}
                  >
                    <SessionStateGlyph isSelected={isSelected} isActiveRuntime={isActiveRuntime} status={agentStatus} />
                  </span>
                  <span className="min-w-0 flex-1 leading-tight">
                    <span className="block text-[11.5px] font-medium tracking-[-0.01em] truncate">{title}</span>
                    <span className="mt-0.5 block text-[10px] truncate" style={{ color: "var(--text-muted)" }}>
                      {statusLabel}
                    </span>
                  </span>
                </button>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-5.5 w-5.5 p-0 rounded shrink-0 border-0 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0 opacity-0 group-hover/session:opacity-100 data-[state=open]:opacity-100"
                      aria-label="Session actions"
                      style={{
                        boxShadow: "none",
                        ...(isSelected ? { opacity: 1 } : {}),
                      }}
                    >
                      <MoreHorizontal className="w-3.5 h-3.5" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    {conversation.archivedAt ? (
                      <DropdownMenuItem
                        className="gap-2 text-xs"
                        onClick={() => onRestoreConversation(conversation)}
                      >
                        <RotateCcw className="w-3.5 h-3.5" />
                        Restore session
                      </DropdownMenuItem>
                    ) : (
                      <DropdownMenuItem
                        className="gap-2 text-xs"
                        onClick={() => onArchiveConversation(conversation)}
                      >
                        <Archive className="w-3.5 h-3.5" />
                        Archive session
                      </DropdownMenuItem>
                    )}
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            );
          })}

          {!conversations.isLoading && visibleConversations.length === 0 && !searchQuery && (
            <div className="px-3.5 py-1.5 flex items-center gap-2">
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

          {!conversations.isLoading && visibleConversations.length === 0 && searchQuery && projectMatchesSearch && (
            <div className="px-3.5 py-1.5 text-[11px]" style={{ color: "var(--text-muted)" }}>
              No matching agents.
            </div>
          )}

          {visibleConversations.length > 0 && conversations.hasNextPage && (
            <div className="px-3.5 py-0.5">
              <button
                type="button"
                className="text-[11px] font-medium transition-colors"
                onClick={() => void conversations.fetchNextPage()}
                disabled={conversations.isFetchingNextPage}
                data-testid={`agents-load-more-${project.id}`}
                style={{
                  color: "var(--accent-primary)",
                  opacity: conversations.isFetchingNextPage ? 0.7 : 1,
                }}
              >
                {conversations.isFetchingNextPage ? "Loading..." : "Load more"}
              </button>
            </div>
          )}

          {conversations.isLoading && (
            <div className="px-3.5 py-1.5 text-[11px]" style={{ color: "var(--text-muted)" }}>
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
          className="w-3.5 h-3.5 shrink-0"
          style={{ color: "var(--status-warning)" }}
        />
      );
    }

    if (status === "failed" || status === "error") {
      return (
        <XCircle
          className="w-3.5 h-3.5 shrink-0"
          style={{ color: "var(--status-error)" }}
        />
      );
    }

    if (status === "completed") {
      return (
        <CheckCircle2
          className="w-3.5 h-3.5 shrink-0"
          style={{ color: "var(--status-success)" }}
        />
      );
    }

    if (status !== "idle") {
      return (
        <Circle
          className="w-3 h-3 shrink-0 fill-current"
          style={{ color: "var(--status-info)" }}
        />
      );
    }
  }

  return isSelected ? (
    <MessageSquare
      className="w-3.5 h-3.5 shrink-0"
      style={{ color: "var(--accent-primary)" }}
    />
  ) : (
    <MessageSquare className="w-3.5 h-3.5 shrink-0" />
  );
}
