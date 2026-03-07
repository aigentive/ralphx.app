/**
 * MCP tool definitions for RalphX
 * All tools are proxies that forward to Tauri backend via HTTP
 */
import { PLAN_TOOLS } from "./plan-tools.js";
import { WORKER_CONTEXT_TOOLS } from "./worker-context-tools.js";
import { STEP_TOOLS } from "./step-tools.js";
import { ISSUE_TOOLS } from "./issue-tools.js";
import { ORCHESTRATOR_IDEATION, ORCHESTRATOR_IDEATION_READONLY, CHAT_TASK, CHAT_PROJECT, REVIEWER, REVIEW_CHAT, REVIEW_HISTORY, WORKER, CODER, SESSION_NAMER, DEPENDENCY_SUGGESTER, MERGER, PROJECT_ANALYZER, SUPERVISOR, QA_PREP, QA_TESTER, ORCHESTRATOR, DEEP_RESEARCHER, MEMORY_MAINTAINER, MEMORY_CAPTURE, IDEATION_TEAM_LEAD, IDEATION_TEAM_MEMBER, WORKER_TEAM_LEAD, WORKER_TEAM_MEMBER, } from "./agentNames.js";
/**
 * All available MCP tools
 * Tools are filtered based on RALPHX_AGENT_TYPE environment variable
 */
