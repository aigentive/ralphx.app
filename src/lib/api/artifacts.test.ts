import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getArtifacts,
  getArtifact,
  createArtifact,
  updateArtifact,
  deleteArtifact,
  getArtifactsByBucket,
  getArtifactsByTask,
  getBuckets,
  createBucket,
  getSystemBuckets,
  addArtifactRelation,
  getArtifactRelations,
  ArtifactResponseSchema,
  BucketResponseSchema,
  ArtifactRelationResponseSchema,
  CreateArtifactInputSchema,
  UpdateArtifactInputSchema,
  CreateBucketInputSchema,
  AddRelationInputSchema,
} from "./artifacts";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

// Test data helpers
const createMockArtifact = (overrides = {}) => ({
  id: "artifact-1",
  name: "Test Artifact",
  artifact_type: "prd",
  content_type: "inline",
  content: "Test content",
  created_at: "2026-01-24T12:00:00Z",
  created_by: "user",
  version: 1,
  bucket_id: null,
  task_id: null,
  process_id: null,
  derived_from: [],
  ...overrides,
});

const createMockBucket = (overrides = {}) => ({
  id: "bucket-1",
  name: "Test Bucket",
  accepted_types: ["prd", "design_doc"],
  writers: ["user", "orchestrator"],
  readers: ["all"],
  is_system: false,
  ...overrides,
});

const createMockRelation = (overrides = {}) => ({
  id: "relation-1",
  from_artifact_id: "artifact-1",
  to_artifact_id: "artifact-2",
  relation_type: "derived_from",
  ...overrides,
});

describe("ArtifactResponseSchema", () => {
  it("should parse valid artifact response", () => {
    const artifact = createMockArtifact();
    expect(() => ArtifactResponseSchema.parse(artifact)).not.toThrow();
  });

  it("should parse artifact with all optional fields", () => {
    const artifact = createMockArtifact({
      bucket_id: "bucket-1",
      task_id: "task-1",
      process_id: "process-1",
      derived_from: ["parent-1", "parent-2"],
    });
    const result = ArtifactResponseSchema.parse(artifact);
    expect(result.bucket_id).toBe("bucket-1");
    expect(result.derived_from).toHaveLength(2);
  });

  it("should parse file content artifact", () => {
    const artifact = createMockArtifact({
      content_type: "file",
      content: "/path/to/file.md",
    });
    const result = ArtifactResponseSchema.parse(artifact);
    expect(result.content_type).toBe("file");
    expect(result.content).toBe("/path/to/file.md");
  });

  it("should reject artifact without required fields", () => {
    expect(() => ArtifactResponseSchema.parse({})).toThrow();
    expect(() => ArtifactResponseSchema.parse({ id: "a1" })).toThrow();
  });

  it("should reject artifact with invalid artifact_type", () => {
    const artifact = createMockArtifact({ artifact_type: "invalid_type" });
    expect(() => ArtifactResponseSchema.parse(artifact)).toThrow();
  });

  it("should reject artifact with invalid content_type", () => {
    const artifact = createMockArtifact({ content_type: "binary" });
    expect(() => ArtifactResponseSchema.parse(artifact)).toThrow();
  });
});

describe("BucketResponseSchema", () => {
  it("should parse valid bucket response", () => {
    const bucket = createMockBucket();
    expect(() => BucketResponseSchema.parse(bucket)).not.toThrow();
  });

  it("should parse system bucket", () => {
    const bucket = createMockBucket({ is_system: true });
    const result = BucketResponseSchema.parse(bucket);
    expect(result.is_system).toBe(true);
  });

  it("should validate accepted_types", () => {
    const bucket = createMockBucket({ accepted_types: ["invalid_type"] });
    expect(() => BucketResponseSchema.parse(bucket)).toThrow();
  });

  it("should reject bucket without required fields", () => {
    expect(() => BucketResponseSchema.parse({})).toThrow();
  });
});

describe("ArtifactRelationResponseSchema", () => {
  it("should parse valid relation response", () => {
    const relation = createMockRelation();
    expect(() => ArtifactRelationResponseSchema.parse(relation)).not.toThrow();
  });

  it("should parse related_to relation", () => {
    const relation = createMockRelation({ relation_type: "related_to" });
    const result = ArtifactRelationResponseSchema.parse(relation);
    expect(result.relation_type).toBe("related_to");
  });

  it("should reject invalid relation_type", () => {
    const relation = createMockRelation({ relation_type: "invalid" });
    expect(() => ArtifactRelationResponseSchema.parse(relation)).toThrow();
  });
});

