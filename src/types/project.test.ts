import { describe, it, expect } from "vitest";
import {
  GitModeSchema,
  ProjectSchema,
  CreateProjectSchema,
  UpdateProjectSchema,
} from "./project";

describe("GitModeSchema", () => {
  it("should parse 'local' as valid", () => {
    expect(GitModeSchema.parse("local")).toBe("local");
  });

  it("should parse 'worktree' as valid", () => {
    expect(GitModeSchema.parse("worktree")).toBe("worktree");
  });

  it("should reject invalid git modes", () => {
    expect(() => GitModeSchema.parse("remote")).toThrow();
    expect(() => GitModeSchema.parse("Local")).toThrow();
    expect(() => GitModeSchema.parse("")).toThrow();
  });
});

describe("ProjectSchema", () => {
  const validProject = {
    id: "550e8400-e29b-41d4-a716-446655440000",
    name: "Test Project",
    workingDirectory: "/path/to/project",
    gitMode: "local" as const,
    worktreePath: null,
    worktreeBranch: null,
    baseBranch: null,
    createdAt: "2026-01-24T12:00:00Z",
    updatedAt: "2026-01-24T12:00:00Z",
  };

  it("should parse a valid project", () => {
    expect(() => ProjectSchema.parse(validProject)).not.toThrow();
  });

  it("should parse a project with worktree fields", () => {
    const worktreeProject = {
      ...validProject,
      gitMode: "worktree" as const,
      worktreePath: "/path/to/worktree",
      worktreeBranch: "feature-branch",
      baseBranch: "main",
    };
    expect(() => ProjectSchema.parse(worktreeProject)).not.toThrow();
  });

  it("should reject project with empty id", () => {
    expect(() =>
      ProjectSchema.parse({ ...validProject, id: "" })
    ).toThrow();
  });

  it("should reject project with empty name", () => {
    expect(() =>
      ProjectSchema.parse({ ...validProject, name: "" })
    ).toThrow();
  });

  it("should reject project with empty workingDirectory", () => {
    expect(() =>
      ProjectSchema.parse({ ...validProject, workingDirectory: "" })
    ).toThrow();
  });

  it("should reject project with invalid gitMode", () => {
    expect(() =>
      ProjectSchema.parse({ ...validProject, gitMode: "invalid" })
    ).toThrow();
  });

  it("should reject project with invalid datetime format", () => {
    expect(() =>
      ProjectSchema.parse({ ...validProject, createdAt: "not-a-date" })
    ).toThrow();
  });

  it("should reject project missing required fields", () => {
    expect(() => ProjectSchema.parse({})).toThrow();
    expect(() => ProjectSchema.parse({ id: "test" })).toThrow();
  });
});

describe("CreateProjectSchema", () => {
  it("should parse valid create project data", () => {
    const createData = {
      name: "New Project",
      workingDirectory: "/path/to/project",
      gitMode: "local" as const,
    };
    expect(() => CreateProjectSchema.parse(createData)).not.toThrow();
  });

  it("should default gitMode to 'local'", () => {
    const createData = {
      name: "New Project",
      workingDirectory: "/path/to/project",
    };
    const result = CreateProjectSchema.parse(createData);
    expect(result.gitMode).toBe("local");
  });

  it("should accept worktree configuration", () => {
    const createData = {
      name: "New Project",
      workingDirectory: "/path/to/project",
      gitMode: "worktree" as const,
      worktreePath: "/path/to/worktree",
      worktreeBranch: "feature-branch",
      baseBranch: "main",
    };
    expect(() => CreateProjectSchema.parse(createData)).not.toThrow();
  });

  it("should reject empty name", () => {
    const createData = {
      name: "",
      workingDirectory: "/path/to/project",
    };
    const result = CreateProjectSchema.safeParse(createData);
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues[0]?.message).toBe("Project name is required");
    }
  });

  it("should reject empty workingDirectory", () => {
    const createData = {
      name: "Test",
      workingDirectory: "",
    };
    const result = CreateProjectSchema.safeParse(createData);
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues[0]?.message).toBe("Working directory is required");
    }
  });
});

describe("UpdateProjectSchema", () => {
  it("should allow updating just the name", () => {
    const updateData = { name: "Updated Name" };
    expect(() => UpdateProjectSchema.parse(updateData)).not.toThrow();
  });

  it("should allow updating multiple fields", () => {
    const updateData = {
      name: "Updated Name",
      workingDirectory: "/new/path",
      gitMode: "worktree" as const,
    };
    expect(() => UpdateProjectSchema.parse(updateData)).not.toThrow();
  });

  it("should allow empty object (no updates)", () => {
    expect(() => UpdateProjectSchema.parse({})).not.toThrow();
  });

  it("should allow setting nullable fields to null", () => {
    const updateData = {
      worktreePath: null,
      worktreeBranch: null,
      baseBranch: null,
    };
    expect(() => UpdateProjectSchema.parse(updateData)).not.toThrow();
  });

  it("should reject empty string for name", () => {
    const updateData = { name: "" };
    expect(() => UpdateProjectSchema.parse(updateData)).toThrow();
  });
});
