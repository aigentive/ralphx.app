import { describe, it, expect, beforeEach } from "vitest";
import {
  useArtifactStore,
  selectSelectedBucket,
  selectSelectedArtifact,
  selectArtifactsByBucket,
  selectArtifactsByType,
  selectArtifactById,
} from "./artifactStore";
import type { Artifact, ArtifactBucket } from "@/types/artifact";

// Helper to create test artifacts
const createTestArtifact = (overrides: Partial<Artifact> = {}): Artifact => ({
  id: `artifact-${Math.random().toString(36).slice(2)}`,
  type: "prd",
  name: "Test Artifact",
  content: { type: "inline", text: "Test content" },
  metadata: {
    createdAt: new Date().toISOString(),
    createdBy: "user",
    version: 1,
  },
  derivedFrom: [],
  ...overrides,
});

// Helper to create test buckets
const createTestBucket = (overrides: Partial<ArtifactBucket> = {}): ArtifactBucket => ({
  id: `bucket-${Math.random().toString(36).slice(2)}`,
  name: "Test Bucket",
  acceptedTypes: ["prd", "specification"],
  writers: ["user"],
  readers: ["all"],
  isSystem: false,
  ...overrides,
});

describe("artifactStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useArtifactStore.setState({
      artifacts: {},
      buckets: {},
      selectedBucketId: null,
      selectedArtifactId: null,
      isLoading: false,
      error: null,
    });
  });

  describe("setArtifacts", () => {
    it("converts array to Record keyed by id", () => {
      const artifacts = [
        createTestArtifact({ id: "artifact-1", name: "Artifact 1" }),
        createTestArtifact({ id: "artifact-2", name: "Artifact 2" }),
      ];

      useArtifactStore.getState().setArtifacts(artifacts);

      const state = useArtifactStore.getState();
      expect(Object.keys(state.artifacts)).toHaveLength(2);
      expect(state.artifacts["artifact-1"]?.name).toBe("Artifact 1");
      expect(state.artifacts["artifact-2"]?.name).toBe("Artifact 2");
    });

    it("replaces existing artifacts", () => {
      useArtifactStore.setState({
        artifacts: {
          "old-artifact": createTestArtifact({ id: "old-artifact", name: "Old" }),
        },
      });

      const newArtifacts = [createTestArtifact({ id: "new-artifact", name: "New" })];
      useArtifactStore.getState().setArtifacts(newArtifacts);

      const state = useArtifactStore.getState();
      expect(state.artifacts["old-artifact"]).toBeUndefined();
      expect(state.artifacts["new-artifact"]?.name).toBe("New");
    });

    it("handles empty array", () => {
      useArtifactStore.getState().setArtifacts([]);

      const state = useArtifactStore.getState();
      expect(Object.keys(state.artifacts)).toHaveLength(0);
    });
  });

  describe("setBuckets", () => {
    it("converts array to Record keyed by id", () => {
      const buckets = [
        createTestBucket({ id: "bucket-1", name: "Bucket 1" }),
        createTestBucket({ id: "bucket-2", name: "Bucket 2" }),
      ];

      useArtifactStore.getState().setBuckets(buckets);

      const state = useArtifactStore.getState();
      expect(Object.keys(state.buckets)).toHaveLength(2);
      expect(state.buckets["bucket-1"]?.name).toBe("Bucket 1");
      expect(state.buckets["bucket-2"]?.name).toBe("Bucket 2");
    });

    it("replaces existing buckets", () => {
      useArtifactStore.setState({
        buckets: {
          "old-bucket": createTestBucket({ id: "old-bucket", name: "Old" }),
        },
      });

      const newBuckets = [createTestBucket({ id: "new-bucket", name: "New" })];
      useArtifactStore.getState().setBuckets(newBuckets);

      const state = useArtifactStore.getState();
      expect(state.buckets["old-bucket"]).toBeUndefined();
      expect(state.buckets["new-bucket"]?.name).toBe("New");
    });

    it("handles empty array", () => {
      useArtifactStore.getState().setBuckets([]);

      const state = useArtifactStore.getState();
      expect(Object.keys(state.buckets)).toHaveLength(0);
    });
  });

  describe("setSelectedBucket", () => {
    it("updates selectedBucketId", () => {
      const bucket = createTestBucket({ id: "bucket-1" });
      useArtifactStore.setState({ buckets: { "bucket-1": bucket } });

      useArtifactStore.getState().setSelectedBucket("bucket-1");

      const state = useArtifactStore.getState();
      expect(state.selectedBucketId).toBe("bucket-1");
    });

    it("sets selectedBucketId to null", () => {
      useArtifactStore.setState({ selectedBucketId: "bucket-1" });

      useArtifactStore.getState().setSelectedBucket(null);

      const state = useArtifactStore.getState();
      expect(state.selectedBucketId).toBeNull();
    });

    it("clears selectedArtifactId when bucket changes", () => {
      useArtifactStore.setState({
        selectedBucketId: "bucket-1",
        selectedArtifactId: "artifact-1",
      });

      useArtifactStore.getState().setSelectedBucket("bucket-2");

      const state = useArtifactStore.getState();
      expect(state.selectedBucketId).toBe("bucket-2");
      expect(state.selectedArtifactId).toBeNull();
    });

    it("does not clear selectedArtifactId when bucket is same", () => {
      useArtifactStore.setState({
        selectedBucketId: "bucket-1",
        selectedArtifactId: "artifact-1",
      });

      useArtifactStore.getState().setSelectedBucket("bucket-1");

      const state = useArtifactStore.getState();
      expect(state.selectedBucketId).toBe("bucket-1");
      expect(state.selectedArtifactId).toBe("artifact-1");
    });
  });

  describe("setSelectedArtifact", () => {
    it("updates selectedArtifactId", () => {
      const artifact = createTestArtifact({ id: "artifact-1" });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact } });

      useArtifactStore.getState().setSelectedArtifact("artifact-1");

      const state = useArtifactStore.getState();
      expect(state.selectedArtifactId).toBe("artifact-1");
    });

    it("sets selectedArtifactId to null", () => {
      useArtifactStore.setState({ selectedArtifactId: "artifact-1" });

      useArtifactStore.getState().setSelectedArtifact(null);

      const state = useArtifactStore.getState();
      expect(state.selectedArtifactId).toBeNull();
    });

    it("replaces previous selectedArtifactId", () => {
      useArtifactStore.setState({ selectedArtifactId: "artifact-1" });

      useArtifactStore.getState().setSelectedArtifact("artifact-2");

      const state = useArtifactStore.getState();
      expect(state.selectedArtifactId).toBe("artifact-2");
    });
  });

  describe("addArtifact", () => {
    it("adds a new artifact to the store", () => {
      const artifact = createTestArtifact({ id: "artifact-1" });

      useArtifactStore.getState().addArtifact(artifact);

      const state = useArtifactStore.getState();
      expect(state.artifacts["artifact-1"]).toBeDefined();
      expect(state.artifacts["artifact-1"]?.name).toBe("Test Artifact");
    });

    it("overwrites artifact with same id", () => {
      const artifact1 = createTestArtifact({ id: "artifact-1", name: "First" });
      const artifact2 = createTestArtifact({ id: "artifact-1", name: "Second" });

      useArtifactStore.getState().addArtifact(artifact1);
      useArtifactStore.getState().addArtifact(artifact2);

      const state = useArtifactStore.getState();
      expect(state.artifacts["artifact-1"]?.name).toBe("Second");
    });

    it("preserves other artifacts when adding", () => {
      const artifact1 = createTestArtifact({ id: "artifact-1", name: "First" });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact1 } });

      const artifact2 = createTestArtifact({ id: "artifact-2", name: "Second" });
      useArtifactStore.getState().addArtifact(artifact2);

      const state = useArtifactStore.getState();
      expect(Object.keys(state.artifacts)).toHaveLength(2);
      expect(state.artifacts["artifact-1"]?.name).toBe("First");
      expect(state.artifacts["artifact-2"]?.name).toBe("Second");
    });
  });

  describe("updateArtifact", () => {
    it("modifies existing artifact", () => {
      const artifact = createTestArtifact({ id: "artifact-1", name: "Original" });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact } });

      useArtifactStore.getState().updateArtifact("artifact-1", { name: "Updated" });

      const state = useArtifactStore.getState();
      expect(state.artifacts["artifact-1"]?.name).toBe("Updated");
    });

    it("updates multiple fields", () => {
      const artifact = createTestArtifact({
        id: "artifact-1",
        name: "Original",
        type: "prd",
      });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact } });

      useArtifactStore
        .getState()
        .updateArtifact("artifact-1", { name: "Updated", type: "specification" });

      const state = useArtifactStore.getState();
      expect(state.artifacts["artifact-1"]?.name).toBe("Updated");
      expect(state.artifacts["artifact-1"]?.type).toBe("specification");
    });

    it("does nothing if artifact not found", () => {
      const artifact = createTestArtifact({ id: "artifact-1" });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact } });

      useArtifactStore.getState().updateArtifact("nonexistent", { name: "Updated" });

      const state = useArtifactStore.getState();
      expect(Object.keys(state.artifacts)).toHaveLength(1);
      expect(state.artifacts["artifact-1"]?.name).toBe("Test Artifact");
    });

    it("preserves other artifact fields", () => {
      const artifact = createTestArtifact({
        id: "artifact-1",
        name: "Original",
        type: "prd",
        content: { type: "inline", text: "Content" },
      });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact } });

      useArtifactStore.getState().updateArtifact("artifact-1", { name: "Updated" });

      const state = useArtifactStore.getState();
      expect(state.artifacts["artifact-1"]?.name).toBe("Updated");
      expect(state.artifacts["artifact-1"]?.type).toBe("prd");
      expect(state.artifacts["artifact-1"]?.content.type).toBe("inline");
    });
  });

  describe("deleteArtifact", () => {
    it("removes an artifact from the store", () => {
      const artifact = createTestArtifact({ id: "artifact-1" });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact } });

      useArtifactStore.getState().deleteArtifact("artifact-1");

      const state = useArtifactStore.getState();
      expect(state.artifacts["artifact-1"]).toBeUndefined();
    });

    it("clears selectedArtifactId if selected artifact is deleted", () => {
      const artifact = createTestArtifact({ id: "artifact-1" });
      useArtifactStore.setState({
        artifacts: { "artifact-1": artifact },
        selectedArtifactId: "artifact-1",
      });

      useArtifactStore.getState().deleteArtifact("artifact-1");

      const state = useArtifactStore.getState();
      expect(state.selectedArtifactId).toBeNull();
    });

    it("does not affect selectedArtifactId if different artifact is deleted", () => {
      const artifact1 = createTestArtifact({ id: "artifact-1" });
      const artifact2 = createTestArtifact({ id: "artifact-2" });
      useArtifactStore.setState({
        artifacts: { "artifact-1": artifact1, "artifact-2": artifact2 },
        selectedArtifactId: "artifact-1",
      });

      useArtifactStore.getState().deleteArtifact("artifact-2");

      const state = useArtifactStore.getState();
      expect(state.selectedArtifactId).toBe("artifact-1");
    });

    it("does nothing if artifact not found", () => {
      const artifact = createTestArtifact({ id: "artifact-1" });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact } });

      useArtifactStore.getState().deleteArtifact("nonexistent");

      const state = useArtifactStore.getState();
      expect(Object.keys(state.artifacts)).toHaveLength(1);
    });
  });

  describe("addBucket", () => {
    it("adds a new bucket to the store", () => {
      const bucket = createTestBucket({ id: "bucket-1" });

      useArtifactStore.getState().addBucket(bucket);

      const state = useArtifactStore.getState();
      expect(state.buckets["bucket-1"]).toBeDefined();
      expect(state.buckets["bucket-1"]?.name).toBe("Test Bucket");
    });

    it("overwrites bucket with same id", () => {
      const bucket1 = createTestBucket({ id: "bucket-1", name: "First" });
      const bucket2 = createTestBucket({ id: "bucket-1", name: "Second" });

      useArtifactStore.getState().addBucket(bucket1);
      useArtifactStore.getState().addBucket(bucket2);

      const state = useArtifactStore.getState();
      expect(state.buckets["bucket-1"]?.name).toBe("Second");
    });
  });

  describe("setLoading", () => {
    it("sets loading state to true", () => {
      useArtifactStore.getState().setLoading(true);

      const state = useArtifactStore.getState();
      expect(state.isLoading).toBe(true);
    });

    it("sets loading state to false", () => {
      useArtifactStore.setState({ isLoading: true });

      useArtifactStore.getState().setLoading(false);

      const state = useArtifactStore.getState();
      expect(state.isLoading).toBe(false);
    });
  });

  describe("setError", () => {
    it("sets error message", () => {
      useArtifactStore.getState().setError("Something went wrong");

      const state = useArtifactStore.getState();
      expect(state.error).toBe("Something went wrong");
    });

    it("clears error with null", () => {
      useArtifactStore.setState({ error: "Previous error" });

      useArtifactStore.getState().setError(null);

      const state = useArtifactStore.getState();
      expect(state.error).toBeNull();
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useArtifactStore.setState({
      artifacts: {},
      buckets: {},
      selectedBucketId: null,
      selectedArtifactId: null,
      isLoading: false,
      error: null,
    });
  });

  describe("selectSelectedBucket", () => {
    it("returns selected bucket when it exists", () => {
      const bucket = createTestBucket({ id: "bucket-1", name: "Selected Bucket" });
      useArtifactStore.setState({
        buckets: { "bucket-1": bucket },
        selectedBucketId: "bucket-1",
      });

      const result = selectSelectedBucket(useArtifactStore.getState());

      expect(result).not.toBeNull();
      expect(result?.name).toBe("Selected Bucket");
    });

    it("returns null when no bucket is selected", () => {
      const bucket = createTestBucket({ id: "bucket-1" });
      useArtifactStore.setState({
        buckets: { "bucket-1": bucket },
        selectedBucketId: null,
      });

      const result = selectSelectedBucket(useArtifactStore.getState());

      expect(result).toBeNull();
    });

    it("returns null when selected bucket does not exist", () => {
      useArtifactStore.setState({
        buckets: {},
        selectedBucketId: "nonexistent",
      });

      const result = selectSelectedBucket(useArtifactStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectSelectedArtifact", () => {
    it("returns selected artifact when it exists", () => {
      const artifact = createTestArtifact({ id: "artifact-1", name: "Selected Artifact" });
      useArtifactStore.setState({
        artifacts: { "artifact-1": artifact },
        selectedArtifactId: "artifact-1",
      });

      const result = selectSelectedArtifact(useArtifactStore.getState());

      expect(result).not.toBeNull();
      expect(result?.name).toBe("Selected Artifact");
    });

    it("returns null when no artifact is selected", () => {
      const artifact = createTestArtifact({ id: "artifact-1" });
      useArtifactStore.setState({
        artifacts: { "artifact-1": artifact },
        selectedArtifactId: null,
      });

      const result = selectSelectedArtifact(useArtifactStore.getState());

      expect(result).toBeNull();
    });

    it("returns null when selected artifact does not exist", () => {
      useArtifactStore.setState({
        artifacts: {},
        selectedArtifactId: "nonexistent",
      });

      const result = selectSelectedArtifact(useArtifactStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectArtifactsByBucket", () => {
    it("returns artifacts in specified bucket", () => {
      const artifact1 = createTestArtifact({
        id: "artifact-1",
        name: "In Bucket",
        bucketId: "bucket-1",
      });
      const artifact2 = createTestArtifact({
        id: "artifact-2",
        name: "Other Bucket",
        bucketId: "bucket-2",
      });
      const artifact3 = createTestArtifact({
        id: "artifact-3",
        name: "Also In Bucket",
        bucketId: "bucket-1",
      });
      useArtifactStore.setState({
        artifacts: {
          "artifact-1": artifact1,
          "artifact-2": artifact2,
          "artifact-3": artifact3,
        },
      });

      const selector = selectArtifactsByBucket("bucket-1");
      const result = selector(useArtifactStore.getState());

      expect(result).toHaveLength(2);
      expect(result.map((a) => a.name)).toContain("In Bucket");
      expect(result.map((a) => a.name)).toContain("Also In Bucket");
    });

    it("returns empty array when no artifacts in bucket", () => {
      const artifact = createTestArtifact({
        id: "artifact-1",
        bucketId: "bucket-1",
      });
      useArtifactStore.setState({
        artifacts: { "artifact-1": artifact },
      });

      const selector = selectArtifactsByBucket("bucket-2");
      const result = selector(useArtifactStore.getState());

      expect(result).toEqual([]);
    });

    it("returns empty array when no artifacts exist", () => {
      useArtifactStore.setState({ artifacts: {} });

      const selector = selectArtifactsByBucket("bucket-1");
      const result = selector(useArtifactStore.getState());

      expect(result).toEqual([]);
    });
  });

  describe("selectArtifactsByType", () => {
    it("returns artifacts of specified type", () => {
      const artifact1 = createTestArtifact({
        id: "artifact-1",
        name: "PRD 1",
        type: "prd",
      });
      const artifact2 = createTestArtifact({
        id: "artifact-2",
        name: "Spec",
        type: "specification",
      });
      const artifact3 = createTestArtifact({
        id: "artifact-3",
        name: "PRD 2",
        type: "prd",
      });
      useArtifactStore.setState({
        artifacts: {
          "artifact-1": artifact1,
          "artifact-2": artifact2,
          "artifact-3": artifact3,
        },
      });

      const selector = selectArtifactsByType("prd");
      const result = selector(useArtifactStore.getState());

      expect(result).toHaveLength(2);
      expect(result.map((a) => a.name)).toContain("PRD 1");
      expect(result.map((a) => a.name)).toContain("PRD 2");
    });

    it("returns empty array when no artifacts of type", () => {
      const artifact = createTestArtifact({
        id: "artifact-1",
        type: "prd",
      });
      useArtifactStore.setState({
        artifacts: { "artifact-1": artifact },
      });

      const selector = selectArtifactsByType("specification");
      const result = selector(useArtifactStore.getState());

      expect(result).toEqual([]);
    });
  });

  describe("selectArtifactById", () => {
    it("returns artifact when it exists", () => {
      const artifact = createTestArtifact({ id: "artifact-1", name: "Found Artifact" });
      useArtifactStore.setState({ artifacts: { "artifact-1": artifact } });

      const selector = selectArtifactById("artifact-1");
      const result = selector(useArtifactStore.getState());

      expect(result).not.toBeUndefined();
      expect(result?.name).toBe("Found Artifact");
    });

    it("returns undefined when artifact does not exist", () => {
      useArtifactStore.setState({ artifacts: {} });

      const selector = selectArtifactById("nonexistent");
      const result = selector(useArtifactStore.getState());

      expect(result).toBeUndefined();
    });
  });
});