describe("CreateArtifactInputSchema", () => {
  it("should parse valid create input", () => {
    const input = {
      name: "New Artifact",
      artifact_type: "prd",
      content_type: "inline",
      content: "Content here",
      created_by: "user",
    };
    expect(() => CreateArtifactInputSchema.parse(input)).not.toThrow();
  });

  it("should parse input with all optional fields", () => {
    const input = {
      name: "Full Artifact",
      artifact_type: "code_change",
      content_type: "file",
      content: "/path/to/file.diff",
      created_by: "worker",
      bucket_id: "bucket-1",
      task_id: "task-1",
      process_id: "process-1",
      derived_from: ["parent-1"],
    };
    const result = CreateArtifactInputSchema.parse(input);
    expect(result.bucket_id).toBe("bucket-1");
    expect(result.derived_from).toHaveLength(1);
  });

  it("should reject input without required fields", () => {
    expect(() => CreateArtifactInputSchema.parse({})).toThrow();
    expect(() => CreateArtifactInputSchema.parse({ name: "Test" })).toThrow();
  });

  it("should reject invalid artifact_type", () => {
    const input = {
      name: "Test",
      artifact_type: "invalid",
      content_type: "inline",
      content: "content",
      created_by: "user",
    };
    expect(() => CreateArtifactInputSchema.parse(input)).toThrow();
  });

  it("should reject invalid content_type", () => {
    const input = {
      name: "Test",
      artifact_type: "prd",
      content_type: "binary",
      content: "content",
      created_by: "user",
    };
    expect(() => CreateArtifactInputSchema.parse(input)).toThrow();
  });
});

describe("UpdateArtifactInputSchema", () => {
  it("should parse partial update", () => {
    const input = { name: "Updated Name" };
    expect(() => UpdateArtifactInputSchema.parse(input)).not.toThrow();
  });

  it("should parse full update", () => {
    const input = {
      name: "Updated",
      content_type: "file",
      content: "/new/path.md",
      bucket_id: "new-bucket",
    };
    const result = UpdateArtifactInputSchema.parse(input);
    expect(result.name).toBe("Updated");
    expect(result.content_type).toBe("file");
  });

  it("should allow empty object", () => {
    const input = {};
    expect(() => UpdateArtifactInputSchema.parse(input)).not.toThrow();
  });

  it("should reject invalid content_type", () => {
    const input = { content_type: "binary" };
    expect(() => UpdateArtifactInputSchema.parse(input)).toThrow();
  });
});

describe("CreateBucketInputSchema", () => {
  it("should parse valid bucket input", () => {
    const input = { name: "New Bucket" };
    expect(() => CreateBucketInputSchema.parse(input)).not.toThrow();
  });

  it("should parse input with all optional fields", () => {
    const input = {
      name: "Full Bucket",
      accepted_types: ["prd", "design_doc"],
      writers: ["user", "orchestrator"],
      readers: ["all"],
    };
    const result = CreateBucketInputSchema.parse(input);
    expect(result.accepted_types).toHaveLength(2);
    expect(result.writers).toHaveLength(2);
  });

  it("should reject input without name", () => {
    expect(() => CreateBucketInputSchema.parse({})).toThrow();
  });

  it("should reject invalid accepted_types", () => {
    const input = { name: "Bucket", accepted_types: ["invalid"] };
    expect(() => CreateBucketInputSchema.parse(input)).toThrow();
  });
});

describe("AddRelationInputSchema", () => {
  it("should parse valid relation input", () => {
    const input = {
      from_artifact_id: "a1",
      to_artifact_id: "a2",
      relation_type: "derived_from",
    };
    expect(() => AddRelationInputSchema.parse(input)).not.toThrow();
  });

  it("should parse related_to input", () => {
    const input = {
      from_artifact_id: "a1",
      to_artifact_id: "a2",
      relation_type: "related_to",
    };
    const result = AddRelationInputSchema.parse(input);
    expect(result.relation_type).toBe("related_to");
  });

  it("should reject invalid relation_type", () => {
    const input = {
      from_artifact_id: "a1",
      to_artifact_id: "a2",
      relation_type: "invalid",
    };
    expect(() => AddRelationInputSchema.parse(input)).toThrow();
  });
});

