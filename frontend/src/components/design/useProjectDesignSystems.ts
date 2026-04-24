import { useMemo } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";

import type { DesignSystemDetailResponse, DesignSystemResponse } from "@/api/design";
import { api, type CreateDesignSystemInput, type CreateDesignSystemResponse } from "@/lib/tauri";
import type { Project } from "@/types/project";
import { buildDesignSystemFromResponse, type DesignSystem } from "./designSystems";

export interface DesignSystemProjectGroup {
  project: Project;
  systems: DesignSystem[];
}

export const designSystemKeys = {
  all: ["design-systems"] as const,
  project: (projectId: string) => [...designSystemKeys.all, "project", projectId] as const,
  projectList: (projectId: string, includeArchived: boolean) =>
    [...designSystemKeys.project(projectId), "list", { includeArchived }] as const,
  detail: (designSystemId: string) => [...designSystemKeys.all, "detail", designSystemId] as const,
};

export function useProjectDesignSystems(
  projects: Project[],
  options: { searchQuery?: string; includeArchived?: boolean } = {},
): { groups: DesignSystemProjectGroup[]; isLoading: boolean; error: Error | null } {
  const normalizedSearch = options.searchQuery?.trim().toLowerCase() ?? "";
  const includeArchived = options.includeArchived ?? false;
  const designSystemQueries = useQueries({
    queries: projects.map((project) => ({
      queryKey: designSystemKeys.projectList(project.id, includeArchived),
      queryFn: () => api.design.listProjectDesignSystems(project.id, includeArchived),
      staleTime: 10 * 1000,
    })),
  });

  const groups = useMemo(() => {
    return projects.map((project, index) => {
      const responses = designSystemQueries[index]?.data ?? [];
      const projectSystems = responses
        .map((response) => buildDesignSystemFromResponse(project, response))
        .filter((system) => {
          if (!includeArchived && system.status === "archived") {
            return false;
          }
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
  }, [designSystemQueries, includeArchived, normalizedSearch, projects]);

  const error = designSystemQueries.find((query) => query.error)?.error ?? null;

  return {
    groups,
    isLoading: designSystemQueries.some((query) => query.isLoading),
    error,
  };
}

export function useCreateDesignSystem() {
  const queryClient = useQueryClient();

  return useMutation<CreateDesignSystemResponse, Error, CreateDesignSystemInput>({
    mutationFn: api.design.createDesignSystem,
    onSuccess: (response) => {
      const projectId = response.designSystem.primaryProjectId;
      queryClient.setQueryData<DesignSystemResponse[]>(
        designSystemKeys.projectList(projectId, false),
        (current = []) => [
          response.designSystem,
          ...current.filter((system) => system.id !== response.designSystem.id),
        ],
      );
      queryClient.invalidateQueries({
        queryKey: designSystemKeys.project(projectId),
      });
      queryClient.setQueryData<DesignSystemDetailResponse>(
        designSystemKeys.detail(response.designSystem.id),
        {
          designSystem: response.designSystem,
          sources: response.sources,
          conversation: response.conversation,
        },
      );
    },
  });
}

export function useDesignSystemDetail(designSystemId: string | null) {
  return useQuery<DesignSystemDetailResponse | null, Error>({
    queryKey: designSystemId ? designSystemKeys.detail(designSystemId) : [...designSystemKeys.all, "detail", "none"],
    queryFn: () => api.design.getDesignSystem(designSystemId!),
    enabled: !!designSystemId,
    staleTime: 10 * 1000,
  });
}
