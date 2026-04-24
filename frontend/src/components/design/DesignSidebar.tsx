import { ChevronDown, ChevronRight, Download, Palette, Plus, Search, Upload, X } from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { DesignSystem } from "./designSystems";
import type { DesignSystemProjectGroup } from "./useProjectDesignSystems";

interface DesignSidebarProps {
  groups: DesignSystemProjectGroup[];
  focusedProjectId: string | null;
  selectedDesignSystemId: string | null;
  searchQuery: string;
  onSearchQueryChange: (query: string) => void;
  onFocusProject: (projectId: string) => void;
  onSelectDesignSystem: (system: DesignSystem) => void;
  onNewDesignSystem: () => void;
  onImportDesignSystem: () => void;
}

export function DesignSidebar({
  groups,
  focusedProjectId,
  selectedDesignSystemId,
  searchQuery,
  onSearchQueryChange,
  onFocusProject,
  onSelectDesignSystem,
  onNewDesignSystem,
  onImportDesignSystem,
}: DesignSidebarProps) {
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [collapsedProjectIds, setCollapsedProjectIds] = useState<Set<string>>(new Set());
  const visibleGroups = useMemo(
    () => groups.filter((group) => group.systems.length > 0 || group.project.id === focusedProjectId),
    [focusedProjectId, groups],
  );

  const toggleProject = (projectId: string) => {
    setCollapsedProjectIds((current) => {
      const next = new Set(current);
      if (next.has(projectId)) {
        next.delete(projectId);
      } else {
        next.add(projectId);
      }
      return next;
    });
  };

  return (
    <aside
      className="w-full h-full flex flex-col border-r overflow-hidden"
      style={{
        background: "color-mix(in srgb, var(--bg-surface) 92%, transparent)",
        borderColor: "var(--overlay-faint)",
      }}
      data-testid="design-sidebar"
    >
      <div className="px-3.5 pt-3.5 pb-2.5 flex items-center gap-2 shrink-0">
        <Palette className="w-4 h-4 shrink-0" style={{ color: "var(--accent-primary)" }} />
        <span className="text-[14px] font-semibold truncate" style={{ color: "var(--text-primary)" }}>
          Design
        </span>
        <div className="ml-auto flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 w-7 p-0 rounded-md"
                onClick={onNewDesignSystem}
                aria-label="New design system"
                data-testid="design-new-system"
              >
                <Plus className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">New design system</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 w-7 p-0 rounded-md"
                onClick={onImportDesignSystem}
                aria-label="Import design system"
                data-testid="design-import-system"
              >
                <Upload className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">Import</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 w-7 p-0 rounded-md"
                onClick={() => {
                  setIsSearchOpen((open) => {
                    if (open) {
                      onSearchQueryChange("");
                    }
                    return !open;
                  });
                }}
                aria-label={isSearchOpen ? "Close search" : "Search design systems"}
                data-testid="design-search-toggle"
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
          <input
            value={searchQuery}
            onChange={(event) => onSearchQueryChange(event.target.value)}
            placeholder="Search"
            className="w-full h-8 px-3 text-[12px] bg-transparent border rounded-md outline-none"
            style={{
              color: "var(--text-primary)",
              borderColor: "var(--overlay-weak)",
              background: "var(--overlay-faint)",
            }}
            autoFocus
            data-testid="design-search-input"
          />
        </div>
      )}

      <div className="flex-1 overflow-y-auto py-1.5 border-t" style={{ borderColor: "var(--overlay-faint)" }}>
        {visibleGroups.length === 0 ? (
          <div className="h-full px-5 flex flex-col items-center justify-center text-center gap-3">
            <div className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
              No design systems
            </div>
            <Button type="button" size="sm" onClick={onNewDesignSystem} className="gap-2">
              <Plus className="w-4 h-4" />
              New design system
            </Button>
          </div>
        ) : (
          visibleGroups.map(({ project, systems }) => {
            const isCollapsed = collapsedProjectIds.has(project.id);
            const isFocused = focusedProjectId === project.id;

            return (
              <div key={project.id} className="mt-1.5 px-3" data-testid={`design-project-${project.id}`}>
                <div className="flex items-center gap-1.5 min-h-8">
                  <button
                    type="button"
                    className="h-5 w-5 flex items-center justify-center rounded-md"
                    onClick={() => toggleProject(project.id)}
                    aria-label={isCollapsed ? "Expand project" : "Collapse project"}
                  >
                    {isCollapsed ? <ChevronRight className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
                  </button>
                  <button
                    type="button"
                    className="min-w-0 flex-1 text-left text-[11px] font-semibold truncate"
                    onClick={() => onFocusProject(project.id)}
                    style={{ color: isFocused ? "var(--text-primary)" : "var(--text-muted)" }}
                  >
                    {project.name}
                  </button>
                  <span className="text-[10px]" style={{ color: "var(--text-muted)" }}>
                    {systems.length}
                  </span>
                </div>

                {!isCollapsed && (
                  <div className="mt-1 space-y-1">
                    {systems.map((system) => (
                      <button
                        key={system.id}
                        type="button"
                        onClick={() => onSelectDesignSystem(system)}
                        className={cn(
                          "w-full rounded-lg border px-2.5 py-2 text-left transition-colors",
                          selectedDesignSystemId === system.id ? "border-accent-primary" : "",
                        )}
                        style={{
                          borderColor:
                            selectedDesignSystemId === system.id
                              ? "var(--accent-border)"
                              : "var(--overlay-faint)",
                          background:
                            selectedDesignSystemId === system.id
                              ? "var(--accent-muted)"
                              : "transparent",
                        }}
                        data-testid={`design-system-${system.id}`}
                      >
                        <div className="flex items-center gap-2">
                          <span className="min-w-0 flex-1 text-[12px] font-medium truncate" style={{ color: "var(--text-primary)" }}>
                            {system.name}
                          </span>
                          <span className="text-[10px]" style={{ color: "var(--text-muted)" }}>
                            v{system.version}
                          </span>
                        </div>
                        <div className="mt-1 flex items-center gap-2 text-[10px]" style={{ color: "var(--text-muted)" }}>
                          <span>{system.status.replace("_", " ")}</span>
                          <span>{system.sourceCount} sources</span>
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>

      <div className="px-3.5 py-3 border-t shrink-0" style={{ borderColor: "var(--overlay-faint)" }}>
        <Button type="button" variant="outline" className="w-full gap-2" onClick={onImportDesignSystem}>
          <Download className="w-4 h-4" />
          Import package
        </Button>
      </div>
    </aside>
  );
}
