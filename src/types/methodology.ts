/**
 * Methodology type definitions for the extensibility system
 *
 * A methodology is a combination of Workflow + Agents + Artifacts. When a user
 * activates a methodology, the Kanban columns change to reflect that methodology's
 * workflow while still mapping to internal statuses for consistent side effects.
 *
 * Supports BMAD (Breakthrough Method for Agile AI-Driven Development) and
 * GSD (Get Shit Done) methodologies as built-ins.
 */

import { z } from "zod";
import { WorkflowSchemaZ } from "./workflow";

// ============================================
// Methodology Status
// ============================================

/**
 * Status of a methodology in a project
 * - available: Available but not active
 * - active: Currently active for the project
 * - disabled: Temporarily disabled
 */
export const MethodologyStatusSchema = z.enum([
  "available",
  "active",
  "disabled",
]);

export type MethodologyStatus = z.infer<typeof MethodologyStatusSchema>;

/**
 * All methodology status values as a readonly array
 */
export const METHODOLOGY_STATUS_VALUES = MethodologyStatusSchema.options;

/**
 * Check if a methodology status is active
 */
export function isMethodologyActive(status: MethodologyStatus): boolean {
  return status === "active";
}

/**
 * Check if a methodology status is available
 */
export function isMethodologyAvailable(status: MethodologyStatus): boolean {
  return status === "available";
}

/**
 * Check if a methodology status is disabled
 */
export function isMethodologyDisabled(status: MethodologyStatus): boolean {
  return status === "disabled";
}

// ============================================
// Methodology Phase
// ============================================

/**
 * A phase or stage in a methodology
 */
export const MethodologyPhaseSchema = z.object({
  /** Unique identifier within the methodology */
  id: z.string(),
  /** Display name for the phase */
  name: z.string(),
  /** Order in the phase sequence (0-based) */
  order: z.number().int().nonnegative(),
  /** Agent profile IDs that work in this phase */
  agentProfiles: z.array(z.string()).default([]),
  /** Description of the phase */
  description: z.string().optional(),
  /** Column IDs in the workflow that belong to this phase */
  columnIds: z.array(z.string()).default([]),
});

export type MethodologyPhase = z.infer<typeof MethodologyPhaseSchema>;

// ============================================
// Methodology Template
// ============================================

/**
 * A document template for a methodology
 */
export const MethodologyTemplateSchema = z.object({
  /** The artifact type this template produces */
  artifactType: z.string(),
  /** Path to the template file (relative to methodology directory) */
  templatePath: z.string(),
  /** Display name for the template */
  name: z.string().optional(),
  /** Description of when to use this template */
  description: z.string().optional(),
});

export type MethodologyTemplate = z.infer<typeof MethodologyTemplateSchema>;

// ============================================
// Methodology Extension
// ============================================

/**
 * A methodology extension - a configuration package that brings workflow,
 * agents, skills, phases, and templates
 */
export const MethodologyExtensionSchema = z.object({
  /** Unique identifier */
  id: z.string(),
  /** Display name for the methodology */
  name: z.string(),
  /** Description of the methodology */
  description: z.string().optional(),
  /** Agent profiles this methodology provides (profile IDs) */
  agentProfiles: z.array(z.string()).default([]),
  /** Skills bundled with methodology (paths to skill directories) */
  skills: z.array(z.string()).default([]),
  /** Custom workflow for this methodology */
  workflow: WorkflowSchemaZ,
  /** Phase/stage definitions */
  phases: z.array(MethodologyPhaseSchema).default([]),
  /** Document templates */
  templates: z.array(MethodologyTemplateSchema).default([]),
  /** Hooks configuration (stored as JSON for flexibility) */
  hooksConfig: z.record(z.string(), z.unknown()).optional(),
  /** Whether this methodology is currently active */
  isActive: z.boolean().default(false),
  /** When the methodology was created (ISO 8601 string) */
  createdAt: z.string(),
});

export type MethodologyExtension = z.infer<typeof MethodologyExtensionSchema>;

/**
 * Input for creating a new methodology extension
 */
export const CreateMethodologyExtensionInputSchema = z.object({
  /** Display name for the methodology */
  name: z.string(),
  /** Description of the methodology */
  description: z.string().optional(),
  /** Agent profiles this methodology provides (profile IDs) */
  agentProfiles: z.array(z.string()).optional(),
  /** Skills bundled with methodology (paths to skill directories) */
  skills: z.array(z.string()).optional(),
  /** Custom workflow for this methodology */
  workflow: WorkflowSchemaZ,
  /** Phase/stage definitions */
  phases: z.array(MethodologyPhaseSchema).optional(),
  /** Document templates */
  templates: z.array(MethodologyTemplateSchema).optional(),
  /** Hooks configuration */
  hooksConfig: z.record(z.string(), z.unknown()).optional(),
});

export type CreateMethodologyExtensionInput = z.infer<
  typeof CreateMethodologyExtensionInputSchema
>;

// ============================================
// Built-in Methodologies
// ============================================