describe("getArtifacts", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_artifacts command without filter", async () => {
    mockInvoke.mockResolvedValue([createMockArtifact()]);

    await getArtifacts();

    expect(mockInvoke).toHaveBeenCalledWith("get_artifacts", { artifact_type: null });
  });

  it("should call get_artifacts with type filter", async () => {
    mockInvoke.mockResolvedValue([createMockArtifact()]);

    await getArtifacts("prd");

    expect(mockInvoke).toHaveBeenCalledWith("get_artifacts", { artifact_type: "prd" });
  });

  it("should return validated array of artifacts", async () => {
    const artifacts = [
      createMockArtifact({ id: "a1", name: "Artifact 1" }),
      createMockArtifact({ id: "a2", name: "Artifact 2" }),
    ];
    mockInvoke.mockResolvedValue(artifacts);

    const result = await getArtifacts();

    expect(result).toHaveLength(2);
    expect(result[0]?.name).toBe("Artifact 1");
  });

  it("should throw on invalid response", async () => {
    mockInvoke.mockResolvedValue([{ invalid: "artifact" }]);

    await expect(getArtifacts()).rejects.toThrow();
  });
});

describe("getArtifact", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_artifact command with id", async () => {
    mockInvoke.mockResolvedValue(createMockArtifact());

    await getArtifact("a-123");

    expect(mockInvoke).toHaveBeenCalledWith("get_artifact", { id: "a-123" });
  });

  it("should return null when artifact not found", async () => {
    mockInvoke.mockResolvedValue(null);

    const result = await getArtifact("nonexistent");

    expect(result).toBeNull();
  });

  it("should return validated artifact", async () => {
    const artifact = createMockArtifact({ name: "Found Artifact" });
    mockInvoke.mockResolvedValue(artifact);

    const result = await getArtifact("a-123");

    expect(result?.name).toBe("Found Artifact");
  });
});

describe("createArtifact", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call create_artifact command with input", async () => {
    mockInvoke.mockResolvedValue(createMockArtifact());
    const input = {
      name: "New Artifact",
      artifact_type: "prd" as const,
      content_type: "inline" as const,
      content: "Content",
      created_by: "user",
    };

    await createArtifact(input);

    expect(mockInvoke).toHaveBeenCalledWith("create_artifact", { input });
  });

  it("should return created artifact", async () => {
    const created = createMockArtifact({ name: "Created" });
    mockInvoke.mockResolvedValue(created);

    const result = await createArtifact({
      name: "Created",
      artifact_type: "prd",
      content_type: "inline",
      content: "Content",
      created_by: "user",
    });

    expect(result.name).toBe("Created");
  });

  it("should validate input before sending", async () => {
    const invalidInput = { name: "No Type" } as never;

    await expect(createArtifact(invalidInput)).rejects.toThrow();
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

describe("updateArtifact", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call update_artifact command with id and input", async () => {
    mockInvoke.mockResolvedValue(createMockArtifact());
    const input = { name: "Updated Name" };

    await updateArtifact("a-123", input);

    expect(mockInvoke).toHaveBeenCalledWith("update_artifact", {
      id: "a-123",
      input,
    });
  });

  it("should return updated artifact", async () => {
    const updated = createMockArtifact({ name: "Updated" });
    mockInvoke.mockResolvedValue(updated);

    const result = await updateArtifact("a-123", { name: "Updated" });

    expect(result.name).toBe("Updated");
  });
});

describe("deleteArtifact", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call delete_artifact command with id", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await deleteArtifact("a-123");

    expect(mockInvoke).toHaveBeenCalledWith("delete_artifact", { id: "a-123" });
  });

  it("should complete without throwing on success", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await expect(deleteArtifact("a-123")).resolves.toBeUndefined();
  });

  it("should propagate backend errors", async () => {
    mockInvoke.mockRejectedValue(new Error("Artifact not found"));

    await expect(deleteArtifact("nonexistent")).rejects.toThrow("Artifact not found");
  });
});

describe("getArtifactsByBucket", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_artifacts_by_bucket command with bucket_id", async () => {
    mockInvoke.mockResolvedValue([createMockArtifact()]);

    await getArtifactsByBucket("bucket-1");

    expect(mockInvoke).toHaveBeenCalledWith("get_artifacts_by_bucket", {
      bucket_id: "bucket-1",
    });
  });

  it("should return validated array of artifacts", async () => {
    const artifacts = [createMockArtifact({ bucket_id: "bucket-1" })];
    mockInvoke.mockResolvedValue(artifacts);

    const result = await getArtifactsByBucket("bucket-1");

    expect(result).toHaveLength(1);
    expect(result[0]?.bucket_id).toBe("bucket-1");
  });
});

