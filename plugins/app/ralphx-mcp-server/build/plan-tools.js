/**
 * MCP tool definitions for plan artifact management
 * Used by ralphx-ideation agent to create and manage implementation plans
 */
/**
 * Plan artifact tools for ralphx-ideation agent
 * All tools are proxies that forward to Tauri backend via HTTP
 */
export const PLAN_TOOLS = [
    {
        name: "create_plan_artifact",
        description: "Create a new implementation plan artifact linked to the ideation session. Use this when the user describes a complex feature that needs architectural planning before breaking into tasks. The plan is stored as a Specification artifact and can be referenced by task proposals. " +
            "For child sessions that inherited a parent's plan: calling this creates a completely independent plan for the child session — it does NOT modify or copy from the parent's plan.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID (provided in context)",
                },
                title: {
                    type: "string",
                    description: "Plan title (e.g., 'Real-time Collaboration Implementation Plan')",
                },
                content: {
                    type: "string",
                    description: "Plan content in markdown format. Should include architecture decisions, data flow, key implementation details, and considerations.",
                },
            },
            required: ["session_id", "title", "content"],
        },
    },
    {
        name: "update_plan_artifact",
        description: "Update an existing implementation plan's content. Creates a NEW version with a new artifact ID (immutable version chain). Stale artifact IDs are auto-resolved: you can pass any previous version's ID and it will resolve to the latest before updating. Linked proposals are automatically re-linked to the new version (plan_version_at_creation is preserved). The response includes `previous_artifact_id` and `session_id` for reference. You do NOT need to call get_session_plan between updates to refresh the ID.",
        inputSchema: {
            type: "object",
            properties: {
                artifact_id: {
                    type: "string",
                    description: "The artifact ID of the plan to update. Can be any version ID — stale IDs are auto-resolved to the latest version.",
                },
                content: {
                    type: "string",
                    description: "Updated plan content in markdown format. This will create a new version of the artifact with a new ID.",
                },
                caller_session_id: {
                    type: "string",
                    description: "The session ID of the caller. Required when calling from a verification child session to bypass the write lock on the plan artifact.",
                },
            },
            required: ["artifact_id", "content"],
        },
    },
    {
        name: "link_proposals_to_plan",
        description: "Link multiple task proposals to an implementation plan. Use after creating proposals to establish the connection between the plan and its derived tasks. Stale artifact IDs are auto-resolved: you can pass any previous version's ID and it will resolve to the latest before linking. This enables traceability and allows the system to suggest updates when the plan changes.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_ids: {
                    type: "array",
                    items: { type: "string" },
                    description: "Array of proposal IDs to link to the plan",
                },
                artifact_id: {
                    type: "string",
                    description: "The plan artifact ID to link proposals to. Can be any version ID — stale IDs are auto-resolved to the latest version.",
                },
            },
            required: ["proposal_ids", "artifact_id"],
        },
    },
    {
        name: "get_session_plan",
        description: "Get the implementation plan artifact for the current ideation session, if one exists. Use to check if a plan has already been created before suggesting a new one. " +
            "Response includes an `is_inherited` boolean: if true, the plan was inherited from a parent session and is read-only — call create_plan_artifact to create an independent plan for this session.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
            },
            required: ["session_id"],
        },
    },
    {
        name: "report_verification_round",
        description: "Verifier-friendly helper for reporting an in-progress verification round on the PARENT ideation session. " +
            "The parent session remains canonical; if a verification child session_id is passed, the backend remaps it automatically. " +
            "This is the simpler alias for update_plan_verification with status fixed to reviewing and in_progress fixed to true. " +
            "Use this after each round once the merged gap list is ready. The response is authoritative for next-step control flow: it returns the backend verification state after convergence checks, so the verifier should use returned status/in_progress/convergence_reason instead of re-implementing zero-blocking, jaccard, or max-round rules in the prompt. If generation is stale, call get_plan_verification again instead of guessing.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "parent-session-id",
                    round: 1,
                    gaps: [],
                    generation: 3,
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "Optional PARENT ideation session ID being verified. In verifier child context the backend resolves the canonical parent automatically.",
                },
                round: {
                    type: "integer",
                    description: "Current round number (1-based).",
                },
                gaps: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            severity: {
                                type: "string",
                                enum: ["critical", "high", "medium", "low"],
                            },
                            category: { type: "string" },
                            description: { type: "string" },
                            why_it_matters: { type: "string" },
                            source: {
                                type: "string",
                                enum: ["layer1", "layer2", "both"],
                                description: "Which critic layer identified this gap (for per-critic tracking)",
                            },
                        },
                        required: ["severity", "category", "description"],
                    },
                    description: "Merged current-round gaps after critic/specialist processing.",
                },
                generation: {
                    type: "integer",
                    description: "Generation counter for zombie protection. Pass on every verifier call. If mismatched, the server rejects the request.",
                },
            },
            required: ["round", "generation"],
        },
    },
    {
        name: "assess_verification_round",
        description: "Verifier-oriented helper that classifies whether required critic findings are complete, still pending, or an infrastructure failure. " +
            "Use this after bounded delegate waits / rescue attempts instead of inferring runtime failure from raw delegate_wait snapshots and artifact polls inside the prompt. " +
            "The tool checks current-round typed finding publication on the PARENT ideation session and combines that with live delegate job state.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "parent-session-id",
                    created_after: "2026-04-13T10:00:00Z",
                    rescue_budget_exhausted: true,
                    delegates: [
                        {
                            job_id: "job-completeness",
                            artifact_prefix: "Completeness: ",
                            required: true,
                            label: "completeness",
                        },
                        {
                            job_id: "job-feasibility",
                            artifact_prefix: "Feasibility: ",
                            required: true,
                            label: "feasibility",
                        },
                    ],
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "Optional PARENT ideation session ID being verified. In verifier child context the backend resolves the canonical parent automatically.",
                },
                created_after: {
                    type: "string",
                    description: "Optional RFC3339 threshold for current-round artifact collection.",
                },
                rescue_budget_exhausted: {
                    type: "boolean",
                    description: "Set true after the verifier has used its allowed wait/rescue budget. Missing required artifacts then classify as infra_failure instead of pending.",
                },
                include_full_content: {
                    type: "boolean",
                    description: "Whether returned artifact matches should include full content. Default: true.",
                },
                include_messages: {
                    type: "boolean",
                    description: "Whether delegated job status hydration should include recent delegated-session messages. Default: true.",
                },
                message_limit: {
                    type: "integer",
                    description: "Optional delegated recent-message limit when include_messages is true. Default: 5, max: 50.",
                },
                delegates: {
                    type: "array",
                    description: "Delegated critic/specialist jobs expected to publish current-round findings. artifact_prefix is the stable slot label for this delegate in settlement output.",
                    items: {
                        type: "object",
                        properties: {
                            job_id: {
                                type: "string",
                                description: "Delegation job ID returned by delegate_start.",
                            },
                            artifact_prefix: {
                                type: "string",
                                description: "Stable slot label for this delegate in settlement output, for example 'Completeness: '.",
                            },
                            required: {
                                type: "boolean",
                                description: "Whether this delegate is required for the round classification. Default: true.",
                            },
                            label: {
                                type: "string",
                                description: "Optional short label for summaries, for example 'completeness' or 'feasibility'.",
                            },
                        },
                        required: ["job_id", "artifact_prefix"],
                    },
                },
            },
            required: ["delegates"],
        },
    },
    {
        name: "run_required_verification_critic_round",
        description: "Verifier-oriented orchestration helper that runs the required completeness and feasibility critics for one verification round on the PARENT ideation session. " +
            "The helper owns initial critic dispatch, one bounded rescue pass for any still-missing required critic finding, and final settlement. " +
            "Use this instead of manually stitching together delegate_start, rescue replacement, and await_verification_round_settlement for required critics in prompt logic.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "parent-session-id",
                    round: 3,
                    max_wait_ms: 600000,
                    poll_interval_ms: 750,
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "Optional PARENT ideation session ID being verified. In verifier child context the backend resolves the canonical parent automatically.",
                },
                round: {
                    type: "integer",
                    description: "Current verification round number (1-based).",
                },
                include_full_content: {
                    type: "boolean",
                    description: "Whether returned artifact matches should include full content. Default: true.",
                },
                include_messages: {
                    type: "boolean",
                    description: "Whether settlement snapshots should include recent delegated-session messages. Default: true.",
                },
                message_limit: {
                    type: "integer",
                    description: "Optional delegated recent-message limit when include_messages is true. Default: 5, max: 50.",
                },
                max_wait_ms: {
                    type: "integer",
                    description: "Maximum wait budget for each settlement pass. Default: 600000, max: 600000.",
                },
                poll_interval_ms: {
                    type: "integer",
                    description: "Polling interval between settlement checks. Default: 750, minimum: 100.",
                },
            },
            required: ["round"],
        },
    },
    {
        name: "run_verification_enrichment",
        description: "Backend-owned one-time verification enrichment helper for the PARENT ideation session. " +
            "The helper reads the current plan, decides whether intent and code-quality enrichment apply, dispatches those specialists once, waits a bounded amount for artifact publication or terminal delegate state, and returns the latest enrichment artifacts plus delegate snapshots. " +
            "Use this instead of manually selecting enrichment specialists, dispatching them, and polling artifacts in the verifier prompt.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "parent-session-id",
                    disabled_specialists: ["code-quality"],
                    max_wait_ms: 15000,
                    poll_interval_ms: 500,
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "Optional PARENT ideation session ID being verified. In verifier child context the backend resolves the canonical parent automatically.",
                },
                disabled_specialists: {
                    type: "array",
                    items: { type: "string" },
                    description: "Optional specialist short names to skip, for example ['intent', 'code-quality'].",
                },
                include_full_content: {
                    type: "boolean",
                    description: "Whether returned artifact matches should include full content. Default: true.",
                },
                include_messages: {
                    type: "boolean",
                    description: "Whether returned delegate snapshots should include recent delegated-session messages. Default: true.",
                },
                message_limit: {
                    type: "integer",
                    description: "Optional delegated recent-message limit when include_messages is true. Default: 5, max: 50.",
                },
                max_wait_ms: {
                    type: "integer",
                    description: "Maximum wall-clock time to wait for enrichment delegates to publish artifacts or settle. Default: 15000, max: 600000.",
                },
                poll_interval_ms: {
                    type: "integer",
                    description: "Polling interval between enrichment snapshots. Default: 500, min: 100.",
                },
            },
            required: [],
        },
    },
    {
        name: "run_verification_round",
        description: "Backend-owned verification round driver for the PARENT ideation session. " +
            "The helper reads the current plan, selects optional specialists, dispatches them, runs the required completeness + feasibility critics through the existing required-critic helper, waits for bounded optional-settlement, and returns structured required critic findings plus backend-owned merged gaps instead of raw artifact JSON. " +
            "Use this as the primary verifier round tool so the prompt only synthesizes findings and revises the plan.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "parent-session-id",
                    round: 2,
                    disabled_specialists: ["ux"],
                    max_wait_ms: 600000,
                    optional_wait_ms: 15000,
                    poll_interval_ms: 750,
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "Optional PARENT ideation session ID being verified. In verifier child context the backend resolves the canonical parent automatically.",
                },
                round: {
                    type: "integer",
                    description: "Current verification round number (1-based).",
                },
                disabled_specialists: {
                    type: "array",
                    items: { type: "string" },
                    description: "Optional specialist short names to skip, for example ['ux', 'pipeline-safety'].",
                },
                include_full_content: {
                    type: "boolean",
                    description: "Whether returned artifact matches should include full content. Default: true.",
                },
                include_messages: {
                    type: "boolean",
                    description: "Whether returned delegate snapshots should include recent delegated-session messages. Default: true.",
                },
                message_limit: {
                    type: "integer",
                    description: "Optional delegated recent-message limit when include_messages is true. Default: 5, max: 50.",
                },
                max_wait_ms: {
                    type: "integer",
                    description: "Maximum wait budget for each required-critic settlement pass. Default: 600000, max: 600000.",
                },
                optional_wait_ms: {
                    type: "integer",
                    description: "Maximum wait budget for optional specialist artifact collection after required critics settle. Default: 15000, max: 600000.",
                },
                poll_interval_ms: {
                    type: "integer",
                    description: "Polling interval between settlement snapshots. Default: 750, min: 100.",
                },
            },
            required: ["round"],
        },
    },
    {
        name: "await_verification_round_settlement",
        description: "Verifier-oriented helper that waits for required delegated critics/specialists to either publish their current-round artifacts or reach a terminal delegated state. " +
            "This is the first-class synchronization barrier for verification rounds: use it instead of narrating manual poll loops in the chat. " +
            "The tool polls round-local artifacts on the PARENT ideation session plus live delegated job state, then returns a settled classification (`complete`, `pending`, or `infra_failure`).",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "parent-session-id",
                    created_after: "2026-04-13T10:00:00Z",
                    rescue_budget_exhausted: false,
                    max_wait_ms: 600000,
                    poll_interval_ms: 750,
                    delegates: [
                        {
                            job_id: "job-completeness",
                            artifact_prefix: "Completeness: ",
                            required: true,
                            label: "completeness",
                        },
                        {
                            job_id: "job-feasibility",
                            artifact_prefix: "Feasibility: ",
                            required: true,
                            label: "feasibility",
                        },
                    ],
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "Optional PARENT ideation session ID being verified. In verifier child context the backend resolves the canonical parent automatically.",
                },
                created_after: {
                    type: "string",
                    description: "Optional RFC3339 threshold for current-round artifact collection.",
                },
                rescue_budget_exhausted: {
                    type: "boolean",
                    description: "Set true after the verifier has used its allowed wait/rescue budget. Missing required artifacts then classify as infra_failure instead of pending.",
                },
                include_full_content: {
                    type: "boolean",
                    description: "Whether returned artifact matches should include full content. Default: true.",
                },
                include_messages: {
                    type: "boolean",
                    description: "Whether delegated job status hydration should include recent delegated-session messages. Default: true.",
                },
                message_limit: {
                    type: "integer",
                    description: "Optional delegated recent-message limit when include_messages is true. Default: 5, max: 50.",
                },
                max_wait_ms: {
                    type: "integer",
                    description: "Maximum wall-clock time to wait for delegates/artifacts to settle before returning. Default: 600000, max: 600000.",
                },
                poll_interval_ms: {
                    type: "integer",
                    description: "Polling interval between settlement snapshots. Default: 750, min: 100.",
                },
                delegates: {
                    type: "array",
                    description: "Delegated critic/specialist jobs expected to publish current-round findings. artifact_prefix is the stable slot label for this delegate in settlement output.",
                    items: {
                        type: "object",
                        properties: {
                            job_id: {
                                type: "string",
                                description: "Delegation job ID returned by delegate_start.",
                            },
                            artifact_prefix: {
                                type: "string",
                                description: "Stable slot label for this delegate in settlement output, for example 'Completeness: '.",
                            },
                            required: {
                                type: "boolean",
                                description: "Whether this delegate is required for the round classification. Default: true.",
                            },
                            label: {
                                type: "string",
                                description: "Optional short label for summaries, for example 'completeness' or 'feasibility'.",
                            },
                        },
                        required: ["job_id", "artifact_prefix"],
                    },
                },
            },
            required: ["delegates"],
        },
    },
    {
        name: "complete_plan_verification",
        description: "Verifier-friendly helper for terminal verification updates on the PARENT ideation session. " +
            "The parent session remains canonical; if a verification child session_id is passed, the backend remaps it automatically. " +
            "This is the simpler alias for update_plan_verification with in_progress fixed to false. " +
            "When `required_delegates` is provided, the tool first waits on the current verification round until required delegate state/artifacts have settled before sending the terminal update. " +
            "When that settled round has typed required-critic findings, the helper derives the canonical terminal gap list from those findings instead of trusting prompt-assembled `gaps`. " +
            "If the required delegate set settles as infrastructure/runtime failure, the backend resets the parent to unverified instead of recording a bogus content verdict. " +
            "Use verified or needs_revision for normal terminal outcomes; skipped remains available only where skip is actually allowed by the backend. Do NOT pass reviewing here. " +
            "If generation is stale, call get_plan_verification again instead of guessing.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "parent-session-id",
                    status: "verified",
                    round: 1,
                    convergence_reason: "zero_blocking",
                    generation: 3,
                    required_delegates: [
                        {
                            job_id: "job-completeness",
                            artifact_prefix: "Completeness: ",
                            required: true,
                            label: "completeness",
                        },
                        {
                            job_id: "job-feasibility",
                            artifact_prefix: "Feasibility: ",
                            required: true,
                            label: "feasibility",
                        },
                    ],
                    created_after: "2026-04-13T10:00:00Z",
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "Optional PARENT ideation session ID being verified. In verifier child context the backend resolves the canonical parent automatically.",
                },
                status: {
                    type: "string",
                    enum: ["needs_revision", "verified", "skipped"],
                    description: "Terminal verification status. Use verified or needs_revision for normal completion, and skipped only for flows where skip is actually allowed.",
                },
                round: {
                    type: "integer",
                    description: "Current round number (1-based). Include when it helps downstream summaries.",
                },
                gaps: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            severity: {
                                type: "string",
                                enum: ["critical", "high", "medium", "low"],
                            },
                            category: { type: "string" },
                            description: { type: "string" },
                            why_it_matters: { type: "string" },
                            source: {
                                type: "string",
                                enum: ["layer1", "layer2", "both"],
                                description: "Which critic layer identified this gap (for per-critic tracking)",
                            },
                        },
                        required: ["severity", "category", "description"],
                    },
                    description: "Optional terminal gap list. When required_delegates + created_after are provided for a verifier round, the helper derives canonical gaps from typed required-critic findings instead.",
                },
                convergence_reason: {
                    type: "string",
                    enum: [
                        "zero_blocking",
                        "jaccard_converged",
                        "max_rounds",
                        "critic_parse_failure",
                        "agent_error",
                        "user_stopped",
                        "user_skipped",
                        "user_reverted",
                        "escalated_to_parent",
                    ],
                    description: "Why verification converged or exited. Recommended on every terminal call and required for the standard verifier cleanup flow.",
                },
                generation: {
                    type: "integer",
                    description: "Generation counter for zombie protection. Pass on every verifier call. If mismatched, the server rejects the request.",
                },
                created_after: {
                    type: "string",
                    description: "Optional RFC3339 threshold for current-round artifact settlement before terminal completion.",
                },
                rescue_budget_exhausted: {
                    type: "boolean",
                    description: "Set true after the verifier has used its allowed wait/rescue budget for the required delegates of this round.",
                },
                include_full_content: {
                    type: "boolean",
                    description: "Whether settlement should fetch full artifact content for the latest matching artifacts. Default: true.",
                },
                include_messages: {
                    type: "boolean",
                    description: "Whether delegated job status hydration should include recent delegated-session messages during settlement. Default: true.",
                },
                message_limit: {
                    type: "integer",
                    description: "Optional delegated recent-message limit when include_messages is true. Default: 5, max: 50.",
                },
                max_wait_ms: {
                    type: "integer",
                    description: "Maximum time to wait for required delegates to settle before terminal completion. Default: 600000, max: 600000.",
                },
                poll_interval_ms: {
                    type: "integer",
                    description: "Polling interval used while waiting for required delegates to settle. Default: 750, min: 100.",
                },
                required_delegates: {
                    type: "array",
                    description: "Optional required delegated critic/specialist jobs for this terminal update. When provided, completion is blocked until the round settles or the helper classifies an infra failure.",
                    items: {
                        type: "object",
                        properties: {
                            job_id: {
                                type: "string",
                                description: "Delegation job ID returned by delegate_start.",
                            },
                            artifact_prefix: {
                                type: "string",
                                description: "Stable slot label for this delegate in settlement output, for example 'Completeness: '.",
                            },
                            required: {
                                type: "boolean",
                                description: "Whether this delegate is required for the round classification. Default: true.",
                            },
                            label: {
                                type: "string",
                                description: "Optional short label for summaries, for example 'completeness' or 'feasibility'.",
                            },
                        },
                        required: ["job_id", "artifact_prefix"],
                    },
                },
            },
            required: ["status", "generation"],
        },
    },
    {
        name: "update_plan_verification",
        description: "Update verification state for an ideation session. Use the PARENT ideation session_id as the canonical target; if a verification child session_id is passed, the backend remaps it automatically. " +
            "Typical verifier flow: mid-round call with status='reviewing', in_progress=true, round=<n>, gaps=[...], generation=<current>; terminal call with in_progress=false and status='verified' or 'needs_revision'. " +
            "External sessions cannot use status='skipped'. If the server rejects a call, read the error and correct the payload instead of guessing a new shape. " +
            "Example reviewing payload: {\"session_id\":\"<parent-session>\",\"status\":\"reviewing\",\"in_progress\":true,\"round\":1,\"gaps\":[],\"generation\":3}. " +
            "Example terminal payload: {\"session_id\":\"<parent-session>\",\"status\":\"verified\",\"in_progress\":false,\"round\":1,\"gaps\":[],\"convergence_reason\":\"zero_blocking\",\"generation\":3}.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "parent-session-id",
                    status: "reviewing",
                    in_progress: true,
                    round: 1,
                    gaps: [],
                    generation: 3,
                },
                {
                    session_id: "parent-session-id",
                    status: "verified",
                    in_progress: false,
                    round: 1,
                    gaps: [],
                    convergence_reason: "zero_blocking",
                    generation: 3,
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "PARENT ideation session ID being verified. Verification child session IDs are auto-remapped to that parent.",
                },
                status: {
                    type: "string",
                    enum: ["reviewing", "needs_revision", "verified", "skipped"],
                    description: "New verification status. Use reviewing for in-progress rounds; use verified or needs_revision only for terminal updates; skipped is not allowed for external sessions.",
                },
                in_progress: {
                    type: "boolean",
                    description: "Whether the verification loop is still active. Mid-round updates should use true; final cleanup should use false.",
                },
                round: {
                    type: "integer",
                    description: "Current round number (1-based). Include on reviewing updates.",
                },
                gaps: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            severity: {
                                type: "string",
                                enum: ["critical", "high", "medium", "low"],
                            },
                            category: { type: "string" },
                            description: { type: "string" },
                            why_it_matters: { type: "string" },
                            source: {
                                type: "string",
                                enum: ["layer1", "layer2", "both"],
                                description: "Which critic layer identified this gap (for per-critic tracking)",
                            },
                        },
                        required: ["severity", "category", "description"],
                    },
                    description: "Gaps identified in this round. For reviewing updates this should reflect the merged current-round gaps.",
                },
                convergence_reason: {
                    type: "string",
                    enum: [
                        "zero_blocking",
                        "jaccard_converged",
                        "max_rounds",
                        "critic_parse_failure",
                        "agent_error",
                        "user_stopped",
                        "user_skipped",
                        "user_reverted",
                        "escalated_to_parent",
                    ],
                    description: "Why verification converged. Required for terminal needs_revision -> verified promotion and recommended on all terminal calls.",
                },
                generation: {
                    type: "integer",
                    description: "Generation counter for zombie protection. Pass on every verifier call. If omitted, weaker models can accidentally write stale state; if mismatched, the server rejects the request.",
                },
            },
            required: ["session_id", "status"],
        },
    },
    {
        name: "get_plan_verification",
        description: "Get the current verification status for the PARENT ideation session. Use this before and during verification to confirm the generation, in_progress flag, and current round before calling report_verification_round or complete_plan_verification. Verification child session_ids are auto-remapped to the parent ideation session. " +
            "If a verification update call is rejected, call this again on the parent session and copy the returned generation/in_progress values instead of guessing.",
        inputSchema: {
            type: "object",
            examples: [{ session_id: "parent-session-id" }],
            properties: {
                session_id: {
                    type: "string",
                    description: "Optional ideation session ID to inspect. In verifier child context the backend resolves the canonical parent automatically.",
                },
            },
            required: [],
        },
    },
    {
        name: "revert_and_skip",
        description: "Atomically revert the plan to a previous version and skip verification. " +
            "Use when the user wants to discard recent plan changes and proceed without re-running adversarial review. " +
            "Restores the exact content of the specified plan version, creates a new artifact entry for auditability, " +
            "and sets verification status to skipped in a single atomic operation.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
                plan_version_to_restore: {
                    type: "string",
                    description: "The artifact ID of the plan version to restore content from. " +
                        "Typically a previous version's artifact ID (e.g., from before edits were made).",
                },
            },
            required: ["session_id", "plan_version_to_restore"],
        },
    },
    {
        name: "stop_verification",
        description: "Stop a running verification loop and skip to completion. Kills the verification child agent immediately, sets status to skipped with convergence_reason user_stopped, and unfreezes the plan artifact. Idempotent: returns success if no verification is in progress.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
            },
            required: ["session_id"],
        },
    },
    {
        name: "edit_plan_artifact",
        description: "Apply anchor-based edit operations to an existing implementation plan. More token-efficient than update_plan_artifact for targeted changes — only send the text to find and replace, not the entire plan content. Each edit finds the first occurrence of old_text and replaces it with new_text. Stale artifact IDs are auto-resolved to the latest version. Edits are applied sequentially; if any edit fails (old_text not found or ambiguous), the entire operation is rejected with details of which edit failed.",
        inputSchema: {
            type: "object",
            properties: {
                artifact_id: {
                    type: "string",
                    description: "The artifact ID of the plan to edit. Can be any version ID — stale IDs are auto-resolved to the latest version.",
                },
                edits: {
                    type: "array",
                    minItems: 1,
                    maxItems: 20,
                    items: {
                        type: "object",
                        properties: {
                            old_text: {
                                type: "string",
                                minLength: 1,
                                description: "The exact text to find in the plan. Must be unique within the plan content to avoid ambiguous replacements.",
                            },
                            new_text: {
                                type: "string",
                                description: "The replacement text. Can be empty string to delete the matched text.",
                            },
                        },
                        required: ["old_text", "new_text"],
                    },
                    description: "List of edit operations to apply sequentially. Each operation finds old_text and replaces with new_text.",
                },
                caller_session_id: {
                    type: "string",
                    description: "The session ID of the caller. Required when calling from a verification child session to bypass the write lock on the plan artifact.",
                },
            },
            required: ["artifact_id", "edits"],
        },
    },
];
//# sourceMappingURL=plan-tools.js.map