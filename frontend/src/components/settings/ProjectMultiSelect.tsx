/**
 * ProjectMultiSelect - Checkbox list for selecting multiple projects.
 *
 * Used in CreateKeyDialog and ApiKeyEntry expanded view to assign
 * project scopes to API keys.
 */

import { useProjects } from "@/hooks/useProjects";
import { Check, FolderOpen } from "lucide-react";

// ============================================================================
// Props
// ============================================================================

export interface ProjectMultiSelectProps {
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  disabled?: boolean;
}

// ============================================================================
// Component
// ============================================================================

export function ProjectMultiSelect({
  selectedIds,
  onChange,
  disabled = false,
}: ProjectMultiSelectProps) {
  const { data: projects = [], isLoading } = useProjects();

  const toggle = (id: string) => {
    if (disabled) return;
    if (selectedIds.includes(id)) {
      onChange(selectedIds.filter((x) => x !== id));
    } else {
      onChange([...selectedIds, id]);
    }
  };

  if (isLoading) {
    return (
      <div className="py-2 flex items-center justify-center">
        <div className="w-3.5 h-3.5 border-2 border-[var(--accent-primary)] border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  if (projects.length === 0) {
    return (
      <p className="text-xs text-[var(--text-muted)] italic">No projects found</p>
    );
  }

  return (
    <div
      className="flex flex-col gap-1 max-h-40 overflow-y-auto"
      data-testid="project-multi-select"
    >
      {projects.map((project) => {
        const isSelected = selectedIds.includes(project.id);
        return (
          <button
            key={project.id}
            type="button"
            onClick={() => toggle(project.id)}
            disabled={disabled}
            data-testid={`project-option-${project.id}`}
            className={[
              "flex items-center gap-2 px-2 py-1.5 rounded-md text-left transition-colors",
              disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer",
              isSelected
                ? "bg-[var(--accent-muted)] text-[var(--text-primary)]"
                : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]",
            ].join(" ")}
          >
            {/* Checkbox */}
            <span
              className={[
                "w-3.5 h-3.5 rounded shrink-0 border flex items-center justify-center",
                isSelected
                  ? "border-[var(--accent-primary)] bg-[var(--accent-primary)]"
                  : "border-[var(--border-default)] bg-transparent",
              ].join(" ")}
            >
              {isSelected && <Check className="w-2.5 h-2.5 text-white" />}
            </span>

            <FolderOpen
              className="w-3.5 h-3.5 shrink-0 text-[var(--text-muted)]"
            />
            <span className="text-xs truncate">{project.name}</span>
          </button>
        );
      })}
    </div>
  );
}
