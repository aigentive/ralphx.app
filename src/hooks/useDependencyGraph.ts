/**
 * useDependencyGraph hooks - TanStack Query wrappers for dependency graph
 *
 * Provides hooks for fetching the dependency graph and managing dependencies
 * between proposals within ideation sessions.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { ideationApi, type DependencyGraphResponse } from "@/api/ideation";
import { proposalKeys } from "./useProposals";
import { ideationKeys } from "./useIdeation";

/**
 * Query key factory for dependencies
 */
export const dependencyKeys = {
  all: ["dependencies"] as const,
  graphs: () => [...dependencyKeys.all, "graph"] as const,
  graph: (sessionId: string) => [...dependencyKeys.graphs(), sessionId] as const,
};

/**
 * Hook to fetch the dependency graph for an ideation session
 *
 * @param sessionId - The session ID to fetch the graph for
 * @returns TanStack Query result with dependency graph
 *
 * @example
 * ```tsx
 * const { data: graph, isLoading, error } = useDependencyGraph("session-123");
 *
 * if (isLoading) return <Loading />;
 * if (error) return <Error message={error.message} />;
 *
 * return (
 *   <div>
 *     <h3>Dependencies</h3>
 *     <p>Nodes: {graph?.nodes.length}</p>
 *     <p>Has cycles: {graph?.hasCycles ? 'Yes' : 'No'}</p>
 *     {graph?.hasCycles && (
 *       <Warning>Circular dependencies detected!</Warning>
 *     )}
 *     <CriticalPath path={graph?.criticalPath ?? []} />
 *   </div>
 * );
 * ```
 */
export function useDependencyGraph(sessionId: string) {
  return useQuery<DependencyGraphResponse, Error>({
    queryKey: dependencyKeys.graph(sessionId),
    queryFn: () => ideationApi.dependencies.analyze(sessionId),
    enabled: Boolean(sessionId),
  });
}

/**
 * Input for dependency mutations
 */
interface DependencyInput {
  proposalId: string;
  dependsOnId: string;
}

/**
 * Hook for dependency management mutations
 *
 * @returns Object with mutation functions for adding/removing dependencies
 *
 * @example
 * ```tsx
 * const { addDependency, removeDependency } = useDependencyMutations();
 *
 * // Add a dependency
 * const handleAdd = async () => {
 *   await addDependency.mutateAsync({
 *     proposalId: "proposal-2",
 *     dependsOnId: "proposal-1",
 *   });
 *   toast.success("Dependency added");
 * };
 *
 * // Remove a dependency
 * const handleRemove = async () => {
 *   await removeDependency.mutateAsync({
 *     proposalId: "proposal-2",
 *     dependsOnId: "proposal-1",
 *   });
 *   toast.success("Dependency removed");
 * };
 * ```
 */
export function useDependencyMutations() {
  const queryClient = useQueryClient();

  const addDependency = useMutation<void, Error, DependencyInput>({
    mutationFn: ({ proposalId, dependsOnId }) =>
      ideationApi.dependencies.add(proposalId, dependsOnId),
    onSuccess: () => {
      // Invalidate all dependency graphs (we don't know the session from the mutation)
      queryClient.invalidateQueries({
        queryKey: dependencyKeys.graphs(),
      });
      // Also invalidate proposals since they may show dependency info
      queryClient.invalidateQueries({
        queryKey: proposalKeys.lists(),
      });
      // And session data
      queryClient.invalidateQueries({
        queryKey: ideationKeys.sessionDetails(),
      });
    },
  });

  const removeDependency = useMutation<void, Error, DependencyInput>({
    mutationFn: ({ proposalId, dependsOnId }) =>
      ideationApi.dependencies.remove(proposalId, dependsOnId),
    onSuccess: () => {
      // Invalidate all dependency graphs
      queryClient.invalidateQueries({
        queryKey: dependencyKeys.graphs(),
      });
      // Also invalidate proposals since they may show dependency info
      queryClient.invalidateQueries({
        queryKey: proposalKeys.lists(),
      });
      // And session data
      queryClient.invalidateQueries({
        queryKey: ideationKeys.sessionDetails(),
      });
    },
  });

  return {
    addDependency,
    removeDependency,
  };
}
