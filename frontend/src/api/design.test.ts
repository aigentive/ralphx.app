import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  CreateDesignStyleguideFeedbackResponseSchema,
  CreateDesignSystemResponseSchema,
  DesignStyleguidePreviewResponseSchema,
  DesignStyleguideViewModelResponseSchema,
  DesignSystemResponseSchema,
  ExportDesignSystemPackageResponseSchema,
  GenerateDesignArtifactResponseSchema,
  GenerateDesignSystemStyleguideResponseSchema,
  ImportDesignSystemPackageResponseSchema,
  designApi,
} from "./design";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

function designSystemResponse(overrides = {}) {
  return {
    id: "design-system-1",
    primaryProjectId: "project-1",
    name: "Product UI",
    description: null,
    status: "draft",
    currentSchemaVersionId: null,
    storageRootRef: "design-root",
    createdAt: "2026-04-24T08:00:00Z",
    updatedAt: "2026-04-24T08:00:00Z",
    archivedAt: null,
    ...overrides,
  };
}

function conversationResponse() {
  return {
    id: "conversation-1",
    context_type: "design",
    context_id: "design-system-1",
    claude_session_id: null,
    provider_session_id: null,
    provider_harness: null,
    upstream_provider: null,
    provider_profile: null,
    title: "Design: Product UI",
    message_count: 0,
    last_message_at: null,
    created_at: "2026-04-24T08:00:00Z",
    updated_at: "2026-04-24T08:00:00Z",
    archived_at: null,
  };
}

function styleguideItemResponse() {
  return {
    id: "item-1",
    designSystemId: "design-system-1",
    schemaVersionId: "schema-1",
    itemId: "components.buttons",
    group: "components",
    label: "Buttons",
    summary: "Button patterns",
    previewArtifactId: "preview-1",
    sourceRefs: [{ project_id: "project-1", path: "frontend/src/Button.tsx", line: 12 }],
    confidence: "medium",
    approvalStatus: "needs_review",
    feedbackStatus: "none",
    updatedAt: "2026-04-24T08:00:00Z",
  };
}

