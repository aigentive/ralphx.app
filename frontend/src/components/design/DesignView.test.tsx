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

const { saveDialogMock, useProjectsMock, writeTextFileMock } = vi.hoisted(() => ({
  saveDialogMock: vi.fn(),
  useProjectsMock: vi.fn(),
  writeTextFileMock: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: saveDialogMock,
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  writeTextFile: writeTextFileMock,
}));

vi.mock("@/hooks/useProjects", () => ({
  useProjects: () => useProjectsMock(),
}));

vi.mock("@/components/Chat/IntegratedChatPanel", () => ({
  IntegratedChatPanel: vi.fn(
    ({
      designSystemId,
      conversationIdOverride,
      sendOptions,
      headerContent,
      renderComposer,
    }: {
      designSystemId?: string;
      conversationIdOverride?: string;
      sendOptions?: { conversationId?: string | null };
      headerContent?: ReactNode;
      renderComposer?: () => ReactNode;
    }) => (
      <div
        data-testid="integrated-design-chat-panel"
        data-design-system-id={designSystemId ?? ""}
        data-conversation-id={conversationIdOverride ?? ""}
        data-send-conversation-id={sendOptions?.conversationId ?? ""}
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
  overrides: Partial<DesignStyleguideItemResponse> = {},
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
    ...overrides,
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
    saveDialogMock.mockReset();
    writeTextFileMock.mockReset();
    saveDialogMock.mockResolvedValue("/tmp/ralphx-design-system-export.json");
    writeTextFileMock.mockResolvedValue(undefined);
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
          sources: [
            designSystemSource(
              designSystem.id,
              designSystem.primaryProjectId,
              `source-${designSystem.id}-primary`,
            ),
          ],
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
    vi.spyOn(api.design, "generateArtifact").mockImplementation(async (input) => ({
      designSystemId: input.designSystemId,
      schemaVersionId: "schema-version-1",
      runId: "run-generated-artifact",
      artifactId: "generated-component-artifact-1",
      previewArtifactId: "generated-component-preview-1",
      artifactKind: input.artifactKind,
      name: input.name,
      createdAt: "2026-04-24T08:00:00Z",
      content: {
        design_system_id: input.designSystemId,
        kind: input.artifactKind,
        name: input.name,
        artifact: {
          storage: "ralphx_owned",
          project_write_status: "not_written",
        },
      },
    }));
  });

  it("renders a project-grouped design sidebar and styleguide pane", async () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    expect(screen.getByTestId("design-sidebar")).toBeInTheDocument();
    expect(await screen.findByTestId("design-styleguide-pane")).toBeInTheDocument();
    expect(await screen.findByTestId("design-system-design-system-project-1")).toHaveTextContent(
      "RalphX Design System",
    );
    expect(await screen.findByTestId("design-chat-context")).toHaveTextContent(
      "Design steward · ready · 2 sources",
    );
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
            summary: "Only fallback source references matched this row; review before treating it as canonical.",
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
    expect(screen.getByTestId("design-caveat")).toHaveTextContent("Source review needed: Buttons");
    expect(screen.getByTestId("design-caveat")).toHaveTextContent(
      "Buttons used fallback source references",
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
    expect(screen.getByTestId("design-component-preview")).toHaveTextContent("Primary");
    expect(screen.getByTestId("design-component-preview")).toHaveTextContent("Secondary");

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
    expect(panel).toHaveAttribute("data-send-conversation-id", "conversation-design-system-project-1");
    expect(screen.getByTestId("design-chat-context")).toHaveTextContent("Design steward · ready · 2 sources");
    expect(screen.queryByTestId("design-chat-runtime-pending")).not.toBeInTheDocument();
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

  it("keeps the source composer open and shows create failures", async () => {
    const { toast } = await import("sonner");
    vi.spyOn(api.design, "createDesignSystem").mockRejectedValueOnce(
      new Error("Failed to canonicalize selected design source path: No such file or directory"),
    );
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-system-design-system-project-1");

    fireEvent.click(screen.getByTestId("design-new-system"));
    await screen.findByTestId("design-source-composer");
    fireEvent.change(screen.getByTestId("design-primary-paths"), {
      target: { value: "components" },
    });
    fireEvent.click(screen.getByTestId("design-create-from-sources"));

    expect(await screen.findByTestId("design-source-create-error")).toHaveTextContent(
      "Failed to canonicalize selected design source path",
    );
    expect(screen.getByTestId("design-source-composer")).toBeInTheDocument();
    expect(toast.error).toHaveBeenCalledWith("Failed to create design system", {
      description: "Failed to canonicalize selected design source path: No such file or directory",
    });
  });

  it("generates a draft styleguide through the backend publish command", async () => {
    const { toast } = await import("sonner");
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
    expect(await screen.findByTestId("design-generation-result")).toHaveTextContent(
      "Styleguide generated with 1 review row",
    );
    expect(toast.success).toHaveBeenCalledWith("Styleguide generated", {
      description: "1 review row",
    });
  });

  it("keeps styleguide generation available for ready systems", async () => {
    const generateSpy = vi.spyOn(api.design, "generateStyleguide");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-styleguide-pane");
    await screen.findByText("Design steward · ready · 2 sources");

    expect(screen.getByTestId("design-generate-styleguide")).toHaveTextContent("Regenerate");
    fireEvent.click(screen.getByTestId("design-generate-styleguide"));

    await waitFor(() => {
      expect(generateSpy).toHaveBeenCalledWith("design-system-project-1");
    });
  });

  it("exports the selected design system package", async () => {
    const { toast } = await import("sonner");
    const exportSpy = vi.spyOn(api.design, "exportPackage");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-styleguide-pane");

    fireEvent.click(screen.getByTestId("design-export-package"));

    await waitFor(() => {
      expect(exportSpy).toHaveBeenCalledWith("design-system-project-1");
    });
    expect(await screen.findByTestId("design-export-result")).toHaveTextContent(
      "Export ready",
    );
    expect(screen.getByTestId("design-export-result")).toHaveTextContent(
      "export-package-1",
    );
    expect(screen.getByTestId("design-download-export-package")).toHaveTextContent(
      "Download JSON",
    );
    expect(toast.success).toHaveBeenCalledWith("Design package exported", {
      description: "Artifact export-p is ready to download.",
    });

    fireEvent.click(screen.getByTestId("design-download-export-package"));

    await waitFor(() => {
      expect(saveDialogMock).toHaveBeenCalledWith({
        filters: [{ name: "RalphX Design Package", extensions: ["json"] }],
        defaultPath: "ralphx-design-system-ralphx-design-system-export-p.json",
      });
    });
    expect(writeTextFileMock).toHaveBeenCalledWith(
      "/tmp/ralphx-design-system-export.json",
      JSON.stringify({ package_version: "1.0", redacted: true }, null, 2),
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
    expect(await screen.findByText("Design steward · ready · 1 source")).toBeInTheDocument();
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
      "component sample / 1 source",
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

  it("renders every persisted styleguide preview kind as a visual sample", async () => {
    const previewCases = [
      {
        itemId: "colors.primary_palette",
        group: "colors" as const,
        label: "Primary palette",
        summary: "Primary, hover, soft, and ring roles.",
        previewArtifactId: "preview-colors",
        previewKind: "color_swatch",
        testId: "design-color-preview",
        expectedText: "Primary palette",
      },
      {
        itemId: "type.typography_scale",
        group: "type" as const,
        label: "Typography scale",
        summary: "Text hierarchy, label density, and code-font usage.",
        previewArtifactId: "preview-type",
        previewKind: "typography_sample",
        testId: "design-typography-preview",
        expectedText: "Review generated artifacts",
      },
      {
        itemId: "components.buttons",
        group: "components" as const,
        label: "Buttons",
        summary: "Button patterns from persisted preview",
        previewArtifactId: "preview-components",
        previewKind: "component_sample",
        testId: "design-component-preview",
        expectedText: "Ask Design to refine this component",
      },
      {
        itemId: "spacing.radii_elevation",
        group: "spacing" as const,
        label: "Spacing, radii, and elevation",
        summary: "Panel spacing, control radius, borders, focus rings, and elevation rules.",
        previewArtifactId: "preview-spacing",
        previewKind: "spacing_sample",
        testId: "design-spacing-preview",
        expectedText: "spacing sample",
      },
      {
        itemId: "ui_kit.workspace_surfaces",
        group: "ui_kit" as const,
        label: "Workspace surfaces",
        summary: "Reviewable layout and pane patterns inferred from source-backed UI.",
        previewArtifactId: "preview-layout",
        previewKind: "layout_sample",
        testId: "design-layout-preview",
        expectedText: "layout sample",
      },
      {
        itemId: "brand.visual_identity",
        group: "brand" as const,
        label: "Visual identity assets",
        summary: "Logo, icon, asset, and brand-adjacent source references.",
        previewArtifactId: "preview-brand",
        previewKind: "asset_sample",
        testId: "design-asset-preview",
        expectedText: "Visual identity assets",
      },
    ];
    vi.mocked(api.design.listStyleguideItems).mockResolvedValue([
      ...previewCases.map((previewCase) =>
        styleguideItemResponse("design-system-project-1", previewCase.itemId, {
          id: `item-${previewCase.itemId}`,
          group: previewCase.group,
          label: previewCase.label,
          summary: previewCase.summary,
          previewArtifactId: previewCase.previewArtifactId,
          sourceRefs: [
            { project_id: "project-1", path: "frontend/src/components/Chat/TextBubble.tsx" },
            { project_id: "project-1", path: "frontend/src/components/Chat/TeamContextBar.tsx" },
            { project_id: "project-1", path: "frontend/src/styles/index.css" },
          ],
          confidence: "high",
        }),
      ),
    ]);
    vi.mocked(api.design.getStyleguidePreview).mockImplementation(
      async (designSystemId: string, previewArtifactId: string) => {
        const previewCase =
          previewCases.find((candidate) => candidate.previewArtifactId === previewArtifactId) ??
          previewCases[0]!;
        return {
          designSystemId,
          schemaVersionId: "schema-version-1",
          artifactId: previewArtifactId,
          artifactType: "design_doc",
          content: {
            design_system_id: designSystemId,
            schema_version_id: "schema-version-1",
            item_id: previewCase.itemId,
            group: previewCase.group,
            label: previewCase.label,
            summary: previewCase.summary,
            preview_kind: previewCase.previewKind,
            confidence: "high",
            source_refs: [
              { project_id: "project-1", path: "frontend/src/components/Chat/TextBubble.tsx" },
              { project_id: "project-1", path: "frontend/src/components/Chat/TeamContextBar.tsx" },
              { project_id: "project-1", path: "frontend/src/styles/index.css" },
            ],
            generated_at: "2026-04-24T08:00:00Z",
          },
        };
      },
    );
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-styleguide-row-ui_kit.workspace_surfaces");

    for (const previewCase of previewCases) {
      fireEvent.click(await screen.findByTestId(`design-styleguide-row-${previewCase.itemId}`));
      expect(await screen.findByTestId(previewCase.testId)).toHaveTextContent(
        previewCase.expectedText,
      );
      expect(
        await screen.findByText(`${previewCase.previewKind.replace(/_/g, " ")} / 3 sources`),
      ).toBeInTheDocument();
      expect(screen.getByTestId(previewCase.testId)).toContainElement(
        screen.getByText(`${previewCase.previewKind.replace(/_/g, " ")} / 3 sources`),
      );
    }
  });

  it("generates a schema-aligned artifact from a persisted styleguide row", async () => {
    vi.mocked(api.design.listStyleguideItems).mockResolvedValue([
      styleguideItemResponse("design-system-project-1"),
    ]);
    const generateArtifactSpy = vi.spyOn(api.design, "generateArtifact");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    await screen.findByText("Button patterns from persisted styleguide rows");
    const row = await screen.findByTestId("design-styleguide-row-components.buttons");
    fireEvent.click(row);
    fireEvent.click(screen.getByTestId("design-generate-artifact-components.buttons"));

    await waitFor(() => {
      expect(generateArtifactSpy).toHaveBeenCalledWith({
        designSystemId: "design-system-project-1",
        artifactKind: "component",
        name: "Buttons component",
        brief: "Button patterns from persisted styleguide rows",
        sourceItemId: "components.buttons",
      });
    });
    expect(await screen.findByTestId("design-generated-artifact-result")).toHaveTextContent(
      "Generated component artifact",
    );
  });

  it("opens a focused preview drawer for a styleguide row", async () => {
    vi.mocked(api.design.listStyleguideItems).mockResolvedValue([
      styleguideItemResponse("design-system-project-1"),
    ]);
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    await screen.findByText("Button patterns from persisted styleguide rows");
    fireEvent.click(await screen.findByTestId("design-styleguide-row-components.buttons"));
    fireEvent.click(screen.getByTestId("design-open-full-preview-components.buttons"));

    expect(await screen.findByTestId("design-focused-item-drawer")).toHaveTextContent("Buttons");
    expect(screen.getByTestId("design-focused-item-drawer")).toHaveTextContent(
      "frontend/src/Button.tsx",
    );
    fireEvent.click(screen.getByTestId("design-close-focused-preview"));
    expect(screen.queryByTestId("design-focused-item-drawer")).not.toBeInTheDocument();
  });
});
