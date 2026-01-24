import { describe, it, expect } from "vitest";
import {
  TaskSchema,
  TaskCategorySchema,
  CreateTaskSchema,
  UpdateTaskSchema,
  TaskListSchema,
  TASK_CATEGORIES,
} from "./task";

describe("TaskSchema", () => {
  const validTask = {
    id: "550e8400-e29b-41d4-a716-446655440000",
    projectId: "project-123",
    category: "feature",
    title: "Test Task",
    description: "A test task description",
    priority: 5,
    internalStatus: "backlog" as const,
    createdAt: "2026-01-24T12:00:00Z",
    updatedAt: "2026-01-24T12:00:00Z",
    startedAt: null,
    completedAt: null,
  };

  it("should parse a valid task", () => {
    expect(() => TaskSchema.parse(validTask)).not.toThrow();
  });

  it("should parse a task with timestamps", () => {
    const taskWithTimestamps = {
      ...validTask,
      startedAt: "2026-01-24T13:00:00Z",
      completedAt: "2026-01-24T14:00:00Z",
    };
    expect(() => TaskSchema.parse(taskWithTimestamps)).not.toThrow();
  });

  it("should parse a task with null description", () => {
    const taskWithNullDescription = {
      ...validTask,
      description: null,
    };
    expect(() => TaskSchema.parse(taskWithNullDescription)).not.toThrow();
  });

  it("should parse all valid internal statuses", () => {
    const statuses = [
      "backlog",
      "ready",
      "blocked",
      "executing",
      "execution_done",
      "qa_refining",
      "qa_testing",
      "qa_passed",
      "qa_failed",
      "pending_review",
      "revision_needed",
      "approved",
      "failed",
      "cancelled",
    ];

    for (const status of statuses) {
      expect(() =>
        TaskSchema.parse({ ...validTask, internalStatus: status })
      ).not.toThrow();
    }
  });

  it("should reject task with empty id", () => {
    expect(() => TaskSchema.parse({ ...validTask, id: "" })).toThrow();
  });

  it("should reject task with empty title", () => {
    expect(() => TaskSchema.parse({ ...validTask, title: "" })).toThrow();
  });

  it("should reject task with invalid status", () => {
    expect(() =>
      TaskSchema.parse({ ...validTask, internalStatus: "invalid" })
    ).toThrow();
  });

  it("should reject task with non-integer priority", () => {
    expect(() =>
      TaskSchema.parse({ ...validTask, priority: 5.5 })
    ).toThrow();
  });

  it("should reject task missing required fields", () => {
    expect(() => TaskSchema.parse({})).toThrow();
    expect(() => TaskSchema.parse({ id: "test" })).toThrow();
  });
});

describe("TaskCategorySchema", () => {
  it("should have 6 categories", () => {
    expect(TASK_CATEGORIES.length).toBe(6);
  });

  it("should parse all valid categories", () => {
    for (const category of TASK_CATEGORIES) {
      expect(TaskCategorySchema.parse(category)).toBe(category);
    }
  });

  it("should include expected categories", () => {
    expect(TASK_CATEGORIES).toContain("feature");
    expect(TASK_CATEGORIES).toContain("bug");
    expect(TASK_CATEGORIES).toContain("chore");
    expect(TASK_CATEGORIES).toContain("docs");
    expect(TASK_CATEGORIES).toContain("test");
    expect(TASK_CATEGORIES).toContain("refactor");
  });

  it("should reject invalid categories", () => {
    expect(() => TaskCategorySchema.parse("invalid")).toThrow();
    expect(() => TaskCategorySchema.parse("Feature")).toThrow();
  });
});

