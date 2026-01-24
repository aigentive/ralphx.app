/**
 * Integration test: Methodology activation and deactivation
 *
 * Tests the complete methodology activation/deactivation flow:
 * - Activate BMAD methodology
 * - Verify workflow columns match BMAD definition
 * - Verify agent profiles loaded
 * - Deactivate methodology returns to default
 *
 * These tests verify the integration between:
 * - useMethodologies hooks (useMethodologies, useActiveMethodology, mutations)
 * - Methodology API wrappers (activate, deactivate)
 * - Methodology components (MethodologyBrowser, MethodologyConfig)
 * - Store updates (methodologyStore, workflowStore)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type {
  MethodologyResponse,
  MethodologyActivationResponse,
  MethodologyPhaseResponse,
} from "@/lib/api/methodologies";
import type { MethodologyExtension } from "@/types/methodology";
import { BMAD_METHODOLOGY, GSD_METHODOLOGY } from "@/types/methodology";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  useMethodologies,
  useActiveMethodology,
  useActivateMethodology,
  useDeactivateMethodology,
} from "@/hooks/useMethodologies";
import { MethodologyBrowser } from "@/components/methodologies/MethodologyBrowser";
import { MethodologyConfig } from "@/components/methodologies/MethodologyConfig";

const mockedInvoke = vi.mocked(invoke);

// ============================================================================
// API Response Factories (for hook tests - snake_case)
// ============================================================================

const createBmadPhaseResponse = (
  id: string,
  name: string,
  order: number,
  agentProfiles: string[]
): MethodologyPhaseResponse => ({
  id,
  name,
  order,
  description: `Phase for ${name}`,
  agent_profiles: agentProfiles,
  column_ids: [],
});

const createBmadResponse = (
  overrides: Partial<MethodologyResponse> = {}
): MethodologyResponse => ({
  id: "bmad-method",
  name: "BMAD Method",
  description: "Breakthrough Method for Agile AI-Driven Development",
  agent_profiles: [
    "bmad-analyst",
    "bmad-pm",
    "bmad-architect",
    "bmad-ux",
    "bmad-developer",
    "bmad-scrum-master",
    "bmad-tea",
    "bmad-tech-writer",
  ],
  skills: [
    "skills/prd-creation",
    "skills/architecture-design",
    "skills/ux-review",
    "skills/story-writing",
  ],
  workflow_id: "bmad-workflow",
  workflow_name: "BMAD Method",
  phases: [
    createBmadPhaseResponse("analysis", "Analysis", 0, ["bmad-analyst"]),
    createBmadPhaseResponse("planning", "Planning", 1, ["bmad-pm", "bmad-ux"]),
    createBmadPhaseResponse("solutioning", "Solutioning", 2, ["bmad-architect"]),
    createBmadPhaseResponse("implementation", "Implementation", 3, ["bmad-developer"]),
  ],
  templates: [
    { artifact_type: "prd", template_path: "templates/bmad/prd.md", name: "PRD Template", description: null },
  ],
  is_active: false,
  phase_count: 4,
  agent_count: 8,
  created_at: "2026-01-24T10:00:00Z",
  ...overrides,
});

const createGsdPhaseResponse = (
  id: string,
  name: string,
  order: number,
  agentProfiles: string[]
): MethodologyPhaseResponse => ({
  id,
  name,
  order,
  description: `GSD phase for ${name}`,
  agent_profiles: agentProfiles,
  column_ids: [],
});

const createGsdResponse = (
  overrides: Partial<MethodologyResponse> = {}
): MethodologyResponse => ({
  id: "gsd-method",
  name: "GSD Method",
  description: "Get Shit Done - Spec-driven development",
  agent_profiles: [
    "gsd-project-researcher",
    "gsd-phase-researcher",
    "gsd-planner",
    "gsd-plan-checker",
    "gsd-executor",
    "gsd-verifier",
    "gsd-debugger",
    "gsd-orchestrator",
    "gsd-monitor",
    "gsd-qa",
    "gsd-docs",
  ],
  skills: [
    "skills/project-analysis",
    "skills/phase-research",
    "skills/wave-planning",
    "skills/checkpoint-handling",
    "skills/verification",
  ],
  workflow_id: "gsd-workflow",
  workflow_name: "GSD Method",
  phases: [
    createGsdPhaseResponse("initialize", "Initialize", 0, ["gsd-project-researcher"]),
    createGsdPhaseResponse("plan", "Plan", 1, ["gsd-planner", "gsd-plan-checker"]),
    createGsdPhaseResponse("execute", "Execute", 2, ["gsd-executor"]),
    createGsdPhaseResponse("verify", "Verify", 3, ["gsd-verifier"]),
  ],
  templates: [],
  is_active: false,
  phase_count: 4,
  agent_count: 11,
  created_at: "2026-01-24T10:00:00Z",
  ...overrides,
});

const createBmadActivationResponse = (): MethodologyActivationResponse => ({
  methodology: createBmadResponse({ is_active: true }),
  workflow: {
    id: "bmad-workflow",
    name: "BMAD Method",
    description: "BMAD workflow",
    column_count: 10,
  },
  agent_profiles: [
    "bmad-analyst",
    "bmad-pm",
    "bmad-architect",
    "bmad-ux",
    "bmad-developer",
    "bmad-scrum-master",
    "bmad-tea",
    "bmad-tech-writer",
  ],
  skills: [
    "skills/prd-creation",
    "skills/architecture-design",
    "skills/ux-review",
    "skills/story-writing",
  ],
  previous_methodology_id: null,
});

// ============================================================================
// Domain Type Factories (for component tests - camelCase)
// ============================================================================

const createBmadMethodologyExtension = (
  overrides: Partial<MethodologyExtension> = {}
): MethodologyExtension => ({
  ...BMAD_METHODOLOGY,
  createdAt: "2026-01-24T10:00:00Z",
  ...overrides,
});

const createGsdMethodologyExtension = (
  overrides: Partial<MethodologyExtension> = {}
): MethodologyExtension => ({
  ...GSD_METHODOLOGY,
  createdAt: "2026-01-24T10:00:00Z",
  ...overrides,
});

// ============================================================================
// Test Wrapper
// ============================================================================

const createTestQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
        staleTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  });

interface WrapperProps {
  children: React.ReactNode;
}

const createWrapper = () => {
  const queryClient = createTestQueryClient();
  return function Wrapper({ children }: WrapperProps) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };
};

// ============================================================================
// Hook Tests (use API response types)
// ============================================================================

describe("Methodology Hooks Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("fetches all methodologies", async () => {
    const methodologies = [createBmadResponse(), createGsdResponse()];
    mockedInvoke.mockResolvedValueOnce(methodologies);

    const { result } = await import("@testing-library/react").then((m) =>
      m.renderHook(() => useMethodologies(), { wrapper: createWrapper() })
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toHaveLength(2);
    expect(mockedInvoke).toHaveBeenCalledWith("get_methodologies", {});
  });

  it("fetches active methodology when one exists", async () => {
    const activeMethodology = createBmadResponse({ is_active: true });
    mockedInvoke.mockResolvedValueOnce(activeMethodology);

    const { result } = await import("@testing-library/react").then((m) =>
      m.renderHook(() => useActiveMethodology(), { wrapper: createWrapper() })
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data?.id).toBe("bmad-method");
    expect(result.current.data?.is_active).toBe(true);
    expect(mockedInvoke).toHaveBeenCalledWith("get_active_methodology", {});
  });

  it("returns null when no active methodology", async () => {
    mockedInvoke.mockResolvedValueOnce(null);

    const { result } = await import("@testing-library/react").then((m) =>
      m.renderHook(() => useActiveMethodology(), { wrapper: createWrapper() })
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toBeNull();
  });

  it("activates BMAD methodology", async () => {
    const activationResponse = createBmadActivationResponse();
    mockedInvoke.mockResolvedValueOnce(activationResponse);

    const { result } = await import("@testing-library/react").then((m) =>
      m.renderHook(() => useActivateMethodology(), { wrapper: createWrapper() })
    );

    const response = await result.current.mutateAsync("bmad-method");

    expect(mockedInvoke).toHaveBeenCalledWith("activate_methodology", {
      id: "bmad-method",
    });
    expect(response.methodology.is_active).toBe(true);
    expect(response.workflow.column_count).toBe(10);
    expect(response.agent_profiles).toHaveLength(8);
  });

  it("deactivates methodology", async () => {
    const deactivatedResponse = createBmadResponse({ is_active: false });
    mockedInvoke.mockResolvedValueOnce(deactivatedResponse);

    const { result } = await import("@testing-library/react").then((m) =>
      m.renderHook(() => useDeactivateMethodology(), { wrapper: createWrapper() })
    );

    const response = await result.current.mutateAsync("bmad-method");

    expect(mockedInvoke).toHaveBeenCalledWith("deactivate_methodology", {
      id: "bmad-method",
    });
    expect(response.is_active).toBe(false);
  });

  it("activation response contains BMAD workflow column count", async () => {
    const activationResponse = createBmadActivationResponse();
    mockedInvoke.mockResolvedValueOnce(activationResponse);

    const { result } = await import("@testing-library/react").then((m) =>
      m.renderHook(() => useActivateMethodology(), { wrapper: createWrapper() })
    );

    const response = await result.current.mutateAsync("bmad-method");

    // Verify BMAD workflow has 10 columns
    expect(response.workflow.column_count).toBe(10);
    expect(response.workflow.name).toBe("BMAD Method");
  });

  it("activation response contains BMAD agent profiles", async () => {
    const activationResponse = createBmadActivationResponse();
    mockedInvoke.mockResolvedValueOnce(activationResponse);

    const { result } = await import("@testing-library/react").then((m) =>
      m.renderHook(() => useActivateMethodology(), { wrapper: createWrapper() })
    );

    const response = await result.current.mutateAsync("bmad-method");

    // Verify 8 agent profiles
    expect(response.agent_profiles).toHaveLength(8);
    expect(response.agent_profiles).toContain("bmad-analyst");
    expect(response.agent_profiles).toContain("bmad-pm");
    expect(response.agent_profiles).toContain("bmad-architect");
    expect(response.agent_profiles).toContain("bmad-developer");
  });
});

// ============================================================================
// Component Integration Tests (use domain types)
// ============================================================================

describe("MethodologyBrowser Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders both BMAD and GSD methodologies", () => {
    const methodologies = [
      createBmadMethodologyExtension(),
      createGsdMethodologyExtension(),
    ];

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyBrowser
          methodologies={methodologies}
          onSelect={vi.fn()}
          onActivate={vi.fn()}
          onDeactivate={vi.fn()}
        />
      </QueryClientProvider>
    );

    expect(screen.getByText("BMAD Method")).toBeInTheDocument();
    expect(screen.getByText("GSD (Get Shit Done)")).toBeInTheDocument();
  });

  it("shows active badge for active methodology", () => {
    const methodologies = [
      createBmadMethodologyExtension({ isActive: true }),
      createGsdMethodologyExtension(),
    ];

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyBrowser
          methodologies={methodologies}
          onSelect={vi.fn()}
          onActivate={vi.fn()}
          onDeactivate={vi.fn()}
        />
      </QueryClientProvider>
    );

    expect(screen.getByTestId("active-badge")).toBeInTheDocument();
  });

  it("shows phase and agent counts", () => {
    const methodologies = [createBmadMethodologyExtension()];

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyBrowser
          methodologies={methodologies}
          onSelect={vi.fn()}
          onActivate={vi.fn()}
          onDeactivate={vi.fn()}
        />
      </QueryClientProvider>
    );

    // BMAD has 4 phases and 8 agents
    expect(screen.getByTestId("phase-count")).toHaveTextContent("4 phases");
    expect(screen.getByTestId("agent-count")).toHaveTextContent("8 agents");
  });

  it("calls onActivate when activate button clicked", async () => {
    const user = userEvent.setup();
    const onActivate = vi.fn();
    const methodologies = [createBmadMethodologyExtension()];

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyBrowser
          methodologies={methodologies}
          onSelect={vi.fn()}
          onActivate={onActivate}
          onDeactivate={vi.fn()}
        />
      </QueryClientProvider>
    );

    await user.click(screen.getByTestId("activate-button"));

    expect(onActivate).toHaveBeenCalledWith("bmad-method");
  });

  it("calls onDeactivate when deactivate button clicked", async () => {
    const user = userEvent.setup();
    const onDeactivate = vi.fn();
    const methodologies = [createBmadMethodologyExtension({ isActive: true })];

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyBrowser
          methodologies={methodologies}
          onSelect={vi.fn()}
          onActivate={vi.fn()}
          onDeactivate={onDeactivate}
        />
      </QueryClientProvider>
    );

    await user.click(screen.getByTestId("deactivate-button"));

    expect(onDeactivate).toHaveBeenCalledWith("bmad-method");
  });
});

describe("MethodologyConfig Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("displays methodology name and description", () => {
    const methodology = createBmadMethodologyExtension({ isActive: true });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyConfig methodology={methodology} />
      </QueryClientProvider>
    );

    expect(screen.getByTestId("methodology-name")).toHaveTextContent("BMAD Method");
    expect(screen.getByTestId("methodology-description")).toHaveTextContent(
      /Breakthrough Method for Agile AI-Driven Development/i
    );
  });

  it("displays workflow columns with color chips", () => {
    const methodology = createBmadMethodologyExtension({ isActive: true });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyConfig methodology={methodology} />
      </QueryClientProvider>
    );

    // BMAD has 10 columns
    const columns = screen.getAllByTestId("workflow-column");
    expect(columns.length).toBe(10);

    // Verify some specific columns
    expect(screen.getByText("Brainstorm")).toBeInTheDocument();
    expect(screen.getByText("Research")).toBeInTheDocument();
    expect(screen.getByText("Done")).toBeInTheDocument();
  });

  it("displays phase progression", () => {
    const methodology = createBmadMethodologyExtension({ isActive: true });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyConfig methodology={methodology} />
      </QueryClientProvider>
    );

    // BMAD has 4 phases: Analysis, Planning, Solutioning, Implementation
    const phases = screen.getAllByTestId("phase-item");
    expect(phases.length).toBe(4);

    expect(screen.getByText("Analysis")).toBeInTheDocument();
    expect(screen.getByText("Planning")).toBeInTheDocument();
    expect(screen.getByText("Solutioning")).toBeInTheDocument();
    expect(screen.getByText("Implementation")).toBeInTheDocument();
  });

  it("displays agent profiles list", () => {
    const methodology = createBmadMethodologyExtension({ isActive: true });

    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyConfig methodology={methodology} />
      </QueryClientProvider>
    );

    // Check for some agent profiles
    const agents = screen.getAllByTestId("agent-item");
    expect(agents.length).toBe(8);

    expect(screen.getByText("bmad-analyst")).toBeInTheDocument();
    expect(screen.getByText("bmad-pm")).toBeInTheDocument();
  });

  it("shows empty state when no methodology", () => {
    render(
      <QueryClientProvider client={createTestQueryClient()}>
        <MethodologyConfig methodology={null} />
      </QueryClientProvider>
    );

    expect(screen.getByText(/no active methodology/i)).toBeInTheDocument();
  });
});

// ============================================================================
// Full Lifecycle Integration Tests (use API response types)
// ============================================================================

describe("Methodology Activation Lifecycle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("completes full activate -> verify -> deactivate cycle", async () => {
    // 1. Activate BMAD
    const activationResponse = createBmadActivationResponse();
    mockedInvoke.mockResolvedValueOnce(activationResponse);

    const { result: activateResult } = await import(
      "@testing-library/react"
    ).then((m) =>
      m.renderHook(() => useActivateMethodology(), { wrapper: createWrapper() })
    );

    const activated = await activateResult.current.mutateAsync("bmad-method");
    expect(activated.methodology.is_active).toBe(true);
    expect(activated.methodology.name).toBe("BMAD Method");

    // 2. Verify workflow column count matches BMAD definition (10 columns)
    expect(activated.workflow.column_count).toBe(10);

    // 3. Verify agent profiles loaded (8 agents)
    expect(activated.agent_profiles).toHaveLength(8);

    // 4. Deactivate methodology
    const deactivatedResponse = createBmadResponse({ is_active: false });
    mockedInvoke.mockResolvedValueOnce(deactivatedResponse);

    const { result: deactivateResult } = await import(
      "@testing-library/react"
    ).then((m) =>
      m.renderHook(() => useDeactivateMethodology(), { wrapper: createWrapper() })
    );

    const deactivated = await deactivateResult.current.mutateAsync("bmad-method");
    expect(deactivated.is_active).toBe(false);
  });

  it("switches from BMAD to GSD methodology", async () => {
    // Activate BMAD first
    const bmadActivation = createBmadActivationResponse();
    mockedInvoke.mockResolvedValueOnce(bmadActivation);

    const { result: activateBmad } = await import("@testing-library/react").then(
      (m) =>
        m.renderHook(() => useActivateMethodology(), { wrapper: createWrapper() })
    );

    await activateBmad.current.mutateAsync("bmad-method");

    // Deactivate BMAD
    const bmadDeactivated = createBmadResponse({ is_active: false });
    mockedInvoke.mockResolvedValueOnce(bmadDeactivated);

    const { result: deactivateBmad } = await import(
      "@testing-library/react"
    ).then((m) =>
      m.renderHook(() => useDeactivateMethodology(), { wrapper: createWrapper() })
    );

    await deactivateBmad.current.mutateAsync("bmad-method");

    // Activate GSD
    const gsdActivation: MethodologyActivationResponse = {
      methodology: createGsdResponse({ is_active: true }),
      workflow: {
        id: "gsd-workflow",
        name: "GSD Method",
        description: "GSD workflow",
        column_count: 11,
      },
      agent_profiles: [
        "gsd-project-researcher",
        "gsd-phase-researcher",
        "gsd-planner",
        "gsd-plan-checker",
        "gsd-executor",
        "gsd-verifier",
        "gsd-debugger",
        "gsd-orchestrator",
        "gsd-monitor",
        "gsd-qa",
        "gsd-docs",
      ],
      skills: ["skills/project-analysis"],
      previous_methodology_id: "bmad-method",
    };
    mockedInvoke.mockResolvedValueOnce(gsdActivation);

    const { result: activateGsd } = await import("@testing-library/react").then(
      (m) =>
        m.renderHook(() => useActivateMethodology(), { wrapper: createWrapper() })
    );

    const gsd = await activateGsd.current.mutateAsync("gsd-method");

    // Verify GSD is now active with 11 columns and 11 agents
    expect(gsd.methodology.name).toBe("GSD Method");
    expect(gsd.workflow.column_count).toBe(11);
    expect(gsd.agent_profiles).toHaveLength(11);
    expect(gsd.previous_methodology_id).toBe("bmad-method");
  });

  it("verifies GSD methodology has correct phase structure", async () => {
    const gsdActivation: MethodologyActivationResponse = {
      methodology: createGsdResponse({ is_active: true }),
      workflow: {
        id: "gsd-workflow",
        name: "GSD Method",
        description: "GSD workflow",
        column_count: 11,
      },
      agent_profiles: [],
      skills: [],
      previous_methodology_id: null,
    };
    mockedInvoke.mockResolvedValueOnce(gsdActivation);

    const { result } = await import("@testing-library/react").then((m) =>
      m.renderHook(() => useActivateMethodology(), { wrapper: createWrapper() })
    );

    const gsd = await result.current.mutateAsync("gsd-method");

    // GSD has 4 phases
    expect(gsd.methodology.phases).toHaveLength(4);

    // Verify phase structure
    const phases = gsd.methodology.phases;
    expect(phases[0].id).toBe("initialize");
    expect(phases[1].id).toBe("plan");
    expect(phases[2].id).toBe("execute");
    expect(phases[3].id).toBe("verify");

    // Each phase has description
    phases.forEach((phase) => {
      expect(phase.description).toBeTruthy();
    });
  });
});
