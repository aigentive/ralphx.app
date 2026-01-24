/**
 * ProjectSidebar - Left sidebar with project list and navigation
 * Shows project list with git mode indicators, navigation items,
 * and New Project button
 */

import { useMemo } from "react";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import type { Project } from "@/types/project";
import type { ViewType } from "@/types/chat";

// ============================================================================
// Props Interface
// ============================================================================

export interface ProjectSidebarProps {
  /** Callback when New Project button is clicked */
  onNewProject: () => void;
}

// ============================================================================
// Icons
// ============================================================================

function FolderIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M2 4a1 1 0 011-1h3.586a1 1 0 01.707.293L8 4h5a1 1 0 011 1v7a1 1 0 01-1 1H3a1 1 0 01-1-1V4z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M8 3v10M3 8h10"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M12 4L4 12M4 4l8 8"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function KanbanIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <rect x="2" y="2" width="3" height="12" rx="1" stroke="currentColor" strokeWidth="1.5" />
      <rect x="6.5" y="2" width="3" height="8" rx="1" stroke="currentColor" strokeWidth="1.5" />
      <rect x="11" y="2" width="3" height="5" rx="1" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}

function IdeationIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M8 1a5 5 0 015 5c0 1.85-1 3.47-2.5 4.33V12a1.5 1.5 0 01-1.5 1.5H7A1.5 1.5 0 015.5 12v-1.67C4 9.47 3 7.85 3 6a5 5 0 015-5z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M6.5 14.5h3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function ActivityIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M14 8H12L10 13L6 3L4 8H2"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M8 10a2 2 0 100-4 2 2 0 000 4z"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <path
        d="M13 8a5 5 0 01-.08 1l1.68 1.28a.4.4 0 01.08.48l-1.6 2.8a.4.4 0 01-.48.16l-2-.8a5 5 0 01-1.68.96l-.32 2.08a.4.4 0 01-.4.32H5.6a.4.4 0 01-.4-.32l-.32-2.08a5 5 0 01-1.68-.96l-2 .8a.4.4 0 01-.48-.16l-1.6-2.8a.4.4 0 01.08-.48L.88 9a5 5 0 010-2L.48 5.72a.4.4 0 01-.08-.48l1.6-2.8a.4.4 0 01.48-.16l2 .8a5 5 0 011.68-.96l.32-2.08a.4.4 0 01.4-.32h3.2a.4.4 0 01.4.32l.32 2.08a5 5 0 011.68.96l2-.8a.4.4 0 01.48.16l1.6 2.8a.4.4 0 01-.08.48L13.88 7a5 5 0 01.12 1z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function GitBranchIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <circle cx="3" cy="3" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="3" cy="9" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="9" cy="5" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <path d="M3 4.5V7.5M7.5 5H4.5C4.5 5 4.5 3 3 3" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

// ============================================================================
// Navigation Items
// ============================================================================

interface NavItem {
  key: ViewType;
  label: string;
  icon: React.ReactNode;
}

const NAV_ITEMS: NavItem[] = [
  { key: "kanban", label: "Kanban", icon: <KanbanIcon /> },
  { key: "ideation", label: "Ideation", icon: <IdeationIcon /> },
  { key: "activity", label: "Activity", icon: <ActivityIcon /> },
  { key: "settings", label: "Settings", icon: <SettingsIcon /> },
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

  return (
    <button
      data-testid={`project-item-${project.id}`}
      data-active={isActive ? "true" : "false"}
      onClick={onClick}
      className="w-full flex items-start gap-3 px-3 py-2 rounded-lg transition-colors text-left"
      style={{
        backgroundColor: isActive ? "var(--bg-elevated)" : "transparent",
        color: isActive ? "var(--text-primary)" : "var(--text-secondary)",
      }}
    >
      <span
        className="mt-0.5"
        style={{ color: isActive ? "var(--accent-primary)" : "var(--text-muted)" }}
      >
        <FolderIcon />
      </span>
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium truncate">{project.name}</div>
        <div className="flex items-center gap-1.5 mt-0.5">
          {isWorktree ? (
            <>
              <span
                className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded"
                style={{
                  backgroundColor: "var(--bg-base)",
                  color: "var(--text-muted)",
                }}
              >
                <GitBranchIcon />
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
        <FolderIcon />
      </div>
      <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
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
      className="flex items-center gap-2 px-3 py-2 rounded-lg"
      style={{ backgroundColor: "var(--bg-base)" }}
    >
      <GitBranchIcon />
      <div className="flex-1 min-w-0">
        <div className="text-xs font-medium truncate" style={{ color: "var(--text-primary)" }}>
          {branch}
        </div>
        <div className="text-xs" style={{ color: "var(--text-muted)" }}>
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
    <nav data-testid="sidebar-navigation" className="flex flex-col gap-1">
      {NAV_ITEMS.map((item) => {
        const isActive = currentView === item.key;
        return (
          <button
            key={item.key}
            data-active={isActive ? "true" : "false"}
            onClick={() => onViewChange(item.key)}
            className="flex items-center gap-3 px-3 py-2 rounded-lg transition-colors"
            style={{
              backgroundColor: isActive ? "var(--bg-elevated)" : "transparent",
              color: isActive ? "var(--accent-primary)" : "var(--text-secondary)",
            }}
          >
            {item.icon}
            <span className="text-sm font-medium">{item.label}</span>
          </button>
        );
      })}
    </nav>
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

  // Convert projects record to sorted array
  const projectList = useMemo(() => {
    return Object.values(projects).sort((a, b) =>
      new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
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
        className="flex items-center justify-between px-4 py-3 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <h2
          className="text-sm font-semibold uppercase tracking-wide"
          style={{ color: "var(--text-muted)" }}
        >
          Projects
        </h2>
        <button
          data-testid="sidebar-close"
          onClick={() => setSidebarOpen(false)}
          className="p-1 rounded transition-colors hover:bg-white/5"
          style={{ color: "var(--text-muted)" }}
        >
          <CloseIcon />
        </button>
      </div>

      {/* Worktree Status (when applicable) */}
      {showWorktreeStatus && (
        <div className="px-3 py-2">
          <WorktreeStatus
            branch={activeProject.worktreeBranch!}
            baseBranch={activeProject.baseBranch!}
          />
        </div>
      )}

      {/* Project List */}
      <div className="flex-1 overflow-y-auto px-2 py-2">
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

      {/* New Project Button */}
      <div className="px-3 py-2 border-t" style={{ borderColor: "var(--border-subtle)" }}>
        <button
          onClick={onNewProject}
          className="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg transition-colors"
          style={{
            backgroundColor: "var(--bg-elevated)",
            color: "var(--text-primary)",
          }}
        >
          <PlusIcon />
          <span className="text-sm font-medium">New Project</span>
        </button>
      </div>

      {/* Divider */}
      <div className="px-3 py-2">
        <div
          className="h-px w-full"
          style={{ backgroundColor: "var(--border-subtle)" }}
        />
      </div>

      {/* Navigation */}
      <div className="px-2 pb-4">
        <Navigation currentView={currentView} onViewChange={setCurrentView} />
      </div>
    </aside>
  );
}
