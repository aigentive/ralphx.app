/**
 * Tauri API wrappers
 *
 * Type-safe wrappers for Tauri commands with Zod validation.
 */

// Workflows API
export {
  getWorkflows,
  getWorkflow,
  createWorkflow,
  updateWorkflow,
  deleteWorkflow,
  setDefaultWorkflow,
  getActiveWorkflowColumns,
  getBuiltinWorkflows,
  WorkflowResponseSchema,
  WorkflowColumnResponseSchema,
  WorkflowColumnInputSchema,
  CreateWorkflowInputSchema,
  UpdateWorkflowInputSchema,
  type WorkflowResponse,
  type WorkflowColumnResponse,
  type WorkflowColumnInput,
  type CreateWorkflowInput,
  type UpdateWorkflowInput,
} from "./workflows";

// Artifacts API
export {
  getArtifacts,
  getArtifact,
  createArtifact,
  updateArtifact,
  deleteArtifact,
  getArtifactsByBucket,
  getArtifactsByTask,
  getBuckets,
  createBucket,
  getSystemBuckets,
  addArtifactRelation,
  getArtifactRelations,
  ArtifactResponseSchema,
  BucketResponseSchema,
  ArtifactRelationResponseSchema,
  ContentTypeSchema,
  CreateArtifactInputSchema,
  UpdateArtifactInputSchema,
  CreateBucketInputSchema,
  AddRelationInputSchema,
  type ArtifactResponse,
  type BucketResponse,
  type ArtifactRelationResponse,
  type ContentType,
  type CreateArtifactInput,
  type UpdateArtifactInput,
  type CreateBucketInput,
  type AddRelationInput,
} from "./artifacts";