export const ALL_TOOLS = [
    // ========================================================================
    // IDEATION TOOLS (orchestrator-ideation agent)
    // ========================================================================
    {
        name: "create_task_proposal",
        description: "Create a new task proposal in the ideation session. Use this when the user describes a new feature, fix, or improvement they want to implement.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID (provided in context)",
                },
                title: {
                    type: "string",
                    description: "Clear, concise task title (e.g., 'Add dark mode toggle')",
                },
                description: {
                    type: "string",
                    description: "Detailed description of what needs to be done",
                },
                category: {
                    type: "string",
                    enum: ["setup", "feature", "fix", "refactor", "docs", "test", "performance", "security", "devops", "research", "design", "chore"],
                    description: "Task category: setup (project init/infra), feature (new functionality), fix (bug fix), refactor (code restructure), docs (documentation), test (testing), performance (optimization), security (security hardening), devops (CI/CD/tooling), research (investigation/spike), design (UX/UI design), chore (maintenance/cleanup)",
                },
                priority: {
                    type: "string",
                    enum: ["critical", "high", "medium", "low"],
                    description: "Suggested priority level. Default: medium",
                },
                steps: {
                    type: "array",
                    items: { type: "string" },
                    description: "Step-by-step implementation plan. Each step should be a clear, actionable task (1-3 sentences). Typically 3-7 steps.",
                },
                acceptance_criteria: {
                    type: "array",
                    items: { type: "string" },
                    description: "Testable criteria to verify task completion (e.g., 'API returns 200 with valid schema', 'All tests pass'). Typically 3-5 criteria.",
                },
            },
            required: ["session_id", "title", "category"],
        },
    },
    {
        name: "update_task_proposal",
        description: "Update an existing task proposal. Use when the user wants to modify a proposal's details, priority, or implementation plan.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_id: {
                    type: "string",
                    description: "The proposal ID to update",
                },
                title: {
                    type: "string",
                    description: "Updated task title",
                },
                description: {
                    type: "string",
                    description: "Updated description",
                },
                category: {
                    type: "string",
                    enum: ["setup", "feature", "fix", "refactor", "docs", "test", "performance", "security", "devops", "research", "design", "chore"],
                    description: "Updated category: setup (project init/infra), feature (new functionality), fix (bug fix), refactor (code restructure), docs (documentation), test (testing), performance (optimization), security (security hardening), devops (CI/CD/tooling), research (investigation/spike), design (UX/UI design), chore (maintenance/cleanup)",
                },
                user_priority: {
                    type: "string",
                    enum: ["critical", "high", "medium", "low"],
                    description: "Updated priority level (overrides AI-suggested priority)",
                },
                steps: {
                    type: "array",
                    items: { type: "string" },
                    description: "Updated implementation steps. Each step should be a clear, actionable task (1-3 sentences). Typically 3-7 steps.",
                },
                acceptance_criteria: {
                    type: "array",
                    items: { type: "string" },
                    description: "Updated acceptance criteria. Testable criteria to verify task completion (e.g., 'API returns 200 with valid schema'). Typically 3-5 criteria.",
                },
            },
            required: ["proposal_id"],
        },
    },
    {
        name: "delete_task_proposal",
        description: "Delete a task proposal. Use when the user wants to remove a proposal that's no longer needed.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_id: {
                    type: "string",
                    description: "The proposal ID to delete",
                },
            },
            required: ["proposal_id"],
        },
    },
    {
        name: "add_proposal_dependency",
        description: "Add a dependency relationship between two proposals. Use when one task must be completed before another can start.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_id: {
                    type: "string",
                    description: "The proposal that depends on another",
                },
                depends_on_id: {
                    type: "string",
                    description: "The proposal that must be completed first",
                },
            },
            required: ["proposal_id", "depends_on_id"],
        },
    },
    {
        name: "apply_proposal_dependencies",
        description: "Apply AI-suggested dependencies directly to proposals. Clears existing dependencies and applies new ones. Used by dependency-suggester agent.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
                dependencies: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            proposal_id: {
                                type: "string",
                                description: "The proposal that depends on another",
                            },
                            depends_on_id: {
                                type: "string",
                                description: "The proposal that must be completed first",
                            },
                            reason: {
                                type: "string",
                                description: "Brief explanation of why this dependency exists",
                            },
                        },
                        required: ["proposal_id", "depends_on_id"],
                    },
                    description: "Array of dependency suggestions to apply",
                },
            },
            required: ["session_id", "dependencies"],
        },
    },
    {
        name: "update_session_title",
        description: "Update the title of an ideation session. Used by session-namer agent to set auto-generated titles.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID to update",
                },
                title: {
                    type: "string",
                    description: "The new title for the session (exactly 2 words)",
                },
            },
            required: ["session_id", "title"],
        },
    },
    {
        name: "list_session_proposals",
        description: "List all task proposals in an ideation session. Returns summary info (id, title, category, priority, dependencies). Use get_proposal for full details including steps and acceptance criteria.",
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
        name: "get_proposal",
        description: "Get full details of a task proposal including steps and acceptance criteria. Use after list_session_proposals to get complete information for a specific proposal.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_id: {
                    type: "string",
                    description: "The proposal ID to fetch",
                },
            },
            required: ["proposal_id"],
        },
    },
    {
        name: "analyze_session_dependencies",
        description: "Get full dependency graph analysis including critical path, cycle detection, and blocking relationships. " +
            "Use to provide intelligent recommendations about proposal execution order. " +
            "If analysis_in_progress is true in the response, wait 2-3 seconds and retry for complete results.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID to analyze",
                },
            },
            required: ["session_id"],
        },
    },
    // ========================================================================
    // QUESTION TOOLS (orchestrator-ideation agent — inline AskUserQuestion)
    // ========================================================================
    {
        name: "ask_user_question",
        description: "Ask the user a clarifying question with optional predefined answer options. " +
            "The question appears as an inline card in the chat. " +
            "This tool blocks until the user responds (up to 5 minutes). " +
            "Use for confirmations, multi-choice selections, or open-ended questions during ideation.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID (provided in context)",
                },
                question: {
                    type: "string",
                    description: "The question text to display to the user",
                },
                header: {
                    type: "string",
                    description: "Optional header/title above the question (e.g., 'Confirm Plan')",
                },
                options: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            label: {
                                type: "string",
                                description: "Short label for the option (e.g., 'Yes', 'Option A')",
                            },
                            value: {
                                type: "string",
                                description: "Programmatic value returned when this option is selected. Defaults to label if omitted.",
                            },
                            description: {
                                type: "string",
                                description: "Optional longer description of what this option means",
                            },
                        },
                        required: ["label"],
                    },
                    description: "Predefined answer options. If omitted, user can type a free-form response.",
                },
                multi_select: {
                    type: "boolean",
                    description: "If true and options are provided, user can select multiple options. Default: false.",
                },
            },
            required: ["session_id", "question"],
        },
    },
    // ========================================================================
    // SESSION LINKING TOOLS (orchestrator-ideation agent)
    // ========================================================================
    {
        name: "create_child_session",
        description: "Create a new ideation session as a child of an existing session. Use when you want to create follow-on work that inherits context from the parent session. " +
            "The child session starts with 'active' status. " +
            "When inherit_context is true (default), the child receives a read-only reference to the parent's plan artifact AND inherits the parent's team_mode and team_config. " +
            "The inherited plan cannot be modified — call create_plan_artifact to create an independent plan for the child session. " +
            "Parent proposals are NOT copied to the child — use get_parent_session_context to access them.",
        inputSchema: {
            type: "object",
            properties: {
                parent_session_id: {
                    type: "string",
                    description: "The parent ideation session ID",
                },
                title: {
                    type: "string",
                    description: "Optional title for the new child session",
                },
                description: {
                    type: "string",
                    description: "Optional description of the child session. When provided, an orchestrator-ideation agent is automatically spawned in the background to process this description and generate task proposals.",
                },
                inherit_context: {
                    type: "boolean",
                    description: "If true, child receives a read-only reference to parent's plan artifact and inherits team_mode/team_config from parent. To create a new plan, call create_plan_artifact — it creates an independent plan for the child. Parent proposals accessible via get_parent_session_context. Default: true.",
                },
                initial_prompt: {
                    type: "string",
                    description: "Optional initial prompt/message to forward to the child session's agent. This is the user's message that triggered the child session creation.",
                },
                team_mode: {
                    type: "string",
                    enum: ["solo", "research", "debate"],
                    description: "Team mode for the child session. If omitted and inherit_context=true, inherits from parent session. Use 'solo' for single-agent execution, 'research' for parallel research teams, 'debate' for adversarial analysis.",
                },
                team_config: {
                    type: "object",
                    description: "Team constraints override. If omitted and inherit_context=true, inherits from parent session. Explicit values replace inherited config. Validated against current project constraints.",
                    properties: {
                        max_teammates: {
                            type: "number",
                            description: "Maximum number of teammates allowed (capped at project constraint)",
                        },
                        model_ceiling: {
                            type: "string",
                            enum: ["haiku", "sonnet", "opus"],
                            description: "Maximum model tier allowed for teammates",
                        },
                        budget_limit: {
                            type: "number",
                            description: "Budget limit for this session (not inherited)",
                        },
                        composition_mode: {
                            type: "string",
                            enum: ["dynamic", "constrained"],
                            description: "dynamic: ad-hoc teammate selection, constrained: use predefined presets",
                        },
                    },
                },
            },
            required: ["parent_session_id"],
        },
    },
    {
        name: "get_parent_session_context",
        description: "Get the parent session context for a child session. Returns parent session metadata, plan content, and proposals summary.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The child session ID",
                },
            },
            required: ["session_id"],
        },
    },
    {
        name: "get_session_messages",
        description: "Get chat messages for an ideation session. Used for context recovery when resuming an expired session. " +
            "Returns messages newest-first (up to limit). The truncated flag indicates if older messages were dropped. " +
            "Default limit: 50, max: 200. Set include_tool_calls=true to include tool_calls JSON (increases token usage).",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
                limit: {
                    type: "number",
                    description: "Maximum messages to return (default: 50, max: 200)",
                },
                include_tool_calls: {
                    type: "boolean",
                    description: "Include tool_calls JSON in response (default: false)",
                },
            },
            required: ["session_id"],
        },
    },
    // ========================================================================
    // TEAM TOOLS (team lead agents)
    // ========================================================================
    {
        name: "request_team_plan",
        description: "Request approval for a team plan before spawning teammates. " +
            "The plan includes the process type, teammate roles/models/tools, and execution strategy. " +
            "User approval is required before teammates can be spawned.",
        inputSchema: {
            type: "object",
            properties: {
                process: {
                    type: "string",
                    description: "Process type: 'ideation-research', 'ideation-debate', 'worker-parallel'",
                },
                teammates: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            role: {
                                type: "string",
                                description: "Teammate role name (e.g., 'frontend-researcher', 'coder-1')",
                            },
                            tools: {
                                type: "array",
                                items: { type: "string" },
                                description: "CLI tools for this teammate (e.g., ['Read', 'Grep', 'Glob'])",
                            },
                            mcp_tools: {
                                type: "array",
                                items: { type: "string" },
                                description: "MCP tools for this teammate (e.g., ['get_session_plan'])",
                            },
                            model: {
                                type: "string",
                                description: "Model to use: 'haiku', 'sonnet', or 'opus'",
                            },
                            preset: {
                                type: "string",
                                description: "Optional predefined agent template (for constrained mode)",
                            },
                            prompt_summary: {
                                type: "string",
                                description: "Brief summary of what this teammate will do",
                            },
                        },
                        required: ["role", "tools", "mcp_tools", "model", "prompt_summary"],
                    },
                    description: "Array of teammate configurations to spawn",
                },
                team_name: {
                    type: "string",
                    description: "Team name from the lead agent's TeamCreate call. Ensures teammates join the same team registry.",
                },
            },
            required: ["process", "teammates", "team_name"],
        },
    },
    {
        name: "request_teammate_spawn",
        description: "Request to spawn a single teammate. " +
            "The backend validates the request against team constraints, then spawns the teammate if approved.",
        inputSchema: {
            type: "object",
            properties: {
                role: {
                    type: "string",
                    description: "Teammate role name (e.g., 'frontend-researcher', 'coder-1')",
                },
                prompt: {
                    type: "string",
                    description: "Full prompt for the teammate describing their role and expected output",
                },
                model: {
                    type: "string",
                    enum: ["haiku", "sonnet", "opus"],
                    description: "Model to use (must be within model_ceiling constraint)",
                },
                tools: {
                    type: "array",
                    items: { type: "string" },
                    description: "Requested CLI tools (intersected with tool_ceiling)",
                },
                mcp_tools: {
                    type: "array",
                    items: { type: "string" },
                    description: "Requested MCP tools (intersected with mcp_tool_ceiling)",
                },
                preset: {
                    type: "string",
                    description: "Optional predefined agent template to use (for constrained mode)",
                },
            },
            required: ["role", "prompt", "model", "tools", "mcp_tools"],
        },
    },
    {
        name: "create_team_artifact",
        description: "Create a team artifact documenting research findings, analysis, or summary. " +
            "Automatically sets bucket_id='team-findings' and populates metadata with team info. " +
            "Use for documenting team discoveries, debate analyses, or lead-synthesized summaries.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation or execution session ID",
                },
                title: {
                    type: "string",
                    description: "Clear, concise title for the artifact",
                },
                content: {
                    type: "string",
                    description: "Markdown content with research findings or analysis",
                },
                artifact_type: {
                    type: "string",
                    enum: ["TeamResearch", "TeamAnalysis", "TeamSummary"],
                    description: "Type: TeamResearch (specialist findings), TeamAnalysis (comparison/debate), TeamSummary (lead synthesis)",
                },
                related_artifact_id: {
                    type: "string",
                    description: "Optional artifact ID to link to (e.g., master plan artifact)",
                },
            },
            required: ["session_id", "title", "content", "artifact_type"],
        },
    },
    {
        name: "get_team_artifacts",
        description: "Retrieve all team artifacts for a session. " +
            "Returns artifacts from the 'team-findings' bucket filtered by session ID.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation or execution session ID",
                },
            },
            required: ["session_id"],
        },
    },
    {
        name: "get_team_session_state",
        description: "Retrieve persisted team composition and phase progress for session recovery. " +
            "Returns team composition (teammate names/roles/prompts), current phase, and artifact IDs.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation or execution session ID",
                },
            },
            required: ["session_id"],
        },
    },
    {
        name: "save_team_session_state",
        description: "Persist current team composition to database for session recovery. " +
            "Called after spawning teammates to enable resume if session is interrupted.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation or execution session ID",
                },
                team_composition: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            name: {
                                type: "string",
                                description: "Teammate name",
                            },
                            role: {
                                type: "string",
                                description: "Teammate role description",
                            },
                            prompt: {
                                type: "string",
                                description: "Full prompt used to spawn this teammate",
                            },
                            model: {
                                type: "string",
                                description: "Model used for this teammate",
                            },
                        },
                        required: ["name", "role", "prompt", "model"],
                    },
                    description: "Array of teammate configurations",
                },
                phase: {
                    type: "string",
                    description: "Current workflow phase (e.g., 'EXPLORE', 'PLAN', 'CONFIRM')",
                },
                artifact_ids: {
                    type: "array",
                    items: { type: "string" },
                    description: "IDs of team artifacts created so far",
                },
            },
            required: ["session_id", "team_composition", "phase"],
        },
    },
    // ========================================================================
    // TASK TOOLS (chat-task agent)
    // ========================================================================
    {
        name: "update_task",
        description: "Update an existing task's details. Use when the user wants to modify task title, description, or priority. For status changes, use move_task or workflow commands.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID to update",
                },
                title: {
                    type: "string",
                    description: "Updated task title",
                },
                description: {
                    type: "string",
                    description: "Updated description",
                },
                priority: {
                    type: "string",
                    enum: ["critical", "high", "medium", "low"],
                    description: "Updated priority",
                },
            },
            required: ["task_id"],
        },
    },
    {
        name: "add_task_note",
        description: "Add a note or comment to a task. Use when the user wants to document progress, issues, or decisions.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID",
                },
                note: {
                    type: "string",
                    description: "The note content",
                },
            },
            required: ["task_id", "note"],
        },
    },
    {
        name: "get_task_details",
        description: "Get full details for a task including current status, notes, and history. Use when you need complete task information.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID",
                },
            },
            required: ["task_id"],
        },
    },
    // ========================================================================
    // PROJECT TOOLS (chat-project agent)
    // ========================================================================
    {
        name: "suggest_task",
        description: "Suggest a new task based on project analysis. Use when you've identified something that should be done based on codebase exploration.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID (provided in context)",
                },
                title: {
                    type: "string",
                    description: "Suggested task title",
                },
                description: {
                    type: "string",
                    description: "Why this task should be done",
                },
                category: {
                    type: "string",
                    enum: ["setup", "feature", "fix", "refactor", "docs", "test", "performance", "security", "devops", "research", "design", "chore"],
                    description: "Task category: setup (project init/infra), feature (new functionality), fix (bug fix), refactor (code restructure), docs (documentation), test (testing), performance (optimization), security (security hardening), devops (CI/CD/tooling), research (investigation/spike), design (UX/UI design), chore (maintenance/cleanup)",
                },
                priority: {
                    type: "string",
                    enum: ["critical", "high", "medium", "low"],
                    description: "Suggested priority level",
                },
            },
            required: ["project_id", "title", "description", "category"],
        },
    },
    {
        name: "list_tasks",
        description: "List tasks in the project with optional filtering. Use to answer questions about what tasks exist, their status, or priorities.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                status: {
                    type: "string",
                    enum: [
                        "backlog",
                        "ready",
                        "blocked",
                        "executing",
                        "qa_refining",
                        "qa_testing",
                        "qa_passed",
                        "qa_failed",
                        "pending_review",
                        "reviewing",
                        "review_passed",
                        "escalated",
                        "revision_needed",
                        "re_executing",
                        "approved",
                        "failed",
                        "cancelled",
                    ],
                    description: "Filter by status (optional)",
                },
                category: {
                    type: "string",
                    enum: ["setup", "feature", "fix", "refactor", "docs", "test", "performance", "security", "devops", "research", "design", "chore"],
                    description: "Filter by category (optional): setup, feature, fix, refactor, docs, test, performance, security, devops, research, design, chore",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "search_memories",
        description: "Search project memories by optional text query and bucket filter. " +
            "Use this to retrieve relevant learned context before planning or answering questions.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                query: {
                    type: "string",
                    description: "Optional text query matched against title/summary/details",
                },
                bucket: {
                    type: "string",
                    enum: [
                        "architecture_patterns",
                        "implementation_discoveries",
                        "operational_playbooks",
                    ],
                    description: "Optional memory bucket filter",
                },
                limit: {
                    type: "number",
                    description: "Optional max number of results",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "get_memory",
        description: "Get a single memory entry by ID. Use after search_memories when you need full details.",
        inputSchema: {
            type: "object",
            properties: {
                memory_id: {
                    type: "string",
                    description: "The memory entry ID",
                },
            },
            required: ["memory_id"],
        },
    },
    {
        name: "get_memories_for_paths",
        description: "Get memories relevant to one or more file paths using scope path matching. " +
            "Use this before editing specific files to load related historical context.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                paths: {
                    type: "array",
                    items: { type: "string" },
                    description: "File paths to match against memory scope paths",
                },
                limit: {
                    type: "number",
                    description: "Optional max number of results",
                },
            },
            required: ["project_id", "paths"],
        },
    },
    // ========================================================================
    // MERGE TOOLS (merger agent)
    // ========================================================================
    {
        name: "report_conflict",
        description: "Signal that merge conflicts could not be resolved automatically. Call this when conflicts are too complex (ambiguous intent, architectural incompatibility, or missing context). This transitions the task from Merging to MergeConflict state, keeping the branch/worktree for manual resolution.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID with unresolved conflicts",
                },
                conflict_files: {
                    type: "array",
                    items: { type: "string" },
                    description: "List of file paths that still have conflicts",
                },
                reason: {
                    type: "string",
                    description: "Explanation of why the conflicts couldn't be resolved",
                },
            },
            required: ["task_id", "conflict_files", "reason"],
        },
    },
    {
        name: "report_incomplete",
        description: "Report that merge cannot be completed due to non-conflict errors (e.g., git operation failures, missing configuration). " +
            "Use this instead of report_conflict when there are no actual merge conflicts but the merge still failed. " +
            "This transitions the task from Merging to MergeIncomplete state.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID where merge failed",
                },
                reason: {
                    type: "string",
                    description: "Detailed explanation of why the merge failed",
                },
                diagnostic_info: {
                    type: "string",
                    description: "Git status, logs, or other diagnostic output to help debug the issue",
                },
            },
            required: ["task_id", "reason"],
        },
    },
    {
        name: "complete_merge",
        description: "Signal that merge conflicts have been resolved and the merge is complete. Call this after successfully resolving all conflicts, staging changes, and completing the rebase/merge. Provide the commit SHA of the final merge commit (use `git rev-parse HEAD`). This transitions the task from Merging to Merged state.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID whose merge is complete",
                },
                commit_sha: {
                    type: "string",
                    description: "Full 40-character SHA of the merge/rebase commit (from `git rev-parse HEAD`)",
                },
            },
            required: ["task_id", "commit_sha"],
        },
    },
    {
        name: "get_merge_target",
        description: "Get the resolved merge target branches for a task. " +
            "Returns source_branch (task's branch) and target_branch (where to merge INTO). " +
            "IMPORTANT: Always call this BEFORE merging to know the correct target. " +
            "The target may be a plan feature branch instead of main.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: { type: "string", description: "The task ID" },
            },
            required: ["task_id"],
        },
    },
    // ========================================================================
    // REVIEW TOOLS (reviewer agent)
    // ========================================================================
    {
        name: "complete_review",
        description: "Submit a code review decision. Use after reviewing changes to approve, request changes, or escalate to supervisor.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task being reviewed",
                },
                decision: {
                    type: "string",
                    enum: ["approved", "needs_changes", "escalate"],
                    description: "Review decision: approved (ship it), needs_changes (fixable issues), escalate (major concerns)",
                },
                feedback: {
                    type: "string",
                    description: "Detailed feedback: what's good, what needs improvement, specific issues found",
                },
                issues: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            severity: {
                                type: "string",
                                enum: ["critical", "major", "minor", "suggestion"],
                            },
                            file: { type: "string" },
                            line: { type: "number" },
                            description: { type: "string" },
                        },
                        required: ["severity", "description"],
                    },
                    description: "Specific issues found during review",
                },
                escalation_reason: {
                    type: "string",
                    description: "Required when decision is 'escalate': concise explanation of why human review is needed",
                },
            },
            required: ["task_id", "decision", "feedback"],
        },
    },
    {
        name: "get_review_notes",
        description: "Get all review feedback for a task. Call this before re-executing a task to understand what needs to be fixed.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID to get review notes for",
                },
            },
            required: ["task_id"],
        },
    },
    {
        name: "approve_task",
        description: "Approve a task after AI review. ONLY available when task is in 'review_passed' or 'escalated' status (awaiting human decision). " +
            "Use this when the user confirms they want to approve the task after discussing the review with you. " +
            "This will NOT work during active review - use complete_review for that.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID to approve",
                },
                comment: {
                    type: "string",
                    description: "Optional approval comment or notes",
                },
            },
            required: ["task_id"],
        },
    },
    {
        name: "request_task_changes",
        description: "Request changes on a task after AI review. ONLY available when task is in 'review_passed' or 'escalated' status (awaiting human decision). " +
            "Use this when the user wants to request changes after discussing the review with you. " +
            "This will NOT work during active review - use complete_review for that.",
        inputSchema: {
            type: "object",
            properties: {
                task_id: {
                    type: "string",
                    description: "The task ID to request changes on",
                },
                feedback: {
                    type: "string",
                    description: "Detailed feedback explaining what changes are needed",
                },
            },
            required: ["task_id", "feedback"],
        },
    },
    // ========================================================================
    // PLAN ARTIFACT TOOLS (orchestrator-ideation agent)
    // ========================================================================
    ...PLAN_TOOLS,
    // ========================================================================
    // WORKER CONTEXT TOOLS (worker agent)
    // ========================================================================
    ...WORKER_CONTEXT_TOOLS,
    // ========================================================================
    // STEP TOOLS (worker agent)
    // ========================================================================
    ...STEP_TOOLS,
    // ========================================================================
    // ISSUE TOOLS (worker + reviewer agents)
    // ========================================================================
    ...ISSUE_TOOLS,
    // ========================================================================
    // MEMORY WRITE TOOLS (memory agents only - restricted via allowlist)
    // ========================================================================
    {
        name: "upsert_memories",
        description: "Batch upsert memory entries to SQLite canonical storage. " +
            "Performs content-hash deduplication to prevent duplicates. " +
            "WRITE-ONLY tool restricted to memory-maintainer and memory-capture agents.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID (from RALPHX_PROJECT_ID env var)",
                },
                memories: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            bucket: {
                                type: "string",
                                enum: ["architecture_patterns", "implementation_discoveries", "operational_playbooks"],
                                description: "Memory bucket classification",
                            },
                            title: {
                                type: "string",
                                description: "Concise title for this memory (50-80 chars)",
                            },
                            summary: {
                                type: "string",
                                description: "Brief summary suitable for rule index files (1-3 sentences)",
                            },
                            details_markdown: {
                                type: "string",
                                description: "Full markdown details with examples, context, and rationale",
                            },
                            scope_paths: {
                                type: "array",
                                items: { type: "string" },
                                description: "Glob patterns for path scoping (e.g., ['src/domain/**', 'src-tauri/src/application/**'])",
                            },
                            source_context_type: {
                                type: "string",
                                description: "Optional: context type (e.g., 'task_execution', 'planning', 'review')",
                            },
                            source_context_id: {
                                type: "string",
                                description: "Optional: source context ID (e.g., task_id, session_id)",
                            },
                            source_conversation_id: {
                                type: "string",
                                description: "Optional: conversation ID for traceability",
                            },
                            quality_score: {
                                type: "number",
                                description: "Optional: quality score 0-1 (higher = more valuable)",
                            },
                        },
                        required: ["bucket", "title", "summary", "details_markdown", "scope_paths"],
                    },
                    description: "Array of memory entries to upsert",
                },
            },
            required: ["project_id", "memories"],
        },
    },
    {
        name: "mark_memory_obsolete",
        description: "Mark a memory entry as obsolete (soft delete). " +
            "The memory remains in DB but is excluded from index generation and searches. " +
            "WRITE-ONLY tool restricted to memory-maintainer agent.",
        inputSchema: {
            type: "object",
            properties: {
                memory_id: {
                    type: "string",
                    description: "The memory entry ID to mark obsolete",
                },
            },
            required: ["memory_id"],
        },
    },
    {
        name: "refresh_memory_rule_index",
        description: "Regenerate .claude/rules/ index files from DB canonical state. " +
            "Reads memory entries for project, groups by scope_key, and writes index files with summaries + memory IDs. " +
            "WRITE-ONLY tool restricted to memory-maintainer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                scope_key: {
                    type: "string",
                    description: "Optional: specific scope_key to refresh. If omitted, refreshes all rule indexes for project.",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "ingest_rule_file",
        description: "Ingest a .claude/rules/*.md file into canonical memory DB. " +
            "Parses content into chunks, classifies buckets, upserts to memory_entries, " +
            "rewrites file to index format, and enqueues archive jobs. " +
            "WRITE-ONLY tool restricted to memory-maintainer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                rule_file_path: {
                    type: "string",
                    description: "Path to rule file relative to project root (e.g., '.claude/rules/task-state-machine.md')",
                },
            },
            required: ["project_id", "rule_file_path"],
        },
    },
    {
        name: "rebuild_archive_snapshots",
        description: "Enqueue full rebuild of archive snapshots from DB canonical state. " +
            "Generates .claude/memory-archive/ snapshots for disaster recovery. " +
            "WRITE-ONLY tool restricted to memory-maintainer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "get_conversation_transcript",
        description: "Retrieve conversation messages for a given conversation ID, ordered chronologically. Used by memory-capture for analysis.",
        inputSchema: {
            type: "object",
            properties: {
                conversation_id: {
                    type: "string",
                    description: "The conversation ID",
                },
            },
            required: ["conversation_id"],
        },
    },
    // ========================================================================
    // PROJECT ANALYSIS TOOLS (worker/reviewer/merger + project-analyzer agents)
    // ========================================================================
    {
        name: "get_project_analysis",
        description: "Get project analysis data including build commands, validation commands, and worktree setup instructions. " +
            "Returns path-scoped entries with resolved template variables ({project_root}, {worktree_path}, {task_branch}). " +
            "If analysis hasn't been run yet, returns { status: 'analyzing', retry_after_secs: 30 }.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID (from RALPHX_PROJECT_ID env var)",
                },
                task_id: {
                    type: "string",
                    description: "Optional task ID for resolving {worktree_path} and {task_branch} template variables",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "save_project_analysis",
        description: "Save auto-detected project analysis data. Updates detected_analysis and analyzed_at fields. " +
            "Never touches custom_analysis (user overrides). Only callable by the project-analyzer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                entries: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            path: {
                                type: "string",
                                description: "Subpath relative to project root (e.g., '.', 'src-tauri/')",
                            },
                            label: {
                                type: "string",
                                description: "Human-readable label (e.g., 'Node.js root', 'Rust backend')",
                            },
                            install: {
                                type: "string",
                                description: "Install command (e.g., 'npm install'). Null if not needed.",
                            },
                            validate: {
                                type: "array",
                                items: { type: "string" },
                                description: "Validation commands (e.g., ['npm run typecheck', 'npm run lint'])",
                            },
                            worktree_setup: {
                                type: "array",
                                items: { type: "string" },
                                description: "Commands to run in worktree setup (e.g., ['ln -s {project_root}/node_modules {worktree_path}/node_modules'])",
                            },
                        },
                        required: ["path", "label"],
                    },
                    description: "Array of path-scoped analysis entries",
                },
            },
            required: ["project_id", "entries"],
        },
    },
];
/**
 * Tool scoping per agent type
 * Hard enforcement: each agent only sees tools appropriate for its role
 */
