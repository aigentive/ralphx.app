/**
 * useArtifacts hooks tests
 *
 * Tests for useArtifacts, useArtifact, useBuckets, and artifact mutation hooks
 * using TanStack Query with mocked API.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import {
  useArtifacts,
  useArtifact,
  useArtifactsByBucket,
  useArtifactsByTask,
  useBuckets,
  useCreateArtifact,
  useUpdateArtifact,
  useDeleteArtifact,
  useCreateBucket,
  useAddArtifactRelation,
  useArtifactRelations,
  artifactKeys,
} from "./useArtifacts";
import * as artifactsApi from "@/lib/api/artifacts";
import type {
  ArtifactResponse,
  BucketResponse,
  ArtifactRelationResponse,
} from "@/lib/api/artifacts";

// Mock the artifacts API
vi.mock("@/lib/api/artifacts", () => ({
  getArtifacts: vi.fn(),
  getArtifact: vi.fn(),
  getArtifactsByBucket: vi.fn(),
  getArtifactsByTask: vi.fn(),
  getBuckets: vi.fn(),
  createArtifact: vi.fn(),
  updateArtifact: vi.fn(),
  deleteArtifact: vi.fn(),
  createBucket: vi.fn(),
  addArtifactRelation: vi.fn(),
  getArtifactRelations: vi.fn(),
}));

// Create mock data
const mockArtifact: ArtifactResponse = {
  id: "artifact-1",
  name: "Test Artifact",
  artifact_type: "prd",
  content_type: "inline",
  content: "Test content",
  created_at: "2026-01-24T10:00:00Z",
  created_by: "user",
  version: 1,
  bucket_id: "bucket-1",
  task_id: null,
  process_id: null,
  derived_from: [],
};

const mockArtifact2: ArtifactResponse = {
  id: "artifact-2",
  name: "Second Artifact",
  artifact_type: "research_document",
  content_type: "file",
  content: "/path/to/file.md",
  created_at: "2026-01-24T11:00:00Z",
  created_by: "orchestrator",
  version: 2,
  bucket_id: "bucket-2",
  task_id: "task-1",
  process_id: "process-1",
  derived_from: ["artifact-1"],
};

const mockBucket: BucketResponse = {
  id: "bucket-1",
  name: "Research Outputs",
  accepted_types: ["research_document", "findings", "recommendations"],
  writers: ["deep-researcher", "orchestrator"],
  readers: ["all"],
  is_system: true,
};

const mockBucket2: BucketResponse = {
  id: "bucket-2",
  name: "PRD Library",
  accepted_types: ["prd", "specification", "design_doc"],
  writers: ["orchestrator", "user"],
  readers: ["all"],
  is_system: true,
};

const mockRelation: ArtifactRelationResponse = {
  id: "relation-1",
  from_artifact_id: "artifact-1",
  to_artifact_id: "artifact-2",
  relation_type: "derived_from",
};

// Test wrapper with QueryClientProvider
function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });

  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(QueryClientProvider, { client: queryClient }, children);
  };
}

describe("artifactKeys", () => {
  it("should generate correct key for all", () => {
    expect(artifactKeys.all).toEqual(["artifacts"]);
  });

  it("should generate correct key for lists", () => {
    expect(artifactKeys.lists()).toEqual(["artifacts", "list"]);
  });

  it("should generate correct key for list with type filter", () => {
    expect(artifactKeys.list("prd")).toEqual(["artifacts", "list", "prd"]);
  });

  it("should generate correct key for list without type filter", () => {
    expect(artifactKeys.list()).toEqual(["artifacts", "list", undefined]);
  });

  it("should generate correct key for detail by id", () => {
    expect(artifactKeys.detail("artifact-1")).toEqual([
      "artifacts",
      "detail",
      "artifact-1",
    ]);
  });

  it("should generate correct key for byBucket", () => {
    expect(artifactKeys.byBucket("bucket-1")).toEqual([
      "artifacts",
      "byBucket",
      "bucket-1",
    ]);
  });

  it("should generate correct key for byTask", () => {
    expect(artifactKeys.byTask("task-1")).toEqual(["artifacts", "byTask", "task-1"]);
  });

  it("should generate correct key for buckets", () => {
    expect(artifactKeys.buckets()).toEqual(["artifacts", "buckets"]);
  });

  it("should generate correct key for relations", () => {
    expect(artifactKeys.relations("artifact-1")).toEqual([
      "artifacts",
      "relations",
      "artifact-1",
    ]);
  });
});

describe("useArtifacts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch all artifacts successfully", async () => {
    const mockArtifacts = [mockArtifact, mockArtifact2];
    vi.mocked(artifactsApi.getArtifacts).mockResolvedValueOnce(mockArtifacts);

    const { result } = renderHook(() => useArtifacts(), {
      wrapper: createWrapper(),
    });

    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockArtifacts);
    expect(artifactsApi.getArtifacts).toHaveBeenCalledWith(undefined);
  });

  it("should fetch artifacts filtered by type", async () => {
    vi.mocked(artifactsApi.getArtifacts).mockResolvedValueOnce([mockArtifact]);

    const { result } = renderHook(() => useArtifacts("prd"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([mockArtifact]);
    expect(artifactsApi.getArtifacts).toHaveBeenCalledWith("prd");
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch artifacts");
    vi.mocked(artifactsApi.getArtifacts).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useArtifacts(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useArtifact", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch a single artifact successfully", async () => {
    vi.mocked(artifactsApi.getArtifact).mockResolvedValueOnce(mockArtifact);

    const { result } = renderHook(() => useArtifact("artifact-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockArtifact);
    expect(artifactsApi.getArtifact).toHaveBeenCalledWith("artifact-1");
  });

  it("should return null for non-existent artifact", async () => {
    vi.mocked(artifactsApi.getArtifact).mockResolvedValueOnce(null);

    const { result } = renderHook(() => useArtifact("non-existent"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it("should not fetch when id is empty", async () => {
    const { result } = renderHook(() => useArtifact(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isFetching).toBe(false);
    expect(artifactsApi.getArtifact).not.toHaveBeenCalled();
  });
});

describe("useArtifactsByBucket", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch artifacts by bucket successfully", async () => {
    vi.mocked(artifactsApi.getArtifactsByBucket).mockResolvedValueOnce([mockArtifact]);

    const { result } = renderHook(() => useArtifactsByBucket("bucket-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([mockArtifact]);
    expect(artifactsApi.getArtifactsByBucket).toHaveBeenCalledWith("bucket-1");
  });

  it("should not fetch when bucketId is empty", async () => {
    const { result } = renderHook(() => useArtifactsByBucket(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isFetching).toBe(false);
    expect(artifactsApi.getArtifactsByBucket).not.toHaveBeenCalled();
  });
});

describe("useArtifactsByTask", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch artifacts by task successfully", async () => {
    vi.mocked(artifactsApi.getArtifactsByTask).mockResolvedValueOnce([mockArtifact2]);

    const { result } = renderHook(() => useArtifactsByTask("task-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([mockArtifact2]);
    expect(artifactsApi.getArtifactsByTask).toHaveBeenCalledWith("task-1");
  });

  it("should not fetch when taskId is empty", async () => {
    const { result } = renderHook(() => useArtifactsByTask(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isFetching).toBe(false);
    expect(artifactsApi.getArtifactsByTask).not.toHaveBeenCalled();
  });
});

describe("useBuckets", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch all buckets successfully", async () => {
    const mockBuckets = [mockBucket, mockBucket2];
    vi.mocked(artifactsApi.getBuckets).mockResolvedValueOnce(mockBuckets);

    const { result } = renderHook(() => useBuckets(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockBuckets);
    expect(artifactsApi.getBuckets).toHaveBeenCalledTimes(1);
  });

  it("should handle fetch error", async () => {
    const error = new Error("Failed to fetch buckets");
    vi.mocked(artifactsApi.getBuckets).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useBuckets(), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toEqual(error);
  });
});

describe("useArtifactRelations", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should fetch artifact relations successfully", async () => {
    vi.mocked(artifactsApi.getArtifactRelations).mockResolvedValueOnce([mockRelation]);

    const { result } = renderHook(() => useArtifactRelations("artifact-1"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([mockRelation]);
    expect(artifactsApi.getArtifactRelations).toHaveBeenCalledWith("artifact-1");
  });

  it("should not fetch when artifactId is empty", async () => {
    const { result } = renderHook(() => useArtifactRelations(""), {
      wrapper: createWrapper(),
    });

    expect(result.current.isFetching).toBe(false);
    expect(artifactsApi.getArtifactRelations).not.toHaveBeenCalled();
  });
});

describe("useCreateArtifact", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should create an artifact successfully", async () => {
    vi.mocked(artifactsApi.createArtifact).mockResolvedValueOnce(mockArtifact);

    const { result } = renderHook(() => useCreateArtifact(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({
        name: "Test Artifact",
        artifact_type: "prd",
        content_type: "inline",
        content: "Test content",
        created_by: "user",
      });
    });

    expect(artifactsApi.createArtifact).toHaveBeenCalled();
    expect(vi.mocked(artifactsApi.createArtifact).mock.calls[0][0]).toEqual({
      name: "Test Artifact",
      artifact_type: "prd",
      content_type: "inline",
      content: "Test content",
      created_by: "user",
    });
  });

  it("should handle creation error", async () => {
    const error = new Error("Failed to create artifact");
    vi.mocked(artifactsApi.createArtifact).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useCreateArtifact(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync({
          name: "Test Artifact",
          artifact_type: "prd",
          content_type: "inline",
          content: "Test content",
          created_by: "user",
        });
      })
    ).rejects.toThrow("Failed to create artifact");
  });
});

describe("useUpdateArtifact", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should update an artifact successfully", async () => {
    const updatedArtifact = { ...mockArtifact, name: "Updated Artifact" };
    vi.mocked(artifactsApi.updateArtifact).mockResolvedValueOnce(updatedArtifact);

    const { result } = renderHook(() => useUpdateArtifact(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({
        id: "artifact-1",
        input: { name: "Updated Artifact" },
      });
    });

    expect(artifactsApi.updateArtifact).toHaveBeenCalledWith("artifact-1", {
      name: "Updated Artifact",
    });
  });

  it("should handle update error", async () => {
    const error = new Error("Failed to update artifact");
    vi.mocked(artifactsApi.updateArtifact).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useUpdateArtifact(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync({
          id: "artifact-1",
          input: { name: "Updated Artifact" },
        });
      })
    ).rejects.toThrow("Failed to update artifact");
  });
});

describe("useDeleteArtifact", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should delete an artifact successfully", async () => {
    vi.mocked(artifactsApi.deleteArtifact).mockResolvedValueOnce(undefined);

    const { result } = renderHook(() => useDeleteArtifact(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync("artifact-1");
    });

    expect(artifactsApi.deleteArtifact).toHaveBeenCalled();
    expect(vi.mocked(artifactsApi.deleteArtifact).mock.calls[0][0]).toBe("artifact-1");
  });

  it("should handle delete error", async () => {
    const error = new Error("Failed to delete artifact");
    vi.mocked(artifactsApi.deleteArtifact).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useDeleteArtifact(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync("artifact-1");
      })
    ).rejects.toThrow("Failed to delete artifact");
  });
});

describe("useCreateBucket", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should create a bucket successfully", async () => {
    vi.mocked(artifactsApi.createBucket).mockResolvedValueOnce(mockBucket);

    const { result } = renderHook(() => useCreateBucket(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({
        name: "Research Outputs",
        accepted_types: ["research_document"],
      });
    });

    expect(artifactsApi.createBucket).toHaveBeenCalled();
    expect(vi.mocked(artifactsApi.createBucket).mock.calls[0][0]).toEqual({
      name: "Research Outputs",
      accepted_types: ["research_document"],
    });
  });

  it("should handle creation error", async () => {
    const error = new Error("Failed to create bucket");
    vi.mocked(artifactsApi.createBucket).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useCreateBucket(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync({ name: "Test Bucket" });
      })
    ).rejects.toThrow("Failed to create bucket");
  });
});

describe("useAddArtifactRelation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("should add a relation successfully", async () => {
    vi.mocked(artifactsApi.addArtifactRelation).mockResolvedValueOnce(mockRelation);

    const { result } = renderHook(() => useAddArtifactRelation(), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.mutateAsync({
        from_artifact_id: "artifact-1",
        to_artifact_id: "artifact-2",
        relation_type: "derived_from",
      });
    });

    expect(artifactsApi.addArtifactRelation).toHaveBeenCalled();
    expect(vi.mocked(artifactsApi.addArtifactRelation).mock.calls[0][0]).toEqual({
      from_artifact_id: "artifact-1",
      to_artifact_id: "artifact-2",
      relation_type: "derived_from",
    });
  });

  it("should handle relation creation error", async () => {
    const error = new Error("Failed to add relation");
    vi.mocked(artifactsApi.addArtifactRelation).mockRejectedValueOnce(error);

    const { result } = renderHook(() => useAddArtifactRelation(), {
      wrapper: createWrapper(),
    });

    await expect(
      act(async () => {
        await result.current.mutateAsync({
          from_artifact_id: "artifact-1",
          to_artifact_id: "artifact-2",
          relation_type: "derived_from",
        });
      })
    ).rejects.toThrow("Failed to add relation");
  });
});
