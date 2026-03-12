/**
 * useGithubSettings tests — verify Tauri invoke is called with correct args.
 *
 * Tests query/mutation hooks via direct invocation of the underlying functions.
 * The global Tauri mock is set up in src/test/setup.ts.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import type { ReactNode } from "react";
import {
  useGitRemoteUrl,
  useGhAuthStatus,
  useUpdateGithubPrEnabled,
} from "./useGithubSettings";

// ============================================================================
// Test helpers
// ============================================================================

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });
  return ({ children }: { children: ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);
}

// ============================================================================
// useGitRemoteUrl
// ============================================================================

describe("useGitRemoteUrl", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("is disabled (does not call invoke) when projectId is null", async () => {
    const { result } = renderHook(() => useGitRemoteUrl(null), {
      wrapper: createWrapper(),
    });

    // Query is disabled — data remains undefined, no invoke call
    expect(result.current.data).toBeUndefined();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("calls get_git_remote_url with projectId when enabled", async () => {
    vi.mocked(invoke).mockResolvedValue("https://github.com/org/repo.git");

    const { result } = renderHook(() => useGitRemoteUrl("proj-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(invoke).toHaveBeenCalledWith("get_git_remote_url", {
      projectId: "proj-1",
    });
    expect(result.current.data).toBe("https://github.com/org/repo.git");
  });

  it("returns null when backend returns null (no remote configured)", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    const { result } = renderHook(() => useGitRemoteUrl("proj-2"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it("exposes isLoading and isError states", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("network error"));

    const { result } = renderHook(() => useGitRemoteUrl("proj-3"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBeDefined();
  });
});

// ============================================================================
// useGhAuthStatus
// ============================================================================

describe("useGhAuthStatus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls check_gh_auth and returns true when authenticated", async () => {
    vi.mocked(invoke).mockResolvedValue(true);

    const { result } = renderHook(() => useGhAuthStatus(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(invoke).toHaveBeenCalledWith("check_gh_auth", {});
    expect(result.current.data).toBe(true);
  });

  it("returns false when gh CLI is not authenticated", async () => {
    vi.mocked(invoke).mockResolvedValue(false);

    const { result } = renderHook(() => useGhAuthStatus(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBe(false);
  });

  it("exposes isLoading and isError states", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("gh not installed"));

    const { result } = renderHook(() => useGhAuthStatus(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.isLoading).toBe(false);
  });
});

// ============================================================================
// useUpdateGithubPrEnabled
// ============================================================================

describe("useUpdateGithubPrEnabled", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls update_github_pr_enabled with projectId and enabled", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useUpdateGithubPrEnabled(), {
      wrapper: createWrapper(),
    });

    result.current.mutate({ projectId: "proj-1", enabled: true });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(invoke).toHaveBeenCalledWith("update_github_pr_enabled", {
      projectId: "proj-1",
      enabled: true,
    });
  });

  it("calls update_github_pr_enabled with enabled=false to disable PR mode", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useUpdateGithubPrEnabled(), {
      wrapper: createWrapper(),
    });

    result.current.mutate({ projectId: "proj-2", enabled: false });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(invoke).toHaveBeenCalledWith("update_github_pr_enabled", {
      projectId: "proj-2",
      enabled: false,
    });
  });

  it("resolves successfully on mutation success", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useUpdateGithubPrEnabled(), {
      wrapper: createWrapper(),
    });

    result.current.mutate({ projectId: "proj-1", enabled: true });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.isError).toBe(false);
  });
});
