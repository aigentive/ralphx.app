import type {
  CompiledContextReadResponse,
  SolutionCritiqueReadResponse,
} from "@/api/solution-critic";

type CritiqueResult = SolutionCritiqueReadResponse | null | undefined;
type CritiqueContext = CompiledContextReadResponse | null | undefined;

export type CritiqueDigestState = "empty" | "loading" | "error" | "fresh" | "stale";

export interface CritiqueDigest {
  state: CritiqueDigestState;
  verdict: string | null;
  verdictLabel: string;
  confidenceLabel: string | null;
  tone: "neutral" | "success" | "warning" | "danger" | "accent";
  primaryAction: "Run critique" | "Open critique" | "Critiquing";
  pillLabel: string;
  claimCount: number;
  flaggedClaimCount: number;
  riskCount: number;
  highestRiskSeverity: string | null;
  projectedGapCount: number;
  generatedAt: string | null;
  isStale: boolean;
}

const severityRank: Record<string, number> = {
  critical: 0,
  high: 1,
  medium: 2,
  low: 3,
};

export function formatCritiqueEnum(value: string | null | undefined): string {
  if (!value) return "";
  return value
    .split("_")
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
    .join(" ");
}

export function critiqueGapOriginLabel(category: string): string | null {
  switch (category) {
    case "solution_critique_claim":
      return "from critique: claim";
    case "solution_critique_risk":
      return "from critique: risk";
    case "solution_critique_verification":
      return "from critique: verification";
    default:
      return null;
  }
}

export function buildCritiqueDigest({
  context,
  result,
  isLoading,
  error,
}: {
  context: CritiqueContext;
  result: CritiqueResult;
  isLoading: boolean;
  error: string | null | undefined;
}): CritiqueDigest {
  if (isLoading) {
    return emptyDigest("loading", "Critiquing", "Critiquing...");
  }

  if (error) {
    return emptyDigest("error", "Run critique", "Critique failed");
  }

  const critique = result?.solutionCritique;
  if (!critique) {
    return emptyDigest("empty", "Run critique", "Critique");
  }

  const isStale = isCritiqueStale(context, result);
  const verdictLabel = formatCritiqueEnum(critique.verdict);
  const confidenceLabel = formatCritiqueEnum(critique.confidence);
  const highestRiskSeverity = highestSeverity(critique.risks.map((risk) => risk.severity));
  const flaggedClaimCount = critique.claims.filter((claim) => claim.status !== "supported").length;
  const riskSuffix =
    critique.risks.length > 0
      ? ` - ${critique.risks.length} risk${critique.risks.length === 1 ? "" : "s"}`
      : "";

  return {
    state: isStale ? "stale" : "fresh",
    verdict: critique.verdict,
    verdictLabel,
    confidenceLabel,
    tone: verdictTone(critique.verdict),
    primaryAction: "Open critique",
    pillLabel: isStale
      ? `${verdictLabel} - stale`
      : `${verdictLabel} - ${critique.confidence}${riskSuffix}`,
    claimCount: critique.claims.length,
    flaggedClaimCount,
    riskCount: critique.risks.length,
    highestRiskSeverity,
    projectedGapCount: result.projectedGaps.length,
    generatedAt: critique.generatedAt,
    isStale,
  };
}

function emptyDigest(
  state: CritiqueDigestState,
  primaryAction: CritiqueDigest["primaryAction"],
  pillLabel: string,
): CritiqueDigest {
  return {
    state,
    verdict: null,
    verdictLabel: state === "error" ? "Critique failed" : "Critique",
    confidenceLabel: null,
    tone: state === "error" ? "danger" : "neutral",
    primaryAction,
    pillLabel,
    claimCount: 0,
    flaggedClaimCount: 0,
    riskCount: 0,
    highestRiskSeverity: null,
    projectedGapCount: 0,
    generatedAt: null,
    isStale: false,
  };
}

function verdictTone(verdict: string): CritiqueDigest["tone"] {
  switch (verdict) {
    case "accept":
      return "success";
    case "revise":
      return "accent";
    case "investigate":
      return "warning";
    case "reject":
      return "danger";
    default:
      return "neutral";
  }
}

function highestSeverity(values: string[]): string | null {
  return values
    .filter((value) => value in severityRank)
    .sort((left, right) => severityRank[left]! - severityRank[right]!)[0] ?? null;
}

function isCritiqueStale(context: CritiqueContext, result: CritiqueResult): boolean {
  const generatedAt = result?.solutionCritique.generatedAt;
  if (!generatedAt || !context?.compiledContext) return false;
  const generatedTime = Date.parse(generatedAt);
  if (Number.isNaN(generatedTime)) return false;
  const sourceTimes = [
    context.compiledContext.generatedAt,
    ...context.compiledContext.sources
      .map((source) => source.createdAt)
      .filter((value): value is string => Boolean(value)),
  ];
  return sourceTimes.some((value) => {
    const sourceTime = Date.parse(value);
    return !Number.isNaN(sourceTime) && sourceTime > generatedTime;
  });
}
