/**
 * useMethodologies hooks - TanStack Query wrappers for methodology operations
 *
 * Provides hooks for:
 * - useMethodologies: Fetch all available methodologies
 * - useActiveMethodology: Fetch the currently active methodology
 * - useActivateMethodology: Activate a methodology
 * - useDeactivateMethodology: Deactivate a methodology
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import type {
  MethodologyResponse,
  MethodologyActivationResponse,
} from "@/api/methodologies";
import { workflowKeys } from "./useWorkflows";

// ============================================================================
// Query Keys
// ============================================================================

/**
 * Query key factory for methodologies
 */
export const methodologyKeys = {
  all: ["methodologies"] as const,
  lists: () => [...methodologyKeys.all, "list"] as const,
  active: () => [...methodologyKeys.all, "active"] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Hook to fetch all available methodologies
 *
 * @returns TanStack Query result with methodologies array
 *
 * @example
 * ```tsx
 * const { data: methodologies } = useMethodologies();
 * return <MethodologyBrowser methodologies={methodologies} />;
 * ```
 */
export function useMethodologies() {
  return useQuery<MethodologyResponse[], Error>({
    queryKey: methodologyKeys.lists(),
    queryFn: api.methodologies.getAll,
    staleTime: 60 * 1000, // 1 minute
  });
}

/**
 * Hook to fetch the currently active methodology
 *
 * @returns TanStack Query result with active methodology or null
 *
 * @example
 * ```tsx
 * const { data: active } = useActiveMethodology();
 * if (active) {
 *   return <MethodologyConfig methodology={active} />;
 * }
 * return <NoActiveMethodology />;
 * ```
 */
export function useActiveMethodology() {
  return useQuery<MethodologyResponse | null, Error>({
    queryKey: methodologyKeys.active(),
    queryFn: api.methodologies.getActive,
    staleTime: 30 * 1000, // 30 seconds
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Hook to activate a methodology
 *
 * When activated, the methodology's workflow becomes active and its
 * agent profiles and skills become available.
 *
 * @returns TanStack Mutation for activating methodologies
 *
 * @example
 * ```tsx
 * const { mutateAsync, isPending } = useActivateMethodology();
 *
 * const handleActivate = async (methodologyId: string) => {
 *   const { workflow, agent_profiles } = await mutateAsync(methodologyId);
 *   toast.success(`Activated ${workflow.name}`);
 * };
 * ```
 */
export function useActivateMethodology() {
  const queryClient = useQueryClient();

  return useMutation<MethodologyActivationResponse, Error, string>({
    mutationFn: api.methodologies.activate,
    onSuccess: () => {
      // Invalidate methodology queries
      queryClient.invalidateQueries({ queryKey: methodologyKeys.all });
      // Invalidate workflow queries since active workflow changes
      queryClient.invalidateQueries({ queryKey: workflowKeys.lists() });
      queryClient.invalidateQueries({ queryKey: workflowKeys.activeColumns() });
    },
  });
}

/**
 * Hook to deactivate a methodology
 *
 * When deactivated, the default workflow becomes active and the
 * methodology's agent profiles and skills are removed.
 *
 * @returns TanStack Mutation for deactivating methodologies
 *
 * @example
 * ```tsx
 * const { mutateAsync, isPending } = useDeactivateMethodology();
 *
 * const handleDeactivate = async (methodologyId: string) => {
 *   await mutateAsync(methodologyId);
 *   toast.success('Returned to default workflow');
 * };
 * ```
 */
export function useDeactivateMethodology() {
  const queryClient = useQueryClient();

  return useMutation<MethodologyResponse, Error, string>({
    mutationFn: api.methodologies.deactivate,
    onSuccess: () => {
      // Invalidate methodology queries
      queryClient.invalidateQueries({ queryKey: methodologyKeys.all });
      // Invalidate workflow queries since active workflow changes
      queryClient.invalidateQueries({ queryKey: workflowKeys.lists() });
      queryClient.invalidateQueries({ queryKey: workflowKeys.activeColumns() });
    },
  });
}
