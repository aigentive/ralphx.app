/**
 * useGitDiff hook tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useGitDiff } from "./useGitDiff";

describe("useGitDiff", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("initialization", () => {
    it("starts with loading states true when enabled", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: true })
      );

      expect(result.current.isLoadingChanges).toBe(true);
      expect(result.current.isLoadingHistory).toBe(true);

      // Wait for data to load
      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });
    });

    it("does not load when disabled", () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: false })
      );

      expect(result.current.changes).toEqual([]);
      expect(result.current.commits).toEqual([]);
      expect(result.current.isLoadingChanges).toBe(false);
      expect(result.current.isLoadingHistory).toBe(false);
    });

    it("does not load when taskId is empty", () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "", enabled: true })
      );

      expect(result.current.changes).toEqual([]);
      expect(result.current.commits).toEqual([]);
    });
  });

  describe("data loading", () => {
    it("loads mock changes data", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: true })
      );

      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });

      expect(result.current.changes.length).toBeGreaterThan(0);
      expect(result.current.changes[0]).toHaveProperty("path");
      expect(result.current.changes[0]).toHaveProperty("status");
      expect(result.current.changes[0]).toHaveProperty("additions");
      expect(result.current.changes[0]).toHaveProperty("deletions");
    });

    it("loads mock commits data", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: true })
      );

      await waitFor(() => {
        expect(result.current.isLoadingHistory).toBe(false);
      });

      expect(result.current.commits.length).toBeGreaterThan(0);
      expect(result.current.commits[0]).toHaveProperty("sha");
      expect(result.current.commits[0]).toHaveProperty("shortSha");
      expect(result.current.commits[0]).toHaveProperty("message");
      expect(result.current.commits[0]).toHaveProperty("author");
      expect(result.current.commits[0]).toHaveProperty("date");
    });

    it("sets error to null on successful load", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: true })
      );

      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });

      expect(result.current.error).toBeNull();
    });
  });

  describe("fetchDiff", () => {
    it("returns diff data for a file path", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: true })
      );

      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });

      let diffData;
      await act(async () => {
        diffData = await result.current.fetchDiff("src/components/LoginForm.tsx");
      });

      expect(diffData).not.toBeNull();
      expect(diffData).toHaveProperty("filePath", "src/components/LoginForm.tsx");
      expect(diffData).toHaveProperty("oldContent");
      expect(diffData).toHaveProperty("newContent");
      expect(diffData).toHaveProperty("hunks");
      expect(diffData).toHaveProperty("language", "typescript");
    });

    it("returns diff data with commit SHA", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: true })
      );

      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });

      let diffData;
      await act(async () => {
        diffData = await result.current.fetchDiff(
          "src/lib/auth.ts",
          "abc1234"
        );
      });

      expect(diffData).not.toBeNull();
      expect(diffData?.oldContent).toContain("abc1234");
    });

    it("returns null when disabled", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: false })
      );

      let diffData;
      await act(async () => {
        diffData = await result.current.fetchDiff("src/file.ts");
      });

      expect(diffData).toBeNull();
    });

    it("detects language from file extension", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: true })
      );

      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });

      const tsxDiff = await result.current.fetchDiff("Component.tsx");
      expect(tsxDiff?.language).toBe("typescript");

      const tsDiff = await result.current.fetchDiff("utils.ts");
      expect(tsDiff?.language).toBe("typescript");

      // Non-ts/tsx files get plaintext in the mock implementation
      const txtDiff = await result.current.fetchDiff("readme.txt");
      expect(txtDiff?.language).toBe("plaintext");
    });
  });

  describe("refresh", () => {
    it("sets loading states during refresh", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: true })
      );

      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });

      act(() => {
        result.current.refresh();
      });

      expect(result.current.isLoadingChanges).toBe(true);
      expect(result.current.isLoadingHistory).toBe(true);

      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });
    });

    it("does nothing when disabled", async () => {
      const { result } = renderHook(() =>
        useGitDiff({ taskId: "task-1", enabled: false })
      );

      await act(async () => {
        await result.current.refresh();
      });

      expect(result.current.isLoadingChanges).toBe(false);
    });
  });

  describe("re-fetching on taskId change", () => {
    it("refetches data when taskId changes", async () => {
      const { result, rerender } = renderHook(
        ({ taskId }) => useGitDiff({ taskId, enabled: true }),
        { initialProps: { taskId: "task-1" } }
      );

      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });

      // Change taskId
      rerender({ taskId: "task-2" });

      // Should start loading again
      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(true);
      });

      // Should eventually finish
      await waitFor(() => {
        expect(result.current.isLoadingChanges).toBe(false);
      });
    });
  });
});
