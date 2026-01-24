/**
 * Project store using Zustand with immer middleware
 *
 * Manages project state for the frontend. Projects are stored in a Record
 * keyed by project ID for O(1) lookup. The active project determines
 * which tasks are displayed in the kanban board.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { Project } from "@/types/project";

// ============================================================================
// State Interface
// ============================================================================

interface ProjectState {
  /** Projects indexed by ID for O(1) lookup */
  projects: Record<string, Project>;
  /** Currently active project ID, or null if none */
  activeProjectId: string | null;
}

// ============================================================================
// Actions Interface
// ============================================================================

interface ProjectActions {
  /** Replace all projects with new array (converts to Record) */
  setProjects: (projects: Project[]) => void;
  /** Update a specific project with partial changes */
  updateProject: (projectId: string, changes: Partial<Project>) => void;
  /** Set the active project ID, or null to deselect */
  selectProject: (projectId: string | null) => void;
  /** Add a single project to the store */
  addProject: (project: Project) => void;
  /** Remove a project from the store */
  removeProject: (projectId: string) => void;
}

// ============================================================================
// Store Implementation
// ============================================================================

export const useProjectStore = create<ProjectState & ProjectActions>()(
  immer((set) => ({
    // Initial state
    projects: {},
    activeProjectId: null,

    // Actions
    setProjects: (projects) =>
      set((state) => {
        state.projects = Object.fromEntries(projects.map((p) => [p.id, p]));
      }),

    updateProject: (projectId, changes) =>
      set((state) => {
        const project = state.projects[projectId];
        if (project) {
          Object.assign(project, changes);
        }
      }),

    selectProject: (projectId) =>
      set((state) => {
        state.activeProjectId = projectId;
      }),

    addProject: (project) =>
      set((state) => {
        state.projects[project.id] = project;
      }),

    removeProject: (projectId) =>
      set((state) => {
        delete state.projects[projectId];
        // Clear selection if removing active project
        if (state.activeProjectId === projectId) {
          state.activeProjectId = null;
        }
      }),
  }))
);

// ============================================================================
// Selectors (defined outside store for memoization)
// ============================================================================

/**
 * Select the currently active project
 * @returns The active project, or null if none selected
 */
export const selectActiveProject = (
  state: ProjectState & ProjectActions
): Project | null =>
  state.activeProjectId
    ? state.projects[state.activeProjectId] ?? null
    : null;

/**
 * Select a project by ID
 * @param projectId - The project ID to look up
 * @returns Selector function returning the project or undefined
 */
export const selectProjectById =
  (projectId: string) =>
  (state: ProjectState): Project | undefined =>
    state.projects[projectId];
