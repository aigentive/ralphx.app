/**
 * useArtifacts hooks - TanStack Query wrappers for artifact operations
 *
 * Provides hooks for:
 * - useArtifacts: Fetch all artifacts, optionally filtered by type
 * - useArtifact: Fetch a single artifact by ID
 * - useArtifactsByBucket: Fetch artifacts in a specific bucket
 * - useArtifactsByTask: Fetch artifacts associated with a task
 * - useBuckets: Fetch all artifact buckets
 * - useArtifactRelations: Fetch relations for an artifact
 * - Mutation hooks for CRUD operations
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import type {
  ArtifactResponse,
  BucketResponse,
  ArtifactRelationResponse,
  CreateArtifactInput,
  UpdateArtifactInput,
  CreateBucketInput,
  AddRelationInput,
} from "@/api/artifacts";

// ============================================================================
// Query Keys
// ============================================================================

/**
 * Query key factory for artifacts
 */
export const artifactKeys = {
  all: ["artifacts"] as const,
  lists: () => [...artifactKeys.all, "list"] as const,
  list: (type?: string) => [...artifactKeys.lists(), type] as const,
  details: () => [...artifactKeys.all, "detail"] as const,
  detail: (id: string) => [...artifactKeys.details(), id] as const,
  byBucket: (bucketId: string) => [...artifactKeys.all, "byBucket", bucketId] as const,
  byTask: (taskId: string) => [...artifactKeys.all, "byTask", taskId] as const,
  buckets: () => [...artifactKeys.all, "buckets"] as const,
  relations: (artifactId: string) =>
    [...artifactKeys.all, "relations", artifactId] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Hook to fetch all artifacts, optionally filtered by type
 *
 * @param artifactType - Optional artifact type to filter by
 * @returns TanStack Query result with artifacts array
 *
 * @example
 * ```tsx
 * const { data: artifacts } = useArtifacts("prd");
 * return <ArtifactList artifacts={artifacts} />;
 * ```
 */
export function useArtifacts(artifactType?: string) {
  return useQuery<ArtifactResponse[], Error>({
    queryKey: artifactKeys.list(artifactType),
    queryFn: () => api.artifacts.getArtifacts(artifactType),
    staleTime: 30 * 1000, // 30 seconds
  });
}

/**
 * Hook to fetch a single artifact by ID
 *
 * @param id - The artifact ID to fetch
 * @returns TanStack Query result with artifact data or null
 */
export function useArtifact(id: string) {
  return useQuery<ArtifactResponse | null, Error>({
    queryKey: artifactKeys.detail(id),
    queryFn: () => api.artifacts.getArtifact(id),
    enabled: !!id,
    staleTime: 30 * 1000, // 30 seconds
  });
}

/**
 * Hook to fetch artifacts in a specific bucket
 *
 * @param bucketId - The bucket ID to fetch artifacts from
 * @returns TanStack Query result with artifacts array
 */
export function useArtifactsByBucket(bucketId: string) {
  return useQuery<ArtifactResponse[], Error>({
    queryKey: artifactKeys.byBucket(bucketId),
    queryFn: () => api.artifacts.getArtifactsByBucket(bucketId),
    enabled: !!bucketId,
    staleTime: 30 * 1000, // 30 seconds
  });
}

/**
 * Hook to fetch artifacts associated with a task
 *
 * @param taskId - The task ID to fetch artifacts for
 * @returns TanStack Query result with artifacts array
 */
export function useArtifactsByTask(taskId: string) {
  return useQuery<ArtifactResponse[], Error>({
    queryKey: artifactKeys.byTask(taskId),
    queryFn: () => api.artifacts.getArtifactsByTask(taskId),
    enabled: !!taskId,
    staleTime: 30 * 1000, // 30 seconds
  });
}

/**
 * Hook to fetch all artifact buckets
 *
 * @returns TanStack Query result with buckets array
 */
export function useBuckets() {
  return useQuery<BucketResponse[], Error>({
    queryKey: artifactKeys.buckets(),
    queryFn: api.artifacts.getBuckets,
    staleTime: 60 * 1000, // 1 minute (buckets rarely change)
  });
}

/**
 * Hook to fetch relations for an artifact
 *
 * @param artifactId - The artifact ID to fetch relations for
 * @returns TanStack Query result with relations array
 */
export function useArtifactRelations(artifactId: string) {
  return useQuery<ArtifactRelationResponse[], Error>({
    queryKey: artifactKeys.relations(artifactId),
    queryFn: () => api.artifacts.getArtifactRelations(artifactId),
    enabled: !!artifactId,
    staleTime: 30 * 1000, // 30 seconds
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Hook to create a new artifact
 *
 * @returns TanStack Mutation for creating artifacts
 */
export function useCreateArtifact() {
  const queryClient = useQueryClient();

  return useMutation<ArtifactResponse, Error, CreateArtifactInput>({
    mutationFn: api.artifacts.createArtifact,
    onSuccess: (artifact) => {
      queryClient.invalidateQueries({ queryKey: artifactKeys.lists() });
      if (artifact.bucket_id) {
        queryClient.invalidateQueries({
          queryKey: artifactKeys.byBucket(artifact.bucket_id),
        });
      }
      if (artifact.task_id) {
        queryClient.invalidateQueries({
          queryKey: artifactKeys.byTask(artifact.task_id),
        });
      }
    },
  });
}

/**
 * Hook to update an existing artifact
 *
 * @returns TanStack Mutation for updating artifacts
 */
export function useUpdateArtifact() {
  const queryClient = useQueryClient();

  return useMutation<
    ArtifactResponse,
    Error,
    { id: string; input: UpdateArtifactInput }
  >({
    mutationFn: ({ id, input }) => api.artifacts.updateArtifact(id, input),
    onSuccess: (artifact, { id }) => {
      queryClient.invalidateQueries({ queryKey: artifactKeys.lists() });
      queryClient.invalidateQueries({ queryKey: artifactKeys.detail(id) });
      if (artifact.bucket_id) {
        queryClient.invalidateQueries({
          queryKey: artifactKeys.byBucket(artifact.bucket_id),
        });
      }
      if (artifact.task_id) {
        queryClient.invalidateQueries({
          queryKey: artifactKeys.byTask(artifact.task_id),
        });
      }
    },
  });
}

/**
 * Hook to delete an artifact
 *
 * @returns TanStack Mutation for deleting artifacts
 */
export function useDeleteArtifact() {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: api.artifacts.deleteArtifact,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: artifactKeys.all });
    },
  });
}

/**
 * Hook to create a new bucket
 *
 * @returns TanStack Mutation for creating buckets
 */
export function useCreateBucket() {
  const queryClient = useQueryClient();

  return useMutation<BucketResponse, Error, CreateBucketInput>({
    mutationFn: api.artifacts.createBucket,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: artifactKeys.buckets() });
    },
  });
}

/**
 * Hook to add a relation between artifacts
 *
 * @returns TanStack Mutation for adding artifact relations
 */
export function useAddArtifactRelation() {
  const queryClient = useQueryClient();

  return useMutation<ArtifactRelationResponse, Error, AddRelationInput>({
    mutationFn: api.artifacts.addArtifactRelation,
    onSuccess: (_, input) => {
      queryClient.invalidateQueries({
        queryKey: artifactKeys.relations(input.from_artifact_id),
      });
      queryClient.invalidateQueries({
        queryKey: artifactKeys.relations(input.to_artifact_id),
      });
    },
  });
}
