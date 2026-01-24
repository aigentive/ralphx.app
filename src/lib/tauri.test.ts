import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { typedInvoke, api, HealthResponseSchema } from "./tauri";

// Cast invoke to a mock function for testing
const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("typedInvoke", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should invoke the command with the given arguments", async () => {
    const schema = z.object({ value: z.number() });
    mockInvoke.mockResolvedValue({ value: 42 });

    await typedInvoke("test_command", { arg1: "test" }, schema);

    expect(mockInvoke).toHaveBeenCalledWith("test_command", { arg1: "test" });
  });

  it("should return validated response when schema matches", async () => {
    const schema = z.object({ value: z.number() });
    mockInvoke.mockResolvedValue({ value: 42 });

    const result = await typedInvoke("test_command", {}, schema);

    expect(result).toEqual({ value: 42 });
  });

  it("should throw when response doesn't match schema", async () => {
    const schema = z.object({ value: z.number() });
    mockInvoke.mockResolvedValue({ value: "not a number" });

    await expect(typedInvoke("test_command", {}, schema)).rejects.toThrow();
  });

  it("should throw when response is missing required fields", async () => {
    const schema = z.object({ required: z.string() });
    mockInvoke.mockResolvedValue({});

    await expect(typedInvoke("test_command", {}, schema)).rejects.toThrow();
  });

  it("should handle null values according to schema", async () => {
    const schema = z.object({ value: z.string().nullable() });
    mockInvoke.mockResolvedValue({ value: null });

    const result = await typedInvoke("test_command", {}, schema);

    expect(result).toEqual({ value: null });
  });

  it("should handle arrays according to schema", async () => {
    const schema = z.array(z.number());
    mockInvoke.mockResolvedValue([1, 2, 3]);

    const result = await typedInvoke("test_command", {}, schema);

    expect(result).toEqual([1, 2, 3]);
  });

  it("should propagate invoke errors", async () => {
    const schema = z.object({ value: z.number() });
    mockInvoke.mockRejectedValue(new Error("Backend error"));

    await expect(typedInvoke("test_command", {}, schema)).rejects.toThrow(
      "Backend error"
    );
  });
});

describe("HealthResponseSchema", () => {
  it("should parse valid health response", () => {
    const response = { status: "ok" };
    expect(() => HealthResponseSchema.parse(response)).not.toThrow();
  });

  it("should reject response without status", () => {
    expect(() => HealthResponseSchema.parse({})).toThrow();
  });

  it("should reject response with non-string status", () => {
    expect(() => HealthResponseSchema.parse({ status: 123 })).toThrow();
  });
});

describe("api.health", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("should call health_check command", async () => {
    mockInvoke.mockResolvedValue({ status: "ok" });

    await api.health.check();

    expect(mockInvoke).toHaveBeenCalledWith("health_check", {});
  });

  it("should return health response", async () => {
    mockInvoke.mockResolvedValue({ status: "ok" });

    const result = await api.health.check();

    expect(result).toEqual({ status: "ok" });
  });

  it("should validate response with HealthResponseSchema", async () => {
    mockInvoke.mockResolvedValue({ status: 123 }); // Invalid

    await expect(api.health.check()).rejects.toThrow();
  });

  it("should propagate backend errors", async () => {
    mockInvoke.mockRejectedValue(new Error("Connection failed"));

    await expect(api.health.check()).rejects.toThrow("Connection failed");
  });
});

// Helper to create mock task
const createMockTask = (overrides = {}) => ({
  id: "task-1",
  projectId: "project-1",
  category: "feature",
  title: "Test Task",
  description: null,
  priority: 0,
  internalStatus: "backlog",
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  startedAt: null,
  completedAt: null,
  ...overrides,
});

// Helper to create mock project
const createMockProject = (overrides = {}) => ({
  id: "project-1",
  name: "Test Project",
  workingDirectory: "/path/to/project",
  gitMode: "local",
  worktreePath: null,
  worktreeBranch: null,
  baseBranch: null,
  createdAt: "2026-01-24T12:00:00Z",
  updatedAt: "2026-01-24T12:00:00Z",
  ...overrides,
});

