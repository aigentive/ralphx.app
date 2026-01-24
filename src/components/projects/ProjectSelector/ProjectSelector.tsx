/**
 * ProjectSelector - Compact header dropdown for project selection
 *
 * A refined dropdown selector showing current project with git mode indicator.
 * Provides quick access to all projects and new project creation.
 *
 * Design: Follows RalphX design system with warm orange accent, SF Pro fonts,
 * 8pt grid, dark theme. Full keyboard accessibility with arrow navigation.
 */

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import type { Project } from "@/types/project";

// ============================================================================
// Props Interface
// ============================================================================

export interface ProjectSelectorProps {
  /** Callback when New Project is selected */
  onNewProject: () => void;
  /** Optional className for custom styling */
  className?: string;
}

// ============================================================================
// Icons
// ============================================================================

function ChevronIcon({ isOpen }: { isOpen: boolean }) {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="none"
      style={{
        transform: isOpen ? "rotate(180deg)" : "rotate(0deg)",
        transition: "transform var(--transition-fast)",
      }}
    >
      <path
        d="M3 4.5L6 7.5L9 4.5"
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

function FolderIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M2 3.5a1 1 0 011-1h2.793a1 1 0 01.707.293L7 3.5h4a1 1 0 011 1v6a1 1 0 01-1 1H3a1 1 0 01-1-1V3.5z"
        stroke="currentColor"
        strokeWidth="1.25"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M7 3v8M3 7h8"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path
        d="M2.5 6.5L4.5 8.5L9.5 3.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
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
        {isWorktree ? <GitBranchIcon /> : null}
        <span>{isWorktree ? branch || "worktree" : "local"}</span>
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
      {isWorktree && <GitBranchIcon />}
      <span>{isWorktree ? "Worktree" : "Local"}</span>
    </span>
  );
}

interface DropdownItemProps {
  project: Project;
  isSelected: boolean;
  isFocused: boolean;
  onClick: () => void;
  onMouseEnter: () => void;
}

function DropdownItem({
  project,
  isSelected,
  isFocused,
  onClick,
  onMouseEnter,
}: DropdownItemProps) {
  const isWorktree = project.gitMode === "worktree";

  return (
    <button
      data-testid={`project-option-${project.id}`}
      role="option"
      aria-selected={isSelected}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      className="w-full flex items-center gap-3 px-3 py-2.5 text-left transition-colors"
      style={{
        backgroundColor: isFocused ? "var(--bg-hover)" : "transparent",
        color: isSelected ? "var(--text-primary)" : "var(--text-secondary)",
      }}
    >
      {/* Selection indicator */}
      <span
        className="w-4 flex-shrink-0 flex items-center justify-center"
        style={{
          color: isSelected ? "var(--accent-primary)" : "transparent",
        }}
      >
        {isSelected && <CheckIcon />}
      </span>

      {/* Folder icon */}
      <span
        className="flex-shrink-0"
        style={{
          color: isSelected ? "var(--accent-primary)" : "var(--text-muted)",
        }}
      >
        <FolderIcon />
      </span>

      {/* Project info */}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium truncate">{project.name}</div>
        <div className="flex items-center gap-2 mt-0.5">
          <GitModeBadge mode={project.gitMode} />
          {isWorktree && project.worktreeBranch && (
            <span
              className="text-xs truncate"
              style={{ color: "var(--text-muted)" }}
            >
              {project.worktreeBranch}
            </span>
          )}
        </div>
      </div>
    </button>
  );
}

interface NewProjectItemProps {
  isFocused: boolean;
  onClick: () => void;
  onMouseEnter: () => void;
}

function NewProjectItem({ isFocused, onClick, onMouseEnter }: NewProjectItemProps) {
  return (
    <button
      data-testid="new-project-option"
      role="option"
      aria-selected={false}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      className="w-full flex items-center gap-3 px-3 py-2.5 text-left transition-colors border-t"
      style={{
        backgroundColor: isFocused ? "var(--bg-hover)" : "transparent",
        borderColor: "var(--border-subtle)",
        color: "var(--text-secondary)",
      }}
    >
      {/* Spacer to align with other items */}
      <span className="w-4 flex-shrink-0" />

      {/* Plus icon */}
      <span
        className="flex-shrink-0"
        style={{ color: "var(--accent-primary)" }}
      >
        <PlusIcon />
      </span>

      {/* Label */}
      <span className="text-sm font-medium" style={{ color: "var(--accent-primary)" }}>
        New Project
      </span>
    </button>
  );
}

