import type { SolutionCritiqueReadResponse } from "@/api/solution-critic";
import { formatCritiqueEnum } from "./critiqueDigest";

export interface CritiqueApprovalWarning {
  title: string;
  description: string;
  confirmText: string;
  variant: "default" | "destructive";
}

export function buildCritiqueApprovalWarning(
  result: SolutionCritiqueReadResponse | null | undefined
): CritiqueApprovalWarning | null {
  const critique = result?.solutionCritique;
  if (!critique) return null;

  const highRisk = critique.risks.find(
    (risk) => risk.severity === "critical" || risk.severity === "high"
  );
  if (critique.verdict !== "reject" && !highRisk) return null;

  const verdictLabel = formatCritiqueEnum(critique.verdict) || "attention";
  const riskText = highRisk
    ? ` It flags a ${formatCritiqueEnum(highRisk.severity).toLowerCase()} risk: ${highRisk.risk}`
    : "";
  const safeNextAction = critique.safeNextAction
    ? ` Safe next action: ${critique.safeNextAction}`
    : "";

  return {
    title: "Approve despite solution critique?",
    description:
      `A saved solution critique returned ${verdictLabel} for this task execution before approval.` +
      riskText +
      safeNextAction +
      " Approve only if you reviewed the critique and accept the remaining risk.",
    confirmText: "Approve",
    variant: "destructive",
  };
}
