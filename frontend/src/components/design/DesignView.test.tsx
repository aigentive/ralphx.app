import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import {
  api,
  type CreateDesignSystemInput,
  type DesignStyleguideItemResponse,
  type DesignSystemResponse,
  type DesignSystemSourceResponse,
} from "@/lib/tauri";
import { DesignView } from "./DesignView";

const { useProjectsMock } = vi.hoisted(() => ({
  useProjectsMock: vi.fn(),
}));

vi.mock("@/hooks/useProjects", () => ({
  useProjects: () => useProjectsMock(),
}));

vi.mock("@/components/Chat/IntegratedChatPanel", () => ({
  IntegratedChatPanel: vi.fn(
    ({
      designSystemId,
      conversationIdOverride,
      headerContent,
      renderComposer,
    }: {
      designSystemId?: string;
      conversationIdOverride?: string;
      headerContent?: ReactNode;
      renderComposer?: () => ReactNode;
    }) => (
      <div
        data-testid="integrated-design-chat-panel"
        data-design-system-id={designSystemId ?? ""}
        data-conversation-id={conversationIdOverride ?? ""}
      >
        {headerContent}
        {renderComposer?.()}
      </div>
    ),
  ),
}));

const projects = [
  {
    id: "project-1",
    name: "RalphX",
    workingDirectory: "/tmp/ralphx",
    gitMode: "worktree" as const,
    baseBranch: null,
    worktreeParentDirectory: null,
    useFeatureBranches: true,
    mergeValidationMode: "block" as const,
    detectedAnalysis: null,
    customAnalysis: null,
    analyzedAt: null,
    githubPrEnabled: false,
    createdAt: "2026-04-24T08:00:00Z",
    updatedAt: "2026-04-24T08:00:00Z",
  },
  {
    id: "project-2",
    name: "Docs",
    workingDirectory: "/tmp/docs",
    gitMode: "worktree" as const,
    baseBranch: null,
    worktreeParentDirectory: null,
    useFeatureBranches: true,
    mergeValidationMode: "block" as const,
    detectedAnalysis: null,
    customAnalysis: null,
    analyzedAt: null,
    githubPrEnabled: false,
    createdAt: "2026-04-24T08:00:00Z",
    updatedAt: "2026-04-24T08:00:00Z",
  },
];

function designSystemResponse(
  projectId: string,
  name: string,
  overrides: Partial<DesignSystemResponse> = {},
): DesignSystemResponse {
  return {
    id: `design-system-${projectId}`,
    primaryProjectId: projectId,
    name,
    description: null,
    status: "ready",
    currentSchemaVersionId: "schema-version-1",
    storageRootRef: "design-root",
    createdAt: "2026-04-24T08:00:00Z",
    updatedAt: "2026-04-24T08:00:00Z",
    archivedAt: null,
    ...overrides,
  };
}

function designSystemSource(
  designSystemId: string,
  projectId: string,
  id: string,
): DesignSystemSourceResponse {
  return {
    id,
    designSystemId,
    projectId,
    role: "primary",
    selectedPaths: [],
    sourceKind: "project_checkout",
    gitCommit: null,
    sourceHashes: {},
    lastAnalyzedAt: null,
  };
}

function styleguideItemResponse(
  designSystemId: string,
  itemId = "components.buttons",
): DesignStyleguideItemResponse {
  return {
    id: `item-${itemId}`,
    designSystemId,
    schemaVersionId: "schema-version-1",
    itemId,
    group: "components",
    label: "Buttons",
    summary: "Button patterns from persisted styleguide rows",
    previewArtifactId: "preview-buttons",
    sourceRefs: [{ project_id: "project-1", path: "frontend/src/Button.tsx", line: 12 }],
    confidence: "medium",
    approvalStatus: "needs_review",
    feedbackStatus: "none",
    updatedAt: "2026-04-24T08:00:00Z",
  };
}

function renderWithProviders(ui: ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>{ui}</TooltipProvider>
    </QueryClientProvider>,
  );
}

