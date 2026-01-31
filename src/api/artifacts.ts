/**
 * Artifacts API Module
 *
 * Provides a centralized API wrapper for artifact, bucket, and relation operations.
 * This module mirrors src/lib/api/artifacts.ts but follows the domain API pattern
 * used by methodologies.ts and other centralized modules.
 */

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { ArtifactTypeSchema, ArtifactRelationTypeSchema } from "@/types/artifact";

// ============================================================================
// Response Schemas (matching Rust response structures)
// ============================================================================

/**
 * Content type for artifacts - either inline text or file path
 */
export const ContentTypeSchema = z.enum(["inline", "file"]);
export type ContentType = z.infer<typeof ContentTypeSchema>;

/**
 * Schema for artifact response from Rust backend
 * Note: Uses snake_case to match Rust serde serialization
 */
export const ArtifactResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  artifact_type: ArtifactTypeSchema,
  content_type: ContentTypeSchema,
  content: z.string(),
  created_at: z.string(),
  created_by: z.string(),
  version: z.number().int().positive(),
  bucket_id: z.string().nullable(),
  task_id: z.string().nullable(),
  process_id: z.string().nullable(),
  derived_from: z.array(z.string()),
});

export type ArtifactResponse = z.infer<typeof ArtifactResponseSchema>;

/**
 * Schema for bucket response from Rust backend
 */
export const BucketResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  accepted_types: z.array(ArtifactTypeSchema),
  writers: z.array(z.string()),
  readers: z.array(z.string()),
  is_system: z.boolean(),
});

export type BucketResponse = z.infer<typeof BucketResponseSchema>;

/**
 * Schema for artifact relation response from Rust backend
 */
export const ArtifactRelationResponseSchema = z.object({
  id: z.string(),
  from_artifact_id: z.string(),
  to_artifact_id: z.string(),
  relation_type: ArtifactRelationTypeSchema,
});

export type ArtifactRelationResponse = z.infer<typeof ArtifactRelationResponseSchema>;

/**
 * Schema for array of artifact responses
 */
const ArtifactListResponseSchema = z.array(ArtifactResponseSchema);

/**
 * Schema for array of bucket responses
 */
const BucketListResponseSchema = z.array(BucketResponseSchema);

/**
 * Schema for array of relation responses
 */
const RelationListResponseSchema = z.array(ArtifactRelationResponseSchema);

// ============================================================================
// Input Schemas (for validating client-side input before sending)
// ============================================================================

/**
 * Schema for creating a new artifact
 */
export const CreateArtifactInputSchema = z.object({
  name: z.string().min(1),
  artifact_type: ArtifactTypeSchema,
  content_type: ContentTypeSchema,
  content: z.string(),
  created_by: z.string().min(1),
  bucket_id: z.string().optional(),
  task_id: z.string().optional(),
  process_id: z.string().optional(),
  derived_from: z.array(z.string()).optional(),
});

export type CreateArtifactInput = z.infer<typeof CreateArtifactInputSchema>;

/**
 * Schema for updating an existing artifact (all fields optional)
 */
export const UpdateArtifactInputSchema = z.object({
  name: z.string().min(1).optional(),
  content_type: ContentTypeSchema.optional(),
  content: z.string().optional(),
  bucket_id: z.string().optional(),
});

export type UpdateArtifactInput = z.infer<typeof UpdateArtifactInputSchema>;

/**
 * Schema for creating a new bucket
 */
export const CreateBucketInputSchema = z.object({
  name: z.string().min(1),
  accepted_types: z.array(ArtifactTypeSchema).optional(),
  writers: z.array(z.string()).optional(),
  readers: z.array(z.string()).optional(),
});

export type CreateBucketInput = z.infer<typeof CreateBucketInputSchema>;

/**
 * Schema for adding an artifact relation
 */
export const AddRelationInputSchema = z.object({
  from_artifact_id: z.string(),
  to_artifact_id: z.string(),
  relation_type: ArtifactRelationTypeSchema,
});

export type AddRelationInput = z.infer<typeof AddRelationInputSchema>;

// ============================================================================
// Artifacts API Object
// ============================================================================

/**
 * Artifacts API object containing all typed Tauri command wrappers
 */
