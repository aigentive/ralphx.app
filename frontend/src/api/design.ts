import { z } from "zod";

import { typedInvoke } from "@/lib/tauri";

export const DesignSystemStatusSchema = z.enum([
  "draft",
  "analyzing",
  "schema_ready",
  "ready",
  "updating",
  "failed",
  "archived",
]);

export const DesignSourceRefSchema = z.object({
  project_id: z.string().min(1),
  path: z.string().min(1),
  line: z.number().int().positive().nullable().optional(),
});

export const DesignSystemResponseSchema = z.object({
  id: z.string().min(1),
  primaryProjectId: z.string().min(1),
  name: z.string().min(1),
  description: z.string().nullable(),
  status: DesignSystemStatusSchema,
  currentSchemaVersionId: z.string().nullable(),
  storageRootRef: z.string().min(1),
  createdAt: z.string().min(1),
  updatedAt: z.string().min(1),
  archivedAt: z.string().nullable(),
});

export const DesignSystemSourceResponseSchema = z.object({
  id: z.string().min(1),
  designSystemId: z.string().min(1),
  projectId: z.string().min(1),
  role: z.enum(["primary", "secondary", "reference"]),
  selectedPaths: z.array(z.string()),
  sourceKind: z.enum(["project_checkout", "upload", "url", "manual_note"]),
  gitCommit: z.string().nullable(),
  sourceHashes: z.record(z.string(), z.string()),
  lastAnalyzedAt: z.string().nullable(),
});

export const AgentConversationResponseSchema = z.object({
  id: z.string().min(1),
  context_type: z.string().optional(),
  contextType: z.string().optional(),
  context_id: z.string().optional(),
  contextId: z.string().optional(),
  claude_session_id: z.string().nullable().optional(),
  claudeSessionId: z.string().nullable().optional(),
  provider_session_id: z.string().nullable().optional(),
  providerSessionId: z.string().nullable().optional(),
  provider_harness: z.string().nullable().optional(),
  providerHarness: z.string().nullable().optional(),
  upstream_provider: z.string().nullable().optional(),
  upstreamProvider: z.string().nullable().optional(),
  provider_profile: z.string().nullable().optional(),
  providerProfile: z.string().nullable().optional(),
  title: z.string().nullable(),
  message_count: z.number().optional(),
  messageCount: z.number().optional(),
  last_message_at: z.string().nullable().optional(),
  lastMessageAt: z.string().nullable().optional(),
  created_at: z.string().optional(),
  createdAt: z.string().optional(),
  updated_at: z.string().optional(),
  updatedAt: z.string().optional(),
  archived_at: z.string().nullable().optional(),
  archivedAt: z.string().nullable().optional(),
});

export const AgentMessageResponseSchema = z.object({
  id: z.string().min(1),
  role: z.string().min(1),
  content: z.string(),
  metadata: z.string().nullable(),
  tool_calls: z.unknown().optional(),
  toolCalls: z.unknown().optional(),
  content_blocks: z.unknown().optional(),
  contentBlocks: z.unknown().optional(),
  attribution_source: z.string().nullable().optional(),
  attributionSource: z.string().nullable().optional(),
  provider_harness: z.string().nullable().optional(),
  providerHarness: z.string().nullable().optional(),
  provider_session_id: z.string().nullable().optional(),
  providerSessionId: z.string().nullable().optional(),
  upstream_provider: z.string().nullable().optional(),
  upstreamProvider: z.string().nullable().optional(),
  provider_profile: z.string().nullable().optional(),
  providerProfile: z.string().nullable().optional(),
  logical_model: z.string().nullable().optional(),
  logicalModel: z.string().nullable().optional(),
  effective_model_id: z.string().nullable().optional(),
  effectiveModelId: z.string().nullable().optional(),
  logical_effort: z.string().nullable().optional(),
  logicalEffort: z.string().nullable().optional(),
  effective_effort: z.string().nullable().optional(),
  effectiveEffort: z.string().nullable().optional(),
  input_tokens: z.number().nullable().optional(),
  inputTokens: z.number().nullable().optional(),
  output_tokens: z.number().nullable().optional(),
  outputTokens: z.number().nullable().optional(),
  cache_creation_tokens: z.number().nullable().optional(),
  cacheCreationTokens: z.number().nullable().optional(),
  cache_read_tokens: z.number().nullable().optional(),
  cacheReadTokens: z.number().nullable().optional(),
  estimated_usd: z.number().nullable().optional(),
  estimatedUsd: z.number().nullable().optional(),
  created_at: z.string().optional(),
  createdAt: z.string().optional(),
});

export const DesignSystemDetailResponseSchema = z.object({
  designSystem: DesignSystemResponseSchema,
  sources: z.array(DesignSystemSourceResponseSchema),
  conversation: AgentConversationResponseSchema.nullable(),
});

export const CreateDesignSystemResponseSchema = z.object({
  designSystem: DesignSystemResponseSchema,
  sources: z.array(DesignSystemSourceResponseSchema),
  conversation: AgentConversationResponseSchema,
});

