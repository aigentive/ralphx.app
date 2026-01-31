/**
 * Mock Research API
 *
 * Mirrors the interface of src/api/research.ts with mock implementations.
 */

import type {
  ResearchProcessResponse,
  ResearchPresetResponse,
  StartResearchInput,
} from "@/api/research";

// ============================================================================
// Mock Data
// ============================================================================

const mockPresets: ResearchPresetResponse[] = [
  {
    id: "preset-quick",
    name: "Quick Research",
    max_iterations: 5,
    timeout_hours: 1,
    checkpoint_interval: 2,
    description: "Fast research for simple questions",
  },
  {
    id: "preset-standard",
    name: "Standard Research",
    max_iterations: 20,
    timeout_hours: 4,
    checkpoint_interval: 5,
    description: "Balanced research depth and time",
  },
  {
    id: "preset-deep",
    name: "Deep Research",
    max_iterations: 50,
    timeout_hours: 12,
    checkpoint_interval: 10,
    description: "Comprehensive research for complex topics",
  },
];

const mockProcesses: ResearchProcessResponse[] = [
  {
    id: "research-001",
    name: "API Design Patterns",
    question: "What are the best practices for REST API design in 2024?",
    context: "Building a new microservices architecture",
    scope: "REST APIs, OpenAPI, versioning strategies",
    constraints: ["Focus on scalability", "Consider backwards compatibility"],
    agent_profile_id: "researcher",
    depth_preset: "standard",
    max_iterations: 50,
    timeout_hours: 2,
    checkpoint_interval: 10,
    target_bucket: "research-artifacts",
    status: "running",
    current_iteration: 20,
    progress_percentage: 40,
    error_message: null,
    created_at: new Date(Date.now() - 3600000).toISOString(),
    started_at: new Date(Date.now() - 3000000).toISOString(),
    completed_at: null,
  },
  {
    id: "research-002",
    name: "State Management Comparison",
    question: "Compare Redux, Zustand, and Jotai for large React applications",
    context: null,
    scope: "React state management libraries",
    constraints: [],
    agent_profile_id: "researcher",
    depth_preset: "deep-dive",
    max_iterations: 200,
    timeout_hours: 8,
    checkpoint_interval: 25,
    target_bucket: "research-artifacts",
    status: "completed",
    current_iteration: 200,
    progress_percentage: 100,
    error_message: null,
    created_at: new Date(Date.now() - 86400000).toISOString(),
    started_at: new Date(Date.now() - 86400000).toISOString(),
    completed_at: new Date(Date.now() - 43200000).toISOString(),
  },
  {
    id: "research-003",
    name: "Testing Strategies",
    question: "What testing strategies work best for Tauri applications?",
    context: "Desktop app with Rust backend and React frontend",
    scope: null,
    constraints: ["Include E2E testing"],
    agent_profile_id: "researcher",
    depth_preset: "quick-scan",
    max_iterations: 10,
    timeout_hours: 0.5,
    checkpoint_interval: 5,
    target_bucket: "research-artifacts",
    status: "paused",
    current_iteration: 6,
    progress_percentage: 60,
    error_message: null,
    created_at: new Date(Date.now() - 7200000).toISOString(),
    started_at: new Date(Date.now() - 7000000).toISOString(),
    completed_at: null,
  },
];

// ============================================================================
// Mock Research API
// ============================================================================

export const mockResearchApi = {
  /**
   * Get all research processes, optionally filtered by status
   */
  getProcesses: async (status?: string): Promise<ResearchProcessResponse[]> => {
    if (status) {
      return mockProcesses.filter((p) => p.status === status);
    }
    return mockProcesses;
  },

  /**
   * Get a single research process by ID
   */
  getProcess: async (id: string): Promise<ResearchProcessResponse | null> => {
    return mockProcesses.find((p) => p.id === id) ?? null;
  },

  /**
   * Get available research depth presets
   */
  getPresets: async (): Promise<ResearchPresetResponse[]> => {
    return mockPresets;
  },

  /**
   * Start a new research process
   */
  start: async (input: StartResearchInput): Promise<ResearchProcessResponse> => {
    const newProcess: ResearchProcessResponse = {
      id: `research-${Date.now()}`,
      name: input.name,
      question: input.question,
      context: input.context ?? null,
      scope: input.scope ?? null,
      constraints: input.constraints ?? [],
      agent_profile_id: input.agent_profile_id,
      depth_preset: input.depth_preset ?? null,
      max_iterations: input.custom_depth?.max_iterations ?? 20,
      timeout_hours: input.custom_depth?.timeout_hours ?? 4,
      checkpoint_interval: input.custom_depth?.checkpoint_interval ?? 5,
      target_bucket: input.target_bucket ?? "research-artifacts",
      status: "running",
      current_iteration: 0,
      progress_percentage: 0,
      error_message: null,
      created_at: new Date().toISOString(),
      started_at: new Date().toISOString(),
      completed_at: null,
    };
    return newProcess;
  },

  /**
   * Pause a running research process
   */
  pause: async (id: string): Promise<ResearchProcessResponse> => {
    const process = mockProcesses.find((p) => p.id === id);
    if (!process) {
      throw new Error(`Research process not found: ${id}`);
    }
    return { ...process, status: "paused" };
  },

  /**
   * Resume a paused research process
   */
  resume: async (id: string): Promise<ResearchProcessResponse> => {
    const process = mockProcesses.find((p) => p.id === id);
    if (!process) {
      throw new Error(`Research process not found: ${id}`);
    }
    return { ...process, status: "running" };
  },

  /**
   * Stop/cancel a research process
   */
  stop: async (id: string): Promise<ResearchProcessResponse> => {
    const process = mockProcesses.find((p) => p.id === id);
    if (!process) {
      throw new Error(`Research process not found: ${id}`);
    }
    return { ...process, status: "failed", error_message: "Cancelled by user" };
  },
} as const;
