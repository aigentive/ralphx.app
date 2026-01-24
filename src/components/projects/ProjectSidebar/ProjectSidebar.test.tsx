/**
 * ProjectSidebar component tests
 * Left sidebar showing project list with status indicators and navigation
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ProjectSidebar } from "./ProjectSidebar";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
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

describe("ProjectSidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset stores to initial state
    useProjectStore.setState({ projects: {}, activeProjectId: null });
    useUiStore.setState({ sidebarOpen: true, currentView: "kanban" });
  });

  describe("rendering", () => {
    it("renders sidebar container with correct testid", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByTestId("project-sidebar")).toBeInTheDocument();
    });

    it("renders Projects header", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByText("Projects")).toBeInTheDocument();
    });

    it("applies design system background color", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      const sidebar = screen.getByTestId("project-sidebar");
      expect(sidebar).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });
  });

  describe("project list", () => {
    it("renders empty state when no projects", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByTestId("project-list-empty")).toBeInTheDocument();
      expect(screen.getByText(/no projects/i)).toBeInTheDocument();
    });

    it("renders project items for each project", () => {
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSidebar onNewProject={() => {}} />);

      expect(screen.getByTestId("project-item-project-1")).toBeInTheDocument();
      expect(screen.getByTestId("project-item-project-2")).toBeInTheDocument();
      expect(screen.getByText("Project Alpha")).toBeInTheDocument();
      expect(screen.getByText("Project Beta")).toBeInTheDocument();
    });

    it("highlights active project", () => {
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: "project-1",
      });

      render(<ProjectSidebar onNewProject={() => {}} />);

      const activeItem = screen.getByTestId("project-item-project-1");
      expect(activeItem).toHaveAttribute("data-active", "true");

      const inactiveItem = screen.getByTestId("project-item-project-2");
      expect(inactiveItem).toHaveAttribute("data-active", "false");
    });

    it("calls selectProject when clicking a project", () => {
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSidebar onNewProject={() => {}} />);

      fireEvent.click(screen.getByTestId("project-item-project-1"));

      const state = useProjectStore.getState();
      expect(state.activeProjectId).toBe("project-1");
    });
  });

  describe("git mode indicators", () => {
    it("shows 'Local' badge for local git mode", () => {
      const project = createMockProject({
        id: "project-1",
        name: "Local Project",
        gitMode: "local",
      });

      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: "project-1",
      });

      render(<ProjectSidebar onNewProject={() => {}} />);

      expect(screen.getByText("Local")).toBeInTheDocument();
    });

    it("shows worktree info for worktree git mode", () => {
      const project = createMockProject({
        id: "project-1",
        name: "My Git Project",
        gitMode: "worktree",
        worktreeBranch: "feature/new-feature",
        baseBranch: "main",
      });

      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: "project-1",
      });

      render(<ProjectSidebar onNewProject={() => {}} />);

      // Check for the worktree badge in project item (may appear in multiple places)
      expect(screen.getByText("Worktree")).toBeInTheDocument();
      // Branch name appears in both WorktreeStatus and ProjectItem
      expect(screen.getAllByText("feature/new-feature").length).toBeGreaterThanOrEqual(1);
    });
  });

  describe("New Project button", () => {
    it("renders New Project button", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByRole("button", { name: /new project/i })).toBeInTheDocument();
    });

    it("calls onNewProject when clicked", () => {
      const onNewProject = vi.fn();
      render(<ProjectSidebar onNewProject={onNewProject} />);

      fireEvent.click(screen.getByRole("button", { name: /new project/i }));
      expect(onNewProject).toHaveBeenCalled();
    });
  });

  describe("navigation items", () => {
    it("renders navigation section", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByTestId("sidebar-navigation")).toBeInTheDocument();
    });

    it("renders Ideation navigation item", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByRole("button", { name: /ideation/i })).toBeInTheDocument();
    });

    it("renders Kanban navigation item", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByRole("button", { name: /kanban/i })).toBeInTheDocument();
    });

    it("renders Activity navigation item", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByRole("button", { name: /activity/i })).toBeInTheDocument();
    });

    it("renders Settings navigation item", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByRole("button", { name: /settings/i })).toBeInTheDocument();
    });

    it("highlights current view in navigation", () => {
      useUiStore.setState({ currentView: "ideation" });
      render(<ProjectSidebar onNewProject={() => {}} />);

      const ideationNav = screen.getByRole("button", { name: /ideation/i });
      expect(ideationNav).toHaveAttribute("data-active", "true");
    });

    it("changes view when navigation item clicked", () => {
      useUiStore.setState({ currentView: "kanban" });
      render(<ProjectSidebar onNewProject={() => {}} />);

      fireEvent.click(screen.getByRole("button", { name: /ideation/i }));

      const state = useUiStore.getState();
      expect(state.currentView).toBe("ideation");
    });
  });

  describe("sidebar toggle", () => {
    it("renders close button", () => {
      render(<ProjectSidebar onNewProject={() => {}} />);
      expect(screen.getByTestId("sidebar-close")).toBeInTheDocument();
    });

    it("closes sidebar when close button clicked", () => {
      useUiStore.setState({ sidebarOpen: true });
      render(<ProjectSidebar onNewProject={() => {}} />);

      fireEvent.click(screen.getByTestId("sidebar-close"));

      const state = useUiStore.getState();
      expect(state.sidebarOpen).toBe(false);
    });
  });

  describe("WorktreeStatus indicator", () => {
    it("renders WorktreeStatus for active project with worktree", () => {
      const project = createMockProject({
        id: "project-1",
        name: "Worktree Project",
        gitMode: "worktree",
        worktreeBranch: "feature/branch",
        baseBranch: "main",
      });

      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: "project-1",
      });

      render(<ProjectSidebar onNewProject={() => {}} />);

      expect(screen.getByTestId("worktree-status")).toBeInTheDocument();
    });

    it("shows branch info in worktree status", () => {
      const project = createMockProject({
        id: "project-1",
        gitMode: "worktree",
        worktreeBranch: "feature/awesome",
        baseBranch: "develop",
      });

      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: "project-1",
      });

      render(<ProjectSidebar onNewProject={() => {}} />);

      // Check the worktree status component shows branch info
      const worktreeStatus = screen.getByTestId("worktree-status");
      expect(worktreeStatus).toHaveTextContent("feature/awesome");
      expect(worktreeStatus).toHaveTextContent("from develop");
    });
  });
});
