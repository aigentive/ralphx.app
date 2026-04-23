import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  AlertTriangle,
  Archive,
  Bot,
  CheckCircle2,
  ChevronLeft,
  ChevronDown,
  ChevronRight,
  Circle,
  Folder,
  MoreHorizontal,
  Plus,
  RotateCcw,
  Search,
  X,
  XCircle,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useChatStore } from "@/stores/chatStore";
import {
  useAgentSessionStore,
  type AgentProjectSort,
} from "@/stores/agentSessionStore";
import { withAlpha } from "@/lib/theme-colors";
import type { Project } from "@/types/project";
import {
  formatAgentConversationCreatedAt,
  getAgentConversationStoreKey,
  type AgentConversation,
} from "./agentConversations";
import { useProjectAgentConversations } from "./useProjectAgentConversations";

const PROJECT_SORT_LABELS: Record<AgentProjectSort, string> = {
  latest: "Latest",
  az: "A-Z",
  za: "Z-A",
};

interface AgentsSidebarProps {
  projects: Project[];
  focusedProjectId: string | null;
  selectedConversationId: string | null;
  onFocusProject: (projectId: string) => void;
  onSelectConversation: (projectId: string, conversation: AgentConversation) => void;
  onCreateAgent: () => void;
  onArchiveProject: (projectId: string) => void | Promise<void>;
  onArchiveConversation: (conversation: AgentConversation) => void;
  onRestoreConversation: (conversation: AgentConversation) => void;
  showArchived: boolean;
  onShowArchivedChange: (showArchived: boolean) => void;
  onCollapse?: () => void;
}

