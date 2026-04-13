function formatToolExamples(tool, limit = 1) {
    const examples = (tool.inputSchema?.examples ?? [])
        .slice(0, limit)
        .map((example) => {
        try {
            return JSON.stringify(example);
        }
        catch {
            return String(example);
        }
    })
        .filter((example) => example.length > 0);
    return examples;
}
export function getToolRecoveryHintFromRegistry(tools, toolName) {
    const tool = tools.find((candidate) => candidate.name === toolName);
    if (!tool) {
        return null;
    }
    switch (toolName) {
        case "update_plan_verification": {
            const examples = formatToolExamples(tool, 2);
            return [
                "Use the PARENT ideation session_id as the canonical target. If a verification child session_id is passed, the backend remaps it automatically.",
                "If report_verification_round / complete_plan_verification are available, prefer those narrower helpers instead of this generic tool.",
                "Use status=reviewing with in_progress=true for mid-round updates; use verified or needs_revision with in_progress=false for terminal updates.",
                "Re-read get_plan_verification if generation/in_progress is unclear instead of guessing.",
                ...examples.map((example, index) => index === 0
                    ? `Example reviewing payload: ${example}`
                    : `Example terminal payload: ${example}`),
            ].join("\n");
        }
        case "report_verification_round": {
            const examples = formatToolExamples(tool);
            return [
                "Use this verifier-friendly helper for in-progress rounds on the PARENT ideation session.",
                "If a verification child session_id is passed, the backend remaps it to the parent automatically.",
                "You only provide round, gaps, and generation; status=reviewing and in_progress=true are filled in automatically.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "complete_plan_verification": {
            const examples = formatToolExamples(tool, 2);
            return [
                "Use this verifier-friendly helper for terminal verification updates on the PARENT ideation session.",
                "If a verification child session_id is passed, the backend remaps it to the parent automatically.",
                "You provide the terminal status and generation; in_progress=false is filled in automatically.",
                "When required_delegates and created_after are present, the helper derives canonical terminal gaps from typed required-critic findings for that round.",
                "External sessions cannot use status=skipped.",
                ...examples.map((example, index) => index === 0
                    ? `Example terminal payload: ${example}`
                    : `Example abort-cleanup payload: ${example}`),
            ].join("\n");
        }
        case "get_plan_verification": {
            const examples = formatToolExamples(tool);
            return [
                "Call this on the PARENT ideation session before retrying report_verification_round, complete_plan_verification, or update_plan_verification. If a verification child session_id is passed, the backend remaps it to the parent automatically.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "create_team_artifact": {
            const examples = formatToolExamples(tool);
            return [
                "Use the PARENT ideation session_id as the canonical target. If a verification child session id is passed, the backend remaps it to the parent automatically.",
                "Use this for general specialist findings and team summaries. Verification-path specialists should use publish_verification_finding instead.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "publish_verification_finding": {
            const examples = formatToolExamples(tool);
            return [
                "Use this for verification-path specialists and required verification critics.",
                "If session_id is omitted, the backend injects the current session context and remaps verification child sessions to the parent ideation session automatically.",
                "Publish one structured finding with critic, round, status, summary, and gaps instead of encoding verifier output into a generic TeamResearch document.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "get_team_artifacts": {
            const examples = formatToolExamples(tool);
            return [
                "Read artifacts from the PARENT ideation session_id as the canonical target. If a verification child session id is passed, the backend remaps it to the parent automatically.",
                "Verification flows should usually prefer get_verification_round_artifacts instead of manually sorting summaries and then loading full artifact ids.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "get_verification_round_artifacts": {
            const examples = formatToolExamples(tool);
            return [
                "This is a low-level verifier helper for debugging.",
                "Normal verifier prompts should prefer run_verification_round instead of manually calling get_team_artifacts + get_artifact + client-side sorting for current-round artifacts.",
                "Provide the parent ideation session_id plus the title prefixes you expect; the MCP proxy filters by created_after and returns the latest match per prefix.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "assess_verification_round": {
            const examples = formatToolExamples(tool);
            return [
                "This is a low-level runtime classification helper.",
                "Normal verifier prompts should prefer run_verification_round; use this only after bounded wait/rescue attempts when debugging that behavior directly.",
                "If rescue_budget_exhausted=true and a required artifact is still missing, treat the result as infrastructure failure instead of inventing direct-review fallback behavior.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "run_verification_enrichment": {
            const examples = formatToolExamples(tool);
            return [
                "Use this as the backend-owned one-time enrichment driver.",
                "It selects and dispatches intent/code-quality specialists, waits a bounded amount, and returns the latest enrichment artifacts plus delegate snapshots.",
                "Prefer this over manually choosing enrichment specialists and polling artifacts in the verifier prompt.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "run_verification_round": {
            const examples = formatToolExamples(tool);
            return [
                "Use this as the primary verifier round driver.",
                "It selects optional specialists, runs the required critics through the backend helper, waits for bounded optional settlement, and returns structured required critic findings plus backend-owned merged_gaps.",
                "Prefer this over manual delegate_start/delegate_wait/get_verification_round_artifacts orchestration in the prompt.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "run_required_verification_critic_round": {
            const examples = formatToolExamples(tool);
            return [
                "This is the lower-level implementation behind the first-class required-critic round driver.",
                "It launches completeness + feasibility, performs one bounded rescue pass when required artifacts are still missing, and returns the final required_delegates set for terminal cleanup.",
                "Normal verifier prompts should prefer run_verification_round.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "await_verification_round_settlement": {
            const examples = formatToolExamples(tool);
            return [
                "This is a low-level synchronization barrier for required critics/specialists.",
                "It waits for required delegate jobs to either publish their current-round artifacts or reach a terminal state, then returns a settled classification.",
                "Normal verifier prompts should prefer run_verification_round instead of calling this directly or narrating manual poll loops.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "get_child_session_status": {
            const examples = formatToolExamples(tool);
            return [
                "When debugging a verification child, set include_recent_messages=true so you can inspect the last assistant/tool outputs.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "send_ideation_session_message": {
            const examples = formatToolExamples(tool);
            return [
                "When nudging a verifier/critic, repeat full invariant context: SESSION_ID, ROUND, artifact prefix/schema, and explicit parent-session target.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        default: {
            const examples = formatToolExamples(tool);
            if (examples.length === 0) {
                return null;
            }
            return examples.map((example) => `Example payload: ${example}`).join("\n");
        }
    }
}
export function formatToolErrorMessageFromRegistry(tools, toolName, message, details) {
    const repairHint = getToolRecoveryHintFromRegistry(tools, toolName);
    return (`ERROR: ${message}` +
        (details ? `\n\nDetails: ${details}` : "") +
        (repairHint ? `\n\nUsage hint for ${toolName}:\n${repairHint}` : ""));
}
//# sourceMappingURL=tool-recovery.js.map