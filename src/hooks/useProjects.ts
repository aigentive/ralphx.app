/**
 * useProjects hooks - TanStack Query wrappers for project fetching
 *
 * Provides hooks for fetching all projects and individual project details
 * with automatic caching, refetching, and error handling.
 */

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import type { Project } from "@/types/project";

/**
 * Query key factory for projects
 */
export const projectKeys = {
  all: ["projects"] as const,
  lists: () => [...projectKeys.all, "list"] as const,
  list: () => [...projectKeys.lists()] as const,
  details: () => [...projectKeys.all, "detail"] as const,
  detail: (projectId: string) => [...projectKeys.details(), projectId] as const,
};

/**
 * Hook to fetch all projects
 *
 * @returns TanStack Query result with projects data
 *
 * @example
 * ```tsx
 * const { data: projects, isLoading, error } = useProjects();
 *
 * if (isLoading) return <Loading />;
 * if (error) return <Error message={error.message} />;
 * return <ProjectList projects={projects} />;
 * ```
 */
export function useProjects() {
  return useQuery<Project[], Error>({
    queryKey: projectKeys.list(),
    queryFn: () => api.projects.list(),
  });
}

/**
 * Hook to fetch a single project by ID
 *
 * @param projectId - The project ID to fetch
 * @returns TanStack Query result with project data
 *
 * @example
 * ```tsx
 * const { data: project, isLoading, error } = useProject("project-123");
 *
 * if (isLoading) return <Loading />;
 * if (error) return <Error message={error.message} />;
 * return <ProjectDetail project={project} />;
 * ```
 */
export function useProject(projectId: string) {
  return useQuery<Project, Error>({
    queryKey: projectKeys.detail(projectId),
    queryFn: () => api.projects.get(projectId),
  });
}