export function AgentsSidebar({
  projects,
  focusedProjectId,
  selectedConversationId,
  onFocusProject,
  onSelectConversation,
  onCreateAgent,
  onArchiveProject,
  onArchiveConversation,
  onRestoreConversation,
  showArchived,
  onShowArchivedChange,
  onCollapse,
}: AgentsSidebarProps) {
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const showAllProjects = useAgentSessionStore((s) => s.showAllProjects);
  const projectSort = useAgentSessionStore((s) => s.projectSort);
  const setShowAllProjects = useAgentSessionStore((s) => s.setShowAllProjects);
  const setProjectSort = useAgentSessionStore((s) => s.setProjectSort);
  const normalizedSearch = searchQuery.trim().toLowerCase();
  const orderedProjects = useMemo(() => {
    if (projectSort === "latest") {
      return projects;
    }

    const sorted = [...projects].sort((left, right) =>
      left.name.localeCompare(right.name, undefined, { sensitivity: "base" })
    );

    return projectSort === "za" ? sorted.reverse() : sorted;
  }, [projectSort, projects]);

  return (
    <aside
      className="w-full h-full flex flex-col border-r overflow-hidden"
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
                onClick={onCreateAgent}
                aria-label="New agent"
                data-testid="agents-new-agent"
                style={{ boxShadow: "none" }}
              >
                <Plus className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              New agent
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
          {onCollapse && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-7 w-7 p-0 rounded-md border-0 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
                  onClick={onCollapse}
                  aria-label="Collapse sidebar"
                  data-testid="agents-sidebar-collapse-button"
                  style={{ boxShadow: "none" }}
                >
                  <ChevronLeft className="w-4 h-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs">
                Collapse sidebar
              </TooltipContent>
            </Tooltip>
          )}
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

      {projects.length > 0 && (
        <div className="px-3.5 pb-2 flex items-center gap-2 shrink-0">
          <button
            type="button"
            onClick={() => setShowAllProjects(!showAllProjects)}
            data-testid="agents-show-all-projects-pill"
            className="h-7 inline-flex items-center rounded-full border px-2.5 text-[11px] font-medium transition-colors outline-none ring-0 focus:outline-none focus-visible:outline-none"
            style={{
              color: showAllProjects ? "var(--text-primary)" : "var(--text-secondary)",
              background: showAllProjects
                ? withAlpha("var(--accent-primary)", 12)
                : "transparent",
              borderColor: showAllProjects ? withAlpha("var(--accent-primary)", 30) : "var(--overlay-weak)",
            }}
          >
            All projects
          </button>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                data-testid="agents-project-sort-pill"
                className="h-7 inline-flex items-center gap-1.5 rounded-full border px-2.5 text-[11px] font-medium transition-colors outline-none ring-0 focus:outline-none focus-visible:outline-none"
                style={{
                  color: "var(--text-secondary)",
                  background: "transparent",
                  borderColor: "var(--overlay-weak)",
                }}
              >
                {PROJECT_SORT_LABELS[projectSort]}
                <ChevronDown className="h-3.5 w-3.5" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuRadioGroup
                value={projectSort}
                onValueChange={(value) => setProjectSort(value as AgentProjectSort)}
              >
                {Object.entries(PROJECT_SORT_LABELS).map(([value, label]) => (
                  <DropdownMenuRadioItem key={value} value={value} className="text-xs">
                    {label}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      )}

      <div className="flex-1 overflow-y-auto py-1.5 border-t" style={{ borderColor: "var(--overlay-faint)" }}>
        {projects.length === 0 ? (
          <div className="h-full px-5 flex flex-col items-center justify-center text-center gap-3">
            <div className="space-y-1">
              <div className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
                No agent conversations yet.
              </div>
              <div className="text-xs leading-5" style={{ color: "var(--text-muted)" }}>
                Open the starter from the + button to begin a conversation and create a
                project inline if you need one.
              </div>
            </div>
            <Button type="button" size="sm" onClick={onCreateAgent} className="gap-2">
              <Plus className="w-4 h-4" />
              Open starter
            </Button>
          </div>
        ) : (
          orderedProjects.map((project) => (
            <ProjectSessionGroup
              key={project.id}
              project={project}
              isFocused={focusedProjectId === project.id}
              selectedConversationId={selectedConversationId}
              searchQuery={normalizedSearch}
              onFocusProject={onFocusProject}
              onSelectConversation={onSelectConversation}
              onArchiveProject={onArchiveProject}
              onArchiveConversation={onArchiveConversation}
              onRestoreConversation={onRestoreConversation}
              showArchived={showArchived}
              showAllProjects={showAllProjects}
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
  onArchiveProject: (projectId: string) => void | Promise<void>;
  onArchiveConversation: (conversation: AgentConversation) => void;
  onRestoreConversation: (conversation: AgentConversation) => void;
  showArchived: boolean;
  showAllProjects: boolean;
}

function ProjectSessionGroup({
  project,
  isFocused,
  selectedConversationId,
  searchQuery,
  onFocusProject,
  onSelectConversation,
  onArchiveProject,
  onArchiveConversation,
  onRestoreConversation,
  showArchived,
  showAllProjects,
}: ProjectSessionGroupProps) {
  const projectActionsWrapperRef = useRef<HTMLDivElement | null>(null);
  const projectActionsTriggerRef = useRef<HTMLButtonElement | null>(null);
  const [projectActionsOpen, setProjectActionsOpen] = useState(false);
  const [projectActionsTooltipOpen, setProjectActionsTooltipOpen] = useState(false);
  const [projectActionsTooltipSuppressed, setProjectActionsTooltipSuppressed] =
    useState(false);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);
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
  const totalConversationCount = conversations.total;
  const activeRuntimeCount = visibleConversations.filter((conversation) => {
    const rowKey = getAgentConversationStoreKey(conversation);
    return (
      activeConversationIds[rowKey] === conversation.id &&
      (agentStatuses[rowKey] ?? "idle") !== "idle"
    );
  }).length;

  useEffect(() => {
    const handlePointerDown = (event: PointerEvent) => {
      if (
        projectActionsWrapperRef.current?.contains(event.target as Node) ??
        false
      ) {
        return;
      }

      setProjectActionsTooltipOpen(false);
      setProjectActionsTooltipSuppressed(true);
      projectActionsTriggerRef.current?.blur();
    };

    document.addEventListener("pointerdown", handlePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true);
    };
  }, []);

  if (
    !conversations.isLoading &&
    visibleConversations.length === 0 &&
    (searchQuery ? !projectMatchesSearch || !showAllProjects : !showAllProjects)
  ) {
    return null;
  }

  return (
    <div className="mt-1.5" data-testid={`agents-project-${project.id}`}>
      <div className="px-3">
        <div className="group/project">
          <div
            className="w-full min-h-8 px-1.5 py-1 flex items-center gap-1.5 rounded-md transition-colors duration-150"
            style={{
              color: isFocused ? "var(--text-primary)" : "var(--text-muted)",
              background: "transparent",
            }}
          >
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
            <button
              type="button"
              className="min-w-0 flex-1 flex items-center gap-2 bg-transparent border-0 p-0 text-left shadow-none outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
              onClick={() => onFocusProject(project.id)}
              style={{ boxShadow: "none" }}
            >
              <Folder className="w-3.5 h-3.5 shrink-0" />
              <span className="min-w-0 flex-1 flex items-center gap-2">
                <span className="text-[11px] font-semibold tracking-[-0.01em] truncate">
                  {project.name}
                </span>
                {totalConversationCount > 0 && (
                  <span
                    className="shrink-0 text-[10px] font-medium leading-none"
                    style={{
                      color: isFocused ? "var(--accent-primary)" : "var(--text-muted)",
                    }}
                  >
                    {totalConversationCount}
                  </span>
                )}
              </span>
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
              ref={projectActionsWrapperRef}
              className={`flex items-center gap-0.5 transition-opacity duration-150 ${
                projectActionsOpen
                  ? "opacity-100"
                  : "opacity-0 group-hover/project:opacity-100 group-focus-within/project:opacity-100"
              }`}
              data-testid={`agents-project-actions-${project.id}`}
              onPointerEnter={() => {
                if (!projectActionsOpen) {
                  setProjectActionsTooltipSuppressed(false);
                }
              }}
              onPointerLeave={() => {
                if (!projectActionsOpen) {
                  setProjectActionsTooltipOpen(false);
                  setProjectActionsTooltipSuppressed(false);
                }
              }}
            >
              <DropdownMenu
                onOpenChange={(open) => {
                  setProjectActionsOpen(open);
                  if (open) {
                    setProjectActionsTooltipOpen(false);
                    setProjectActionsTooltipSuppressed(true);
                  }
                  if (!open) {
                    setProjectActionsTooltipOpen(false);
                    requestAnimationFrame(() => {
                      projectActionsTriggerRef.current?.blur();
                    });
                  }
                }}
              >
                <Tooltip
                  open={
                    projectActionsTooltipSuppressed ? false : projectActionsTooltipOpen
                  }
                  onOpenChange={(open) => {
                    if (projectActionsTooltipSuppressed && open) {
                      return;
                    }
                    setProjectActionsTooltipOpen(open);
                  }}
                >
                  <TooltipTrigger asChild>
                    <DropdownMenuTrigger asChild>
                      <Button
                        ref={projectActionsTriggerRef}
                        type="button"
                        variant="ghost"
                        size="sm"
                        className="h-5.5 w-5.5 p-0 rounded border-0 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
                        aria-label="Project actions"
                        data-theme-button-skip="true"
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
                    onClick={() => setArchiveDialogOpen(true)}
                  >
                    <Archive className="w-3.5 h-3.5" />
                    Archive project
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>

          <AlertDialog open={archiveDialogOpen} onOpenChange={setArchiveDialogOpen}>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Archive project?</AlertDialogTitle>
                <AlertDialogDescription>
                  This removes <span className="font-medium">{project.name}</span> from the
                  sidebar without deleting it. You can add the same repository again later
                  from the normal project flow.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    setArchiveDialogOpen(false);
                    void onArchiveProject(project.id);
                  }}
                  variant="destructive"
                >
                  Archive project
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>

          {expanded && (
            <div className="mt-0.5 ml-5 space-y-0.5">
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
                      className="group/session"
                      data-testid={`agents-session-${conversation.id}`}
                    >
                      <div
                        className="w-full min-h-[30px] px-1.5 py-1 flex items-center gap-1.5 cursor-pointer rounded-md transition-all duration-150 ease-out"
                        style={{
                          color: isSelected ? "var(--text-primary)" : "var(--text-secondary)",
                          background: isSelected
                            ? withAlpha("var(--accent-primary)", 6)
                            : "transparent",
                          opacity: conversation.archivedAt ? 0.58 : 1,
                        }}
                        >
                          <button
                            type="button"
                            className="min-w-0 flex-1 flex items-center gap-1.5 bg-transparent border-0 p-0 text-left shadow-none outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none focus-visible:ring-0"
                            onClick={() => onSelectConversation(project.id, conversation)}
                            style={{ boxShadow: "none" }}
                          >
                            <span
                              className="w-3.5 h-3.5 flex items-center justify-center shrink-0"
                              style={{
                                color: isSelected ? "var(--accent-primary)" : "var(--text-muted)",
                              }}
                          >
                            <SessionStateGlyph
                              isSelected={isSelected}
                              isActiveRuntime={isActiveRuntime}
                              status={agentStatus}
                              />
                            </span>
                            <span className="min-w-0 flex-1 flex items-baseline gap-2 leading-none">
                              <span className="min-w-0 truncate text-[10.75px] font-medium tracking-[-0.01em]">
                                {title}
                              </span>
                              <span
                                className="shrink-0 text-[10px]"
                                style={{ color: "var(--text-muted)" }}
                              >
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
                      </div>
                    );
                  })}

                {visibleConversations.length > 0 && conversations.hasNextPage && (
                  <div className="py-0.5">
                    <button
                      type="button"
                      className="inline-flex items-center pl-[26px] text-[10.75px] font-medium transition-colors"
                      onClick={() => void conversations.fetchNextPage()}
                      disabled={conversations.isFetchingNextPage}
                      data-testid={`agents-load-more-${project.id}`}
                      style={{
                        color: "var(--text-muted)",
                        opacity: conversations.isFetchingNextPage ? 0.7 : 1,
                      }}
                    >
                      {conversations.isFetchingNextPage ? "Loading..." : "Load more"}
                    </button>
                  </div>
                )}

                {conversations.isLoading && (
                  <div className="py-1.5 text-[11px]" style={{ color: "var(--text-muted)" }}>
                    Loading...
                  </div>
                )}
            </div>
          )}
        </div>
      </div>
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
          className="w-3 h-3 shrink-0"
          style={{ color: "var(--status-warning)" }}
        />
      );
    }

    if (status === "failed" || status === "error") {
      return (
        <XCircle
          className="w-3 h-3 shrink-0"
          style={{ color: "var(--status-error)" }}
        />
      );
    }

    if (status === "completed") {
      return (
        <CheckCircle2
          className="w-3 h-3 shrink-0"
          style={{ color: "var(--status-success)" }}
        />
      );
    }

    if (status !== "idle") {
      return (
        <Circle
          className="w-2.5 h-2.5 shrink-0 fill-current"
          style={{ color: "var(--status-info)" }}
        />
      );
    }
  }

  return (
    <Circle
      className="w-2.5 h-2.5 shrink-0 fill-current"
      style={{ color: isSelected ? "var(--accent-primary)" : "var(--text-muted)" }}
    />
  );
}