export const TOOL_ALLOWLIST = {
    [ORCHESTRATOR_IDEATION]: [
        "create_task_proposal",
        "update_task_proposal",
        "delete_task_proposal",
        // Note: add_proposal_dependency removed - dependencies are now auto-suggested by dependency-suggester agent
        "list_session_proposals",
        "get_proposal",
        "analyze_session_dependencies",
        "create_plan_artifact",
        "update_plan_artifact",
        "get_plan_artifact",
        "link_proposals_to_plan",
        "get_session_plan",
        "ask_user_question",
        // session linking tools
        "create_child_session",
        "get_parent_session_context",
        // session context recovery
        "get_session_messages",
        // team artifact tools (for local Task agent fallback)
        "get_team_artifacts",
        // verification tools
        "update_plan_verification",
        "get_plan_verification",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    [ORCHESTRATOR_IDEATION_READONLY]: [
        "list_session_proposals",
        "get_proposal",
        "get_plan_artifact",
        "get_session_plan",
        "get_parent_session_context",
        // session linking tools
        "create_child_session",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    [CHAT_TASK]: [
        "update_task",
        "add_task_note",
        "get_task_details",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    [CHAT_PROJECT]: [
        "suggest_task",
        "list_tasks",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
        "get_conversation_transcript",
    ],
    [REVIEWER]: [
        // specific review tools
        "complete_review",
        // issue tools (re-review workflow)
        "get_task_issues",
        "get_step_progress",
        "get_issue_progress",
        // project analysis tools
        "get_project_analysis",
        // common context tools
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Post-review chat agent - helps user discuss review findings and take action
    [REVIEW_CHAT]: [
        // specific review tools
        "approve_task",
        "request_task_changes",
        // common context tools
        "get_review_notes",
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Historical review discussion agent - read-only, no mutation tools (approved tasks)
    [REVIEW_HISTORY]: [
        "get_review_notes",
        "get_task_context",
        "get_task_issues",
        "get_task_steps",
        "get_step_progress",
        "get_issue_progress",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    [WORKER]: [
        // step management tools
        "start_step",
        "complete_step",
        "skip_step",
        "fail_step",
        "add_step",
        "get_step_progress",
        "get_step_context",
        "get_sub_steps",
        "execution_complete",
        // issue tools (re-execution workflow)
        "get_task_issues",
        "mark_issue_in_progress",
        "mark_issue_addressed",
        // project analysis tools
        "get_project_analysis",
        // common context tools
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    [CODER]: [
        // step management tools
        "start_step",
        "complete_step",
        "skip_step",
        "fail_step",
        "add_step",
        "get_step_progress",
        "get_step_context",
        // issue tools (re-execution workflow)
        "get_task_issues",
        "mark_issue_in_progress",
        "mark_issue_addressed",
        // project analysis tools
        "get_project_analysis",
        // common context tools
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Session naming agent - generates titles for IDA sessions
    [SESSION_NAMER]: ["update_session_title"],
    // Dependency suggester agent - analyzes proposals and auto-applies dependencies
    [DEPENDENCY_SUGGESTER]: ["apply_proposal_dependencies"],
    // Merger agent - resolves merge conflicts when programmatic merge fails
    [MERGER]: [
        // merge tools
        "report_conflict",
        "report_incomplete",
        "complete_merge",
        "get_merge_target",
        // project analysis tools
        "get_project_analysis",
        // common context tools
        "get_task_context",
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Orchestrator agent - plans and coordinates complex tasks
    [ORCHESTRATOR]: [
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Deep researcher agent - conducts thorough research and analysis
    [DEEP_RESEARCHER]: [
        // memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Project analyzer agent - detects build/validation commands
    [PROJECT_ANALYZER]: [
        "save_project_analysis",
        "get_project_analysis",
    ],
    // These agents have NO MCP tools - they use filesystem tools only
    [SUPERVISOR]: [],
    [QA_PREP]: [],
    [QA_TESTER]: [],
    // Memory agents - write-only memory tools (RESTRICTED - do not grant to other agents)
    [MEMORY_MAINTAINER]: [
        // Memory write tools (exclusive to memory agents)
        "upsert_memories",
        "mark_memory_obsolete",
        "refresh_memory_rule_index",
        "ingest_rule_file",
        "rebuild_archive_snapshots",
        // Read tools for context
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
        "get_conversation_transcript",
    ],
    [MEMORY_CAPTURE]: [
        // Memory write tools (exclusive to memory agents)
        "upsert_memories",
        // Read tools for deduplication and context
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
        "get_conversation_transcript",
    ],
    // Team lead agents - coordinate team execution
    [IDEATION_TEAM_LEAD]: [
        // Team coordination tools
        "request_team_plan",
        "request_teammate_spawn",
        "create_team_artifact",
        "get_team_artifacts",
        "get_team_session_state",
        "save_team_session_state",
        // Existing ideation tools
        "create_task_proposal",
        "update_task_proposal",
        "delete_task_proposal",
        "list_session_proposals",
        "get_proposal",
        "analyze_session_dependencies",
        "create_plan_artifact",
        "update_plan_artifact",
        "get_plan_artifact",
        "link_proposals_to_plan",
        "get_session_plan",
        "ask_user_question",
        // Session linking tools
        "create_child_session",
        "get_parent_session_context",
        // Session context recovery
        "get_session_messages",
        // Verification tools
        "update_plan_verification",
        "get_plan_verification",
        // Memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Ideation team members - research and analysis (read-only)
    [IDEATION_TEAM_MEMBER]: [
        // Team artifact tools
        "create_team_artifact",
        "get_team_artifacts",
        // Plan/proposal access (read-only)
        "get_session_plan",
        "list_session_proposals",
        "get_plan_artifact",
        // Memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Worker team lead - coordinates team execution for tasks
    [WORKER_TEAM_LEAD]: [
        // Team coordination tools
        "request_team_plan",
        "request_teammate_spawn",
        "create_team_artifact",
        "get_team_artifacts",
        "get_team_session_state",
        "save_team_session_state",
        // Step management tools
        "start_step",
        "complete_step",
        "skip_step",
        "fail_step",
        "add_step",
        "get_step_progress",
        "get_step_context",
        "get_sub_steps",
        // Execution completion signal
        "execution_complete",
        // Issue tools (re-execution workflow)
        "get_task_issues",
        "mark_issue_in_progress",
        "mark_issue_addressed",
        // Project analysis tools
        "get_project_analysis",
        // Common context tools
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
        // Memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Worker team members - implementation with team coordination
    [WORKER_TEAM_MEMBER]: [
        // Team artifact tools (document decisions)
        "create_team_artifact",
        "get_team_artifacts",
        // Step management tools
        "start_step",
        "complete_step",
        "skip_step",
        "fail_step",
        "add_step",
        "get_step_progress",
        "get_step_context",
        "get_sub_steps",
        // Issue tools (re-execution workflow)
        "get_task_issues",
        "mark_issue_in_progress",
        "mark_issue_addressed",
        // Project analysis tools
        "get_project_analysis",
        // Common context tools
        "get_task_context",
        "get_artifact",
        "get_artifact_version",
        "get_related_artifacts",
        "search_project_artifacts",
        "get_review_notes",
        "get_task_steps",
        // Memory read tools
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
    ],
    // Debug mode: shows ALL tools (use RALPHX_AGENT_TYPE=debug)
    debug: ALL_TOOLS.map((t) => t.name),
};
/**
 * Module-level agent type storage
 * Set by index.ts on startup after parsing CLI args
 * This is needed because CLI args take precedence over env vars
 * (Claude CLI doesn't pass env vars to MCP servers it spawns)
 */
let currentAgentType = "";
/**
 * Set the current agent type (called from index.ts after parsing CLI args)
 * @param agentType - The agent type to set
 */
export function setAgentType(agentType) {
    currentAgentType = agentType;
}
/**
 * Get the current agent type
 * @returns The current agent type
 */
export function getAgentType() {
    return currentAgentType || process.env.RALPHX_AGENT_TYPE || "";
}
/**
 * Get allowed tool names for the current agent type
 * @returns Array of tool names this agent is allowed to use
 */
export function getAllowedToolNames() {
    // Check for env var override (used for dynamic teammate tool scoping)
    const envAllowedTools = process.env.RALPHX_ALLOWED_MCP_TOOLS;
    if (envAllowedTools) {
        return envAllowedTools.split(',').map(t => t.trim()).filter(t => t.length > 0);
    }
    // Default: use agent type from TOOL_ALLOWLIST
    const agentType = getAgentType();
    return TOOL_ALLOWLIST[agentType] || [];
}
/**
 * Get filtered tools based on agent type
 * @returns Tools available to the current agent
 */
export function getFilteredTools() {
    const allowedNames = getAllowedToolNames();
    return ALL_TOOLS.filter((tool) => allowedNames.includes(tool.name));
}
/**
 * Check if a tool is allowed for the current agent type
 * @param toolName - Name of the tool to check
 * @returns true if allowed, false otherwise
 */
export function isToolAllowed(toolName) {
    const allowedNames = getAllowedToolNames();
    return allowedNames.includes(toolName);
}
/**
 * Get all tools regardless of agent type (for debugging)
 * @returns All available tools
 */
export function getAllTools() {
    return ALL_TOOLS;
}
/**
 * Get all tool names grouped by agent type (for debugging)
 * @returns Object mapping agent types to their allowed tools
 */
export function getToolsByAgent() {
    return TOOL_ALLOWLIST;
}
/**
 * Print all available tools to stderr (for debugging)
 * Call this to see what tools the MCP server can provide
 */
export function logAllTools() {
    console.error("\n=== RalphX MCP Server - All Available Tools ===\n");
    for (const [agentType, tools] of Object.entries(TOOL_ALLOWLIST)) {
        if (tools.length > 0) {
            console.error(`[${agentType}]`);
            tools.forEach((t) => console.error(`  - ${t}`));
            console.error("");
        }
    }
    console.error("=== End of Tools List ===\n");
}
//# sourceMappingURL=tools.js.map