/**
 * Mock Artifacts API
 *
 * Mirrors the interface of src/api/artifacts.ts with mock implementations.
 */

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
// Mock Data
// ============================================================================

const mockArtifacts: ArtifactResponse[] = [
  {
    id: "artifact-prd-001",
    name: "Feature PRD: User Authentication",
    artifact_type: "prd",
    content_type: "inline",
    content: "# User Authentication\n\n## Overview\nImplement secure user authentication...",
    created_at: new Date().toISOString(),
    created_by: "developer",
    version: 1,
    bucket_id: "bucket-prd-library",
    task_id: null,
    process_id: null,
    derived_from: [],
  },
  {
    id: "artifact-spec-001",
    name: "API Specification",
    artifact_type: "specification",
    content_type: "inline",
    content: "# API Specification\n\n## Endpoints\n/api/v1/auth...",
    created_at: new Date().toISOString(),
    created_by: "developer",
    version: 1,
    bucket_id: "bucket-research-outputs",
    task_id: "task-001",
    process_id: null,
    derived_from: ["artifact-prd-001"],
  },
];

const mockBuckets: BucketResponse[] = [
  {
    id: "bucket-research-outputs",
    name: "Research Outputs",
    accepted_types: ["specification", "research_document", "findings"],
    writers: ["researcher", "developer"],
    readers: ["*"],
    is_system: true,
  },
  {
    id: "bucket-prd-library",
    name: "PRD Library",
    accepted_types: ["prd"],
    writers: ["developer", "pm"],
    readers: ["*"],
    is_system: true,
  },
  {
    id: "bucket-work-context",
    name: "Work Context",
    accepted_types: ["context", "previous_work"],
    writers: ["*"],
    readers: ["*"],
    is_system: true,
  },
];

const mockRelations: ArtifactRelationResponse[] = [
  {
    id: "relation-001",
    from_artifact_id: "artifact-spec-001",
    to_artifact_id: "artifact-prd-001",
    relation_type: "derived_from",
  },
];

// ============================================================================
// Mock Artifacts API
// ============================================================================

export const mockArtifactsApi = {
  /**
   * Get all artifacts, optionally filtered by type
   */
  getArtifacts: async (artifactType?: string): Promise<ArtifactResponse[]> => {
    if (artifactType) {
      return mockArtifacts.filter((a) => a.artifact_type === artifactType);
    }
    return mockArtifacts;
  },

  /**
   * Get a single artifact by ID
   */
  getArtifact: async (id: string): Promise<ArtifactResponse | null> => {
    return mockArtifacts.find((a) => a.id === id) ?? null;
  },

  /**
   * Create a new artifact
   */
  createArtifact: async (input: CreateArtifactInput): Promise<ArtifactResponse> => {
    const newArtifact: ArtifactResponse = {
      id: `artifact-${Date.now()}`,
      name: input.name,
      artifact_type: input.artifact_type,
      content_type: input.content_type,
      content: input.content,
      created_at: new Date().toISOString(),
      created_by: input.created_by,
      version: 1,
      bucket_id: input.bucket_id ?? null,
      task_id: input.task_id ?? null,
      process_id: input.process_id ?? null,
      derived_from: input.derived_from ?? [],
    };
    return newArtifact;
  },

  /**
   * Update an existing artifact
   */
  updateArtifact: async (id: string, input: UpdateArtifactInput): Promise<ArtifactResponse> => {
    const artifact = mockArtifacts.find((a) => a.id === id);
    if (!artifact) {
      throw new Error(`Artifact not found: ${id}`);
    }
    return {
      ...artifact,
      name: input.name ?? artifact.name,
      content_type: input.content_type ?? artifact.content_type,
      content: input.content ?? artifact.content,
      bucket_id: input.bucket_id ?? artifact.bucket_id,
      version: artifact.version + 1,
    };
  },

  /**
   * Delete an artifact by ID
   */
  deleteArtifact: async (_id: string): Promise<void> => {
    // No-op in mock mode
  },

  /**
   * Get all artifacts in a specific bucket
   */
  getArtifactsByBucket: async (bucketId: string): Promise<ArtifactResponse[]> => {
    return mockArtifacts.filter((a) => a.bucket_id === bucketId);
  },

  /**
   * Get all artifacts associated with a task
   */
  getArtifactsByTask: async (taskId: string): Promise<ArtifactResponse[]> => {
    return mockArtifacts.filter((a) => a.task_id === taskId);
  },

  /**
   * Get all artifact buckets
   */
  getBuckets: async (): Promise<BucketResponse[]> => {
    return mockBuckets;
  },

  /**
   * Create a new artifact bucket
   */
  createBucket: async (input: CreateBucketInput): Promise<BucketResponse> => {
    const newBucket: BucketResponse = {
      id: `bucket-${Date.now()}`,
      name: input.name,
      accepted_types: input.accepted_types ?? [],
      writers: input.writers ?? [],
      readers: input.readers ?? [],
      is_system: false,
    };
    return newBucket;
  },

  /**
   * Get the system buckets
   */
  getSystemBuckets: async (): Promise<BucketResponse[]> => {
    return mockBuckets.filter((b) => b.is_system);
  },

  /**
   * Add a relation between two artifacts
   */
  addArtifactRelation: async (input: AddRelationInput): Promise<ArtifactRelationResponse> => {
    const newRelation: ArtifactRelationResponse = {
      id: `relation-${Date.now()}`,
      from_artifact_id: input.from_artifact_id,
      to_artifact_id: input.to_artifact_id,
      relation_type: input.relation_type,
    };
    return newRelation;
  },

  /**
   * Get all relations for an artifact
   */
  getArtifactRelations: async (artifactId: string): Promise<ArtifactRelationResponse[]> => {
    return mockRelations.filter(
      (r) => r.from_artifact_id === artifactId || r.to_artifact_id === artifactId
    );
  },
} as const;

// Legacy export for backward compatibility (will be removed)
export const mockArtifactApi = {
  get: mockArtifactsApi.getArtifact,
  getAtVersion: async (_artifactId: string, _version: number): Promise<ArtifactResponse | null> => {
    return null;
  },
  getByTask: mockArtifactsApi.getArtifactsByTask,
  getByBucket: mockArtifactsApi.getArtifactsByBucket,
} as const;
