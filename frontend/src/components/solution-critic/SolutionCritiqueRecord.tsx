import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ShieldCheck } from "lucide-react";
import {
  solutionCriticApi,
  solutionCriticQueryKeys,
  type SolutionCritiqueTargetInput,
} from "@/api/solution-critic";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { buildCritiqueDigest } from "./critiqueDigest";
import { SolutionCritiqueDetails } from "./SolutionCritiqueDetails";

interface SolutionCritiqueRecordProps {
  sessionId: string | null | undefined;
  target: SolutionCritiqueTargetInput;
  title?: string;
}

export function SolutionCritiqueRecord({
  sessionId,
  target,
  title = "Saved Solution Critique",
}: SolutionCritiqueRecordProps) {
  const [open, setOpen] = useState(false);
  const contextQuery = useQuery({
    queryKey: solutionCriticQueryKeys.targetContext(sessionId, target),
    queryFn: () => solutionCriticApi.getLatestTargetCompiledContext(sessionId!, target),
    enabled: Boolean(sessionId),
    staleTime: 30_000,
    retry: false,
  });
  const critiqueQuery = useQuery({
    queryKey: solutionCriticQueryKeys.targetCritique(sessionId, target),
    queryFn: () => solutionCriticApi.getLatestTargetSolutionCritique(sessionId!, target),
    enabled: Boolean(sessionId),
    staleTime: 30_000,
    retry: false,
  });

  const result = critiqueQuery.data ?? null;
  const context = contextQuery.data ?? null;
  if (!sessionId || !result) return null;

  const digest = buildCritiqueDigest({
    context,
    result,
    isLoading: false,
    error: null,
  });
  const riskLabel = `${digest.riskCount} risk${digest.riskCount === 1 ? "" : "s"}`;

  return (
    <>
      <div
        data-testid="solution-critique-record"
        className="flex items-center justify-between gap-3 rounded-md border px-3 py-2"
        style={{
          background: "var(--overlay-faint)",
          borderColor: "var(--overlay-weak)",
        }}
      >
        <div className="flex min-w-0 items-center gap-2">
          <ShieldCheck className="h-4 w-4 shrink-0 text-text-primary/45" />
          <div className="min-w-0">
            <div className="text-[11px] font-semibold uppercase text-text-primary/40">
              {title}
            </div>
            <div className="truncate text-[12px] text-text-primary/60">
              {digest.verdictLabel} - {digest.confidenceLabel ?? "unknown"} confidence - {riskLabel}
              {digest.isStale ? " - stale" : ""}
            </div>
          </div>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 shrink-0 rounded-md px-2 text-[11px]"
          onClick={() => setOpen(true)}
        >
          Open critique
        </Button>
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent
          hideCloseButton={false}
          className="left-auto right-4 top-4 h-[calc(100vh-2rem)] max-h-[calc(100vh-2rem)] w-[min(560px,calc(100vw-2rem))] max-w-none translate-x-0 translate-y-0 overflow-hidden p-0"
          onClick={(event) => event.stopPropagation()}
        >
          <DialogTitle className="sr-only">Solution critique record</DialogTitle>
          <DialogDescription className="sr-only">
            Saved evidence, risks, verification plan, and safe next action for the selected target.
          </DialogDescription>
          <SolutionCritiqueDetails
            targetLabel={target.label ?? title}
            context={context}
            result={result}
            digest={digest}
            isLoading={false}
            error={null}
            readOnly
          />
        </DialogContent>
      </Dialog>
    </>
  );
}
