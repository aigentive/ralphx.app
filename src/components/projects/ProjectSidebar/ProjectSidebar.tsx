/**
 * ProjectSidebar - Premium left sidebar with project list and navigation
 * Shows project list with git mode indicators, navigation items,
 * and New Project button
 *
 * Design: specs/design/pages/project-sidebar.md
 */

import { useMemo } from "react";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import type { Project } from "@/types/project";
import type { ViewType } from "@/types/chat";

// shadcn/ui components
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

// Lucide icons
import {
  X,
  Folder,
  FolderOpen,
  FolderGit2,
  GitBranch,
  Plus,
  LayoutGrid,
  Lightbulb,
  Activity,
  Settings,
} from "lucide-react";

// ============================================================================
// Props Interface
// ============================================================================

export interface ProjectSidebarProps {
  /** Callback when New Project button is clicked */
  onNewProject: () => void;
}

// ============================================================================
// Navigation Items
// ============================================================================

interface NavItem {
  key: ViewType;
  label: string;
  icon: typeof LayoutGrid;
  shortcut: string;
}

const NAV_ITEMS: NavItem[] = [
  { key: "kanban", label: "Kanban", icon: LayoutGrid, shortcut: "⌘1" },
  { key: "ideation", label: "Ideation", icon: Lightbulb, shortcut: "⌘2" },
  { key: "activity", label: "Activity", icon: Activity, shortcut: "⌘3" },
  { key: "settings", label: "Settings", icon: Settings, shortcut: "⌘4" },
];

// ============================================================================
// Sub-components
// ============================================================================

interface ProjectItemProps {
  project: Project;
  isActive: boolean;
  onClick: () => void;
}

function ProjectItem({ project, isActive, onClick }: ProjectItemProps) {
  const isWorktree = project.gitMode === "worktree";

  // Choose icon based on state
  const IconComponent = isActive
    ? FolderOpen
    : isWorktree
      ? FolderGit2
      : Folder;

  return (
    <button
      data-testid={`project-item-${project.id}`}
      data-active={isActive ? "true" : "false"}
      onClick={onClick}
      className="group relative w-full flex items-start gap-3 px-3 py-2 rounded-lg text-left transition-all duration-150 hover:translate-x-0.5"
      style={{
        backgroundColor: isActive ? "var(--bg-elevated)" : "transparent",
        color: isActive ? "var(--text-primary)" : "var(--text-secondary)",
      }}
      onMouseEnter={(e) => {
        if (!isActive) {
          e.currentTarget.style.backgroundColor = "var(--bg-hover)";
          e.currentTarget.style.color = "var(--text-primary)";
        }
      }}
      onMouseLeave={(e) => {
        if (!isActive) {
          e.currentTarget.style.backgroundColor = "transparent";
          e.currentTarget.style.color = "var(--text-secondary)";
        }
      }}
    >
      {/* Active indicator bar */}
      {isActive && (
        <span
          className="absolute left-0 top-[20%] bottom-[20%] w-[3px] rounded-r animate-in fade-in duration-150"
          style={{ backgroundColor: "var(--accent-primary)" }}
        />
      )}

      {/* Project icon */}
      <IconComponent
        className="mt-0.5 flex-shrink-0 transition-colors"
        style={{ color: isActive ? "var(--accent-primary)" : "var(--text-muted)" }}
        size={16}
        strokeWidth={1.5}
      />

      <div className="flex-1 min-w-0">
        {/* Project name */}
        <div className="text-sm font-medium truncate">{project.name}</div>

        {/* Git mode badge */}
        <div className="flex items-center gap-1.5 mt-1">
          {isWorktree ? (
            <>
              <span
                className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded"
                style={{
                  backgroundColor: "var(--bg-base)",
                  color: "var(--text-muted)",
                }}
              >
                <GitBranch size={10} strokeWidth={1.5} />
                Worktree
              </span>
              <span
                className="text-xs truncate"
                style={{ color: "var(--text-muted)" }}
              >
                {project.worktreeBranch}
              </span>
            </>
          ) : (
            <span
              className="text-xs px-1.5 py-0.5 rounded"
              style={{
                backgroundColor: "var(--bg-base)",
                color: "var(--text-muted)",
              }}
            >
              Local
            </span>
          )}
        </div>
      </div>
    </button>
  );
}

function EmptyProjectList() {
  return (
    <div
      data-testid="project-list-empty"
      className="flex flex-col items-center justify-center p-6 text-center"
    >
      <div
        className="w-12 h-12 rounded-full flex items-center justify-center mb-3"
        style={{ backgroundColor: "var(--bg-base)" }}
      >
        <Folder
          size={24}
          strokeWidth={1.5}
          style={{ color: "var(--text-muted)", opacity: 0.5 }}
        />
      </div>
      <p className="text-sm font-medium" style={{ color: "var(--text-secondary)" }}>
        No projects yet
      </p>
      <p className="text-xs mt-1" style={{ color: "var(--text-muted)" }}>
        Create a project to get started
      </p>
    </div>
  );
}

interface WorktreeStatusProps {
  branch: string;
  baseBranch: string;
}

function WorktreeStatus({ branch, baseBranch }: WorktreeStatusProps) {
  return (
    <div
      data-testid="worktree-status"
      className="flex items-start gap-2 mx-3 my-2 px-3 py-2 rounded-lg"
      style={{ backgroundColor: "var(--bg-base)" }}
    >
      <GitBranch
        className="mt-0.5 flex-shrink-0"
        size={14}
        strokeWidth={1.5}
        style={{ color: "var(--text-muted)" }}
      />
      <div className="flex-1 min-w-0">
        <div
          className="text-xs font-medium truncate leading-tight"
          style={{ color: "var(--text-primary)" }}
        >
          {branch}
        </div>
        <div className="text-xs leading-tight" style={{ color: "var(--text-muted)" }}>
          from {baseBranch}
        </div>
      </div>
    </div>
  );
}