describe("design API schemas", () => {
  it("parses a design system response", () => {
    expect(DesignSystemResponseSchema.parse(designSystemResponse()).status).toBe("draft");
  });

  it("parses create response with source hashes", () => {
    const parsed = CreateDesignSystemResponseSchema.parse({
      designSystem: designSystemResponse(),
      sources: [
        {
          id: "source-1",
          designSystemId: "design-system-1",
          projectId: "project-1",
          role: "primary",
          selectedPaths: ["frontend/src"],
          sourceKind: "project_checkout",
          gitCommit: null,
          sourceHashes: { "frontend/src/App.tsx": "a".repeat(64) },
          lastAnalyzedAt: null,
        },
      ],
      conversation: conversationResponse(),
    });

    expect(parsed.sources[0]?.sourceHashes["frontend/src/App.tsx"]).toHaveLength(64);
  });

  it("parses feedback bridge response with source refs", () => {
    const parsed = CreateDesignStyleguideFeedbackResponseSchema.parse({
      feedback: {
        id: "feedback-1",
        designSystemId: "design-system-1",
        schemaVersionId: "schema-1",
        itemId: "components.buttons",
        conversationId: "conversation-1",
        messageId: "message-1",
        previewArtifactId: "preview-1",
        sourceRefs: [{ project_id: "project-1", path: "frontend/src/Button.tsx", line: 12 }],
        feedback: "Needs stronger focus state",
        status: "open",
        createdAt: "2026-04-24T08:00:00Z",
        resolvedAt: null,
      },
      item: {
        id: "item-1",
        designSystemId: "design-system-1",
        schemaVersionId: "schema-1",
        itemId: "components.buttons",
        group: "components",
        label: "Buttons",
        summary: "Button patterns",
        previewArtifactId: "preview-1",
        sourceRefs: [{ project_id: "project-1", path: "frontend/src/Button.tsx", line: 12 }],
        confidence: "medium",
        approvalStatus: "needs_work",
        feedbackStatus: "open",
        updatedAt: "2026-04-24T08:00:00Z",
      },
      message: {
        id: "message-1",
        role: "user",
        content: "Feedback on Buttons: Needs stronger focus state",
        metadata: "{}",
        created_at: "2026-04-24T08:00:00Z",
      },
    });

    expect(parsed.feedback.sourceRefs[0]?.project_id).toBe("project-1");
  });

  it("parses generated styleguide response with source-backed items", () => {
    const parsed = GenerateDesignSystemStyleguideResponseSchema.parse({
      designSystem: designSystemResponse({
        status: "ready",
        currentSchemaVersionId: "schema-1",
      }),
      schemaVersionId: "schema-1",
      runId: "run-1",
      items: [styleguideItemResponse()],
    });

    expect(parsed.items[0]?.sourceRefs[0]?.path).toBe("frontend/src/Button.tsx");
  });

  it("parses persisted styleguide and preview artifact responses", () => {
    const viewModel = DesignStyleguideViewModelResponseSchema.parse({
      designSystemId: "design-system-1",
      schemaVersionId: "schema-1",
      artifactId: "styleguide-1",
      artifactType: "design_doc",
      content: {
        design_system_id: "design-system-1",
        schema_version_id: "schema-1",
        version: "0.1.0",
        generated_at: "2026-04-24T08:00:00Z",
        ready_summary: "Product UI is ready for review.",
        caveats: [{ item_id: "components.buttons", severity: "medium", summary: "Check focus states." }],
        groups: [
          {
            id: "components",
            label: "Components",
            items: [
              {
                id: "components.buttons",
                group: "components",
                label: "Buttons",
                summary: "Button patterns",
                preview_artifact_id: "preview-1",
                source_refs: [{ project_id: "project-1", path: "frontend/src/Button.tsx" }],
                confidence: "medium",
              },
            ],
          },
        ],
      },
    });
    const preview = DesignStyleguidePreviewResponseSchema.parse({
      designSystemId: "design-system-1",
      schemaVersionId: "schema-1",
      artifactId: "preview-1",
      artifactType: "design_doc",
      content: {
        design_system_id: "design-system-1",
        schema_version_id: "schema-1",
        item_id: "components.buttons",
        group: "components",
        label: "Buttons",
        summary: "Button patterns",
        preview_kind: "component_sample",
        confidence: "medium",
        source_refs: [{ project_id: "project-1", path: "frontend/src/Button.tsx" }],
        generated_at: "2026-04-24T08:00:00Z",
      },
    });

    expect(viewModel.content.groups[0]?.items[0]?.preview_artifact_id).toBe("preview-1");
    expect(preview.content.preview_kind).toBe("component_sample");
  });

  it("parses a redacted design export package response", () => {
    const parsed = ExportDesignSystemPackageResponseSchema.parse({
      designSystemId: "design-system-1",
      schemaVersionId: "schema-1",
      artifactId: "export-1",
      redacted: true,
      exportedAt: "2026-04-24T08:00:00Z",
      content: {
        package_version: "1.0",
        redacted: true,
      },
    });

    expect(parsed.redacted).toBe(true);
  });

  it("parses a generated design artifact response", () => {
    const parsed = GenerateDesignArtifactResponseSchema.parse({
      designSystemId: "design-system-1",
      schemaVersionId: "schema-1",
      runId: "run-1",
      artifactId: "artifact-1",
      previewArtifactId: "preview-1",
      artifactKind: "component",
      name: "Pricing cards component",
      createdAt: "2026-04-24T08:00:00Z",
      content: {
        design_system_id: "design-system-1",
        kind: "component",
        artifact: {
          storage: "ralphx_owned",
          project_write_status: "not_written",
        },
      },
    });

    expect(parsed.artifactKind).toBe("component");
    expect(parsed.content.artifact).toBeDefined();
  });

  it("parses an imported design package response", () => {
    const parsed = ImportDesignSystemPackageResponseSchema.parse({
      designSystem: {
        id: "design-system-imported",
        primaryProjectId: "project-1",
        name: "Imported UI",
        description: "Imported design system package",
        status: "ready",
        currentSchemaVersionId: "schema-1",
        storageRootRef: "design-root",
        createdAt: "2026-04-24T08:00:00Z",
        updatedAt: "2026-04-24T08:00:00Z",
        archivedAt: null,
      },
      sources: [
        {
          id: "source-1",
          designSystemId: "design-system-imported",
          projectId: "project-1",
          role: "primary",
          selectedPaths: [],
          sourceKind: "manual_note",
          gitCommit: null,
          sourceHashes: {},
          lastAnalyzedAt: "2026-04-24T08:00:00Z",
        },
      ],
      conversation: {
        id: "conversation-1",
        contextType: "design",
        contextId: "design-system-imported",
        title: "Design: Imported UI",
        messageCount: 0,
        lastMessageAt: null,
        createdAt: "2026-04-24T08:00:00Z",
        updatedAt: "2026-04-24T08:00:00Z",
        archivedAt: null,
      },
      schemaVersionId: "schema-1",
      runId: "run-1",
      packageArtifactId: "export-1",
      items: [],
    });

    expect(parsed.packageArtifactId).toBe("export-1");
  });
});

