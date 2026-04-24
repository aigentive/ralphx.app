import { useMemo } from "react";

import type { Project } from "@/types/project";
import { buildMockDesignSystems, type DesignSystem } from "./designSystems";

export interface DesignSystemProjectGroup {
  project: Project;
  systems: DesignSystem[];
}

export function useProjectDesignSystems(
  projects: Project[],
  options: { searchQuery?: string; includeArchived?: boolean } = {},
): DesignSystemProjectGroup[] {
  const normalizedSearch = options.searchQuery?.trim().toLowerCase() ?? "";
  const includeArchived = options.includeArchived ?? false;

  return useMemo(() => {
    const systems = buildMockDesignSystems(projects).filter(
      (system) => includeArchived || system.status !== "archived",
    );

    return projects.map((project) => {
      const projectSystems = systems.filter((system) => {
        if (system.primaryProjectId !== project.id) {
          return false;
        }
        if (!normalizedSearch) {
          return true;
        }
        return (
          project.name.toLowerCase().includes(normalizedSearch) ||
          system.name.toLowerCase().includes(normalizedSearch)
        );
      });

      return { project, systems: projectSystems };
    });
  }, [includeArchived, normalizedSearch, projects]);
}
