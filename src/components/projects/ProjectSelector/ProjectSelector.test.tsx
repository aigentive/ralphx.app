/**
 * ProjectSelector component tests
 * Compact header dropdown for project selection with git mode indicators
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ProjectSelector } from "./ProjectSelector";
import { useProjectStore } from "@/stores/projectStore";
import type { Project } from "@/types/project";

// Create a mock project
const createMockProject = (overrides: Partial<Project> = {}): Project => ({
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

describe("ProjectSelector", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset stores to initial state
    useProjectStore.setState({ projects: {}, activeProjectId: null });
  });

  describe("trigger button", () => {
    it("renders trigger button with correct testid", () => {
      render(<ProjectSelector onNewProject={() => {}} />);
      expect(screen.getByTestId("project-selector-trigger")).toBeInTheDocument();
    });

    it("shows 'Select Project' when no project is active", () => {
      render(<ProjectSelector onNewProject={() => {}} />);
      expect(screen.getByText("Select Project")).toBeInTheDocument();
    });

    it("shows active project name when a project is selected", () => {
      const project = createMockProject({ id: "project-1", name: "My Project" });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: "project-1",
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      expect(screen.getByText("My Project")).toBeInTheDocument();
    });

    it("shows git mode indicator for active local project", () => {
      const project = createMockProject({
        id: "project-1",
        name: "Local Project",
        gitMode: "local",
      });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: "project-1",
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      expect(screen.getByText("local")).toBeInTheDocument();
    });

    it("shows worktree branch for active worktree project", () => {
      const project = createMockProject({
        id: "project-1",
        name: "Worktree Project",
        gitMode: "worktree",
        worktreeBranch: "feature/test",
      });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: "project-1",
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      expect(screen.getByText("feature/test")).toBeInTheDocument();
    });

    it("has correct aria attributes", () => {
      render(<ProjectSelector onNewProject={() => {}} />);
      const trigger = screen.getByTestId("project-selector-trigger");
      expect(trigger).toHaveAttribute("aria-haspopup", "listbox");
      expect(trigger).toHaveAttribute("aria-expanded", "false");
    });
  });

  describe("dropdown behavior", () => {
    it("opens dropdown when trigger is clicked", () => {
      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);

      expect(screen.getByTestId("project-selector-dropdown")).toBeInTheDocument();
      expect(trigger).toHaveAttribute("aria-expanded", "true");
    });

    it("closes dropdown when trigger is clicked again", () => {
      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);
      expect(screen.getByTestId("project-selector-dropdown")).toBeInTheDocument();

      fireEvent.click(trigger);
      expect(screen.queryByTestId("project-selector-dropdown")).not.toBeInTheDocument();
    });

    it("closes dropdown when Escape is pressed", () => {
      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);
      expect(screen.getByTestId("project-selector-dropdown")).toBeInTheDocument();

      fireEvent.keyDown(trigger, { key: "Escape" });
      expect(screen.queryByTestId("project-selector-dropdown")).not.toBeInTheDocument();
    });

    it("opens dropdown with ArrowDown when closed", () => {
      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.keyDown(trigger, { key: "ArrowDown" });

      expect(screen.getByTestId("project-selector-dropdown")).toBeInTheDocument();
    });
  });

  describe("project list", () => {
    it("shows empty state when no projects exist", () => {
      render(<ProjectSelector onNewProject={() => {}} />);

      fireEvent.click(screen.getByTestId("project-selector-trigger"));

      expect(screen.getByText(/no projects/i)).toBeInTheDocument();
    });

    it("renders project options for each project", () => {
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));

      expect(screen.getByTestId("project-option-project-1")).toBeInTheDocument();
      expect(screen.getByTestId("project-option-project-2")).toBeInTheDocument();
      expect(screen.getByText("Project Alpha")).toBeInTheDocument();
      expect(screen.getByText("Project Beta")).toBeInTheDocument();
    });

    it("shows check icon for selected project", () => {
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: "project-1",
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));

      const selectedOption = screen.getByTestId("project-option-project-1");
      expect(selectedOption).toHaveAttribute("aria-selected", "true");
    });

    it("selects project when clicked", () => {
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));
      fireEvent.click(screen.getByTestId("project-option-project-2"));

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("project-2");
    });

    it("closes dropdown after selecting a project", () => {
      const project = createMockProject({ id: "project-1", name: "Test" });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));
      fireEvent.click(screen.getByTestId("project-option-project-1"));

      expect(screen.queryByTestId("project-selector-dropdown")).not.toBeInTheDocument();
    });
  });

  describe("git mode badges in dropdown", () => {
    it("shows Local badge for local projects", () => {
      const project = createMockProject({
        id: "project-1",
        name: "Local Project",
        gitMode: "local",
      });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));

      expect(screen.getByText("Local")).toBeInTheDocument();
    });

    it("shows Worktree badge for worktree projects", () => {
      const project = createMockProject({
        id: "project-1",
        name: "Worktree Project",
        gitMode: "worktree",
        worktreeBranch: "feature/branch",
      });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));

      expect(screen.getByText("Worktree")).toBeInTheDocument();
      expect(screen.getByText("feature/branch")).toBeInTheDocument();
    });
  });

  describe("New Project option", () => {
    it("renders New Project option", () => {
      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));

      expect(screen.getByTestId("new-project-option")).toBeInTheDocument();
      expect(screen.getByText("New Project")).toBeInTheDocument();
    });

    it("calls onNewProject when clicked", () => {
      const onNewProject = vi.fn();
      render(<ProjectSelector onNewProject={onNewProject} />);

      fireEvent.click(screen.getByTestId("project-selector-trigger"));
      fireEvent.click(screen.getByTestId("new-project-option"));

      expect(onNewProject).toHaveBeenCalled();
    });

    it("closes dropdown after clicking New Project", () => {
      const onNewProject = vi.fn();
      render(<ProjectSelector onNewProject={onNewProject} />);

      fireEvent.click(screen.getByTestId("project-selector-trigger"));
      fireEvent.click(screen.getByTestId("new-project-option"));

      expect(screen.queryByTestId("project-selector-dropdown")).not.toBeInTheDocument();
    });
  });

  describe("keyboard navigation", () => {
    it("navigates to next item with ArrowDown", () => {
      // Projects are sorted by updatedAt descending, so project-2 (newer) comes first
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha", updatedAt: "2026-01-24T11:00:00Z" }),
        createMockProject({ id: "project-2", name: "Project Beta", updatedAt: "2026-01-24T12:00:00Z" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);

      // After opening, ArrowDown moves focus. First item is project-2 (most recent).
      // Another ArrowDown moves to project-1.
      fireEvent.keyDown(trigger, { key: "ArrowDown" });
      fireEvent.keyDown(trigger, { key: "Enter" });

      // Should have selected project-1 (second item after sort)
      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("project-1");
    });

    it("navigates to previous item with ArrowUp", () => {
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);

      // ArrowUp from start wraps to end (New Project option)
      fireEvent.keyDown(trigger, { key: "ArrowUp" });
      fireEvent.keyDown(trigger, { key: "ArrowUp" });
      fireEvent.keyDown(trigger, { key: "Enter" });

      // Should have selected last project (project-2)
      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("project-2");
    });

    it("jumps to first item with Home", () => {
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);

      fireEvent.keyDown(trigger, { key: "End" });
      fireEvent.keyDown(trigger, { key: "Home" });
      fireEvent.keyDown(trigger, { key: "Enter" });

      // Should have selected first project
      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("project-1");
    });

    it("jumps to last item with End", () => {
      const onNewProject = vi.fn();
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={onNewProject} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);

      fireEvent.keyDown(trigger, { key: "End" });
      fireEvent.keyDown(trigger, { key: "Enter" });

      // Should have triggered New Project (last item)
      expect(onNewProject).toHaveBeenCalled();
    });

    it("selects item with Enter", () => {
      const project = createMockProject({ id: "project-1", name: "Test" });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);
      fireEvent.keyDown(trigger, { key: "Enter" });

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("project-1");
    });

    it("selects item with Space", () => {
      const project = createMockProject({ id: "project-1", name: "Test" });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      fireEvent.click(trigger);
      fireEvent.keyDown(trigger, { key: " " });

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("project-1");
    });
  });

  describe("accessibility", () => {
    it("has accessible label when project is selected", () => {
      const project = createMockProject({ id: "project-1", name: "My Project" });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: "project-1",
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      const trigger = screen.getByTestId("project-selector-trigger");
      expect(trigger).toHaveAttribute("aria-label", "Current project: My Project");
    });

    it("has accessible label when no project is selected", () => {
      render(<ProjectSelector onNewProject={() => {}} />);
      const trigger = screen.getByTestId("project-selector-trigger");
      expect(trigger).toHaveAttribute("aria-label", "Select a project");
    });

    it("dropdown has listbox role", () => {
      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));

      const dropdown = screen.getByTestId("project-selector-dropdown");
      expect(dropdown).toHaveAttribute("role", "listbox");
    });

    it("project options have option role", () => {
      const project = createMockProject({ id: "project-1", name: "Test" });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      fireEvent.click(screen.getByTestId("project-selector-trigger"));

      const option = screen.getByTestId("project-option-project-1");
      expect(option).toHaveAttribute("role", "option");
    });
  });

  describe("className prop", () => {
    it("applies custom className to container", () => {
      const { container } = render(
        <ProjectSelector onNewProject={() => {}} className="custom-class" />
      );
      expect(container.firstChild).toHaveClass("custom-class");
    });
  });
});