export const DesignStyleguideItemResponseSchema = z.object({
  id: z.string().min(1),
  designSystemId: z.string().min(1),
  schemaVersionId: z.string().min(1),
  itemId: z.string().min(1),
  group: z.enum(["ui_kit", "type", "colors", "spacing", "components", "brand"]),
  label: z.string().min(1),
  summary: z.string(),
  previewArtifactId: z.string().nullable(),
  sourceRefs: z.array(DesignSourceRefSchema),
  confidence: z.enum(["high", "medium", "low"]),
  approvalStatus: z.enum(["needs_review", "approved", "needs_work"]),
  feedbackStatus: z.enum(["none", "open", "in_progress", "resolved", "dismissed"]),
  updatedAt: z.string().min(1),
});

export const DesignStyleguideFeedbackResponseSchema = z.object({
  id: z.string().min(1),
  designSystemId: z.string().min(1),
  schemaVersionId: z.string().min(1),
  itemId: z.string().min(1),
  conversationId: z.string().min(1),
  messageId: z.string().nullable(),
  previewArtifactId: z.string().nullable(),
  sourceRefs: z.array(DesignSourceRefSchema),
  feedback: z.string(),
  status: z.enum(["none", "open", "in_progress", "resolved", "dismissed"]),
  createdAt: z.string().min(1),
  resolvedAt: z.string().nullable(),
});

export const CreateDesignStyleguideFeedbackResponseSchema = z.object({
  feedback: DesignStyleguideFeedbackResponseSchema,
  item: DesignStyleguideItemResponseSchema,
  message: AgentMessageResponseSchema,
});

export const CreateDesignSystemSourceInputSchema = z.object({
  projectId: z.string().min(1),
  role: z.enum(["secondary", "reference"]).optional(),
  selectedPaths: z.array(z.string()).default([]),
});

export const CreateDesignSystemInputSchema = z.object({
  primaryProjectId: z.string().min(1),
  name: z.string().min(1),
  description: z.string().optional(),
  selectedPaths: z.array(z.string()).default([]),
  sources: z.array(CreateDesignSystemSourceInputSchema).default([]),
});

export const CreateDesignStyleguideFeedbackInputSchema = z.object({
  designSystemId: z.string().min(1),
  itemId: z.string().min(1),
  feedback: z.string().min(1),
  conversationId: z.string().optional(),
});

export type DesignSystemResponse = z.infer<typeof DesignSystemResponseSchema>;
export type DesignSystemSourceResponse = z.infer<typeof DesignSystemSourceResponseSchema>;
export type DesignSystemDetailResponse = z.infer<typeof DesignSystemDetailResponseSchema>;
export type CreateDesignSystemInput = z.infer<typeof CreateDesignSystemInputSchema>;
export type CreateDesignSystemResponse = z.infer<typeof CreateDesignSystemResponseSchema>;
export type DesignStyleguideItemResponse = z.infer<typeof DesignStyleguideItemResponseSchema>;
export type DesignStyleguideFeedbackResponse = z.infer<typeof DesignStyleguideFeedbackResponseSchema>;
export type CreateDesignStyleguideFeedbackInput = z.infer<typeof CreateDesignStyleguideFeedbackInputSchema>;
export type CreateDesignStyleguideFeedbackResponse = z.infer<typeof CreateDesignStyleguideFeedbackResponseSchema>;

export const designApi = {
  listProjectDesignSystems: (projectId: string, includeArchived = false) =>
    typedInvoke("list_project_design_systems", { projectId, includeArchived }, z.array(DesignSystemResponseSchema)),

  getDesignSystem: (id: string) =>
    typedInvoke("get_design_system", { id }, DesignSystemDetailResponseSchema.nullable()),

  createDesignSystem: (input: CreateDesignSystemInput) =>
    typedInvoke("create_design_system", { input: CreateDesignSystemInputSchema.parse(input) }, CreateDesignSystemResponseSchema),

  archiveDesignSystem: (id: string) =>
    typedInvoke("archive_design_system", { id }, DesignSystemResponseSchema),

  approveStyleguideItem: (designSystemId: string, itemId: string) =>
    typedInvoke(
      "approve_design_styleguide_item",
      { input: { designSystemId, itemId } },
      DesignStyleguideItemResponseSchema,
    ),

  createStyleguideFeedback: (input: CreateDesignStyleguideFeedbackInput) =>
    typedInvoke(
      "create_design_styleguide_feedback",
      { input: CreateDesignStyleguideFeedbackInputSchema.parse(input) },
      CreateDesignStyleguideFeedbackResponseSchema,
    ),

  resolveStyleguideFeedback: (feedbackId: string) =>
    typedInvoke(
      "resolve_design_styleguide_feedback",
      { input: { feedbackId } },
      DesignStyleguideFeedbackResponseSchema,
    ),
} as const;
