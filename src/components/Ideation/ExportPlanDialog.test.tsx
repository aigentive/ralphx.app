/**
 * ExportPlanDialog.test.tsx
 * Tests for the dialog that exports a verified ideation plan to another project
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { UseMutationResult } from "@tanstack/react-query";
import { ExportPlanDialog } from "./ExportPlanDialog";
import type { Project } from "@/types/project";
import type { IdeationSession } from "@/types/ideation";
import type { ExportPlanInput } from "@/hooks/useExportPlanToProject";

// ============================================================================
// Mocks
// ============================================================================

const mockMutate = vi.fn();
const mockReset = vi.fn();

vi.mock("@/hooks/useExportPlanToProject", () => ({
  useExportPlanToProject: () => ({
    mutate: mockMutate,
    isPending: false,
    isError: false,
    isSuccess: false,
    error: null,
    reset: mockReset,
  }),
}));

const mockProjects: Project[] = [
  {
    id: "proj-1",
    name: "Alpha Project",
    workingDirectory: "/home/user/alpha",
    gitMode: "worktree",
    baseBranch: "main",
    worktreeParentDirectory: null,
    useFeatureBranches: true,
    mergeValidationMode: "block",
    detectedAnalysis: null,
    customAnalysis: null,
    analyzedAt: null,
    githubPrEnabled: false,
    createdAt: "2026-01-01T00:00:00+00:00",
    updatedAt: "2026-01-01T00:00:00+00:00",
  },
  {
    id: "proj-2",
    name: "Beta Project",
    workingDirectory: "/home/user/beta",
    gitMode: "worktree",
    baseBranch: "main",
    worktreeParentDirectory: null,
    useFeatureBranches: true,
    mergeValidationMode: "block",
    detectedAnalysis: null,
    customAnalysis: null,
    analyzedAt: null,
    githubPrEnabled: false,
    createdAt: "2026-01-01T00:00:00+00:00",
    updatedAt: "2026-01-01T00:00:00+00:00",
  },
];

vi.mock("@/hooks/useProjects", () => ({
  useProjects: () => ({
    data: mockProjects,
    isLoading: false,
    error: null,
  }),
}));

// ============================================================================
// Helpers
// ============================================================================

function renderWithQuery(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
  );
}

const defaultProps = {
  open: true,
  onOpenChange: vi.fn(),
  sessionId: "session-abc",
  sessionTitle: "My Verified Plan",
  verificationStatus: "verified",
};

// ============================================================================
// Tests
// ============================================================================

describe("ExportPlanDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders title 'Export Plan to Project' when open", () => {
    renderWithQuery(<ExportPlanDialog {...defaultProps} />);
    expect(screen.getByText("Export Plan to Project")).toBeInTheDocument();
  });

  it("renders source plan title and verification badge", () => {
    renderWithQuery(<ExportPlanDialog {...defaultProps} />);
    expect(screen.getByText("My Verified Plan")).toBeInTheDocument();
    expect(screen.getByText("Verified")).toBeInTheDocument();
  });

  it("renders 'Verified (imported)' for imported_verified status", () => {
    renderWithQuery(
      <ExportPlanDialog
        {...defaultProps}
        verificationStatus="imported_verified"
      />
    );
    expect(screen.getByText("Verified (imported)")).toBeInTheDocument();
  });

  it("renders 'Verified' for verified status", () => {
    renderWithQuery(
      <ExportPlanDialog {...defaultProps} verificationStatus="verified" />
    );
    expect(screen.getByText("Verified")).toBeInTheDocument();
  });

  it("'Create in Project' button is disabled when path is empty", () => {
    renderWithQuery(<ExportPlanDialog {...defaultProps} />);
    const button = screen.getByRole("button", { name: /Create in Project/i });
    expect(button).toBeDisabled();
  });

  it("'Create in Project' button is enabled when path is typed", async () => {
    const user = userEvent.setup();
    renderWithQuery(<ExportPlanDialog {...defaultProps} />);

    const input = screen.getByPlaceholderText("/path/to/project");
    await user.type(input, "/some/path");

    const button = screen.getByRole("button", { name: /Create in Project/i });
    expect(button).not.toBeDisabled();
  });

  it("calls mutation.mutate with correct args on submit", async () => {
    const user = userEvent.setup();
    renderWithQuery(<ExportPlanDialog {...defaultProps} />);

    const input = screen.getByPlaceholderText("/path/to/project");
    await user.type(input, "/target/project");

    const button = screen.getByRole("button", { name: /Create in Project/i });
    await user.click(button);

    expect(mockMutate).toHaveBeenCalledTimes(1);
    expect(mockMutate).toHaveBeenCalledWith(
      {
        targetProjectPath: "/target/project",
        sourceSessionId: "session-abc",
      },
      expect.objectContaining({ onSuccess: expect.any(Function) })
    );
  });

  it("shows autocomplete dropdown with matching projects when typing", async () => {
    const user = userEvent.setup();
    renderWithQuery(<ExportPlanDialog {...defaultProps} />);

    const input = screen.getByPlaceholderText("/path/to/project");
    await user.type(input, "/home/user");

    expect(screen.getByText("Alpha Project")).toBeInTheDocument();
    expect(screen.getByText("/home/user/alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta Project")).toBeInTheDocument();
    expect(screen.getByText("/home/user/beta")).toBeInTheDocument();
  });

  it("clicking autocomplete suggestion fills input", async () => {
    const user = userEvent.setup();
    renderWithQuery(<ExportPlanDialog {...defaultProps} />);

    const input = screen.getByPlaceholderText("/path/to/project");
    await user.type(input, "/home/user");

    const suggestion = screen.getByText("Alpha Project");
    await user.click(suggestion);

    expect(input).toHaveValue("/home/user/alpha");
  });

  it("shows error message when mutation fails", () => {
    // The top-level vi.mock is already set up; override for this test by using
    // a separate mock module approach via the factory pattern below.
    // This test is covered by "shows error message when isError is true".
  });

  it("shows error message when isError is true", async () => {
    const mod = await import("@/hooks/useExportPlanToProject");
    vi.spyOn(mod, "useExportPlanToProject").mockReturnValueOnce({
      mutate: mockMutate,
      isPending: false,
      isError: true,
      isSuccess: false,
      error: new Error("Export failed"),
      reset: mockReset,
      status: "error",
      failureCount: 1,
      failureReason: new Error("Export failed"),
      isIdle: false,
      isPaused: false,
      submittedAt: Date.now(),
      variables: undefined,
      context: undefined,
      data: undefined,
      mutateAsync: vi.fn(),
    } satisfies UseMutationResult<IdeationSession, Error, ExportPlanInput>);

    renderWithQuery(<ExportPlanDialog {...defaultProps} />);
    expect(screen.getByText("Export failed")).toBeInTheDocument();
  });

  it("shows success state after successful mutation via onSuccess callback", async () => {
    const user = userEvent.setup();

    mockMutate.mockImplementation(
      (
        _args: unknown,
        options: {
          onSuccess: (session: {
            id: string;
            title: string;
            projectId: string;
            status: string;
            verificationStatus: string;
          }) => void;
        }
      ) => {
        options.onSuccess({
          id: "new-session-1",
          title: "Exported Session",
          projectId: "proj-1",
          status: "active",
          verificationStatus: "imported_verified",
        });
      }
    );

    renderWithQuery(<ExportPlanDialog {...defaultProps} />);

    const input = screen.getByPlaceholderText("/path/to/project");
    await user.type(input, "/target/project");

    const button = screen.getByRole("button", { name: /Create in Project/i });
    await user.click(button);

    expect(screen.getByText("Plan exported successfully")).toBeInTheDocument();
    expect(screen.getByText(/Exported Session/)).toBeInTheDocument();
  });

  it("does not call mutation when path is empty", async () => {
    const user = userEvent.setup();
    renderWithQuery(<ExportPlanDialog {...defaultProps} />);

    // Button should be disabled — attempt click anyway
    const button = screen.getByRole("button", { name: /Create in Project/i });
    // The button is disabled so userEvent won't click it, but verify the button state
    expect(button).toBeDisabled();
    expect(mockMutate).not.toHaveBeenCalled();

    // Also verify that even with programmatic interaction, mutate is not called
    await user.click(button);
    expect(mockMutate).not.toHaveBeenCalled();
  });
});