describe("DesignView", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useProjectsMock.mockReturnValue({ data: projects, isLoading: false });
    const systemsByProject = new Map(
      projects.map((project) => [
        project.id,
        [designSystemResponse(project.id, `${project.name} Design System`)],
      ]),
    );
    vi.spyOn(api.design, "listProjectDesignSystems").mockImplementation(
      async (projectId: string) => systemsByProject.get(projectId) ?? [],
    );
    vi.spyOn(api.design, "getDesignSystem").mockImplementation(async (id: string) => {
      const designSystem = Array.from(systemsByProject.values()).flat().find((system) => system.id === id);
      if (!designSystem) {
        return null;
      }
      return {
        designSystem,
        sources: [
          designSystemSource(designSystem.id, designSystem.primaryProjectId, "source-primary"),
          designSystemSource(designSystem.id, "project-2", "source-reference"),
        ],
        conversation: {
          id: `conversation-${designSystem.id}`,
          contextType: "design",
          contextId: designSystem.id,
          title: `Design: ${designSystem.name}`,
          messageCount: 0,
          lastMessageAt: null,
          createdAt: "2026-04-24T08:00:00Z",
          updatedAt: "2026-04-24T08:00:00Z",
          archivedAt: null,
        },
      };
    });
    vi.spyOn(api.design, "listStyleguideItems").mockResolvedValue([]);
    vi.spyOn(api.design, "getStyleguideViewModel").mockResolvedValue(null);
    vi.spyOn(api.design, "getStyleguidePreview").mockImplementation(
      async (designSystemId: string, previewArtifactId: string) => ({
        designSystemId,
        schemaVersionId: "schema-version-1",
        artifactId: previewArtifactId,
        artifactType: "design_doc",
        content: {
          design_system_id: designSystemId,
          schema_version_id: "schema-version-1",
          item_id: "components.buttons",
          group: "components",
          label: "Buttons",
          summary: "Button patterns from persisted preview",
          preview_kind: "component_sample",
          confidence: "medium",
          source_refs: [{ project_id: "project-1", path: "frontend/src/Button.tsx", line: 12 }],
          generated_at: "2026-04-24T08:00:00Z",
        },
      }),
    );
    vi.spyOn(api.design, "approveStyleguideItem").mockImplementation(
      async (designSystemId: string, itemId: string) => ({
        ...styleguideItemResponse(designSystemId, itemId),
        approvalStatus: "approved",
        feedbackStatus: "resolved",
      }),
    );
    vi.spyOn(api.design, "createStyleguideFeedback").mockImplementation(async (input) => ({
      feedback: {
        id: "feedback-1",
        designSystemId: input.designSystemId,
        schemaVersionId: "schema-version-1",
        itemId: input.itemId,
        conversationId: input.conversationId ?? "conversation-1",
        messageId: "message-1",
        previewArtifactId: null,
        sourceRefs: [],
        feedback: input.feedback,
        status: "open",
        createdAt: "2026-04-24T08:00:00Z",
        resolvedAt: null,
      },
      item: {
        ...styleguideItemResponse(input.designSystemId, input.itemId),
        approvalStatus: "needs_work",
        feedbackStatus: "open",
      },
      message: {
        id: "message-1",
        role: "user",
        content: input.feedback,
        metadata: null,
        createdAt: "2026-04-24T08:00:00Z",
      },
    }));
    vi.spyOn(api.design, "createDesignSystem").mockImplementation(
      async (input: CreateDesignSystemInput) => {
        const designSystem = designSystemResponse(input.primaryProjectId, input.name, {
          id: `created-design-system-${input.primaryProjectId}`,
          status: "draft",
          currentSchemaVersionId: null,
        });
        systemsByProject.set(input.primaryProjectId, [
          designSystem,
          ...(systemsByProject.get(input.primaryProjectId) ?? []),
        ]);
        return {
          designSystem,
          sources: [],
          conversation: {
            id: `conversation-${input.primaryProjectId}`,
            contextType: "design",
            contextId: `created-design-system-${input.primaryProjectId}`,
            title: `Design: ${input.name}`,
            messageCount: 0,
            lastMessageAt: null,
            createdAt: "2026-04-24T08:00:00Z",
            updatedAt: "2026-04-24T08:00:00Z",
            archivedAt: null,
          },
        };
      },
    );
    vi.spyOn(api.design, "generateStyleguide").mockImplementation(
      async (designSystemId: string) => {
        const current = Array.from(systemsByProject.values()).flat().find((system) => system.id === designSystemId);
        const designSystem = {
          ...(current ?? designSystemResponse("project-1", "Generated Design System")),
          status: "ready" as const,
          currentSchemaVersionId: "schema-version-generated",
          updatedAt: "2026-04-24T08:10:00Z",
        };
        systemsByProject.set(
          designSystem.primaryProjectId,
          (systemsByProject.get(designSystem.primaryProjectId) ?? []).map((system) =>
            system.id === designSystem.id ? designSystem : system,
          ),
        );
        return {
          designSystem,
          schemaVersionId: "schema-version-generated",
          runId: "run-generated",
          items: [styleguideItemResponse(designSystem.id)],
        };
      },
    );
    vi.spyOn(api.design, "exportPackage").mockImplementation(
      async (designSystemId: string) => ({
        designSystemId,
        schemaVersionId: "schema-version-1",
        artifactId: "export-package-1",
        redacted: true,
        exportedAt: "2026-04-24T08:00:00Z",
        content: {
          package_version: "1.0",
          redacted: true,
        },
      }),
    );
    vi.spyOn(api.design, "importPackage").mockImplementation(async (input) => {
      const designSystem = designSystemResponse(input.attachProjectId, input.name ?? "Imported Design System", {
        id: `imported-design-system-${input.attachProjectId}`,
        status: "ready",
        currentSchemaVersionId: "schema-version-imported",
        sourceCount: 1,
      });
      systemsByProject.set(input.attachProjectId, [
        designSystem,
        ...(systemsByProject.get(input.attachProjectId) ?? []),
      ]);
      return {
        designSystem,
        sources: [
          {
            id: "source-imported-primary",
            designSystemId: designSystem.id,
            projectId: input.attachProjectId,
            role: "primary",
            selectedPaths: [],
            sourceKind: "manual_note",
            gitCommit: null,
            sourceHashes: {},
            lastAnalyzedAt: "2026-04-24T08:00:00Z",
          },
        ],
        conversation: {
          id: `conversation-${designSystem.id}`,
          contextType: "design",
          contextId: designSystem.id,
          title: `Design: ${designSystem.name}`,
          messageCount: 0,
          lastMessageAt: null,
          createdAt: "2026-04-24T08:00:00Z",
          updatedAt: "2026-04-24T08:00:00Z",
          archivedAt: null,
        },
        schemaVersionId: "schema-version-imported",
        runId: "run-imported",
        packageArtifactId: input.packageArtifactId,
        items: [
          {
            ...styleguideItemResponse(designSystem.id),
            schemaVersionId: "schema-version-imported",
            previewArtifactId: null,
            sourceRefs: [],
          },
        ],
      };
    });
  });

  it("renders a project-grouped design sidebar and styleguide pane", async () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    expect(screen.getByTestId("design-sidebar")).toBeInTheDocument();
    expect(await screen.findByTestId("design-styleguide-pane")).toBeInTheDocument();
    expect(await screen.findByTestId("design-system-design-system-project-1")).toHaveTextContent(
      "RalphX Design System",
    );
    expect(await screen.findByText("ready / 2 sources")).toBeInTheDocument();
    expect(screen.getByTestId("design-styleguide-group-colors")).toHaveTextContent(
      "Primary palette",
    );
  });

  it("hydrates the styleguide pane from the persisted styleguide artifact", async () => {
    vi.mocked(api.design.getStyleguideViewModel).mockResolvedValue({
      designSystemId: "design-system-project-1",
      schemaVersionId: "schema-version-1",
      artifactId: "styleguide-artifact-1",
      artifactType: "design_doc",
      content: {
        design_system_id: "design-system-project-1",
        schema_version_id: "schema-version-1",
        version: "0.1.0",
        generated_at: "2026-04-24T08:00:00Z",
        ready_summary: "Persisted artifact summary is ready for review.",
        caveats: [
          {
            item_id: "components.buttons",
            severity: "medium",
            summary: "Check source confidence before approval.",
          },
        ],
        groups: [
          {
            id: "components",
            label: "Components",
            items: [
              {
                id: "components.buttons",
                group: "components",
                label: "Buttons",
                summary: "Buttons from the persisted styleguide artifact",
                preview_artifact_id: "preview-buttons",
                source_refs: [{ project_id: "project-1", path: "frontend/src/Button.tsx" }],
                confidence: "medium",
                approval_status: "needs_review",
                feedback_status: "none",
                updated_at: "2026-04-24T08:00:00Z",
              },
            ],
          },
        ],
      },
    });

    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    expect(await screen.findByText("Persisted artifact summary is ready for review.")).toBeInTheDocument();
    expect(await screen.findByText("Buttons from the persisted styleguide artifact")).toBeInTheDocument();
    expect(screen.getByTestId("design-caveat")).toHaveTextContent(
      "Check source confidence before approval.",
    );
  });

  it("filters design systems by project or design-system name", async () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-system-design-system-project-1");

    fireEvent.click(screen.getByTestId("design-search-toggle"));
    fireEvent.change(screen.getByTestId("design-search-input"), {
      target: { value: "Docs" },
    });

    expect(await screen.findByTestId("design-system-design-system-project-2")).toHaveTextContent(
      "Docs Design System",
    );
    expect(screen.queryByTestId("design-system-design-system-project-1")).not.toBeInTheDocument();
  });

  it("expands preview rows and opens the feedback composer", async () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-styleguide-row-components.buttons");

    fireEvent.click(screen.getByTestId("design-styleguide-row-components.buttons"));
    expect(screen.getByTestId("design-component-preview")).toHaveTextContent(
      "design-preview-buttons",
    );

    fireEvent.click(screen.getByTestId("design-needs-work-components.buttons"));
    expect(screen.getByTestId("design-feedback-composer")).toBeInTheDocument();
  });

  it("records a local approval state for a styleguide row", async () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    const row = await screen.findByTestId("design-styleguide-row-colors.primary_palette");
    fireEvent.click(row);
    fireEvent.click(screen.getByTestId("design-approve-colors.primary_palette"));

    expect(within(row).getByText("approved")).toBeInTheDocument();
  });

  it("passes the selected design conversation to the integrated chat panel", async () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    const panel = await screen.findByTestId("integrated-design-chat-panel");

    expect(panel).toHaveAttribute("data-design-system-id", "design-system-project-1");
    await waitFor(() => {
      expect(panel).toHaveAttribute("data-conversation-id", "conversation-design-system-project-1");
    });
    expect(screen.getByTestId("design-chat-runtime-pending")).toHaveTextContent(
      "Review notes appear here",
    );
  });

  it("creates a draft design system from selected source projects and paths", async () => {
    const createSpy = vi.spyOn(api.design, "createDesignSystem");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-system-design-system-project-1");

    fireEvent.click(screen.getByTestId("design-new-system"));
    await screen.findByTestId("design-source-composer");
    fireEvent.change(screen.getByTestId("design-primary-paths"), {
      target: { value: "frontend/src\ncomponents, frontend/src" },
    });
    fireEvent.click(screen.getByTestId("design-reference-source-project-2"));
    fireEvent.change(screen.getByTestId("design-reference-paths-project-2"), {
      target: { value: "docs" },
    });
    fireEvent.click(screen.getByTestId("design-create-from-sources"));

    await waitFor(() => {
      expect(createSpy.mock.calls[0]?.[0]).toEqual({
        primaryProjectId: "project-1",
        name: "RalphX Design System",
        selectedPaths: ["frontend/src", "components"],
        sources: [
          {
            projectId: "project-2",
            role: "reference",
            selectedPaths: ["docs"],
          },
        ],
      });
    });
    expect(await screen.findByTestId("design-system-created-design-system-project-1")).toHaveTextContent(
      "RalphX Design System",
    );
  });

  it("generates a draft styleguide through the backend publish command", async () => {
    const generateSpy = vi.spyOn(api.design, "generateStyleguide");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-system-design-system-project-1");

    fireEvent.click(screen.getByTestId("design-new-system"));
    await screen.findByTestId("design-source-composer");
    fireEvent.click(screen.getByTestId("design-create-from-sources"));
    await screen.findByTestId("design-system-created-design-system-project-1");
    fireEvent.click(await screen.findByTestId("design-generate-styleguide"));

    await waitFor(() => {
      expect(generateSpy).toHaveBeenCalledWith("created-design-system-project-1");
    });
    expect(await screen.findByText("Button patterns from persisted styleguide rows")).toBeInTheDocument();
  });

  it("exports the selected design system package", async () => {
    const exportSpy = vi.spyOn(api.design, "exportPackage");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-styleguide-pane");

    fireEvent.click(screen.getByTestId("design-export-package"));

    await waitFor(() => {
      expect(exportSpy).toHaveBeenCalledWith("design-system-project-1");
    });
    expect(await screen.findByTestId("design-export-result")).toHaveTextContent(
      "export-p",
    );
  });

  it("imports a design package artifact into the selected project", async () => {
    const importSpy = vi.spyOn(api.design, "importPackage");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-styleguide-pane");

    fireEvent.click(screen.getByTestId("design-import-package"));
    await screen.findByTestId("design-package-import-dialog");
    fireEvent.change(screen.getByTestId("design-import-package-artifact-id"), {
      target: { value: "export-package-1" },
    });
    fireEvent.change(screen.getByTestId("design-import-name"), {
      target: { value: "Imported Product UI" },
    });
    fireEvent.click(screen.getByTestId("design-import-project-project-2"));
    fireEvent.click(screen.getByTestId("design-import-package-submit"));

    await waitFor(() => {
      expect(importSpy).toHaveBeenCalledWith({
        packageArtifactId: "export-package-1",
        attachProjectId: "project-2",
        name: "Imported Product UI",
      });
    });
    expect(await screen.findByTestId("design-system-imported-design-system-project-2")).toHaveTextContent(
      "Imported Product UI",
    );
    expect(await screen.findByText("ready / 1 sources")).toBeInTheDocument();
  });

  it("uses backend styleguide commands for persisted rows", async () => {
    vi.mocked(api.design.listStyleguideItems).mockResolvedValue([
      styleguideItemResponse("design-system-project-1"),
    ]);
    const previewSpy = vi.spyOn(api.design, "getStyleguidePreview");
    const approveSpy = vi.spyOn(api.design, "approveStyleguideItem");
    const feedbackSpy = vi.spyOn(api.design, "createStyleguideFeedback");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    await screen.findByText("Button patterns from persisted styleguide rows");
    const row = await screen.findByTestId("design-styleguide-row-components.buttons");
    fireEvent.click(row);
    expect(await screen.findByTestId("design-preview-kind")).toHaveTextContent(
      "component sample / 1 sources",
    );
    expect(previewSpy).toHaveBeenCalledWith("design-system-project-1", "preview-buttons");
    fireEvent.click(screen.getByTestId("design-approve-components.buttons"));

    await waitFor(() => {
      expect(approveSpy).toHaveBeenCalledWith("design-system-project-1", "components.buttons");
    });

    fireEvent.click(screen.getByTestId("design-needs-work-components.buttons"));
    fireEvent.change(screen.getByPlaceholderText("Feedback"), {
      target: { value: "Use a clearer focus ring" },
    });
    fireEvent.click(screen.getByText("Send feedback to Design"));

    await waitFor(() => {
      expect(feedbackSpy).toHaveBeenCalledWith({
        designSystemId: "design-system-project-1",
        itemId: "components.buttons",
        feedback: "Use a clearer focus ring",
        conversationId: "conversation-design-system-project-1",
      });
    });
  });
});