describe("api.tasks", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("list", () => {
    it("should call list_tasks with projectId", async () => {
      mockInvoke.mockResolvedValue([createMockTask()]);

      await api.tasks.list("project-1");

      expect(mockInvoke).toHaveBeenCalledWith("list_tasks", {
        projectId: "project-1",
      });
    });

    it("should return array of tasks", async () => {
      const tasks = [createMockTask({ id: "t1" }), createMockTask({ id: "t2" })];
      mockInvoke.mockResolvedValue(tasks);

      const result = await api.tasks.list("project-1");

      expect(result).toHaveLength(2);
      expect(result[0]?.id).toBe("t1");
    });

    it("should validate task schema", async () => {
      mockInvoke.mockResolvedValue([{ invalid: "task" }]);

      await expect(api.tasks.list("project-1")).rejects.toThrow();
    });
  });

  describe("get", () => {
    it("should call get_task with taskId", async () => {
      mockInvoke.mockResolvedValue(createMockTask());

      await api.tasks.get("task-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_task", { taskId: "task-1" });
    });

    it("should return task", async () => {
      const task = createMockTask({ title: "My Task" });
      mockInvoke.mockResolvedValue(task);

      const result = await api.tasks.get("task-1");

      expect(result.title).toBe("My Task");
    });
  });

  describe("create", () => {
    it("should call create_task with input", async () => {
      mockInvoke.mockResolvedValue(createMockTask());
      const input = { projectId: "p1", title: "New Task" };

      await api.tasks.create(input);

      expect(mockInvoke).toHaveBeenCalledWith("create_task", { input });
    });

    it("should return created task", async () => {
      const task = createMockTask({ title: "Created Task" });
      mockInvoke.mockResolvedValue(task);

      const result = await api.tasks.create({ projectId: "p1", title: "Created Task" });

      expect(result.title).toBe("Created Task");
    });
  });

  describe("update", () => {
    it("should call update_task with taskId and input", async () => {
      mockInvoke.mockResolvedValue(createMockTask());
      const input = { title: "Updated" };

      await api.tasks.update("task-1", input);

      expect(mockInvoke).toHaveBeenCalledWith("update_task", {
        taskId: "task-1",
        input,
      });
    });
  });

  describe("delete", () => {
    it("should call delete_task with taskId", async () => {
      mockInvoke.mockResolvedValue(true);

      await api.tasks.delete("task-1");

      expect(mockInvoke).toHaveBeenCalledWith("delete_task", { taskId: "task-1" });
    });

    it("should return boolean", async () => {
      mockInvoke.mockResolvedValue(true);

      const result = await api.tasks.delete("task-1");

      expect(result).toBe(true);
    });
  });

  describe("move", () => {
    it("should call move_task with taskId and toStatus", async () => {
      mockInvoke.mockResolvedValue(createMockTask({ internalStatus: "ready" }));

      await api.tasks.move("task-1", "ready");

      expect(mockInvoke).toHaveBeenCalledWith("move_task", {
        taskId: "task-1",
        toStatus: "ready",
      });
    });
  });
});

describe("api.projects", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("list", () => {
    it("should call list_projects", async () => {
      mockInvoke.mockResolvedValue([createMockProject()]);

      await api.projects.list();

      expect(mockInvoke).toHaveBeenCalledWith("list_projects", {});
    });

    it("should return array of projects", async () => {
      const projects = [
        createMockProject({ id: "p1" }),
        createMockProject({ id: "p2" }),
      ];
      mockInvoke.mockResolvedValue(projects);

      const result = await api.projects.list();

      expect(result).toHaveLength(2);
    });

    it("should validate project schema", async () => {
      mockInvoke.mockResolvedValue([{ invalid: "project" }]);

      await expect(api.projects.list()).rejects.toThrow();
    });
  });

  describe("get", () => {
    it("should call get_project with projectId", async () => {
      mockInvoke.mockResolvedValue(createMockProject());

      await api.projects.get("project-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_project", {
        projectId: "project-1",
      });
    });

    it("should return project", async () => {
      const project = createMockProject({ name: "My Project" });
      mockInvoke.mockResolvedValue(project);

      const result = await api.projects.get("project-1");

      expect(result.name).toBe("My Project");
    });
  });

  describe("create", () => {
    it("should call create_project with input", async () => {
      mockInvoke.mockResolvedValue(createMockProject());
      const input = { name: "New Project", workingDirectory: "/path" };

      await api.projects.create(input);

      expect(mockInvoke).toHaveBeenCalledWith("create_project", { input });
    });
  });

  describe("update", () => {
    it("should call update_project with projectId and input", async () => {
      mockInvoke.mockResolvedValue(createMockProject());
      const input = { name: "Updated" };

      await api.projects.update("project-1", input);

      expect(mockInvoke).toHaveBeenCalledWith("update_project", {
        projectId: "project-1",
        input,
      });
    });
  });

  describe("delete", () => {
    it("should call delete_project with projectId", async () => {
      mockInvoke.mockResolvedValue(true);

      await api.projects.delete("project-1");

      expect(mockInvoke).toHaveBeenCalledWith("delete_project", {
        projectId: "project-1",
      });
    });

    it("should return boolean", async () => {
      mockInvoke.mockResolvedValue(true);

      const result = await api.projects.delete("project-1");

      expect(result).toBe(true);
    });
  });
});
