import { Tool } from "@modelcontextprotocol/sdk/types.js";

export const SOLUTION_CRITIC_TOOLS: Tool[] = [
  {
    name: "compile_context",
    description:
      "Read-generation helper that collects deterministic ideation context for the selected plan artifact and persists a CompiledContext artifact. " +
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
            "The plan artifact ID to compile context for. Stale plan artifact IDs are resolved to the latest version.",
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
      required: ["target_artifact_id"],
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
      "Read-generation helper that critiques the selected plan artifact against a CompiledContext artifact, persists a SolutionCritique artifact, and returns backend-projected verification gaps. " +
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
            "The plan artifact ID to critique. Stale plan artifact IDs are resolved to the latest version.",
        },
        compiled_context_artifact_id: {
          type: "string",
          description: "The CompiledContext artifact ID to critique against.",
        },
      },
      required: ["target_artifact_id", "compiled_context_artifact_id"],
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
