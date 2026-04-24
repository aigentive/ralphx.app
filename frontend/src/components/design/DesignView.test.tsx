import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
import {
  api,
  type CreateDesignSystemInput,
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

  it("keeps composer messages scoped to the selected design system surface", async () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-composer-input");

    fireEvent.change(screen.getByTestId("design-composer-input"), {
      target: { value: "Generate a settings screen" },
    });
    fireEvent.click(screen.getByTestId("design-composer-submit"));

    expect(screen.getByText("Generate a settings screen")).toBeInTheDocument();
  });

  it("creates a draft design system for the focused project", async () => {
    const createSpy = vi.spyOn(api.design, "createDesignSystem");
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);
    await screen.findByTestId("design-system-design-system-project-1");

    fireEvent.click(screen.getByTestId("design-new-system"));

    await waitFor(() => {
      expect(createSpy.mock.calls[0]?.[0]).toEqual({
        primaryProjectId: "project-1",
        name: "RalphX Design System",
        selectedPaths: [],
        sources: [],
      });
    });
    expect(await screen.findByTestId("design-system-created-design-system-project-1")).toHaveTextContent(
      "RalphX Design System",
    );
  });
});
