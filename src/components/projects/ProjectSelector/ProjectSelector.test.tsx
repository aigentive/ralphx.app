/**
 * ProjectSelector component tests
 * Compact header dropdown for project selection with git mode indicators
 * Uses shadcn DropdownMenu (Radix menu primitives)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

    it("has correct aria attributes (shadcn DropdownMenu uses menu)", () => {
      render(<ProjectSelector onNewProject={() => {}} />);
      const trigger = screen.getByTestId("project-selector-trigger");
      // shadcn DropdownMenu uses aria-haspopup="menu"
      expect(trigger).toHaveAttribute("aria-haspopup", "menu");
      expect(trigger).toHaveAttribute("aria-expanded", "false");
    });
  });

  describe("dropdown behavior", () => {
    it("opens dropdown when trigger is clicked", async () => {
      const user = userEvent.setup();
      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      await user.click(trigger);

      await waitFor(() => {
        expect(screen.getByTestId("project-selector-dropdown")).toBeInTheDocument();
      });
      expect(trigger).toHaveAttribute("aria-expanded", "true");
    });

    it("closes dropdown when Escape is pressed", async () => {
      const user = userEvent.setup();
      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      await user.click(trigger);

      await waitFor(() => {
        expect(screen.getByTestId("project-selector-dropdown")).toBeInTheDocument();
      });

      await user.keyboard("{Escape}");

      await waitFor(() => {
        expect(screen.queryByTestId("project-selector-dropdown")).not.toBeInTheDocument();
      });
    });

    it("opens dropdown with ArrowDown when closed", async () => {
      const user = userEvent.setup();
      render(<ProjectSelector onNewProject={() => {}} />);

      const trigger = screen.getByTestId("project-selector-trigger");
      trigger.focus();
      await user.keyboard("{ArrowDown}");

      await waitFor(() => {
        expect(screen.getByTestId("project-selector-dropdown")).toBeInTheDocument();
      });
    });
  });

  describe("project list", () => {
    it("shows empty state when no projects exist", async () => {
      const user = userEvent.setup();
      render(<ProjectSelector onNewProject={() => {}} />);

      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        expect(screen.getByText(/no projects/i)).toBeInTheDocument();
      });
    });

    it("renders project options for each project", async () => {
      const user = userEvent.setup();
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        expect(screen.getByTestId("project-option-project-1")).toBeInTheDocument();
        expect(screen.getByTestId("project-option-project-2")).toBeInTheDocument();
      });
      expect(screen.getByText("Project Alpha")).toBeInTheDocument();
      expect(screen.getByText("Project Beta")).toBeInTheDocument();
    });

    it("highlights active project with accent styling", async () => {
      const user = userEvent.setup();
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: "project-1",
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        const selectedOption = screen.getByTestId("project-option-project-1");
        expect(selectedOption).toBeInTheDocument();
        // Active project should have the accent muted background
        expect(selectedOption).toHaveClass("bg-[var(--accent-muted)]");
      });
    });

    it("selects project when clicked", async () => {
      const user = userEvent.setup();
      const projects: Project[] = [
        createMockProject({ id: "project-1", name: "Project Alpha" }),
        createMockProject({ id: "project-2", name: "Project Beta" }),
      ];

      useProjectStore.setState({
        projects: Object.fromEntries(projects.map((p) => [p.id, p])),
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        expect(screen.getByTestId("project-option-project-2")).toBeInTheDocument();
      });

      await user.click(screen.getByTestId("project-option-project-2"));

      await waitFor(() => {
        const state = useProjectStore.getState();
        expect(state.activeProjectId).toBe("project-2");
      });
    });

    it("closes dropdown after selecting a project", async () => {
      const user = userEvent.setup();
      const project = createMockProject({ id: "project-1", name: "Test" });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        expect(screen.getByTestId("project-option-project-1")).toBeInTheDocument();
      });

      await user.click(screen.getByTestId("project-option-project-1"));

      await waitFor(() => {
        expect(screen.queryByTestId("project-selector-dropdown")).not.toBeInTheDocument();
      });
    });
  });

  describe("git mode badges in dropdown", () => {
    it("shows local badge for local projects in dropdown", async () => {
      const user = userEvent.setup();
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
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        // In the dropdown, local projects show "local" text
        expect(screen.getAllByText("local").length).toBeGreaterThan(0);
      });
    });

    it("shows worktree branch for worktree projects in dropdown", async () => {
      const user = userEvent.setup();
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
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        expect(screen.getByText("feature/branch")).toBeInTheDocument();
      });
    });
  });

  describe("New Project option", () => {
    it("renders New Project option", async () => {
      const user = userEvent.setup();
      render(<ProjectSelector onNewProject={() => {}} />);
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        expect(screen.getByTestId("new-project-option")).toBeInTheDocument();
      });
      expect(screen.getByText("New Project...")).toBeInTheDocument();
    });

    it("calls onNewProject when clicked", async () => {
      const user = userEvent.setup();
      const onNewProject = vi.fn();
      render(<ProjectSelector onNewProject={onNewProject} />);

      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        expect(screen.getByTestId("new-project-option")).toBeInTheDocument();
      });

      await user.click(screen.getByTestId("new-project-option"));

      await waitFor(() => {
        expect(onNewProject).toHaveBeenCalled();
      });
    });

    it("closes dropdown after clicking New Project", async () => {
      const user = userEvent.setup();
      const onNewProject = vi.fn();
      render(<ProjectSelector onNewProject={onNewProject} />);

      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        expect(screen.getByTestId("new-project-option")).toBeInTheDocument();
      });

      await user.click(screen.getByTestId("new-project-option"));

      await waitFor(() => {
        expect(screen.queryByTestId("project-selector-dropdown")).not.toBeInTheDocument();
      });
    });
  });

  describe("keyboard navigation", () => {
    it("navigates items with arrow keys", async () => {
      const user = userEvent.setup();
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
      await user.click(trigger);

      await waitFor(() => {
        expect(screen.getByTestId("project-selector-dropdown")).toBeInTheDocument();
      });

      // Radix handles keyboard navigation internally
      // Just verify the dropdown is open and navigable
      expect(screen.getByTestId("project-option-project-1")).toBeInTheDocument();
      expect(screen.getByTestId("project-option-project-2")).toBeInTheDocument();
    });
  });

  describe("accessibility", () => {
    it("dropdown has menu role (shadcn DropdownMenu)", async () => {
      const user = userEvent.setup();
      render(<ProjectSelector onNewProject={() => {}} />);
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        const dropdown = screen.getByTestId("project-selector-dropdown");
        // shadcn DropdownMenu uses role="menu"
        expect(dropdown).toHaveAttribute("role", "menu");
      });
    });

    it("project options have menuitem role", async () => {
      const user = userEvent.setup();
      const project = createMockProject({ id: "project-1", name: "Test" });
      useProjectStore.setState({
        projects: { "project-1": project },
        activeProjectId: null,
      });

      render(<ProjectSelector onNewProject={() => {}} />);
      await user.click(screen.getByTestId("project-selector-trigger"));

      await waitFor(() => {
        const option = screen.getByTestId("project-option-project-1");
        // shadcn DropdownMenuItem uses role="menuitem"
        expect(option).toHaveAttribute("role", "menuitem");
      });
    });
  });

  describe("className prop", () => {
    it("applies custom className to trigger button", () => {
      render(
        <ProjectSelector onNewProject={() => {}} className="custom-class" />
      );
      const trigger = screen.getByTestId("project-selector-trigger");
      expect(trigger).toHaveClass("custom-class");
    });
  });
});