describe("designApi", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("lists project design systems with camelCase invoke args", async () => {
    mockInvoke.mockResolvedValueOnce([designSystemResponse()]);

    await designApi.listProjectDesignSystems("project-1", true);

    expect(mockInvoke).toHaveBeenCalledWith("list_project_design_systems", {
      projectId: "project-1",
      includeArchived: true,
    });
  });

  it("creates a design system through the draft command", async () => {
    mockInvoke.mockResolvedValueOnce({
      designSystem: designSystemResponse(),
      sources: [],
      conversation: conversationResponse(),
    });

    const response = await designApi.createDesignSystem({
      primaryProjectId: "project-1",
      name: "Product UI",
      selectedPaths: ["frontend/src"],
      sources: [],
    });

    expect(response.designSystem.id).toBe("design-system-1");
    expect(mockInvoke).toHaveBeenCalledWith("create_design_system", {
      input: {
        primaryProjectId: "project-1",
        name: "Product UI",
        selectedPaths: ["frontend/src"],
        sources: [],
      },
    });
  });

  it("lists styleguide items through the current-schema command", async () => {
    mockInvoke.mockResolvedValueOnce([styleguideItemResponse()]);

    const response = await designApi.listStyleguideItems("design-system-1");

    expect(response[0]?.itemId).toBe("components.buttons");
    expect(mockInvoke).toHaveBeenCalledWith("list_design_styleguide_items", {
      input: {
        designSystemId: "design-system-1",
        schemaVersionId: undefined,
      },
    });
  });

  it("generates an initial styleguide through the publish command", async () => {
    mockInvoke.mockResolvedValueOnce({
      designSystem: designSystemResponse({
        status: "ready",
        currentSchemaVersionId: "schema-1",
      }),
      schemaVersionId: "schema-1",
      runId: "run-1",
      items: [styleguideItemResponse()],
    });

    const response = await designApi.generateStyleguide("design-system-1");

    expect(response.runId).toBe("run-1");
    expect(mockInvoke).toHaveBeenCalledWith("generate_design_system_styleguide", {
      input: {
        designSystemId: "design-system-1",
      },
    });
  });

  it("loads persisted styleguide and preview artifacts through design commands", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        designSystemId: "design-system-1",
        schemaVersionId: "schema-1",
        artifactId: "styleguide-1",
        artifactType: "design_doc",
        content: {
          design_system_id: "design-system-1",
          schema_version_id: "schema-1",
          version: "0.1.0",
          generated_at: "2026-04-24T08:00:00Z",
          caveats: [],
          groups: [],
        },
      })
      .mockResolvedValueOnce({
        designSystemId: "design-system-1",
        schemaVersionId: "schema-1",
        artifactId: "preview-1",
        artifactType: "design_doc",
        content: {
          design_system_id: "design-system-1",
          schema_version_id: "schema-1",
          item_id: "components.buttons",
          group: "components",
          label: "Buttons",
          summary: "Button patterns",
          preview_kind: "component_sample",
          source_refs: [],
          generated_at: "2026-04-24T08:00:00Z",
        },
      });

    await designApi.getStyleguideViewModel("design-system-1");
    await designApi.getStyleguidePreview("design-system-1", "preview-1");

    expect(mockInvoke).toHaveBeenNthCalledWith(1, "get_design_styleguide_view_model", {
      input: {
        designSystemId: "design-system-1",
        schemaVersionId: undefined,
      },
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "get_design_styleguide_preview", {
      input: {
        designSystemId: "design-system-1",
        previewArtifactId: "preview-1",
      },
    });
  });

  it("exports a redacted design package through the backend command", async () => {
    mockInvoke.mockResolvedValueOnce({
      designSystemId: "design-system-1",
      schemaVersionId: "schema-1",
      artifactId: "export-1",
      redacted: true,
      exportedAt: "2026-04-24T08:00:00Z",
      content: {
        package_version: "1.0",
        redacted: true,
      },
    });

    await designApi.exportPackage("design-system-1");

    expect(mockInvoke).toHaveBeenCalledWith("export_design_system_package", {
      input: {
        designSystemId: "design-system-1",
        includeFullProvenance: false,
      },
    });
  });

  it("imports a design package through the backend command", async () => {
    mockInvoke.mockResolvedValueOnce({
      designSystem: {
        id: "design-system-imported",
        primaryProjectId: "project-1",
        name: "Imported UI",
        description: "Imported design system package",
        status: "ready",
        currentSchemaVersionId: "schema-1",
        storageRootRef: "design-root",
        createdAt: "2026-04-24T08:00:00Z",
        updatedAt: "2026-04-24T08:00:00Z",
        archivedAt: null,
      },
      sources: [],
      conversation: {
        id: "conversation-1",
        contextType: "design",
        contextId: "design-system-imported",
        title: "Design: Imported UI",
        messageCount: 0,
        lastMessageAt: null,
        createdAt: "2026-04-24T08:00:00Z",
        updatedAt: "2026-04-24T08:00:00Z",
        archivedAt: null,
      },
      schemaVersionId: "schema-1",
      runId: "run-1",
      packageArtifactId: "export-1",
      items: [],
    });

    await designApi.importPackage({
      packageArtifactId: "export-1",
      attachProjectId: "project-1",
      name: "Imported UI",
    });

    expect(mockInvoke).toHaveBeenCalledWith("import_design_system_package", {
      input: {
        packageArtifactId: "export-1",
        attachProjectId: "project-1",
        name: "Imported UI",
      },
    });
  });

  it("generates a screen or component artifact through the backend command", async () => {
    mockInvoke.mockResolvedValueOnce({
      designSystemId: "design-system-1",
      schemaVersionId: "schema-1",
      runId: "run-1",
      artifactId: "artifact-1",
      previewArtifactId: "preview-1",
      artifactKind: "screen",
      name: "Settings screen",
      createdAt: "2026-04-24T08:00:00Z",
      content: {
        kind: "screen",
      },
    });

    await designApi.generateArtifact({
      designSystemId: "design-system-1",
      artifactKind: "screen",
      name: "Settings screen",
      sourceItemId: "ui_kit.workspace_surfaces",
    });

    expect(mockInvoke).toHaveBeenCalledWith("generate_design_artifact", {
      input: {
        designSystemId: "design-system-1",
        artifactKind: "screen",
        name: "Settings screen",
        sourceItemId: "ui_kit.workspace_surfaces",
      },
    });
  });

  it("creates styleguide feedback through the bridge command", async () => {
    mockInvoke.mockResolvedValueOnce({
      feedback: {
        id: "feedback-1",
        designSystemId: "design-system-1",
        schemaVersionId: "schema-1",
        itemId: "components.buttons",
        conversationId: "conversation-1",
        messageId: "message-1",
        previewArtifactId: null,
        sourceRefs: [],
        feedback: "Needs stronger focus state",
        status: "open",
        createdAt: "2026-04-24T08:00:00Z",
        resolvedAt: null,
      },
      item: {
        id: "item-1",
        designSystemId: "design-system-1",
        schemaVersionId: "schema-1",
        itemId: "components.buttons",
        group: "components",
        label: "Buttons",
        summary: "Button patterns",
        previewArtifactId: null,
        sourceRefs: [],
        confidence: "medium",
        approvalStatus: "needs_work",
        feedbackStatus: "open",
        updatedAt: "2026-04-24T08:00:00Z",
      },
      message: {
        id: "message-1",
        role: "user",
        content: "Feedback on Buttons: Needs stronger focus state",
        metadata: null,
        createdAt: "2026-04-24T08:00:00Z",
      },
    });

    await designApi.createStyleguideFeedback({
      designSystemId: "design-system-1",
      itemId: "components.buttons",
      feedback: "Needs stronger focus state",
      conversationId: "conversation-1",
    });

    expect(mockInvoke).toHaveBeenCalledWith("create_design_styleguide_feedback", {
      input: {
        designSystemId: "design-system-1",
        itemId: "components.buttons",
        feedback: "Needs stronger focus state",
        conversationId: "conversation-1",
      },
    });
  });
});