interface NavigationProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
}

function Navigation({ currentView, onViewChange }: NavigationProps) {
  return (
    <TooltipProvider delayDuration={300}>
      <nav data-testid="sidebar-navigation" className="flex flex-col gap-1 px-2 pb-4">
        {NAV_ITEMS.map((item) => {
          const isActive = currentView === item.key;
          const Icon = item.icon;

          return (
            <Tooltip key={item.key}>
              <TooltipTrigger asChild>
                <button
                  data-active={isActive ? "true" : "false"}
                  onClick={() => onViewChange(item.key)}
                  className="relative flex items-center gap-3 px-3 py-2 h-9 rounded-lg transition-colors duration-150"
                  style={{
                    backgroundColor: isActive ? "var(--bg-elevated)" : "transparent",
                    color: isActive ? "var(--accent-primary)" : "var(--text-secondary)",
                  }}
                  onMouseEnter={(e) => {
                    if (!isActive) {
                      e.currentTarget.style.backgroundColor = "var(--bg-hover)";
                      e.currentTarget.style.color = "var(--text-primary)";
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (!isActive) {
                      e.currentTarget.style.backgroundColor = "transparent";
                      e.currentTarget.style.color = "var(--text-secondary)";
                    }
                  }}
                >
                  {/* Active indicator bar */}
                  {isActive && (
                    <span
                      className="absolute left-0 top-[20%] bottom-[20%] w-[3px] rounded-r"
                      style={{ backgroundColor: "var(--accent-primary)" }}
                    />
                  )}
                  <Icon size={18} strokeWidth={1.5} />
                  <span className="text-sm font-medium">{item.label}</span>
                </button>
              </TooltipTrigger>
              <TooltipContent side="right" className="flex items-center gap-2">
                <span>{item.label}</span>
                <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-black/20 rounded">
                  {item.shortcut}
                </kbd>
              </TooltipContent>
            </Tooltip>
          );
        })}
      </nav>
    </TooltipProvider>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function ProjectSidebar({ onNewProject }: ProjectSidebarProps) {
  const projects = useProjectStore((s) => s.projects);
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const selectProject = useProjectStore((s) => s.selectProject);
  const activeProject = useProjectStore(selectActiveProject);
  const currentView = useUiStore((s) => s.currentView);
  const setCurrentView = useUiStore((s) => s.setCurrentView);
  const setSidebarOpen = useUiStore((s) => s.setSidebarOpen);

  // Convert projects record to sorted array (most recently updated first)
  const projectList = useMemo(() => {
    return Object.values(projects).sort(
      (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
    );
  }, [projects]);

  const hasProjects = projectList.length > 0;
  const showWorktreeStatus =
    activeProject?.gitMode === "worktree" &&
    activeProject.worktreeBranch &&
    activeProject.baseBranch;

  return (
    <aside
      data-testid="project-sidebar"
      className="flex flex-col h-full w-64 border-r"
      style={{
        backgroundColor: "var(--bg-surface)",
        borderColor: "var(--border-subtle)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-3 h-12 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <h2
          className="text-xs font-semibold uppercase"
          style={{
            color: "var(--text-muted)",
            letterSpacing: "0.05em",
          }}
        >
          Projects
        </h2>
        <button
          data-testid="sidebar-close"
          onClick={() => setSidebarOpen(false)}
          aria-label="Close sidebar"
          className="p-1.5 rounded-lg transition-colors duration-150"
          style={{ color: "var(--text-muted)" }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = "var(--bg-hover)";
            e.currentTarget.style.color = "var(--text-primary)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = "transparent";
            e.currentTarget.style.color = "var(--text-muted)";
          }}
        >
          <X size={16} strokeWidth={2} />
        </button>
      </div>

      {/* Worktree Status (when applicable) */}
      {showWorktreeStatus && (
        <WorktreeStatus
          branch={activeProject.worktreeBranch!}
          baseBranch={activeProject.baseBranch!}
        />
      )}

      {/* Project List */}
      <ScrollArea className="flex-1">
        <div className="px-2 py-2">
          {hasProjects ? (
            <div className="flex flex-col gap-1">
              {projectList.map((project) => (
                <ProjectItem
                  key={project.id}
                  project={project}
                  isActive={project.id === activeProjectId}
                  onClick={() => selectProject(project.id)}
                />
              ))}
            </div>
          ) : (
            <EmptyProjectList />
          )}
        </div>
      </ScrollArea>

      {/* New Project Button */}
      <div className="px-3 py-2 border-t" style={{ borderColor: "var(--border-subtle)" }}>
        <Button
          variant="secondary"
          className="w-full justify-center gap-2 h-9"
          onClick={onNewProject}
          style={{
            backgroundColor: "var(--bg-elevated)",
            color: "var(--text-primary)",
          }}
        >
          <Plus size={16} strokeWidth={2} />
          <span>New Project</span>
        </Button>
      </div>

      {/* Divider */}
      <div className="px-3 py-2">
        <Separator style={{ backgroundColor: "var(--border-subtle)" }} />
      </div>

      {/* Navigation */}
      <Navigation currentView={currentView} onViewChange={setCurrentView} />
    </aside>
  );
}
