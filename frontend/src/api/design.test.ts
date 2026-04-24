import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  CreateDesignStyleguideFeedbackResponseSchema,
  CreateDesignSystemResponseSchema,
  DesignSystemResponseSchema,
  GenerateDesignSystemStyleguideResponseSchema,
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
