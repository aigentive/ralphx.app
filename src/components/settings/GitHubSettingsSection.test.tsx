/**
 * GitHubSettingsSection Tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { GitHubSettingsSection } from "./GitHubSettingsSection";

// Mock the github settings hooks
vi.mock("@/hooks/useGithubSettings", () => ({
  useGitRemoteUrl: vi.fn(),
  useGhAuthStatus: vi.fn(),
  useUpdateGithubPrEnabled: vi.fn(),
}));

// Mock the project store
vi.mock("@/stores/projectStore", () => ({
  useProjectStore: vi.fn(),
  selectActiveProject: vi.fn(),
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

describe("GitHubSettingsSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock: returns the active project when called as useProjectStore(selector)
    vi.mocked(useProjectStore).mockReturnValue(mockProject);

    // Default: GitHub remote URL
    vi.mocked(useGitRemoteUrl).mockReturnValue({
      data: "https://github.com/user/repo.git",
      isLoading: false,
    } as ReturnType<typeof useGitRemoteUrl>);

    // Default: authenticated
    vi.mocked(useGhAuthStatus).mockReturnValue({
      data: true,
      isLoading: false,
    } as ReturnType<typeof useGhAuthStatus>);

    // Default: mutation setup
    vi.mocked(useUpdateGithubPrEnabled).mockReturnValue({
      mutateAsync: mockMutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof useUpdateGithubPrEnabled>);
  });

  it("renders null when no project selected", () => {
    vi.mocked(useProjectStore).mockReturnValue(null);

    const { container } = render(<GitHubSettingsSection />, {
      wrapper: createWrapper(),
    });

    expect(container.firstChild).toBeNull();
  });

  it("shows remote URL when loaded", () => {
    render(<GitHubSettingsSection />, { wrapper: createWrapper() });

    expect(
      screen.getByText("https://github.com/user/repo.git")
    ).toBeInTheDocument();
  });

  it("shows Authenticated when gh authed", () => {
    render(<GitHubSettingsSection />, { wrapper: createWrapper() });

    expect(screen.getByText("Authenticated")).toBeInTheDocument();
  });

  it("shows Not authenticated when gh not authed", () => {
    vi.mocked(useGhAuthStatus).mockReturnValue({
      data: false,
      isLoading: false,
    } as ReturnType<typeof useGhAuthStatus>);

    render(<GitHubSettingsSection />, { wrapper: createWrapper() });

    expect(screen.getByText("Not authenticated")).toBeInTheDocument();
  });

  it("disables toggle when remote is not GitHub", () => {
    vi.mocked(useGitRemoteUrl).mockReturnValue({
      data: "https://gitlab.com/user/repo.git",
      isLoading: false,
    } as ReturnType<typeof useGitRemoteUrl>);

    render(<GitHubSettingsSection />, { wrapper: createWrapper() });

    const toggle = screen.getByTestId("github-pr-enabled");
    expect(toggle).toBeDisabled();
  });

  it("enables toggle when remote is GitHub", () => {
    render(<GitHubSettingsSection />, { wrapper: createWrapper() });

    const toggle = screen.getByTestId("github-pr-enabled");
    expect(toggle).not.toBeDisabled();
  });

  it("calls updatePrEnabled.mutateAsync on toggle", async () => {
    const user = userEvent.setup();
    mockMutateAsync.mockResolvedValue(undefined);

    render(<GitHubSettingsSection />, { wrapper: createWrapper() });

    const toggle = screen.getByTestId("github-pr-enabled");
    await user.click(toggle);

    await waitFor(() => {
      expect(mockMutateAsync).toHaveBeenCalledWith({
        projectId: "proj-1",
        enabled: true, // !project.githubPrEnabled (false) = true
      });
    });
  });
});
