import { describe, it, expect, beforeEach } from "vitest";
import {
  useProjectStore,
  selectActiveProject,
  selectProjectById,
} from "./projectStore";
import type { Project } from "@/types/project";

// Helper to create test projects
const createTestProject = (overrides: Partial<Project> = {}): Project => ({
  id: `project-${Math.random().toString(36).slice(2)}`,
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

describe("projectStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useProjectStore.setState({
      projects: {},
      activeProjectId: null,
    });
  });

  describe("setProjects", () => {
    it("converts array to Record keyed by id", () => {
      const projects = [
        createTestProject({ id: "proj-1", name: "Project 1" }),
        createTestProject({ id: "proj-2", name: "Project 2" }),
      ];

      useProjectStore.getState().setProjects(projects);

      const state = useProjectStore.getState();
      expect(Object.keys(state.projects)).toHaveLength(2);
      expect(state.projects["proj-1"]?.name).toBe("Project 1");
      expect(state.projects["proj-2"]?.name).toBe("Project 2");
    });

    it("replaces existing projects", () => {
      useProjectStore.setState({
        projects: {
          old: createTestProject({ id: "old", name: "Old Project" }),
        },
      });

      const newProjects = [
        createTestProject({ id: "new", name: "New Project" }),
      ];
      useProjectStore.getState().setProjects(newProjects);

      const state = useProjectStore.getState();
      expect(state.projects["old"]).toBeUndefined();
      expect(state.projects["new"]?.name).toBe("New Project");
    });

    it("handles empty array", () => {
      useProjectStore.getState().setProjects([]);

      const state = useProjectStore.getState();
      expect(Object.keys(state.projects)).toHaveLength(0);
    });
  });

  describe("updateProject", () => {
    it("modifies existing project", () => {
      const project = createTestProject({
        id: "proj-1",
        name: "Original Name",
      });
      useProjectStore.setState({ projects: { "proj-1": project } });

      useProjectStore
        .getState()
        .updateProject("proj-1", { name: "Updated Name" });

      const state = useProjectStore.getState();
      expect(state.projects["proj-1"]?.name).toBe("Updated Name");
    });

    it("updates multiple fields", () => {
      const project = createTestProject({
        id: "proj-1",
        name: "Original",
        gitMode: "local",
      });
      useProjectStore.setState({ projects: { "proj-1": project } });

      useProjectStore.getState().updateProject("proj-1", {
        name: "Updated",
        gitMode: "worktree",
        worktreePath: "/path/to/worktree",
      });

      const state = useProjectStore.getState();
      const updated = state.projects["proj-1"];
      expect(updated?.name).toBe("Updated");
      expect(updated?.gitMode).toBe("worktree");
      expect(updated?.worktreePath).toBe("/path/to/worktree");
    });

    it("does nothing if project not found", () => {
      const project = createTestProject({ id: "proj-1" });
      useProjectStore.setState({ projects: { "proj-1": project } });

      useProjectStore
        .getState()
        .updateProject("nonexistent", { name: "Updated" });

      const state = useProjectStore.getState();
      expect(Object.keys(state.projects)).toHaveLength(1);
      expect(state.projects["proj-1"]?.name).toBe("Test Project");
    });

    it("preserves other project fields", () => {
      const project = createTestProject({
        id: "proj-1",
        name: "Original",
        workingDirectory: "/original/path",
      });
      useProjectStore.setState({ projects: { "proj-1": project } });

      useProjectStore.getState().updateProject("proj-1", { name: "Updated" });

      const state = useProjectStore.getState();
      const updated = state.projects["proj-1"];
      expect(updated?.name).toBe("Updated");
      expect(updated?.workingDirectory).toBe("/original/path");
    });
  });

  describe("selectProject (set active)", () => {
    it("updates activeProjectId", () => {
      useProjectStore.getState().selectProject("proj-1");

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("proj-1");
    });

    it("sets activeProjectId to null", () => {
      useProjectStore.setState({ activeProjectId: "proj-1" });

      useProjectStore.getState().selectProject(null);

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBeNull();
    });

    it("replaces previous selection", () => {
      useProjectStore.setState({ activeProjectId: "proj-1" });

      useProjectStore.getState().selectProject("proj-2");

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("proj-2");
    });
  });

  describe("addProject", () => {
    it("adds a new project to the store", () => {
      const project = createTestProject({ id: "proj-1" });

      useProjectStore.getState().addProject(project);

      const state = useProjectStore.getState();
      expect(state.projects["proj-1"]).toBeDefined();
    });

    it("overwrites project with same id", () => {
      const project1 = createTestProject({ id: "proj-1", name: "First" });
      const project2 = createTestProject({ id: "proj-1", name: "Second" });

      useProjectStore.getState().addProject(project1);
      useProjectStore.getState().addProject(project2);

      const state = useProjectStore.getState();
      expect(state.projects["proj-1"]?.name).toBe("Second");
    });
  });

  describe("removeProject", () => {
    it("removes a project from the store", () => {
      const project = createTestProject({ id: "proj-1" });
      useProjectStore.setState({ projects: { "proj-1": project } });

      useProjectStore.getState().removeProject("proj-1");

      const state = useProjectStore.getState();
      expect(state.projects["proj-1"]).toBeUndefined();
    });

    it("clears selection if active project is removed", () => {
      const project = createTestProject({ id: "proj-1" });
      useProjectStore.setState({
        projects: { "proj-1": project },
        activeProjectId: "proj-1",
      });

      useProjectStore.getState().removeProject("proj-1");

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBeNull();
    });

    it("does not affect selection if different project is removed", () => {
      const proj1 = createTestProject({ id: "proj-1" });
      const proj2 = createTestProject({ id: "proj-2" });
      useProjectStore.setState({
        projects: { "proj-1": proj1, "proj-2": proj2 },
        activeProjectId: "proj-1",
      });

      useProjectStore.getState().removeProject("proj-2");

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("proj-1");
    });
  });
});

describe("selectors", () => {
  beforeEach(() => {
    useProjectStore.setState({
      projects: {},
      activeProjectId: null,
    });
  });

  describe("selectActiveProject", () => {
    it("returns active project when it exists", () => {
      const project = createTestProject({
        id: "proj-1",
        name: "Active Project",
      });
      useProjectStore.setState({
        projects: { "proj-1": project },
        activeProjectId: "proj-1",
      });

      const result = selectActiveProject(useProjectStore.getState());

      expect(result).not.toBeNull();
      expect(result?.name).toBe("Active Project");
    });

    it("returns null when no project is active", () => {
      const project = createTestProject({ id: "proj-1" });
      useProjectStore.setState({
        projects: { "proj-1": project },
        activeProjectId: null,
      });

      const result = selectActiveProject(useProjectStore.getState());

      expect(result).toBeNull();
    });

    it("returns null when active project does not exist", () => {
      useProjectStore.setState({
        projects: {},
        activeProjectId: "nonexistent",
      });

      const result = selectActiveProject(useProjectStore.getState());

      expect(result).toBeNull();
    });
  });

  describe("selectProjectById", () => {
    it("returns project when it exists", () => {
      const project = createTestProject({ id: "proj-1", name: "Test" });
      useProjectStore.setState({ projects: { "proj-1": project } });

      const selector = selectProjectById("proj-1");
      const result = selector(useProjectStore.getState());

      expect(result?.name).toBe("Test");
    });

    it("returns undefined when project does not exist", () => {
      const selector = selectProjectById("nonexistent");
      const result = selector(useProjectStore.getState());

      expect(result).toBeUndefined();
    });
  });
});