describe("getArtifactsByTask", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_artifacts_by_task command with task_id", async () => {
    mockInvoke.mockResolvedValue([createMockArtifact()]);

    await getArtifactsByTask("task-1");

    expect(mockInvoke).toHaveBeenCalledWith("get_artifacts_by_task", {
      task_id: "task-1",
    });
  });

  it("should return validated array of artifacts", async () => {
    const artifacts = [createMockArtifact({ task_id: "task-1" })];
    mockInvoke.mockResolvedValue(artifacts);

    const result = await getArtifactsByTask("task-1");

    expect(result).toHaveLength(1);
    expect(result[0]?.task_id).toBe("task-1");
  });
});

describe("getBuckets", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_buckets command", async () => {
    mockInvoke.mockResolvedValue([createMockBucket()]);

    await getBuckets();

    expect(mockInvoke).toHaveBeenCalledWith("get_buckets", {});
  });

  it("should return validated array of buckets", async () => {
    const buckets = [
      createMockBucket({ id: "b1", name: "Bucket 1" }),
      createMockBucket({ id: "b2", name: "Bucket 2" }),
    ];
    mockInvoke.mockResolvedValue(buckets);

    const result = await getBuckets();

    expect(result).toHaveLength(2);
  });

  it("should throw on invalid response", async () => {
    mockInvoke.mockResolvedValue([{ invalid: "bucket" }]);

    await expect(getBuckets()).rejects.toThrow();
  });
});

describe("createBucket", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call create_bucket command with input", async () => {
    mockInvoke.mockResolvedValue(createMockBucket());
    const input = { name: "New Bucket" };

    await createBucket(input);

    expect(mockInvoke).toHaveBeenCalledWith("create_bucket", { input });
  });

  it("should return created bucket", async () => {
    const created = createMockBucket({ name: "Created Bucket" });
    mockInvoke.mockResolvedValue(created);

    const result = await createBucket({ name: "Created Bucket" });

    expect(result.name).toBe("Created Bucket");
  });

  it("should validate input before sending", async () => {
    const invalidInput = {} as never;

    await expect(createBucket(invalidInput)).rejects.toThrow();
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

describe("getSystemBuckets", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_system_buckets command", async () => {
    mockInvoke.mockResolvedValue([createMockBucket({ is_system: true })]);

    await getSystemBuckets();

    expect(mockInvoke).toHaveBeenCalledWith("get_system_buckets", {});
  });

  it("should return validated system buckets", async () => {
    const buckets = [
      createMockBucket({ id: "research-outputs", name: "Research Outputs", is_system: true }),
      createMockBucket({ id: "work-context", name: "Work Context", is_system: true }),
    ];
    mockInvoke.mockResolvedValue(buckets);

    const result = await getSystemBuckets();

    expect(result).toHaveLength(2);
    expect(result.every((b) => b.is_system)).toBe(true);
  });
});

describe("addArtifactRelation", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call add_artifact_relation command with input", async () => {
    mockInvoke.mockResolvedValue(createMockRelation());
    const input = {
      from_artifact_id: "a1",
      to_artifact_id: "a2",
      relation_type: "derived_from" as const,
    };

    await addArtifactRelation(input);

    expect(mockInvoke).toHaveBeenCalledWith("add_artifact_relation", { input });
  });

  it("should return created relation", async () => {
    const relation = createMockRelation();
    mockInvoke.mockResolvedValue(relation);

    const result = await addArtifactRelation({
      from_artifact_id: "a1",
      to_artifact_id: "a2",
      relation_type: "derived_from",
    });

    expect(result.relation_type).toBe("derived_from");
  });

  it("should validate input before sending", async () => {
    const invalidInput = { from_artifact_id: "a1" } as never;

    await expect(addArtifactRelation(invalidInput)).rejects.toThrow();
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

describe("getArtifactRelations", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call get_artifact_relations command with artifact_id", async () => {
    mockInvoke.mockResolvedValue([createMockRelation()]);

    await getArtifactRelations("a-123");

    expect(mockInvoke).toHaveBeenCalledWith("get_artifact_relations", {
      artifact_id: "a-123",
    });
  });

  it("should return validated array of relations", async () => {
    const relations = [
      createMockRelation({ id: "r1" }),
      createMockRelation({ id: "r2", relation_type: "related_to" }),
    ];
    mockInvoke.mockResolvedValue(relations);

    const result = await getArtifactRelations("a-123");

    expect(result).toHaveLength(2);
    expect(result[1]?.relation_type).toBe("related_to");
  });

  it("should return empty array when no relations", async () => {
    mockInvoke.mockResolvedValue([]);

    const result = await getArtifactRelations("a-123");

    expect(result).toEqual([]);
  });
});