describe("CreateTaskSchema", () => {
  it("should parse valid create task data", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
    };
    expect(() => CreateTaskSchema.parse(createData)).not.toThrow();
  });

  it("should default category to 'feature'", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
    };
    const result = CreateTaskSchema.parse(createData);
    expect(result.category).toBe("feature");
  });

  it("should default priority to 0", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
    };
    const result = CreateTaskSchema.parse(createData);
    expect(result.priority).toBe(0);
  });

  it("should allow custom category and priority", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
      category: "bug",
      priority: 10,
    };
    const result = CreateTaskSchema.parse(createData);
    expect(result.category).toBe("bug");
    expect(result.priority).toBe(10);
  });

  it("should allow optional description", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
      description: "A detailed description",
    };
    const result = CreateTaskSchema.parse(createData);
    expect(result.description).toBe("A detailed description");
  });

  it("should reject empty projectId", () => {
    const result = CreateTaskSchema.safeParse({
      projectId: "",
      title: "Test",
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues[0]?.message).toBe("Project ID is required");
    }
  });

  it("should reject empty title", () => {
    const result = CreateTaskSchema.safeParse({
      projectId: "project-123",
      title: "",
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues[0]?.message).toBe("Title is required");
    }
  });

  it("should allow needsQa as true", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
      needsQa: true,
    };
    const result = CreateTaskSchema.parse(createData);
    expect(result.needsQa).toBe(true);
  });

  it("should allow needsQa as false", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
      needsQa: false,
    };
    const result = CreateTaskSchema.parse(createData);
    expect(result.needsQa).toBe(false);
  });

  it("should allow needsQa as null (inherit from global)", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
      needsQa: null,
    };
    const result = CreateTaskSchema.parse(createData);
    expect(result.needsQa).toBe(null);
  });

  it("should default needsQa to undefined when not provided", () => {
    const createData = {
      projectId: "project-123",
      title: "New Task",
    };
    const result = CreateTaskSchema.parse(createData);
    expect(result.needsQa).toBeUndefined();
  });
});

describe("UpdateTaskSchema", () => {
  it("should allow updating just the title", () => {
    const updateData = { title: "Updated Title" };
    expect(() => UpdateTaskSchema.parse(updateData)).not.toThrow();
  });

  it("should allow updating multiple fields", () => {
    const updateData = {
      title: "Updated Title",
      category: "bug",
      priority: 5,
    };
    expect(() => UpdateTaskSchema.parse(updateData)).not.toThrow();
  });

  it("should allow empty object (no updates)", () => {
    expect(() => UpdateTaskSchema.parse({})).not.toThrow();
  });

  it("should allow setting description to null", () => {
    const updateData = { description: null };
    expect(() => UpdateTaskSchema.parse(updateData)).not.toThrow();
  });

  it("should reject empty string for title", () => {
    const updateData = { title: "" };
    expect(() => UpdateTaskSchema.parse(updateData)).toThrow();
  });

  it("should reject non-integer priority", () => {
    const updateData = { priority: 5.5 };
    expect(() => UpdateTaskSchema.parse(updateData)).toThrow();
  });
});

describe("TaskListSchema", () => {
  it("should parse empty array", () => {
    expect(TaskListSchema.parse([])).toEqual([]);
  });

  it("should parse array of valid tasks", () => {
    const tasks = [
      {
        id: "task-1",
        projectId: "project-1",
        category: "feature",
        title: "Task 1",
        description: null,
        priority: 0,
        internalStatus: "backlog" as const,
        createdAt: "2026-01-24T12:00:00Z",
        updatedAt: "2026-01-24T12:00:00Z",
        startedAt: null,
        completedAt: null,
      },
      {
        id: "task-2",
        projectId: "project-1",
        category: "bug",
        title: "Task 2",
        description: "A bug fix",
        priority: 5,
        internalStatus: "executing" as const,
        createdAt: "2026-01-24T12:00:00Z",
        updatedAt: "2026-01-24T12:00:00Z",
        startedAt: "2026-01-24T13:00:00Z",
        completedAt: null,
      },
    ];
    expect(() => TaskListSchema.parse(tasks)).not.toThrow();
    expect(TaskListSchema.parse(tasks)).toHaveLength(2);
  });

  it("should reject array with invalid task", () => {
    const tasks = [
      {
        id: "task-1",
        title: "Missing fields",
      },
    ];
    expect(() => TaskListSchema.parse(tasks)).toThrow();
  });
});
