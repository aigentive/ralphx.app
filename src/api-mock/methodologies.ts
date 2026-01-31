/**
 * Mock Methodologies API
 *
 * Mirrors the interface of src/api/methodologies.ts with mock implementations.
 */

import type {
  MethodologyResponse,
  MethodologyActivationResponse,
} from "@/api/methodologies";

// ============================================================================
// Mock Data
// ============================================================================

const mockMethodology: MethodologyResponse = {
  id: "methodology-scrum",
  name: "Scrum Methodology",
  description: "Agile Scrum workflow with sprints and ceremonies",
  agent_profiles: ["developer", "reviewer", "scrum-master"],
  skills: ["code-review", "sprint-planning", "retrospective"],
  workflow_id: "workflow-scrum",
  workflow_name: "Scrum Workflow",
  phases: [
    {
      id: "phase-planning",
      name: "Sprint Planning",
      order: 0,
      description: "Plan the sprint backlog",
      agent_profiles: ["scrum-master"],
      column_ids: ["col-backlog"],
    },
    {
      id: "phase-development",
      name: "Development",
      order: 1,
      description: "Execute sprint tasks",
      agent_profiles: ["developer"],
      column_ids: ["col-in-progress"],
    },
    {
      id: "phase-review",
      name: "Sprint Review",
      order: 2,
      description: "Review completed work",
      agent_profiles: ["reviewer"],
      column_ids: ["col-review"],
    },
  ],
  templates: [
    {
      artifact_type: "sprint_goal",
      template_path: "templates/sprint-goal.md",
      name: "Sprint Goal Template",
      description: "Template for defining sprint goals",
    },
  ],
  is_active: false,
  phase_count: 3,
  agent_count: 3,
  created_at: new Date().toISOString(),
};

const mockMethodologies: MethodologyResponse[] = [mockMethodology];

// ============================================================================
// Mock Methodologies API
// ============================================================================

export const mockMethodologiesApi = {
  /**
   * Get all methodologies
   */
  getAll: async (): Promise<MethodologyResponse[]> => {
    return mockMethodologies;
  },

  /**
   * Get the currently active methodology (if any)
   */
  getActive: async (): Promise<MethodologyResponse | null> => {
    const active = mockMethodologies.find((m) => m.is_active);
    return active ?? null;
  },

  /**
   * Activate a methodology by ID
   */
  activate: async (id: string): Promise<MethodologyActivationResponse> => {
    const methodology = mockMethodologies.find((m) => m.id === id);
    if (!methodology) {
      throw new Error(`Methodology not found: ${id}`);
    }
    return {
      methodology: { ...methodology, is_active: true },
      workflow: {
        id: methodology.workflow_id,
        name: methodology.workflow_name,
        description: methodology.description,
        column_count: 5,
      },
      agent_profiles: methodology.agent_profiles,
      skills: methodology.skills,
      previous_methodology_id: null,
    };
  },

  /**
   * Deactivate a methodology by ID
   */
  deactivate: async (id: string): Promise<MethodologyResponse> => {
    const methodology = mockMethodologies.find((m) => m.id === id);
    if (!methodology) {
      throw new Error(`Methodology not found: ${id}`);
    }
    return { ...methodology, is_active: false };
  },
} as const;
