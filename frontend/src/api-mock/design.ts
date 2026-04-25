import type {
  CreateDesignStyleguideFeedbackInput,
  CreateDesignStyleguideFeedbackResponse,
  CreateDesignSystemInput,
  CreateDesignSystemResponse,
  DesignStyleguideFeedbackResponse,
  DesignStyleguideItemResponse,
  DesignStyleguidePreviewResponse,
  DesignStyleguideViewModelResponse,
  DesignSystemDetailResponse,
  DesignSystemResponse,
  DesignSystemSourceResponse,
  ExportDesignSystemPackageInput,
  ExportDesignSystemPackageResponse,
  GenerateDesignArtifactInput,
  GenerateDesignArtifactResponse,
  GenerateDesignSystemStyleguideResponse,
  ImportDesignSystemPackageInput,
  ImportDesignSystemPackageResponse,
} from "@/api/design";
import type { ChatConversation } from "@/types/chat-conversation";
import { getStore } from "./store";
import { seedMockConversation } from "./chat";

const mockSystemsByProject = new Map<string, DesignSystemResponse[]>();
const mockSourcesByDesignSystem = new Map<string, DesignSystemSourceResponse[]>();
const mockConversationsByDesignSystem = new Map<string, ChatConversation>();

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

function conversationForDesignSystem(designSystem: DesignSystemResponse): ChatConversation {
  const existing = mockConversationsByDesignSystem.get(designSystem.id);
  if (existing) {
    return existing;
  }

  const conversation: ChatConversation = {
    id: `conversation-${designSystem.id}`,
    contextType: "design",
    contextId: designSystem.id,
    claudeSessionId: null,
    providerSessionId: null,
    providerHarness: null,
    upstreamProvider: null,
    providerProfile: null,
    agentMode: "chat",
    title: `Design: ${designSystem.name}`,
    messageCount: 0,
    lastMessageAt: null,
    createdAt: nowIso(),
    updatedAt: nowIso(),
    archivedAt: null,
  };
  mockConversationsByDesignSystem.set(designSystem.id, conversation);
  seedMockConversation(conversation, []);
  return conversation;
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
    {
      id: `item-${designSystem.id}-type`,
      designSystemId: designSystem.id,
      schemaVersionId: designSystem.currentSchemaVersionId ?? "schema-version-mock",
      itemId: "type.typography_scale",
      group: "type",
      label: "Typography scale",
      summary: "Text hierarchy, label density, and code-font usage.",
      previewArtifactId: `design-preview-${designSystem.id}-type`,
      sourceRefs: [
        { project_id: projectId, path: "frontend/src/components/Chat/TextBubble.tsx" },
        { project_id: projectId, path: "frontend/src/components/Chat/TeamContextBar.tsx" },
        { project_id: projectId, path: "frontend/src/styles/index.css" },
      ],
      confidence: "high",
      approvalStatus: "needs_review",
      feedbackStatus: "none",
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
      sources: mockSourcesByDesignSystem.get(designSystem.id) ?? [],
      conversation: conversationForDesignSystem(designSystem),
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
    return designSystem?.currentSchemaVersionId ? mockStyleguideItems(designSystem) : [];
  },

  getStyleguideViewModel: async (
    designSystemId: string,
  ): Promise<DesignStyleguideViewModelResponse | null> => {
    const designSystem = allSystems().find((system) => system.id === designSystemId);
    if (!designSystem?.currentSchemaVersionId) {
      return null;
    }
    const items = mockStyleguideItems(designSystem);
    return {
      designSystemId,
      schemaVersionId: designSystem.currentSchemaVersionId,
      artifactId: `styleguide-${designSystem.id}`,
      artifactType: "design_doc",
      content: {
        design_system_id: designSystem.id,
        schema_version_id: designSystem.currentSchemaVersionId,
        version: "0.1.0",
        generated_at: nowIso(),
        ready_summary: `${designSystem.name} is ready for source-grounded styleguide review.`,
        caveats: [],
        groups: [
          {
            id: "colors",
            label: "Colors",
            items: items
              .filter((item) => item.group === "colors")
              .map((item) => styleguideItemContent(item)),
          },
          {
            id: "components",
            label: "Components",
            items: items
              .filter((item) => item.group === "components")
              .map((item) => styleguideItemContent(item)),
          },
          {
            id: "type",
            label: "Type",
            items: items
              .filter((item) => item.group === "type")
              .map((item) => styleguideItemContent(item)),
          },
        ],
      },
    };
  },

  getStyleguidePreview: async (
    designSystemId: string,
    previewArtifactId: string,
  ): Promise<DesignStyleguidePreviewResponse> => {
    const designSystem = allSystems().find((system) => system.id === designSystemId);
    const item =
      designSystem
        ? mockStyleguideItems(designSystem).find(
            (candidate) => candidate.previewArtifactId === previewArtifactId,
          )
        : null;
    const fallback = item ?? mockStyleguideItemForAction(designSystemId, "components.buttons");
    return {
      designSystemId,
      schemaVersionId: fallback.schemaVersionId,
      artifactId: previewArtifactId,
      artifactType: "design_doc",
      content: {
        design_system_id: designSystemId,
        schema_version_id: fallback.schemaVersionId,
        item_id: fallback.itemId,
        group: fallback.group,
        label: fallback.label,
        summary: fallback.summary,
        preview_kind: previewKindForGroup(fallback.group),
        confidence: fallback.confidence,
        source_refs: fallback.sourceRefs,
        source_paths: fallback.sourceRefs.map((sourceRef) => sourceRef.path),
        source_labels: sourceLabelsFromRefs(fallback.sourceRefs),
        ...previewPayloadForItem(fallback),
        generated_at: nowIso(),
      },
    };
  },

  exportPackage: async (
    input: ExportDesignSystemPackageInput,
  ): Promise<ExportDesignSystemPackageResponse> => {
    const designSystem =
      allSystems().find((system) => system.id === input.designSystemId) ??
      mockDesignSystem("project-mock");
    const schemaVersionId = designSystem.currentSchemaVersionId ?? `schema-${designSystem.id}`;
    const includeFullProvenance = input.includeFullProvenance ?? false;
    return {
      designSystemId: input.designSystemId,
      schemaVersionId,
      runId: `run-export-${designSystem.id}`,
      artifactId: `export-${designSystem.id}`,
      redacted: !includeFullProvenance,
      exportedAt: nowIso(),
      filePath: input.destinationPath ?? null,
      content: {
        package_version: "1.0",
        redacted: !includeFullProvenance,
        design_system: {
          id: designSystem.id,
          name: designSystem.name,
          schema_version_id: schemaVersionId,
        },
      },
    };
  },

  importPackage: async (
    input: ImportDesignSystemPackageInput,
  ): Promise<ImportDesignSystemPackageResponse> => {
    const store = getStore();
    const project = store.projects.get(input.attachProjectId);
    if (!project) {
      throw new Error(`Project not found: ${input.attachProjectId}`);
    }
    const existing = systemsForProject(input.attachProjectId);
    const designSystem = mockDesignSystem(
      input.attachProjectId,
      input.name?.trim() || `${project.name} Imported Design System`,
      {
        id: `imported-design-system-${input.attachProjectId}-${existing.length + 1}`,
        status: "ready",
        currentSchemaVersionId: `schema-imported-${existing.length + 1}`,
        sourceCount: 1,
      },
    );
    const sources: DesignSystemSourceResponse[] = [
      {
        id: `source-${designSystem.id}-import`,
        designSystemId: designSystem.id,
        projectId: input.attachProjectId,
        role: "primary",
        selectedPaths: [],
        sourceKind: "manual_note",
        gitCommit: null,
        sourceHashes: {},
        lastAnalyzedAt: nowIso(),
      },
    ];
    mockSystemsByProject.set(input.attachProjectId, [
      designSystem,
      ...existing.filter((system) => system.id !== designSystem.id),
    ]);
    mockSourcesByDesignSystem.set(designSystem.id, sources);

    const conversation = conversationForDesignSystem(designSystem);

    return {
      designSystem,
      sources,
      conversation,
      schemaVersionId: designSystem.currentSchemaVersionId ?? "schema-imported",
      runId: `run-import-${designSystem.id}`,
      packageArtifactId: input.packageArtifactId ?? null,
      packagePath: input.packagePath ?? null,
      items: mockStyleguideItems(designSystem).map((item) => ({
        ...item,
        previewArtifactId: null,
        sourceRefs: [],
      })),
    };
  },

  generateArtifact: async (
    input: GenerateDesignArtifactInput,
  ): Promise<GenerateDesignArtifactResponse> => {
    const designSystem =
      allSystems().find((system) => system.id === input.designSystemId) ??
      mockDesignSystem("project-mock");
    const sourceItem = mockStyleguideItemForAction(
      input.designSystemId,
      input.sourceItemId ?? (input.artifactKind === "screen" ? "ui_kit.workspace_surfaces" : "components.buttons"),
    );
    return {
      designSystemId: input.designSystemId,
      schemaVersionId: designSystem.currentSchemaVersionId ?? `schema-${designSystem.id}`,
      runId: `run-generated-${input.artifactKind}-${designSystem.id}`,
      artifactId: `generated-${input.artifactKind}-${designSystem.id}`,
      previewArtifactId: `generated-preview-${input.artifactKind}-${designSystem.id}`,
      artifactKind: input.artifactKind,
      name: input.name,
      createdAt: nowIso(),
      content: {
        design_system_id: input.designSystemId,
        kind: input.artifactKind,
        name: input.name,
        source_item: {
          item_id: sourceItem.itemId,
          label: sourceItem.label,
        },
        source_refs: sourceItem.sourceRefs,
        artifact: {
          storage: "ralphx_owned",
          project_write_status: "not_written",
        },
      },
    };
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
    runId: `run-feedback-${input.itemId}`,
    message: {
      id: "message-mock",
      role: "user",
      content: [
        "Design styleguide feedback",
        `Item: ${input.itemId}`,
        "Preview: preview pending",
        "Source refs: none",
        "",
        input.feedback,
      ].join("\n"),
      metadata: JSON.stringify({
        kind: "design_styleguide_feedback",
        designSystemId: input.designSystemId,
        schemaVersionId: "schema-version-mock",
        itemId: input.itemId,
        previewArtifactId: null,
        sourceRefs: [],
      }),
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

function previewKindForGroup(group: DesignStyleguideItemResponse["group"]) {
  switch (group) {
    case "colors":
      return "color_swatch";
    case "type":
      return "typography_sample";
    case "spacing":
      return "spacing_sample";
    case "ui_kit":
      return "layout_sample";
    case "brand":
      return "asset_sample";
    case "components":
    default:
      return "component_sample";
  }
}

function previewPayloadForItem(item: DesignStyleguideItemResponse): Record<string, unknown> {
  const sourceLabels = sourceLabelsFromRefs(item.sourceRefs);
  switch (item.group) {
    case "colors":
      return {
        swatches: [
          { label: "Mock primary", value: "#ff6b35" },
          { label: "Mock soft", value: "rgba(255, 107, 53, 0.12)" },
        ],
      };
    case "type":
      return {
        typography_samples: [
          { label: "Display", sample: sourceLabels[0] ?? item.label },
          { label: "Body", sample: sourceLabels[1] ?? item.summary },
        ],
      };
    case "spacing":
    case "ui_kit":
      return {
        layout_regions: sourceLabels.map((label) => ({ label })),
      };
    case "brand":
      return {
        asset_samples: sourceLabels.map((label, index) => ({
          label,
          path: item.sourceRefs[index]?.path ?? null,
        })),
      };
    case "components":
    default:
      return {
        component_samples: sourceLabels.map((label) => ({ label })),
      };
  }
}

function sourceLabelsFromRefs(sourceRefs: DesignStyleguideItemResponse["sourceRefs"]): string[] {
  return sourceRefs.map((sourceRef) => labelFromPath(sourceRef.path));
}

function labelFromPath(path: string): string {
  const filename = path.split("/").pop() ?? path;
  return filename
    .replace(/\.[^.]+$/, "")
    .replace(/[-_.]+/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/\b\w/g, (match) => match.toUpperCase())
    .trim();
}

function styleguideItemContent(item: DesignStyleguideItemResponse) {
  return {
    id: item.itemId,
    group: item.group,
    label: item.label,
    summary: item.summary,
    preview_artifact_id: item.previewArtifactId,
    source_refs: item.sourceRefs,
    confidence: item.confidence,
    approval_status: item.approvalStatus,
    feedback_status: item.feedbackStatus,
    updated_at: item.updatedAt,
  };
}

function createMockDesignSystem(input: CreateDesignSystemInput): CreateDesignSystemResponse {
  const existing = systemsForProject(input.primaryProjectId);
  const designSystem = mockDesignSystem(input.primaryProjectId, input.name, {
    id: `design-system-${input.primaryProjectId}-${existing.length + 1}`,
    description: input.description ?? null,
    status: "draft",
  });

  const sources: DesignSystemSourceResponse[] = [
    {
      id: `source-${designSystem.id}-primary`,
      designSystemId: designSystem.id,
      projectId: input.primaryProjectId,
      role: "primary",
      selectedPaths: input.selectedPaths,
      sourceKind: "project_checkout",
      gitCommit: null,
      sourceHashes: {},
      lastAnalyzedAt: null,
    },
    ...input.sources.map((source, index) => ({
      id: `source-${designSystem.id}-${index}`,
      designSystemId: designSystem.id,
      projectId: source.projectId,
      role: source.role ?? "reference",
      selectedPaths: source.selectedPaths,
      sourceKind: "project_checkout" as const,
      gitCommit: null,
      sourceHashes: {},
      lastAnalyzedAt: null,
    })),
  ];
  const designSystemWithSourceCount = {
    ...designSystem,
    sourceCount: sources.length,
  };
  mockSystemsByProject.set(input.primaryProjectId, [
    designSystemWithSourceCount,
    ...existing.filter((system) => system.id !== designSystem.id),
  ]);
  mockSourcesByDesignSystem.set(designSystem.id, sources);

  const conversation = conversationForDesignSystem(designSystemWithSourceCount);

  return {
    designSystem: designSystemWithSourceCount,
    sources,
    conversation,
  };
}
