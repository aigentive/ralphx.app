/**
 * ProjectSelector - Compact header dropdown for project selection
 *
 * A refined dropdown selector showing current project with git mode indicator.
 * Uses shadcn DropdownMenu for proper keyboard accessibility and animations.
 *
 * Design: Follows RalphX design system with warm orange accent, SF Pro fonts,
 * 8pt grid, dark theme. Full keyboard accessibility with arrow navigation.
 */

import { useMemo, useCallback } from "react";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import { useProjects } from "@/hooks/useProjects";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  FolderOpen,
  ChevronDown,
  Plus,
  GitBranch,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { Project } from "@/types/project";

// ============================================================================
// Props Interface
// ============================================================================

export interface ProjectSelectorProps {
  /** Callback when New Project is selected */
  onNewProject: () => void;
  /** Optional className for custom styling */
  className?: string;
  /** Dropdown alignment - defaults to center */
  align?: "start" | "center" | "end";
}

// ============================================================================
// Sub-components
// ============================================================================

interface GitModeBadgeProps {
  mode: "local" | "worktree";
  branch?: string | null;
  compact?: boolean;
}

function GitModeBadge({ mode, branch, compact = false }: GitModeBadgeProps) {
  const isWorktree = mode === "worktree";

  if (compact) {
    return (
      <span
        className="inline-flex items-center gap-1 text-xs"
        style={{ color: "var(--text-muted)" }}
      >
        {isWorktree && <GitBranch className="w-3 h-3" />}
        <span className="font-mono">{isWorktree ? branch || "worktree" : "local"}</span>
      </span>
    );
  }

  return (
    <span
      className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded"
      style={{
        backgroundColor: isWorktree
          ? "rgba(255, 107, 53, 0.1)"
          : "var(--bg-base)",
        color: isWorktree
          ? "var(--accent-secondary)"
          : "var(--text-muted)",
      }}
    >
      {isWorktree && <GitBranch className="w-3 h-3" />}
      <span>{isWorktree ? "Worktree" : "Local"}</span>
    </span>
  );
}

interface ProjectItemProps {
  project: Project;
  isActive: boolean;
  onSelect: () => void;
}

function ProjectItem({ project, isActive, onSelect }: ProjectItemProps) {
  const isWorktree = project.gitMode === "worktree";

  return (
    <DropdownMenuItem
      className={cn(
        "flex items-center justify-between gap-2 px-3 py-2 cursor-pointer",
        isActive && "border-l-2 border-[var(--accent-primary)] bg-[var(--accent-muted)]"
      )}
      onClick={onSelect}
      data-testid={`project-option-${project.id}`}
    >
      <div className="flex items-center gap-2 min-w-0">
        {/* Active dot indicator */}
        <span
          className={cn(
            "w-1.5 h-1.5 rounded-full flex-shrink-0",
            isActive ? "bg-[var(--accent-primary)]" : "bg-transparent"
          )}
        />
        {/* Project name */}
        <span className="text-sm font-medium truncate">{project.name}</span>
      </div>
      <div className="flex items-center gap-1.5 flex-shrink-0">
        {/* Dirty indicator (if needed, project doesn't have this yet) */}
        {/* Branch name */}
        {isWorktree && project.worktreeBranch && (
          <span className="text-xs font-mono text-[var(--text-muted)]">
            {project.worktreeBranch}
          </span>
        )}
        {!isWorktree && (
          <span className="text-xs text-[var(--text-muted)]">local</span>
        )}
      </div>
    </DropdownMenuItem>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function ProjectSelector({ onNewProject, className = "", align = "center" }: ProjectSelectorProps) {
  // Store state (selection only)
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const selectProject = useProjectStore((s) => s.selectProject);
  const storeActiveProject = useProjectStore(selectActiveProject);

  // Fetch projects directly from backend via TanStack Query.
  // This avoids depending on the Zustand store sync (useEffect in App.tsx)
  // which can lag behind, causing projects to briefly disappear.
  const { data: fetchedProjects } = useProjects();

  // Convert projects to sorted array (most recently updated first)
  const projectList = useMemo(() => {
    if (!fetchedProjects) return [];
    return [...fetchedProjects].sort((a, b) =>
      new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
    );
  }, [fetchedProjects]);

  // Resolve active project: prefer store (already synced), fall back to query data
  // This handles the window where activeProjectId is hydrated from localStorage
  // but the store's projects record hasn't been populated yet.
  const activeProject = useMemo(() => {
    if (storeActiveProject) return storeActiveProject;
    if (!activeProjectId || !fetchedProjects) return null;
    return fetchedProjects.find((p) => p.id === activeProjectId) ?? null;
  }, [storeActiveProject, activeProjectId, fetchedProjects]);

  const handleSelectProject = useCallback(
    (projectId: string) => {
      selectProject(projectId);
    },
    [selectProject]
  );

  const hasProjects = projectList.length > 0;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          className={cn(
            "inline-flex items-center gap-2 px-3 h-8 border border-[var(--border-default)] max-w-[280px] overflow-hidden",
            className
          )}
          data-testid="project-selector-trigger"
        >
          <FolderOpen className="w-4 h-4 text-[var(--text-secondary)] flex-shrink-0" />
          {activeProject ? (
            <span className="flex items-center gap-2 min-w-0 overflow-hidden">
              <span className="text-sm font-medium truncate">{activeProject.name}</span>
              <GitModeBadge
                mode={activeProject.gitMode}
                branch={activeProject.worktreeBranch}
                compact
              />
            </span>
          ) : (
            <span className="text-sm text-[var(--text-muted)]">Select Project</span>
          )}
          <ChevronDown className="w-3.5 h-3.5 text-[var(--text-muted)] flex-shrink-0" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent
        className="w-60 bg-[var(--bg-elevated)] border-[var(--border-default)]"
        align={align}
        sideOffset={8}
        data-testid="project-selector-dropdown"
      >
        {/* Section label */}
        <DropdownMenuLabel
          className="text-xs uppercase tracking-wide text-[var(--text-muted)] px-3 py-2"
        >
          Recent Projects
        </DropdownMenuLabel>

        {/* Project list */}
        {hasProjects ? (
          <div className="max-h-[240px] overflow-y-auto">
            {projectList.map((project) => (
              <ProjectItem
                key={project.id}
                project={project}
                isActive={project.id === activeProjectId}
                onSelect={() => handleSelectProject(project.id)}
              />
            ))}
          </div>
        ) : (
          <div className="px-3 py-4 text-center text-sm text-[var(--text-muted)]">
            No projects yet
          </div>
        )}

        <DropdownMenuSeparator className="bg-[var(--border-subtle)]" />

        {/* New Project option */}
        <DropdownMenuItem
          className="flex items-center gap-2 px-3 py-2 cursor-pointer text-[var(--text-secondary)] hover:text-[var(--text-primary)] focus:text-[var(--text-primary)]"
          onClick={onNewProject}
          data-testid="new-project-option"
        >
          <Plus className="w-4 h-4 text-[var(--accent-primary)]" />
          <span className="text-sm font-medium">New Project...</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
