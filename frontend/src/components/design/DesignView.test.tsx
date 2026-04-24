import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "@/components/ui/tooltip";
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
    useProjectsMock.mockReturnValue({ data: projects, isLoading: false });
  });

  it("renders a project-grouped design sidebar and styleguide pane", () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    expect(screen.getByTestId("design-sidebar")).toBeInTheDocument();
    expect(screen.getByTestId("design-styleguide-pane")).toBeInTheDocument();
    expect(screen.getByTestId("design-system-design-system-project-1")).toHaveTextContent(
      "RalphX Design System",
    );
    expect(screen.getByTestId("design-styleguide-group-colors")).toHaveTextContent(
      "Primary palette",
    );
  });

  it("filters design systems by project or design-system name", () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    fireEvent.click(screen.getByTestId("design-search-toggle"));
    fireEvent.change(screen.getByTestId("design-search-input"), {
      target: { value: "Docs" },
    });

    expect(screen.getByTestId("design-system-design-system-project-2")).toHaveTextContent(
      "Docs Design System",
    );
    expect(screen.queryByTestId("design-system-design-system-project-1")).not.toBeInTheDocument();
  });

  it("expands preview rows and opens the feedback composer", () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    fireEvent.click(screen.getByTestId("design-styleguide-row-components.buttons"));
    expect(screen.getByTestId("design-component-preview")).toHaveTextContent(
      "design-preview-buttons",
    );

    fireEvent.click(screen.getByTestId("design-needs-work-components.buttons"));
    expect(screen.getByTestId("design-feedback-composer")).toBeInTheDocument();
  });

  it("records a local approval state for a styleguide row", () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    const row = screen.getByTestId("design-styleguide-row-colors.primary_palette");
    fireEvent.click(row);
    fireEvent.click(screen.getByTestId("design-approve-colors.primary_palette"));

    expect(within(row).getByText("approved")).toBeInTheDocument();
  });

  it("keeps composer messages scoped to the selected design system surface", () => {
    renderWithProviders(<DesignView projectId="project-1" onCreateProject={vi.fn()} />);

    fireEvent.change(screen.getByTestId("design-composer-input"), {
      target: { value: "Generate a settings screen" },
    });
    fireEvent.click(screen.getByTestId("design-composer-submit"));

    expect(screen.getByText("Generate a settings screen")).toBeInTheDocument();
  });
});
