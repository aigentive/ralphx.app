import { assessVerificationRound, type VerificationFindingSummary, type VerificationRoundDelegateInput, type VerificationRoundDelegateSnapshot } from "./verification-round-assessment.js";
import { type RequiredCriticRoundResult, type VerificationPlanSnapshot } from "./verification-orchestration.js";
type VerificationFindingMatch = {
    critic: string;
    found: boolean;
    total_matches: number;
    finding?: VerificationFindingSummary;
};
export type VerificationSettlementArgs = {
    session_id: string;
    delegates: VerificationRoundDelegateInput[];
    created_after?: string;
    rescue_budget_exhausted?: boolean;
    include_full_content?: boolean;
    include_messages?: boolean;
    message_limit?: number;
    max_wait_ms?: number;
    poll_interval_ms?: number;
};
type ManagedVerificationDelegate = VerificationRoundDelegateInput & {
    agent_name: string;
    delegated_session_id?: string;
};
type AwaitVerificationRoundSettlementResult = {
    session_id: string;
    created_after: string | null;
    rescue_budget_exhausted: boolean;
    settled: boolean;
    timed_out: boolean;
    polls_performed: number;
    max_wait_ms: number;
    poll_interval_ms: number;
    verification_findings: VerificationFindingSummary[];
    classification: "complete" | "pending" | "infra_failure";
    recommended_next_action: "continue_round_analysis" | "perform_single_rescue_or_wait" | "complete_verification_with_infra_failure";
    summary: string;
    missing_required_critics: string[];
    delegate_assessments: ReturnType<typeof assessVerificationRound>["delegate_assessments"];
    findings_by_critic: ReturnType<typeof assessVerificationRound>["findings_by_critic"];
    delegate_snapshots: VerificationRoundDelegateSnapshot[];
};
type CachedVerificationRoundState = {
    round: number;
    classification: "complete" | "pending" | "infra_failure";
    createdAfter: string;
    requiredDelegates: VerificationRoundDelegateInput[];
    mergedGaps: unknown[];
    solutionCritique?: SolutionCritiqueRoundProjection;
};
type SolutionCritiqueRoundProjection = {
    compiled_context_artifact_id: string;
    critique_artifact_id: string;
    projected_gaps: unknown[];
};
type VerificationRuntimeDeps = {
    callTauri: (endpoint: string, payload: Record<string, unknown>) => Promise<unknown>;
    callTauriGet: (endpoint: string) => Promise<unknown>;
    agentType: string;
    contextType?: string;
    contextId?: string;
};
export declare function createVerificationRuntime(deps: VerificationRuntimeDeps): {
    getPlanVerificationForTool: (args: {
        session_id?: string;
    }) => Promise<unknown>;
    reportVerificationRoundForTool: (args: {
        session_id?: string;
        round: number;
        gaps?: unknown[];
        generation: number;
        [key: string]: unknown;
    }) => Promise<unknown>;
    completePlanVerificationForTool: (args: {
        session_id?: string;
        status: string;
        round?: number;
        convergence_reason?: string;
        generation: number;
    }) => Promise<unknown>;
    runVerificationEnrichment: (args: {
        session_id?: string;
        selected_specialists?: string[];
    }) => Promise<unknown>;
    runVerificationRound: (args: {
        session_id?: string;
        round: number;
        selected_specialists?: string[];
    }) => Promise<unknown>;
    rememberVerificationRoundState: (sessionId: string, state: CachedVerificationRoundState) => void;
    loadVerificationFindingsByCritic: (args: {
        sessionId: string;
        critics: string[];
        round?: number;
        createdAfter?: string;
    }) => Promise<VerificationFindingMatch[]>;
    loadVerificationDelegateSnapshots: (args: {
        delegates: VerificationRoundDelegateInput[];
        includeMessages: boolean;
        messageLimit: number;
    }) => Promise<(VerificationRoundDelegateSnapshot & {
        delegated_status?: unknown;
    })[]>;
    loadVerificationPlanSnapshot: (sessionId: string) => Promise<VerificationPlanSnapshot>;
    awaitOptionalVerificationDelegates: (args: {
        delegates: ManagedVerificationDelegate[];
        sessionId: string;
        createdAfter: string;
        critics: string[];
        includeFullContent: boolean;
        includeMessages: boolean;
        messageLimit: number;
        maxWaitMs: number;
        pollIntervalMs: number;
    }) => Promise<{
        created_after: string;
        polls_performed: number;
        timed_out: boolean;
        delegates: {
            job_id: string;
            critic: string;
            label: string | undefined;
            required: boolean | undefined;
        }[];
        findings_by_critic: VerificationFindingMatch[];
        delegate_snapshots: (VerificationRoundDelegateSnapshot & {
            delegated_status?: unknown;
        })[];
    }>;
    startManagedVerificationDelegate: (args: {
        agentName: string;
        parentSessionId: string;
        prompt: string;
        delegatedSessionId?: string;
    }) => Promise<{
        job_id: string;
        delegated_session_id?: string;
        agent_name: string;
        harness?: string;
        status?: string;
    }>;
    runRequiredVerificationCriticRound: (args: {
        sessionId: string;
        round: number;
        includeFullContent: boolean;
        includeMessages: boolean;
        messageLimit: number;
        maxWaitMs: number;
        pollIntervalMs: number;
    }) => Promise<RequiredCriticRoundResult>;
    awaitVerificationRoundSettlement: (args: VerificationSettlementArgs) => Promise<AwaitVerificationRoundSettlementResult>;
    getVerificationRoundState: (sessionId: string) => CachedVerificationRoundState | undefined;
    clearVerificationRoundState: (sessionId: string) => void;
    resolveVerifierParentSessionId: (rawSessionId: unknown, toolName: string) => Promise<string>;
    resolveVerificationFindingSessionId: (rawSessionId: unknown, toolName: string) => Promise<string>;
    resolveContextSessionId: (rawSessionId: unknown, toolName: string) => string;
};
export {};
//# sourceMappingURL=verification-runtime.d.ts.map