export const artifactsApi = {
  /**
   * Get all artifacts, optionally filtered by type
   * @param artifactType Optional artifact type filter
   * @returns Array of artifact responses
   */
  getArtifacts: async (artifactType?: string): Promise<ArtifactResponse[]> => {
    const result = await invoke("get_artifacts", {
      artifact_type: artifactType ?? null,
    });
    return ArtifactListResponseSchema.parse(result);
  },

  /**
   * Get a single artifact by ID
   * @param id The artifact ID
   * @returns The artifact or null if not found
   */
  getArtifact: async (id: string): Promise<ArtifactResponse | null> => {
    const result = await invoke("get_artifact", { id });
    return ArtifactResponseSchema.nullable().parse(result);
  },

  /**
   * Create a new artifact
   * @param input Artifact creation data
   * @returns The created artifact
   * @throws ZodError if input validation fails
   */
  createArtifact: async (input: CreateArtifactInput): Promise<ArtifactResponse> => {
    const validatedInput = CreateArtifactInputSchema.parse(input);
    const result = await invoke("create_artifact", { input: validatedInput });
    return ArtifactResponseSchema.parse(result);
  },

  /**
   * Update an existing artifact
   * @param id The artifact ID
   * @param input Partial artifact data to update
   * @returns The updated artifact
   * @throws ZodError if input validation fails
   */
  updateArtifact: async (id: string, input: UpdateArtifactInput): Promise<ArtifactResponse> => {
    const validatedInput = UpdateArtifactInputSchema.parse(input);
    const result = await invoke("update_artifact", { id, input: validatedInput });
    return ArtifactResponseSchema.parse(result);
  },

  /**
   * Delete an artifact by ID
   * @param id The artifact ID
   */
  deleteArtifact: async (id: string): Promise<void> => {
    await invoke("delete_artifact", { id });
  },

  /**
   * Get all artifacts in a specific bucket
   * @param bucketId The bucket ID
   * @returns Array of artifact responses
   */
  getArtifactsByBucket: async (bucketId: string): Promise<ArtifactResponse[]> => {
    const result = await invoke("get_artifacts_by_bucket", { bucket_id: bucketId });
    return ArtifactListResponseSchema.parse(result);
  },

  /**
   * Get all artifacts associated with a task
   * @param taskId The task ID
   * @returns Array of artifact responses
   */
  getArtifactsByTask: async (taskId: string): Promise<ArtifactResponse[]> => {
    const result = await invoke("get_artifacts_by_task", { task_id: taskId });
    return ArtifactListResponseSchema.parse(result);
  },

  /**
   * Get all artifact buckets
   * @returns Array of bucket responses
   */
  getBuckets: async (): Promise<BucketResponse[]> => {
    const result = await invoke("get_buckets", {});
    return BucketListResponseSchema.parse(result);
  },

  /**
   * Create a new artifact bucket
   * @param input Bucket creation data
   * @returns The created bucket
   * @throws ZodError if input validation fails
   */
  createBucket: async (input: CreateBucketInput): Promise<BucketResponse> => {
    const validatedInput = CreateBucketInputSchema.parse(input);
    const result = await invoke("create_bucket", { input: validatedInput });
    return BucketResponseSchema.parse(result);
  },

  /**
   * Get the system buckets (research-outputs, work-context, code-changes, prd-library)
   * @returns Array of system bucket responses
   */
  getSystemBuckets: async (): Promise<BucketResponse[]> => {
    const result = await invoke("get_system_buckets", {});
    return BucketListResponseSchema.parse(result);
  },

  /**
   * Add a relation between two artifacts
   * @param input Relation data
   * @returns The created relation
   * @throws ZodError if input validation fails
   */
  addArtifactRelation: async (input: AddRelationInput): Promise<ArtifactRelationResponse> => {
    const validatedInput = AddRelationInputSchema.parse(input);
    const result = await invoke("add_artifact_relation", { input: validatedInput });
    return ArtifactRelationResponseSchema.parse(result);
  },

  /**
   * Get all relations for an artifact
   * @param artifactId The artifact ID
   * @returns Array of relation responses
   */
  getArtifactRelations: async (artifactId: string): Promise<ArtifactRelationResponse[]> => {
    const result = await invoke("get_artifact_relations", { artifact_id: artifactId });
    return RelationListResponseSchema.parse(result);
  },
} as const;
