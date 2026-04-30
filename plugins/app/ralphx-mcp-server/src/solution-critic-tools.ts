import { Tool } from "@modelcontextprotocol/sdk/types.js";

const CRITIQUE_TARGET_TYPE_SCHEMA = {
  type: "string",
  enum: [
    "plan_artifact",
    "artifact",
    "chat_message",
    "agent_run",
    "task",
    "task_execution",
    "review_report",
  ],
} as const;

export const SOLUTION_CRITIC_TOOLS: Tool[] = [
  {
    name: "compile_context",
    description:
      "Read-generation helper that collects deterministic ideation context for the selected critique target and persists a CompiledContext artifact. " +
      "For verifier-owned runs, omit session_id; the backend resolves the parent ideation session from the active verification child context.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description:
            "Optional ideation session ID. Verifier-owned calls should omit this.",
        },
        target_artifact_id: {
          type: "string",
          description:
            "Legacy shorthand for a plan artifact target. Stale plan artifact IDs are resolved to the latest version.",
        },
        target_type: {
          ...CRITIQUE_TARGET_TYPE_SCHEMA,
          description:
            "Optional typed target kind. Use with target_id for non-plan targets.",
        },
        target_id: {
          type: "string",
          description:
            "Optional typed target ID. Use with target_type for assistant messages, artifacts, task execution, or review reports.",
        },
        source_limits: {
          type: "object",
          properties: {
            chat_messages: { type: "integer", minimum: 0 },
            task_proposals: { type: "integer", minimum: 0 },
            related_artifacts: { type: "integer", minimum: 0 },
            agent_runs: { type: "integer", minimum: 0 },
          },
          additionalProperties: false,
          description:
            "Optional bounded source limits for context collection. Omit for backend defaults.",
        },
      },
      required: [],
    },
  },
  {
    name: "get_compiled_context",
    description:
      "Read a persisted CompiledContext artifact for the current ideation plan scope. For verifier-owned runs, omit session_id.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description:
            "Optional ideation session ID. Verifier-owned calls should omit this.",
        },
        artifact_id: {
          type: "string",
          description: "The CompiledContext artifact ID.",
        },
      },
      required: ["artifact_id"],
    },
  },
  {
    name: "critique_artifact",
    description:
      "Read-generation helper that critiques the selected target against a CompiledContext artifact, persists a SolutionCritique artifact, and returns backend-projected verification gaps. " +
      "Do not hand-derive gaps from the full critique payload; use the returned projected_gaps when a gap projection is needed.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description:
            "Optional ideation session ID. Verifier-owned calls should omit this.",
        },
        target_artifact_id: {
          type: "string",
          description:
            "Legacy shorthand for a plan artifact target. Stale plan artifact IDs are resolved to the latest version.",
        },
        target_type: {
          ...CRITIQUE_TARGET_TYPE_SCHEMA,
          description:
            "Optional typed target kind. Use with target_id for non-plan targets.",
        },
        target_id: {
          type: "string",
          description:
            "Optional typed target ID. Use with target_type for assistant messages, artifacts, task execution, or review reports.",
        },
        compiled_context_artifact_id: {
          type: "string",
          description: "The CompiledContext artifact ID to critique against.",
        },
      },
      required: ["compiled_context_artifact_id"],
    },
  },
  {
    name: "get_solution_critique",
    description:
      "Read a persisted SolutionCritique artifact plus its backend-projected verification gaps for the current ideation plan scope. For verifier-owned runs, omit session_id.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description:
            "Optional ideation session ID. Verifier-owned calls should omit this.",
        },
        artifact_id: {
          type: "string",
          description: "The SolutionCritique artifact ID.",
        },
      },
      required: ["artifact_id"],
    },
  },
];
