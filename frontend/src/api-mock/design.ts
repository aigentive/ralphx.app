import type {
  CreateDesignStyleguideFeedbackInput,
  CreateDesignStyleguideFeedbackResponse,
  CreateDesignSystemInput,
  CreateDesignSystemResponse,
  DesignStyleguideFeedbackResponse,
  DesignStyleguideItemResponse,
  DesignSystemDetailResponse,
  DesignSystemResponse,
  GenerateDesignSystemStyleguideResponse,
} from "@/api/design";
import { getStore } from "./store";

const mockSystemsByProject = new Map<string, DesignSystemResponse[]>();

function nowIso() {
  return new Date("2026-04-24T08:00:00.000Z").toISOString();
}

function projectDesignSystemName(projectId: string) {
  return `${getStore().projects.get(projectId)?.name ?? projectId} Design System`;
}

function mockDesignSystem(
  projectId: string,
  name = projectDesignSystemName(projectId),
  overrides: Partial<DesignSystemResponse> = {},
): DesignSystemResponse {
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
    ...overrides,
  };
}

function systemsForProject(projectId: string): DesignSystemResponse[] {
  const existing = mockSystemsByProject.get(projectId);
  if (existing) {
    return existing;
  }

  const seeded = [mockDesignSystem(projectId)];
  mockSystemsByProject.set(projectId, seeded);
  return seeded;
}

function allSystems(): DesignSystemResponse[] {
  return Array.from(mockSystemsByProject.values()).flat();
}

function mockStyleguideItems(designSystem: DesignSystemResponse): DesignStyleguideItemResponse[] {
  const projectId = designSystem.primaryProjectId;
  return [
    {
      id: `item-${designSystem.id}-colors`,
      designSystemId: designSystem.id,
      schemaVersionId: designSystem.currentSchemaVersionId ?? "schema-version-mock",
      itemId: "colors.primary_palette",
      group: "colors",
      label: "Primary palette",
      summary: "Primary, hover, soft, and ring roles.",
      previewArtifactId: `design-preview-${designSystem.id}-colors`,
      sourceRefs: [{ project_id: projectId, path: "specs/design/styleguide.md" }],
      confidence: "high",
      approvalStatus: "needs_review",
      feedbackStatus: "none",
      updatedAt: nowIso(),
    },
    {
      id: `item-${designSystem.id}-buttons`,
      designSystemId: designSystem.id,
      schemaVersionId: designSystem.currentSchemaVersionId ?? "schema-version-mock",
      itemId: "components.buttons",
      group: "components",
      label: "Buttons",
      summary: "Primary, secondary, ghost, icon, and loading button patterns.",
      previewArtifactId: `design-preview-${designSystem.id}-buttons`,
      sourceRefs: [{ project_id: projectId, path: "frontend/src/components/ui/button.tsx" }],
      confidence: "medium",
      approvalStatus: "needs_work",
      feedbackStatus: "open",
      updatedAt: nowIso(),
    },
  ];
}

export const mockDesignApi = {
  listProjectDesignSystems: async (
    projectId: string,
    includeArchived = false,
  ): Promise<DesignSystemResponse[]> =>
    systemsForProject(projectId).filter((system) => includeArchived || system.status !== "archived"),

  getDesignSystem: async (id: string): Promise<DesignSystemDetailResponse | null> => {
    const designSystem = allSystems().find((system) => system.id === id);
    if (!designSystem) {
      return null;
    }
    return {
      designSystem,
      sources: [],
      conversation: null,
    };
  },

  createDesignSystem: async (input: CreateDesignSystemInput): Promise<CreateDesignSystemResponse> =>
    createMockDesignSystem(input),

  archiveDesignSystem: async (id: string): Promise<DesignSystemResponse> => {
    const designSystem = allSystems().find((system) => system.id === id);
    const archived = {
      ...(designSystem ?? mockDesignSystem(id)),
      status: "archived" as const,
      archivedAt: nowIso(),
      updatedAt: nowIso(),
    };
    mockSystemsByProject.set(
      archived.primaryProjectId,
      systemsForProject(archived.primaryProjectId).map((system) =>
        system.id === archived.id ? archived : system,
      ),
    );
    return archived;
  },

  generateStyleguide: async (designSystemId: string): Promise<GenerateDesignSystemStyleguideResponse> => {
    const designSystem = allSystems().find((system) => system.id === designSystemId);
    if (!designSystem) {
      throw new Error(`Design system not found: ${designSystemId}`);
    }
    const generated = {
      ...designSystem,
      status: "ready" as const,
      currentSchemaVersionId: designSystem.currentSchemaVersionId ?? `schema-${designSystem.id}`,
      updatedAt: nowIso(),
    };
    mockSystemsByProject.set(
      generated.primaryProjectId,
      systemsForProject(generated.primaryProjectId).map((system) =>
        system.id === generated.id ? generated : system,
      ),
    );
    return {
      designSystem: generated,
      schemaVersionId: generated.currentSchemaVersionId,
      runId: `run-${generated.id}`,
      items: mockStyleguideItems(generated),
    };
  },

  listStyleguideItems: async (designSystemId: string): Promise<DesignStyleguideItemResponse[]> => {
    const designSystem = allSystems().find((system) => system.id === designSystemId);
    return designSystem ? mockStyleguideItems(designSystem) : [];
  },

  approveStyleguideItem: async (
    designSystemId: string,
    itemId: string,
  ): Promise<DesignStyleguideItemResponse> => ({
    ...mockStyleguideItemForAction(designSystemId, itemId),
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
      ...mockStyleguideItemForAction(input.designSystemId, input.itemId),
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

function mockStyleguideItemForAction(
  designSystemId: string,
  itemId: string,
): DesignStyleguideItemResponse {
  const designSystem = allSystems().find((system) => system.id === designSystemId);
  return (
    designSystem
      ? mockStyleguideItems(designSystem).find((item) => item.itemId === itemId)
      : null
  ) ?? {
    id: `item-${designSystemId}-${itemId}`,
    designSystemId,
    schemaVersionId: "schema-version-mock",
    itemId,
    group: "components",
    label: itemId,
    summary: "",
    previewArtifactId: null,
    sourceRefs: [],
    confidence: "medium",
    approvalStatus: "needs_review",
    feedbackStatus: "none",
    updatedAt: nowIso(),
  };
}

function createMockDesignSystem(input: CreateDesignSystemInput): CreateDesignSystemResponse {
  const existing = systemsForProject(input.primaryProjectId);
  const designSystem = mockDesignSystem(input.primaryProjectId, input.name, {
    id: `design-system-${input.primaryProjectId}-${existing.length + 1}`,
    description: input.description ?? null,
    status: "draft",
  });

  mockSystemsByProject.set(input.primaryProjectId, [
    designSystem,
    ...existing.filter((system) => system.id !== designSystem.id),
  ]);

  return {
    designSystem,
    sources: [],
    conversation: {
      id: `conversation-${designSystem.id}`,
      contextType: "design",
      contextId: designSystem.id,
      title: `Design: ${input.name}`,
      messageCount: 0,
      lastMessageAt: null,
      createdAt: nowIso(),
      updatedAt: nowIso(),
      archivedAt: null,
    },
  };
}