function EmptyState() {
  return (
    <div
      className="px-4 py-6 text-center"
      style={{ color: "var(--text-muted)" }}
    >
      <FolderIcon />
      <p className="text-sm mt-2">No projects yet</p>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function ProjectSelector({ onNewProject, className = "" }: ProjectSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [focusedIndex, setFocusedIndex] = useState<number>(-1);

  const triggerRef = useRef<HTMLButtonElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Store state
  const projects = useProjectStore((s) => s.projects);
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const selectProject = useProjectStore((s) => s.selectProject);
  const activeProject = useProjectStore(selectActiveProject);

  // Convert projects to sorted array
  const projectList = useMemo(() => {
    return Object.values(projects).sort((a, b) =>
      new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
    );
  }, [projects]);

  // Total items count (projects + new project option)
  const totalItems = projectList.length + 1;

  // Close dropdown when clicking outside
  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node) &&
        triggerRef.current &&
        !triggerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
        setFocusedIndex(-1);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [isOpen]);

  const handleSelectProject = useCallback(
    (projectId: string) => {
      selectProject(projectId);
      setIsOpen(false);
      setFocusedIndex(-1);
      triggerRef.current?.focus();
    },
    [selectProject]
  );

  const handleNewProject = useCallback(() => {
    setIsOpen(false);
    setFocusedIndex(-1);
    onNewProject();
  }, [onNewProject]);

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (!isOpen) {
        if (event.key === "Enter" || event.key === " " || event.key === "ArrowDown") {
          event.preventDefault();
          setIsOpen(true);
          setFocusedIndex(0);
        }
        return;
      }

      switch (event.key) {
        case "Escape":
          event.preventDefault();
          setIsOpen(false);
          setFocusedIndex(-1);
          triggerRef.current?.focus();
          break;

        case "ArrowDown":
          event.preventDefault();
          setFocusedIndex((prev) => (prev + 1) % totalItems);
          break;

        case "ArrowUp":
          event.preventDefault();
          setFocusedIndex((prev) => (prev - 1 + totalItems) % totalItems);
          break;

        case "Enter":
        case " ":
          event.preventDefault();
          if (focusedIndex >= 0 && focusedIndex < projectList.length) {
            const focusedProject = projectList[focusedIndex];
            if (focusedProject) {
              handleSelectProject(focusedProject.id);
            }
          } else if (focusedIndex === projectList.length) {
            handleNewProject();
          }
          break;

        case "Home":
          event.preventDefault();
          setFocusedIndex(0);
          break;

        case "End":
          event.preventDefault();
          setFocusedIndex(totalItems - 1);
          break;

        case "Tab":
          setIsOpen(false);
          setFocusedIndex(-1);
          break;
      }
    },
    [isOpen, focusedIndex, projectList, totalItems, handleSelectProject, handleNewProject]
  );

  const handleToggle = useCallback(() => {
    setIsOpen((prev) => !prev);
    if (!isOpen) {
      // Find the currently selected project index
      const selectedIndex = projectList.findIndex((p) => p.id === activeProjectId);
      setFocusedIndex(selectedIndex >= 0 ? selectedIndex : 0);
    } else {
      setFocusedIndex(-1);
    }
  }, [isOpen, projectList, activeProjectId]);

  const hasProjects = projectList.length > 0;

  return (
    <div className={`relative ${className}`}>
      {/* Trigger Button */}
      <button
        ref={triggerRef}
        data-testid="project-selector-trigger"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-label={activeProject ? `Current project: ${activeProject.name}` : "Select a project"}
        onClick={handleToggle}
        onKeyDown={handleKeyDown}
        className="flex items-center gap-2 px-3 py-1.5 rounded-lg transition-all"
        style={{
          backgroundColor: isOpen ? "var(--bg-elevated)" : "transparent",
          color: isOpen ? "var(--text-primary)" : "var(--text-secondary)",
          border: "1px solid",
          borderColor: isOpen ? "var(--border-default)" : "transparent",
        }}
      >
        {activeProject ? (
          <>
            {/* Project name */}
            <span className="text-sm font-medium max-w-[160px] truncate">
              {activeProject.name}
            </span>

            {/* Git mode indicator */}
            <GitModeBadge
              mode={activeProject.gitMode}
              branch={activeProject.worktreeBranch}
              compact
            />
          </>
        ) : (
          <span className="text-sm" style={{ color: "var(--text-muted)" }}>
            Select Project
          </span>
        )}

        {/* Chevron */}
        <span style={{ color: "var(--text-muted)" }}>
          <ChevronIcon isOpen={isOpen} />
        </span>
      </button>

      {/* Dropdown */}
      {isOpen && (
        <div
          ref={dropdownRef}
          data-testid="project-selector-dropdown"
          role="listbox"
          aria-label="Projects"
          className="absolute top-full left-0 mt-1 min-w-[280px] max-w-[340px] rounded-lg overflow-hidden z-50"
          style={{
            backgroundColor: "var(--bg-surface)",
            border: "1px solid var(--border-default)",
            boxShadow: "var(--shadow-lg)",
            animation: "dropdown-enter 150ms ease-out",
          }}
        >
          {/* Project list with max height and scroll */}
          <div
            className="max-h-[320px] overflow-y-auto"
            style={{
              scrollbarWidth: "thin",
              scrollbarColor: "var(--border-default) var(--bg-surface)",
            }}
          >
            {hasProjects ? (
              projectList.map((project, index) => (
                <DropdownItem
                  key={project.id}
                  project={project}
                  isSelected={project.id === activeProjectId}
                  isFocused={focusedIndex === index}
                  onClick={() => handleSelectProject(project.id)}
                  onMouseEnter={() => setFocusedIndex(index)}
                />
              ))
            ) : (
              <EmptyState />
            )}
          </div>

          {/* New Project option (always visible) */}
          <NewProjectItem
            isFocused={focusedIndex === projectList.length}
            onClick={handleNewProject}
            onMouseEnter={() => setFocusedIndex(projectList.length)}
          />
        </div>
      )}

      {/* Animation keyframes injected as inline style */}
      <style>{`
        @keyframes dropdown-enter {
          from {
            opacity: 0;
            transform: translateY(-4px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }
      `}</style>
    </div>
  );
}
