import type {
  CreateDesignStyleguideFeedbackInput,
  CreateDesignStyleguideFeedbackResponse,
  CreateDesignSystemInput,
  CreateDesignSystemResponse,
  DesignStyleguideFeedbackResponse,
  DesignStyleguideItemResponse,
  DesignSystemDetailResponse,
  DesignSystemResponse,
} from "@/api/design";

function nowIso() {
  return new Date("2026-04-24T08:00:00.000Z").toISOString();
}

function mockDesignSystem(projectId: string, name = "Mock Design System"): DesignSystemResponse {
  return {
    id: `design-system-${projectId}`,
    primaryProjectId: projectId,
    name,
    description: null,
    status: "draft",
    currentSchemaVersionId: null,
    storageRootRef: "design-mock",
    createdAt: nowIso(),
    updatedAt: nowIso(),
    archivedAt: null,
  };
}

export const mockDesignApi = {
  listProjectDesignSystems: async (projectId: string): Promise<DesignSystemResponse[]> => [
    mockDesignSystem(projectId, `${projectId} Design System`),
  ],

  getDesignSystem: async (id: string): Promise<DesignSystemDetailResponse | null> => ({
    designSystem: mockDesignSystem(id),
    sources: [],
    conversation: null,
  }),

  createDesignSystem: async (input: CreateDesignSystemInput): Promise<CreateDesignSystemResponse> => ({
    designSystem: mockDesignSystem(input.primaryProjectId, input.name),
    sources: [],
    conversation: {
      id: `conversation-${input.primaryProjectId}`,
      contextType: "design",
      contextId: `design-system-${input.primaryProjectId}`,
      title: `Design: ${input.name}`,
      messageCount: 0,
      lastMessageAt: null,
      createdAt: nowIso(),
      updatedAt: nowIso(),
      archivedAt: null,
    },
  }),

  archiveDesignSystem: async (id: string): Promise<DesignSystemResponse> => ({
    ...mockDesignSystem(id),
    status: "archived",
    archivedAt: nowIso(),
  }),

  approveStyleguideItem: async (
    designSystemId: string,
    itemId: string,
  ): Promise<DesignStyleguideItemResponse> => ({
    id: `item-${itemId}`,
    designSystemId,
    schemaVersionId: "schema-version-mock",
    itemId,
    group: "components",
    label: itemId,
    summary: "",
    previewArtifactId: null,
    sourceRefs: [],
    confidence: "medium",
    approvalStatus: "approved",
    feedbackStatus: "resolved",
    updatedAt: nowIso(),
  }),

  createStyleguideFeedback: async (
    input: CreateDesignStyleguideFeedbackInput,
  ): Promise<CreateDesignStyleguideFeedbackResponse> => ({
    feedback: {
      id: `feedback-${input.itemId}`,
      designSystemId: input.designSystemId,
      schemaVersionId: "schema-version-mock",
      itemId: input.itemId,
      conversationId: input.conversationId ?? "conversation-mock",
      messageId: "message-mock",
      previewArtifactId: null,
      sourceRefs: [],
      feedback: input.feedback,
      status: "open",
      createdAt: nowIso(),
      resolvedAt: null,
    },
    item: {
      id: `item-${input.itemId}`,
      designSystemId: input.designSystemId,
      schemaVersionId: "schema-version-mock",
      itemId: input.itemId,
      group: "components",
      label: input.itemId,
      summary: "",
      previewArtifactId: null,
      sourceRefs: [],
      confidence: "medium",
      approvalStatus: "needs_work",
      feedbackStatus: "open",
      updatedAt: nowIso(),
    },
    message: {
      id: "message-mock",
      role: "user",
      content: input.feedback,
      metadata: null,
      createdAt: nowIso(),
    },
  }),

  resolveStyleguideFeedback: async (feedbackId: string): Promise<DesignStyleguideFeedbackResponse> => ({
    id: feedbackId,
    designSystemId: "design-system-mock",
    schemaVersionId: "schema-version-mock",
    itemId: "item-mock",
    conversationId: "conversation-mock",
    messageId: "message-mock",
    previewArtifactId: null,
    sourceRefs: [],
    feedback: "",
    status: "resolved",
    createdAt: nowIso(),
    resolvedAt: nowIso(),
  }),
} as const;
