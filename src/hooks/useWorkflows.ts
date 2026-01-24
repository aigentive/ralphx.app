/**
 * useWorkflows hooks - TanStack Query wrappers for workflow operations
 *
 * Provides hooks for:
 * - useWorkflows: Fetch all workflows
 * - useWorkflow: Fetch a single workflow by ID
 * - useActiveWorkflowColumns: Fetch columns for the active workflow
 * - useCreateWorkflow: Create a new workflow
 * - useUpdateWorkflow: Update an existing workflow
 * - useDeleteWorkflow: Delete a workflow
 * - useSetDefaultWorkflow: Set a workflow as the default
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import * as workflowsApi from "@/lib/api/workflows";
import type {
  WorkflowResponse,
  WorkflowColumnResponse,
  CreateWorkflowInput,
  UpdateWorkflowInput,
} from "@/lib/api/workflows";

// ============================================================================
// Query Keys
// ============================================================================

/**
 * Query key factory for workflows
 */
export const workflowKeys = {
  all: ["workflows"] as const,
  lists: () => [...workflowKeys.all, "list"] as const,
  details: () => [...workflowKeys.all, "detail"] as const,
  detail: (id: string) => [...workflowKeys.details(), id] as const,
  activeColumns: () => [...workflowKeys.all, "activeColumns"] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Hook to fetch all workflows
 *
 * @returns TanStack Query result with workflows array
 *
 * @example
 * ```tsx
 * const { data: workflows, isLoading, error } = useWorkflows();
 *
 * if (isLoading) return <Spinner />;
 * if (error) return <Error message={error.message} />;
 * return <WorkflowList workflows={workflows} />;
 * ```
 */
export function useWorkflows() {
  return useQuery<WorkflowResponse[], Error>({
    queryKey: workflowKeys.lists(),
    queryFn: workflowsApi.getWorkflows,
    staleTime: 60 * 1000, // 1 minute
  });
}

/**
 * Hook to fetch a single workflow by ID
 *
 * @param id - The workflow ID to fetch
 * @returns TanStack Query result with workflow data or null
 *
 * @example
 * ```tsx
 * const { data: workflow, isLoading } = useWorkflow("workflow-123");
 *
 * if (isLoading) return <Spinner />;
 * if (!workflow) return <p>Workflow not found</p>;
 * return <WorkflowEditor workflow={workflow} />;
 * ```
 */
export function useWorkflow(id: string) {
  return useQuery<WorkflowResponse | null, Error>({
    queryKey: workflowKeys.detail(id),
    queryFn: () => workflowsApi.getWorkflow(id),
    enabled: !!id,
    staleTime: 60 * 1000, // 1 minute
  });
}

/**
 * Hook to fetch columns for the active/default workflow
 *
 * @returns TanStack Query result with workflow columns array
 *
 * @example
 * ```tsx
 * const { data: columns } = useActiveWorkflowColumns();
 *
 * return (
 *   <div>
 *     {columns?.map(col => <Column key={col.id} column={col} />)}
 *   </div>
 * );
 * ```
 */
export function useActiveWorkflowColumns() {
  return useQuery<WorkflowColumnResponse[], Error>({
    queryKey: workflowKeys.activeColumns(),
    queryFn: workflowsApi.getActiveWorkflowColumns,
    staleTime: 60 * 1000, // 1 minute
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Hook to create a new workflow
 *
 * @returns TanStack Mutation for creating workflows
 *
 * @example
 * ```tsx
 * const { mutateAsync, isPending } = useCreateWorkflow();
 *
 * const handleCreate = async () => {
 *   await mutateAsync({
 *     name: "My Workflow",
 *     columns: [{ id: "backlog", name: "Backlog", maps_to: "backlog" }],
 *   });
 * };
 * ```
 */
export function useCreateWorkflow() {
  const queryClient = useQueryClient();

  return useMutation<WorkflowResponse, Error, CreateWorkflowInput>({
    mutationFn: workflowsApi.createWorkflow,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: workflowKeys.lists() });
    },
  });
}

/**
 * Hook to update an existing workflow
 *
 * @returns TanStack Mutation for updating workflows
 *
 * @example
 * ```tsx
 * const { mutateAsync, isPending } = useUpdateWorkflow();
 *
 * const handleUpdate = async () => {
 *   await mutateAsync({
 *     id: "workflow-123",
 *     input: { name: "Updated Name" },
 *   });
 * };
 * ```
 */
export function useUpdateWorkflow() {
  const queryClient = useQueryClient();

  return useMutation<
    WorkflowResponse,
    Error,
    { id: string; input: UpdateWorkflowInput }
  >({
    mutationFn: ({ id, input }) => workflowsApi.updateWorkflow(id, input),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: workflowKeys.lists() });
      queryClient.invalidateQueries({ queryKey: workflowKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: workflowKeys.activeColumns() });
    },
  });
}

/**
 * Hook to delete a workflow
 *
 * @returns TanStack Mutation for deleting workflows
 *
 * @example
 * ```tsx
 * const { mutateAsync, isPending } = useDeleteWorkflow();
 *
 * const handleDelete = async () => {
 *   await mutateAsync("workflow-123");
 * };
 * ```
 */
export function useDeleteWorkflow() {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: workflowsApi.deleteWorkflow,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: workflowKeys.lists() });
      queryClient.invalidateQueries({ queryKey: workflowKeys.activeColumns() });
    },
  });
}

/**
 * Hook to set a workflow as the default
 *
 * @returns TanStack Mutation for setting default workflow
 *
 * @example
 * ```tsx
 * const { mutateAsync, isPending } = useSetDefaultWorkflow();
 *
 * const handleSetDefault = async () => {
 *   await mutateAsync("workflow-123");
 * };
 * ```
 */
export function useSetDefaultWorkflow() {
  const queryClient = useQueryClient();

  return useMutation<WorkflowResponse, Error, string>({
    mutationFn: workflowsApi.setDefaultWorkflow,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: workflowKeys.lists() });
      queryClient.invalidateQueries({ queryKey: workflowKeys.activeColumns() });
    },
  });
}
