import { type VerificationFindingSummary, type VerificationRoundDelegateInput } from "./verification-round-assessment.js";
export type VerificationTerminalBody = {
    status: string;
    generation: number;
    round?: number;
    convergence_reason?: string;
    [key: string]: unknown;
};
export type VerificationSettlementResult = {
    classification: "complete" | "pending" | "infra_failure";
    missing_required_critics: string[];
    verification_findings?: VerificationFindingSummary[];
    [key: string]: unknown;
};
export declare function completePlanVerificationWithSettlement(deps: {
    sessionId: string;
    body: VerificationTerminalBody;
    requiredDelegates: VerificationRoundDelegateInput[];
    createdAfter?: string;
    rescueBudgetExhausted: boolean;
    includeFullContent: boolean;
    includeMessages: boolean;
    messageLimit: number;
    maxWaitMs: number;
    pollIntervalMs: number;
    awaitVerificationRoundSettlement: (args: {
        session_id: string;
        delegates: VerificationRoundDelegateInput[];
        created_after?: string;
        rescue_budget_exhausted?: boolean;
        include_full_content?: boolean;
        include_messages?: boolean;
        message_limit?: number;
        max_wait_ms?: number;
        poll_interval_ms?: number;
    }) => Promise<VerificationSettlementResult>;
    callInfraFailure: (args: {
        generation: number;
        convergence_reason?: string;
        round?: number;
    }) => Promise<Record<string, unknown>>;
    callCompletion: (body: Record<string, unknown>) => Promise<unknown>;
}): Promise<Record<string, unknown> | unknown>;
//# sourceMappingURL=verification-completion.d.ts.map