/**
 * BMAD (Breakthrough Method for Agile AI-Driven Development) methodology
 *
 * Uses:
 * - 8 agents: Analyst, PM, Architect, UX Designer, Developer, Scrum Master, TEA, Tech Writer
 * - 4 phases: Analysis → Planning → Solutioning → Implementation
 * - Document-centric: PRD, Architecture Doc, UX Design, Stories/Epics
 */
export const BMAD_METHODOLOGY: MethodologyExtension = {
  id: "bmad-method",
  name: "BMAD Method",
  description:
    "Breakthrough Method for Agile AI-Driven Development - a document-centric " +
    "methodology with 4 phases: Analysis, Planning, Solutioning, Implementation",
  agentProfiles: [
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
  workflow: {
    id: "bmad-method",
    name: "BMAD Method",
    description: "Breakthrough Method for Agile AI-Driven Development",
    columns: [
      // Phase 1: Analysis
      {
        id: "brainstorm",
        name: "Brainstorm",
        mapsTo: "backlog",
        behavior: { agentProfile: "bmad-analyst" },
      },
      {
        id: "research",
        name: "Research",
        mapsTo: "executing",
        behavior: { agentProfile: "bmad-analyst" },
      },
      // Phase 2: Planning
      {
        id: "prd-draft",
        name: "PRD Draft",
        mapsTo: "executing",
        behavior: { agentProfile: "bmad-pm" },
      },
      {
        id: "prd-review",
        name: "PRD Review",
        mapsTo: "pending_review",
        behavior: { agentProfile: "bmad-pm" },
      },
      {
        id: "ux-design",
        name: "UX Design",
        mapsTo: "executing",
        behavior: { agentProfile: "bmad-ux" },
      },
      // Phase 3: Solutioning
      {
        id: "architecture",
        name: "Architecture",
        mapsTo: "executing",
        behavior: { agentProfile: "bmad-architect" },
      },
      {
        id: "stories",
        name: "Stories",
        mapsTo: "ready",
        behavior: { agentProfile: "bmad-pm" },
      },
      // Phase 4: Implementation
      {
        id: "sprint",
        name: "Sprint",
        mapsTo: "executing",
        behavior: { agentProfile: "bmad-developer" },
      },
      {
        id: "code-review",
        name: "Code Review",
        mapsTo: "pending_review",
        behavior: { agentProfile: "bmad-developer" },
      },
      { id: "done", name: "Done", mapsTo: "approved" },
    ],
    isDefault: false,
  },
  phases: [
    {
      id: "analysis",
      name: "Analysis",
      order: 0,
      description: "Analyze requirements and research domain",
      agentProfiles: ["bmad-analyst"],
      columnIds: ["brainstorm", "research"],
    },
    {
      id: "planning",
      name: "Planning",
      order: 1,
      description: "Create PRD and UX design documents",
      agentProfiles: ["bmad-pm", "bmad-ux"],
      columnIds: ["prd-draft", "prd-review", "ux-design"],
    },
    {
      id: "solutioning",
      name: "Solutioning",
      order: 2,
      description: "Design architecture and create user stories",
      agentProfiles: ["bmad-architect", "bmad-pm"],
      columnIds: ["architecture", "stories"],
    },
    {
      id: "implementation",
      name: "Implementation",
      order: 3,
      description: "Execute sprints and code review",
      agentProfiles: ["bmad-developer"],
      columnIds: ["sprint", "code-review", "done"],
    },
  ],
  templates: [
    {
      artifactType: "prd",
      templatePath: "templates/bmad/prd.md",
      name: "PRD Template",
      description: "Product Requirements Document for BMAD",
    },
    {
      artifactType: "design_doc",
      templatePath: "templates/bmad/architecture.md",
      name: "Architecture Document",
      description: "System architecture design document",
    },
    {
      artifactType: "specification",
      templatePath: "templates/bmad/ux-design.md",
      name: "UX Design Spec",
      description: "User experience design specification",
    },
  ],
  hooksConfig: {
    phase_gates: {
      analysis: ["requirements_documented"],
      planning: ["prd_approved", "ux_approved"],
      solutioning: ["architecture_approved"],
    },
    validation_checklists: {
      prd: ["clear_objectives", "success_metrics", "scope_defined"],
      architecture: ["scalability", "security", "maintainability"],
    },
  },
  isActive: false,
  createdAt: new Date().toISOString(),
};

/**
 * GSD (Get Shit Done) methodology
 *
 * Uses:
 * - 11 agents: project-researcher, phase-researcher, planner, executor, verifier, debugger, etc.
 * - Wave-based parallelization: Plans grouped into waves for parallel execution
 * - Checkpoint protocol: human-verify, decision, human-action types
 * - Goal-backward verification: must-haves derived from phase goals
 */
export const GSD_METHODOLOGY: MethodologyExtension = {
  id: "gsd-method",
  name: "GSD (Get Shit Done)",
  description:
    "Spec-driven development with wave-based parallelization. Features checkpoint " +
    "protocols (human-verify, decision, human-action) and goal-backward verification " +
    "with must-haves derived from phase goals.",
  agentProfiles: [
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
  workflow: {
    id: "gsd-method",
    name: "GSD (Get Shit Done)",
    description: "Spec-driven development with wave-based parallelization",
    columns: [
      // Initialize
      {
        id: "initialize",
        name: "Initialize",
        mapsTo: "backlog",
        behavior: { agentProfile: "gsd-project-researcher" },
      },
      // Discuss (optional)
      {
        id: "discuss",
        name: "Discuss",
        mapsTo: "blocked",
        behavior: { agentProfile: "gsd-orchestrator" },
      },
      // Plan
      {
        id: "research",
        name: "Research",
        mapsTo: "executing",
        behavior: { agentProfile: "gsd-phase-researcher" },
      },
      {
        id: "planning",
        name: "Planning",
        mapsTo: "executing",
        behavior: { agentProfile: "gsd-planner" },
      },
      {
        id: "plan-check",
        name: "Plan Check",
        mapsTo: "pending_review",
        behavior: { agentProfile: "gsd-plan-checker" },
      },
      // Execute (wave-based)
      { id: "queued", name: "Queued", mapsTo: "ready" },
      {
        id: "executing",
        name: "Executing",
        mapsTo: "executing",
        behavior: { agentProfile: "gsd-executor" },
      },
      { id: "checkpoint", name: "Checkpoint", mapsTo: "blocked" },
      // Verify
      {
        id: "verifying",
        name: "Verifying",
        mapsTo: "pending_review",
        behavior: { agentProfile: "gsd-verifier" },
      },
      {
        id: "debugging",
        name: "Debugging",
        mapsTo: "revision_needed",
        behavior: { agentProfile: "gsd-debugger" },
      },
      // Complete
      { id: "done", name: "Done", mapsTo: "approved" },
    ],
    isDefault: false,
  },
  phases: [
    {
      id: "initialize",
      name: "Initialize",
      order: 0,
      description: "Project research and initialization",
      agentProfiles: ["gsd-project-researcher"],
      columnIds: ["initialize"],
    },
    {
      id: "plan",
      name: "Plan",
      order: 1,
      description: "Research, planning, and plan verification",
      agentProfiles: ["gsd-phase-researcher", "gsd-planner", "gsd-plan-checker"],
      columnIds: ["discuss", "research", "planning", "plan-check"],
    },
    {
      id: "execute",
      name: "Execute",
      order: 2,
      description: "Wave-based parallel execution with checkpoints",
      agentProfiles: ["gsd-executor"],
      columnIds: ["queued", "executing", "checkpoint"],
    },
    {
      id: "verify",
      name: "Verify",
      order: 3,
      description: "Verification and debugging",
      agentProfiles: ["gsd-verifier", "gsd-debugger"],
      columnIds: ["verifying", "debugging", "done"],
    },
  ],
  templates: [
    {
      artifactType: "specification",
      templatePath: "templates/gsd/phase-spec.md",
      name: "Phase Specification",
      description: "Specification for a GSD phase",
    },
    {
      artifactType: "task_spec",
      templatePath: "templates/gsd/plan-spec.md",
      name: "Plan Specification",
      description: "Detailed plan specification with must-haves",
    },
    {
      artifactType: "context",
      templatePath: "templates/gsd/state.md",
      name: "STATE.md Template",
      description: "State tracking document for GSD execution",
    },
  ],
  hooksConfig: {
    checkpoint_types: ["auto", "human-verify", "decision", "human-action"],
    wave_execution: {
      max_parallel: 5,
      wave_completion_required: true,
    },
    verification: {
      must_haves_required: true,
      goal_backward_check: true,
    },
  },
  isActive: false,
  createdAt: new Date().toISOString(),
};

/**
 * All built-in methodologies
 */
export const BUILTIN_METHODOLOGIES: readonly MethodologyExtension[] = [
  BMAD_METHODOLOGY,
  GSD_METHODOLOGY,
] as const;

/**
 * Get a built-in methodology by ID
 */
export function getBuiltinMethodology(
  id: string
): MethodologyExtension | undefined {
  return BUILTIN_METHODOLOGIES.find((m) => m.id === id);
}

// ============================================
// Parsing Helpers
// ============================================

/**
 * Parse and validate a MethodologyExtension
 */
export function parseMethodologyExtension(data: unknown): MethodologyExtension {
  return MethodologyExtensionSchema.parse(data);
}

/**
 * Safely parse a MethodologyExtension, returning null on failure
 */
export function safeParseMethodologyExtension(
  data: unknown
): MethodologyExtension | null {
  const result = MethodologyExtensionSchema.safeParse(data);
  return result.success ? result.data : null;
}

/**
 * Parse and validate a MethodologyPhase
 */
export function parseMethodologyPhase(data: unknown): MethodologyPhase {
  return MethodologyPhaseSchema.parse(data);
}

/**
 * Safely parse a MethodologyPhase, returning null on failure
 */
export function safeParseMethodologyPhase(
  data: unknown
): MethodologyPhase | null {
  const result = MethodologyPhaseSchema.safeParse(data);
  return result.success ? result.data : null;
}
