// Tauri invoke wrappers for artifact system with type safety

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { Artifact } from "@/types/artifact";

// ============================================================================
// Response Schemas (matching Rust backend serialization with snake_case)
// ============================================================================

const ArtifactResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  artifact_type: z.string(),
  content_type: z.string(),
  content: z.string(),
  created_at: z.string(),
  created_by: z.string(),
  version: z.number(),
  bucket_id: z.string().nullable(),
  task_id: z.string().nullable(),
  process_id: z.string().nullable(),
  derived_from: z.array(z.string()),
});

type ArtifactResponse = z.infer<typeof ArtifactResponseSchema>;

// ============================================================================
// Transform Functions (snake_case -> camelCase)
// ============================================================================

function transformArtifact(raw: ArtifactResponse): Artifact {
  const content =
    raw.content_type === "inline"
      ? { type: "inline" as const, text: raw.content }
      : { type: "file" as const, path: raw.content };

  return {
    id: raw.id,
    type: raw.artifact_type as Artifact["type"],
    name: raw.name,
    content,
    metadata: {
      createdAt: raw.created_at,
      createdBy: raw.created_by,
      version: raw.version,
      taskId: raw.task_id ?? undefined,
      processId: raw.process_id ?? undefined,
    },
    derivedFrom: raw.derived_from,
    bucketId: raw.bucket_id ?? undefined,
  };
}

// ============================================================================
// Typed Invoke Helpers
// ============================================================================

async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

// ============================================================================
// API Object
// ============================================================================

/**
 * Artifact API wrappers for Tauri commands
 */
export const artifactApi = {
  /**
   * Get an artifact by ID
   * @param artifactId The artifact ID
   * @returns The artifact or null if not found
   */
  get: async (artifactId: string): Promise<Artifact | null> => {
    const raw = await typedInvoke(
      "get_artifact",
      { id: artifactId },
      ArtifactResponseSchema.nullable()
    );
    return raw ? transformArtifact(raw) : null;
  },

  /**
   * Get an artifact at a specific version
   * @param artifactId The artifact ID
   * @param version The version number to retrieve
   * @returns The artifact at the specified version or null if not found
   */
  getAtVersion: async (artifactId: string, version: number): Promise<Artifact | null> => {
    const raw = await typedInvoke(
      "get_artifact_at_version",
      { id: artifactId, version },
      ArtifactResponseSchema.nullable()
    );
    return raw ? transformArtifact(raw) : null;
  },

  /**
   * Get all artifacts for a task
   * @param taskId The task ID
   * @returns Array of artifacts
   */
  getByTask: async (taskId: string): Promise<Artifact[]> => {
    const raw = await typedInvoke(
      "get_artifacts_by_task",
      { task_id: taskId },
      z.array(ArtifactResponseSchema)
    );
    return raw.map(transformArtifact);
  },

  /**
   * Get all artifacts in a bucket
   * @param bucketId The bucket ID
   * @returns Array of artifacts
   */
  getByBucket: async (bucketId: string): Promise<Artifact[]> => {
    const raw = await typedInvoke(
      "get_artifacts_by_bucket",
      { bucket_id: bucketId },
      z.array(ArtifactResponseSchema)
    );
    return raw.map(transformArtifact);
  },
} as const;
