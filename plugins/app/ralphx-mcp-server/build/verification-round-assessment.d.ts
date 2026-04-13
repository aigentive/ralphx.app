type ArtifactSummary = {
    id?: string;
    name?: string;
    created_at?: string;
    content?: string;
};
export type VerificationRoundArtifactMatch = {
    prefix: string;
    found: boolean;
    total_matches: number;
    artifact?: ArtifactSummary;
};
export type VerificationRoundDelegateInput = {
    job_id: string;
    artifact_prefix: string;
    required?: boolean;
    label?: string;
};
export type VerificationRoundDelegateSnapshot = {
    job_id: string;
    status?: string;
    error?: string | null;
    delegated_status?: {
        agent_state?: {
            estimated_status?: string | null;
        };
        latest_run?: {
            status?: string | null;
            error_message?: string | null;
        } | null;
    } | null;
};
type DelegateAssessmentKind = "artifact_published" | "pending" | "infra_failure";
export type VerificationRoundDelegateAssessment = {
    job_id: string;
    label: string;
    artifact_prefix: string;
    required: boolean;
    artifact_found: boolean;
    assessment: DelegateAssessmentKind;
    status: string;
    reason: string;
};
export type VerificationRoundAssessment = {
    classification: "complete" | "pending" | "infra_failure";
    recommended_next_action: "continue_round_analysis" | "perform_single_rescue_or_wait" | "complete_verification_with_infra_failure";
    summary: string;
    missing_required_prefixes: string[];
    delegate_assessments: VerificationRoundDelegateAssessment[];
    artifacts_by_prefix: VerificationRoundArtifactMatch[];
};
export declare function assessVerificationRound(params: {
    delegates: VerificationRoundDelegateInput[];
    artifactsByPrefix: VerificationRoundArtifactMatch[];
    delegateSnapshots: VerificationRoundDelegateSnapshot[];
    rescueBudgetExhausted?: boolean;
}): VerificationRoundAssessment;
export {};
//# sourceMappingURL=verification-round-assessment.d.ts.map