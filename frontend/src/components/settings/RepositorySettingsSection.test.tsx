import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RepositorySettingsSection } from "./RepositorySettingsSection";

vi.mock("@/hooks/useGithubSettings", () => ({
  useGitRemoteUrl: vi.fn(),
  useGhAuthStatus: vi.fn(),
  useUpdateGithubPrEnabled: vi.fn(),
}));

vi.mock("@/stores/projectStore", () => ({
  useProjectStore: vi.fn(),
  selectActiveProject: vi.fn(),
}));

vi.mock("@/lib/tauri", () => ({
  api: {
    projects: {
      update: vi.fn(),
    },
  },
  getGitDefaultBranch: vi.fn(),
}));

import {
  useGitRemoteUrl,
  useGhAuthStatus,
  useUpdateGithubPrEnabled,
} from "@/hooks/useGithubSettings";
import { useProjectStore } from "@/stores/projectStore";

const mockProject = {
  id: "proj-1",
  name: "Test Project",
  githubPrEnabled: false,
  workingDirectory: "/home/user/project",
  baseBranch: "main",
  useFeatureBranches: false,
  mergeValidationMode: "block" as const,
  worktreeParentDirectory: null,
  createdAt: "2024-01-01T00:00:00Z",
  updatedAt: "2024-01-01T00:00:00Z",
};

const mockMutateAsync = vi.fn();

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

describe("RepositorySettingsSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(useProjectStore).mockReturnValue(mockProject);

    vi.mocked(useGitRemoteUrl).mockReturnValue({
      data: "https://github.com/user/repo.git",
      isLoading: false,
    } as ReturnType<typeof useGitRemoteUrl>);

    vi.mocked(useGhAuthStatus).mockReturnValue({
      data: true,
      isLoading: false,
    } as ReturnType<typeof useGhAuthStatus>);

    vi.mocked(useUpdateGithubPrEnabled).mockReturnValue({
      mutateAsync: mockMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof useUpdateGithubPrEnabled>);
  });

  it("renders null when no project selected", () => {
    vi.mocked(useProjectStore).mockReturnValue(null);

    const { container } = render(<RepositorySettingsSection />, {
      wrapper: createWrapper(),
    });

    expect(container.firstChild).toBeNull();
  });

  it("renders Repository section title", () => {
    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    expect(screen.getByText("Repository")).toBeInTheDocument();
  });

  it("renders Branching, Merge Behavior, and Diagnostics subsections", () => {
    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    expect(screen.getByText("Branching")).toBeInTheDocument();
    expect(screen.getByText("Merge Behavior")).toBeInTheDocument();
    expect(screen.getByText("Diagnostics")).toBeInTheDocument();
  });

  it("shows remote URL in Diagnostics", () => {
    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    expect(
      screen.getByText("https://github.com/user/repo.git")
    ).toBeInTheDocument();
  });

  it("shows Authenticated when gh authed", () => {
    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    expect(screen.getByText("Authenticated")).toBeInTheDocument();
  });

  it("shows Not authenticated when gh not authed", () => {
    vi.mocked(useGhAuthStatus).mockReturnValue({
      data: false,
      isLoading: false,
    } as ReturnType<typeof useGhAuthStatus>);

    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    expect(screen.getByText("Not authenticated")).toBeInTheDocument();
  });

  it("disables PR mode toggle when remote is not GitHub", () => {
    vi.mocked(useGitRemoteUrl).mockReturnValue({
      data: "https://gitlab.com/user/repo.git",
      isLoading: false,
    } as ReturnType<typeof useGitRemoteUrl>);

    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    const toggle = screen.getByTestId("github-pr-enabled");
    expect(toggle).toBeDisabled();
  });

  it("enables PR mode toggle when remote is GitHub", () => {
    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    const toggle = screen.getByTestId("github-pr-enabled");
    expect(toggle).not.toBeDisabled();
  });

  it("enables PR mode toggle for GitHub SSH remotes", () => {
    vi.mocked(useGitRemoteUrl).mockReturnValue({
      data: "git@github.com:user/repo.git",
      isLoading: false,
    } as ReturnType<typeof useGitRemoteUrl>);

    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    const toggle = screen.getByTestId("github-pr-enabled");
    expect(toggle).not.toBeDisabled();
  });

  it("disables PR mode toggle for URLs that only mention github.com in a query string", () => {
    vi.mocked(useGitRemoteUrl).mockReturnValue({
      data: "https://evil.example.com/redirect?target=https://github.com/user/repo.git",
      isLoading: false,
    } as ReturnType<typeof useGitRemoteUrl>);

    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    const toggle = screen.getByTestId("github-pr-enabled");
    expect(toggle).toBeDisabled();
  });

  it("calls updatePrEnabled.mutateAsync on PR toggle", async () => {
    const user = userEvent.setup();
    mockMutateAsync.mockResolvedValue(undefined);

    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    const toggle = screen.getByTestId("github-pr-enabled");
    await user.click(toggle);

    await waitFor(() => {
      expect(mockMutateAsync).toHaveBeenCalledWith({
        projectId: "proj-1",
        enabled: true,
      });
    });
  });

  it("shows base-branch and worktree-location inputs", () => {
    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    expect(screen.getByTestId("base-branch")).toBeInTheDocument();
    expect(screen.getByTestId("worktree-location")).toBeInTheDocument();
  });

  it("shows merge validation select", () => {
    render(<RepositorySettingsSection />, { wrapper: createWrapper() });

    expect(screen.getByTestId("merge-validation-mode")).toBeInTheDocument();
  });
});
