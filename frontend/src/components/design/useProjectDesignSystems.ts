import { useMemo } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";

import type {
  CreateDesignStyleguideFeedbackInput,
  CreateDesignStyleguideFeedbackResponse,
  DesignStyleguideItemResponse,
  DesignStyleguidePreviewResponse,
  DesignStyleguideViewModelResponse,
  DesignSystemDetailResponse,
  DesignSystemResponse,
  ExportDesignSystemPackageResponse,
  GenerateDesignSystemStyleguideResponse,
  ImportDesignSystemPackageInput,
  ImportDesignSystemPackageResponse,
} from "@/api/design";
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
  styleguideItems: (designSystemId: string) =>
    [...designSystemKeys.all, "styleguide-items", designSystemId] as const,
  styleguideViewModel: (designSystemId: string) =>
    [...designSystemKeys.all, "styleguide-view-model", designSystemId] as const,
  styleguidePreview: (designSystemId: string, previewArtifactId: string) =>
    [...designSystemKeys.all, "styleguide-preview", designSystemId, previewArtifactId] as const,
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
    mutationFn: (input) => api.design.createDesignSystem(input),
    onSuccess: (response) => {
      const projectId = response.designSystem.primaryProjectId;
      queryClient.setQueryData<DesignSystemResponse[]>(
        designSystemKeys.projectList(projectId, false),
        (current = []) => [
          { ...response.designSystem, sourceCount: response.sources.length },
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

export function useDesignStyleguideItems(designSystemId: string | null) {
  return useQuery<DesignStyleguideItemResponse[], Error>({
    queryKey: designSystemId
      ? designSystemKeys.styleguideItems(designSystemId)
      : [...designSystemKeys.all, "styleguide-items", "none"],
    queryFn: () => api.design.listStyleguideItems(designSystemId!),
    enabled: !!designSystemId,
    staleTime: 10 * 1000,
  });
}

export function useDesignStyleguideViewModel(designSystemId: string | null) {
  return useQuery<DesignStyleguideViewModelResponse | null, Error>({
    queryKey: designSystemId
      ? designSystemKeys.styleguideViewModel(designSystemId)
      : [...designSystemKeys.all, "styleguide-view-model", "none"],
    queryFn: () => api.design.getStyleguideViewModel(designSystemId!),
    enabled: !!designSystemId,
    staleTime: 10 * 1000,
  });
}

export function useDesignStyleguidePreview(
  designSystemId: string | null,
  previewArtifactId: string | null,
) {
  return useQuery<DesignStyleguidePreviewResponse, Error>({
    queryKey: designSystemId && previewArtifactId
      ? designSystemKeys.styleguidePreview(designSystemId, previewArtifactId)
      : [...designSystemKeys.all, "styleguide-preview", "none"],
    queryFn: () => api.design.getStyleguidePreview(designSystemId!, previewArtifactId!),
    enabled: !!designSystemId && !!previewArtifactId,
    staleTime: 60 * 1000,
  });
}

export function useGenerateDesignSystemStyleguide() {
  const queryClient = useQueryClient();

  return useMutation<GenerateDesignSystemStyleguideResponse, Error, string>({
    mutationFn: (designSystemId) => api.design.generateStyleguide(designSystemId),
    onSuccess: (response) => {
      const projectId = response.designSystem.primaryProjectId;
      queryClient.setQueryData<DesignSystemResponse[]>(
        designSystemKeys.projectList(projectId, false),
        (current = []) => {
          const previous = current.find((system) => system.id === response.designSystem.id);
          return [
            {
              ...response.designSystem,
              sourceCount: previous?.sourceCount,
            },
            ...current.filter((system) => system.id !== response.designSystem.id),
          ];
        },
      );
      queryClient.setQueryData<DesignSystemDetailResponse | null>(
        designSystemKeys.detail(response.designSystem.id),
        (current) =>
          current
            ? {
                ...current,
                designSystem: response.designSystem,
              }
            : current,
      );
      queryClient.setQueryData<DesignStyleguideItemResponse[]>(
        designSystemKeys.styleguideItems(response.designSystem.id),
        response.items,
      );
      queryClient.invalidateQueries({
        queryKey: designSystemKeys.styleguideViewModel(response.designSystem.id),
      });
      queryClient.invalidateQueries({
        queryKey: designSystemKeys.project(projectId),
      });
    },
  });
}

export function useExportDesignSystemPackage() {
  return useMutation<ExportDesignSystemPackageResponse, Error, string>({
    mutationFn: (designSystemId) => api.design.exportPackage(designSystemId),
  });
}

export function useImportDesignSystemPackage() {
  const queryClient = useQueryClient();

  return useMutation<
    ImportDesignSystemPackageResponse,
    Error,
    ImportDesignSystemPackageInput
  >({
    mutationFn: (input) => api.design.importPackage(input),
    onSuccess: (response) => {
      const projectId = response.designSystem.primaryProjectId;
      queryClient.setQueryData<DesignSystemResponse[]>(
        designSystemKeys.projectList(projectId, false),
        (current = []) => [
          { ...response.designSystem, sourceCount: response.sources.length },
          ...current.filter((system) => system.id !== response.designSystem.id),
        ],
      );
      queryClient.setQueryData<DesignSystemDetailResponse>(
        designSystemKeys.detail(response.designSystem.id),
        {
          designSystem: response.designSystem,
          sources: response.sources,
          conversation: response.conversation,
        },
      );
      queryClient.setQueryData<DesignStyleguideItemResponse[]>(
        designSystemKeys.styleguideItems(response.designSystem.id),
        response.items,
      );
      queryClient.invalidateQueries({
        queryKey: designSystemKeys.styleguideViewModel(response.designSystem.id),
      });
      queryClient.invalidateQueries({
        queryKey: designSystemKeys.project(projectId),
      });
    },
  });
}

export function useApproveDesignStyleguideItem() {
  const queryClient = useQueryClient();

  return useMutation<
    DesignStyleguideItemResponse,
    Error,
    { designSystemId: string; itemId: string }
  >({
    mutationFn: ({ designSystemId, itemId }) =>
      api.design.approveStyleguideItem(designSystemId, itemId),
    onSuccess: (item) => {
      queryClient.invalidateQueries({
        queryKey: designSystemKeys.styleguideItems(item.designSystemId),
      });
    },
  });
}

export function useCreateDesignStyleguideFeedback() {
  const queryClient = useQueryClient();

  return useMutation<
    CreateDesignStyleguideFeedbackResponse,
    Error,
    CreateDesignStyleguideFeedbackInput
  >({
    mutationFn: (input) => api.design.createStyleguideFeedback(input),
    onSuccess: (response) => {
      queryClient.invalidateQueries({
        queryKey: designSystemKeys.styleguideItems(response.item.designSystemId),
      });
    },
  });
}
